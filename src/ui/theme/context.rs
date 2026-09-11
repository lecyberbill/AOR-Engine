// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Theme Context and runtime engine
use egui::{Context, Stroke, Style};
use super::tokens::ThemeTokens;
use super::typography::TypographyConfig;
use super::presets::ThemePreset;

/// Moteur de Thème dynamique gérant les tokens et la synchronisation avec le contexte egui
#[derive(Debug, Clone)]
pub struct ThemeEngine {
    pub tokens: ThemeTokens,
    pub typography: TypographyConfig,
    pub active_preset: Option<ThemePreset>,
}

impl Default for ThemeEngine {
    fn default() -> Self {
        Self {
            tokens: ThemeTokens::tokyo_night(),
            typography: TypographyConfig::default(),
            active_preset: Some(ThemePreset::TokyoNight),
        }
    }
}

impl ThemeEngine {
    pub fn new(preset: ThemePreset) -> Self {
        Self {
            tokens: preset.to_tokens(),
            typography: TypographyConfig::default(),
            active_preset: Some(preset),
        }
    }

    pub fn set_preset(&mut self, preset: ThemePreset, ctx: &Context) {
        self.active_preset = Some(preset);
        self.tokens = preset.to_tokens();
        self.apply(ctx);
    }

    /// Synchronise l'ensemble des styles egui avec les tokens et la typographie
    pub fn apply(&self, ctx: &Context) {
        let mut style: Style = (*ctx.style()).clone();

        // 1. Visuals globaux
        style.visuals.dark_mode = self.tokens.bg_app.r() < 128;
        style.visuals.panel_fill = self.tokens.bg_panel;
        style.visuals.window_fill = self.tokens.bg_panel;
        style.visuals.extreme_bg_color = self.tokens.bg_app;
        style.visuals.override_text_color = Some(self.tokens.text_primary);

        style.visuals.window_rounding = self.tokens.radius_md;
        style.visuals.window_stroke = Stroke::new(1.0, self.tokens.border_subtle);
        style.visuals.window_shadow.blur = self.tokens.shadow_blur;
        style.visuals.window_shadow.color = self.tokens.shadow_color;

        // 2. Widgets Non-interactifs
        let noninteractive = &mut style.visuals.widgets.noninteractive;
        noninteractive.bg_fill = self.tokens.bg_card;
        noninteractive.bg_stroke = Stroke::new(1.0, self.tokens.border_subtle);
        noninteractive.rounding = self.tokens.radius_sm;
        noninteractive.fg_stroke = Stroke::new(1.0, self.tokens.text_primary);

        // 3. Widgets Inactifs
        let inactive = &mut style.visuals.widgets.inactive;
        inactive.bg_fill = self.tokens.bg_card;
        inactive.bg_stroke = Stroke::new(1.0, self.tokens.border_subtle);
        inactive.rounding = self.tokens.radius_sm;
        inactive.fg_stroke = Stroke::new(1.0, self.tokens.text_primary);

        // 4. Widgets Survolés (Hovered)
        let hovered = &mut style.visuals.widgets.hovered;
        hovered.bg_fill = self.tokens.bg_card_hover;
        hovered.bg_stroke = Stroke::new(1.0, self.tokens.primary_hover);
        hovered.rounding = self.tokens.radius_sm;
        hovered.fg_stroke = Stroke::new(1.0, self.tokens.text_primary);

        // 5. Widgets Actifs / Cliqués
        let active = &mut style.visuals.widgets.active;
        active.bg_fill = self.tokens.primary_active;
        active.bg_stroke = Stroke::new(1.5, self.tokens.primary);
        active.rounding = self.tokens.radius_sm;
        active.fg_stroke = Stroke::new(1.0, self.tokens.text_inverse);

        // 6. Espacements
        style.spacing.item_spacing = self.tokens.item_spacing;
        style.spacing.button_padding = egui::Vec2::new(12.0, 8.0);

        ctx.set_style(style);

        // 7. Appliquer la typographie
        self.typography.apply_to_ctx(ctx);
    }
}
