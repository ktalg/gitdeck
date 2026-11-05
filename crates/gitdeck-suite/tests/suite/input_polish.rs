use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, EditorTab};
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    box_glyphs, changes_panel_layout, panel_error_layout, CHANGES_PANEL_CONTENT_TOP,
    COMMIT_FAILED_LABEL, FILE_NAME_COLUMN, PANEL_ERROR_OK_LABEL, REFRESH_FAILED_LABEL,
};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SELECTED_BACKGROUND: Color = Color::Rgb(0x2E, 0x43, 0x6E);
const ERROR_COLOR: Color = Color::Rgb(0xDB, 0x5C, 0x5C);
const MUTED_COLOR: Color = Color::Rgb(0x86, 0x8A, 0x91);
const PANEL_BACKGROUND: Color = Color::Rgb(0x2B, 0x2D, 0x30);
const WHEEL_STEP_ROWS: u16 = 3;
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const LARGE_FILE_PATH: &str = "src/large.txt";
const APP_FILE_PATH: &str = "src/app.rs";
const APP_FILE_TITLE: &str = "app.rs (Working Tree)";
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const GIT_DIRECTORY_NAME: &str = ".git";
const BROKEN_GIT_DIRECTORY_NAME: &str = ".git-broken";
const BROKEN_GIT_DIRECTORY_FILE: &str = "gitdir: ./missing-gitdir\n";

fn app_terminal(
    fixture: FixtureRepo,
    width: u16,
    height: u16,
    caps: CapsProfile,
) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        fixture,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn changes_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible")
}

fn changes_content_top(terminal: &TestTerminal<App>) -> u16 {
    changes_pane(terminal).y + CHANGES_PANEL_CONTENT_TOP
}

fn changes_list_rows(terminal: &TestTerminal<App>) -> Rect {
    changes_panel_layout(changes_pane(terminal), false)
        .expect("the changes panel holds a commit block")
        .list
}

fn changes_line_text(terminal: &TestTerminal<App>, row: u16) -> String {
    let changes = changes_pane(terminal);
    let mut text = String::new();
    for column in changes.x..changes.right() {
        text.push_str(&terminal.cell(column, row).symbol);
    }
    text.trim_end().to_string()
}

fn changes_pane_row(terminal: &TestTerminal<App>, left: u16, row: u16) -> String {
    let changes = changes_pane(terminal);
    let mut text = String::new();
    for column in left..changes.right() {
        text.push_str(&terminal.cell(column, row).symbol);
    }
    text.trim_end().to_string()
}

fn changes_row_of(terminal: &TestTerminal<App>, needle: &str) -> u16 {
    let changes = changes_pane(terminal);
    (changes.y..changes.bottom())
        .find(|row| changes_line_text(terminal, *row).contains(needle))
        .unwrap_or_else(|| panic!("no changes row holds {needle:?}"))
}

fn selected_changes_row(terminal: &TestTerminal<App>) -> Option<u16> {
    let changes = changes_pane(terminal);
    (changes.y..changes.bottom())
        .find(|row| terminal.cell(changes.x, *row).background == SELECTED_BACKGROUND)
}

fn select_file(terminal: &mut TestTerminal<App>, path: &str) {
    terminal.send_keys("ctrl+k");
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        if terminal.driver().selected_file_path() == Some(path) {
            return;
        }
        terminal.send_keys("j");
    }
    panic!(
        "the changes pane never selected {path}, the cursor holds {:?}",
        terminal.driver().selected_file_path()
    );
}

fn open_selected_diff(terminal: &mut TestTerminal<App>, path: &str) {
    select_file(terminal, path);
    terminal.send_keys("enter");
}

fn diff_row_total(terminal: &TestTerminal<App>) -> usize {
    terminal
        .driver()
        .file_diff()
        .map(|diff| {
            diff.hunks
                .iter()
                .map(|hunk| hunk.lines.len() + 1)
                .sum::<usize>()
        })
        .expect("a diff is open")
}

fn selected_commit_id(terminal: &TestTerminal<App>) -> Option<String> {
    terminal
        .driver()
        .selected_commit()
        .map(|commit| commit.id.clone())
}

