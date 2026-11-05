use std::collections::HashMap;

use gitdeck_core::{BranchEntry, CommitSummary, RefKind, RefName};

use crate::decoding::decode_text;
use crate::error::GitError;

const FIELD_SEPARATOR: u8 = 0x1f;
const LOG_REF_FIELD_COUNT: usize = 4;
const BRANCH_FIELD_COUNT: usize = 5;
const HEAD_MARKER: &[u8] = b"*";
const SYMBOLIC_REF_SUFFIX: &str = "/HEAD";
const HEAD_REFERENCE_NAME: &str = "HEAD";
const REF_CONTEXT: &str = "for-each-ref";

pub(crate) const LOG_REF_FORMAT_ARGUMENT: &str =
    "--format=%(objectname)%1f%(refname)%1f%(refname:short)%1f%(HEAD)";
pub(crate) const BRANCH_FORMAT_ARGUMENT: &str =
    "--format=%(objectname)%1f%(refname)%1f%(refname:short)%1f%(HEAD)%1f%(upstream:short)";

pub(crate) struct CommitRef {
    pub object_id: String,
    pub name: RefName,
}

pub(crate) fn parse_log_refs(output: &[u8]) -> Result<Vec<CommitRef>, GitError> {
    let mut commit_refs = Vec::new();
    for line in listing_lines(output) {
        let fields = split_listing_fields(line, LOG_REF_FIELD_COUNT)?;
        let full = decode_text(fields[1]);
        let Some(kind) = ref_kind_from_full_name(&full) else {
            continue;
        };
        commit_refs.push(CommitRef {
            object_id: decode_text(fields[0]),
            name: RefName {
                full,
                short: decode_text(fields[2]),
                kind,
            },
        });
    }
    Ok(commit_refs)
}

pub(crate) fn parse_branch_listing(output: &[u8]) -> Result<Vec<BranchEntry>, GitError> {
    let mut branches = Vec::new();
    for line in listing_lines(output) {
        let fields = split_listing_fields(line, BRANCH_FIELD_COUNT)?;
        let full = decode_text(fields[1]);
        if full.ends_with(SYMBOLIC_REF_SUFFIX) {
            continue;
        }
        let Some(kind) = ref_kind_from_full_name(&full) else {
            continue;
        };
        let upstream = decode_text(fields[4]);
        branches.push(BranchEntry {
            name: decode_text(fields[2]),
            kind,
            is_head: fields[3] == HEAD_MARKER,
            upstream: if upstream.is_empty() {
                None
            } else {
                Some(upstream)
            },
        });
    }
    Ok(branches)
}

