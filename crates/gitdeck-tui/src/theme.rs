use gitdeck_caps::ThemeId;
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub panel: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub selection: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub added_word_highlight: Color,
    pub removed_word_highlight: Color,
}

pub const fn theme_palette(theme_id: ThemeId) -> Theme {
    match theme_id {
        ThemeId::JbDark => JB_DARK,
        ThemeId::JbLight => JB_LIGHT,
        ThemeId::Solarized => SOLARIZED,
        ThemeId::Nord => NORD,
        ThemeId::HighContrast => HIGH_CONTRAST,
        ThemeId::Mono => MONO,
    }
}

const JB_DARK: Theme = Theme {
    background: rgb(0x1E, 0x1F, 0x22),
    panel: rgb(0x2B, 0x2D, 0x30),
    border: rgb(0x39, 0x3B, 0x40),
    text: rgb(0xDF, 0xE1, 0xE5),
    muted: rgb(0x86, 0x8A, 0x91),
    accent: rgb(0x35, 0x74, 0xF0),
    selection: rgb(0x2E, 0x43, 0x6E),
    success: rgb(0x57, 0x96, 0x5C),
    error: rgb(0xDB, 0x5C, 0x5C),
    warning: rgb(0xC9, 0xA6, 0x3C),
    added_word_highlight: rgb(0x3A, 0x6B, 0x47),
    removed_word_highlight: rgb(0x6B, 0x3A, 0x3A),
};

const JB_LIGHT: Theme = Theme {
    background: rgb(0xFF, 0xFF, 0xFF),
    panel: rgb(0xF7, 0xF8, 0xFA),
    border: rgb(0xDF, 0xE1, 0xE5),
    text: rgb(0x1E, 0x1F, 0x22),
    muted: rgb(0x6C, 0x70, 0x7E),
    accent: rgb(0x35, 0x74, 0xF0),
    selection: rgb(0xD4, 0xE2, 0xFF),
    success: JB_DARK.success,
    error: JB_DARK.error,
    warning: JB_DARK.warning,
    added_word_highlight: rgb(0x35, 0x6B, 0x42),
    removed_word_highlight: rgb(0x6B, 0x35, 0x35),
};

const SOLARIZED: Theme = Theme {
    background: rgb(0x00, 0x2B, 0x36),
    panel: rgb(0x07, 0x36, 0x42),
    border: rgb(0x58, 0x6E, 0x75),
    text: rgb(0x93, 0xA1, 0xA1),
    muted: rgb(0x65, 0x7B, 0x83),
    accent: rgb(0x26, 0x8B, 0xD2),
    selection: rgb(0x07, 0x36, 0x42),
    success: rgb(0x85, 0x99, 0x00),
    error: rgb(0xDC, 0x32, 0x2F),
    warning: rgb(0xB5, 0x89, 0x00),
    added_word_highlight: rgb(0x2C, 0x4A, 0x24),
    removed_word_highlight: rgb(0x4A, 0x2C, 0x24),
};

const NORD: Theme = Theme {
    background: rgb(0x2E, 0x34, 0x40),
    panel: rgb(0x3B, 0x42, 0x52),
    border: rgb(0x4C, 0x56, 0x6A),
    text: rgb(0xD8, 0xDE, 0xE9),
    muted: rgb(0x7B, 0x88, 0xA1),
    accent: rgb(0x88, 0xC0, 0xD0),
    selection: rgb(0x43, 0x4C, 0x5E),
    success: rgb(0xA3, 0xBE, 0x8C),
    error: rgb(0xBF, 0x61, 0x6A),
    warning: rgb(0xEB, 0xCB, 0x8B),
    added_word_highlight: rgb(0x3B, 0x5A, 0x44),
    removed_word_highlight: rgb(0x5A, 0x3B, 0x3F),
};

