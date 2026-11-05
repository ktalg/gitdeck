use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, ChangesFocus, STATUS_MESSAGE_LIFETIME_TICKS};
use gitdeck_core::{ChangeKind, PaneId};
use gitdeck_git::GitError;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    changes_panel_layout, commit_label_color, text_display_width, theme_palette,
    ChangesPanelLayout, PanelErrorSource, StatusTone, COMMIT_BUTTON_LABEL, COMMIT_FAILED_LABEL,
    COMMIT_MESSAGE_BOX_MIN_HEIGHT, COMMIT_MESSAGE_PLACEHOLDER, PUSH_FAILED_LABEL,
    REFRESH_FAILED_LABEL,
};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const SHORT_PANEL_HEIGHT: u16 = 16;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/commit-pipeline";
const GIT_DIRECTORY_NAME: &str = ".git";
const BROKEN_GIT_DIRECTORY_NAME: &str = ".git-broken";
const BROKEN_GIT_DIRECTORY_FILE: &str = "gitdir: ./missing-gitdir\n";
const HEAD_SUBJECT: &str = "call run twice";
const NOTES_PATH: &str = "notes.txt";
const TYPED_MESSAGE: &str = "修复：冲突 📦";
const CLEAN_TREE_SUBJECT: &str = "clean work tree";
const AMENDED_SUBJECT: &str = "amended message";
const EMPTY_MESSAGE_STATUS: &str = "Commit message is empty";
const NOTHING_TO_COMMIT_STATUS: &str = "Nothing to commit";
const PUSH_FAILED_STATUS_PREFIX: &str = "Push failed: ";

fn commit_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the dirty fixture repository is copied");
    target
}

fn pipeline_terminal(case: &str, caps: CapsProfile) -> TestTerminal<App> {
    let repository_path = commit_repository(&format!("{case}-{}", caps.as_str()));
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, caps, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
}

fn panel_layout(terminal: &TestTerminal<App>) -> ChangesPanelLayout {
    changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        false,
    )
    .expect("the changes pane holds the commit block")
}

fn editable_terminal(case: &str, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = pipeline_terminal(case, caps);
    terminal.send_keys("ctrl+k");
    terminal.send_keys("i");
    assert_eq!(
        terminal.driver().changes_focus(),
        ChangesFocus::Message,
        "{caps:?} the i key enters the message editor"
    );
    terminal
}

fn commit_label_foreground(terminal: &TestTerminal<App>) -> Color {
    let layout = panel_layout(terminal);
    terminal
        .cell(layout.commit_button.x, layout.commit_button.y)
        .foreground
}

fn git_output(repository_path: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn head_subject(terminal: &TestTerminal<App>) -> String {
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the terminal holds a repository");
    git_output(repository_path, &["log", "-1", "--format=%s"])
        .trim()
        .to_string()
}

fn commit_count(terminal: &TestTerminal<App>) -> usize {
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the terminal holds a repository");
    git_output(repository_path, &["rev-list", "--count", "HEAD"])
        .trim()
        .parse()
        .expect("the commit count is a number")
}

fn break_git_directory(repository_path: &Path) {
    fs::rename(
        repository_path.join(GIT_DIRECTORY_NAME),
        repository_path.join(BROKEN_GIT_DIRECTORY_NAME),
    )
    .expect("the git directory is moved aside");
    fs::write(
        repository_path.join(GIT_DIRECTORY_NAME),
        BROKEN_GIT_DIRECTORY_FILE,
    )
    .expect("the broken git directory file is written");
}

fn restore_git_directory(repository_path: &Path) {
    fs::remove_file(repository_path.join(GIT_DIRECTORY_NAME))
        .expect("the broken git directory file is removed");
    fs::rename(
        repository_path.join(BROKEN_GIT_DIRECTORY_NAME),
        repository_path.join(GIT_DIRECTORY_NAME),
    )
    .expect("the git directory is restored");
}

fn move_to_the_unversioned_group(terminal: &mut TestTerminal<App>) {
    for _ in 0..4 {
        terminal.send_keys("j");
    }
}

fn uncheck_the_unversioned_drafts(terminal: &mut TestTerminal<App>) {
    move_to_the_unversioned_group(terminal);
    terminal.send_keys("space");
    assert!(
        !terminal
            .driver()
            .changes()
            .iter()
            .any(|entry| entry.kind == ChangeKind::Untracked
                && terminal.driver().checked_file_paths().contains(&entry.path)),
        "the unversioned drafts leave the checked set"
    );
}

fn check_only_the_app_file(terminal: &mut TestTerminal<App>) {
    move_to_the_unversioned_group(terminal);
    terminal.send_keys("space");
    terminal.send_keys("k k k");
    terminal.send_keys("space");
    terminal.send_keys("j j");
    terminal.send_keys("space");
    assert_eq!(
        terminal.driver().checked_file_paths(),
        vec![String::from("src/app.rs")]
    );
}

#[test]
fn typing_appends_to_the_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = editable_terminal("typing", caps);
        let mut typed = String::new();
        for character in TYPED_MESSAGE.chars() {
            terminal.send_text(&character.to_string());
            typed.push(character);
            assert_eq!(
                terminal.driver().commit_message(),
                typed,
                "{caps:?} typing {character:?} appends one character"
            );
        }
        assert_eq!(terminal.driver().commit_message(), TYPED_MESSAGE);
        let layout = panel_layout(&terminal);
        assert_eq!(
            terminal
                .cell(layout.message_lines.x, layout.message_lines.y)
                .symbol,
            "修",
            "{caps:?} the message box shows the typed message"
        );
        terminal.assert_missing_glyphs(&['修', '复', '：', '冲', '突', '📦']);
        assert_eq!(
            commit_label_foreground(&terminal),
            theme_palette(ThemeId::JbDark).accent,
            "{caps:?} a typed message enables the commit label"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("commit_pipeline/typing");
        }
    }
}

