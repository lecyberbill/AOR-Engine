// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: AppShell modern layout
use egui::{Context, Margin, RichText, Stroke, Ui};
use crate::ui::theme::{ThemeEngine, ThemePreset, ThemeTokens};

pub struct ModernAppShell<'a> {
    app_name: &'a str,
    version_tag: &'a str,
}

impl<'a> ModernAppShell<'a> {
    pub fn new(app_name: &'a str) -> Self {
        Self {
            app_name,
            version_tag: "v1.0",
        }
    }

    pub fn version(mut self, version: &'a str) -> Self {
        self.version_tag = version;
        self
    }

    /// Rendu de la barre supérieure (Top Bar) avec actions et sélecteur de thème rapide
    pub fn show_header<R>(
        &self,
        ctx: &Context,
        theme_engine: &mut ThemeEngine,
        add_header_actions: impl FnOnce(&mut Ui, &ThemeTokens) -> R,
    ) {
        let tokens = theme_engine.tokens.clone();
        let mut new_preset = None;

        egui::TopBottomPanel::top("app_header")
            .frame(
                egui::Frame::none()
                    .fill(tokens.bg_panel)
                    .stroke(Stroke::new(1.0, tokens.border_subtle))
                    .inner_margin(Margin::symmetric(tokens.spacing_lg, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(self.app_name)
                            .size(16.0)
                            .strong()
                            .color(tokens.primary),
                    );

                    ui.label(
                        RichText::new(self.version_tag)
                            .size(11.0)
                            .color(tokens.text_muted),
                    );

                    ui.add_space(20.0);

                    // Actions custom / onglets
                    add_header_actions(ui, &tokens);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Switcher de Thème rapide
                        for preset in ThemePreset::ALL.iter().rev() {
                            let is_active = theme_engine.active_preset == Some(*preset);
                            let color = if is_active { tokens.secondary } else { tokens.text_muted };

                            if ui.button(RichText::new(preset.name()).size(11.0).color(color)).clicked() {
                                new_preset = Some(*preset);
                            }
                        }

                        ui.label(RichText::new("THÈMES :").size(10.0).color(tokens.text_muted));
                    });
                });
            });

        if let Some(preset) = new_preset {
            theme_engine.set_preset(preset, ctx);
        }
    }
}