const HIGH_CONTRAST: Theme = Theme {
    background: rgb(0x00, 0x00, 0x00),
    panel: rgb(0x0A, 0x0A, 0x0A),
    border: rgb(0xFF, 0xFF, 0xFF),
    text: rgb(0xFF, 0xFF, 0xFF),
    muted: rgb(0xC0, 0xC0, 0xC0),
    accent: rgb(0x3B, 0x8B, 0xFF),
    selection: rgb(0x00, 0x40, 0x80),
    success: rgb(0x57, 0xC2, 0x5C),
    error: rgb(0xFF, 0x6B, 0x6B),
    warning: rgb(0xFF, 0xD7, 0x5F),
    added_word_highlight: rgb(0x1F, 0x5F, 0x2B),
    removed_word_highlight: rgb(0x6F, 0x1F, 0x1F),
};

const MONO: Theme = Theme {
    background: rgb(0x00, 0x00, 0x00),
    panel: rgb(0x1A, 0x1A, 0x1A),
    border: rgb(0x7F, 0x7F, 0x7F),
    text: rgb(0xFF, 0xFF, 0xFF),
    muted: rgb(0xA0, 0xA0, 0xA0),
    accent: rgb(0xFF, 0xFF, 0xFF),
    selection: rgb(0x4D, 0x4D, 0x4D),
    success: rgb(0xFF, 0xFF, 0xFF),
    error: rgb(0xFF, 0xFF, 0xFF),
    warning: rgb(0xFF, 0xFF, 0xFF),
    added_word_highlight: rgb(0x40, 0x40, 0x40),
    removed_word_highlight: rgb(0x55, 0x55, 0x55),
};

const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::Rgb(red, green, blue)
}

#[cfg(test)]
mod theme_palette_tests {
    use gitdeck_caps::ThemeId;
    use ratatui::style::Color;

    use super::theme_palette;

    #[test]
    fn jb_dark_uses_the_frozen_jetbrains_tokens() {
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(theme.background, Color::Rgb(0x1E, 0x1F, 0x22));
        assert_eq!(theme.panel, Color::Rgb(0x2B, 0x2D, 0x30));
        assert_eq!(theme.border, Color::Rgb(0x39, 0x3B, 0x40));
        assert_eq!(theme.text, Color::Rgb(0xDF, 0xE1, 0xE5));
        assert_eq!(theme.muted, Color::Rgb(0x86, 0x8A, 0x91));
        assert_eq!(theme.accent, Color::Rgb(0x35, 0x74, 0xF0));
        assert_eq!(theme.selection, Color::Rgb(0x2E, 0x43, 0x6E));
        assert_eq!(theme.success, Color::Rgb(0x57, 0x96, 0x5C));
        assert_eq!(theme.error, Color::Rgb(0xDB, 0x5C, 0x5C));
        assert_eq!(theme.warning, Color::Rgb(0xC9, 0xA6, 0x3C));
        assert_eq!(theme.added_word_highlight, Color::Rgb(0x3A, 0x6B, 0x47));
        assert_eq!(theme.removed_word_highlight, Color::Rgb(0x6B, 0x3A, 0x3A));
    }

    #[test]
    fn jb_light_inverts_the_surface_and_text_tokens() {
        let theme = theme_palette(ThemeId::JbLight);
        assert_eq!(theme.background, Color::Rgb(0xFF, 0xFF, 0xFF));
        assert_eq!(theme.panel, Color::Rgb(0xF7, 0xF8, 0xFA));
        assert_eq!(theme.border, Color::Rgb(0xDF, 0xE1, 0xE5));
        assert_eq!(theme.text, Color::Rgb(0x1E, 0x1F, 0x22));
        assert_eq!(theme.muted, Color::Rgb(0x6C, 0x70, 0x7E));
        assert_eq!(theme.accent, Color::Rgb(0x35, 0x74, 0xF0));
        assert_eq!(theme.selection, Color::Rgb(0xD4, 0xE2, 0xFF));
    }

    #[test]
    fn every_theme_keeps_text_and_surface_apart() {
        for theme_id in [
            ThemeId::JbDark,
            ThemeId::JbLight,
            ThemeId::Solarized,
            ThemeId::Nord,
            ThemeId::HighContrast,
            ThemeId::Mono,
        ] {
            let theme = theme_palette(theme_id);
            assert_ne!(theme.text, theme.background, "theme {theme_id:?}");
            assert_ne!(theme.text, theme.panel, "theme {theme_id:?}");
            assert_ne!(theme.border, theme.panel, "theme {theme_id:?}");
        }
    }
}
