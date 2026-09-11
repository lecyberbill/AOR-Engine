// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Typography engine definition
use egui::{FontFamily, TextStyle};

/// Configuration Typographique Dynamique
#[derive(Debug, Clone)]
pub struct TypographyConfig {
    /// Facteur d'échelle global pour agrandir/réduire tous les textes à la volée
    pub scale: f32,

    // Échelle de tailles (en points)
    pub size_xs: f32,
    pub size_sm: f32,
    pub size_base: f32,
    pub size_md: f32,
    pub size_lg: f32,
    pub size_xl: f32,
    pub size_h2: f32,
    pub size_h1: f32,

    // Famille de police principale
    pub font_family: FontFamily,
}

impl Default for TypographyConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            size_xs: 10.0,
            size_sm: 12.0,
            size_base: 14.0,
            size_md: 16.0,
            size_lg: 18.0,
            size_xl: 22.0,
            size_h2: 26.0,
            size_h1: 32.0,
            font_family: FontFamily::Proportional,
        }
    }
}

impl TypographyConfig {
    /// Applique les polices et l'échelle typographique au contexte egui
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        style.text_styles = [
            (TextStyle::Small, egui::FontId::new(self.size_xs * self.scale, self.font_family.clone())),
            (TextStyle::Body, egui::FontId::new(self.size_base * self.scale, self.font_family.clone())),
            (TextStyle::Button, egui::FontId::new(self.size_base * self.scale, self.font_family.clone())),
            (TextStyle::Heading, egui::FontId::new(self.size_xl * self.scale, self.font_family.clone())),
            (TextStyle::Monospace, egui::FontId::new(self.size_sm * self.scale, FontFamily::Monospace)),
        ]
        .into();

        ctx.set_style(style);
    }
}
