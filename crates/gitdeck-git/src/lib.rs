mod backend;
mod changed_files;
mod cli;
mod command;
mod decoding;
mod diff;
mod error;
mod log_records;
mod patch;
mod porcelain;
mod ref_listing;
mod temporary_index;

pub use backend::{
    CommitOutcome, CommitRequest, ConflictResolution, GitBackend, MergeOutcome, PullStrategy,
    PushRequest,
};
pub use changed_files::ChangedFile;
pub use cli::{CliGit, LOG_FILTER_SCAN_LIMIT};
pub use diff::{DiffHunk, DiffLine, DiffLineKind, FileDiff};
pub use error::GitError;
pub use patch::HunkSpan;
