use std::{ops::Deref, sync::Arc, fmt::Write, f32::consts::PI};

use eframe::{
    egui::{self, CollapsingHeader, Frame, Key, Margin, Sense, SidePanel, TextEdit, TextStyle, Ui, WidgetText, FontSelection},
    epaint::{Color32, Rounding, Stroke, TextShape, Shape, PathShape},
};
use emath::{vec2, Rect, Pos2, pos2};

use crate::{
    app::SimulationContext,
    board::{ActiveCircuitBoard, CircuitBoard, SelectedItem, StoredCircuitBoard, selection::Selection},
    circuits::{props::{CircuitPropertyImpl, CircuitPropertyStore}, CircuitPreview},
    vector::{Vec2f, Vec2i},
    Direction4, DynStaticStr, PaintContext, PanAndZoom, PastePreview, RwLock, Screen, time::Instant, state::WireState,
};

use super::{
    drawing, CollapsibleSidePanel, DoubleSelectableLabel, InventoryItemGroup, PropertyEditor,
    PropertyStoreItem, Inventory, InventoryItem,
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum SelectedItemId {
    None,
    Wires,
    Selection,
    Paste,
    Circuit(DynStaticStr),
    Board(u128),
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub struct TileDrawBounds {
    pub screen_tl: Vec2f,
    pub screen_br: Vec2f,

    pub tiles_tl: Vec2i,
    pub tiles_br: Vec2i,

    pub chunks_tl: Vec2i,
    pub chunks_br: Vec2i,
}

impl TileDrawBounds {
    pub const EVERYTHING: TileDrawBounds = TileDrawBounds {
        screen_tl: Vec2f::single_value(f32::NEG_INFINITY),
        screen_br: Vec2f::single_value(f32::INFINITY),
        tiles_tl: Vec2i::single_value(i32::MIN),
        tiles_br: Vec2i::single_value(i32::MAX),
        chunks_tl: Vec2i::single_value(i32::MIN),
        chunks_br: Vec2i::single_value(i32::MAX),
    };
}

pub struct CircuitBoardEditor {
    pan_zoom: PanAndZoom,
    board: ActiveCircuitBoard,

    debug: bool,

    paste: Option<Arc<PastePreview>>,
    inventory_items: Arc<[InventoryItemGroup]>,
    selected_id: SelectedItemId,

    props_ui: PropertyEditor,
    sim: Arc<SimulationContext>,
}

static INVENTORY_CIRCUIT_ORDER: &[&str] = &["or", "nor", "and", "nand", "xor", "xnor", "not"];

static COMPONENT_BUILTIN_ORDER: &[&str] = &[
    "button",
    "or",
    "nor",
    "and",
    "nand",
    "xor",
    "xnor",
    "not",
    "transistor",
    "pin",
    "pullup",
    "freq_meter",
];


struct SelectionInventoryItem {}
impl InventoryItem for SelectionInventoryItem {
    fn id(&self) -> SelectedItemId {
        SelectedItemId::Selection
    }

    fn draw(&self, ctx: &PaintContext) {
        let rect = ctx.rect.shrink2(ctx.rect.size() / 5.0);
        ctx.paint
            .rect_filled(rect, Rounding::none(), Selection::fill_color());
        let rect_corners = [
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
            rect.left_top(),
        ];

        let mut shapes = vec![];
        Shape::dashed_line_many(
            &rect_corners,
            Stroke::new(1.0, Selection::border_color()),
            3.0,
            2.0,
            &mut shapes,
        );

        shapes.into_iter().for_each(|s| {
            ctx.paint.add(s);
        });
    }
}

struct WireInventoryItem {}
impl InventoryItem for WireInventoryItem {
    fn id(&self) -> SelectedItemId {
        SelectedItemId::Wires
    }

    fn draw(&self, ctx: &PaintContext) {
        let color = WireState::False.color();

        let rect1 = Rect::from_center_size(
            ctx.rect.lerp_inside([0.2, 0.2].into()),
            ctx.rect.size() * 0.2,
        );
        let rect2 = Rect::from_center_size(
            ctx.rect.lerp_inside([0.8, 0.8].into()),
            ctx.rect.size() * 0.2,
        );

        ctx.paint
            .line_segment([rect1.center(), rect2.center()], Stroke::new(2.5, color));

        ctx.paint.add(Shape::Path(PathShape {
            points: rotated_rect_shape(rect1, PI * 0.25, rect1.center()),
            closed: true,
            fill: color,
            stroke: Stroke::NONE,
        }));

        ctx.paint.add(Shape::Path(PathShape {
            points: rotated_rect_shape(rect2, PI * 0.25, rect2.center()),
            closed: true,
            fill: color,
            stroke: Stroke::NONE,
        }));
    }
}

struct CircuitInventoryItem {
    preview: Arc<CircuitPreview>,
    id: DynStaticStr,
}
impl InventoryItem for CircuitInventoryItem {
    fn id(&self) -> SelectedItemId {
        SelectedItemId::Circuit(self.id.clone())
    }

    fn draw(&self, ctx: &PaintContext) {
        let size = self.preview.describe().size.convert(|v| v as f32);
        let scale = Vec2f::from(ctx.rect.size()) / size;
        let scale = scale.x().min(scale.y());
        let size = size * scale;
        let rect = Rect::from_center_size(ctx.rect.center(), size.into());

        let circ_ctx = PaintContext {
            screen: Screen {
                scale,
                ..ctx.screen
            },
            rect,
            ..*ctx
        };
        self.preview.draw(&circ_ctx, false);
    }
}

fn rotated_rect_shape(rect: Rect, angle: f32, origin: Pos2) -> Vec<Pos2> {
    let mut points = vec![
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];

    let cos = angle.cos();
    let sin = angle.sin();

    for p in points.iter_mut() {
        let pl = *p - origin;

        let x = cos * pl.x - sin * pl.y;
        let y = sin * pl.x + cos * pl.y;
        *p = pos2(x, y) + origin.to_vec2();
    }

    points
}

impl CircuitBoardEditor {
    pub fn new(board: ActiveCircuitBoard, ctx: &Arc<SimulationContext>) -> Self {

        let inventory_group: Vec<_> = INVENTORY_CIRCUIT_ORDER
            .iter()
            .filter_map(|id| ctx.previews.get(*id))
            .map(|preview| {
                Box::new(CircuitInventoryItem {
                    preview: preview.clone(),
                    id: preview.imp.type_name(),
                }) as Box<dyn InventoryItem>
            })
            .collect();

        Self {
            pan_zoom: PanAndZoom::default(),
            board,
            debug: false,
            paste: None,
            inventory_items: vec![
                InventoryItemGroup::SingleItem(Box::new(SelectionInventoryItem {})),
                InventoryItemGroup::SingleItem(Box::new(WireInventoryItem {})),
                InventoryItemGroup::Group(inventory_group),
            ].into(),
            selected_id: SelectedItemId::None,
            props_ui: PropertyEditor::new(),
            sim: ctx.clone()
        }
    }

    pub fn draw_background(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        self.pan_zoom
            .update(ui, rect, self.selected_id == SelectedItemId::None);

        cfg_if::cfg_if! {
            if #[cfg(all(not(web_sys_unstable_apis), feature = "wasm"))] {
                let paste = ui
                    .input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::V))
                    .then(|| crate::io::GLOBAL_CLIPBOARD.lock().clone())
                    .flatten();
            } else {
                let paste = ui.input(|input| {
                    input
                        .events
                        .iter()
                        .find_map(|e| match e {
                            egui::Event::Paste(s) => Some(s),
                            _ => None,
                        })
                        .and_then(|p| ron::from_str::<crate::io::CopyPasteData>(p).ok())
                });
            }
        }

        if let Some(paste) = paste {
            self.paste = Some(Arc::new(PastePreview::new(paste, &self.sim)));
            self.selected_id = SelectedItemId::Paste;
        }

        let selected_item = self.selected_item();

        if !ui.ctx().wants_keyboard_input() {
            if ui.input(|input| input.key_pressed(Key::F9)) {
                self.debug = !self.debug;
            } else if ui.input(|input| input.key_pressed(Key::F8)) {
                let board = self.board.board.clone();
                let state = self.board.state.clone();
                self.board = ActiveCircuitBoard::new(board, state);
            } else if ui.input(|input| input.key_pressed(Key::F7)) {
                self.board.board.write().regenerate_temp_design();
                if let Some(board) = self.sim.boards.read().get(&self.board.board.read().uid) {
                    if let Some(preview) = &board.preview {
                        preview.redescribe();
                    }
                }
            } else if ui.input(|input| input.key_pressed(Key::F4)) {
                let state = &self.board.state;
                state.reset();
                state.update_everything();
            }

            if ui.input(|input| input.key_pressed(Key::R)) {
                self.change_selected_props(&selected_item, "dir", |d: &mut Direction4| {
                    *d = d.rotate_clockwise()
                });
            }

            if ui.input(|input| input.key_pressed(Key::F)) {
                self.change_selected_props(&selected_item, "flip", |f: &mut bool| *f = !*f);
            }

            if ui.input(|input| input.key_pressed(Key::Q)) {
                let sim_lock = self.board.board.read().sim_lock.clone();
                let sim_lock = sim_lock.write();

                let mut board = self.board.board.write();
                let ordered = board.is_ordered_queue();
                board.set_ordered_queue(!ordered, false);
                drop(sim_lock);
            }
        }

        let screen = self.pan_zoom.to_screen(rect);
        let paint = ui.painter_at(rect);
        drawing::draw_dynamic_grid(&screen, 16.0, 16.into(), &paint);
        drawing::draw_cross(&screen, rect, &paint);

        let ctx = PaintContext {
            screen,
            paint: &paint,
            rect,
            ui,
        };

        let tile_bounds = self.calc_draw_bounds(&screen);

        self.board
            .update(&ctx, tile_bounds, selected_item, self.debug);
    }

    pub fn draw_ui(&mut self, ui: &mut Ui) {

        let start_time = Instant::now();

        if let SelectedItemId::Board(board_id) = self.selected_id {
            if let Some(board) = self.sim.boards.write().get_mut(&board_id) {
                if board.preview.is_none() {
                    let preview =
                        crate::circuits::board::Preview::new_from_board(board.board.clone());
                    let preview =
                        CircuitPreview::new(Box::new(preview), CircuitPropertyStore::default());
                    board.preview = Some(Arc::new(preview));
                }
            }
        }

        let left_panel_rect = self.components_ui(ui);

        if let SelectedItem::Circuit(p) = self.selected_item() {
            let props = [((), &p.props).into()];
            let changed = Self::properties_ui(&mut self.props_ui, ui, Some(props))
                .is_some_and(|v| !v.is_empty());
            if changed {
                p.redescribe();
            }
        } else {
            let selection = self.board.selection.borrow();
            if !selection.selection.is_empty() {
                let selected_circuit_props = selection.selection.iter().filter_map(|o| match o {
                    crate::board::selection::SelectedWorldObject::Circuit { id } => Some(*id),
                    _ => None,
                });
                let board = self.board.board.read();
                let stores: Vec<_> = selected_circuit_props
                    .filter_map(|id| board.circuits.get(id).map(|c| (id, &c.props).into()))
                    .collect();

                let response = Self::properties_ui(&mut self.props_ui, ui, Some(stores));
                drop(selection);
                drop(board);

                if let Some(changes) = response {
                    for property in changes {
                        for circuit in property.affected_values {
                            self.board.circuit_property_changed(
                                circuit,
                                &property.id,
                                property.old_value.as_ref(),
                            );
                        }
                    }
                }
            } else {
                Self::properties_ui(
                    &mut self.props_ui,
                    ui,
                    None::<[PropertyStoreItem<'_, ()>; 1]>,
                );
            }
        }
        {
            let mut rect = ui.clip_rect();
            rect.min.x += left_panel_rect.width();
            rect = rect.shrink(10.0);
            let mut ui = ui.child_ui(rect, *ui.layout());

            //let mut selected = self.selected_id.clone();
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                self.selected_id = SelectedItemId::None;
            }

            let inv_resp = ui.add(Inventory {
                selected: &mut self.selected_id,
                groups: &self.inventory_items,
                item_size: [28.0, 28.0].into(),
                item_margin: Margin::same(6.0),
                margin: Margin::default(),
            });

            match (
                self.selected_id == SelectedItemId::Paste,
                self.paste.is_some(),
            ) {
                (true, true) => (),
                (true, false) => self.selected_id = SelectedItemId::None,
                (_, true) => self.paste = None,
                _ => (),
            }

            let selected_name = match self.selected_item() {
                SelectedItem::None => None,
                SelectedItem::Selection => Some("Selection".into()),
                SelectedItem::Wire => Some("Wire".into()),
                SelectedItem::Paste(_) => Some("Pasted objects".into()),
                SelectedItem::Circuit(c) => Some(c.imp.display_name()),
            };

            if let Some(selected_name) = selected_name {
                let galley = WidgetText::from(selected_name.deref())
                    .fallback_text_style(TextStyle::Monospace)
                    .into_galley(
                        &ui,
                        Some(true),
                        inv_resp.rect.width(),
                        FontSelection::Style(TextStyle::Monospace),
                    )
                    .galley;

                let size = galley.rect.size() + vec2(12.0, 6.0);
                let offset = vec2(
                    20.0f32.min(inv_resp.rect.width() - 5.0 - size.x).max(0.0),
                    -2.5,
                );

                let resp = ui.allocate_response(size + offset, Sense::hover());
                let rect = Rect::from_min_size(resp.rect.min + offset, size);
                let paint = ui.painter();
                paint.rect(
                    rect,
                    Rounding {
                        nw: 0.0,
                        ne: 0.0,
                        sw: 3.0,
                        se: 3.0,
                    },
                    ui.style().visuals.panel_fill,
                    ui.style().visuals.window_stroke,
                );

                paint.add(TextShape {
                    pos: rect.min + vec2(6.0, 3.0),
                    galley,
                    underline: Stroke::NONE,
                    override_text_color: Some(ui.style().visuals.text_color()),
                    angle: 0.0,
                });
            }

            let mut text = String::new();

            #[cfg(feature = "single_thread")]
            {
                let sim_time = sim_time.as_secs_f32() * 1000.0;
                text.write_fmt(format_args!("Simulation time: {sim_time:.02}ms\n"))
                    .unwrap();
            }

            let paint_time = (Instant::now() - start_time).as_secs_f32() * 1000.0;
            let debug = self.debug;
            let ordered_queue = self.board.board.read().is_ordered_queue();

            text.write_fmt(format_args!(
                "Paint time: {paint_time:.02}ms\n\
                 [F9] Debug: {debug}\n\
                 [F8] Board reload\n\
                 [F7] Regenerate design\n\
                 [F4] State reset\n\
                 [R]  Rotate\n\
                 [F]  Flip\n\
                 [Q]  Ordered queue: {ordered_queue}\n\
                "
            ))
            .unwrap();

            ui.monospace(text);
        }
    }

    fn calc_draw_bounds(&self, screen: &Screen) -> TileDrawBounds {
        let chunk_size: Vec2f = (screen.scale * 16.0).into();

        let screen_tl = screen.wld_pos * screen.scale;
        let screen_br = screen_tl + screen.scr_rect.size();

        TileDrawBounds {
            screen_tl,
            screen_br,

            tiles_tl: (screen_tl / screen.scale).convert(|v| v.floor() as i32),
            tiles_br: (screen_br / screen.scale).convert(|v| v.floor() as i32),

            chunks_tl: (screen_tl / chunk_size).convert(|v| v.floor() as i32),
            chunks_br: (screen_br / chunk_size).convert(|v| v.floor() as i32),
        }
    }

    fn change_selected_props<T: CircuitPropertyImpl>(
        &mut self,
        selected_item: &SelectedItem,
        id: &str,
        f: impl Fn(&mut T),
    ) {
        if let SelectedItem::Circuit(pre) = selected_item {
            pre.props.write(id, f);
            pre.redescribe();
        } else {
            let selected_circuits: Vec<_> = self
                .board
                .selection
                .borrow()
                .selection
                .iter()
                .filter_map(|o| match o {
                    crate::board::selection::SelectedWorldObject::Circuit { id } => Some(*id),
                    _ => None,
                })
                .collect();

            let mut vec = vec![];
            let board = self.board.board.read();
            for circuit_id in selected_circuits {
                let circuit = board.circuits.get(circuit_id);
                let circuit = unwrap_option_or_continue!(circuit);
                let old = circuit.props.write(id, |p: &mut T| {
                    let old = p.clone();
                    f(p);
                    old
                });
                let old = unwrap_option_or_continue!(old);
                vec.push((circuit_id, old))
            }
            drop(board);
            for (circuit, old) in vec {
                self.board
                    .circuit_property_changed(circuit, id, old.as_ref());
            }
        }
    }

    fn selected_item(&self) -> SelectedItem {
        match &self.selected_id {
            SelectedItemId::None => SelectedItem::None,
            SelectedItemId::Paste => match &self.paste {
                Some(p) => SelectedItem::Paste(p.clone()),
                None => SelectedItem::None,
            },
            SelectedItemId::Selection => SelectedItem::Selection,
            SelectedItemId::Wires => SelectedItem::Wire,
            SelectedItemId::Circuit(circ) => match self.sim.previews.get(circ) {
                Some(p) => SelectedItem::Circuit(p.clone()),
                None => SelectedItem::None,
            },
            SelectedItemId::Board(id) => {
                let o = self
                    .sim
                    .boards
                    .read()
                    .get(id)
                    .and_then(|b| b.preview.clone());
                match o {
                    Some(p) => SelectedItem::Circuit(p),
                    None => SelectedItem::None,
                }
            }
        }
    }

    fn properties_ui<'a, T: Clone>(
        editor: &'a mut PropertyEditor,
        ui: &mut Ui,
        props: Option<impl IntoIterator<Item = PropertyStoreItem<'a, T>>>,
    ) -> Option<Vec<crate::ui::ChangedProperty<T>>> {
        let style = ui.style().clone();
        CollapsibleSidePanel::new("prop-ui", "Properties editor")
            .active(props.is_some())
            .header_offset(20.0)
            .side(egui::panel::Side::Right)
            .panel_transformer(Some(Box::new(move |panel: SidePanel| {
                panel
                    .frame(
                        Frame::side_top_panel(&style)
                            .rounding(Rounding {
                                nw: 5.0,
                                ne: 0.0,
                                sw: 5.0,
                                se: 0.0,
                            })
                            .outer_margin(Margin::symmetric(0.0, 8.0))
                            .inner_margin(Margin::symmetric(5.0, 5.0))
                            .stroke(style.visuals.window_stroke),
                    )
                    .show_separator_line(false)
            })))
            .show(ui, |ui| props.map(|props| editor.ui(ui, props).changes))
            .panel?
            .inner
    }

    fn components_ui(&mut self, ui: &mut Ui) -> Rect {
        let style = ui.style().clone();
        CollapsibleSidePanel::new("components-ui", "Components")
            .header_offset(20.0)
            .side(egui::panel::Side::Left)
            .panel_transformer(Some(Box::new(move |panel: SidePanel| {
                panel
                    .frame(
                        Frame::side_top_panel(&style)
                            .rounding(Rounding {
                                ne: 5.0,
                                nw: 0.0,
                                se: 5.0,
                                sw: 0.0,
                            })
                            .outer_margin(Margin::symmetric(0.0, 8.0))
                            .inner_margin(Margin::symmetric(5.0, 5.0))
                            .stroke(style.visuals.window_stroke),
                    )
                    .show_separator_line(false)
            })))
            .show(ui, |ui| {
                let font = TextStyle::Monospace.resolve(ui.style());

                CollapsingHeader::new("Built-in")
                    .default_open(true)
                    .show(ui, |ui| {
                        for name in COMPONENT_BUILTIN_ORDER {
                            if let Some(preview) =
                                self.sim.previews.get(&DynStaticStr::Static(name))
                            {
                                ui.horizontal(|ui| {
                                    let resp = ui.allocate_response(
                                        vec2(font.size, font.size),
                                        Sense::hover(),
                                    );
                                    let (rect, scale) = drawing::align_rect_scaled(
                                        resp.rect.min,
                                        vec2(font.size, font.size),
                                        preview.describe().size.convert(|v| v as f32).into(),
                                    );

                                    let paint_ctx = PaintContext::new_on_ui(ui, rect, scale);
                                    preview.draw(&paint_ctx, false);

                                    let selected = match &self.selected_id {
                                        SelectedItemId::Circuit(id) => {
                                            *id == preview.imp.type_name()
                                        }
                                        _ => false,
                                    };

                                    if ui
                                        .selectable_label(
                                            selected,
                                            preview.imp.display_name().deref(),
                                        )
                                        .clicked()
                                    {
                                        self.selected_id = match selected {
                                            true => SelectedItemId::None,
                                            false => {
                                                SelectedItemId::Circuit(preview.imp.type_name())
                                            }
                                        };
                                    }
                                });
                            }
                        }
                    });

                CollapsingHeader::new("Circuit boards")
                    .default_open(true)
                    .show(ui, |ui| {
                        let renamer_memory_id = ui.id().with("__renamer_memory");
                        let renamer_id = ui.id().with("__renamer_input");
                        let rename = ui
                            .memory(|mem| mem.data.get_temp::<Option<u128>>(renamer_memory_id))
                            .flatten();

                        let mut queued_deletion = None;
                        let mut drawn_renamer = false;
                        let no_delete = self.sim.boards.read().len() <= 1;
                        for board in self.sim.boards.read().values() {
                            let board_guard = board.board.read();

                            if Some(board_guard.uid) == rename && !drawn_renamer {
                                drop(board_guard);
                                let mut board_guard = board.board.write();

                                let res = TextEdit::singleline(board_guard.name.get_mut())
                                    .id(renamer_id)
                                    .show(ui);
                                drawn_renamer = true;

                                if res.response.lost_focus() {
                                    ui.memory_mut(|mem| {
                                        mem.data.insert_temp(renamer_memory_id, None::<u128>);
                                    });
                                }
                            } else {
                                let selected =
                                    self.selected_id == SelectedItemId::Board(board_guard.uid);
                                let active = board_guard.uid == self.board.board.read().uid;

                                let resp = ui.add(DoubleSelectableLabel::new(
                                    selected,
                                    active,
                                    board_guard.name.get_str().deref(),
                                    Color32::WHITE.gamma_multiply(0.3),
                                    None,
                                    Stroke::new(1.0, Color32::LIGHT_GREEN),
                                ));

                                if resp.clicked_by(egui::PointerButton::Primary) && !selected {
                                    self.selected_id = SelectedItemId::Board(board_guard.uid);
                                }

                                if resp.double_clicked_by(egui::PointerButton::Primary) && !active {
                                    self.board = ActiveCircuitBoard::new_main(board.board.clone());
                                }

                                resp.context_menu(|ui| {
                                    if ui.button("Rename").clicked() {
                                        // same hack as below
                                        if !drawn_renamer {
                                            TextEdit::singleline(&mut "").id(renamer_id).show(ui);
                                        }

                                        ui.memory_mut(|mem| {
                                            mem.data.insert_temp(
                                                renamer_memory_id,
                                                Some(board_guard.uid),
                                            );
                                            mem.request_focus(renamer_id);
                                        });
                                        ui.close_menu();
                                    }

                                    if !no_delete {
                                        if ui.input(|input| input.modifiers.shift) {
                                            if ui.button("Delete").clicked() {
                                                queued_deletion = Some(board_guard.uid);
                                                ui.close_menu();
                                            }
                                        } else {
                                            ui.menu_button("Delete", |ui| {
                                                if ui.button("Confirm").clicked() {
                                                    queued_deletion = Some(board_guard.uid);
                                                    ui.close_menu();
                                                }
                                            });
                                        }
                                    }
                                });
                            }
                        }

                        if ui.button("Add board").clicked() {
                            let mut board = CircuitBoard::new(self.sim.clone());
                            let uid = board.uid;
                            board.name = "New board".into();
                            let board = Arc::new(RwLock::new(board));
                            self.sim
                                .boards
                                .write()
                                .insert(uid, StoredCircuitBoard::new(board.clone()));
                            self.board = ActiveCircuitBoard::new_main(board);

                            // HACK: widget must exist before `request_focus` can be called on its id, panics otherwise
                            if !drawn_renamer {
                                TextEdit::singleline(&mut "").id(renamer_id).show(ui);
                            }

                            ui.memory_mut(|mem| {
                                mem.data.insert_temp(renamer_memory_id, Some(uid));
                                mem.request_focus(renamer_id);
                            });
                        }

                        if let Some(uid) = queued_deletion {
                            let mut boards = self.sim.boards.write();
                            boards.remove(&uid);
                            if self.board.board.read().uid == uid {
                                let board = boards.values().next().expect("Boards must exist!");
                                self.board = ActiveCircuitBoard::new_main(board.board.clone());
                            }
                        }
                    });
            })
            .full_rect
    }
}
