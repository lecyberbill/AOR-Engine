// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Modal and Toast notifications
use egui::{Align2, Color32, Context, Frame, Margin, Rounding, Stroke, Ui};
use crate::ui::theme::ThemeTokens;

pub struct ModernModal<'a> {
    title: &'a str,
    accent: Option<Color32>,
}

impl<'a> ModernModal<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            accent: None,
        }
    }

    pub fn accent(mut self, color: Color32) -> Self {
        self.accent = Some(color);
        self
    }

    pub fn show<R>(
        self,
        ctx: &Context,
        tokens: &ThemeTokens,
        open: &mut bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        if !*open {
            return None;
        }

        let accent = self.accent.unwrap_or(tokens.primary);

        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("modal_backdrop")));
        // Backdrop noir translucide
        painter.rect_filled(screen_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 0, 0, 190));

        let mut res = None;

        egui::Window::new(self.title)
            .collapsible(false)
            .resizable(false)
            .pivot(Align2::CENTER_CENTER)
            .current_pos(screen_rect.center())
            .frame(
                Frame::none()
                    .fill(tokens.bg_panel)
                    .stroke(Stroke::new(1.5, accent))
                    .rounding(tokens.radius_lg)
                    .inner_margin(Margin::same(20.0)),
            )
            .show(ctx, |ui| {
                ui.set_max_width(450.0);
                res = Some(add_contents(ui));
            });

        res
    }
}
