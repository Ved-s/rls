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
pub struct Or;

impl GateImpl for Or {
    fn id() -> &'static str {
        "gate_or"
    }

    fn display_name() -> &'static str {
        "OR gate"
    }

    fn init_state() -> bool {
        false
    }

    fn fold(_: &mut bool, input: bool) -> GateOutput {
        if input {
            GateOutput {
                out: true,
                fin: true,
            }
        } else {
            GateOutput {
                out: false,
                fin: false,
            }
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
    }
}