#[test]
fn backspace_and_undo_restore_previous_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = editable_terminal("backspace-undo", caps);
        terminal.send_text("subject line");
        let typed = String::from("subject line");
        assert_eq!(terminal.driver().commit_message(), typed);
        for _ in 0..3 {
            terminal.send_keys("backspace");
        }
        assert_eq!(terminal.driver().commit_message(), "subject l");
        for _ in 0..3 {
            terminal.send_keys("ctrl+z");
        }
        assert_eq!(
            terminal.driver().commit_message(),
            typed,
            "{caps:?} three undos restore the three removed characters"
        );
        terminal.send_keys("ctrl+z");
        assert_eq!(
            terminal.driver().commit_message(),
            "subject lin",
            "{caps:?} undo keeps walking back through the edit history"
        );
    }
}

#[test]
fn enter_inserts_a_newline_in_the_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = editable_terminal("newline", caps);
        terminal.send_text("subject");
        terminal.send_keys("enter");
        terminal.send_text("body");
        assert_eq!(terminal.driver().commit_message(), "subject\nbody");
        let layout = panel_layout(&terminal);
        assert_eq!(
            terminal
                .cell(layout.message_lines.x, layout.message_lines.y + 1)
                .symbol,
            "b",
            "{caps:?} the second message line is rendered below the first"
        );
    }
}

#[test]
fn commit_is_disabled_without_a_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = editable_terminal("disabled-without-message", caps);
        terminal.send_keys("esc");
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(
            commit_label_foreground(&terminal),
            commit_label_color(false, &theme),
            "{caps:?} the commit label is muted while the message is empty"
        );
        let layout = panel_layout(&terminal);
        assert_eq!(
            terminal
                .cell(layout.message_lines.x, layout.message_lines.y)
                .symbol,
            "C",
            "{caps:?} the empty message box shows the placeholder"
        );
        assert!(
            terminal.screen_text().contains(COMMIT_MESSAGE_PLACEHOLDER),
            "{caps:?} the placeholder label is visible"
        );
        let commits_before = commit_count(&terminal);
        terminal.send_keys("ctrl+enter");
        terminal.mouse_click(layout.commit_button.x, layout.commit_button.y);
        assert_eq!(terminal.driver().panel_error(), None);
        assert_eq!(terminal.driver().commit_message(), "");
        assert!(
            !terminal.screen_text().contains(REFRESH_FAILED_LABEL),
            "{caps:?} a commit without a message reports no error"
        );
        assert_eq!(commit_count(&terminal), commits_before);
        assert_eq!(terminal.driver().changes().len(), 4);
        let error = terminal
            .driver_mut()
            .commit()
            .expect_err("an empty message is rejected");
        assert!(
            matches!(error, GitError::EmptyCommitMessage),
            "{caps:?} {error:?}"
        );
    }
}

