// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Premium Web-inspired theme presets (Catppuccin, Nord, Tokyo Night, Tailwind Slate)
use egui::{Color32, Rounding, Stroke, Vec2};

/// Design Tokens System with comprehensive web-standard color palettes
#[derive(Debug, Clone)]
pub struct ThemeTokens {
    // --- Surface & Background Colors ---
    pub bg_app: Color32,
    pub bg_panel: Color32,
    pub bg_card: Color32,
    pub bg_card_hover: Color32,
    pub bg_input: Color32,
    pub bg_glass_tint: Color32,

    // --- Action & Accent Colors ---
    pub primary: Color32,
    pub primary_hover: Color32,
    pub primary_active: Color32,
    pub secondary: Color32,
    pub secondary_hover: Color32,
    pub accent: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub info: Color32,

    // --- Text & Foreground Colors ---
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_inverse: Color32,

    // --- Borders & Outlines ---
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub border_accent: Color32,

    // --- Glows, Shadows & Light Effects ---
    pub glow_color: Color32,
    pub glow_radius: f32,
    pub shadow_blur: f32,
    pub shadow_color: Color32,

    // --- Geometry, Radii & Spacing ---
    pub radius_sm: Rounding,
    pub radius_md: Rounding,
    pub radius_lg: Rounding,
    pub radius_pill: Rounding,

    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub spacing_lg: f32,
    pub spacing_xl: f32,

    pub item_spacing: Vec2,
    pub card_padding: Vec2,
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::tokyo_night()
    }
}

impl ThemeTokens {
    /// 🌸 Tokyo Night (Cyber Neon) - Deep navy blue with neon magenta and cyan
    pub fn tokyo_night() -> Self {
        Self {
            bg_app: Color32::from_rgb(13, 15, 24),          // #0d0f18
            bg_panel: Color32::from_rgb(18, 21, 33),        // #121521
            bg_card: Color32::from_rgb(25, 30, 48),         // #191e30
            bg_card_hover: Color32::from_rgb(34, 40, 64),   // #222840
            bg_input: Color32::from_rgb(15, 18, 28),
            bg_glass_tint: Color32::from_rgba_premultiplied(25, 30, 48, 210),

            primary: Color32::from_rgb(247, 118, 142),      // Tokyo Pink #f7768e
            primary_hover: Color32::from_rgb(255, 140, 160),
            primary_active: Color32::from_rgb(215, 80, 110),

            secondary: Color32::from_rgb(122, 162, 247),    // Tokyo Blue #7aa2f7
            secondary_hover: Color32::from_rgb(145, 180, 255),

            accent: Color32::from_rgb(187, 154, 247),      // Tokyo Purple #bb9af7
            warning: Color32::from_rgb(224, 175, 104),     // Tokyo Yellow #e0af68
            danger: Color32::from_rgb(247, 118, 142),       // Tokyo Red
            success: Color32::from_rgb(158, 206, 106),      // Tokyo Green #9ece6a
            info: Color32::from_rgb(125, 207, 255),        // Tokyo Cyan #7dcfff

            text_primary: Color32::from_rgb(240, 243, 252), // Blanc doux hyper net
            text_secondary: Color32::from_rgb(192, 202, 245),// Bleu pastel très lisible
            text_muted: Color32::from_rgb(132, 144, 186),   // Gris bleuté clair (lisible)
            text_inverse: Color32::from_rgb(13, 15, 24),

            border_subtle: Color32::from_rgb(38, 45, 72),
            border_strong: Color32::from_rgb(58, 68, 106),
            border_accent: Color32::from_rgb(122, 162, 247),

            glow_color: Color32::from_rgba_premultiplied(122, 162, 247, 70),
            glow_radius: 10.0,
            shadow_blur: 24.0,
            shadow_color: Color32::from_rgba_premultiplied(0, 0, 0, 180),

            radius_sm: Rounding::same(6.0),
            radius_md: Rounding::same(10.0),
            radius_lg: Rounding::same(14.0),
            radius_pill: Rounding::same(999.0),

            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,

            item_spacing: Vec2::new(10.0, 10.0),
            card_padding: Vec2::new(16.0, 16.0),
        }
    }

