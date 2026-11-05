use std::ops::Range;

use crate::diff::{DiffRow, DiffRowKind};

pub fn assign_word_highlights(rows: &mut [DiffRow<'_>]) {
    let mut index = 0;
    while index < rows.len() {
        if rows[index].kind != DiffRowKind::Removed {
            index += 1;
            continue;
        }
        let removed_start = index;
        while index < rows.len() && rows[index].kind == DiffRowKind::Removed {
            index += 1;
        }
        let removed_end = index;
        let mut added_end = index;
        while added_end < rows.len() && rows[added_end].kind == DiffRowKind::Added {
            added_end += 1;
        }
        if added_end == index {
            continue;
        }
        pair_removed_and_added_runs(rows, removed_start..removed_end, index..added_end);
        index = added_end;
    }
}

pub fn changed_char_ranges(
    old_text: &str,
    new_text: &str,
) -> (Option<Range<usize>>, Option<Range<usize>>) {
    let old_characters: Vec<char> = old_text.chars().collect();
    let new_characters: Vec<char> = new_text.chars().collect();
    let prefix_length = old_characters
        .iter()
        .zip(new_characters.iter())
        .take_while(|(old_character, new_character)| old_character == new_character)
        .count();
    let shared_length = old_characters.len().min(new_characters.len());
    let suffix_length = old_characters
        .iter()
        .rev()
        .zip(new_characters.iter().rev())
        .take(shared_length.saturating_sub(prefix_length))
        .take_while(|(old_character, new_character)| old_character == new_character)
        .count();
    (
        non_empty_character_range(prefix_length, old_characters.len() - suffix_length),
        non_empty_character_range(prefix_length, new_characters.len() - suffix_length),
    )
}

fn pair_removed_and_added_runs(
    rows: &mut [DiffRow<'_>],
    removed: Range<usize>,
    added: Range<usize>,
) {
    let paired_count = removed.len().min(added.len());
    let mut highlights: Vec<(usize, Option<Range<usize>>)> =
        Vec::with_capacity(removed.len() + added.len());
    for offset in 0..paired_count {
        let removed_index = removed.start + offset;
        let added_index = added.start + offset;
        let (removed_range, added_range) = changed_char_ranges(
            rows[removed_index].text.as_ref(),
            rows[added_index].text.as_ref(),
        );
        highlights.push((removed_index, removed_range));
        highlights.push((added_index, added_range));
    }
    push_whole_line_highlights(
        rows,
        removed.start + paired_count..removed.end,
        &mut highlights,
    );
    push_whole_line_highlights(rows, added.start + paired_count..added.end, &mut highlights);
    for (index, highlight) in highlights {
        rows[index].highlight = highlight;
    }
}

fn push_whole_line_highlights(
    rows: &[DiffRow<'_>],
    unpaired: Range<usize>,
    highlights: &mut Vec<(usize, Option<Range<usize>>)>,
) {
    for (offset, row) in rows[unpaired.clone()].iter().enumerate() {
        highlights.push((
            unpaired.start + offset,
            whole_character_range(row.text.as_ref()),
        ));
    }
}

fn whole_character_range(text: &str) -> Option<Range<usize>> {
    non_empty_character_range(0, text.chars().count())
}

fn non_empty_character_range(start: usize, end: usize) -> Option<Range<usize>> {
    (start < end).then_some(start..end)
}

#[cfg(test)]
mod word_highlight_tests {
    use std::borrow::Cow;
    use std::ops::Range;

    use crate::diff::{DiffRow, DiffRowKind};

    use super::{assign_word_highlights, changed_char_ranges};

    fn row(kind: DiffRowKind, text: &str) -> DiffRow<'_> {
        DiffRow {
            kind,
            old_line_number: None,
            new_line_number: None,
            hunk_index: None,
            text: Cow::Borrowed(text),
            highlight: None,
        }
    }

    fn highlights(rows: &[DiffRow<'_>]) -> Vec<Option<Range<usize>>> {
        rows.iter().map(|row| row.highlight.clone()).collect()
    }

    #[test]
    fn paired_lines_highlight_only_the_middle_between_the_common_edges() {
        assert_eq!(
            changed_char_ranges("let a = 1;", "let b = 2;"),
            (Some(4..9), Some(4..9))
        );
        assert_eq!(
            changed_char_ranges("pub const A: u32 = 5;", "pub const A: u32 = 50;"),
            (None, Some(20..21))
        );
    }

    #[test]
    fn identical_text_has_no_changed_range() {
        assert_eq!(changed_char_ranges("same line", "same line"), (None, None));
        assert_eq!(changed_char_ranges("", ""), (None, None));
    }

    #[test]
    fn a_longer_added_line_highlights_only_its_tail() {
        assert_eq!(changed_char_ranges("keep", "keep plus"), (None, Some(4..9)));
    }

    #[test]
    fn the_removed_run_is_paired_with_the_following_added_run() {
        let mut rows = [
            row(DiffRowKind::Context, "let a = 1;"),
            row(DiffRowKind::Removed, "let a = 1;"),
            row(DiffRowKind::Removed, "let b = 2;"),
            row(DiffRowKind::Added, "let a = 9;"),
            row(DiffRowKind::Added, "let b = 8;"),
            row(DiffRowKind::Added, "let c = 3;"),
            row(DiffRowKind::Context, "tail"),
        ];
        assign_word_highlights(&mut rows);
        assert_eq!(
            highlights(&rows),
            vec![
                None,
                Some(8..9),
                Some(8..9),
                Some(8..9),
                Some(8..9),
                Some(0..10),
                None,
            ]
        );
    }

    #[test]
    fn hunk_headers_and_context_rows_break_the_pairing() {
        let mut rows = [
            row(DiffRowKind::Removed, "old"),
            row(DiffRowKind::Context, "context"),
            row(DiffRowKind::Added, "new"),
        ];
        assign_word_highlights(&mut rows);
        assert_eq!(highlights(&rows), vec![None, None, None]);
    }
}
