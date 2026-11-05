use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_app::App;
use gitdeck_core::RepoOperation;
use gitdeck_git::{ConflictResolution, PullStrategy};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::REBASING_LABEL;

const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/post-operation-refresh";
const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const CURRENT_BRANCH: &str = "main";
const SOURCE_BRANCH: &str = "feature/side";
const SHARED_PATH: &str = "src/shared.rs";
const DRIFT_PATH: &str = "drifted-after-the-action.md";
const BASE_CONTENT: &str = "fn shared() {\n    base();\n}\n";
const MAIN_CONTENT: &str = "fn shared() {\n    main_side();\n}\n";
const SIDE_CONTENT: &str = "fn shared() {\n    side_branch();\n}\n";
const MISSING_BRANCH: &str = "feature/missing";
const MISSING_PATH: &str = "src/missing.rs";

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

fn run_git(repository_path: &Path, arguments: &[&str]) {
    let status = git_command(repository_path, arguments)
        .status()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        status.success(),
        "git {arguments:?} in {} was expected to succeed",
        repository_path.display()
    );
}

fn run_failing_git(repository_path: &Path, arguments: &[&str]) {
    let status = git_command(repository_path, arguments)
        .status()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        !status.success(),
        "git {arguments:?} in {} was expected to fail",
        repository_path.display()
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

fn empty_scratch_directory(name: &str) -> PathBuf {
    let path = scratch_path(name);
    if path.exists() {
        fs::remove_dir_all(&path).expect("the stale scratch directory is removed");
    }
    fs::create_dir_all(&path).expect("the scratch directory is created");
    path
}

fn diverged_worktree(name: &str) -> PathBuf {
    let path = empty_scratch_directory(name).join("worktree");
    fs::create_dir_all(&path).expect("the scratch repository directory is created");
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, BASE_CONTENT, "base commit");
    run_git(&path, &["switch", "-q", "-c", SOURCE_BRANCH]);
    commit_file(&path, SHARED_PATH, SIDE_CONTENT, "side commit");
    run_git(&path, &["switch", "-q", CURRENT_BRANCH]);
    commit_file(&path, SHARED_PATH, MAIN_CONTENT, "main commit");
    path
}

