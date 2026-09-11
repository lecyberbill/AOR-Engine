// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Complete public Prelude and Ergonomic API
//! # Rust Modern UI (`rust_modern_ui`)
//!
//! Une bibliothèque graphique moderne, élégante et ultra-personnalisable pour Rust avec `egui` / `eframe`.
//!
//! ## Utilisation rapide
//!
//! ```rust,no_run
//! use rust_modern_ui::prelude::*;
//! ```

pub mod theme;
pub mod primitives;
pub mod widgets;
pub mod layout;

/// Prélude regroupant tous les éléments indispensables pour développer rapidement
pub mod prelude {
    pub use crate::theme::{
        ThemeEngine, ThemePreset, ThemeTokens, TypographyConfig,
    };
    pub use crate::widgets::{
        ModernButton, ModernButtonVariant,
        ModernCard,
        ModernStatCard,
        ModernBadge, BadgeVariant,
        ModernModal,
        modern_switch,
        modern_slider,
        modern_segmented_control,
    };
    pub use crate::layout::ModernAppShell;
    pub use crate::primitives::{
        glass_frame, paint_glass_panel,
        paint_glow_rect, paint_led_indicator,
        paint_divider, paint_tech_corner,
    };
    // Re-exports egui utiles pour éviter les imports redondants
    pub use egui::{self, Color32, Margin, Pos2, Rect, RichText, Rounding, Stroke, Ui, Vec2};
}

// Re-exports directs
pub use prelude::*;
