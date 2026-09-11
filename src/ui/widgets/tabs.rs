// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Segmented controls with full Light and Dark theme support
use egui::{Align2, Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use crate::ui::theme::ThemeTokens;

/// Segmented Control moderne (Pill selector comme Grid / List / Map)
pub fn modern_segmented_control<T: PartialEq + Clone>(
    ui: &mut Ui,
    selected: &mut T,
    options: &[(T, &str)],
    tokens: &ThemeTokens,
) -> bool {
    let is_light = tokens.bg_app.r() > 180;
    let mut changed = false;
    let count = options.len();
    if count == 0 {
        return false;
    }

    let desired_size = Vec2::new(ui.available_width(), 32.0);
    let (container_rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(container_rect) {
        let painter = ui.painter();

        // Conteneur de fond
        let bg_fill = if is_light { Color32::from_rgb(241, 245, 249) } else { Color32::from_rgb(12, 16, 26) };
        let border_stroke = Stroke::new(1.0, tokens.border_subtle);

        painter.rect_filled(container_rect, Rounding::same(6.0), bg_fill);
        painter.rect_stroke(container_rect, Rounding::same(6.0), border_stroke);

        let segment_width = container_rect.width() / (count as f32);

        for (i, (val, label)) in options.iter().enumerate() {
            let seg_min = egui::pos2(container_rect.min.x + (i as f32) * segment_width, container_rect.min.y);
            let seg_rect = Rect::from_min_size(seg_min, Vec2::new(segment_width, container_rect.height()));

            let seg_id = ui.id().with(i).with(label);
            let seg_response = ui.interact(seg_rect, seg_id, egui::Sense::click());

            let is_selected = *selected == *val;

            if seg_response.clicked() && !is_selected {
                *selected = val.clone();
                changed = true;
            }

            if is_selected {
                let active_rect = seg_rect.shrink(2.0);
                if is_light {
                    painter.rect_filled(active_rect, Rounding::same(5.0), Color32::WHITE);
                    painter.rect_stroke(active_rect, Rounding::same(5.0), Stroke::new(1.0, tokens.border_strong));
                } else {
                    painter.rect_filled(active_rect, Rounding::same(4.0), Color32::from_rgb(14, 38, 52));
                    painter.rect_stroke(active_rect, Rounding::same(4.0), Stroke::new(1.2, tokens.secondary));
                }
            } else if seg_response.hovered() {
                let hover_rect = seg_rect.shrink(2.0);
                let hover_bg = if is_light { Color32::from_rgb(226, 232, 240) } else { Color32::from_rgb(20, 28, 44) };
                painter.rect_filled(hover_rect, Rounding::same(4.0), hover_bg);
            }

            let text_color = if is_selected {
                if is_light { tokens.primary } else { tokens.secondary }
            } else if seg_response.hovered() {
                tokens.text_primary
            } else {
                tokens.text_muted
            };

            painter.text(
                seg_rect.center(),
                Align2::CENTER_CENTER,
                *label,
                egui::FontId::proportional(11.5),
                text_color,
            );
        }
    }

    changed
}