fn repository_with_a_conflicting_upstream(name: &str) -> PathBuf {
    let root = empty_scratch_directory(name);
    let remote = root.join("origin.git");
    fs::create_dir_all(&remote).expect("the bare remote directory is created");
    run_git(
        &remote,
        &["init", "--bare", "--quiet", "-b", CURRENT_BRANCH],
    );

    let upstream = root.join("upstream");
    fs::create_dir_all(&upstream).expect("the upstream repository directory is created");
    run_git(&upstream, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&upstream, SHARED_PATH, BASE_CONTENT, "base commit");
    run_git(&upstream, &["add", "--all"]);
    run_git(
        &upstream,
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
        &upstream,
        &[
            "push",
            "--quiet",
            "--set-upstream",
            "origin",
            CURRENT_BRANCH,
        ],
    );

    let worktree = root.join("worktree");
    run_git(
        &root,
        &[
            "clone",
            "--quiet",
            remote
                .to_str()
                .expect("the bare remote path is valid utf-8"),
            worktree.to_str().expect("the worktree path is valid utf-8"),
        ],
    );
    commit_file(&upstream, SHARED_PATH, SIDE_CONTENT, "upstream commit");
    run_git(&upstream, &["push", "--quiet", "origin", CURRENT_BRANCH]);
    commit_file(&worktree, SHARED_PATH, MAIN_CONTENT, "local commit");
    worktree
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

fn change_paths(terminal: &TestTerminal<App>) -> Vec<String> {
    terminal
        .driver()
        .changes()
        .iter()
        .map(|entry| entry.path.clone())
        .collect()
}

fn write_drift_file(repository_path: &Path) {
    fs::write(
        repository_path.join(DRIFT_PATH),
        "# written behind the app\n",
    )
    .expect("the drift file is written");
}

fn conflicted_paths(terminal: &TestTerminal<App>) -> Vec<String> {
    terminal
        .driver()
        .changes()
        .iter()
        .filter(|entry| entry.conflict.is_some())
        .map(|entry| entry.path.clone())
        .collect()
}

#[test]
fn a_failed_pull_refresh_reports_the_rebase_state() {
    let repository_path = repository_with_a_conflicting_upstream("failed-pull");
    let mut terminal = terminal_for(&repository_path);
    assert_eq!(
        terminal.driver().head_info().operation,
        None,
        "the scratch repository starts outside any operation"
    );

    let failure = terminal
        .driver_mut()
        .pull(PullStrategy::Rebase)
        .expect_err("the rebase pull stops on the conflicting upstream commit");
    assert!(
        repository_path.join(".git").join("rebase-merge").exists(),
        "the failed pull leaves the repository in a rebase: {failure}"
    );

    terminal.tick();
    assert_eq!(
        terminal.driver().head_info().operation,
        Some(RepoOperation::Rebasing),
        "the app reads the rebase state without a manual refresh, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        terminal.driver().head_info().detached,
        "the rebase leaves a detached head in the app state"
    );
    assert_eq!(
        conflicted_paths(&terminal),
        vec![String::from(SHARED_PATH)],
        "the refresh lists the conflicting path, screen was:\n{}",
        terminal.screen_text()
    );
    let screen_text = terminal.screen_text();
    assert!(
        screen_text.contains(REBASING_LABEL),
        "the footer reports the rebase, screen was:\n{screen_text}"
    );
    let (source, message) = terminal
        .driver()
        .panel_error()
        .expect("the failed pull reports itself");
    assert_eq!(
        source,
        gitdeck_tui::PanelErrorSource::Pull,
        "the failed pull keeps its own title"
    );
    assert!(
        message.contains("could not apply"),
        "the block keeps the git text that stopped the rebase: {message}"
    );
    assert!(
        screen_text.contains(gitdeck_tui::MERGE_CONFLICT_HINT),
        "the block names the continue and abort keys, screen was:\n{screen_text}"
    );
    assert!(
        screen_text.contains("Merge Conflicts"),
        "the changes pane holds the conflict group, screen was:\n{screen_text}"
    );
    terminal.screenshot("post_operation_refresh/failed-pull-rebasing");
}

#[test]
fn a_failed_merge_refresh_reports_the_merge_state() {
    let repository_path = diverged_worktree("failed-merge");
    let mut terminal = terminal_for(&repository_path);
    assert_eq!(terminal.driver().head_info().operation, None);

    run_failing_git(&repository_path, &["merge", "--quiet", SOURCE_BRANCH]);
    assert!(
        repository_path.join(".git").join("MERGE_HEAD").exists(),
        "the merge stops with conflicts"
    );

    terminal
        .driver_mut()
        .merge_branch(&repository_path_branch());

    terminal.tick();
    let (source, message) = terminal
        .driver()
        .panel_error()
        .expect("the refused merge reports itself");
    assert_eq!(
        source,
        gitdeck_tui::PanelErrorSource::Merge,
        "the refused merge keeps its own title"
    );
    assert!(
        !message.trim().is_empty(),
        "the refused merge carries a reason"
    );
    assert_eq!(
        terminal.driver().head_info().operation,
        Some(RepoOperation::Merging),
        "the app reads the merge state without a manual refresh, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        conflicted_paths(&terminal),
        vec![String::from(SHARED_PATH)],
        "the refresh lists the conflicting path, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("post_operation_refresh/failed-merge-merging");
}

fn repository_path_branch() -> String {
    String::from(SOURCE_BRANCH)
}

#[test]
fn a_failed_checkout_still_refreshes_the_view() {
    let repository_path = diverged_worktree("failed-checkout");
    let mut terminal = terminal_for(&repository_path);
    assert!(
        !change_paths(&terminal).contains(&String::from(DRIFT_PATH)),
        "the drift file is unknown before it exists"
    );

    write_drift_file(&repository_path);
    terminal.driver_mut().checkout_branch(MISSING_BRANCH);

    terminal.tick();
    assert!(
        change_paths(&terminal).contains(&String::from(DRIFT_PATH)),
        "the failed checkout still re-reads the changes, got {:?}",
        change_paths(&terminal)
    );
    terminal.screenshot("post_operation_refresh/failed-checkout-refreshed");
}

type FailingWriteAction = fn(&mut TestTerminal<App>);

fn failing_write_actions() -> Vec<(&'static str, FailingWriteAction)> {
    vec![
        ("commit", |terminal| {
            terminal.driver_mut().commit().ok();
        }),
        ("push", |terminal| {
            terminal
                .driver_mut()
                .push_request(no_remote_push_request())
                .ok();
        }),
        ("pull-fast-forward", |terminal| {
            terminal
                .driver_mut()
                .pull(PullStrategy::FastForwardOnly)
                .ok();
        }),
        ("pull-merge", |terminal| {
            terminal.driver_mut().pull(PullStrategy::Merge).ok();
        }),
        ("pull-rebase", |terminal| {
            terminal.driver_mut().pull(PullStrategy::Rebase).ok();
        }),
        ("fetch", |terminal| {
            terminal.driver_mut().fetch(None).ok();
        }),
        ("checkout", |terminal| {
            terminal.driver_mut().checkout_branch(MISSING_BRANCH);
        }),
        ("create-branch", |terminal| {
            terminal.send_keys("alt+n");
            terminal.send_text("bad name");
            terminal.send_keys("enter");
        }),
        ("delete-branch", |terminal| {
            terminal.driver_mut().delete_branch(MISSING_BRANCH);
        }),
        ("merge", |terminal| {
            terminal.driver_mut().merge_branch(MISSING_BRANCH);
        }),
        ("resolve-conflict", |terminal| {
            terminal
                .driver_mut()
                .resolve_conflict(MISSING_PATH, ConflictResolution::Ours);
        }),
        ("discard", |terminal| {
            terminal.driver_mut().discard_changes(MISSING_PATH);
        }),
        ("abort", |terminal| {
            terminal.driver_mut().abort_operation();
        }),
        ("continue", |terminal| {
            terminal.driver_mut().continue_operation();
        }),
    ]
}

fn no_remote_push_request() -> gitdeck_git::PushRequest {
    gitdeck_git::PushRequest {
        branch: String::from(CURRENT_BRANCH),
        set_upstream: false,
        force_with_lease: false,
    }
}

#[test]
fn every_write_action_refreshes_the_repository_state() {
    for (action, run_action) in failing_write_actions() {
        let repository_path = diverged_worktree(&format!("table-{action}"));
        let mut terminal = terminal_for(&repository_path);
        write_drift_file(&repository_path);
        assert!(
            !change_paths(&terminal).contains(&String::from(DRIFT_PATH)),
            "{action}: the drift file is unknown before the action runs"
        );

        run_action(&mut terminal);

        terminal.tick();
        assert!(
            change_paths(&terminal).contains(&String::from(DRIFT_PATH)),
            "{action}: the write action re-reads the changes even when it fails, got {:?}, screen was:\n{}",
            change_paths(&terminal),
            terminal.screen_text()
        );
        terminal.screenshot(&format!("post_operation_refresh/table-{action}"));
    }
}