#[test]
fn commit_is_disabled_without_checked_files() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("disabled-without-files", caps);
        terminal.send_keys("space");
        move_to_the_unversioned_group(&mut terminal);
        terminal.send_keys("space");
        assert!(
            terminal.driver().checked_file_paths().is_empty(),
            "{caps:?} every file is unchecked"
        );
        terminal.send_keys("i");
        terminal.send_text("nothing selected");
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(
            commit_label_foreground(&terminal),
            commit_label_color(false, &theme),
            "{caps:?} the commit label is muted without checked files"
        );
        let commits_before = commit_count(&terminal);
        terminal.send_keys("ctrl+enter");
        let layout = panel_layout(&terminal);
        terminal.mouse_click(layout.push_button.x, layout.push_button.y);
        assert_eq!(terminal.driver().panel_error(), None);
        assert_eq!(terminal.driver().commit_message(), "nothing selected");
        assert_eq!(commit_count(&terminal), commits_before);
        assert_eq!(terminal.driver().changes().len(), 4);
        let error = terminal
            .driver_mut()
            .commit()
            .expect_err("an empty path list is rejected");
        assert!(
            matches!(error, GitError::NothingToCommit),
            "{caps:?} {error:?}"
        );
        assert_eq!(
            terminal.driver().commit_message(),
            "nothing selected",
            "{caps:?} the rejected commit keeps the message"
        );
    }
}

#[test]
fn alt_a_toggles_amend_and_prefills_the_last_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("amend-toggle", caps);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("alt+a");
        assert!(terminal.driver().amend(), "{caps:?} alt+a checks amend");
        assert_eq!(
            terminal.driver().commit_message(),
            HEAD_SUBJECT,
            "{caps:?} amend prefills the last commit message"
        );
        let layout = panel_layout(&terminal);
        assert!(
            terminal
                .cell(layout.amend_row.x + 2, layout.amend_row.y)
                .symbol
                .contains('['),
            "{caps:?} the amend row keeps its checkbox"
        );
        terminal.send_keys("alt+a");
        assert!(!terminal.driver().amend(), "{caps:?} alt+a unchecks amend");
        assert_eq!(
            terminal.driver().commit_message(),
            HEAD_SUBJECT,
            "{caps:?} unchecking amend keeps the typed message"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("commit_pipeline/amend");
        }
    }
}

#[test]
fn alt_s_toggles_sign_off() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("sign-off-toggle", caps);
        terminal.send_keys("ctrl+k");
        assert!(!terminal.driver().sign_off());
        terminal.send_keys("alt+s");
        assert!(terminal.driver().sign_off(), "{caps:?} alt+s signs off");
        terminal.send_keys("alt+s");
        assert!(!terminal.driver().sign_off(), "{caps:?} alt+s toggles back");
    }
}

#[test]
fn commit_takes_only_the_checked_paths_and_leaves_untracked_files_alone() {
    let mut terminal = pipeline_terminal("checked-only", CapsProfile::Rich);
    check_only_the_app_file(&mut terminal);
    terminal.send_keys("i");
    terminal.send_text("commit the app file");
    terminal.send_keys("ctrl+enter");
    assert_eq!(terminal.driver().panel_error(), None);
    assert_eq!(terminal.driver().commit_message(), "");
    assert_eq!(head_subject(&terminal), "commit the app file");
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the terminal holds a repository");
    let mut committed_paths: Vec<String> = git_output(
        repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    )
    .lines()
    .map(str::to_string)
    .collect();
    committed_paths.sort();
    assert_eq!(
        committed_paths,
        vec![String::from("src/app.rs")],
        "the commit takes the checked file only, not the pre-staged one"
    );
    assert!(
        !committed_paths.contains(&String::from(NOTES_PATH)),
        "the unchecked untracked file stays out of the commit: {committed_paths:?}"
    );
    let mut remaining: Vec<String> = terminal
        .driver()
        .changes()
        .iter()
        .map(|change| change.path.clone())
        .collect();
    remaining.sort();
    assert_eq!(
        remaining,
        vec![
            String::from("docs/readme.md"),
            String::from(NOTES_PATH),
            String::from("src/module.rs"),
        ],
        "the committed path leaves the list and the work tree edits stay"
    );
    assert_eq!(
        git_output(repository_path, &["diff", "--cached", "--name-only"]).trim(),
        "src/module.rs",
        "the pre-staged unchecked path keeps its index entry"
    );
    terminal.screenshot("commit_pipeline/checked-only");
}