    /// 🌿 Catppuccin Mocha - The most beloved developer pastel dark theme
    pub fn catppuccin_mocha() -> Self {
        Self {
            bg_app: Color32::from_rgb(30, 30, 46),          // Base #1e1e2e
            bg_panel: Color32::from_rgb(24, 24, 37),        // Crust #181825
            bg_card: Color32::from_rgb(49, 50, 68),         // Surface0 #313244
            bg_card_hover: Color32::from_rgb(69, 71, 90),   // Surface1 #45475a
            bg_input: Color32::from_rgb(24, 24, 37),        // Mantle
            bg_glass_tint: Color32::from_rgba_premultiplied(49, 50, 68, 190),

            primary: Color32::from_rgb(203, 166, 247),      // Mauve #cba6f7
            primary_hover: Color32::from_rgb(220, 190, 255),
            primary_active: Color32::from_rgb(180, 140, 230),

            secondary: Color32::from_rgb(137, 220, 235),    // Sky #89dceb
            secondary_hover: Color32::from_rgb(160, 235, 245),

            accent: Color32::from_rgb(245, 194, 231),       // Pink #f5c2e7
            warning: Color32::from_rgb(249, 226, 175),      // Yellow #f9e2af
            danger: Color32::from_rgb(243, 139, 168),       // Red #f38ba8
            success: Color32::from_rgb(166, 227, 161),      // Green #a6e3a1
            info: Color32::from_rgb(137, 180, 250),         // Blue #89b4fa

            text_primary: Color32::from_rgb(205, 214, 244), // Text #cdd6f4
            text_secondary: Color32::from_rgb(186, 194, 222),// Subtext1 #bac2de
            text_muted: Color32::from_rgb(108, 112, 134),   // Overlay0 #6c7086
            text_inverse: Color32::from_rgb(17, 17, 27),

            border_subtle: Color32::from_rgb(69, 71, 90),   // Surface1
            border_strong: Color32::from_rgb(88, 91, 112),   // Surface2
            border_accent: Color32::from_rgb(203, 166, 247),

            glow_color: Color32::from_rgba_premultiplied(203, 166, 247, 50),
            glow_radius: 8.0,
            shadow_blur: 20.0,
            shadow_color: Color32::from_rgba_premultiplied(0, 0, 0, 160),

            radius_sm: Rounding::same(6.0),
            radius_md: Rounding::same(10.0),
            radius_lg: Rounding::same(14.0),
            radius_pill: Rounding::same(999.0),

            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,

            item_spacing: Vec2::new(10.0, 10.0),
            card_padding: Vec2::new(16.0, 16.0),
        }
    }

    /// ❄️ Nord (Arctic Blue) - Legendary Scandinavian dark arctic palette
    pub fn nord_dark() -> Self {
        Self {
            bg_app: Color32::from_rgb(46, 52, 64),          // Nord0 #2e3440
            bg_panel: Color32::from_rgb(36, 41, 51),        // Darker Polar
            bg_card: Color32::from_rgb(59, 66, 82),          // Nord1 #3b4252
            bg_card_hover: Color32::from_rgb(67, 76, 94),   // Nord2 #434c5e
            bg_input: Color32::from_rgb(36, 41, 51),
            bg_glass_tint: Color32::from_rgba_premultiplied(59, 66, 82, 190),

            primary: Color32::from_rgb(136, 192, 208),      // Nord8 Frost Cyan #88c0d0
            primary_hover: Color32::from_rgb(155, 210, 225),
            primary_active: Color32::from_rgb(115, 170, 190),

            secondary: Color32::from_rgb(129, 161, 193),    // Nord9 Frost Blue #81a1c1
            secondary_hover: Color32::from_rgb(148, 180, 212),

            accent: Color32::from_rgb(180, 142, 173),       // Nord15 Aurora Purple #b48ead
            warning: Color32::from_rgb(235, 203, 139),      // Nord13 Aurora Yellow #ebcb8b
            danger: Color32::from_rgb(191, 97, 106),        // Nord11 Aurora Red #bf616a
            success: Color32::from_rgb(163, 190, 140),      // Nord14 Aurora Green #a3be8c
            info: Color32::from_rgb(94, 129, 172),          // Nord10 Frost Dark #5e81ac

            text_primary: Color32::from_rgb(236, 239, 244), // Nord6 Snow White #eceff4
            text_secondary: Color32::from_rgb(216, 222, 233),// Nord4 #d8dee9
            text_muted: Color32::from_rgb(118, 128, 148),
            text_inverse: Color32::from_rgb(46, 52, 64),

            border_subtle: Color32::from_rgb(76, 86, 106),  // Nord3
            border_strong: Color32::from_rgb(95, 106, 130),
            border_accent: Color32::from_rgb(136, 192, 208),

            glow_color: Color32::from_rgba_premultiplied(136, 192, 208, 45),
            glow_radius: 8.0,
            shadow_blur: 20.0,
            shadow_color: Color32::from_rgba_premultiplied(0, 0, 0, 160),

            radius_sm: Rounding::same(4.0),
            radius_md: Rounding::same(8.0),
            radius_lg: Rounding::same(12.0),
            radius_pill: Rounding::same(999.0),

            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,

            item_spacing: Vec2::new(10.0, 10.0),
            card_padding: Vec2::new(16.0, 16.0),
        }
    }

