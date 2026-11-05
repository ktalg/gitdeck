use ratatui::style::Color;

const ANSI_BASE_COLORS: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00),
    (0x80, 0x00, 0x00),
    (0x00, 0x80, 0x00),
    (0x80, 0x80, 0x00),
    (0x00, 0x00, 0x80),
    (0x80, 0x00, 0x80),
    (0x00, 0x80, 0x80),
    (0xC0, 0xC0, 0xC0),
    (0x80, 0x80, 0x80),
    (0xFF, 0x00, 0x00),
    (0x00, 0xFF, 0x00),
    (0xFF, 0xFF, 0x00),
    (0x00, 0x00, 0xFF),
    (0xFF, 0x00, 0xFF),
    (0x00, 0xFF, 0xFF),
    (0xFF, 0xFF, 0xFF),
];

const COLOR_CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

pub(crate) fn resolve_color(color: Color, default: (u8, u8, u8)) -> (u8, u8, u8) {
    match color {
        Color::Reset => default,
        Color::Black => ANSI_BASE_COLORS[0],
        Color::Red => ANSI_BASE_COLORS[1],
        Color::Green => ANSI_BASE_COLORS[2],
        Color::Yellow => ANSI_BASE_COLORS[3],
        Color::Blue => ANSI_BASE_COLORS[4],
        Color::Magenta => ANSI_BASE_COLORS[5],
        Color::Cyan => ANSI_BASE_COLORS[6],
        Color::Gray => ANSI_BASE_COLORS[7],
        Color::DarkGray => ANSI_BASE_COLORS[8],
        Color::LightRed => ANSI_BASE_COLORS[9],
        Color::LightGreen => ANSI_BASE_COLORS[10],
        Color::LightYellow => ANSI_BASE_COLORS[11],
        Color::LightBlue => ANSI_BASE_COLORS[12],
        Color::LightMagenta => ANSI_BASE_COLORS[13],
        Color::LightCyan => ANSI_BASE_COLORS[14],
        Color::White => ANSI_BASE_COLORS[15],
        Color::Rgb(red, green, blue) => (red, green, blue),
        Color::Indexed(index) => indexed_color(index),
    }
}

fn indexed_color(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => ANSI_BASE_COLORS[usize::from(index)],
        16..=231 => {
            let offset = index - 16;
            (
                COLOR_CUBE_LEVELS[usize::from(offset / 36)],
                COLOR_CUBE_LEVELS[usize::from((offset % 36) / 6)],
                COLOR_CUBE_LEVELS[usize::from(offset % 6)],
            )
        }
        _ => {
            let gray_level = 8 + (index - 232) * 10;
            (gray_level, gray_level, gray_level)
        }
    }
}

#[cfg(test)]
mod ansi_palette_tests {
    use ratatui::style::Color;

    use super::{indexed_color, resolve_color};

    #[test]
    fn rgb_color_resolves_to_its_own_channels() {
        assert_eq!(resolve_color(Color::Rgb(1, 2, 3), (0, 0, 0)), (1, 2, 3));
    }

    #[test]
    fn reset_color_resolves_to_theme_default() {
        assert_eq!(resolve_color(Color::Reset, (9, 8, 7)), (9, 8, 7));
    }

    #[test]
    fn indexed_color_matches_ansi_tables() {
        assert_eq!(indexed_color(0), (0x00, 0x00, 0x00));
        assert_eq!(indexed_color(15), (0xFF, 0xFF, 0xFF));
        assert_eq!(indexed_color(16), (0, 0, 0));
        assert_eq!(indexed_color(196), (255, 0, 0));
        assert_eq!(indexed_color(231), (255, 255, 255));
        assert_eq!(indexed_color(232), (8, 8, 8));
        assert_eq!(indexed_color(255), (238, 238, 238));
    }

    #[test]
    fn named_colors_match_ansi_table_entries() {
        for (named_color, indexed_color_value) in [
            (Color::Black, 0),
            (Color::Red, 1),
            (Color::Green, 2),
            (Color::Yellow, 3),
            (Color::Blue, 4),
            (Color::Magenta, 5),
            (Color::Cyan, 6),
            (Color::Gray, 7),
            (Color::DarkGray, 8),
            (Color::LightRed, 9),
            (Color::LightGreen, 10),
            (Color::LightYellow, 11),
            (Color::LightBlue, 12),
            (Color::LightMagenta, 13),
            (Color::LightCyan, 14),
            (Color::White, 15),
        ] {
            assert_eq!(
                resolve_color(named_color, (0, 0, 0)),
                indexed_color(indexed_color_value),
                "color {named_color:?}"
            );
        }
    }
}
