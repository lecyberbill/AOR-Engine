// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Button widgets handling Light & Dark mode seamlessly
use egui::{Align2, Color32, Pos2, Response, Sense, Stroke, Ui, Vec2};
use crate::ui::theme::ThemeTokens;
use crate::ui::primitives::paint_glow_rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModernButtonVariant {
    Primary,
    Secondary,
    Warning,
    Danger,
    Ghost,
    Outline,
}

pub struct ModernButton<'a> {
    text: &'a str,
    variant: ModernButtonVariant,
    width: Option<f32>,
    height: f32,
    icon: Option<&'a str>,
    glow: bool,
    disabled: bool,
}

impl<'a> ModernButton<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            variant: ModernButtonVariant::Primary,
            width: None,
            height: 36.0,
            icon: None,
            glow: true,
            disabled: false,
        }
    }

    pub fn primary(text: &'a str) -> Self {
        Self::new(text).variant(ModernButtonVariant::Primary)
    }

    pub fn secondary(text: &'a str) -> Self {
        Self::new(text).variant(ModernButtonVariant::Secondary)
    }

    pub fn warning(text: &'a str) -> Self {
        Self::new(text).variant(ModernButtonVariant::Warning)
    }

    pub fn danger(text: &'a str) -> Self {
        Self::new(text).variant(ModernButtonVariant::Danger)
    }

    pub fn ghost(text: &'a str) -> Self {
        Self::new(text).variant(ModernButtonVariant::Ghost).glow(false)
    }

    pub fn outline(text: &'a str) -> Self {
        Self::new(text).variant(ModernButtonVariant::Outline).glow(false)
    }

    pub fn variant(mut self, variant: ModernButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn glow(mut self, glow: bool) -> Self {
        self.glow = glow;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.width = Some(f32::INFINITY);
        self
    }

    pub fn show(self, ui: &mut Ui, tokens: &ThemeTokens) -> Response {
        let is_light_theme = tokens.bg_app.r() > 180;
        let desired_width = match self.width {
            Some(w) if w == f32::INFINITY => ui.available_width(),
            Some(w) => w,
            None => {
                let char_count = self.text.chars().count() as f32;
                (char_count * 8.0 + 26.0).max(64.0)
            }
        };
        let desired_size = Vec2::new(desired_width, self.height);

        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click()
        };

        let (rect, response) = ui.allocate_exact_size(desired_size, sense);

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let is_hovered = response.hovered() && !self.disabled;
            let is_clicked = response.is_pointer_button_down_on() && !self.disabled;

            let (accent_color, bg_color, border_stroke, text_color) = if self.disabled {
                (
                    tokens.text_muted,
                    if is_light_theme { Color32::from_rgb(241, 245, 249) } else { Color32::from_rgb(18, 22, 34) },
                    Stroke::new(1.0, tokens.border_subtle),
                    tokens.text_muted,
                )
            } else if is_light_theme {
                // Rendu Élégant & Moderne pour Thème Clair
                match self.variant {
                    ModernButtonVariant::Primary => {
                        let accent = tokens.primary;
                        if is_clicked {
                            (accent, tokens.primary_active, Stroke::NONE, Color32::WHITE)
                        } else if is_hovered {
                            (accent, tokens.primary_hover, Stroke::NONE, Color32::WHITE)
                        } else {
                            (accent, accent, Stroke::NONE, Color32::WHITE)
                        }
                    }
                    ModernButtonVariant::Secondary => {
                        let accent = tokens.secondary;
                        if is_clicked {
                            (accent, Color32::from_rgb(224, 242, 254), Stroke::new(1.5, accent), accent)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(240, 249, 255), Stroke::new(1.2, accent), accent)
                        } else {
                            (accent, Color32::WHITE, Stroke::new(1.0, tokens.border_strong), tokens.text_primary)
                        }
                    }
                    ModernButtonVariant::Warning => {
                        let accent = tokens.warning;
                        if is_clicked {
                            (accent, Color32::from_rgb(180, 83, 9), Stroke::NONE, Color32::WHITE)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(245, 158, 11), Stroke::NONE, Color32::WHITE)
                        } else {
                            (accent, accent, Stroke::NONE, Color32::WHITE)
                        }
                    }
                    ModernButtonVariant::Danger => {
                        let accent = tokens.danger;
                        if is_clicked {
                            (accent, Color32::from_rgb(190, 18, 60), Stroke::NONE, Color32::WHITE)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(244, 63, 94), Stroke::NONE, Color32::WHITE)
                        } else {
                            (accent, accent, Stroke::NONE, Color32::WHITE)
                        }
                    }
                    ModernButtonVariant::Outline => {
                        if is_clicked {
                            (tokens.secondary, Color32::from_rgb(241, 245, 249), Stroke::new(1.5, tokens.primary), tokens.primary)
                        } else if is_hovered {
                            (tokens.secondary, Color32::from_rgb(248, 250, 252), Stroke::new(1.2, tokens.primary), tokens.primary)
                        } else {
                            (tokens.secondary, Color32::WHITE, Stroke::new(1.0, tokens.border_strong), tokens.text_primary)
                        }
                    }
                    ModernButtonVariant::Ghost => {
                        if is_hovered {
                            (tokens.text_secondary, Color32::from_rgb(241, 245, 249), Stroke::NONE, tokens.text_primary)
                        } else {
                            (tokens.text_secondary, Color32::TRANSPARENT, Stroke::NONE, tokens.text_secondary)
                        }
                    }
                }
            } else {
                // Rendu Dark / Cyber / Neon
                match self.variant {
                    ModernButtonVariant::Primary => {
                        let accent = tokens.primary;
                        if is_clicked {
                            (accent, tokens.primary_active, Stroke::new(1.5, accent), Color32::WHITE)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(60, 24, 42), Stroke::new(1.5, tokens.primary_hover), Color32::WHITE)
                        } else {
                            (accent, Color32::from_rgb(38, 18, 30), Stroke::new(1.0, Color32::from_rgb(160, 36, 76)), accent)
                        }
                    }
                    ModernButtonVariant::Secondary => {
                        let accent = tokens.secondary;
                        if is_clicked {
                            (accent, tokens.secondary_hover, Stroke::new(1.5, accent), Color32::from_rgb(13, 15, 24))
                        } else if is_hovered {
                            (accent, Color32::from_rgb(22, 48, 76), Stroke::new(1.5, tokens.secondary_hover), Color32::WHITE)
                        } else {
                            (accent, Color32::from_rgb(16, 32, 54), Stroke::new(1.0, Color32::from_rgb(30, 80, 140)), tokens.text_primary)
                        }
                    }
                    ModernButtonVariant::Warning => {
                        let accent = tokens.warning;
                        if is_clicked {
                            (accent, accent, Stroke::new(1.5, accent), Color32::BLACK)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(56, 44, 20), Stroke::new(1.5, accent), Color32::WHITE)
                        } else {
                            (accent, Color32::from_rgb(38, 30, 16), Stroke::new(1.0, Color32::from_rgb(160, 120, 28)), accent)
                        }
                    }
                    ModernButtonVariant::Danger => {
                        let accent = tokens.danger;
                        if is_clicked {
                            (accent, accent, Stroke::new(1.5, accent), Color32::WHITE)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(58, 20, 24), Stroke::new(1.5, accent), Color32::WHITE)
                        } else {
                            (accent, Color32::from_rgb(38, 16, 18), Stroke::new(1.0, Color32::from_rgb(160, 36, 42)), accent)
                        }
                    }
                    ModernButtonVariant::Outline => {
                        let accent = tokens.secondary;
                        if is_clicked {
                            (accent, Color32::from_rgba_premultiplied(122, 162, 247, 40), Stroke::new(1.5, accent), Color32::WHITE)
                        } else if is_hovered {
                            (accent, Color32::from_rgb(22, 32, 52), Stroke::new(1.5, accent), Color32::WHITE)
                        } else {
                            (accent, Color32::from_rgb(16, 20, 34), Stroke::new(1.0, tokens.border_strong), tokens.text_primary)
                        }
                    }
                    ModernButtonVariant::Ghost => {
                        if is_hovered {
                            (tokens.text_secondary, Color32::from_rgb(28, 36, 60), Stroke::NONE, Color32::WHITE)
                        } else {
                            (tokens.text_secondary, Color32::TRANSPARENT, Stroke::NONE, tokens.text_secondary)
                        }
                    }
                }
            };

            // Halo GPU doux pour le thème sombre
            if self.glow && !self.disabled && !is_light_theme && self.variant != ModernButtonVariant::Outline && self.variant != ModernButtonVariant::Ghost {
                let glow_alpha = if is_clicked { 140 } else if is_hovered { 100 } else { 35 };
                let glow_color = Color32::from_rgba_premultiplied(accent_color.r(), accent_color.g(), accent_color.b(), glow_alpha);
                let blur = if is_hovered { 14.0 } else { 8.0 };
                paint_glow_rect(painter, rect, tokens.radius_sm, glow_color, blur);
            }

            painter.rect_filled(rect, tokens.radius_sm, bg_color);
            if border_stroke != Stroke::NONE {
                painter.rect_stroke(rect, tokens.radius_sm, border_stroke);
            }

            let full_label = if let Some(icon) = self.icon {
                format!("{}  {}", icon, self.text)
            } else {
                self.text.to_string()
            };

            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                full_label,
                egui::FontId::proportional(12.0),
                text_color,
            );
        }

        response
    }
}
