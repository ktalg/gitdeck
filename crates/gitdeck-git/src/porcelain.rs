use gitdeck_core::{ChangeEntry, ChangeKind, ConflictKind, HeadInfo};

use crate::decoding::decode_text;
use crate::error::GitError;

pub(crate) const STATUS_ARGUMENTS: [&str; 5] = [
    "status",
    "--porcelain=v2",
    "-z",
    "--untracked-files=all",
    "--branch",
];

const STATUS_RECORD_CONTEXT: &str = "status --porcelain=v2";
const ORDINARY_FIELD_COUNT: usize = 9;
const RENAME_FIELD_COUNT: usize = 10;
const UNMERGED_FIELD_COUNT: usize = 11;
const UNTRACKED_FIELD_COUNT: usize = 2;
const DETACHED_BRANCH_LABEL: &str = "(detached)";

pub(crate) struct StatusSnapshot {
    pub head: HeadInfo,
    pub changes: Vec<ChangeEntry>,
}

pub(crate) fn parse_status_porcelain_v2(output: &[u8]) -> Result<StatusSnapshot, GitError> {
    let mut snapshot = StatusSnapshot {
        head: HeadInfo::default(),
        changes: Vec::new(),
    };
    let mut records = output.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        match record[0] {
            b'#' => apply_branch_header(&mut snapshot.head, record)?,
            b'1' => snapshot.changes.push(parse_ordinary_record(record)?),
            b'2' => {
                let previous_path = records.next().ok_or_else(|| GitError::UnexpectedOutput {
                    context: STATUS_RECORD_CONTEXT,
                    message: String::from("rename record is missing the original path segment"),
                })?;
                snapshot
                    .changes
                    .push(parse_rename_record(record, previous_path)?);
            }
            b'u' => snapshot.changes.push(parse_unmerged_record(record)?),
            b'?' => snapshot.changes.push(parse_untracked_record(record)?),
            b'!' => {}
            record_type => {
                return Err(GitError::UnexpectedOutput {
                    context: STATUS_RECORD_CONTEXT,
                    message: format!("unknown record type {:?}", char::from(record_type)),
                })
            }
        }
    }
    Ok(snapshot)
}

fn apply_branch_header(head: &mut HeadInfo, header: &[u8]) -> Result<(), GitError> {
    let header = decode_text(header);
    if let Some(value) = header.strip_prefix("# branch.head ") {
        if value == DETACHED_BRANCH_LABEL {
            head.branch = None;
            head.detached = true;
        } else {
            head.branch = Some(value.to_string());
            head.detached = false;
        }
        return Ok(());
    }
    if let Some(value) = header.strip_prefix("# branch.upstream ") {
        head.upstream = Some(value.to_string());
        return Ok(());
    }
    if let Some(value) = header.strip_prefix("# branch.ab ") {
        let (ahead, behind) = parse_divergence_counts(value)?;
        head.ahead = ahead;
        head.behind = behind;
    }
    Ok(())
}

fn parse_divergence_counts(value: &str) -> Result<(u32, u32), GitError> {
    let mut counts = value.split(' ');
    let ahead = counts
        .next()
        .ok_or_else(|| malformed_status("branch.ab header is missing the ahead count"))?;
    let behind = counts
        .next()
        .ok_or_else(|| malformed_status("branch.ab header is missing the behind count"))?;
    if counts.next().is_some() {
        return Err(malformed_status(
            "branch.ab header holds more than two counts",
        ));
    }
    Ok((parse_count(ahead, '+')?, parse_count(behind, '-')?))
}

fn parse_count(token: &str, sign: char) -> Result<u32, GitError> {
    let digits = token
        .strip_prefix(sign)
        .ok_or_else(|| malformed_status("branch.ab header count is missing its sign"))?;
    digits
        .parse::<u32>()
        .map_err(|_| malformed_status("branch.ab header count is not an unsigned integer"))
}

fn parse_ordinary_record(record: &[u8]) -> Result<ChangeEntry, GitError> {
    let fields = split_record_fields(record, ORDINARY_FIELD_COUNT, "ordinary record")?;
    let (index_kind, worktree_kind) = parse_reported_change_kinds(fields[1], "ordinary record")?;
    Ok(ChangeEntry::from_status_codes(
        decode_text(fields[8]),
        None,
        index_kind,
        worktree_kind,
        None,
    ))
}