#[test]
fn wheel_over_changes_moves_selection_by_three() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            app_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.send_keys("ctrl+k");
        let changes = changes_pane(&terminal);
        let top = changes_content_top(&terminal);
        let last_row = top + 5;
        assert_eq!(
            selected_changes_row(&terminal),
            Some(top),
            "{caps:?} starts on the group row"
        );
        terminal.mouse_wheel(changes.x + 10, top + 1, -1);
        assert_eq!(
            selected_changes_row(&terminal),
            Some(top + WHEEL_STEP_ROWS),
            "{caps:?} one wheel notch moves the selection three rows"
        );
        for _ in 0..10 {
            terminal.mouse_wheel(changes.x + 10, top + 1, -1);
        }
        assert_eq!(
            selected_changes_row(&terminal),
            Some(last_row),
            "{caps:?} the wheel stops on the last row"
        );
        for _ in 0..10 {
            terminal.mouse_wheel(changes.x + 10, top + 1, 1);
        }
        assert_eq!(
            selected_changes_row(&terminal),
            Some(top),
            "{caps:?} the wheel stops on the first row"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("input_polish/wheel-changes");
        }
    }
}

#[test]
fn wheel_over_commits_scrolls_the_viewport_and_drags_the_selection() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = app_terminal(
            FixtureRepo::RebaseDiverged,
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            caps,
        );
        let history = terminal.driver().commits().to_vec();
        assert_eq!(history.len(), 4, "{caps:?} commit count");
        terminal.send_keys("ctrl+k");
        let commits = terminal
            .pane_rect(PaneId::Log)
            .expect("the commits pane is visible");
        let changes = changes_pane(&terminal);
        terminal.mouse_wheel(changes.x + 10, changes.y + 3, -1);
        assert_eq!(
            selected_commit_id(&terminal),
            Some(history[0].id.clone()),
            "{caps:?} the wheel over the focused changes pane leaves the commit selection alone"
        );
        terminal.mouse_wheel(commits.x + 10, commits.y + 1, -1);
        assert_eq!(
            selected_commit_id(&terminal),
            Some(history[3].id.clone()),
            "{caps:?} one wheel notch moves the commit selection three rows"
        );
        for _ in 0..gitdeck_app::DETAILS_QUERY_SETTLE_TICKS {
            terminal.tick();
        }
        assert!(
            terminal
                .driver()
                .selected_commit_files()
                .iter()
                .any(|file| file.path == "src/service.rs"),
            "{caps:?} the details column loads the files of the wheeled commit"
        );
        assert!(
            terminal
                .screen_lines()
                .iter()
                .any(|line| line.contains(&history[3].short_id[..7])),
            "{caps:?} the details column shows the commit {}",
            history[3].short_id
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("input_polish/wheel-commits");
        }
    }
}

#[test]
fn uppercase_letters_are_inserted_in_the_message() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            app_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("i");
        for piece in ["Hello World", " ABC", " MiXeD123"] {
            type_with_shift(&mut terminal, piece);
        }
        assert_eq!(
            terminal.driver().commit_message(),
            "Hello World ABC MiXeD123",
            "{caps:?} the editor keeps every uppercase letter"
        );
        assert!(
            terminal.screen_text().contains("MiXeD123"),
            "{caps:?} the message box shows the typed text, screen was:\n{}",
            terminal.screen_text()
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("input_polish/uppercase-message");
        }
    }
}

fn type_with_shift(terminal: &mut TestTerminal<App>, text: &str) {
    for character in text.chars() {
        match character {
            ' ' => terminal.send_keys("space"),
            uppercase if uppercase.is_ascii_uppercase() => {
                terminal.send_keys(&format!("shift+{}", uppercase.to_ascii_lowercase()))
            }
            other => terminal.send_keys(&other.to_string()),
        }
    }
}