pub(crate) fn sort_containing_branches_head_first(branches: &mut [BranchEntry]) {
    branches.sort_by(|left, right| {
        branch_containment_rank(left)
            .cmp(&branch_containment_rank(right))
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn branch_containment_rank(branch: &BranchEntry) -> u8 {
    if branch.is_head && branch.kind == RefKind::LocalBranch {
        return 0;
    }
    match branch.kind {
        RefKind::LocalBranch => 1,
        RefKind::RemoteBranch => 2,
        RefKind::Head | RefKind::Tag => 3,
    }
}

pub(crate) fn attach_refs_to_commits(
    commits: &mut [CommitSummary],
    commit_refs: Vec<CommitRef>,
    head_commit_id: Option<&str>,
) {
    let index_by_commit_id: HashMap<String, usize> = commits
        .iter()
        .enumerate()
        .map(|(index, commit)| (commit.id.clone(), index))
        .collect();
    let mut refs_by_commit: Vec<Vec<RefName>> = vec![Vec::new(); commits.len()];
    if let Some(index) = head_commit_id.and_then(|id| index_by_commit_id.get(id)) {
        refs_by_commit[*index].push(RefName {
            full: String::from(HEAD_REFERENCE_NAME),
            short: String::from(HEAD_REFERENCE_NAME),
            kind: RefKind::Head,
        });
    }
    for commit_ref in commit_refs {
        if let Some(index) = index_by_commit_id.get(&commit_ref.object_id) {
            refs_by_commit[*index].push(commit_ref.name);
        }
    }
    for (commit, refs) in commits.iter_mut().zip(refs_by_commit) {
        commit.refs = refs;
    }
}

fn listing_lines(output: &[u8]) -> impl Iterator<Item = &[u8]> {
    output
        .split(|byte| *byte == b'\n')
        .map(|line| match line.split_last() {
            Some((b'\r', rest)) => rest,
            _ => line,
        })
        .filter(|line| !line.is_empty())
}

fn split_listing_fields(line: &[u8], field_count: usize) -> Result<Vec<&[u8]>, GitError> {
    let fields: Vec<&[u8]> = line.split(|byte| *byte == FIELD_SEPARATOR).collect();
    if fields.len() != field_count {
        return Err(GitError::UnexpectedOutput {
            context: REF_CONTEXT,
            message: format!(
                "line needs {field_count} unit separated fields, found {}",
                fields.len()
            ),
        });
    }
    Ok(fields)
}

fn ref_kind_from_full_name(full: &str) -> Option<RefKind> {
    if full.starts_with("refs/heads/") {
        Some(RefKind::LocalBranch)
    } else if full.starts_with("refs/remotes/") {
        Some(RefKind::RemoteBranch)
    } else if full.starts_with("refs/tags/") {
        Some(RefKind::Tag)
    } else {
        None
    }
}

#[cfg(test)]
mod ref_listing_tests {
    use gitdeck_core::{BranchEntry, CommitSummary, RefKind, RefName};

    use super::{
        attach_refs_to_commits, parse_branch_listing, parse_log_refs,
        sort_containing_branches_head_first, CommitRef,
    };

    const COMMIT_ID: &str = "1111111111111111111111111111111111111111";
    const OTHER_COMMIT_ID: &str = "2222222222222222222222222222222222222222";

    fn log_ref_line(object_id: &str, full: &str, short: &str, head_marker: &str) -> String {
        format!("{object_id}\u{1f}{full}\u{1f}{short}\u{1f}{head_marker}\n")
    }

    fn branch_line(
        object_id: &str,
        full: &str,
        short: &str,
        head_marker: &str,
        upstream: &str,
    ) -> String {
        format!("{object_id}\u{1f}{full}\u{1f}{short}\u{1f}{head_marker}\u{1f}{upstream}\n")
    }

    fn commit_summary(id: &str) -> CommitSummary {
        CommitSummary {
            id: String::from(id),
            short_id: String::from(&id[..7]),
            subject: String::from("subject"),
            author_name: String::from("gitdeck-fixture"),
            author_email: String::from("fixture@gitdeck.invalid"),
            author_timestamp: 1_577_836_800,
            committer_name: String::from("gitdeck-fixture"),
            committer_email: String::from("fixture@gitdeck.invalid"),
            committer_timestamp: 1_577_836_800,
            parent_ids: Vec::new(),
            refs: Vec::new(),
        }
    }

    #[test]
    fn parse_log_refs_maps_every_ref_namespace() {
        let mut output = log_ref_line(COMMIT_ID, "refs/heads/main", "main", "*");
        output.push_str(&log_ref_line(
            COMMIT_ID,
            "refs/remotes/origin/main",
            "origin/main",
            " ",
        ));
        output.push_str(&log_ref_line(COMMIT_ID, "refs/tags/v0.1.0", "v0.1.0", " "));
        let commit_refs = parse_log_refs(output.as_bytes()).expect("refs parse");
        let kinds: Vec<RefKind> = commit_refs
            .iter()
            .map(|commit_ref| commit_ref.name.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![RefKind::LocalBranch, RefKind::RemoteBranch, RefKind::Tag]
        );
        assert_eq!(commit_refs[1].name.full, "refs/remotes/origin/main");
        assert_eq!(commit_refs[1].name.short, "origin/main");
    }

    #[test]
    fn parse_log_refs_keeps_wide_characters_of_a_branch_name() {
        let output = log_ref_line(COMMIT_ID, "refs/heads/功能/中文分支", "功能/中文分支", " ");
        let commit_refs = parse_log_refs(output.as_bytes()).expect("refs parse");
        assert_eq!(commit_refs[0].name.short, "功能/中文分支");
    }

    #[test]
    fn parse_log_refs_rejects_lines_with_a_wrong_field_count() {
        assert!(
            parse_log_refs(b"1111111111111111111111111111111111111111\x1frefs/heads/main\n")
                .is_err()
        );
    }

    #[test]
    fn parse_branch_listing_marks_only_the_starred_ref_as_head() {
        let mut output = branch_line(COMMIT_ID, "refs/heads/main", "main", "*", "origin/main");
        output.push_str(&branch_line(
            OTHER_COMMIT_ID,
            "refs/heads/feature",
            "feature",
            " ",
            "",
        ));
        let branches = parse_branch_listing(output.as_bytes()).expect("branches parse");
        assert_eq!(
            branches,
            vec![
                BranchEntry {
                    name: String::from("main"),
                    kind: RefKind::LocalBranch,
                    is_head: true,
                    upstream: Some(String::from("origin/main")),
                },
                BranchEntry {
                    name: String::from("feature"),
                    kind: RefKind::LocalBranch,
                    is_head: false,
                    upstream: None,
                },
            ]
        );
    }

    #[test]
    fn parse_branch_listing_excludes_the_symbolic_remote_head_ref() {
        let mut output = branch_line(
            COMMIT_ID,
            "refs/remotes/origin/HEAD",
            "origin/HEAD",
            " ",
            "",
        );
        output.push_str(&branch_line(
            COMMIT_ID,
            "refs/remotes/origin/main",
            "origin/main",
            " ",
            "",
        ));
        let branches = parse_branch_listing(output.as_bytes()).expect("branches parse");
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "origin/main");
        assert_eq!(branches[0].kind, RefKind::RemoteBranch);
    }

    #[test]
    fn parse_branch_listing_rejects_lines_with_a_wrong_field_count() {
        assert!(parse_branch_listing(
            b"1111111111111111111111111111111111111111\x1frefs/heads/main\n"
        )
        .is_err());
    }

    #[test]
    fn sort_containing_branches_head_first_lists_locals_then_remotes_by_name() {
        let mut branches = vec![
            BranchEntry {
                name: String::from("origin/topic"),
                kind: RefKind::RemoteBranch,
                is_head: false,
                upstream: None,
            },
            BranchEntry {
                name: String::from("topic"),
                kind: RefKind::LocalBranch,
                is_head: false,
                upstream: None,
            },
            BranchEntry {
                name: String::from("main"),
                kind: RefKind::LocalBranch,
                is_head: true,
                upstream: None,
            },
            BranchEntry {
                name: String::from("origin/main"),
                kind: RefKind::RemoteBranch,
                is_head: false,
                upstream: None,
            },
            BranchEntry {
                name: String::from("alpha"),
                kind: RefKind::LocalBranch,
                is_head: false,
                upstream: None,
            },
        ];
        sort_containing_branches_head_first(&mut branches);
        let names: Vec<&str> = branches.iter().map(|branch| branch.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["main", "alpha", "topic", "origin/main", "origin/topic"]
        );
    }

    #[test]
    fn attach_refs_to_commits_puts_the_head_reference_first_on_the_head_commit() {
        let mut commits = vec![commit_summary(COMMIT_ID), commit_summary(OTHER_COMMIT_ID)];
        let commit_refs = vec![CommitRef {
            object_id: String::from(OTHER_COMMIT_ID),
            name: RefName {
                full: String::from("refs/heads/side"),
                short: String::from("side"),
                kind: RefKind::LocalBranch,
            },
        }];
        attach_refs_to_commits(&mut commits, commit_refs, Some(COMMIT_ID));
        let head_ref_names: Vec<(String, RefKind)> = commits[0]
            .refs
            .iter()
            .map(|name| (name.full.clone(), name.kind))
            .collect();
        assert_eq!(head_ref_names, vec![(String::from("HEAD"), RefKind::Head)]);
        assert_eq!(commits[1].refs.len(), 1);
        assert_eq!(commits[1].refs[0].short, "side");
    }

    #[test]
    fn attach_refs_to_commits_ignores_refs_outside_the_loaded_page() {
        let mut commits = vec![commit_summary(COMMIT_ID)];
        let commit_refs = vec![CommitRef {
            object_id: String::from(OTHER_COMMIT_ID),
            name: RefName {
                full: String::from("refs/heads/side"),
                short: String::from("side"),
                kind: RefKind::LocalBranch,
            },
        }];
        attach_refs_to_commits(&mut commits, commit_refs, Some(OTHER_COMMIT_ID));
        assert!(commits[0].refs.is_empty());
    }
}
