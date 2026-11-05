use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gitdeck_core::{
    BranchEntry, ChangeEntry, ChangeKind, CommitSummary, HeadInfo, LogFilter, LogQuery,
    RepoOperation,
};

use crate::backend::{
    CommitOutcome, CommitRequest, ConflictResolution, GitBackend, MergeOutcome, PullStrategy,
    PushRequest,
};
use crate::changed_files::{parse_name_status, ChangedFile};
use crate::command::{
    capture_git_command, command_failed_error, run_git_command, run_remote_git_command,
    run_write_git_command, run_write_git_command_with_environment,
    run_write_git_command_with_input,
};
use crate::decoding::{decode_line, decode_text};
use crate::diff::{parse_file_diff, FileDiff};
use crate::error::GitError;
use crate::log_records::{parse_log_records, LOG_FORMAT_ARGUMENT};
use crate::patch::{assemble_patch, diff_reports_binary_file};
use crate::porcelain::{parse_status_porcelain_v2, StatusSnapshot, STATUS_ARGUMENTS};
use crate::ref_listing::{
    attach_refs_to_commits, parse_branch_listing, parse_log_refs,
    sort_containing_branches_head_first, BRANCH_FORMAT_ARGUMENT, LOG_REF_FORMAT_ARGUMENT,
};
use crate::temporary_index::{CommitArguments, TemporaryIndex};

const REPOSITORY_PATH_ARGUMENTS: [&str; 2] = ["rev-parse", "--show-toplevel"];
const WORK_TREE_ARGUMENTS: [&str; 2] = ["rev-parse", "--is-inside-work-tree"];
const HEAD_COMMIT_ARGUMENTS: [&str; 4] = ["rev-parse", "--verify", "--quiet", "HEAD"];
const COMMIT_NAME_STATUS_ARGUMENTS: [&str; 6] = [
    "show",
    "--no-color",
    "--no-ext-diff",
    "-z",
    "--name-status",
    "--format=",
];
const TRACKED_DIFF_ARGUMENTS: [&str; 6] = [
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "-U3",
    "--",
];
const STAGED_PATH_ARGUMENTS: [&str; 4] = ["diff", "--cached", "--name-only", "-z"];
const COMMIT_DIFF_ARGUMENTS: [&str; 5] = [
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "-U3",
];
const ROOT_COMMIT_DIFF_ARGUMENTS: [&str; 7] = [
    "show",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--format=",
    "--root",
    "-U3",
];
const COMMIT_PARENT_SUFFIX: char = '^';
const SECOND_PARENT_SUFFIX: &str = "^2";
const NAME_STATUS_DIFF_ARGUMENTS: [&str; 5] =
    ["diff", "--no-color", "--no-ext-diff", "-z", "--name-status"];