#[test]
fn wheel_over_diff_scrolls_the_diff() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = app_terminal(
            FixtureRepo::DiffShowcase,
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            caps,
        );
        open_selected_diff(&mut terminal, LARGE_FILE_PATH);
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Diff(String::from(LARGE_FILE_PATH)),
            "{caps:?} active tab"
        );
        let log = terminal
            .pane_rect(PaneId::Log)
            .expect("the diff pane is visible");
        terminal.mouse_wheel(log.x + 10, log.y + 1, -1);
        assert_eq!(
            terminal.driver().diff_scroll(),
            usize::from(WHEEL_STEP_ROWS),
            "{caps:?} one wheel notch scrolls the diff three rows"
        );
        terminal.mouse_wheel(log.x + 10, log.y + 1, 1);
        assert_eq!(
            terminal.driver().diff_scroll(),
            0,
            "{caps:?} a wheel notch upwards scrolls back"
        );
        for _ in 0..200 {
            terminal.mouse_wheel(log.x + 10, log.y + 1, -1);
        }
        assert_eq!(
            terminal.driver().diff_scroll(),
            diff_row_total(&terminal) - 1,
            "{caps:?} the wheel stops at the last diff row"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("input_polish/wheel-diff");
        }
    }
}

#[test]
fn wheel_on_the_divider_moves_the_left_neighbour_selection() {
    let mut terminal = app_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    let changes = changes_pane(&terminal);
    let top = changes_content_top(&terminal);
    terminal.mouse_wheel(changes.right() - 1, top + 1, -1);
    assert_eq!(
        selected_changes_row(&terminal),
        Some(top + WHEEL_STEP_ROWS),
        "the divider column belongs to the changes pane on its left"
    );
}

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", "main"]);
    fs::write(path.join("tracked.txt"), "tracked\n").expect("the tracked file is written");
    run_git(&path, &["add", "tracked.txt"]);
    run_git(
        &path,
        &[
            "-c",
            "user.name=gitdeck-suite",
            "-c",
            "user.email=suite@gitdeck.invalid",
            "commit",
            "-q",
            "-m",
            "base commit",
        ],
    );
    fs::write(path.join("tracked.txt"), "tracked change\n").expect("the edit is written");
    path
}

