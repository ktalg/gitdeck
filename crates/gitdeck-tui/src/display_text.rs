use gitdeck_caps::CapsProfile;
use unicode_width::UnicodeWidthChar;

pub const TAB_STOP_WIDTH: u16 = 8;
pub const UNICODE_CONTROL_PLACEHOLDER: char = '·';
pub const ASCII_CONTROL_PLACEHOLDER: char = '?';

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayGlyph {
    pub character: char,
    pub source_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SanitizedText {
    glyphs: Vec<DisplayGlyph>,
    text: String,
    display_width: u16,
}

impl SanitizedText {
    pub fn glyphs(&self) -> &[DisplayGlyph] {
        &self.glyphs
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn display_width(&self) -> u16 {
        self.display_width
    }

    pub fn glyph_index_at_display_column(&self, column: u16) -> usize {
        let mut consumed_width = 0u16;
        for (index, glyph) in self.glyphs.iter().enumerate() {
            if consumed_width >= column {
                return index;
            }
            consumed_width = consumed_width.saturating_add(glyph_display_width(glyph.character));
        }
        self.glyphs.len()
    }
}

pub const fn control_placeholder(caps: CapsProfile) -> char {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => UNICODE_CONTROL_PLACEHOLDER,
        CapsProfile::Basic => ASCII_CONTROL_PLACEHOLDER,
    }
}

pub fn glyph_display_width(character: char) -> u16 {
    u16::try_from(UnicodeWidthChar::width(character).unwrap_or(1).max(1)).unwrap_or(1)
}

pub fn sanitize_display_text(text: &str, start_column: u16, caps: CapsProfile) -> SanitizedText {
    let mut glyphs = Vec::with_capacity(text.chars().count());
    let mut sanitized = String::with_capacity(text.len());
    let mut column = start_column;
    for (source_index, character) in text.chars().enumerate() {
        match character {
            '\r' => {}
            '\t' => {
                let space_count = TAB_STOP_WIDTH - column % TAB_STOP_WIDTH;
                for _ in 0..space_count {
                    glyphs.push(DisplayGlyph {
                        character: ' ',
                        source_index,
                    });
                    sanitized.push(' ');
                    column = column.saturating_add(1);
                }
            }
            character if character.is_control() => {
                let placeholder = control_placeholder(caps);
                glyphs.push(DisplayGlyph {
                    character: placeholder,
                    source_index,
                });
                sanitized.push(placeholder);
                column = column.saturating_add(1);
            }
            character => {
                glyphs.push(DisplayGlyph {
                    character,
                    source_index,
                });
                sanitized.push(character);
                column = column.saturating_add(glyph_display_width(character));
            }
        }
    }
    SanitizedText {
        glyphs,
        text: sanitized,
        display_width: column.saturating_sub(start_column),
    }
}

pub fn sanitize_display_string(text: &str, start_column: u16, caps: CapsProfile) -> String {
    sanitize_display_text(text, start_column, caps).text
}

#[cfg(test)]
mod display_text_tests {
    use gitdeck_caps::CapsProfile;

    use super::{
        control_placeholder, sanitize_display_string, sanitize_display_text, TAB_STOP_WIDTH,
    };

    fn spaces(count: usize) -> String {
        " ".repeat(count)
    }

    #[test]
    fn tab_expands_to_the_next_eight_column_stop() {
        assert_eq!(
            sanitize_display_string("\tx", 0, CapsProfile::Rich),
            format!("{}x", spaces(8))
        );
        assert_eq!(
            sanitize_display_string("\tx", 3, CapsProfile::Rich),
            format!("{}x", spaces(5))
        );
        assert_eq!(
            sanitize_display_string("a\tb", 0, CapsProfile::Rich),
            format!("a{}b", spaces(7))
        );
        assert_eq!(TAB_STOP_WIDTH, 8);
    }

    #[test]
    fn carriage_returns_are_dropped() {
        assert_eq!(
            sanitize_display_string("a\rb", 0, CapsProfile::Rich),
            String::from("ab")
        );
        assert_eq!(
            sanitize_display_string("\r", 0, CapsProfile::Rich),
            String::new()
        );
        let sanitized = sanitize_display_text("a\r\nb", 0, CapsProfile::Rich);
        assert_eq!(sanitized.text(), "a·b");
        assert_eq!(sanitized.display_width(), 3);
    }

    #[test]
    fn other_control_characters_become_a_placeholder() {
        assert_eq!(
            sanitize_display_string("a\u{1}b\u{1b}c\u{7f}", 0, CapsProfile::Rich),
            String::from("a·b·c·")
        );
        assert_eq!(
            sanitize_display_string("\u{0}", 0, CapsProfile::Rich),
            String::from("·")
        );
    }

    #[test]
    fn printable_and_cjk_text_is_unchanged() {
        let sanitized = sanitize_display_text("中a文", 0, CapsProfile::Rich);
        assert_eq!(sanitized.text(), "中a文");
        assert_eq!(sanitized.display_width(), 5);
        assert_eq!(sanitized.glyphs().len(), 3);
        assert_eq!(sanitized.glyphs()[2].source_index, 2);
    }

    #[test]
    fn basic_caps_uses_ascii_placeholders() {
        assert_eq!(control_placeholder(CapsProfile::Basic), '?');
        assert_eq!(control_placeholder(CapsProfile::Standard), '·');
        assert_eq!(control_placeholder(CapsProfile::Rich), '·');
        let sanitized = sanitize_display_string("a\u{1}\tb", 0, CapsProfile::Basic);
        assert!(sanitized.is_ascii(), "{sanitized:?} must stay ascii");
        assert_eq!(sanitized, format!("a?{}b", spaces(6)));
    }
}
