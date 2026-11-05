use ratatui::buffer::Buffer;
use unicode_width::UnicodeWidthStr;

pub(crate) fn symbol_columns(symbol: &str) -> u16 {
    u16::try_from(UnicodeWidthStr::width(symbol))
        .unwrap_or(1)
        .max(1)
}

pub(crate) fn buffer_lines(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|row| trim_trailing_spaces(buffer_line(buffer, row)))
        .collect()
}

pub(crate) fn buffer_text(buffer: &Buffer) -> String {
    buffer_lines(buffer).join("\n")
}

pub(crate) fn buffer_line(buffer: &Buffer, row: u16) -> String {
    let mut line = String::new();
    let mut column = 0;
    while column < buffer.area.width {
        let Some(cell) = buffer.cell((column, row)) else {
            break;
        };
        let symbol = cell.symbol();
        line.push_str(symbol);
        column += symbol_columns(symbol);
    }
    line
}

fn trim_trailing_spaces(line: String) -> String {
    let trimmed_length = line.trim_end_matches(' ').len();
    line[..trimmed_length].to_string()
}

#[cfg(test)]
mod screen_text_tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    use super::{buffer_line, buffer_lines, buffer_text, symbol_columns};

    #[test]
    fn trailing_spaces_are_trimmed_from_every_line() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 2));
        buffer.set_string(0, 0, "ab", Style::default());
        buffer.set_string(0, 1, "abcdef", Style::default());
        assert_eq!(buffer_lines(&buffer), vec!["ab", "abcdef"]);
    }

    #[test]
    fn wide_symbols_span_two_columns_and_hide_the_continuation_column() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
        buffer.set_string(0, 0, "中a", Style::default());
        assert_eq!(buffer_text(&buffer), "中a");
        assert_eq!(buffer_lines(&buffer), vec![String::from("中a")]);
        assert_eq!(buffer_line(&buffer, 0), "中a   ");
    }

    #[test]
    fn symbol_columns_reports_terminal_cell_width() {
        assert_eq!(symbol_columns("a"), 1);
        assert_eq!(symbol_columns(" "), 1);
        assert_eq!(symbol_columns("中"), 2);
        assert_eq!(symbol_columns("📦"), 2);
    }

    #[test]
    fn text_joins_lines_without_trailing_newline() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 2));
        buffer.set_string(0, 0, "one", Style::default());
        buffer.set_string(0, 1, "two", Style::default());
        assert_eq!(buffer_text(&buffer), "one\ntwo");
    }
}