#[test]
fn successful_commit_clears_the_message_and_the_changes_list() {
    let mut terminal = pipeline_terminal("successful", CapsProfile::Rich);
    terminal.send_keys("ctrl+k");
    let commits_before = commit_count(&terminal);
    terminal.send_keys("i");
    terminal.send_text("commit everything");
    terminal.send_keys("ctrl+enter");
    assert_eq!(terminal.driver().panel_error(), None);
    assert_eq!(terminal.driver().commit_message(), "");
    assert_eq!(
        terminal.driver().changes_focus(),
        ChangesFocus::List,
        "a successful commit leaves the editor"
    );
    assert!(terminal.driver().changes().is_empty());
    assert!(terminal.screen_text().contains("No changes"));
    assert_eq!(commit_count(&terminal), commits_before + 1);
    let newest = terminal
        .driver()
        .commits()
        .first()
        .expect("the log reloads after the commit");
    assert_eq!(newest.subject, "commit everything");
    assert!(!terminal.driver().amend());
    terminal.screenshot("commit_pipeline/successful");
}

#[test]
fn commit_failure_is_visible_and_does_not_clear_the_message() {
    let mut terminal = pipeline_terminal("failing", CapsProfile::Rich);
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the terminal holds a repository")
        .to_path_buf();
    terminal.send_keys("ctrl+k");
    terminal.send_keys("i");
    terminal.send_text("doomed commit");
    break_git_directory(&repository_path);
    terminal.send_keys("ctrl+enter");
    let (source, message) = terminal
        .driver()
        .panel_error()
        .expect("the failing commit records a visible error");
    assert_eq!(source, PanelErrorSource::Commit);
    assert_eq!(source.title(), COMMIT_FAILED_LABEL);
    assert!(
        !message.is_empty(),
        "the panel error carries the git message"
    );
    assert!(
        terminal.screen_text().contains(COMMIT_FAILED_LABEL),
        "the failure is rendered, screen text was:\n{}",
        terminal.screen_text()
    );
    assert!(
        !terminal.screen_text().contains(REFRESH_FAILED_LABEL),
        "a failed commit is not reported as a failed refresh, screen text was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(terminal.driver().commit_message(), "doomed commit");
    restore_git_directory(&repository_path);
}

#[test]
fn open_repository_stores_a_trait_object_backend() {
    let repository_path = commit_repository("trait-object-backend");
    let mut app = App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
        .expect("the scratch repository opens");
    let stored_path = app
        .repository_path()
        .expect("the app keeps the backend path");
    assert_eq!(
        std::fs::canonicalize(stored_path).expect("the stored path exists"),
        std::fs::canonicalize(&repository_path).expect("the scratch repository exists")
    );
    assert!(
        !app.changes().is_empty(),
        "the backend reports the dirty work tree"
    );
    assert!(app.refresh().is_ok());
    assert_eq!(app.panel_error(), None);
}

fn clean_pipeline_terminal(case: &str, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = pipeline_terminal(case, caps);
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the terminal holds a repository")
        .to_path_buf();
    git_output(&repository_path, &["add", "--all"]);
    git_output(
        &repository_path,
        &["commit", "-q", "-m", CLEAN_TREE_SUBJECT],
    );
    terminal.send_keys("f5");
    terminal
}

#[test]
fn amend_without_file_changes_rewrites_the_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = clean_pipeline_terminal("amend-message-only", caps);
        assert!(
            terminal.driver().changes().is_empty(),
            "{caps:?} the work tree starts clean"
        );
        let commits_before = commit_count(&terminal);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("alt+a");
        assert!(terminal.driver().amend(), "{caps:?} alt+a enables amend");
        terminal.driver_mut().set_commit_message(AMENDED_SUBJECT);
        terminal.send_keys("ctrl+enter");
        assert_eq!(
            terminal.driver().panel_error(),
            None,
            "{caps:?} the message-only amend succeeds"
        );
        assert_eq!(
            commit_count(&terminal),
            commits_before,
            "{caps:?} the amend adds no commit"
        );
        assert_eq!(head_subject(&terminal), AMENDED_SUBJECT);
        assert!(terminal.driver().changes().is_empty());
        if caps == CapsProfile::Rich {
            terminal.screenshot("commit_pipeline/amend-message-only");
        }
    }
}

