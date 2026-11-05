use gitdeck_caps::ThemeId;

pub(crate) fn theme_foreground(theme: ThemeId) -> (u8, u8, u8) {
    match theme {
        ThemeId::JbDark => (0xBC, 0xBE, 0xC4),
        ThemeId::JbLight => (0x1E, 0x1F, 0x22),
        ThemeId::Solarized => (0x83, 0x94, 0x96),
        ThemeId::Nord => (0xD8, 0xDE, 0xE9),
        ThemeId::HighContrast => (0xFF, 0xFF, 0xFF),
        ThemeId::Mono => (0xFF, 0xFF, 0xFF),
    }
}

pub(crate) fn theme_background(theme: ThemeId) -> (u8, u8, u8) {
    match theme {
        ThemeId::JbDark => (0x1E, 0x1F, 0x22),
        ThemeId::JbLight => (0xFF, 0xFF, 0xFF),
        ThemeId::Solarized => (0x00, 0x2B, 0x36),
        ThemeId::Nord => (0x2E, 0x34, 0x40),
        ThemeId::HighContrast => (0x00, 0x00, 0x00),
        ThemeId::Mono => (0x00, 0x00, 0x00),
    }
}

#[cfg(test)]
mod theme_palette_tests {
    use gitdeck_caps::ThemeId;

    use super::{theme_background, theme_foreground};

    const EVERY_THEME_ID: [ThemeId; 6] = [
        ThemeId::JbDark,
        ThemeId::JbLight,
        ThemeId::Solarized,
        ThemeId::Nord,
        ThemeId::HighContrast,
        ThemeId::Mono,
    ];

    #[test]
    fn every_theme_has_distinct_foreground_and_background() {
        for theme_id in EVERY_THEME_ID {
            assert_ne!(
                theme_foreground(theme_id),
                theme_background(theme_id),
                "theme {theme_id:?}"
            );
        }
    }
}