const UNTRACKED_DIFF_ARGUMENTS: [&str; 7] = [
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--no-index",
    "-U3",
    "--",
];
#[cfg(windows)]
const NULL_DEVICE_PATH: &str = "NUL";
#[cfg(not(windows))]
const NULL_DEVICE_PATH: &str = "/dev/null";
const DIFFERENCES_FOUND_EXIT_CODE: Option<i32> = Some(1);
const REMOTE_LIST_ARGUMENTS: [&str; 1] = ["remote"];
const BRANCH_CONTAINS_ARGUMENT: &str = "--contains";
const BRANCH_REF_NAMESPACES: [&str; 2] = ["refs/heads", "refs/remotes"];
const LAST_COMMIT_MESSAGE_ARGUMENTS: [&str; 4] = ["log", "-1", "--format=%B", "HEAD"];
const HEAD_REVISION_ARGUMENTS: [&str; 2] = ["rev-parse", "HEAD"];
const MERGE_HEAD_ARGUMENTS: [&str; 4] = ["rev-parse", "--quiet", "--verify", "MERGE_HEAD"];
const OPERATION_MARKERS: [(&str, RepoOperation); 5] = [
    ("MERGE_HEAD", RepoOperation::Merging),
    ("rebase-merge", RepoOperation::Rebasing),
    ("rebase-apply", RepoOperation::Rebasing),
    ("CHERRY_PICK_HEAD", RepoOperation::CherryPicking),
    ("REVERT_HEAD", RepoOperation::Reverting),
];
const GIT_PATH_ARGUMENTS_PREFIX: [&str; 2] = ["rev-parse", "--git-path"];
const ABORT_ARGUMENT: &str = "--abort";
const CONTINUE_ARGUMENT: &str = "--continue";
const GIT_EDITOR_VARIABLE: &str = "GIT_EDITOR";
const NO_EDITOR_VALUE: &str = "true";
const ANCESTOR_ARGUMENTS_PREFIX: [&str; 2] = ["merge-base", "--is-ancestor"];
const MERGE_ARGUMENT: &str = "merge";
const MERGE_NO_EDIT_ARGUMENT: &str = "--no-edit";
const STAGE_ARGUMENT_PREFIX: [&str; 3] = ["add", "-A", "--"];
const APPLY_PATCH_ARGUMENTS: [&str; 4] = ["apply", "--cached", "--whitespace=nowarn", "-"];
const RESET_INDEX_ARGUMENTS: [&str; 3] = ["reset", "-q", "--"];
const REAL_INDEX_COMMIT_ARGUMENT_PREFIX: [&str; 3] = ["commit", "-F", "-"];
const REAL_INDEX_COMMIT_AMEND_ARGUMENT: &str = "--amend";
const REAL_INDEX_COMMIT_SIGN_OFF_ARGUMENT: &str = "--signoff";
const PATHSPEC_SEPARATOR: &str = "--";
const PUSH_REMOTE_NAME: &str = "origin";
const CHECKOUT_ARGUMENT: &str = "checkout";
const BRANCH_ARGUMENT: &str = "branch";
const DELETE_BRANCH_ARGUMENT: &str = "-d";
const GITIGNORE_FILE_NAME: &str = ".gitignore";
const GITIGNORE_LINE_ENDING: &str = "\n";
const GITIGNORE_CRLF_LINE_ENDING: &str = "\r\n";
const REMOTE_BRANCH_REF_PREFIX: &str = "refs/remotes/";
const CHECKOUT_NEW_BRANCH_ARGUMENT: &str = "-b";
const CHECK_REF_FORMAT_ARGUMENT: &str = "check-ref-format";
const CHECK_BRANCH_NAME_ARGUMENT: &str = "--branch";
const CHECKOUT_OURS_ARGUMENT: &str = "--ours";
const CHECKOUT_THEIRS_ARGUMENT: &str = "--theirs";
const ADD_ARGUMENT: &str = "add";
const CLEAN_ARGUMENT: &str = "clean";
const CLEAN_FORCE_ARGUMENT: &str = "-f";
const HEAD_REVISION: &str = "HEAD";
const FETCH_ARGUMENT: &str = "fetch";
const PULL_ARGUMENT: &str = "pull";
const PULL_FAST_FORWARD_ONLY_ARGUMENT: &str = "--ff-only";
const PULL_MERGE_ARGUMENT: &str = "--no-rebase";
const PULL_REBASE_ARGUMENT: &str = "--rebase";
const PUSH_ARGUMENT: &str = "push";
const PUSH_SET_UPSTREAM_ARGUMENT: &str = "-u";
const PUSH_FORCE_WITH_LEASE_ARGUMENT: &str = "--force-with-lease";
pub const LOG_FILTER_SCAN_LIMIT: usize = 5_000;
const LOG_FILTER_MINIMUM_BLOCK_SIZE: usize = 200;
const LOG_DEFAULT_REVISION: &str = "HEAD";

fn pull_strategy_argument(strategy: PullStrategy) -> &'static str {
    match strategy {
        PullStrategy::FastForwardOnly => PULL_FAST_FORWARD_ONLY_ARGUMENT,
        PullStrategy::Merge => PULL_MERGE_ARGUMENT,
        PullStrategy::Rebase => PULL_REBASE_ARGUMENT,
    }
}

fn conflict_side_argument(resolution: ConflictResolution) -> Option<&'static str> {
    match resolution {
        ConflictResolution::Ours => Some(CHECKOUT_OURS_ARGUMENT),
        ConflictResolution::Theirs => Some(CHECKOUT_THEIRS_ARGUMENT),
        ConflictResolution::MarkResolved => None,
    }
}

fn is_safe_pathspec(path: &str) -> bool {
    !path.is_empty() && !path.contains('\0') && !path.starts_with('-')
}

fn is_repository_relative_path(path: &str) -> bool {
    is_safe_pathspec(path)
        && !Path::new(path).is_absolute()
        && !path.split('/').any(|segment| segment == "..")
}

fn ignore_entry_is_present(contents: &str, path: &str) -> bool {
    contents.lines().any(|line| line.trim_end() == path)
}

fn ignore_entry_separator(contents: &str) -> &'static str {
    if contents.contains(GITIGNORE_CRLF_LINE_ENDING) {
        GITIGNORE_CRLF_LINE_ENDING
    } else {
        GITIGNORE_LINE_ENDING
    }
}

fn commit_matches_text(commit: &CommitSummary, needle: Option<&str>, case_sensitive: bool) -> bool {
    let Some(needle) = needle else {
        return true;
    };
    if case_sensitive {
        return [
            &commit.subject,
            &commit.author_name,
            &commit.author_email,
            &commit.short_id,
        ]
        .into_iter()
        .any(|field| field.contains(needle));
    }
    let lowered_needle = needle.to_lowercase();
    [
        &commit.subject,
        &commit.author_name,
        &commit.author_email,
        &commit.short_id,
    ]
    .into_iter()
    .any(|field| field.to_lowercase().contains(&lowered_needle))
}

#[derive(Debug)]
pub struct CliGit {
    repository_path: PathBuf,
}