fn run_git(repository_path: &Path, arguments: &[&str]) {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

fn scratch_terminal(repository_path: &Path, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, caps, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
}

fn reference_screen() -> Rect {
    Rect::new(0, 0, REFERENCE_WIDTH, REFERENCE_HEIGHT)
}

#[test]
fn refresh_error_is_rendered_after_a_failing_refresh() {
    for caps in EVERY_CAPS_PROFILE {
        let repository_path = scratch_repository(&format!("input-polish-failing-{caps:?}"));
        let mut terminal = scratch_terminal(&repository_path, caps);
        let changes = changes_pane(&terminal);
        let list = changes_list_rows(&terminal);
        let block = panel_error_layout(list, reference_screen())
            .expect("the changes list holds the error block")
            .block;
        let body = panel_error_layout(list, reference_screen())
            .expect("the changes list holds the error block")
            .body;
        assert_eq!(
            changes_line_text(&terminal, block.y),
            "",
            "{caps:?} no error row before the refresh fails"
        );
        break_git_directory(&repository_path);
        terminal.send_keys("f5");
        assert!(
            terminal.driver().panel_error().is_some(),
            "{caps:?} the failing refresh records an error"
        );
        let title_text = changes_line_text(&terminal, block.y);
        assert!(
            title_text.contains(REFRESH_FAILED_LABEL),
            "{caps:?} the block names the failing source, row {title_text:?}"
        );
        assert_eq!(
            terminal.cell(changes.x, block.y).symbol,
            box_glyphs(caps).top_left.to_string(),
            "{caps:?} the block starts with its top left corner"
        );
        assert_eq!(
            terminal.cell(body.x, block.y).foreground,
            ERROR_COLOR,
            "{caps:?} the title uses the error color"
        );
        assert_eq!(
            terminal.cell(body.x, block.y).background,
            PANEL_BACKGROUND,
            "{caps:?} the title keeps the panel background"
        );
        let failure_text = changes_pane_row(&terminal, body.x, body.y);
        assert!(
            failure_text.starts_with("git status"),
            "{caps:?} the failure reason starts inside the block, got {failure_text:?}"
        );
        let wrapped_text = changes_pane_row(&terminal, body.x, body.y + 1);
        assert!(
            !wrapped_text.trim().is_empty(),
            "{caps:?} the failure reason wraps onto the second body row"
        );
        assert!(
            failure_text.chars().count() < usize::from(changes.width),
            "{caps:?} the failure reason wraps instead of overflowing, row {failure_text:?}"
        );
        assert_eq!(
            terminal.cell(body.x, body.y).foreground,
            MUTED_COLOR,
            "{caps:?} the failure reason uses the muted color"
        );
        assert!(
            changes_line_text(&terminal, changes_content_top(&terminal)).contains("Changes 1 file"),
            "{caps:?} the list keeps its rows above the error"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("input_polish/refresh-failed");
        }
        restore_git_directory(&repository_path);
    }
}

#[test]
fn successful_refresh_clears_the_error() {
    for caps in EVERY_CAPS_PROFILE {
        let repository_path = scratch_repository(&format!("input-polish-recovering-{caps:?}"));
        let mut terminal = scratch_terminal(&repository_path, caps);
        let block = panel_error_layout(changes_list_rows(&terminal), reference_screen())
            .expect("the changes list holds the error block")
            .block;
        break_git_directory(&repository_path);
        terminal.send_keys("f5");
        assert!(
            changes_line_text(&terminal, block.y).contains(REFRESH_FAILED_LABEL),
            "{caps:?} the error block is visible before the recovery"
        );
        restore_git_directory(&repository_path);
        terminal.send_keys("f5");
        assert_eq!(
            terminal.driver().panel_error(),
            None,
            "{caps:?} a successful refresh clears the error"
        );
        assert_eq!(
            changes_line_text(&terminal, block.y),
            "",
            "{caps:?} the cleared error row is blank again"
        );
        assert!(
            !terminal.screen_text().contains(REFRESH_FAILED_LABEL),
            "{caps:?} the label leaves the screen"
        );
    }
}

fn install_failing_commit_hook(repository_path: &Path) {
    let hook_path = repository_path
        .join(GIT_DIRECTORY_NAME)
        .join("hooks")
        .join("pre-commit");
    fs::write(
        &hook_path,
        "#!/bin/sh\nprintf 'the pre-commit hook refused the commit\\n' >&2\nexit 1\n",
    )
    .expect("the failing pre-commit hook is written");
    grant_execute_permission(&hook_path);
}

#[cfg(unix)]
fn grant_execute_permission(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .expect("the hook file exists")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("the hook file becomes executable");
}

#[cfg(not(unix))]
fn grant_execute_permission(_path: &Path) {}

fn terminal_holding_a_failed_commit(name: &str, caps: CapsProfile) -> TestTerminal<App> {
    let repository_path = scratch_repository(name);
    install_failing_commit_hook(&repository_path);
    let mut terminal = scratch_terminal(&repository_path, caps);
    terminal.driver_mut().set_commit_message("doomed commit");
    let commit_button = changes_panel_layout(changes_pane(&terminal), false)
        .expect("the changes panel holds the commit block")
        .commit_button;
    terminal.mouse_click(commit_button.x + 1, commit_button.y);
    assert!(
        terminal.driver().panel_error().is_some(),
        "{caps:?} the failing commit records a panel error"
    );
    terminal
}

#[test]
fn panel_error_shows_a_bordered_block_with_ok() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal =
            terminal_holding_a_failed_commit(&format!("input-polish-error-block-{caps:?}"), caps);
        let layout = panel_error_layout(changes_list_rows(&terminal), reference_screen())
            .expect("the changes list holds the error block");
        let block = layout.block;
        let glyphs = box_glyphs(caps);
        let title_line = changes_line_text(&terminal, block.y);
        assert!(
            title_line.contains(COMMIT_FAILED_LABEL),
            "{caps:?} the block title names the source, row {title_line:?}"
        );
        assert_eq!(
            terminal.cell(block.x, block.y).symbol,
            glyphs.top_left.to_string(),
            "{caps:?} the block draws its top left corner"
        );
        assert_eq!(
            terminal.cell(block.right() - 1, block.y).symbol,
            glyphs.top_right.to_string(),
            "{caps:?} the block draws its top right corner"
        );
        assert_eq!(
            terminal.cell(block.x, block.bottom() - 1).symbol,
            glyphs.bottom_left.to_string(),
            "{caps:?} the block draws its bottom left corner"
        );
        assert_eq!(
            terminal.cell(block.right() - 1, block.bottom() - 1).symbol,
            glyphs.bottom_right.to_string(),
            "{caps:?} the block draws its bottom right corner"
        );
        let ok_line = changes_line_text(&terminal, layout.ok_button.y);
        assert!(
            ok_line.contains(PANEL_ERROR_OK_LABEL),
            "{caps:?} the block ends with the ok button, row {ok_line:?}"
        );
        if caps == CapsProfile::Basic {
            terminal.screenshot("regressions/panel-error-basic");
        }
    }
}