    /// ☀️ Tailwind Slate (Modern Light) - State of the art modern web light theme
    pub fn tailwind_slate_light() -> Self {
        Self {
            bg_app: Color32::from_rgb(248, 250, 252),         // Slate 50 #f8fafc
            bg_panel: Color32::from_rgb(255, 255, 255),       // Pure White
            bg_card: Color32::from_rgb(255, 255, 255),
            bg_card_hover: Color32::from_rgb(241, 245, 249),  // Slate 100
            bg_input: Color32::from_rgb(241, 245, 249),
            bg_glass_tint: Color32::from_rgba_premultiplied(255, 255, 255, 230),

            primary: Color32::from_rgb(79, 70, 229),          // Indigo 600 #4f46e5
            primary_hover: Color32::from_rgb(67, 56, 202),   // Indigo 700
            primary_active: Color32::from_rgb(55, 48, 163),

            secondary: Color32::from_rgb(14, 165, 233),       // Sky 500 #0ea5e9
            secondary_hover: Color32::from_rgb(2, 132, 199),

            accent: Color32::from_rgb(147, 51, 234),          // Purple 600
            warning: Color32::from_rgb(217, 119, 6),          // Amber 600
            danger: Color32::from_rgb(225, 29, 72),           // Rose 600
            success: Color32::from_rgb(16, 185, 129),         // Emerald 500
            info: Color32::from_rgb(2, 132, 199),

            text_primary: Color32::from_rgb(15, 23, 42),       // Slate 900 #0f172a
            text_secondary: Color32::from_rgb(51, 65, 85),     // Slate 700 #334155
            text_muted: Color32::from_rgb(100, 116, 139),      // Slate 500 #64748b
            text_inverse: Color32::from_rgb(255, 255, 255),

            border_subtle: Color32::from_rgb(226, 232, 240),  // Slate 200 #e2e8f0
            border_strong: Color32::from_rgb(203, 213, 225),  // Slate 300 #cbd5e1
            border_accent: Color32::from_rgb(79, 70, 229),

            glow_color: Color32::from_rgba_premultiplied(79, 70, 229, 20),
            glow_radius: 6.0,
            shadow_blur: 14.0,
            shadow_color: Color32::from_rgba_premultiplied(15, 23, 42, 15),

            radius_sm: Rounding::same(6.0),
            radius_md: Rounding::same(10.0),
            radius_lg: Rounding::same(14.0),
            radius_pill: Rounding::same(999.0),

            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            spacing_xl: 32.0,

            item_spacing: Vec2::new(10.0, 10.0),
            card_padding: Vec2::new(16.0, 16.0),
        }
    }

    /// Stroke helper
    pub fn stroke_subtle(&self) -> Stroke {
        Stroke::new(1.0, self.border_subtle)
    }

    /// Stroke helper
    pub fn stroke_accent(&self) -> Stroke {
        Stroke::new(1.5, self.border_accent)
    }
}
