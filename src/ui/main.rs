// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Main showcase interactive application
use eframe::egui::{self, Color32, Margin, RichText, Stroke};
use rust_modern_ui::{
    modern_segmented_control, modern_slider, modern_switch,
    ModernAppShell, ModernBadge, ModernButton,
    ModernCard, ModernModal, ModernStatCard, ThemeEngine, ThemePreset,
};

#[derive(PartialEq, Clone)]
enum ShowcaseTab {
    Dashboard,
    Widgets,
    ThemeInspector,
}

#[derive(PartialEq, Clone)]
enum ViewMode {
    Grid,
    List,
    Graph,
}

struct ShowcaseApp {
    theme_engine: ThemeEngine,
    current_tab: ShowcaseTab,

    // Form demo states
    _text_input: String,
    _password_input: String,
    switch_state1: bool,
    switch_state2: bool,
    slider_value: f32,
    selected_view_mode: ViewMode,

    // Modal state
    show_modal: bool,

    // Live Custom Color Overrides
    custom_primary_hex: [u8; 3],
    custom_secondary_hex: [u8; 3],
    scale_factor: f32,
}

impl ShowcaseApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme_engine = ThemeEngine::new(ThemePreset::CyberNeon);
        theme_engine.apply(&cc.egui_ctx);

        Self {
            theme_engine,
            current_tab: ShowcaseTab::Dashboard,
            _text_input: "NEON_PROTOCOL_01".to_string(),
            _password_input: "secret_access_key".to_string(),
            switch_state1: true,
            switch_state2: false,
            slider_value: 78.5,
            selected_view_mode: ViewMode::Grid,
            show_modal: false,
            custom_primary_hex: [255, 42, 116],
            custom_secondary_hex: [0, 240, 255],
            scale_factor: 1.0,
        }
    }
}

impl eframe::App for ShowcaseApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let tokens = self.theme_engine.tokens.clone();

        // 1. HEADER & THEME PICKER
        let app_shell = ModernAppShell::new("MODERN_RUST_UI").version("v1.0.0-PRO");
        app_shell.show_header(ctx, &mut self.theme_engine, |ui, _t| {
            if ui.selectable_label(self.current_tab == ShowcaseTab::Dashboard, "DASHBOARD").clicked() {
                self.current_tab = ShowcaseTab::Dashboard;
            }
            if ui.selectable_label(self.current_tab == ShowcaseTab::Widgets, "COMPONENTS").clicked() {
                self.current_tab = ShowcaseTab::Widgets;
            }
            if ui.selectable_label(self.current_tab == ShowcaseTab::ThemeInspector, "THEME CUSTOMIZER").clicked() {
                self.current_tab = ShowcaseTab::ThemeInspector;
            }
        });

        // 2. SIDEBAR
        egui::SidePanel::left("left_sidebar")
            .frame(
                egui::Frame::none()
                    .fill(tokens.bg_panel)
                    .stroke(Stroke::new(1.0, tokens.border_subtle))
                    .inner_margin(Margin::same(tokens.spacing_md)),
            )
            .exact_width(220.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("SYSTEM STATUS").size(10.0).color(tokens.text_muted));
                    ui.add_space(4.0);

                    ModernBadge::success("ONLINE • SECURE").show(ui, &tokens);
                    ui.add_space(16.0);

                    ui.label(RichText::new("NAVIGATION").size(10.0).color(tokens.text_muted));
                    ui.add_space(6.0);

                    let nav_buttons = [
                        (ShowcaseTab::Dashboard, "Dashboard", "⚡"),
                        (ShowcaseTab::Widgets, "UI Components", "🎨"),
                        (ShowcaseTab::ThemeInspector, "Live Customizer", "⚙"),
                    ];

                    for (tab, label, icon) in nav_buttons {
                        let is_active = self.current_tab == tab;
                        let btn = if is_active {
                            ModernButton::secondary(label).icon(icon)
                        } else {
                            ModernButton::ghost(label).icon(icon)
                        };

                        if btn.show(ui, &tokens).clicked() {
                            self.current_tab = tab;
                        }
                        ui.add_space(4.0);
                    }

                    ui.add_space(20.0);
                    ui.label(RichText::new("QUICK ACTION").size(10.0).color(tokens.text_muted));
                    ui.add_space(6.0);

                    if ModernButton::primary("NEW TASK").icon("+").show(ui, &tokens).clicked() {
                        self.show_modal = true;
                    }
                });
            });

        // 3. MAIN CONTENT
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(tokens.bg_app).inner_margin(Margin::same(tokens.spacing_lg)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.current_tab {
                        ShowcaseTab::Dashboard => self.show_dashboard_view(ui, &tokens),
                        ShowcaseTab::Widgets => self.show_components_view(ui, &tokens),
                        ShowcaseTab::ThemeInspector => self.show_theme_customizer_view(ui, ctx),
                    }
                });
            });

        // 4. MODAL DEMO
        let mut close_modal = false;
        if self.show_modal {
            ModernModal::new("OVERRIDE SECURITY MATRIX")
                .accent(tokens.primary)
                .show(ctx, &tokens, &mut self.show_modal, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Êtes-vous sûr de vouloir réinitialiser les protocoles du sous-système ?").color(tokens.text_primary));
                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            if ModernButton::danger("CONFIRMER").width(120.0).show(ui, &tokens).clicked() {
                                close_modal = true;
                            }
                            if ModernButton::outline("ANNULER").width(120.0).show(ui, &tokens).clicked() {
                                close_modal = true;
                            }
                        });
                    });
                });
        }
        if close_modal {
            self.show_modal = false;
        }
    }
}

