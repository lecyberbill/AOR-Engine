// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Status badges & indicators with high contrast
use egui::{Align2, Color32, Pos2, Rounding, Stroke, Ui, Vec2};
use crate::ui::theme::ThemeTokens;
use crate::ui::primitives::paint_led_indicator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant {
    Success,
    Warning,
    Danger,
    Info,
    Neutral,
}

pub struct ModernBadge<'a> {
    text: &'a str,
    variant: BadgeVariant,
    pulse: bool,
}

impl<'a> ModernBadge<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            variant: BadgeVariant::Info,
            pulse: true,
        }
    }

    pub fn success(text: &'a str) -> Self {
        Self::new(text).variant(BadgeVariant::Success)
    }

    pub fn warning(text: &'a str) -> Self {
        Self::new(text).variant(BadgeVariant::Warning)
    }

    pub fn danger(text: &'a str) -> Self {
        Self::new(text).variant(BadgeVariant::Danger)
    }

    pub fn info(text: &'a str) -> Self {
        Self::new(text).variant(BadgeVariant::Info)
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show(self, ui: &mut Ui, tokens: &ThemeTokens) {
        let accent = match self.variant {
            BadgeVariant::Success => tokens.success,
            BadgeVariant::Warning => tokens.warning,
            BadgeVariant::Danger => tokens.danger,
            BadgeVariant::Info => tokens.info,
            BadgeVariant::Neutral => tokens.text_muted,
        };

        let font_id = egui::FontId::proportional(11.0);
        let text_layout = ui.painter().layout_no_wrap(self.text.to_string(), font_id, accent);

        let padding_x = 10.0;
        let dot_size = if self.pulse { 12.0 } else { 0.0 };

        let total_width = text_layout.size().x + padding_x * 2.0 + dot_size;
        let total_height = 24.0;

        let (rect, _) = ui.allocate_exact_size(Vec2::new(total_width, total_height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Fond sombre translucide teinté (Dark pill) pour que le texte fluo ressorte parfaitement
            let bg_color = Color32::from_rgba_premultiplied(12, 18, 28, 220);
            let border_color = Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 120);

            painter.rect_filled(rect, Rounding::same(12.0), bg_color);
            painter.rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, border_color));

            let mut cur_x = rect.min.x + padding_x;
            if self.pulse {
                let dot_center = Pos2::new(cur_x + 3.0, rect.center().y);
                paint_led_indicator(painter, dot_center, 3.0, accent, true);
                cur_x += dot_size;
            }

            painter.text(
                Pos2::new(cur_x, rect.center().y),
                Align2::LEFT_CENTER,
                self.text,
                egui::FontId::proportional(11.0),
                accent,
            );
        }
    }
}