fn parse_rename_record(record: &[u8], previous_path: &[u8]) -> Result<ChangeEntry, GitError> {
    let fields = split_record_fields(record, RENAME_FIELD_COUNT, "rename record")?;
    if previous_path.is_empty() {
        return Err(malformed_status(
            "rename record holds an empty original path",
        ));
    }
    let (index_kind, worktree_kind) = parse_reported_change_kinds(fields[1], "rename record")?;
    Ok(ChangeEntry::from_status_codes(
        decode_text(fields[9]),
        Some(decode_text(previous_path)),
        index_kind,
        worktree_kind,
        None,
    ))
}

fn parse_unmerged_record(record: &[u8]) -> Result<ChangeEntry, GitError> {
    let fields = split_record_fields(record, UNMERGED_FIELD_COUNT, "unmerged record")?;
    let (index_code, worktree_code) = parse_status_pair(fields[1])?;
    Ok(ChangeEntry::from_status_codes(
        decode_text(fields[10]),
        None,
        None,
        None,
        Some(conflict_kind_of_status_pair(index_code, worktree_code)?),
    ))
}

fn parse_untracked_record(record: &[u8]) -> Result<ChangeEntry, GitError> {
    let fields = split_record_fields(record, UNTRACKED_FIELD_COUNT, "untracked record")?;
    Ok(ChangeEntry::untracked(decode_text(fields[1])))
}

fn split_record_fields<'record>(
    record: &'record [u8],
    field_count: usize,
    record_name: &str,
) -> Result<Vec<&'record [u8]>, GitError> {
    let fields: Vec<&[u8]> = record.splitn(field_count, |byte| *byte == b' ').collect();
    if fields.len() != field_count {
        return Err(malformed_status(format!(
            "{record_name} needs {field_count} space-separated fields, found {}",
            fields.len()
        )));
    }
    Ok(fields)
}

fn parse_status_pair(field: &[u8]) -> Result<(u8, u8), GitError> {
    match field {
        [index_code, worktree_code] => Ok((*index_code, *worktree_code)),
        other => Err(malformed_status(format!(
            "status codes must hold exactly two letters, found {:?}",
            decode_text(other)
        ))),
    }
}

fn parse_reported_change_kinds(
    field: &[u8],
    record_name: &str,
) -> Result<(Option<ChangeKind>, Option<ChangeKind>), GitError> {
    let (index_code, worktree_code) = parse_status_pair(field)?;
    let index_kind = change_kind_of_status_code(index_code)?;
    let worktree_kind = change_kind_of_status_code(worktree_code)?;
    if index_kind.is_none() && worktree_kind.is_none() {
        return Err(malformed_status(format!(
            "{record_name} reports no change in either the index or the worktree"
        )));
    }
    Ok((index_kind, worktree_kind))
}

fn change_kind_of_status_code(code: u8) -> Result<Option<ChangeKind>, GitError> {
    match code {
        b'.' => Ok(None),
        b'M' => Ok(Some(ChangeKind::Modified)),
        b'A' | b'C' => Ok(Some(ChangeKind::Added)),
        b'D' => Ok(Some(ChangeKind::Deleted)),
        b'R' => Ok(Some(ChangeKind::Renamed)),
        b'T' => Ok(Some(ChangeKind::TypeChanged)),
        other => Err(malformed_status(format!(
            "unsupported status letter {:?}",
            char::from(other)
        ))),
    }
}

fn conflict_kind_of_status_pair(
    index_code: u8,
    worktree_code: u8,
) -> Result<ConflictKind, GitError> {
    match (index_code, worktree_code) {
        (b'D', b'D') => Ok(ConflictKind::BothDeleted),
        (b'A', b'U') => Ok(ConflictKind::AddedByUs),
        (b'U', b'D') => Ok(ConflictKind::DeletedByThem),
        (b'U', b'A') => Ok(ConflictKind::AddedByThem),
        (b'D', b'U') => Ok(ConflictKind::DeletedByUs),
        (b'A', b'A') => Ok(ConflictKind::BothAdded),
        (b'U', b'U') => Ok(ConflictKind::BothModified),
        _ => Err(malformed_status(format!(
            "unsupported unmerged status pair {:?}",
            decode_text(&[index_code, worktree_code])
        ))),
    }
}