impl CliGit {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let requested_path = path.as_ref();
        let repository_path = run_git_command(requested_path, &REPOSITORY_PATH_ARGUMENTS)?;
        let inside_work_tree = run_git_command(requested_path, &WORK_TREE_ARGUMENTS)?;
        if decode_line(&inside_work_tree) != "true" {
            return Err(GitError::NotInsideWorkTree {
                path: requested_path.to_path_buf(),
                message: String::from("rev-parse --is-inside-work-tree did not report true"),
            });
        }
        Ok(Self {
            repository_path: PathBuf::from(decode_line(&repository_path)),
        })
    }

    fn status_snapshot(&self) -> Result<StatusSnapshot, GitError> {
        let output = run_git_command(&self.repository_path, &STATUS_ARGUMENTS)?;
        parse_status_porcelain_v2(&output)
    }

    fn operation_marker_exists(&self, marker: &str) -> Result<bool, GitError> {
        let mut arguments = GIT_PATH_ARGUMENTS_PREFIX.to_vec();
        arguments.push(marker);
        let output = run_git_command(&self.repository_path, &arguments)?;
        let marker_path = PathBuf::from(decode_line(&output));
        let resolved = if marker_path.is_absolute() {
            marker_path
        } else {
            self.repository_path.join(marker_path)
        };
        Ok(resolved.exists())
    }

    fn merge_head_present(&self) -> Result<bool, GitError> {
        let result = capture_git_command(&self.repository_path, &MERGE_HEAD_ARGUMENTS)?;
        Ok(result.status.success())
    }

    fn is_ancestor_of(&self, commit_id: &str, descendant: &str) -> Result<bool, GitError> {
        let mut arguments = ANCESTOR_ARGUMENTS_PREFIX.to_vec();
        arguments.push(commit_id);
        arguments.push(descendant);
        let result = capture_git_command(&self.repository_path, &arguments)?;
        Ok(result.status.success())
    }

    fn head_commit_id(&self) -> Result<Option<String>, GitError> {
        let result = capture_git_command(&self.repository_path, &HEAD_COMMIT_ARGUMENTS)?;
        if !result.status.success() {
            return Ok(None);
        }
        Ok(Some(decode_line(&result.stdout)))
    }

    pub fn log_with_scan_limit(
        &self,
        query: LogQuery,
        filter: &LogFilter,
        scan_limit: usize,
    ) -> Result<Vec<CommitSummary>, GitError> {
        let Some(head_commit_id) = self.head_commit_id()? else {
            return Ok(Vec::new());
        };
        let mut commits = self.collect_matching_commits(query, filter, scan_limit)?;
        let refs_output = run_git_command(
            &self.repository_path,
            &[
                "for-each-ref",
                LOG_REF_FORMAT_ARGUMENT,
                "refs/heads",
                "refs/remotes",
                "refs/tags",
            ],
        )?;
        attach_refs_to_commits(
            &mut commits,
            parse_log_refs(&refs_output)?,
            Some(&head_commit_id),
        );
        Ok(commits)
    }

    fn collect_matching_commits(
        &self,
        query: LogQuery,
        filter: &LogFilter,
        scan_limit: usize,
    ) -> Result<Vec<CommitSummary>, GitError> {
        let lowered_needle = filter.text.as_deref();
        let case_sensitive = filter.case_sensitive;
        let block_size = query.max_commits.max(LOG_FILTER_MINIMUM_BLOCK_SIZE);
        let mut matched_commits = Vec::new();
        let mut scanned_commits = 0;
        let mut skipped_matching_commits = 0;
        while matched_commits.len() < query.max_commits && scanned_commits < scan_limit {
            let remaining_scan_budget = scan_limit - scanned_commits;
            let block = self.read_log_block(
                filter.branch.as_deref(),
                block_size.min(remaining_scan_budget),
                scanned_commits,
            )?;
            if block.is_empty() {
                break;
            }
            scanned_commits += block.len();
            for commit in block {
                if !commit_matches_text(&commit, lowered_needle, case_sensitive) {
                    continue;
                }
                if skipped_matching_commits < query.skip {
                    skipped_matching_commits += 1;
                    continue;
                }
                matched_commits.push(commit);
                if matched_commits.len() == query.max_commits {
                    break;
                }
            }
        }
        Ok(matched_commits)
    }

    fn read_log_block(
        &self,
        branch: Option<&str>,
        max_commits: usize,
        skip: usize,
    ) -> Result<Vec<CommitSummary>, GitError> {
        let max_commit_argument = format!("--max-count={max_commits}");
        let skip_argument = format!("--skip={skip}");
        let log_output = run_git_command(
            &self.repository_path,
            &[
                "log",
                "--no-show-signature",
                "--topo-order",
                LOG_FORMAT_ARGUMENT,
                max_commit_argument.as_str(),
                skip_argument.as_str(),
                branch.unwrap_or(LOG_DEFAULT_REVISION),
            ],
        )?;
        parse_log_records(&log_output)
    }

    fn is_tracked(&self, path: &str) -> Result<bool, GitError> {
        let output = run_git_command(&self.repository_path, &["ls-files", "--", path])?;
        Ok(!decode_line(&output).is_empty())
    }

    fn tracked_diff(&self, path: &str) -> Result<FileDiff, GitError> {
        let mut arguments = TRACKED_DIFF_ARGUMENTS.to_vec();
        arguments.push(path);
        let output = run_git_command(&self.repository_path, &arguments)?;
        parse_file_diff(path, &output)
    }

    fn commit_parent_reference(&self, commit_id: &str) -> String {
        format!("{commit_id}{COMMIT_PARENT_SUFFIX}")
    }

    fn has_commit_parent(&self, commit_id: &str) -> Result<bool, GitError> {
        let parent_reference = self.commit_parent_reference(commit_id);
        let result = capture_git_command(
            &self.repository_path,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                parent_reference.as_str(),
            ],
        )?;
        Ok(result.status.success())
    }

    fn has_second_parent(&self, commit_id: &str) -> Result<bool, GitError> {
        let reference = format!("{commit_id}{SECOND_PARENT_SUFFIX}");
        let result = capture_git_command(
            &self.repository_path,
            &["rev-parse", "--verify", "--quiet", reference.as_str()],
        )?;
        Ok(result.status.success())
    }

    fn first_parent_name_status(&self, commit_id: &str) -> Result<Vec<u8>, GitError> {
        let first_parent = self.commit_parent_reference(commit_id);
        let mut arguments = NAME_STATUS_DIFF_ARGUMENTS.to_vec();
        arguments.push(first_parent.as_str());
        arguments.push(commit_id);
        arguments.push(PATHSPEC_SEPARATOR);
        run_git_command(&self.repository_path, &arguments)
    }

    fn commit_diff_output(&self, commit_id: &str, path: &str) -> Result<Vec<u8>, GitError> {
        if !self.has_commit_parent(commit_id)? {
            let mut arguments = ROOT_COMMIT_DIFF_ARGUMENTS.to_vec();
            arguments.push(commit_id);
            arguments.push("--");
            arguments.push(path);
            return run_git_command(&self.repository_path, &arguments);
        }
        let parent_reference = self.commit_parent_reference(commit_id);
        let mut arguments = COMMIT_DIFF_ARGUMENTS.to_vec();
        arguments.push(parent_reference.as_str());
        arguments.push(commit_id);
        arguments.push("--");
        arguments.push(path);
        run_git_command(&self.repository_path, &arguments)
    }

    fn untracked_diff(&self, path: &str) -> Result<FileDiff, GitError> {
        let mut arguments = UNTRACKED_DIFF_ARGUMENTS.to_vec();
        arguments.push(NULL_DEVICE_PATH);
        arguments.push(path);
        let result = capture_git_command(&self.repository_path, &arguments)?;
        if !result.status.success() && result.status.code() != DIFFERENCES_FOUND_EXIT_CODE {
            return Err(command_failed_error(
                &self.repository_path,
                &arguments,
                &result.stderr,
            ));
        }
        parse_file_diff(path, &result.stdout)
    }

    fn remote_names(&self) -> Result<Vec<String>, GitError> {
        let output = run_git_command(&self.repository_path, &REMOTE_LIST_ARGUMENTS)?;
        Ok(decode_text(&output)
            .lines()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect())
    }

    fn resolve_fetch_remote(&self, requested_remote: Option<&str>) -> Result<String, GitError> {
        if let Some(remote) = requested_remote {
            return Ok(remote.to_string());
        }
        if self
            .remote_names()?
            .iter()
            .any(|name| name == PUSH_REMOTE_NAME)
        {
            return Ok(PUSH_REMOTE_NAME.to_string());
        }
        Err(GitError::NoRemoteToFetch)
    }

    fn require_a_remote(&self) -> Result<(), GitError> {
        if self.remote_names()?.is_empty() {
            return Err(GitError::NoRemote);
        }
        Ok(())
    }

    fn require_a_remote_to_pull(&self) -> Result<(), GitError> {
        if self.remote_names()?.is_empty() {
            return Err(GitError::NoRemoteToPull);
        }
        Ok(())
    }

    fn stage_commit_paths(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut arguments = STAGE_ARGUMENT_PREFIX.to_vec();
        arguments.extend(paths.iter().map(String::as_str));
        run_write_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn require_a_whole_file_commit(&self, path: &str) -> Result<(), GitError> {
        let snapshot = self.status_snapshot()?;
        let Some(change) = snapshot.changes.iter().find(|change| change.path == path) else {
            return Ok(());
        };
        if change.kind == ChangeKind::Untracked {
            return Err(GitError::UntrackedFileNeedsWholeCommit {
                path: path.to_string(),
            });
        }
        if change.previous_path.is_some() {
            return Err(GitError::RenamedFileNeedsWholeCommit {
                path: path.to_string(),
            });
        }
        Ok(())
    }

    fn raw_worktree_diff(&self, path: &str) -> Result<Vec<u8>, GitError> {
        let mut arguments = TRACKED_DIFF_ARGUMENTS.to_vec();
        arguments.push(path);
        run_git_command(&self.repository_path, &arguments)
    }

    fn apply_patch_to_index(&self, patch: &[u8]) -> Result<(), GitError> {
        run_write_git_command_with_input(&self.repository_path, &APPLY_PATCH_ARGUMENTS, patch)
            .map(|_| ())
    }

    fn assemble_selected_hunks(
        &self,
        path: &str,
        hunk_indices: &[usize],
    ) -> Result<Vec<u8>, GitError> {
        let raw_diff = self.raw_worktree_diff(path)?;
        if diff_reports_binary_file(&raw_diff) {
            return Err(GitError::BinaryFileNeedsWholeCommit {
                path: path.to_string(),
            });
        }
        assemble_patch(&raw_diff, path, hunk_indices)
    }

    fn stage_hunks_into_the_index(
        &self,
        path: &str,
        hunk_indices: &[usize],
    ) -> Result<(), GitError> {
        let patch = self.assemble_selected_hunks(path, hunk_indices)?;
        self.apply_patch_to_index(&patch)
    }

    fn staged_paths(&self) -> Result<Vec<String>, GitError> {
        let output = run_git_command(&self.repository_path, &STAGED_PATH_ARGUMENTS)?;
        Ok(decode_text(&output)
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect())
    }

    fn reset_unselected_staged_paths(
        &self,
        temporary_index: &TemporaryIndex,
        request: &CommitRequest,
    ) -> Result<(), GitError> {
        let selected_paths: BTreeSet<&str> = request
            .paths
            .iter()
            .map(String::as_str)
            .chain(request.hunks.iter().map(|(path, _)| path.as_str()))
            .collect();
        for path in self.staged_paths()? {
            if !selected_paths.contains(path.as_str()) {
                temporary_index.reset_path(&self.repository_path, &path)?;
            }
        }
        Ok(())
    }

    fn fill_temporary_index(
        &self,
        temporary_index: &TemporaryIndex,
        request: &CommitRequest,
    ) -> Result<(), GitError> {
        let whole_file_paths = request.paths.iter().filter(|path| {
            !request
                .hunks
                .iter()
                .any(|(hunk_path, _)| hunk_path == *path)
        });
        for path in whole_file_paths {
            temporary_index.add_path(&self.repository_path, path)?;
        }
        for (path, hunk_indices) in &request.hunks {
            self.require_a_whole_file_commit(path)?;
            let patch = self.assemble_selected_hunks(path, hunk_indices)?;
            temporary_index
                .apply_patch(&self.repository_path, &patch)
                .map_err(|error| GitError::HunkPatchFailed {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
        }
        Ok(())
    }

    fn reset_committed_paths(&self, request: &CommitRequest) -> Result<(), GitError> {
        let mut paths: Vec<&str> = request.paths.iter().map(String::as_str).collect();
        paths.extend(request.hunks.iter().map(|(path, _)| path.as_str()));
        paths.sort_unstable();
        paths.dedup();
        if paths.is_empty() {
            return Ok(());
        }
        let mut arguments = RESET_INDEX_ARGUMENTS.to_vec();
        arguments.extend(paths);
        run_write_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn commit_through_the_real_index(&self, request: &CommitRequest) -> Result<(), GitError> {
        let mut arguments = REAL_INDEX_COMMIT_ARGUMENT_PREFIX.to_vec();
        if request.amend {
            arguments.push(REAL_INDEX_COMMIT_AMEND_ARGUMENT);
        }
        if request.sign_off {
            arguments.push(REAL_INDEX_COMMIT_SIGN_OFF_ARGUMENT);
        }
        run_write_git_command_with_input(
            &self.repository_path,
            &arguments,
            request.message.as_bytes(),
        )
        .map(|_| ())
    }

    fn commit_through_a_temporary_index(&self, request: &CommitRequest) -> Result<(), GitError> {
        let temporary_index = TemporaryIndex::create(&self.repository_path)?;
        if self.head_commit_id()?.is_some() {
            temporary_index.seed_from_real_index(&self.repository_path)?;
            self.reset_unselected_staged_paths(&temporary_index, request)?;
        } else {
            temporary_index.read_empty_tree(&self.repository_path)?;
        }
        let commit_result = self
            .fill_temporary_index(&temporary_index, request)
            .and_then(|()| {
                temporary_index.commit(
                    &self.repository_path,
                    CommitArguments {
                        message: &request.message,
                        amend: request.amend,
                        sign_off: request.sign_off,
                    },
                )
            })
            .and_then(|()| self.reset_committed_paths(request));
        let cleanup_result = temporary_index.remove_file();
        commit_result?;
        cleanup_result
    }

    fn require_a_conflicted_path(&self, path: &str) -> Result<(), GitError> {
        if !is_safe_pathspec(path) {
            return Err(GitError::UnsafePathspec {
                path: path.to_string(),
            });
        }
        let snapshot = self.status_snapshot()?;
        let is_conflicted = snapshot
            .changes
            .iter()
            .any(|change| change.path == path && change.conflict.is_some());
        if !is_conflicted {
            return Err(GitError::NotConflicted {
                path: path.to_string(),
            });
        }
        Ok(())
    }

    fn stage_conflicted_path(
        &self,
        path: &str,
        resolution: ConflictResolution,
    ) -> Result<(), GitError> {
        if let Some(side) = conflict_side_argument(resolution) {
            let checkout_arguments = [CHECKOUT_ARGUMENT, side, PATHSPEC_SEPARATOR, path];
            run_write_git_command(&self.repository_path, &checkout_arguments)?;
        }
        let add_arguments = [ADD_ARGUMENT, PATHSPEC_SEPARATOR, path];
        run_write_git_command(&self.repository_path, &add_arguments).map(|_| ())
    }

    fn require_a_discardable_change_kind(&self, path: &str) -> Result<ChangeKind, GitError> {
        if !is_safe_pathspec(path) {
            return Err(GitError::UnsafePathspec {
                path: path.to_string(),
            });
        }
        if self.repository_path.join(path).is_dir() {
            return Err(GitError::DiscardPathIsDirectory {
                path: path.to_string(),
            });
        }
        let snapshot = self.status_snapshot()?;
        snapshot
            .changes
            .iter()
            .find(|change| change.path == path)
            .map(|change| change.kind)
            .ok_or_else(|| GitError::NothingToDiscard {
                path: path.to_string(),
            })
    }

    fn discard_change_of_kind(&self, path: &str, kind: ChangeKind) -> Result<(), GitError> {
        if kind == ChangeKind::Untracked {
            let clean_arguments = [
                CLEAN_ARGUMENT,
                CLEAN_FORCE_ARGUMENT,
                PATHSPEC_SEPARATOR,
                path,
            ];
            return run_write_git_command(&self.repository_path, &clean_arguments).map(|_| ());
        }
        let checkout_arguments = [CHECKOUT_ARGUMENT, HEAD_REVISION, PATHSPEC_SEPARATOR, path];
        run_write_git_command(&self.repository_path, &checkout_arguments).map(|_| ())
    }

    fn reject_invalid_branch_name(&self, name: &str) -> Result<(), GitError> {
        let invalid = || GitError::InvalidBranchName {
            name: name.to_string(),
        };
        if name.is_empty() || name.starts_with('-') || name.contains('\0') {
            return Err(invalid());
        }
        let arguments = [CHECK_REF_FORMAT_ARGUMENT, CHECK_BRANCH_NAME_ARGUMENT, name];
        let result = capture_git_command(&self.repository_path, &arguments)?;
        if !result.status.success() {
            return Err(invalid());
        }
        Ok(())
    }

    fn branch_ref_exists(&self, reference: &str) -> Result<bool, GitError> {
        let result = capture_git_command(
            &self.repository_path,
            &["rev-parse", "--verify", "--quiet", reference],
        )?;
        Ok(result.status.success())
    }

    fn reject_remote_branch_deletion(&self, name: &str) -> Result<(), GitError> {
        let remote_reference = format!("{REMOTE_BRANCH_REF_PREFIX}{name}");
        if self.branch_ref_exists(&remote_reference)? {
            return Err(GitError::RemoteBranchNotDeletable {
                name: name.to_string(),
            });
        }
        Ok(())
    }

    fn append_ignore_entry(&self, path: &str) -> Result<(), GitError> {
        let ignore_path = self.repository_path.join(GITIGNORE_FILE_NAME);
        let contents = match std::fs::read_to_string(&ignore_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(GitError::IgnoreFile {
                    path: ignore_path,
                    source: error,
                })
            }
        };
        if ignore_entry_is_present(&contents, path) {
            return Ok(());
        }
        let separator = ignore_entry_separator(&contents);
        let mut updated = contents;
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push_str(separator);
        }
        updated.push_str(path);
        updated.push_str(separator);
        std::fs::write(&ignore_path, updated).map_err(|error| GitError::IgnoreFile {
            path: ignore_path,
            source: error,
        })
    }

    fn conflicted_paths(&self) -> Result<Vec<String>, GitError> {
        Ok(self
            .status_snapshot()?
            .changes
            .iter()
            .filter(|change| change.conflict.is_some())
            .map(|change| change.path.clone())
            .collect())
    }

    fn head_commit_outcome(&self) -> Result<CommitOutcome, GitError> {
        let output = run_git_command(&self.repository_path, &HEAD_REVISION_ARGUMENTS)?;
        Ok(CommitOutcome {
            commit_id: decode_line(&output),
        })
    }
}

impl GitBackend for CliGit {
    fn repository_path(&self) -> &Path {
        &self.repository_path
    }

    fn head_info(&self) -> Result<HeadInfo, GitError> {
        let mut head = self.status_snapshot()?.head;
        head.operation = self.current_operation()?;
        head.merging = head.operation.is_some_and(RepoOperation::is_merging);
        Ok(head)
    }

    fn changes(&self) -> Result<Vec<ChangeEntry>, GitError> {
        Ok(self.status_snapshot()?.changes)
    }

    fn log(&self, query: LogQuery, filter: &LogFilter) -> Result<Vec<CommitSummary>, GitError> {
        self.log_with_scan_limit(query, filter, LOG_FILTER_SCAN_LIMIT)
    }

    fn branches(&self) -> Result<Vec<BranchEntry>, GitError> {
        let output = run_git_command(
            &self.repository_path,
            &[
                "for-each-ref",
                BRANCH_FORMAT_ARGUMENT,
                "refs/heads",
                "refs/remotes",
            ],
        )?;
        parse_branch_listing(&output)
    }

    fn branches_containing(&self, commit_id: &str) -> Result<Vec<BranchEntry>, GitError> {
        let mut arguments = vec!["for-each-ref", BRANCH_CONTAINS_ARGUMENT, commit_id];
        arguments.push(BRANCH_FORMAT_ARGUMENT);
        arguments.extend(BRANCH_REF_NAMESPACES);
        let output = run_git_command(&self.repository_path, &arguments)?;
        let mut branches = parse_branch_listing(&output)?;
        sort_containing_branches_head_first(&mut branches);
        Ok(branches)
    }

    fn diff_working_tree(&self, path: &str) -> Result<FileDiff, GitError> {
        if self.is_tracked(path)? {
            self.tracked_diff(path)
        } else {
            self.untracked_diff(path)
        }
    }

    fn diff_commit_file(&self, commit_id: &str, path: &str) -> Result<FileDiff, GitError> {
        parse_file_diff(path, &self.commit_diff_output(commit_id, path)?)
    }

    fn commit_changed_files(&self, commit_id: &str) -> Result<Vec<ChangedFile>, GitError> {
        if self.has_second_parent(commit_id)? {
            return parse_name_status(&self.first_parent_name_status(commit_id)?);
        }
        let mut arguments = COMMIT_NAME_STATUS_ARGUMENTS.to_vec();
        arguments.push(commit_id);
        arguments.push(PATHSPEC_SEPARATOR);
        let output = run_git_command(&self.repository_path, &arguments)?;
        parse_name_status(&output)
    }

    fn last_commit_message(&self) -> Result<String, GitError> {
        let output = run_git_command(&self.repository_path, &LAST_COMMIT_MESSAGE_ARGUMENTS)?;
        Ok(decode_text(&output).trim_end_matches('\n').to_string())
    }

    fn commit(&self, request: CommitRequest) -> Result<CommitOutcome, GitError> {
        if request.message.trim().is_empty() {
            return Err(GitError::EmptyCommitMessage);
        }
        let merging = self.merge_head_present()?;
        if request.paths.is_empty()
            && request.hunks.is_empty()
            && !request.rewrites_the_head_message()
            && !merging
        {
            return Err(GitError::NothingToCommit);
        }
        if merging {
            self.commit_through_the_real_index(&request)?;
        } else {
            self.commit_through_a_temporary_index(&request)?;
        }
        self.head_commit_outcome()
    }

    fn stage_paths(&self, paths: &[String]) -> Result<(), GitError> {
        self.stage_commit_paths(paths)
    }

    fn stage_hunks(&self, path: &str, hunk_indices: &[usize]) -> Result<(), GitError> {
        if hunk_indices.is_empty() {
            return Err(GitError::EmptyHunkSelection {
                path: path.to_string(),
            });
        }
        self.require_a_whole_file_commit(path)?;
        self.stage_hunks_into_the_index(path, hunk_indices)
    }

    fn stage_hunk_selection(&self, selection: &[(String, Vec<usize>)]) -> Result<(), GitError> {
        let selected_files: Vec<&(String, Vec<usize>)> = selection
            .iter()
            .filter(|(_, hunk_indices)| !hunk_indices.is_empty())
            .collect();
        if selected_files.is_empty() {
            return Ok(());
        }
        for (staged_file_count, (path, hunk_indices)) in selected_files.into_iter().enumerate() {
            let staged = self
                .require_a_whole_file_commit(path)
                .and_then(|()| self.stage_hunks_into_the_index(path, hunk_indices));
            if let Err(error) = staged {
                if staged_file_count == 0 {
                    return Err(error);
                }
                return Err(GitError::PartialHunkStaging {
                    staged_file_count,
                    message: error.to_string(),
                });
            }
        }
        Ok(())
    }

    fn fetch(&self, remote: Option<&str>) -> Result<(), GitError> {
        let remote_name = self.resolve_fetch_remote(remote)?;
        let arguments = [FETCH_ARGUMENT, remote_name.as_str()];
        run_remote_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn pull(&self, strategy: PullStrategy) -> Result<(), GitError> {
        self.require_a_remote_to_pull()?;
        let arguments = [PULL_ARGUMENT, pull_strategy_argument(strategy)];
        run_remote_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn push_request(&self, request: PushRequest) -> Result<(), GitError> {
        if request.branch.trim().is_empty() {
            return Err(GitError::NoBranch);
        }
        self.require_a_remote()?;
        let mut arguments = vec![PUSH_ARGUMENT];
        if request.set_upstream {
            arguments.push(PUSH_SET_UPSTREAM_ARGUMENT);
        }
        if request.force_with_lease {
            arguments.push(PUSH_FORCE_WITH_LEASE_ARGUMENT);
        }
        arguments.push(PUSH_REMOTE_NAME);
        arguments.push(request.branch.as_str());
        run_remote_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        let arguments = [CHECKOUT_ARGUMENT, name];
        run_write_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn create_branch(&self, name: &str, start_point: Option<&str>) -> Result<(), GitError> {
        self.reject_invalid_branch_name(name)?;
        let mut arguments = vec![CHECKOUT_ARGUMENT, CHECKOUT_NEW_BRANCH_ARGUMENT, name];
        if let Some(start_point) = start_point {
            arguments.push(start_point);
        }
        run_write_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        self.reject_invalid_branch_name(name)?;
        self.reject_remote_branch_deletion(name)?;
        let arguments = [BRANCH_ARGUMENT, DELETE_BRANCH_ARGUMENT, name];
        run_write_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn ignore_path(&self, path: &str) -> Result<(), GitError> {
        if !is_repository_relative_path(path) {
            return Err(GitError::UnsafePathspec {
                path: path.to_string(),
            });
        }
        self.append_ignore_entry(path)
    }

    fn merge_branch(&self, name: &str) -> Result<MergeOutcome, GitError> {
        if self.merge_head_present()? {
            return Err(GitError::MergeInProgress);
        }
        let head_before = self.head_commit_id()?;
        let fast_forwards = match head_before.as_deref() {
            Some(head) => self.is_ancestor_of(head, name)?,
            None => false,
        };
        let arguments = [MERGE_ARGUMENT, MERGE_NO_EDIT_ARGUMENT, name];
        let result = capture_git_command(&self.repository_path, &arguments)?;
        if !result.status.success() {
            let paths = self.conflicted_paths()?;
            if !paths.is_empty() {
                return Ok(MergeOutcome::Conflicts { paths });
            }
            return Err(command_failed_error(
                &self.repository_path,
                &arguments,
                &result.stderr,
            ));
        }
        let head_after = self
            .head_commit_id()?
            .ok_or_else(|| GitError::UnexpectedOutput {
                context: "merge",
                message: String::from("the merged repository has no head commit"),
            })?;
        if head_before.as_deref() == Some(head_after.as_str()) {
            return Ok(MergeOutcome::AlreadyUpToDate);
        }
        if fast_forwards {
            return Ok(MergeOutcome::FastForward {
                commit_id: head_after,
            });
        }
        Ok(MergeOutcome::Merged {
            commit_id: head_after,
        })
    }

    fn resolve_conflict(&self, path: &str, resolution: ConflictResolution) -> Result<(), GitError> {
        self.require_a_conflicted_path(path)?;
        self.stage_conflicted_path(path, resolution)
    }

    fn discard_changes(&self, path: &str) -> Result<(), GitError> {
        let kind = self.require_a_discardable_change_kind(path)?;
        self.discard_change_of_kind(path, kind)
    }

    fn current_operation(&self) -> Result<Option<RepoOperation>, GitError> {
        for (marker, operation) in OPERATION_MARKERS {
            if self.operation_marker_exists(marker)? {
                return Ok(Some(operation));
            }
        }
        Ok(None)
    }

    fn abort_operation(&self) -> Result<(), GitError> {
        let Some(operation) = self.current_operation()? else {
            return Err(GitError::NoOperationInProgress);
        };
        let arguments = [operation.noun(), ABORT_ARGUMENT];
        run_write_git_command(&self.repository_path, &arguments).map(|_| ())
    }

    fn continue_operation(&self) -> Result<(), GitError> {
        let Some(operation) = self.current_operation()? else {
            return Err(GitError::NoOperationInProgress);
        };
        let Some(command) = operation.continuation() else {
            return Err(GitError::MergeHasNoContinueStep);
        };
        let arguments = [command, CONTINUE_ARGUMENT];
        run_write_git_command_with_environment(
            &self.repository_path,
            &arguments,
            &[(GIT_EDITOR_VARIABLE, NO_EDITOR_VALUE)],
        )
        .map(|_| ())
    }
}
