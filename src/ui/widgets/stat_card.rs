// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: StatCard metrics widget
use egui::{Color32, Margin, Stroke, Ui};
use crate::ui::theme::ThemeTokens;

pub struct ModernStatCard<'a> {
    title: &'a str,
    value: &'a str,
    trend: Option<&'a str>,
    is_positive: bool,
    accent: Option<Color32>,
    icon: Option<&'a str>,
}

impl<'a> ModernStatCard<'a> {
    pub fn new(title: &'a str, value: &'a str) -> Self {
        Self {
            title,
            value,
            trend: None,
            is_positive: true,
            accent: None,
            icon: None,
        }
    }

    pub fn trend(mut self, trend: &'a str, positive: bool) -> Self {
        self.trend = Some(trend);
        self.is_positive = positive;
        self
    }

    pub fn accent(mut self, color: Color32) -> Self {
        self.accent = Some(color);
        self
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn show(self, ui: &mut Ui, tokens: &ThemeTokens) {
        let accent_color = self.accent.unwrap_or(tokens.primary);

        let frame = egui::Frame::none()
            .fill(tokens.bg_card)
            .stroke(Stroke::new(1.0, tokens.border_subtle))
            .rounding(tokens.radius_md)
            .inner_margin(Margin::same(14.0));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if let Some(icon) = self.icon {
                        ui.label(egui::RichText::new(icon).color(accent_color).size(12.0));
                    }
                    ui.label(
                        egui::RichText::new(self.title)
                            .size(11.0)
                            .color(tokens.text_muted)
                            .strong(),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(self.value)
                            .size(20.0)
                            .strong()
                            .color(tokens.text_primary),
                    );

                    if let Some(trend) = self.trend {
                        let trend_color = if self.is_positive { tokens.success } else { tokens.danger };
                        let prefix = if self.is_positive { "↑ " } else { "↓ " };
                        ui.label(
                            egui::RichText::new(format!("{}{}", prefix, trend))
                                .size(11.0)
                                .color(trend_color),
                        );
                    }
                });
            });
        });
    }
}