fn malformed_status(message: impl Into<String>) -> GitError {
    GitError::UnexpectedOutput {
        context: STATUS_RECORD_CONTEXT,
        message: message.into(),
    }
}

#[cfg(test)]
mod porcelain_tests {
    use gitdeck_core::{ChangeEntry, ChangeKind, ConflictKind, HeadInfo};

    use super::parse_status_porcelain_v2;

    const ORDINARY_PREFIX: &str =
        "1 {xy} N... 100644 100644 100644 1111111111111111111111111111111111111111 \
         2222222222222222222222222222222222222222 ";
    const RENAME_PREFIX: &str =
        "2 {xy} N... 100644 100644 100644 1111111111111111111111111111111111111111 \
         2222222222222222222222222222222222222222 R100 ";
    const UNMERGED_PREFIX: &str = "u {xy} N... 100644 100644 100644 100644 \
         1111111111111111111111111111111111111111 2222222222222222222222222222222222222222 \
         3333333333333333333333333333333333333333 ";
    const STATUS_LETTERS: [(&str, Option<ChangeKind>); 6] = [
        (".", None),
        ("M", Some(ChangeKind::Modified)),
        ("A", Some(ChangeKind::Added)),
        ("D", Some(ChangeKind::Deleted)),
        ("R", Some(ChangeKind::Renamed)),
        ("T", Some(ChangeKind::TypeChanged)),
    ];

    fn ordinary(xy: &str, path: &str) -> String {
        format!("{}\u{0}", ORDINARY_PREFIX.replace("{xy}", xy) + path)
    }

    fn rename(xy: &str, path: &str, previous_path: &str) -> String {
        format!(
            "{}{}\u{0}{previous_path}\u{0}",
            RENAME_PREFIX.replace("{xy}", xy),
            path
        )
    }

    fn unmerged(xy: &str, path: &str) -> String {
        format!("{}{path}\u{0}", UNMERGED_PREFIX.replace("{xy}", xy))
    }

    fn untracked(path: &str) -> String {
        format!("? {path}\u{0}")
    }

    fn branch_header(head: &str, upstream: Option<&str>, divergence: Option<(u32, u32)>) -> String {
        let mut output = String::from("# branch.oid 1111111111111111111111111111111111111111\u{0}");
        output.push_str(&format!("# branch.head {head}\u{0}"));
        if let Some(upstream) = upstream {
            output.push_str(&format!("# branch.upstream {upstream}\u{0}"));
        }
        if let Some((ahead, behind)) = divergence {
            output.push_str(&format!("# branch.ab +{ahead} -{behind}\u{0}"));
        }
        output
    }

    #[derive(Clone, Copy)]
    struct ExpectedStatus {
        index_kind: Option<ChangeKind>,
        worktree_kind: Option<ChangeKind>,
        conflict: Option<ConflictKind>,
        kind: ChangeKind,
        staged: bool,
    }

    fn status(
        index_kind: Option<ChangeKind>,
        worktree_kind: Option<ChangeKind>,
        kind: ChangeKind,
        staged: bool,
    ) -> ExpectedStatus {
        ExpectedStatus {
            index_kind,
            worktree_kind,
            conflict: None,
            kind,
            staged,
        }
    }

    fn conflicted(conflict: ConflictKind) -> ExpectedStatus {
        ExpectedStatus {
            index_kind: None,
            worktree_kind: None,
            conflict: Some(conflict),
            kind: ChangeKind::Conflicted,
            staged: false,
        }
    }

    fn single_change(output: &str, label: &str) -> ChangeEntry {
        let snapshot = parse_status_porcelain_v2(output.as_bytes())
            .unwrap_or_else(|error| panic!("{label} failed to parse: {error}"));
        assert_eq!(snapshot.changes.len(), 1, "{label} change count");
        snapshot
            .changes
            .into_iter()
            .next()
            .expect("a single change")
    }