impl ShowcaseApp {
    fn show_dashboard_view(&mut self, ui: &mut egui::Ui, tokens: &rust_modern_ui::ThemeTokens) {
        ui.label(RichText::new("SYSTEM OVERVIEW").size(24.0).strong().color(tokens.text_primary));
        ui.label(RichText::new("Real-time telemetry and component visualizer").size(12.0).color(tokens.text_secondary));
        ui.add_space(16.0);

        // STATS ROW
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            ModernStatCard::new("CORE FREQUENCY", "4.82 GHz").trend("+12%", true).accent(tokens.primary).icon("⚡").show(ui, tokens);
            ModernStatCard::new("MEMORY LINK", "64.2 GB").trend("NOMINAL", true).accent(tokens.secondary).icon("💾").show(ui, tokens);
            ModernStatCard::new("NETWORK LATENCY", "1.4 ms").trend("-0.3ms", true).accent(tokens.success).icon("🌐").show(ui, tokens);
            ModernStatCard::new("SECURITY THREATS", "0 DETECTED").trend("CLEAN", true).accent(tokens.info).icon("🛡").show(ui, tokens);
        });

        ui.add_space(20.0);

        // TWO COLUMN CARDS
        ui.columns(2, |cols| {
            // COLONNE GAUCHE
            ModernCard::new()
                .title("ACTIVE MODULATORS")
                .subtitle("Real-time signal tuning")
                .accent(tokens.secondary)
                .show(&mut cols[0], tokens, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("NEURAL LINK:").size(12.0).color(tokens.text_secondary));
                        modern_switch(ui, &mut self.switch_state1, tokens);

                        ui.add_space(20.0);

                        ui.label(RichText::new("STEALTH MODE:").size(12.0).color(tokens.text_secondary));
                        modern_switch(ui, &mut self.switch_state2, tokens);
                    });

                    ui.add_space(16.0);
                    ui.label(RichText::new(format!("CORE OUTPUT: {:.1}%", self.slider_value)).size(12.0).color(tokens.text_secondary));
                    modern_slider(ui, &mut self.slider_value, 0.0, 100.0, tokens);
                });

            // COLONNE DROITE
            ModernCard::new()
                .title("VIEW CONTROLS & DISPATCH")
                .subtitle("Interactive view selector")
                .accent(tokens.primary)
                .show(&mut cols[1], tokens, |ui| {
                    ui.label(RichText::new("PROJECTION MODE").size(11.0).color(tokens.text_muted));
                    ui.add_space(4.0);

                    let options = [
                        (ViewMode::Grid, "GRID"),
                        (ViewMode::List, "LIST"),
                        (ViewMode::Graph, "GRAPH"),
                    ];
                    modern_segmented_control(ui, &mut self.selected_view_mode, &options, tokens);

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ModernButton::primary("EXECUTE").width(120.0).show(ui, tokens);
                        ModernButton::outline("RELOAD").width(100.0).show(ui, tokens);
                    });
                });
        });
    }

    fn show_components_view(&mut self, ui: &mut egui::Ui, tokens: &rust_modern_ui::ThemeTokens) {
        ui.label(RichText::new("COMPONENT GALLERY").size(24.0).strong().color(tokens.text_primary));
        ui.label(RichText::new("Collection of pure Rust modern widgets").size(12.0).color(tokens.text_secondary));
        ui.add_space(16.0);

        ui.columns(2, |cols| {
            // BOUTONS
            ModernCard::new().title("BUTTON VARIANTS").accent(tokens.primary).show(&mut cols[0], tokens, |ui| {
                ui.horizontal(|ui| {
                    ModernButton::primary("PRIMARY").width(110.0).show(ui, tokens);
                    ModernButton::secondary("SECONDARY").width(110.0).show(ui, tokens);
                    ModernButton::warning("WARNING").width(110.0).show(ui, tokens);
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ModernButton::danger("DANGER").width(110.0).show(ui, tokens);
                    ModernButton::outline("OUTLINE").width(110.0).show(ui, tokens);
                    ModernButton::ghost("GHOST").width(110.0).show(ui, tokens);
                });
            });

            // BADGES ET STATUS
            ModernCard::new().title("BADGES & INDICATORS").accent(tokens.secondary).show(&mut cols[1], tokens, |ui| {
                ui.horizontal(|ui| {
                    ModernBadge::success("ACTIVE").show(ui, tokens);
                    ModernBadge::warning("DEGRADED").show(ui, tokens);
                    ModernBadge::danger("OFFLINE").show(ui, tokens);
                    ModernBadge::info("SCANNING").show(ui, tokens);
                });
            });
        });
    }

    fn show_theme_customizer_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let tokens = self.theme_engine.tokens.clone();

        ui.label(RichText::new("LIVE THEME & TYPO CUSTOMIZER").size(24.0).strong().color(tokens.text_primary));
        ui.label(RichText::new("Ajustez instantanément les couleurs et la typographie").size(12.0).color(tokens.text_secondary));
        ui.add_space(16.0);

        ModernCard::new().title("DYNAMIC COLOR PALETTE").accent(tokens.secondary).show(ui, &tokens, |ui| {
            ui.horizontal(|ui| {
                ui.label("Couleur Primaire (RGB) : ");
                if ui.color_edit_button_srgb(&mut self.custom_primary_hex).changed() {
                    self.theme_engine.tokens.primary = Color32::from_rgb(
                        self.custom_primary_hex[0],
                        self.custom_primary_hex[1],
                        self.custom_primary_hex[2],
                    );
                    self.theme_engine.apply(ctx);
                }

                ui.add_space(20.0);

                ui.label("Couleur Secondaire (RGB) : ");
                if ui.color_edit_button_srgb(&mut self.custom_secondary_hex).changed() {
                    self.theme_engine.tokens.secondary = Color32::from_rgb(
                        self.custom_secondary_hex[0],
                        self.custom_secondary_hex[1],
                        self.custom_secondary_hex[2],
                    );
                    self.theme_engine.apply(ctx);
                }
            });
        });

        ui.add_space(16.0);

        ModernCard::new().title("TYPOGRAPHY SCALE").accent(tokens.primary).show(ui, &tokens, |ui| {
            ui.horizontal(|ui| {
                ui.label("Échelle Globale Typo : ");
                if ui.add(egui::Slider::new(&mut self.scale_factor, 0.8..=1.5).text("Zoom")).changed() {
                    self.theme_engine.typography.scale = self.scale_factor;
                    self.theme_engine.apply(ctx);
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Rust Modern UI Framework - Showcase"),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Modern UI",
        native_options,
        Box::new(|cc| Ok(Box::new(ShowcaseApp::new(cc)))),
    )
}
