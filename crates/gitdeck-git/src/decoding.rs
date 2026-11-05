pub(crate) fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub(crate) fn decode_line(bytes: &[u8]) -> String {
    decode_text(bytes)
        .trim_end_matches(['\n', '\r'])
        .to_string()
}

#[cfg(test)]
mod decoding_tests {
    use super::{decode_line, decode_text};

    #[test]
    fn decode_text_keeps_wide_characters_of_the_source_bytes() {
        assert_eq!(decode_text("文档/说明.md".as_bytes()), "文档/说明.md");
    }

    #[test]
    fn decode_line_drops_trailing_line_breaks_only() {
        assert_eq!(decode_line(b"origin/main\n"), "origin/main");
        assert_eq!(decode_line(b"src/app.rs\r\n"), "src/app.rs");
        assert_eq!(decode_line("带 空格.md\n".as_bytes()), "带 空格.md");
        assert_eq!(decode_line(b"trailing space \n"), "trailing space ");
    }
}
