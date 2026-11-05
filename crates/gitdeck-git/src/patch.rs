use std::collections::BTreeSet;

use crate::error::GitError;

const HUNK_HEADER_PREFIX: &[u8] = b"@@";
const LINE_SEPARATOR: u8 = b'\n';
const BINARY_NOTICE_PREFIX: &[u8] = b"Binary files ";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HunkSpan {
    pub index: usize,
    pub start: usize,
    pub end: usize,
}

pub(crate) fn hunk_spans(raw: &[u8]) -> Vec<HunkSpan> {
    let starts: Vec<usize> = line_starts(raw)
        .filter(|start| raw[*start..].starts_with(HUNK_HEADER_PREFIX))
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(index, start)| HunkSpan {
            index,
            start: *start,
            end: starts.get(index + 1).copied().unwrap_or(raw.len()),
        })
        .collect()
}

pub(crate) fn patch_header(raw: &[u8]) -> &[u8] {
    let header_end = hunk_spans(raw)
        .first()
        .map(|span| span.start)
        .unwrap_or(raw.len());
    &raw[..header_end]
}

pub(crate) fn diff_reports_binary_file(raw: &[u8]) -> bool {
    line_starts(raw).any(|start| raw[start..].starts_with(BINARY_NOTICE_PREFIX))
}

pub(crate) fn assemble_patch(
    raw: &[u8],
    path: &str,
    hunk_indices: &[usize],
) -> Result<Vec<u8>, GitError> {
    if hunk_indices.is_empty() {
        return Err(GitError::EmptyHunkSelection {
            path: path.to_string(),
        });
    }
    let spans = hunk_spans(raw);
    let mut patch = patch_header(raw).to_vec();
    for index in hunk_indices.iter().copied().collect::<BTreeSet<usize>>() {
        let span = spans
            .get(index)
            .ok_or_else(|| GitError::HunkIndexOutOfRange {
                path: path.to_string(),
                index,
                hunk_count: spans.len(),
            })?;
        patch.extend_from_slice(&raw[span.start..span.end]);
    }
    Ok(patch)
}

fn line_starts(raw: &[u8]) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(
        raw.iter()
            .enumerate()
            .filter(|(_, byte)| **byte == LINE_SEPARATOR)
            .map(|(position, _)| position + 1),
    )
}

#[cfg(test)]
mod patch_tests {
    use super::{assemble_patch, diff_reports_binary_file, hunk_spans, patch_header};
    use crate::error::GitError;

    const TWO_HUNK_DIFF: &[u8] = b"diff --git a/notes.txt b/notes.txt\nindex 1111111..2222222 100644\n--- a/notes.txt\n+++ b/notes.txt\n@@ -1,3 +1,3 @@\n first\n-second\n+second changed\n third\n@@ -20,3 +20,3 @@\n alpha\n-beta\n+beta changed\n omega\n";

    #[test]
    fn hunk_spans_cover_each_hunk_header_through_the_next_hunk() {
        let spans = hunk_spans(TWO_HUNK_DIFF);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].index, 0);
        assert_eq!(spans[1].index, 1);
        assert!(TWO_HUNK_DIFF[spans[0].start..].starts_with(b"@@ -1,3 +1,3 @@\n"));
        assert!(TWO_HUNK_DIFF[spans[1].start..].starts_with(b"@@ -20,3 +20,3 @@\n"));
        assert_eq!(spans[0].end, spans[1].start);
        assert_eq!(spans[1].end, TWO_HUNK_DIFF.len());
    }

    #[test]
    fn patch_header_stops_before_the_first_hunk() {
        assert_eq!(
            patch_header(TWO_HUNK_DIFF),
            b"diff --git a/notes.txt b/notes.txt\nindex 1111111..2222222 100644\n--- a/notes.txt\n+++ b/notes.txt\n"
        );
    }

    #[test]
    fn patch_slicing_keeps_hunk_headers_verbatim() {
        let spans = hunk_spans(TWO_HUNK_DIFF);
        let patch = assemble_patch(TWO_HUNK_DIFF, "notes.txt", &[1]).expect("hunk one is selected");
        let header = patch_header(TWO_HUNK_DIFF);
        assert_eq!(&patch[..header.len()], header);
        assert_eq!(
            &patch[header.len()..],
            &TWO_HUNK_DIFF[spans[1].start..spans[1].end]
        );
        assert_eq!(patch, [header, &TWO_HUNK_DIFF[spans[1].start..]].concat());
    }

    #[test]
    fn patch_slicing_keeps_carriage_returns() {
        let crlf_diff = b"diff --git a/notes.txt b/notes.txt\nindex 1111111..2222222 100644\n--- a/notes.txt\n+++ b/notes.txt\n@@ -1,2 +1,2 @@\n first\r\n-second\r\n+second changed\r\n";
        let patch = assemble_patch(crlf_diff, "notes.txt", &[0]).expect("hunk zero is selected");
        assert_eq!(patch, crlf_diff);
    }

    #[test]
    fn patch_slicing_orders_the_selected_hunks_ascending() {
        let patch = assemble_patch(TWO_HUNK_DIFF, "notes.txt", &[1, 0]).expect("both hunks");
        assert_eq!(patch, TWO_HUNK_DIFF);
    }

    #[test]
    fn patch_slicing_rejects_an_empty_selection() {
        let error = assemble_patch(TWO_HUNK_DIFF, "notes.txt", &[]).expect_err("empty selection");
        assert!(
            matches!(error, GitError::EmptyHunkSelection { .. }),
            "{error:?}"
        );
    }

    #[test]
    fn patch_slicing_rejects_an_index_past_the_last_hunk() {
        let error =
            assemble_patch(TWO_HUNK_DIFF, "notes.txt", &[2]).expect_err("index two is rejected");
        match error {
            GitError::HunkIndexOutOfRange {
                index, hunk_count, ..
            } => {
                assert_eq!(index, 2);
                assert_eq!(hunk_count, 2);
            }
            other => panic!("unexpected error {other:?}"),
        }
    }

    #[test]
    fn binary_notice_is_detected_on_its_own_line() {
        assert!(diff_reports_binary_file(
            b"diff --git a/logo.bin b/logo.bin\nindex 1111111..2222222 100644\nBinary files a/logo.bin and b/logo.bin differ\n"
        ));
        assert!(!diff_reports_binary_file(
            b"diff --git a/notes.txt b/notes.txt\n+Binary files are fine here\n"
        ));
    }
}
