use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git {arguments} failed in {path}: {message}")]
    CommandFailed {
        path: PathBuf,
        arguments: String,
        message: String,
    },
    #[error("cannot start git in {path}: {source}")]
    Spawn {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("unexpected git output from {context}: {message}")]
    UnexpectedOutput {
        context: &'static str,
        message: String,
    },
    #[error("{path} is not inside a git work tree: {message}")]
    NotInsideWorkTree { path: PathBuf, message: String },
    #[error("cannot send the commit message to git in {path}: {source}")]
    WriteInput {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("the commit message is empty")]
    EmptyCommitMessage,
    #[error("nothing is selected to commit")]
    NothingToCommit,
    #[error("a merge is already in progress")]
    MergeInProgress,
    #[error("no merge, rebase, cherry-pick or revert is in progress")]
    NoOperationInProgress,
    #[error("a merge has no continue step: commit the merge result instead")]
    MergeHasNoContinueStep,
    #[error("{path} is a binary file: binary files must be committed whole")]
    BinaryFileNeedsWholeCommit { path: String },
    #[error("{path} is renamed: renamed files must be committed whole")]
    RenamedFileNeedsWholeCommit { path: String },
    #[error("{path} is untracked: untracked files must be committed whole")]
    UntrackedFileNeedsWholeCommit { path: String },
    #[error("cannot manage the temporary index file {path}: {source}")]
    TemporaryIndexFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("no hunk is selected for {path}")]
    EmptyHunkSelection { path: String },
    #[error("{message}; the hunks of {staged_file_count} file(s) stay staged in the index")]
    PartialHunkStaging {
        staged_file_count: usize,
        message: String,
    },
    #[error("hunk index {index} is out of range: {path} has {hunk_count} hunks")]
    HunkIndexOutOfRange {
        path: String,
        index: usize,
        hunk_count: usize,
    },
    #[error(
        "the selected hunks of {path} no longer apply to the index: {message}; next step: cancel the staged changes of the file and select the hunk again"
    )]
    HunkPatchFailed { path: String, message: String },
    #[error("{path} is not in a conflicted state")]
    NotConflicted { path: String },
    #[error("{path} is not a safe pathspec")]
    UnsafePathspec { path: String },
    #[error("{path} has no local changes to discard")]
    NothingToDiscard { path: String },
    #[error("{path} is a directory")]
    DiscardPathIsDirectory { path: String },
    #[error("{name} is not a valid branch name")]
    InvalidBranchName { name: String },
    #[error("remote branches cannot be deleted here: {name}")]
    RemoteBranchNotDeletable { name: String },
    #[error("cannot update {path}: {source}")]
    IgnoreFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("the repository has no remote to push to")]
    NoRemote,
    #[error("the repository has no remote to pull from")]
    NoRemoteToPull,
    #[error("the repository has no remote to fetch from")]
    NoRemoteToFetch,
    #[error("the repository has no branch to push")]
    NoBranch,
    #[error("no repository is open")]
    NoRepository,
}