    fn assert_status(entry: &ChangeEntry, expected: ExpectedStatus, label: &str) {
        assert_eq!(entry.index_kind, expected.index_kind, "{label} index kind");
        assert_eq!(
            entry.worktree_kind, expected.worktree_kind,
            "{label} worktree kind"
        );
        assert_eq!(entry.conflict, expected.conflict, "{label} conflict kind");
        assert_eq!(entry.kind, expected.kind, "{label} kind");
        assert_eq!(entry.staged, expected.staged, "{label} staged");
    }

    fn kind_of_letter(letter: &str) -> Option<ChangeKind> {
        STATUS_LETTERS
            .into_iter()
            .find(|(known, _)| *known == letter)
            .map(|(_, kind)| kind)
            .expect("a known status letter")
    }

    fn expected_matrix_status(index_letter: &str, worktree_letter: &str) -> ExpectedStatus {
        let index_kind = kind_of_letter(index_letter);
        let worktree_kind = kind_of_letter(worktree_letter);
        let reported_kind = if worktree_letter == "." {
            index_kind
        } else {
            worktree_kind
        }
        .expect("the matrix only reports records holding a change code");
        status(
            index_kind,
            worktree_kind,
            reported_kind,
            index_letter != ".",
        )
    }

    struct StatusCase {
        name: &'static str,
        output: String,
        path: &'static str,
        previous_path: Option<&'static str>,
        expected: ExpectedStatus,
    }

    fn status_cases() -> Vec<StatusCase> {
        vec![
            StatusCase {
                name: "worktree modification",
                output: ordinary(".M", "src/app.rs"),
                path: "src/app.rs",
                previous_path: None,
                expected: status(
                    None,
                    Some(ChangeKind::Modified),
                    ChangeKind::Modified,
                    false,
                ),
            },
            StatusCase {
                name: "staged addition",
                output: ordinary("A.", "src/module.rs"),
                path: "src/module.rs",
                previous_path: None,
                expected: status(Some(ChangeKind::Added), None, ChangeKind::Added, true),
            },
            StatusCase {
                name: "staged and unstaged modification",
                output: ordinary("MM", "src/app.rs"),
                path: "src/app.rs",
                previous_path: None,
                expected: status(
                    Some(ChangeKind::Modified),
                    Some(ChangeKind::Modified),
                    ChangeKind::Modified,
                    true,
                ),
            },
            StatusCase {
                name: "staged deletion",
                output: ordinary("D.", "docs/readme.md"),
                path: "docs/readme.md",
                previous_path: None,
                expected: status(Some(ChangeKind::Deleted), None, ChangeKind::Deleted, true),
            },
            StatusCase {
                name: "worktree type change",
                output: ordinary(".T", "scripts/run"),
                path: "scripts/run",
                previous_path: None,
                expected: status(
                    None,
                    Some(ChangeKind::TypeChanged),
                    ChangeKind::TypeChanged,
                    false,
                ),
            },
            StatusCase {
                name: "staged rename",
                output: rename("R.", "src/new_name.rs", "src/old_name.rs"),
                path: "src/new_name.rs",
                previous_path: Some("src/old_name.rs"),
                expected: status(Some(ChangeKind::Renamed), None, ChangeKind::Renamed, true),
            },
            StatusCase {
                name: "staged copy",
                output: rename("C.", "src/copy.rs", "src/source.rs"),
                path: "src/copy.rs",
                previous_path: Some("src/source.rs"),
                expected: status(Some(ChangeKind::Added), None, ChangeKind::Added, true),
            },
            StatusCase {
                name: "unmerged both modified",
                output: unmerged("UU", "src/conflict.txt"),
                path: "src/conflict.txt",
                previous_path: None,
                expected: conflicted(ConflictKind::BothModified),
            },
            StatusCase {
                name: "unmerged both deleted",
                output: unmerged("DD", "src/removed.txt"),
                path: "src/removed.txt",
                previous_path: None,
                expected: conflicted(ConflictKind::BothDeleted),
            },
            StatusCase {
                name: "untracked",
                output: untracked("notes.txt"),
                path: "notes.txt",
                previous_path: None,
                expected: status(
                    None,
                    Some(ChangeKind::Untracked),
                    ChangeKind::Untracked,
                    false,
                ),
            },
            StatusCase {
                name: "wide character path",
                output: untracked("\u{6587}\u{6863}/\u{8bf4}\u{660e}.md"),
                path: "\u{6587}\u{6863}/\u{8bf4}\u{660e}.md",
                previous_path: None,
                expected: status(
                    None,
                    Some(ChangeKind::Untracked),
                    ChangeKind::Untracked,
                    false,
                ),
            },
            StatusCase {
                name: "path holding a space",
                output: ordinary(".M", "docs/my notes.md"),
                path: "docs/my notes.md",
                previous_path: None,
                expected: status(
                    None,
                    Some(ChangeKind::Modified),
                    ChangeKind::Modified,
                    false,
                ),
            },
            StatusCase {
                name: "path holding spaces around a rename",
                output: rename(
                    "R.",
                    "\u{6587}\u{6863}/\u{65b0} \u{540d}\u{5b57}.md",
                    "\u{6587}\u{6863}/\u{65e7} \u{540d}\u{5b57}.md",
                ),
                path: "\u{6587}\u{6863}/\u{65b0} \u{540d}\u{5b57}.md",
                previous_path: Some("\u{6587}\u{6863}/\u{65e7} \u{540d}\u{5b57}.md"),
                expected: status(Some(ChangeKind::Renamed), None, ChangeKind::Renamed, true),
            },
        ]
    }

