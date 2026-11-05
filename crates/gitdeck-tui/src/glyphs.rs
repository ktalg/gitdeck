use gitdeck_caps::CapsProfile;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconGlyph {
    Menu,
    ArrowLeft,
    Pointer,
    Chevron,
    DotFilled,
    DotOutline,
    Diamond,
    Check,
    Cross,
    Warning,
    ArrowUp,
    ArrowDown,
    Option,
    Gear,
}

pub const EVERY_ICON_GLYPH: [IconGlyph; 14] = [
    IconGlyph::Menu,
    IconGlyph::ArrowLeft,
    IconGlyph::Pointer,
    IconGlyph::Chevron,
    IconGlyph::DotFilled,
    IconGlyph::DotOutline,
    IconGlyph::Diamond,
    IconGlyph::Check,
    IconGlyph::Cross,
    IconGlyph::Warning,
    IconGlyph::ArrowUp,
    IconGlyph::ArrowDown,
    IconGlyph::Option,
    IconGlyph::Gear,
];

impl IconGlyph {
    pub const fn rich_character(self) -> char {
        match self {
            Self::Menu => '≡',
            Self::ArrowLeft => '←',
            Self::Pointer => '▸',
            Self::Chevron => '▾',
            Self::DotFilled => '●',
            Self::DotOutline => '○',
            Self::Diamond => '◆',
            Self::Check => '✓',
            Self::Cross => '✗',
            Self::Warning => '⚠',
            Self::ArrowUp => '↑',
            Self::ArrowDown => '↓',
            Self::Option => '⌥',
            Self::Gear => '⚙',
        }
    }

    pub const fn basic_character(self) -> char {
        match self {
            Self::Menu => '=',
            Self::ArrowLeft => '<',
            Self::Pointer => '>',
            Self::Chevron => 'v',
            Self::DotFilled => '*',
            Self::DotOutline => 'o',
            Self::Diamond => '+',
            Self::Check => 'y',
            Self::Cross => 'x',
            Self::Warning => '!',
            Self::ArrowUp => '^',
            Self::ArrowDown => 'v',
            Self::Option => '*',
            Self::Gear => '*',
        }
    }

    pub const fn character(self, caps: CapsProfile) -> char {
        match caps {
            CapsProfile::Rich | CapsProfile::Standard => self.rich_character(),
            CapsProfile::Basic => self.basic_character(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineGlyphs {
    pub horizontal: char,
    pub vertical: char,
    pub tee_up: char,
    pub tee_down: char,
    pub branch_corner: char,
    pub merge_corner: char,
}

pub const fn line_glyphs(caps: CapsProfile) -> LineGlyphs {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => LineGlyphs {
            horizontal: '─',
            vertical: '│',
            tee_up: '┴',
            tee_down: '┬',
            branch_corner: '╮',
            merge_corner: '╯',
        },
        CapsProfile::Basic => LineGlyphs {
            horizontal: '-',
            vertical: '|',
            tee_up: '+',
            tee_down: '+',
            branch_corner: '+',
            merge_corner: '+',
        },
    }
}

pub const fn ellipsis_glyph(caps: CapsProfile) -> char {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => '…',
        CapsProfile::Basic => '~',
    }
}

#[cfg(test)]
mod glyph_tests {
    use gitdeck_caps::CapsProfile;

    use super::{ellipsis_glyph, line_glyphs, IconGlyph, EVERY_ICON_GLYPH};

    const BASIC_DOWNGRADES: [(IconGlyph, char); 14] = [
        (IconGlyph::Menu, '='),
        (IconGlyph::ArrowLeft, '<'),
        (IconGlyph::Pointer, '>'),
        (IconGlyph::Chevron, 'v'),
        (IconGlyph::DotFilled, '*'),
        (IconGlyph::DotOutline, 'o'),
        (IconGlyph::Diamond, '+'),
        (IconGlyph::Check, 'y'),
        (IconGlyph::Cross, 'x'),
        (IconGlyph::Warning, '!'),
        (IconGlyph::ArrowUp, '^'),
        (IconGlyph::ArrowDown, 'v'),
        (IconGlyph::Option, '*'),
        (IconGlyph::Gear, '*'),
    ];

    #[test]
    fn rich_profile_uses_the_icon_character_itself() {
        for icon in EVERY_ICON_GLYPH {
            assert_eq!(
                icon.character(CapsProfile::Rich),
                icon.rich_character(),
                "icon {icon:?}"
            );
            assert_eq!(icon.character(CapsProfile::Standard), icon.rich_character());
        }
    }

    #[test]
    fn basic_profile_downgrades_every_icon_to_ascii() {
        for (icon, expected_character) in BASIC_DOWNGRADES {
            let character = icon.character(CapsProfile::Basic);
            assert_eq!(character, expected_character, "icon {icon:?}");
            assert!(character.is_ascii(), "icon {icon:?} is not ascii");
        }
    }

    #[test]
    fn basic_profile_draws_ascii_lines() {
        let glyphs = line_glyphs(CapsProfile::Basic);
        assert_eq!(glyphs.horizontal, '-');
        assert_eq!(glyphs.vertical, '|');
        assert_eq!(glyphs.tee_up, '+');
        assert_eq!(glyphs.tee_down, '+');
        assert_eq!(glyphs.branch_corner, '+');
        assert_eq!(glyphs.merge_corner, '+');
        let rich_glyphs = line_glyphs(CapsProfile::Rich);
        assert_eq!(rich_glyphs.horizontal, '─');
        assert_eq!(rich_glyphs.vertical, '│');
        assert_eq!(rich_glyphs.tee_up, '┴');
        assert_eq!(rich_glyphs.tee_down, '┬');
        assert_eq!(rich_glyphs.branch_corner, '╮');
        assert_eq!(rich_glyphs.merge_corner, '╯');
    }

    #[test]
    fn basic_profile_uses_an_ascii_ellipsis() {
        assert_eq!(ellipsis_glyph(CapsProfile::Rich), '…');
        assert_eq!(ellipsis_glyph(CapsProfile::Standard), '…');
        assert_eq!(ellipsis_glyph(CapsProfile::Basic), '~');
        assert!(ellipsis_glyph(CapsProfile::Basic).is_ascii());
    }
}
