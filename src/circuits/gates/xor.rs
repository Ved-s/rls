use std::ops::Div;

use eframe::{
    egui::{remap, Color32, Stroke},
    epaint::PathShape,
};

use crate::{
    circuits::CircuitRenderingContext,
    path::{Path, PointPath},
};

use super::{GateImpl, GateOutput};

#[derive(Clone)]
pub struct Xor;

impl GateImpl for Xor {
    fn id() -> &'static str {
        "gate_xor"
    }

    fn display_name() -> &'static str {
        "XOR gate"
    }

    fn init_state() -> bool {
        false
    }

    // TODO: =1 mode
    fn fold(state: &mut bool, input: bool) -> GateOutput {
        if input {
            *state = !*state;
        }

        GateOutput {
            out: *state,
            fin: false,
        }
    }

    #[rustfmt::skip]
    fn draw(ctx: &CircuitRenderingContext) {
        let size = ctx.world_size().convert(|v| v as f32);

        let border_color = Color32::BLACK;
        let fill_color = Color32::from_gray(200);
        let straightness = (0.3 / (ctx.paint.screen.scale.sqrt()))
            .div(size.y)
            .max(0.02);

        let bez_x = remap(size.x, 4.0..=5.0, 1.0..=1.2);

        let path = PointPath::new(size.x - 0.5, size.y / 2.0)
            .quadratic_bezier((3.0 / 5.0) * size.x, 0.0, 0.25, 0.0, straightness)
            .cubic_bezier(
                bez_x, (1.0 / 5.0) * size.y,
                bez_x, (4.0 / 5.0) * size.y,
                0.25, size.y,
                straightness,
            )
            .quadratic_bezier(
                (3.0 / 5.0) * size.x, size.y,
                size.x - 0.5, size.y / 2.0,
                straightness,
            );

        let points = path
            .iter_points(|v| ctx.transform_pos(v))
            .map(Into::into)
            .collect();

        let path = PathShape {
            points,
            closed: true,
            fill: fill_color,
            stroke: Stroke::new(0.15 * ctx.paint.screen.scale, border_color),
        };

        ctx.paint.painter.add(path);

        let arc_inner = PointPath::new(-0.2, -0.03)
            .line_to(-0.2, -0.025)
            .cubic_bezier(
                bez_x - 0.27, (1.0 / 5.0) * size.y, 
                bez_x - 0.27, (4.0 / 5.0) * size.y, 
                -0.2, size.y + 0.025,
                straightness
            )
            .line_to(-0.2, size.y + 0.03);

        let arc_outer = PointPath::new(-0.1, -0.025)
            .cubic_bezier(
                bez_x - 0.22, (1.0 / 5.0) * size.y,
                bez_x - 0.22, (4.0 / 5.0) * size.y,
                -0.1, size.y + 0.025, 
                straightness
            )
            .line_to(-0.3, size.y + 0.025)
            .cubic_bezier(
                bez_x - 0.32, (4.0 / 5.0) * size.y,
                bez_x - 0.32, (1.0 / 5.0) * size.y,
                -0.3, -0.025, 
                straightness
            )
            .line_to(-0.1, -0.025);

        let points_inner = arc_inner
            .iter_points(|v| ctx.transform_pos(v))
            .map(Into::into)
            .collect();

        let path_inner = PathShape {
            points: points_inner,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: Stroke::new(0.1 * ctx.paint.screen.scale, fill_color),
        };

        let points_outer = arc_outer
            .iter_points(|v| ctx.transform_pos(v))
            .map(Into::into)
            .collect();

        let path_outer = PathShape {
            points: points_outer,
            closed: true,
            fill: Color32::TRANSPARENT,
            stroke: Stroke::new(0.08 * ctx.paint.screen.scale, border_color),
        };

        ctx.paint.painter.add(path_inner);
        ctx.paint.painter.add(path_outer);
    }
}