#[test]
fn enter_dismisses_the_panel_error_before_opening_a_diff() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            terminal_holding_a_failed_commit(&format!("input-polish-error-enter-{caps:?}"), caps);
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Log,
            "{caps:?} the failed commit opens no diff"
        );
        let block = panel_error_layout(changes_list_rows(&terminal), reference_screen())
            .expect("the changes list holds the error block")
            .block;
        terminal.send_keys("enter");
        assert_eq!(
            terminal.driver().panel_error(),
            None,
            "{caps:?} enter dismisses the panel error"
        );
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Log,
            "{caps:?} the enter key does not open a diff while it closes the error"
        );
        assert_eq!(
            changes_line_text(&terminal, block.y),
            "",
            "{caps:?} the dismissed block leaves the panel"
        );
        terminal.send_keys("ctrl+enter");
        assert!(
            terminal.driver().panel_error().is_some(),
            "{caps:?} a second failure brings the block back"
        );
    }
}

#[test]
fn clicking_ok_dismisses_the_panel_error() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            terminal_holding_a_failed_commit(&format!("input-polish-error-click-{caps:?}"), caps);
        let ok_button = panel_error_layout(changes_list_rows(&terminal), reference_screen())
            .expect("the changes list holds the error block")
            .ok_button;
        terminal.mouse_click(ok_button.x + 1, ok_button.y);
        assert_eq!(
            terminal.driver().panel_error(),
            None,
            "{caps:?} the ok button dismisses the panel error"
        );
        assert!(
            !terminal.screen_text().contains(COMMIT_FAILED_LABEL),
            "{caps:?} the dismissed block leaves the screen"
        );
    }
}

#[test]
fn double_click_opens_the_diff_tab() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            app_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.send_keys("ctrl+k");
        let changes = changes_pane(&terminal);
        let row = changes_row_of(&terminal, "app.rs");
        terminal.mouse_double_click(changes.x + FILE_NAME_COLUMN, row);
        assert_eq!(
            terminal.driver().selected_file_path(),
            Some(APP_FILE_PATH),
            "{caps:?} the double click selects the clicked file"
        );
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Diff(String::from(APP_FILE_PATH)),
            "{caps:?} the double click opens the diff tab"
        );
        assert!(
            terminal
                .driver()
                .tab_titles()
                .contains(&String::from(APP_FILE_TITLE)),
            "{caps:?} tab titles {:?}",
            terminal.driver().tab_titles()
        );
        assert!(
            terminal.screen_text().contains(APP_FILE_TITLE),
            "{caps:?} the diff tab is drawn"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("input_polish/double-click-diff");
        }
    }
}

#[test]
fn slash_keys_keep_their_tab_dependent_meaning() {
    let mut terminal = app_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let history = terminal.driver().commits().to_vec();
    open_selected_diff(&mut terminal, APP_FILE_PATH);
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    let selected_before = selected_commit_id(&terminal);
    terminal.send_keys("j");
    assert_eq!(
        terminal.driver().diff_scroll(),
        1,
        "j scrolls the diff while a diff tab shows"
    );
    assert_eq!(
        selected_commit_id(&terminal),
        selected_before,
        "j leaves the commit selection alone while a diff tab shows"
    );
    terminal.send_keys("k");
    assert_eq!(terminal.driver().diff_scroll(), 0);
    terminal.send_keys("ctrl+w");
    assert_eq!(terminal.driver().active_tab(), EditorTab::Log);
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    terminal.send_keys("j");
    assert_eq!(
        selected_commit_id(&terminal),
        Some(history[1].id.clone()),
        "j moves the commit selection while the log tab shows"
    );
    assert_eq!(
        terminal.driver().diff_scroll(),
        0,
        "no diff stays open to scroll"
    );
    terminal.send_keys("k");
    assert_eq!(
        selected_commit_id(&terminal),
        Some(history[0].id.clone()),
        "k moves the commit selection back"
    );
}
