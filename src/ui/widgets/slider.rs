// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Slider matching Light and Dark themes
use egui::{Align2, Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2, Response, Sense};
use crate::ui::theme::ThemeTokens;

/// Slider Cyberpunk / Pro épuré et net
pub fn modern_slider(
    ui: &mut Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    tokens: &ThemeTokens,
) -> Response {
    let is_light = tokens.bg_app.r() > 180;
    let desired_size = Vec2::new(ui.available_width(), 16.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    let range = max - min;

    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let pct = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            *value = min + pct * range;
            response.mark_changed();
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let track_y = rect.center().y;

        // Piste d'arrière-plan
        let bg_track_color = if is_light { Color32::from_rgb(226, 232, 240) } else { Color32::from_rgb(32, 40, 64) };
        painter.line_segment(
            [Pos2::new(rect.left(), track_y), Pos2::new(rect.right(), track_y)],
            Stroke::new(4.0, bg_track_color),
        );

        // Piste active
        let normalized = ((*value - min) / range).clamp(0.0, 1.0);
        let fill_width = rect.width() * normalized;
        painter.line_segment(
            [Pos2::new(rect.left(), track_y), Pos2::new(rect.left() + fill_width, track_y)],
            Stroke::new(4.0, tokens.primary),
        );

        // Curseur
        let thumb_pos = Pos2::new(rect.left() + fill_width, track_y);
        painter.circle_filled(thumb_pos, 7.0, tokens.primary);
        painter.circle_filled(thumb_pos, 3.0, Color32::WHITE);
    }

    response
}
