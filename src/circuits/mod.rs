use std::{
    f32::consts::TAU,
    hash::{DefaultHasher, Hasher},
    ops::Deref,
    sync::Arc,
};

use eframe::egui::{Color32, Rect};
use parking_lot::{Mutex, RwLock};

use crate::{
    board::Wire,
    editor::QuarterPos,
    ext::IteratorProduct,
    pool::get_pooled,
    selection::SelectionRenderer,
    str::ArcStaticStr,
    vector::{Vec2f, Vec2isize, Vec2usize},
    vertex_renderer::{ColoredTriangleBuffer, ColoredVertexRenderer},
    Direction4, Direction8, PaintContext,
};

pub struct Circuit {
    pub id: usize,
    pub info: RwLock<CircuitInfo>,
    pub imp: RwLock<CircuitImplBox>,
    pub pins: RwLock<Box<[RealizedPin]>>,
}

impl Circuit {
    pub fn pin_color(&self, pin: &CircuitPin) -> Color32 {
        // TODO: actual pin color
        let mut hasher = DefaultHasher::new();
        hasher.write_usize(75402938460);
        hasher.write_usize(self.id);
        hasher.write_usize(pin.id);
        let v = hasher.finish();
        let r = ((v >> 16) & 0xff) as u8;
        let g = ((v >> 8) & 0xff) as u8;
        let b = (v & 0xff) as u8;

        Color32::from_rgb(r, g, b)
    }
}

#[derive(Clone)]
pub struct CircuitInfo {
    pub pos: Vec2isize,
    pub render_size: Vec2usize,
    pub size: Vec2usize,
    pub transform: CircuitTransform,
}

#[derive(Clone, Copy)]
pub enum PinType {
    Custom,
    Inside,
    Outside,
}

#[derive(Clone)]
pub struct PinDescription {
    pub pos: Vec2usize,
    pub id: ArcStaticStr,
    pub display_name: ArcStaticStr,
    pub dir: Option<Direction8>,
    pub ty: PinType,
}
impl PinDescription {
    pub fn pos_dir_mut(&mut self) -> PosDirMut {
        PosDirMut {
            pos: &mut self.pos,
            dir: self.dir.as_mut(),
        }
    }
}

pub struct CircuitPin {
    pub id: usize,
    pub wire: RwLock<Option<Arc<Wire>>>,
}

pub struct RealizedPin {
    pub desc: PinDescription,
    pub pin: Arc<CircuitPin>,
}

pub struct CircuitSelectionRenderingContext<'a> {
    pub renderer: Arc<Mutex<SelectionRenderer>>,
    pub custom_selection: &'a mut bool,
}

pub struct CircuitRenderingContext<'a> {
    pub paint: &'a PaintContext<'a>,
    pub screen_rect: Rect,
    pub selection: Option<CircuitSelectionRenderingContext<'a>>,
    pub transform: CircuitTransform,

    // internal for transform_pos
    render_size: Vec2usize,
    angle: Option<f32>,
    flip: Option<FlipType>,
}

impl<'a> CircuitRenderingContext<'a> {
    pub fn new(
        ctx: &'a PaintContext,
        screen_rect: Rect,
        render_size: Vec2usize,
        selection: Option<CircuitSelectionRenderingContext<'a>>,
        transform: CircuitTransform,
    ) -> Self {
        let flip = transform
            .flip
            .then(|| transform.support.flip_type(None))
            .flatten();
        let angle = transform.support.rotation.and_then(|r| {
            if transform.dir == r.default_dir {
                None
            } else {
                Some(
                    transform
                        .dir
                        .rotated_counterclockwise_by(r.default_dir)
                        .into_angle_xp_cw()
                        + TAU / 4.0,
                )
            }
        });

        Self {
            paint: ctx,
            screen_rect,
            render_size,
            selection,
            transform,
            angle,
            flip,
        }
    }

