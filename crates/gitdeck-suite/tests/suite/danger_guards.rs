use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use gitdeck_app::App;
use gitdeck_core::{
    BranchEntry, ChangeEntry, CommitSummary, HeadInfo, LogFilter, LogQuery, RepoOperation,
};
use gitdeck_git::{
    ChangedFile, CliGit, CommitOutcome, CommitRequest, ConflictResolution, FileDiff, GitBackend,
    GitError, MergeOutcome, PullStrategy, PushRequest,
};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{ConfirmDialogButton, StatusTone, MERGING_LABEL};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/danger-guards";
const CURRENT_BRANCH: &str = "main";
const SOURCE_BRANCH: &str = "feature/local";
const SHARED_PATH: &str = "src/guarded.rs";
const BASE_CONTENT: &str = "fn shared() {\n    base();\n}\n";
const CURRENT_CONTENT: &str = "fn shared() {\n    current();\n}\n";
const BRANCH_CONTENT: &str = "fn shared() {\n    branch();\n}\n";
const MERGE_ABORT_MESSAGE: &str = "Abort the merge in progress?";
const ABORT_CANCELLED_RECEIPT: &str = "Abort cancelled";
const MERGE_ABORTED_RECEIPT: &str = "Aborted the merge";
const UNSAVED_MESSAGE_QUIT_TEXT: &str = "You have an unsaved commit message.";
const REFRESH_FAILURE_PREFIX: &str = "Operation finished; refresh failed:";

fn scratch_path(name: &str) -> PathBuf {
    repository_root()
        .join(SCRATCH_REPOSITORY_DIRECTORY)
        .join(name)
}

fn git_command(repository_path: &Path, arguments: &[&str]) -> Command {
    let mut command = fixture_git_command(repository_path);
    command.args(arguments).env("GIT_EDITOR", "true");
    command
}

fn git_output(repository_path: &Path, arguments: &[&str]) -> String {
    let output = git_command(repository_path, arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_git(repository_path: &Path, arguments: &[&str]) {
    let status = git_command(repository_path, arguments)
        .status()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        status.success(),
        "git {arguments:?} was expected to succeed"
    );
}

fn commit_file(repository_path: &Path, relative_path: &str, content: &str, message: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, content).expect("the scratch file is written");
    run_git(repository_path, &["add", "--all"]);
    run_git(repository_path, &["commit", "-q", "-m", message]);
}

fn diverged_repository(name: &str) -> PathBuf {
    let path = scratch_path(name);
    if path.exists() {
        fs::remove_dir_all(&path).expect("the stale scratch repository is removed");
    }
    fs::create_dir_all(&path).expect("the scratch repository directory is created");
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, BASE_CONTENT, "base commit");
    run_git(&path, &["switch", "-q", "-c", SOURCE_BRANCH]);
    commit_file(&path, SHARED_PATH, BRANCH_CONTENT, "branch commit");
    run_git(&path, &["switch", "-q", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, CURRENT_CONTENT, "current commit");
    path
}

fn repository_stopped_in_a_merge(name: &str) -> PathBuf {
    let path = diverged_repository(name);
    git_output(&path, &["merge", SOURCE_BRANCH]);
    assert!(
        path.join(".git").join("MERGE_HEAD").exists(),
        "the merge stops with conflicts"
    );
    path
}

fn terminal_for(repository_path: &Path) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        MEDIUM_WIDTH,
        MEDIUM_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    terminal.tick();
    terminal
}

fn published_repository(name: &str) -> PathBuf {
    let repository = diverged_repository(name);
    let remote = scratch_path(&format!("{name}.git"));
    if remote.exists() {
        fs::remove_dir_all(&remote).expect("the stale bare remote is removed");
    }
    fs::create_dir_all(&remote).expect("the bare remote directory is created");
    git_output(&remote, &["init", "--bare", "--quiet"]);
    git_output(
        &repository,
        &[
            "remote",
            "add",
            "origin",
            remote
                .to_str()
                .expect("the bare remote path is valid utf-8"),
        ],
    );
    run_git(
        &repository,
        &[
            "push",
            "--quiet",
            "--set-upstream",
            "origin",
            CURRENT_BRANCH,
        ],
    );
    repository
}

struct ReadFailingBackend {
    inner: CliGit,
    head_info_reads: Arc<AtomicUsize>,
    reads_fail: Arc<AtomicBool>,
}

macro_rules! delegate_to_inner {
    ($($name:ident($($argument:ident: $argument_type:ty),*) -> $return_type:ty;)*) => {
        $(
            fn $name(&self, $($argument: $argument_type),*) -> $return_type {
                self.inner.$name($($argument),*)
            }
        )*
    };
}

impl GitBackend for ReadFailingBackend {
    fn repository_path(&self) -> &Path {
        self.inner.repository_path()
    }

    fn head_info(&self) -> Result<HeadInfo, GitError> {
        self.head_info_reads.fetch_add(1, Ordering::SeqCst);
        if self.reads_fail.load(Ordering::SeqCst) {
            return Err(GitError::UnexpectedOutput {
                context: "guarded refresh",
                message: String::from("the repository is unreadable"),
            });
        }
        self.inner.head_info()
    }

