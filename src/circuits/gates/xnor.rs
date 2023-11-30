use std::ops::Div;

use eframe::epaint::{Color32, FontId, Stroke};
use emath::{pos2, remap, Align2, Pos2};

use crate::{
    board::ActiveCircuitBoard,
    path::{PathItem, PathItemIterator},
    state::WireState,
    vector::{Vec2f, Vec2u},
};

use super::gate::{calc_size_from_inputs, GateImpl, GateWireStates};

pub struct Xnor;

impl GateImpl for Xnor {
    fn id() -> &'static str {
        "xnor"
    }

    fn name() -> &'static str {
        "XNOR gate"
    }

    fn extra_toggle_name() -> Option<&'static str> {
        Some("Parity mode")
    }

    fn process(inputs: &[bool], parity: bool) -> bool {
        let count = inputs.iter().filter(|b| **b).count();
        match parity {
            false => count != 1,
            true => count % 2 != 1,
        }
    }

    fn draw(
        wires: GateWireStates,
        angle: f32,
        in_world_preview: bool,
        parity: bool,
        ctx: &crate::PaintContext,
    ) {
        let size: Vec2u = calc_size_from_inputs(wires.count() as u32).into();
        let size_f = size.convert(|v| v as f32);

        let width = size_f.x();
        let height = size_f.y();

        let transformer = |p: Pos2| {
            ctx.rect
                .lerp_inside(Vec2f::from(p).div(size_f).rotated_xy(angle, 0.5).into())
        };

        let straightness = (0.3 / (ctx.screen.scale.sqrt())).div(height).max(0.02);
        let bez_x = remap(width, 4.0..=5.0, 1.0..=1.2);

        let inner_bez = bezier_nd::Bezier::cubic(
            &Vec2f::from([0.22, 0.0]),
            &Vec2f::from([bez_x, (1.0 / 5.0) * height]),
            &Vec2f::from([bez_x, (4.0 / 5.0) * height]),
            &Vec2f::from([0.22, height]),
        );
        for line in inner_bez.as_lines(straightness * 2.0) {
            let start_y = line.0.y().floor() as usize;
            let end_y = line.1.y().ceil() as usize;

            for y in start_y..end_y {
                if wires.count() % 2 == 0 && wires.count() >> 1 == y {
                    continue;
                }

                let wire_index = if wires.count() % 2 == 0 && wires.count() >> 1 < y {
                    y - 1
                } else {
                    y
                };
                let y = y as f32 + 0.5;
                if line.0.y() > y || line.1.y() < y {
                    continue;
                }

                let x = remap(y, line.0.y()..=line.1.y(), line.0.x()..=line.1.x());
                let start = pos2(0.5, y);
                let end = pos2(x + 0.02, y);
                if start.x >= end.x {
                    continue;
                }
                ctx.paint.line_segment(
                    [transformer(start), transformer(end)],
                    Stroke::new(
                        ActiveCircuitBoard::WIRE_THICKNESS * ctx.screen.scale,
                        wires.get(wire_index, WireState::False).color(),
                    ),
                )
            }
        }

        let fill = [
            PathItem::MoveTo(pos2(width - 0.75, height / 2.0)),
            PathItem::QuadraticBezier(pos2((3.0 / 5.0) * width, 0.0), pos2(0.25, 0.0)),
            PathItem::CubicBezier(
                pos2(bez_x, (1.0 / 5.0) * height),
                pos2(bez_x, (4.0 / 5.0) * height),
                pos2(0.25, height),
            ),
            PathItem::QuadraticBezier(
                pos2((3.0 / 5.0) * width, height),
                pos2(width - 0.75, height / 2.0),
            ),
            PathItem::ClosePath,
        ];

        let outer = [
            PathItem::MoveTo(pos2(0.26, 0.0)),
            PathItem::QuadraticBezier(
                pos2((3.0 / 5.0) * width, 0.0),
                pos2(width - 0.75, height / 2.0),
            ),
            PathItem::QuadraticBezier(pos2((3.0 / 5.0) * width, height), pos2(0.26, height)),
        ];

        let inner = [
            PathItem::MoveTo(pos2(0.26, -0.025)),
            PathItem::LineTo(pos2(0.22, -0.025)),
            PathItem::CubicBezier(
                pos2(bez_x, (1.0 / 5.0) * height),
                pos2(bez_x, (4.0 / 5.0) * height),
                pos2(0.22, height + 0.025),
            ),
            PathItem::LineTo(pos2(0.26, height + 0.025)),
        ];

        let opacity = if in_world_preview { 0.6 } else { 1.0 };

        let border_color = Color32::BLACK.linear_multiply(opacity);
        let fill_color = Color32::from_gray(200).linear_multiply(opacity);

        fill.into_iter().create_path_shapes(
            fill_color,
            Stroke::NONE,
            straightness,
            transformer,
            |_, s| {
                ctx.paint.add(s);
            },
        );

        outer.into_iter().create_path_shapes(
            Color32::TRANSPARENT,
            Stroke::new(0.15 * ctx.screen.scale, border_color),
            straightness,
            transformer,
            |_, s| {
                ctx.paint.add(s);
            },
        );

        inner.into_iter().create_path_shapes(
            Color32::TRANSPARENT,
            Stroke::new(0.1 * ctx.screen.scale, border_color),
            straightness,
            transformer,
            |_, s| {
                ctx.paint.add(s);
            },
        );

        let circle_pos = transformer(pos2(width - 0.68, height / 2.0));
        ctx.paint.circle(
            circle_pos,
            0.2 * ctx.screen.scale,
            fill_color,
            Stroke::new(0.15 * ctx.screen.scale, border_color),
        );

        let arc_inner = [
            PathItem::MoveTo(pos2(-0.1, -0.025)),
            PathItem::CubicBezier(
                pos2(bez_x - 0.27, (1.0 / 5.0) * height),
                pos2(bez_x - 0.27, (4.0 / 5.0) * height),
                pos2(-0.1, height + 0.025),
            ),
        ];

        arc_inner.into_iter().create_path_shapes(
            Color32::TRANSPARENT,
            Stroke::new(0.1 * ctx.screen.scale, fill_color),
            straightness,
            transformer,
            |_, s| {
                ctx.paint.add(s);
            },
        );

        let arc_outer = [
            PathItem::MoveTo(pos2(0.0, -0.025)),
            PathItem::CubicBezier(
                pos2(bez_x - 0.22, (1.0 / 5.0) * height),
                pos2(bez_x - 0.22, (4.0 / 5.0) * height),
                pos2(0.0, height + 0.025),
            ),
            PathItem::LineTo(pos2(-0.2, height + 0.025)),
            PathItem::CubicBezier(
                pos2(bez_x - 0.32, (4.0 / 5.0) * height),
                pos2(bez_x - 0.32, (1.0 / 5.0) * height),
                pos2(-0.2, -0.025),
            ),
            PathItem::LineTo(pos2(0.0, -0.025)),
            PathItem::ClosePath,
        ];

        arc_outer.into_iter().create_path_shapes(
            Color32::TRANSPARENT,
            Stroke::new(0.08 * ctx.screen.scale, border_color),
            straightness,
            transformer,
            |_, s| {
                ctx.paint.add(s);
            },
        );

        if !parity {
            ctx.paint.text(
                transformer(pos2(width / 2.0, height / 2.0)),
                Align2::CENTER_CENTER,
                "!1",
                FontId::monospace(width / 3.0 * ctx.screen.scale),
                border_color,
            );
        }
    }
}