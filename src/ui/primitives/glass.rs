// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Glassmorphism primitives
use egui::{Color32, Frame, Margin, Painter, Pos2, Rect, Rounding, Stroke};
use crate::ui::theme::ThemeTokens;

/// Dessine un rectangle avec effet de verre dépoli / Glassmorphism
pub fn paint_glass_panel(
    painter: &Painter,
    rect: Rect,
    rounding: Rounding,
    bg_tint: Color32,
    border_color: Color32,
) {
    // 1. Fond semi-transparent
    painter.rect_filled(rect, rounding, bg_tint);

    // 2. Bordure avec liseré supérieur lumineux (effet de réflexion de lumière)
    painter.rect_stroke(rect, rounding, Stroke::new(1.0, border_color));

    // Ligne de reflet supérieur très discret
    if rect.width() > 10.0 {
        let top_left = Pos2::new(rect.min.x + 2.0, rect.min.y + 1.0);
        let top_right = Pos2::new(rect.max.x - 2.0, rect.min.y + 1.0);
        let highlight_color = Color32::from_rgba_premultiplied(255, 255, 255, 30);
        painter.line_segment([top_left, top_right], Stroke::new(1.0, highlight_color));
    }
}

/// Helper Frame pour encapsuler du contenu dans un panneau glassmorphism
pub fn glass_frame(tokens: &ThemeTokens) -> Frame {
    Frame::none()
        .fill(tokens.bg_glass_tint)
        .stroke(Stroke::new(1.0, tokens.border_subtle))
        .rounding(tokens.radius_md)
        .inner_margin(Margin::same(tokens.spacing_md))
}
