use crate::decoding::decode_text;
use crate::error::GitError;

const DIFF_OUTPUT_CONTEXT: &str = "diff";
const HUNK_HEADER_PREFIX: &str = "@@ -";
const HUNK_HEADER_RANGE_SEPARATOR: &str = " +";
const HUNK_HEADER_TERMINATOR: &str = " @@";
const HUNK_LINE_COUNT_SEPARATOR: char = ',';
const BINARY_FILE_NOTICE_PREFIX: &str = "Binary files ";
const NO_NEWLINE_MARKER: &str = "\\ No newline at end of file";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line_number: Option<u32>,
    pub new_line_number: Option<u32>,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileDiff {
    pub path: String,
    pub binary: bool,
    pub missing_newline_at_end: bool,
    pub hunks: Vec<DiffHunk>,
}

impl FileDiff {
    fn without_hunks(path: &str) -> Self {
        Self {
            path: path.to_string(),
            binary: false,
            missing_newline_at_end: false,
            hunks: Vec::new(),
        }
    }
}

struct HunkParser {
    diff: FileDiff,
    open_hunk: Option<DiffHunk>,
    next_old_line_number: u32,
    next_new_line_number: u32,
}

pub(crate) fn parse_file_diff(path: &str, output: &[u8]) -> Result<FileDiff, GitError> {
    let mut parser = HunkParser {
        diff: FileDiff::without_hunks(path),
        open_hunk: None,
        next_old_line_number: 0,
        next_new_line_number: 0,
    };
    for raw_line in decode_text(output).split('\n') {
        if parser.apply_line(strip_carriage_return(raw_line))? {
            return Ok(parser.into_binary_diff());
        }
    }
    Ok(parser.finish())
}