    /// Transform circuit coordinate [0..size] to screen coordinate
    pub fn transform_pos(&self, pos: Vec2f) -> Vec2f {
        let norm = pos / self.render_size.convert(|v| v as f32);

        let norm = match self.flip {
            None => norm,
            Some(FlipType::Vertical) => [norm.x, 1.0 - norm.y].into(),
            Some(FlipType::Horizontal) => [1.0 - norm.x, norm.y].into(),
            Some(FlipType::Both) => [1.0 - norm.x, 1.0 - norm.y].into(),
        };

        let norm = match self.angle {
            None => norm,
            Some(a) => norm.rotated(a, 0.5),
        };

        self.screen_rect.lerp_inside(norm.into()).into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformSupport {
    Automatic,
    Manual,
}

#[derive(Debug, Clone, Copy)]
pub enum FlipType {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitRotationSupport {
    pub support: TransformSupport,
    pub default_dir: Direction4,
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitFlipSupport {
    pub support: TransformSupport,
    pub ty: FlipType,
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitTransformSupport {
    pub rotation: Option<CircuitRotationSupport>,
    pub flip: Option<CircuitFlipSupport>,
}

impl CircuitTransformSupport {
    pub fn rotation_default_dir(&self, support: Option<TransformSupport>) -> Option<Direction4> {
        let rot = self.rotation?;
        if support.is_some_and(|s| rot.support != s) {
            return None;
        }
        Some(rot.default_dir)
    }

    pub fn flip_type(&self, support: Option<TransformSupport>) -> Option<FlipType> {
        let flip = self.flip?;
        if support.is_some_and(|s| flip.support != s) {
            return None;
        }
        Some(flip.ty)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitTransform {
    pub support: CircuitTransformSupport,
    pub dir: Direction4,
    pub flip: bool,
}
impl CircuitTransform {
    pub fn transform_size(&self, size: Vec2usize, support: Option<TransformSupport>) -> Vec2usize {
        let Some(default_dir) = self.support.rotation_default_dir(support) else {
            return size;
        };

        if default_dir.is_vertical() == self.dir.is_vertical() {
            size
        } else {
            size.swapped()
        }
    }

    pub fn transform_pos(
        &self,
        size: Vec2usize,
        pos: Vec2usize,
        support: Option<TransformSupport>,
    ) -> Vec2usize {
        let flip = self.flip.then(|| self.support.flip_type(support)).flatten();

        let flipped_pos = match flip {
            None => pos,
            Some(FlipType::Vertical) => [pos.x, size.y - pos.y - 1].into(),
            Some(FlipType::Horizontal) => [size.x - pos.x - 1, pos.y].into(),
            Some(FlipType::Both) => [size.x - pos.x - 1, size.y - pos.y - 1].into(),
        };

        let default_dir = self.support.rotation_default_dir(support);

        match default_dir {
            None => flipped_pos,
            Some(default_dir) => {
                let dir = self.dir.rotated_counterclockwise_by(default_dir);
                let transformed_size = if default_dir.is_vertical() == self.dir.is_vertical() {
                    size
                } else {
                    size.swapped()
                };

                rotate_pos(flipped_pos, transformed_size, dir)
            }
        }
    }

    pub fn backtransform_pos(
        &self,
        size: Vec2usize,
        pos: Vec2usize,
        support: Option<TransformSupport>,
    ) -> Vec2usize {
        let default_dir = self.support.rotation_default_dir(support);

        let rotated_pos = match default_dir {
            None => pos,
            Some(default_dir) => {
                let dir = default_dir.rotated_counterclockwise_by(self.dir);
                rotate_pos(pos, size, dir)
            }
        };

        let flip = self.flip.then(|| self.support.flip_type(support)).flatten();

        match flip {
            None => rotated_pos,
            Some(FlipType::Vertical) => [rotated_pos.x, size.y - rotated_pos.y - 1].into(),
            Some(FlipType::Horizontal) => [size.x - rotated_pos.x - 1, rotated_pos.y].into(),
            Some(FlipType::Both) => [size.x - rotated_pos.x - 1, size.y - pos.y - 1].into(),
        }
    }

    pub fn transform_dir(&self, dir: Direction8, support: Option<TransformSupport>) -> Direction8 {
        let flip = self.flip.then(|| self.support.flip_type(support)).flatten();

        let flipped = match flip {
            None => dir,
            Some(FlipType::Vertical) => dir.flip_by(Direction8::Left),
            Some(FlipType::Horizontal) => dir.flip_by(Direction8::Up),
            Some(FlipType::Both) => dir.inverted(),
        };

        let default_dir = self.support.rotation_default_dir(support);

        match default_dir {
            None => flipped,
            Some(default_dir) => {
                let dir = self.dir.rotated_counterclockwise_by(default_dir);
                flipped.rotated_clockwise_by(dir.into())
            }
        }
    }

    fn transform_pins(
        &self,
        size: Vec2usize,
        pins: &mut dyn Iterator<Item = PosDirMut>,
        support: Option<TransformSupport>,
    ) {
        for pin in pins {
            *pin.pos = self.transform_pos(size, *pin.pos, support);

            if let Some(dir) = pin.dir {
                *dir = self.transform_dir(*dir, support);
            }
        }
    }
}

pub trait CircuitImpl: Clone + Send + Sync {
    fn id(&self) -> ArcStaticStr;
    fn display_name(&self) -> ArcStaticStr;

    /// Ignore `transform` if circuit doesn't support transforms, or supports auto transforms
    fn size(&self, transform: CircuitTransform) -> Vec2usize;

    /// Ignore `transform` if circuit doesn't support transforms, or supports auto transforms
    fn occupies_quarter(&self, transform: CircuitTransform, qpos: Vec2usize) -> bool {
        let _ = (transform, qpos);
        true
    }

    /// Ignore `transform` if circuit doesn't support transforms, or supports auto transforms
    fn describe_pins(&self, transform: CircuitTransform) -> Box<[PinDescription]>;

    fn transform_support(&self) -> CircuitTransformSupport {
        CircuitTransformSupport {
            rotation: Some(CircuitRotationSupport {
                support: TransformSupport::Automatic,
                default_dir: Direction4::Up,
            }),
            flip: None,
        }
    }

    fn draw(&self, ctx: &CircuitRenderingContext);
}

traitbox::traitbox! {
    pub box CircuitImplBox;

    #[as_impl]
    trait CircuitImpl {
        fn id(&self) -> ArcStaticStr;
        fn display_name(&self) -> ArcStaticStr;
        fn size(&self, transform: CircuitTransform) -> Vec2usize;
        fn occupies_quarter(&self, transform: CircuitTransform, qpos: Vec2usize) -> bool;
        fn describe_pins(&self, transform: CircuitTransform) -> Box<[PinDescription]>;
        fn transform_support(&self) -> CircuitTransformSupport;
        fn draw(&self, ctx: &CircuitRenderingContext);
    }

    trait Clone {
        fn clone(&self) -> Self;
    }

    auto trait Send {}
    auto trait Sync {}
}

pub struct CircuitBlueprint {
    pub id: ArcStaticStr,
    pub display_name: ArcStaticStr,
    pub imp: CircuitImplBox,
    pub inner_size: Vec2usize,
    pub transformed_size: Vec2usize,
    pub pins: Box<[PinDescription]>,
    pub transform: CircuitTransform,
}

impl CircuitBlueprint {
    pub fn new(imp: CircuitImplBox) -> Self {
        let trans_support = imp.transform_support();
        let dir = trans_support
            .rotation
            .map(|r| r.default_dir)
            .unwrap_or(Direction4::Up);
        let transform = CircuitTransform {
            support: trans_support,
            dir,
            flip: false,
        };

        let size = imp.size(transform);

        Self {
            id: imp.id(),
            display_name: imp.display_name(),
            inner_size: size,
            transformed_size: size,
            pins: imp.describe_pins(transform),
            imp,
            transform,
        }
    }

    pub fn recalculate(&mut self) {
        self.transform.support = self.imp.transform_support();
        self.inner_size = self.imp.size(self.transform);
        self.transformed_size = self
            .transform
            .transform_size(self.inner_size, Some(TransformSupport::Automatic));
        self.pins = self.imp.describe_pins(self.transform);
        self.transform.transform_pins(
            self.inner_size,
            &mut self.pins.iter_mut().map(|p| p.pos_dir_mut()),
            Some(TransformSupport::Automatic),
        );
    }
}

impl<T: CircuitImpl + 'static> From<T> for CircuitBlueprint {
    fn from(value: T) -> Self {
        Self::new(CircuitImplBox::new(value))
    }
}

pub struct PosDirMut<'a> {
    pub pos: &'a mut Vec2usize,
    pub dir: Option<&'a mut Direction8>,
}

pub const fn rotate_pos(pos: Vec2usize, target_size: Vec2usize, dir: Direction4) -> Vec2usize {
    match dir {
        Direction4::Up => pos,
        Direction4::Left => Vec2usize::new(pos.y, target_size.y - pos.x - 1),
        Direction4::Down => Vec2usize::new(target_size.x - pos.x - 1, target_size.y - pos.y - 1),
        Direction4::Right => Vec2usize::new(target_size.x - pos.y - 1, pos.x),
    }
}

#[derive(Clone)]
pub struct TestCircuit;

impl CircuitImpl for TestCircuit {
    fn id(&self) -> ArcStaticStr {
        "test".into()
    }

    fn display_name(&self) -> ArcStaticStr {
        "Test circuit".into()
    }

    fn size(&self, _transform: CircuitTransform) -> Vec2usize {
        [4, 3].into()
    }

    fn occupies_quarter(&self, _transform: CircuitTransform, qpos: Vec2usize) -> bool {
        const QUARTERS: [[usize; 8]; 6] = [
            [0, 0, 0, 0, 0, 0, 0, 0],
            [0, 1, 1, 1, 1, 1, 0, 0],
            [0, 1, 1, 1, 1, 1, 1, 0],
            [0, 1, 1, 1, 1, 1, 1, 0],
            [0, 1, 1, 0, 1, 1, 0, 0],
            [0, 0, 0, 0, 0, 0, 0, 0],
        ];

        QUARTERS[qpos.y][qpos.x] != 0
    }

    fn draw(&self, ctx: &CircuitRenderingContext) {
        let size = self.size(ctx.transform);

        let mut buffer = get_pooled::<ColoredTriangleBuffer>();

        for ((y, x), q) in (0..size.y)
            .product_clone(0..size.x)
            .product_clone(QuarterPos::ALL.iter().copied())
        {
            let pos = Vec2usize::new(x, y);
            let quarter_pos = pos * 2 + q.into_position();
            if !self.occupies_quarter(ctx.transform, quarter_pos) {
                continue;
            }

            let pos = pos.convert(|v| v as f32) + q.into_quarter_position_f32();

            let tl = ctx.transform_pos(pos);
            let br = ctx.transform_pos(pos + 0.5);

            let rect = Rect::from_two_pos(tl.into(), br.into());

            buffer.add_new_rect(
                rect.left_top(),
                rect.size(),
                Color32::WHITE.to_normalized_gamma_f32(),
            )
        }

        ctx.paint.custom_draw(move |ctx| {
            let mut vertexes = ColoredVertexRenderer::global(ctx.painter.gl());
            vertexes.draw(
                ctx.painter.gl(),
                ctx.paint_info.screen_size_px,
                buffer.deref(),
            );
        });

        // ctx.paint.painter.rect_stroke(
        //     ctx.screen_rect,
        //     Rounding::ZERO,
        //     Stroke::new(2.0, Color32::YELLOW),
        // );
    }

    fn describe_pins(&self, _transform: CircuitTransform) -> Box<[PinDescription]> {
        [
            PinDescription {
                pos: [0, 0].into(),
                id: "a".into(),
                display_name: "A".into(),
                dir: Some(Direction8::UpLeft),
                ty: PinType::Inside,
            },
            PinDescription {
                pos: [0, 2].into(),
                id: "b".into(),
                display_name: "B".into(),
                dir: Some(Direction8::DownLeft),
                ty: PinType::Inside,
            },
            PinDescription {
                pos: [2, 0].into(),
                id: "c".into(),
                display_name: "C".into(),
                dir: Some(Direction8::Up),
                ty: PinType::Inside,
            },
            PinDescription {
                pos: [2, 2].into(),
                id: "d".into(),
                display_name: "D".into(),
                dir: Some(Direction8::Down),
                ty: PinType::Inside,
            },
            PinDescription {
                pos: [3, 1].into(),
                id: "e".into(),
                display_name: "E".into(),
                dir: Some(Direction8::Right),
                ty: PinType::Outside,
            },
        ]
        .into()
    }

    fn transform_support(&self) -> CircuitTransformSupport {
        CircuitTransformSupport {
            rotation: Some(CircuitRotationSupport {
                support: TransformSupport::Automatic,
                default_dir: Direction4::Right,
            }),
            flip: Some(CircuitFlipSupport {
                support: TransformSupport::Automatic,
                ty: FlipType::Vertical,
            }),
        }
    }
}
