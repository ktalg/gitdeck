#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    TypeChanged,
    Conflicted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ConflictKind {
    BothDeleted,
    AddedByUs,
    DeletedByThem,
    AddedByThem,
    DeletedByUs,
    BothAdded,
    BothModified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeEntry {
    pub path: String,
    pub previous_path: Option<String>,
    pub kind: ChangeKind,
    pub staged: bool,
    pub index_kind: Option<ChangeKind>,
    pub worktree_kind: Option<ChangeKind>,
    pub conflict: Option<ConflictKind>,
}

impl ChangeEntry {
    pub fn from_status_codes(
        path: String,
        previous_path: Option<String>,
        index_kind: Option<ChangeKind>,
        worktree_kind: Option<ChangeKind>,
        conflict: Option<ConflictKind>,
    ) -> Self {
        Self {
            path,
            previous_path,
            kind: derive_change_kind(index_kind, worktree_kind, conflict),
            staged: index_kind.is_some(),
            index_kind,
            worktree_kind,
            conflict,
        }
    }

    pub fn unstaged(path: String, previous_path: Option<String>, kind: ChangeKind) -> Self {
        Self::from_status_codes(path, previous_path, None, Some(kind), None)
    }

    pub fn untracked(path: String) -> Self {
        Self::from_status_codes(path, None, None, Some(ChangeKind::Untracked), None)
    }
}

fn derive_change_kind(
    index_kind: Option<ChangeKind>,
    worktree_kind: Option<ChangeKind>,
    conflict: Option<ConflictKind>,
) -> ChangeKind {
    if conflict.is_some() {
        return ChangeKind::Conflicted;
    }
    match worktree_kind.or(index_kind) {
        Some(kind) => kind,
        None => ChangeKind::Modified,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RepoOperation {
    Merging,
    Rebasing,
    CherryPicking,
    Reverting,
}

impl RepoOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Merging => "MERGING",
            Self::Rebasing => "REBASING",
            Self::CherryPicking => "CHERRY-PICKING",
            Self::Reverting => "REVERTING",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Merging => "Merge",
            Self::Rebasing => "Rebase",
            Self::CherryPicking => "Cherry-pick",
            Self::Reverting => "Revert",
        }
    }

    pub const fn noun(self) -> &'static str {
        match self {
            Self::Merging => "merge",
            Self::Rebasing => "rebase",
            Self::CherryPicking => "cherry-pick",
            Self::Reverting => "revert",
        }
    }

    pub const fn continuation(self) -> Option<&'static str> {
        match self {
            Self::Merging => None,
            Self::Rebasing => Some("rebase"),
            Self::CherryPicking => Some("cherry-pick"),
            Self::Reverting => Some("revert"),
        }
    }

    pub const fn is_merging(self) -> bool {
        matches!(self, Self::Merging)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HeadInfo {
    pub branch: Option<String>,
    pub detached: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub merging: bool,
    pub operation: Option<RepoOperation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RefKind {
    Head,
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefName {
    pub full: String,
    pub short: String,
    pub kind: RefKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitSummary {
    pub id: String,
    pub short_id: String,
    pub subject: String,
    pub author_name: String,
    pub author_email: String,
    pub author_timestamp: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_timestamp: i64,
    pub parent_ids: Vec<String>,
    pub refs: Vec<RefName>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct LogQuery {
    pub max_commits: usize,
    pub skip: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogFilter {
    pub branch: Option<String>,
    pub text: Option<String>,
    pub case_sensitive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchEntry {
    pub name: String,
    pub kind: RefKind,
    pub is_head: bool,
    pub upstream: Option<String>,
}

#[cfg(test)]
mod repo_model_tests {
    use super::{ChangeEntry, ChangeKind, ConflictKind, HeadInfo, LogQuery};

    #[test]
    fn default_head_info_holds_no_branch_and_no_divergence() {
        assert_eq!(
            HeadInfo::default(),
            HeadInfo {
                branch: None,
                detached: false,
                upstream: None,
                ahead: 0,
                behind: 0,
                merging: false,
                operation: None,
            }
        );
    }

    #[test]
    fn change_entry_compares_by_path_kind_and_staging() {
        let staged_add = ChangeEntry::from_status_codes(
            String::from("src/module.rs"),
            None,
            Some(ChangeKind::Added),
            None,
            None,
        );
        let unstaged_add =
            ChangeEntry::unstaged(String::from("src/module.rs"), None, ChangeKind::Added);
        let staged_modification = ChangeEntry::from_status_codes(
            String::from("src/module.rs"),
            None,
            Some(ChangeKind::Modified),
            None,
            None,
        );
        let staged_rename = ChangeEntry::from_status_codes(
            String::from("src/renamed.rs"),
            Some(String::from("src/module.rs")),
            Some(ChangeKind::Renamed),
            None,
            None,
        );
        assert_eq!(
            (
                staged_add.kind,
                staged_add.staged,
                staged_add.index_kind,
                staged_add.worktree_kind
            ),
            (ChangeKind::Added, true, Some(ChangeKind::Added), None)
        );
        assert_ne!(staged_add, unstaged_add);
        assert_ne!(staged_add, staged_modification);
        assert_ne!(staged_add, staged_rename);
    }

    #[test]
    fn change_entry_derives_kind_from_conflict_before_status_codes() {
        let conflicted = ChangeEntry::from_status_codes(
            String::from("src/conflict.txt"),
            None,
            None,
            None,
            Some(ConflictKind::BothModified),
        );
        assert_eq!(conflicted.kind, ChangeKind::Conflicted);
        assert!(!conflicted.staged);
        assert_eq!(conflicted.conflict, Some(ConflictKind::BothModified));

        let untracked = ChangeEntry::untracked(String::from("notes.txt"));
        assert_eq!(untracked.kind, ChangeKind::Untracked);
        assert!(!untracked.staged);
        assert_eq!(untracked.index_kind, None);
        assert_eq!(untracked.worktree_kind, Some(ChangeKind::Untracked));
    }

    #[test]
    fn log_query_copies_and_hashes_by_page_bounds() {
        let query = LogQuery {
            max_commits: 200,
            skip: 400,
        };
        let copied = query;
        assert_eq!(copied, query);
        let mut seen = std::collections::HashSet::new();
        seen.insert(query);
        assert!(seen.contains(&copied));
    }
}