    #[test]
    fn parse_status_porcelain_v2_maps_each_record_shape() {
        for case in status_cases() {
            let entry = single_change(&case.output, case.name);
            assert_eq!(entry.path, case.path, "{} path", case.name);
            assert_eq!(
                entry.previous_path.as_deref(),
                case.previous_path,
                "{} previous path",
                case.name
            );
            assert_status(&entry, case.expected, case.name);
        }
    }

    #[test]
    fn status_matrix_covers_every_xy_combination() {
        for (index_letter, index_kind) in STATUS_LETTERS {
            for (worktree_letter, worktree_kind) in STATUS_LETTERS {
                let xy = format!("{index_letter}{worktree_letter}");
                let record = ordinary(&xy, "src/app.rs");
                if index_kind.is_none() && worktree_kind.is_none() {
                    assert!(
                        parse_status_porcelain_v2(record.as_bytes()).is_err(),
                        "{xy} parsed without a change code"
                    );
                    continue;
                }
                let entry = single_change(&record, &xy);
                assert_status(
                    &entry,
                    expected_matrix_status(index_letter, worktree_letter),
                    &xy,
                );
            }
        }
    }

    #[test]
    fn conflict_matrix_covers_all_seven_states() {
        for (xy, conflict) in [
            ("DD", ConflictKind::BothDeleted),
            ("AU", ConflictKind::AddedByUs),
            ("UD", ConflictKind::DeletedByThem),
            ("UA", ConflictKind::AddedByThem),
            ("DU", ConflictKind::DeletedByUs),
            ("AA", ConflictKind::BothAdded),
            ("UU", ConflictKind::BothModified),
        ] {
            let entry = single_change(&unmerged(xy, "src/conflict.txt"), xy);
            assert_eq!(entry.conflict, Some(conflict), "{xy} conflict kind");
            assert_eq!(entry.kind, ChangeKind::Conflicted, "{xy} kind");
            assert_eq!(entry.path, "src/conflict.txt", "{xy} path");
        }
    }

    #[test]
    fn a_file_staged_and_then_modified_keeps_both_kinds() {
        let entry = single_change(&ordinary("MM", "src/app.rs"), "MM");
        assert_eq!(entry.index_kind, Some(ChangeKind::Modified));
        assert_eq!(entry.worktree_kind, Some(ChangeKind::Modified));
        assert_eq!(entry.kind, ChangeKind::Modified);
        assert!(entry.staged);
    }