    delegate_to_inner! {
        changes() -> Result<Vec<ChangeEntry>, GitError>;
        log(query: LogQuery, filter: &LogFilter) -> Result<Vec<CommitSummary>, GitError>;
        branches() -> Result<Vec<BranchEntry>, GitError>;
        branches_containing(commit_id: &str) -> Result<Vec<BranchEntry>, GitError>;
        diff_working_tree(path: &str) -> Result<FileDiff, GitError>;
        diff_commit_file(commit_id: &str, path: &str) -> Result<FileDiff, GitError>;
        commit_changed_files(commit_id: &str) -> Result<Vec<ChangedFile>, GitError>;
        last_commit_message() -> Result<String, GitError>;
        commit(request: CommitRequest) -> Result<CommitOutcome, GitError>;
        stage_paths(paths: &[String]) -> Result<(), GitError>;
        stage_hunks(path: &str, hunk_indices: &[usize]) -> Result<(), GitError>;
        stage_hunk_selection(selection: &[(String, Vec<usize>)]) -> Result<(), GitError>;
        fetch(remote: Option<&str>) -> Result<(), GitError>;
        pull(strategy: PullStrategy) -> Result<(), GitError>;
        push_request(request: PushRequest) -> Result<(), GitError>;
        checkout_branch(name: &str) -> Result<(), GitError>;
        create_branch(name: &str, start_point: Option<&str>) -> Result<(), GitError>;
        merge_branch(name: &str) -> Result<MergeOutcome, GitError>;
        delete_branch(name: &str) -> Result<(), GitError>;
        ignore_path(path: &str) -> Result<(), GitError>;
        resolve_conflict(path: &str, resolution: ConflictResolution) -> Result<(), GitError>;
        current_operation() -> Result<Option<RepoOperation>, GitError>;
        abort_operation() -> Result<(), GitError>;
        continue_operation() -> Result<(), GitError>;
        discard_changes(path: &str) -> Result<(), GitError>;
    }
}

