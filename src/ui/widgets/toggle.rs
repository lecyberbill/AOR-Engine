// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Clean unused imports in widgets/toggle.rs
use egui::{Color32, Pos2, Response, Rounding, Sense, Stroke, Ui, Vec2};
use crate::ui::theme::ThemeTokens;

/// Switch / Bascule néon animée
pub fn modern_switch(
    ui: &mut Ui,
    value: &mut bool,
    tokens: &ThemeTokens,
) -> Response {
    let desired_size = Vec2::new(38.0, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        let track_color = if *value {
            tokens.secondary
        } else {
            Color32::from_rgb(32, 40, 60)
        };

        let bg_fill = if *value {
            Color32::from_rgb(14, 42, 56)
        } else {
            Color32::from_rgb(14, 18, 28)
        };

        painter.rect_filled(rect, Rounding::same(9.0), bg_fill);
        painter.rect_stroke(rect, Rounding::same(9.0), Stroke::new(1.0, track_color));

        let thumb_radius = 6.5;
        let thumb_x = if *value {
            rect.max.x - thumb_radius - 2.5
        } else {
            rect.min.x + thumb_radius + 2.5
        };
        let thumb_center = Pos2::new(thumb_x, rect.center().y);

        let thumb_color = if *value {
            tokens.secondary
        } else {
            Color32::from_rgb(100, 116, 139)
        };

        painter.circle_filled(thumb_center, thumb_radius, thumb_color);
    }

    response
}