    #[test]
    fn an_unknown_conflict_combination_is_rejected() {
        for xy in ["MX", "MM", ".U", "XY"] {
            assert!(
                parse_status_porcelain_v2(unmerged(xy, "src/conflict.txt").as_bytes()).is_err(),
                "{xy} parsed as a conflict"
            );
        }
    }

    #[test]
    fn typechange_is_its_own_kind() {
        for (xy, staged) in [(".T", false), ("T.", true)] {
            let entry = single_change(&ordinary(xy, "scripts/run"), xy);
            assert_eq!(entry.kind, ChangeKind::TypeChanged, "{xy} kind");
            assert_eq!(entry.staged, staged, "{xy} staged");
        }
        let worktree_typechange = single_change(&ordinary(".T", "scripts/run"), ".T");
        assert_eq!(
            worktree_typechange.worktree_kind,
            Some(ChangeKind::TypeChanged)
        );
        assert_eq!(worktree_typechange.index_kind, None);
        let index_typechange = single_change(&ordinary("T.", "scripts/run"), "T.");
        assert_eq!(index_typechange.index_kind, Some(ChangeKind::TypeChanged));
        assert_eq!(index_typechange.worktree_kind, None);
    }

    #[test]
    fn parse_status_porcelain_v2_reads_branch_headers_with_divergence() {
        let mut output = branch_header("main", Some("origin/main"), Some((3, 7)));
        output.push_str(&ordinary(".M", "src/app.rs"));
        let snapshot = parse_status_porcelain_v2(output.as_bytes()).expect("headers parse");
        assert_eq!(
            snapshot.head,
            HeadInfo {
                branch: Some(String::from("main")),
                detached: false,
                upstream: Some(String::from("origin/main")),
                ahead: 3,
                behind: 7,
                merging: false,
                operation: None,
            }
        );
        assert_eq!(snapshot.changes.len(), 1);
    }

    #[test]
    fn parse_status_porcelain_v2_reads_detached_head_without_divergence() {
        let output = branch_header("(detached)", None, None);
        let snapshot = parse_status_porcelain_v2(output.as_bytes()).expect("headers parse");
        assert_eq!(
            snapshot.head,
            HeadInfo {
                branch: None,
                detached: true,
                upstream: None,
                ahead: 0,
                behind: 0,
                merging: false,
                operation: None,
            }
        );
        assert!(snapshot.changes.is_empty());
    }

    #[test]
    fn parse_status_porcelain_v2_keeps_only_the_branch_headers_it_models() {
        let mut output = String::from("# branch.oid (initial)\u{0}");
        output.push_str("# branch.head main\u{0}");
        output.push_str("# stash 2\u{0}");
        let snapshot = parse_status_porcelain_v2(output.as_bytes()).expect("headers parse");
        assert_eq!(snapshot.head.branch.as_deref(), Some("main"));
        assert!(!snapshot.head.detached);
    }

    #[test]
    fn parse_status_porcelain_v2_rejects_records_with_missing_fields() {
        for malformed in [
            "1 .M N... 100644 100644 100644 1111111111111111111111111111111111111111\u{0}",
            "u UU N... 100644\u{0}",
            "?\u{0}",
            "x bogus\u{0}",
            "1 M. N...\u{0}",
        ] {
            assert!(
                parse_status_porcelain_v2(malformed.as_bytes()).is_err(),
                "{malformed:?} parsed"
            );
        }
    }

    #[test]
    fn parse_status_porcelain_v2_rejects_a_rename_without_its_original_path() {
        let record = format!(
            "{}src/new_name.rs\u{0}",
            RENAME_PREFIX.replace("{xy}", "R.")
        );
        assert!(parse_status_porcelain_v2(record.as_bytes()).is_err());
    }

    #[test]
    fn parse_status_porcelain_v2_rejects_malformed_divergence_counts() {
        for header in [
            "# branch.ab +1\u{0}",
            "# branch.ab 1 -2\u{0}",
            "# branch.ab +one -2\u{0}",
            "# branch.ab +1 -2 +3\u{0}",
        ] {
            assert!(
                parse_status_porcelain_v2(header.as_bytes()).is_err(),
                "{header:?} parsed"
            );
        }
    }
}
