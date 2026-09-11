// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Custom shapes & accents
use egui::{Color32, Painter, Pos2, Rect, Stroke};

/// Dessine un séparateur horizontal futuriste avec dégradé central
pub fn paint_divider(
    painter: &Painter,
    rect: Rect,
    color: Color32,
) {
    let center_y = rect.center().y;
    let p_start = Pos2::new(rect.min.x, center_y);
    let p_end = Pos2::new(rect.max.x, center_y);

    painter.line_segment([p_start, p_end], Stroke::new(1.0, color));
}

/// Dessine un coin biseauté cyber / tech
pub fn paint_tech_corner(
    painter: &Painter,
    rect: Rect,
    corner_size: f32,
    color: Color32,
) {
    let top_left = rect.min;
    let stroke = Stroke::new(1.5, color);

    // Coin Supérieur Gauche
    painter.line_segment([top_left, Pos2::new(top_left.x + corner_size, top_left.y)], stroke);
    painter.line_segment([top_left, Pos2::new(top_left.x, top_left.y + corner_size)], stroke);

    // Coin Inférieur Droit
    let bottom_right = rect.max;
    painter.line_segment([bottom_right, Pos2::new(bottom_right.x - corner_size, bottom_right.y)], stroke);
    painter.line_segment([bottom_right, Pos2::new(bottom_right.x, bottom_right.y - corner_size)], stroke);
}