#[test]
fn amend_without_files_reports_an_amended_commit() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = clean_pipeline_terminal("amend-receipt-message-only", caps);
        let commits_before = commit_count(&terminal);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("alt+a");
        terminal.driver_mut().set_commit_message(AMENDED_SUBJECT);
        terminal.send_keys("ctrl+enter");
        assert_eq!(
            terminal.driver().status_message(),
            Some(("Amended commit", StatusTone::Info)),
            "{caps:?} a message-only amend reports the amended commit"
        );
        let screen = terminal.screen_text();
        assert!(
            screen.contains("Amended commit"),
            "{caps:?} the receipt is on the status bar, screen was:\n{screen}"
        );
        assert!(
            !screen.contains("Committed 0 files"),
            "{caps:?} the amend never claims a zero file commit, screen was:\n{screen}"
        );
        assert_eq!(commit_count(&terminal), commits_before);
        assert_eq!(head_subject(&terminal), AMENDED_SUBJECT);
        terminal.screenshot("commit_pipeline/amend-receipt");
    }
}

#[test]
fn amend_with_files_counts_them() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("amend-receipt-with-files", caps);
        let commits_before = commit_count(&terminal);
        terminal.send_keys("ctrl+k");
        uncheck_the_unversioned_drafts(&mut terminal);
        terminal.send_keys("alt+a");
        let checked_file_count = terminal.driver().checked_file_paths().len();
        assert!(
            checked_file_count > 0,
            "{caps:?} the dirty work tree has checked files"
        );
        terminal.send_keys("ctrl+enter");
        let expected_receipt = format!("Amended commit with {checked_file_count} files");
        assert_eq!(
            terminal.driver().status_message(),
            Some((expected_receipt.as_str(), StatusTone::Info)),
            "{caps:?} the amend receipt counts the committed files"
        );
        assert!(
            terminal.screen_text().contains(&expected_receipt),
            "{caps:?} the receipt is on the status bar"
        );
        assert_eq!(
            commit_count(&terminal),
            commits_before,
            "{caps:?} the amend rewrites the head commit"
        );
        assert_eq!(head_subject(&terminal), HEAD_SUBJECT);
    }
}

#[test]
fn receipts_use_singular_for_one_file() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("receipt-singular", caps);
        terminal.send_keys("ctrl+k");
        check_only_the_app_file(&mut terminal);
        assert_eq!(terminal.driver().checked_file_paths().len(), 1);
        terminal.send_keys("i");
        terminal.send_text("single file receipt");
        terminal.send_keys("ctrl+enter");
        assert_eq!(
            terminal.driver().status_message(),
            Some(("Committed 1 file to main", StatusTone::Info)),
            "{caps:?} a single committed file reads as one file, screen was:\n{}",
            terminal.screen_text()
        );
        let mut amended = pipeline_terminal("receipt-singular-amend", caps);
        amended.send_keys("ctrl+k");
        check_only_the_app_file(&mut amended);
        amended.send_keys("alt+a");
        amended.send_keys("ctrl+enter");
        assert_eq!(
            amended.driver().status_message(),
            Some(("Amended commit with 1 file", StatusTone::Info)),
            "{caps:?} a single amended file reads as one file, screen was:\n{}",
            amended.screen_text()
        );
    }
}

#[test]
fn plain_commit_message_is_unchanged() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("receipt-plain", caps);
        terminal.send_keys("ctrl+k");
        let checked_file_count = terminal.driver().checked_file_paths().len();
        terminal.send_keys("i");
        terminal.send_text("plain receipt");
        terminal.send_keys("ctrl+enter");
        let expected_receipt = format!("Committed {checked_file_count} files to main");
        assert_eq!(
            terminal.driver().status_message(),
            Some((expected_receipt.as_str(), StatusTone::Info)),
            "{caps:?} a plain commit keeps its receipt wording"
        );
    }
}

#[test]
fn empty_message_shows_a_status_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("status-empty-message", caps);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("ctrl+enter");
        assert_eq!(
            terminal.driver().status_message(),
            Some((EMPTY_MESSAGE_STATUS, StatusTone::Failure)),
            "{caps:?} the empty message is reported"
        );
        assert!(terminal.screen_text().contains(EMPTY_MESSAGE_STATUS));
        for _ in 0..STATUS_MESSAGE_LIFETIME_TICKS - 1 {
            terminal.tick();
        }
        assert_eq!(
            terminal.driver().status_message(),
            Some((EMPTY_MESSAGE_STATUS, StatusTone::Failure)),
            "{caps:?} the message survives the first ticks"
        );
        terminal.tick();
        assert_eq!(
            terminal.driver().status_message(),
            None,
            "{caps:?} the message expires after its lifetime"
        );
        assert!(!terminal.screen_text().contains(EMPTY_MESSAGE_STATUS));
    }
}