fn instrumented_terminal(
    repository_path: &Path,
) -> (TestTerminal<App>, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let backend = CliGit::open(repository_path).expect("the scratch repository opens");
    let head_info_reads = Arc::new(AtomicUsize::new(0));
    let reads_fail = Arc::new(AtomicBool::new(false));
    let guarded = ReadFailingBackend {
        inner: backend,
        head_info_reads: Arc::clone(&head_info_reads),
        reads_fail: Arc::clone(&reads_fail),
    };
    let mut terminal = TestTerminal::new(
        App::with_backend(Box::new(guarded), CapsProfile::Rich, ThemeId::JbDark)
            .expect("the guarded backend opens"),
        MEDIUM_WIDTH,
        MEDIUM_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    terminal.tick();
    (terminal, head_info_reads, reads_fail)
}

#[test]
fn alt_x_asks_before_aborting() {
    let repository_path = repository_stopped_in_a_merge("asks-before-aborting");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("alt+x");
    terminal.tick();
    assert!(
        terminal.driver().abort_dialog_visible(),
        "alt+x opens the abort confirmation, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().abort_dialog_message().as_deref(),
        Some(MERGE_ABORT_MESSAGE)
    );
    assert!(
        terminal.screen_text().contains(MERGE_ABORT_MESSAGE),
        "the dialog asks before aborting, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(terminal.driver().status_message(), None);
    assert!(
        repository_path.join(".git").join("MERGE_HEAD").exists(),
        "the merge is still in progress while the dialog waits"
    );
    terminal.screenshot("danger_guards/alt-x-asks-before-aborting");
}

#[test]
fn cancelling_the_abort_keeps_the_operation() {
    let repository_path = repository_stopped_in_a_merge("cancelling-keeps");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("alt+x");
    terminal.tick();
    terminal.send_keys("enter");
    terminal.tick();
    assert!(
        !terminal.driver().abort_dialog_visible(),
        "the cancel closes the dialog, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().status_message(),
        Some((ABORT_CANCELLED_RECEIPT, StatusTone::Info))
    );
    assert!(
        repository_path.join(".git").join("MERGE_HEAD").exists(),
        "the cancelled abort leaves the merge in progress"
    );
    assert!(
        terminal.screen_text().contains(MERGING_LABEL),
        "the footer still reports the merge, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("danger_guards/alt-x-abort-cancelled");
}

#[test]
fn confirming_the_abort_runs_it() {
    let repository_path = repository_stopped_in_a_merge("confirming-runs");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("alt+x");
    terminal.tick();
    terminal.send_keys("tab,enter");
    terminal.tick();
    terminal.tick();
    assert!(
        !terminal.driver().abort_dialog_visible(),
        "the confirmed abort closes the dialog, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().status_message(),
        Some((MERGE_ABORTED_RECEIPT, StatusTone::Info)),
        "the status bar reports the aborted merge, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        !repository_path.join(".git").join("MERGE_HEAD").exists(),
        "the confirmed abort drops the merge state"
    );
    assert!(
        git_output(&repository_path, &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "the aborted merge leaves a clean work tree"
    );
    terminal.screenshot("danger_guards/alt-x-abort-confirmed");
}

#[test]
fn the_abort_dialog_focuses_cancel_by_default() {
    let repository_path = repository_stopped_in_a_merge("focuses-cancel");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("alt+x");
    terminal.tick();
    assert_eq!(
        terminal.driver().abort_dialog_button(),
        ConfirmDialogButton::Cancel,
        "the abort dialog opens on cancel, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.send_keys("tab");
    terminal.tick();
    assert_eq!(
        terminal.driver().abort_dialog_button(),
        ConfirmDialogButton::Confirm
    );
    terminal.send_keys("shift+tab");
    terminal.tick();
    assert_eq!(
        terminal.driver().abort_dialog_button(),
        ConfirmDialogButton::Cancel
    );
    terminal.send_keys("esc");
    terminal.tick();
    assert!(!terminal.driver().abort_dialog_visible());
    assert!(
        repository_path.join(".git").join("MERGE_HEAD").exists(),
        "escape keeps the merge in progress"
    );
    terminal.screenshot("danger_guards/alt-x-dialog-focus");
}

#[test]
fn ctrl_q_asks_before_dropping_a_commit_message() {
    let repository_path = diverged_repository("quit-with-a-message");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("ctrl+k");
    terminal.send_text("a message nobody committed");
    terminal.tick();
    terminal.send_keys("ctrl+q");
    terminal.tick();
    assert!(
        terminal.driver().quit_dialog_visible(),
        "ctrl+q opens the quit confirmation, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(!terminal.driver().exit_requested());
    assert_eq!(
        terminal.driver().quit_dialog_message(),
        Some(UNSAVED_MESSAGE_QUIT_TEXT)
    );
    assert!(
        terminal.screen_text().contains(UNSAVED_MESSAGE_QUIT_TEXT),
        "the dialog names the unsaved message, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.send_keys("enter");
    terminal.tick();
    assert!(
        !terminal.driver().exit_requested(),
        "the default cancel keeps the app alive"
    );
    assert!(!terminal.driver().quit_dialog_visible());
    terminal.screenshot("danger_guards/ctrl-q-asks-before-quitting");
}

#[test]
fn ctrl_q_quits_immediately_without_a_draft() {
    let repository_path = diverged_repository("quit-without-a-draft");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("ctrl+q");
    terminal.tick();
    assert!(
        !terminal.driver().quit_dialog_visible(),
        "a clean session quits without a dialog, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(terminal.driver().exit_requested());
}

#[test]
fn ctrl_c_still_quits_immediately() {
    let repository_path = diverged_repository("quit-with-ctrl-c");
    let mut terminal = terminal_for(&repository_path);
    terminal.send_keys("ctrl+k");
    terminal.send_text("a message nobody committed");
    terminal.tick();
    terminal.send_keys("ctrl+c");
    terminal.tick();
    assert!(
        !terminal.driver().quit_dialog_visible(),
        "ctrl+c never asks, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(terminal.driver().exit_requested());
}

#[test]
fn a_successful_push_reports_success_even_when_refresh_fails() {
    let repository_path = published_repository("push-refresh-fails");
    let (mut terminal, _, reads_fail) = instrumented_terminal(&repository_path);
    reads_fail.store(true, Ordering::SeqCst);
    let push_result = terminal.driver_mut().push_request(PushRequest {
        branch: String::from(CURRENT_BRANCH),
        set_upstream: false,
        force_with_lease: false,
    });
    terminal.tick();
    assert!(
        push_result.is_ok(),
        "the successful push is not downgraded to a failure: {push_result:?}"
    );
    assert!(
        terminal.driver().panel_error().is_none(),
        "no error block is raised for a refresh hiccup, screen was:\n{}",
        terminal.screen_text()
    );
    let (message, tone) = terminal.driver().status_message().unwrap_or_else(|| {
        panic!(
            "the refresh failure is reported, screen was:\n{}",
            terminal.screen_text()
        )
    });
    assert_eq!(tone, StatusTone::Warning);
    assert!(
        message.starts_with(REFRESH_FAILURE_PREFIX),
        "the message is separate from the success receipt: {message}"
    );
    terminal.screenshot("danger_guards/push-refresh-failure-warning");
}

#[test]
fn commit_refreshes_the_repository_once() {
    let repository_path = diverged_repository("commit-refresh-count");
    fs::write(
        repository_path.join(SHARED_PATH),
        "fn shared() {\n    edited();\n}\n",
    )
    .expect("the edit is written");
    let (mut terminal, head_info_reads, _) = instrumented_terminal(&repository_path);
    head_info_reads.store(0, Ordering::SeqCst);
    terminal.send_keys("ctrl+k");
    terminal.send_text("commit the edit");
    terminal.tick();
    terminal.send_keys("ctrl+enter");
    terminal.tick();
    terminal.tick();
    assert_eq!(
        head_info_reads.load(Ordering::SeqCst),
        1,
        "a commit refreshes the repository once, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("danger_guards/commit-single-refresh");
}
