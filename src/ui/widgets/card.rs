// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Clean Card & Container widgets
use egui::{Color32, Frame, Margin, Stroke, Ui, Vec2};
use crate::ui::theme::ThemeTokens;
use crate::ui::primitives::paint_led_indicator;

pub struct ModernCard<'a> {
    title: Option<&'a str>,
    subtitle: Option<&'a str>,
    accent_color: Option<Color32>,
    indicator: bool,
}

impl<'a> ModernCard<'a> {
    pub fn new() -> Self {
        Self {
            title: None,
            subtitle: None,
            accent_color: None,
            indicator: true,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn accent(mut self, color: Color32) -> Self {
        self.accent_color = Some(color);
        self
    }

    pub fn show<R>(self, ui: &mut Ui, tokens: &ThemeTokens, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        let accent = self.accent_color.unwrap_or(tokens.primary);

        let frame = Frame::none()
            .fill(tokens.bg_card)
            .stroke(Stroke::new(1.0, tokens.border_subtle))
            .rounding(tokens.radius_md)
            .inner_margin(Margin::same(tokens.spacing_md));

        frame.show(ui, |ui| {
            ui.vertical(|ui| {
                if let Some(title) = self.title {
                    ui.horizontal(|ui| {
                        if self.indicator {
                            let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            paint_led_indicator(ui.painter(), dot_rect.center(), 3.5, accent, true);
                        }

                        ui.label(
                            egui::RichText::new(title)
                                .size(11.5)
                                .strong()
                                .color(accent),
                        );
                    });

                    if let Some(sub) = self.subtitle {
                        ui.label(
                            egui::RichText::new(sub)
                                .size(10.0)
                                .color(tokens.text_muted),
                        );
                    }

                    ui.add_space(8.0);
                }

                add_contents(ui)
            }).inner
        }).inner
    }
}
