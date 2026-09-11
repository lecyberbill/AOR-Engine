// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Theme Presets enum with industry standard web themes
use super::tokens::ThemeTokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    TokyoNight,
    CatppuccinMocha,
    NordDark,
    TailwindLight,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 4] = [
        ThemePreset::TokyoNight,
        ThemePreset::CatppuccinMocha,
        ThemePreset::NordDark,
        ThemePreset::TailwindLight,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::TokyoNight => "🌸 Tokyo Night",
            ThemePreset::CatppuccinMocha => "🌿 Catppuccin",
            ThemePreset::NordDark => "❄️ Nord Arctic",
            ThemePreset::TailwindLight => "☀️ Tailwind Light",
        }
    }

    pub fn to_tokens(&self) -> ThemeTokens {
        match self {
            ThemePreset::TokyoNight => ThemeTokens::tokyo_night(),
            ThemePreset::CatppuccinMocha => ThemeTokens::catppuccin_mocha(),
            ThemePreset::NordDark => ThemeTokens::nord_dark(),
            ThemePreset::TailwindLight => ThemeTokens::tailwind_slate_light(),
        }
    }
}
