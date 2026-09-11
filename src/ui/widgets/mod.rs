pub mod button;
pub mod card;
pub mod toggle;
pub mod slider;
pub mod tabs;
pub mod badge;
pub mod stat_card;
pub mod modal;

pub use button::{ModernButton, ModernButtonVariant};
pub use card::ModernCard;
pub use toggle::modern_switch;
pub use slider::modern_slider;
pub use tabs::modern_segmented_control;
pub use badge::{ModernBadge, BadgeVariant};
pub use stat_card::ModernStatCard;
pub use modal::ModernModal;