#[test]
fn nothing_checked_shows_a_status_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("status-nothing-checked", caps);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("space");
        move_to_the_unversioned_group(&mut terminal);
        terminal.send_keys("space");
        assert!(terminal.driver().checked_file_paths().is_empty());
        terminal.send_keys("i");
        terminal.send_text("nothing to commit");
        terminal.send_keys("ctrl+enter");
        assert_eq!(
            terminal.driver().status_message(),
            Some((NOTHING_TO_COMMIT_STATUS, StatusTone::Failure)),
            "{caps:?} the empty selection is reported"
        );
        assert!(terminal.screen_text().contains(NOTHING_TO_COMMIT_STATUS));
    }
}

#[test]
fn successful_commit_shows_a_receipt() {
    let mut terminal = pipeline_terminal("status-receipt", CapsProfile::Rich);
    terminal.send_keys("ctrl+k");
    let checked_file_count = terminal.driver().checked_file_paths().len();
    terminal.send_keys("i");
    terminal.send_text("receipt commit");
    terminal.send_keys("ctrl+enter");
    let expected_receipt = format!("Committed {checked_file_count} files to main");
    assert_eq!(
        terminal.driver().status_message(),
        Some((expected_receipt.as_str(), StatusTone::Info))
    );
    assert!(
        terminal.screen_text().contains(&expected_receipt),
        "the receipt is on the status bar, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("commit_pipeline/status-receipt");
}

#[test]
fn push_failure_uses_its_own_title() {
    let mut terminal = pipeline_terminal("status-push-failure", CapsProfile::Rich);
    terminal.send_keys("ctrl+k");
    terminal.send_keys("i");
    terminal.send_text("commit and push");
    let layout = panel_layout(&terminal);
    terminal.mouse_click(layout.push_button.x + 1, layout.push_button.y);
    let panel_error = terminal
        .driver()
        .panel_error()
        .expect("the failed push records a panel error");
    assert_eq!(panel_error.0, PanelErrorSource::Push);
    assert!(terminal.screen_text().contains(PUSH_FAILED_LABEL));
    assert!(
        !terminal.screen_text().contains(REFRESH_FAILED_LABEL),
        "the push failure keeps its own title"
    );
    let (status_text, status_tone) = terminal
        .driver()
        .status_message()
        .expect("the failed push leaves a status message");
    assert!(
        status_text.starts_with(PUSH_FAILED_STATUS_PREFIX),
        "status message {status_text:?}"
    );
    assert_eq!(status_tone, StatusTone::Failure);
    terminal.screenshot("commit_pipeline/push-failure");
}

#[test]
fn commit_button_row_survives_a_short_panel() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = pipeline_terminal("short-panel", caps);
        terminal.resize(80, SHORT_PANEL_HEIGHT);
        let layout = panel_layout(&terminal);
        assert_eq!(
            layout.message_box.height, COMMIT_MESSAGE_BOX_MIN_HEIGHT,
            "{caps:?} a short panel keeps three message rows"
        );
        assert!(
            layout.message_lines.height < layout.message_box.height,
            "{caps:?} the message box keeps its border"
        );
        assert_eq!(layout.button_row.height, 1);
        let button_row = layout.button_row;
        let text: String = (button_row.x..button_row.right())
            .map(|column| terminal.cell(column, button_row.y).symbol.clone())
            .collect();
        assert!(
            text.contains(COMMIT_BUTTON_LABEL),
            "{caps:?} the commit button row survives, row {text:?}"
        );
        let push_label = layout
            .buttons
            .push_label(caps)
            .expect("the narrow row still offers the push button");
        assert!(
            text.contains(&push_label),
            "{caps:?} the push button row survives with the label that fits, row {text:?}"
        );
        assert!(
            layout.push_button.width >= text_display_width(&push_label),
            "{caps:?} the push button is never narrower than its label, row {text:?}"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("commit_pipeline/short-panel");
        }
    }
}
