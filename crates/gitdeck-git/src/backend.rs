use std::path::Path;

use gitdeck_core::{
    BranchEntry, ChangeEntry, CommitSummary, HeadInfo, LogFilter, LogQuery, RepoOperation,
};

use crate::changed_files::ChangedFile;
use crate::diff::FileDiff;
use crate::error::GitError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRequest {
    pub message: String,
    pub amend: bool,
    pub sign_off: bool,
    pub paths: Vec<String>,
    pub hunks: Vec<(String, Vec<usize>)>,
}

impl CommitRequest {
    pub fn rewrites_the_head_message(&self) -> bool {
        self.amend && self.paths.is_empty() && self.hunks.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitOutcome {
    pub commit_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PullStrategy {
    FastForwardOnly,
    Merge,
    Rebase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictResolution {
    Ours,
    Theirs,
    MarkResolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PushRequest {
    pub branch: String,
    pub set_upstream: bool,
    pub force_with_lease: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MergeOutcome {
    AlreadyUpToDate,
    FastForward { commit_id: String },
    Merged { commit_id: String },
    Conflicts { paths: Vec<String> },
}

pub trait GitBackend {
    fn repository_path(&self) -> &Path;
    fn head_info(&self) -> Result<HeadInfo, GitError>;
    fn changes(&self) -> Result<Vec<ChangeEntry>, GitError>;
    fn log(&self, query: LogQuery, filter: &LogFilter) -> Result<Vec<CommitSummary>, GitError>;
    fn branches(&self) -> Result<Vec<BranchEntry>, GitError>;
    fn branches_containing(&self, commit_id: &str) -> Result<Vec<BranchEntry>, GitError>;
    fn diff_working_tree(&self, path: &str) -> Result<FileDiff, GitError>;
    fn diff_commit_file(&self, commit_id: &str, path: &str) -> Result<FileDiff, GitError>;
    fn commit_changed_files(&self, commit_id: &str) -> Result<Vec<ChangedFile>, GitError>;
    fn last_commit_message(&self) -> Result<String, GitError>;
    fn commit(&self, request: CommitRequest) -> Result<CommitOutcome, GitError>;
    fn stage_paths(&self, paths: &[String]) -> Result<(), GitError>;
    fn stage_hunks(&self, path: &str, hunk_indices: &[usize]) -> Result<(), GitError>;
    fn stage_hunk_selection(&self, selection: &[(String, Vec<usize>)]) -> Result<(), GitError>;
    fn fetch(&self, remote: Option<&str>) -> Result<(), GitError>;
    fn pull(&self, strategy: PullStrategy) -> Result<(), GitError>;
    fn push_request(&self, request: PushRequest) -> Result<(), GitError>;
    fn push(&self, branch: &str) -> Result<(), GitError> {
        self.push_request(PushRequest {
            branch: branch.to_string(),
            set_upstream: false,
            force_with_lease: false,
        })
    }
    fn checkout_branch(&self, name: &str) -> Result<(), GitError>;
    fn create_branch(&self, name: &str, start_point: Option<&str>) -> Result<(), GitError>;
    fn merge_branch(&self, name: &str) -> Result<MergeOutcome, GitError>;
    fn delete_branch(&self, name: &str) -> Result<(), GitError>;
    fn ignore_path(&self, path: &str) -> Result<(), GitError>;
    fn resolve_conflict(&self, path: &str, resolution: ConflictResolution) -> Result<(), GitError>;
    fn current_operation(&self) -> Result<Option<RepoOperation>, GitError>;
    fn abort_operation(&self) -> Result<(), GitError>;
    fn continue_operation(&self) -> Result<(), GitError>;
    fn discard_changes(&self, path: &str) -> Result<(), GitError>;
}
