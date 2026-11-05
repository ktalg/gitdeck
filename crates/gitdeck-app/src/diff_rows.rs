use std::borrow::Cow;

use gitdeck_git::{DiffHunk, DiffLine, DiffLineKind, FileDiff};
use gitdeck_tui::{DiffRow, DiffRowKind, BINARY_FILE_LABEL, IDENTICAL_CONTENTS_LABEL};

pub(crate) fn diff_rows(diff: &FileDiff) -> Vec<DiffRow<'_>> {
    if diff.binary {
        return vec![notice_row(BINARY_FILE_LABEL)];
    }
    if diff.hunks.is_empty() {
        return vec![notice_row(IDENTICAL_CONTENTS_LABEL)];
    }
    let mut rows = Vec::with_capacity(diff_row_count(diff));
    for (hunk_index, hunk) in diff.hunks.iter().enumerate() {
        rows.push(hunk_header_row(hunk, hunk_index));
        rows.extend(hunk.lines.iter().map(line_row));
    }
    rows
}

pub(crate) fn diff_row_count(diff: &FileDiff) -> usize {
    let line_count: usize = diff.hunks.iter().map(|hunk| hunk.lines.len()).sum();
    let hunk_header_count = diff.hunks.len();
    let notice_count = usize::from(diff.binary || diff.hunks.is_empty());
    (line_count + hunk_header_count + notice_count).max(1)
}

fn notice_row<'a>(text: &'a str) -> DiffRow<'a> {
    DiffRow {
        kind: DiffRowKind::Notice,
        old_line_number: None,
        new_line_number: None,
        hunk_index: None,
        highlight: None,
        text: Cow::Borrowed(text),
    }
}

fn hunk_header_row(hunk: &DiffHunk, hunk_index: usize) -> DiffRow<'static> {
    DiffRow {
        kind: DiffRowKind::HunkHeader,
        old_line_number: None,
        new_line_number: None,
        hunk_index: Some(hunk_index),
        text: Cow::Owned(format!(
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
        )),
        highlight: None,
    }
}

fn line_row(line: &DiffLine) -> DiffRow<'_> {
    DiffRow {
        kind: diff_row_kind(line),
        old_line_number: line.old_line_number,
        new_line_number: line.new_line_number,
        hunk_index: None,
        highlight: None,
        text: Cow::Borrowed(line.text.as_str()),
    }
}

fn diff_row_kind(line: &DiffLine) -> DiffRowKind {
    if line.old_line_number.is_none() && line.new_line_number.is_none() {
        return DiffRowKind::NoNewlineMarker;
    }
    match line.kind {
        DiffLineKind::Context => DiffRowKind::Context,
        DiffLineKind::Added => DiffRowKind::Added,
        DiffLineKind::Removed => DiffRowKind::Removed,
    }
}

#[cfg(test)]
mod diff_rows_tests {
    use gitdeck_git::{DiffHunk, DiffLine, DiffLineKind, FileDiff};
    use gitdeck_tui::DiffRowKind;

    use super::{diff_row_count, diff_rows};

    fn line(kind: DiffLineKind, old: Option<u32>, new: Option<u32>, text: &str) -> DiffLine {
        DiffLine {
            kind,
            old_line_number: old,
            new_line_number: new,
            text: text.to_string(),
        }
    }

    fn diff_with(hunks: Vec<DiffHunk>) -> FileDiff {
        FileDiff {
            path: String::from("src/app.rs"),
            binary: false,
            missing_newline_at_end: false,
            hunks,
        }
    }

    fn hunk(lines: Vec<DiffLine>) -> DiffHunk {
        DiffHunk {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            lines,
        }
    }

    #[test]
    fn every_hunk_starts_with_its_header_row() {
        let diff = diff_with(vec![hunk(vec![
            line(DiffLineKind::Removed, Some(1), None, "old"),
            line(DiffLineKind::Added, None, Some(1), "new"),
        ])]);
        let rows = diff_rows(&diff);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].kind, DiffRowKind::HunkHeader);
        assert_eq!(rows[0].text, "@@ -1,3 +1,3 @@");
        assert_eq!(rows[1].kind, DiffRowKind::Removed);
        assert_eq!(rows[1].old_line_number, Some(1));
        assert_eq!(rows[2].kind, DiffRowKind::Added);
        assert_eq!(rows[2].new_line_number, Some(1));
        assert_eq!(diff_row_count(&diff), 3);
    }

    #[test]
    fn marker_lines_without_line_numbers_become_no_newline_rows() {
        let diff = diff_with(vec![hunk(vec![line(
            DiffLineKind::Context,
            None,
            None,
            "\\ No newline at end of file",
        )])]);
        let rows = diff_rows(&diff);
        assert_eq!(rows[1].kind, DiffRowKind::NoNewlineMarker);
    }

    #[test]
    fn binary_and_identical_files_render_a_single_notice_row() {
        let binary = FileDiff {
            binary: true,
            ..diff_with(Vec::new())
        };
        assert_eq!(diff_rows(&binary)[0].kind, DiffRowKind::Notice);
        assert_eq!(diff_rows(&binary)[0].text, "Binary file");
        assert_eq!(diff_row_count(&binary), 1);
        let identical = diff_with(Vec::new());
        assert_eq!(diff_rows(&identical)[0].text, "Contents are identical");
        assert_eq!(diff_row_count(&identical), 1);
    }
}