fn strip_carriage_return(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

impl HunkParser {
    fn apply_line(&mut self, line: &str) -> Result<bool, GitError> {
        if line.starts_with(BINARY_FILE_NOTICE_PREFIX) {
            return Ok(true);
        }
        if let Some(hunk_range) = line.strip_prefix(HUNK_HEADER_PREFIX) {
            self.open_hunk(hunk_range)?;
            return Ok(false);
        }
        if self.open_hunk.is_none() {
            return Ok(false);
        }
        self.apply_hunk_line(line);
        Ok(false)
    }

    fn open_hunk(&mut self, hunk_range: &str) -> Result<(), GitError> {
        let (old_start, old_lines, new_start, new_lines) = parse_hunk_range(hunk_range)?;
        if let Some(finished_hunk) = self.open_hunk.take() {
            self.diff.hunks.push(finished_hunk);
        }
        self.next_old_line_number = old_start;
        self.next_new_line_number = new_start;
        self.open_hunk = Some(DiffHunk {
            old_start,
            old_lines,
            new_start,
            new_lines,
            lines: Vec::new(),
        });
        Ok(())
    }

    fn apply_hunk_line(&mut self, line: &str) {
        let line_kind = match line.chars().next() {
            Some('+') => Some(DiffLineKind::Added),
            Some('-') => Some(DiffLineKind::Removed),
            Some(' ') => Some(DiffLineKind::Context),
            Some(_) | None => None,
        };
        let Some(line_kind) = line_kind else {
            if line == NO_NEWLINE_MARKER {
                self.diff.missing_newline_at_end = true;
                self.push_line(DiffLineKind::Context, None, None, line);
            }
            return;
        };
        let (old_line_number, new_line_number) = self.take_line_numbers(line_kind);
        self.push_line(line_kind, old_line_number, new_line_number, line);
    }

    fn take_line_numbers(&mut self, line_kind: DiffLineKind) -> (Option<u32>, Option<u32>) {
        let old_line_number = matches!(line_kind, DiffLineKind::Context | DiffLineKind::Removed)
            .then(|| take_next_line_number(&mut self.next_old_line_number));
        let new_line_number = matches!(line_kind, DiffLineKind::Context | DiffLineKind::Added)
            .then(|| take_next_line_number(&mut self.next_new_line_number));
        (old_line_number, new_line_number)
    }

    fn push_line(
        &mut self,
        line_kind: DiffLineKind,
        old_line_number: Option<u32>,
        new_line_number: Option<u32>,
        line: &str,
    ) {
        let Some(open_hunk) = self.open_hunk.as_mut() else {
            return;
        };
        open_hunk.lines.push(DiffLine {
            kind: line_kind,
            old_line_number,
            new_line_number,
            text: line_text(line, line_kind),
        });
    }

    fn finish(mut self) -> FileDiff {
        if let Some(finished_hunk) = self.open_hunk.take() {
            self.diff.hunks.push(finished_hunk);
        }
        self.diff
    }

    fn into_binary_diff(mut self) -> FileDiff {
        self.diff.binary = true;
        self.diff.hunks.clear();
        self.diff.missing_newline_at_end = false;
        self.diff
    }
}

fn take_next_line_number(next_line_number: &mut u32) -> u32 {
    let line_number = *next_line_number;
    *next_line_number = next_line_number.saturating_add(1);
    line_number
}

fn line_text(line: &str, line_kind: DiffLineKind) -> String {
    match line_kind {
        DiffLineKind::Context => line.strip_prefix(' ').unwrap_or(line).to_string(),
        DiffLineKind::Added | DiffLineKind::Removed => {
            let mut characters = line.chars();
            characters.next();
            characters.as_str().to_string()
        }
    }
}

fn parse_hunk_range(hunk_range: &str) -> Result<(u32, u32, u32, u32), GitError> {
    let (old_range, remaining_range) = hunk_range
        .split_once(HUNK_HEADER_RANGE_SEPARATOR)
        .ok_or_else(|| malformed_hunk_range(hunk_range))?;
    let (new_range, _section) = remaining_range
        .split_once(HUNK_HEADER_TERMINATOR)
        .ok_or_else(|| malformed_hunk_range(hunk_range))?;
    let (old_start, old_lines) = parse_line_count(old_range)?;
    let (new_start, new_lines) = parse_line_count(new_range)?;
    Ok((old_start, old_lines, new_start, new_lines))
}

fn parse_line_count(line_range: &str) -> Result<(u32, u32), GitError> {
    let (start, lines) = match line_range.split_once(HUNK_LINE_COUNT_SEPARATOR) {
        Some((start, lines)) => (start, lines),
        None => (line_range, "1"),
    };
    let start = start
        .parse::<u32>()
        .map_err(|_| malformed_hunk_range(line_range))?;
    let lines = lines
        .parse::<u32>()
        .map_err(|_| malformed_hunk_range(line_range))?;
    Ok((start, lines))
}

fn malformed_hunk_range(hunk_range: &str) -> GitError {
    GitError::UnexpectedOutput {
        context: DIFF_OUTPUT_CONTEXT,
        message: format!("hunk header holds an unreadable line range {hunk_range:?}"),
    }
}

#[cfg(test)]
mod diff_tests {
    use std::time::{Duration, Instant};

    use crate::diff::{parse_file_diff, DiffHunk, DiffLine, DiffLineKind, FileDiff};

    const MODIFIED_DIFF: &str = concat!(
        "diff --git a/src/app.rs b/src/app.rs\n",
        "index 1111111..2222222 100644\n",
        "--- a/src/app.rs\n",
        "+++ b/src/app.rs\n",
        "@@ -1,3 +1,3 @@\n",
        " fn main() {\n",
        "-    run();\n",
        "+    run_twice();\n",
        " }\n",
    );

    const LARGE_DIFF_TIME_LIMIT: Duration = Duration::from_millis(100);
    const LARGE_DIFF_LINE_COUNT: u32 = 5_000;
    const LARGE_DIFF_CHANGED_LINE_COUNT: u32 = 10;

    fn counted_diff() -> String {
        let mut output = String::from(
            "diff --git a/src/large.txt b/src/large.txt\n\
--- a/src/large.txt\n\
+++ b/src/large.txt\n\
@@ -1,5000 +1,5000 @@\n",
        );
        for line_number in 1..=LARGE_DIFF_LINE_COUNT {
            let marker = if line_number % 500 == 0 { '+' } else { ' ' };
            output.push_str(&format!("{marker}line {line_number}\n"));
        }
        output
    }

    #[test]
    fn context_added_and_removed_lines_carry_their_line_numbers() {
        let diff = parse_file_diff("src/app.rs", MODIFIED_DIFF.as_bytes()).expect("diff parses");
        assert_eq!(
            diff,
            FileDiff {
                path: String::from("src/app.rs"),
                binary: false,
                missing_newline_at_end: false,
                hunks: vec![DiffHunk {
                    old_start: 1,
                    old_lines: 3,
                    new_start: 1,
                    new_lines: 3,
                    lines: vec![
                        DiffLine {
                            kind: DiffLineKind::Context,
                            old_line_number: Some(1),
                            new_line_number: Some(1),
                            text: String::from("fn main() {"),
                        },
                        DiffLine {
                            kind: DiffLineKind::Removed,
                            old_line_number: Some(2),
                            new_line_number: None,
                            text: String::from("    run();"),
                        },
                        DiffLine {
                            kind: DiffLineKind::Added,
                            old_line_number: None,
                            new_line_number: Some(2),
                            text: String::from("    run_twice();"),
                        },
                        DiffLine {
                            kind: DiffLineKind::Context,
                            old_line_number: Some(3),
                            new_line_number: Some(3),
                            text: String::from("}"),
                        },
                    ],
                }],
            }
        );
    }

    #[test]
    fn added_file_counts_every_line_as_added_without_old_line_numbers() {
        let output = "diff --git a/src/new.rs b/src/new.rs\n\
new file mode 100644\n\
--- /dev/null\n\
+++ b/src/new.rs\n\
@@ -0,0 +1,2 @@\n\
+first\n\
+second\n";
        let diff = parse_file_diff("src/new.rs", output.as_bytes()).expect("diff parses");
        assert_eq!(diff.hunks.len(), 1);
        let hunk = &diff.hunks[0];
        assert_eq!((hunk.old_start, hunk.old_lines), (0, 0));
        assert_eq!((hunk.new_start, hunk.new_lines), (1, 2));
        assert_eq!(hunk.lines.len(), 2);
        assert!(hunk
            .lines
            .iter()
            .all(|line| line.kind == DiffLineKind::Added && line.old_line_number.is_none()));
        assert_eq!(hunk.lines[1].new_line_number, Some(2));
    }

    #[test]
    fn carriage_returns_are_dropped_from_the_line_text() {
        let output = "@@ -1,1 +1,1 @@\n\
-old line\r\n\
+new line\r\n";
        let diff = parse_file_diff("src/crlf.txt", output.as_bytes()).expect("diff parses");
        let texts: Vec<&str> = diff.hunks[0]
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(texts, vec!["old line", "new line"]);
    }

    #[test]
    fn missing_trailing_newline_marker_is_recorded_without_line_numbers() {
        let output = concat!(
            "@@ -1,2 +1,2 @@\n",
            " first\n",
            "-last\n",
            "\\ No newline at end of file\n",
            "+last\n",
            "\\ No newline at end of file\n",
        );
        let diff = parse_file_diff("src/last.txt", output.as_bytes()).expect("diff parses");
        assert!(diff.missing_newline_at_end);
        assert_eq!(diff.hunks[0].lines[0].text, "first");
        let marker_lines: Vec<&DiffLine> = diff.hunks[0]
            .lines
            .iter()
            .filter(|line| line.old_line_number.is_none() && line.new_line_number.is_none())
            .collect();
        assert_eq!(marker_lines.len(), 2);
        assert!(marker_lines
            .iter()
            .all(|line| line.text == "\\ No newline at end of file"));
    }

    #[test]
    fn binary_notice_yields_a_binary_diff_without_hunks() {
        let output = "diff --git a/assets/logo.bin b/assets/logo.bin\n\
index 1111111..2222222 100644\n\
Binary files a/assets/logo.bin and b/assets/logo.bin differ\n";
        let diff = parse_file_diff("assets/logo.bin", output.as_bytes()).expect("diff parses");
        assert!(diff.binary);
        assert!(diff.hunks.is_empty());
        assert_eq!(diff.path, "assets/logo.bin");
    }

    #[test]
    fn identical_files_yield_no_hunks() {
        let diff = parse_file_diff("src/app.rs", b"").expect("empty diff parses");
        assert!(!diff.binary);
        assert!(diff.hunks.is_empty());
        assert!(!diff.missing_newline_at_end);
    }

    #[test]
    fn multiple_hunks_restart_the_line_counters() {
        let output = "@@ -1,1 +1,1 @@\n\
-first\n\
+first changed\n\
@@ -10,1 +10,1 @@\n\
-tenth\n\
+tenth changed\n";
        let diff = parse_file_diff("src/app.rs", output.as_bytes()).expect("diff parses");
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.hunks[0].lines[1].new_line_number, Some(1));
        assert_eq!(diff.hunks[1].lines[0].old_line_number, Some(10));
        assert_eq!(diff.hunks[1].lines[1].new_line_number, Some(10));
    }

    #[test]
    fn hunk_header_without_line_counts_counts_one_line() {
        let output = "@@ -3 +3 @@\n\
-old\n\
+new\n";
        let diff = parse_file_diff("src/app.rs", output.as_bytes()).expect("diff parses");
        assert_eq!((diff.hunks[0].old_start, diff.hunks[0].old_lines), (3, 1));
        assert_eq!((diff.hunks[0].new_start, diff.hunks[0].new_lines), (3, 1));
    }

    #[test]
    fn malformed_hunk_headers_are_rejected() {
        for output in [
            "@@ -a,b +1,1 @@\n+a\n",
            "@@ -1,1\n+a\n",
            "@@ -1,1 -1,1 @@\n+a\n",
        ] {
            assert!(
                parse_file_diff("src/app.rs", output.as_bytes()).is_err(),
                "{output:?} parsed"
            );
        }
    }

    #[test]
    fn wide_character_line_text_survives_the_parser() {
        let output = "@@ -1,1 +1,1 @@\n\
-旧的一行\n\
+新的一行\n";
        let diff = parse_file_diff("文档/说明.md", output.as_bytes()).expect("diff parses");
        assert_eq!(diff.path, "文档/说明.md");
        assert_eq!(diff.hunks[0].lines[0].text, "旧的一行");
        assert_eq!(diff.hunks[0].lines[1].text, "新的一行");
    }

    #[test]
    fn large_diff_parses_within_the_time_budget() {
        let output = counted_diff();
        let started_at = Instant::now();
        let diff = parse_file_diff("src/large.txt", output.as_bytes()).expect("diff parses");
        let elapsed = started_at.elapsed();
        println!(
            "{LARGE_DIFF_LINE_COUNT} diff lines parsed in {elapsed:?}, limit is {LARGE_DIFF_TIME_LIMIT:?}"
        );
        assert!(
            elapsed < LARGE_DIFF_TIME_LIMIT,
            "parsing took {elapsed:?}, limit is {LARGE_DIFF_TIME_LIMIT:?}"
        );
        assert_eq!(diff.hunks[0].lines.len(), LARGE_DIFF_LINE_COUNT as usize);
        let added_count = diff.hunks[0]
            .lines
            .iter()
            .filter(|line| line.kind == DiffLineKind::Added)
            .count();
        assert_eq!(added_count, LARGE_DIFF_CHANGED_LINE_COUNT as usize);
    }
}
