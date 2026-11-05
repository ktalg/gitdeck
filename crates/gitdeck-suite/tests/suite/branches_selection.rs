use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, LogView};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{BRANCHES_TITLE, NEW_BRANCH_DIALOG_TITLE, NEW_BRANCH_INPUT_LABEL};

const WIDE_WIDTH: u16 = 120;
const WIDE_HEIGHT: u16 = 32;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const CURRENT_BRANCH: &str = "main";
const LOCAL_BRANCH: &str = "feature/local";
const CREATED_BRANCH: &str = "feature/created";
const BASE_SUBJECT: &str = "first commit";
const LOCAL_BRANCH_SUBJECT: &str = "work on the local branch";
const CURRENT_BRANCH_CHIP: &str = " main";
const SELECTED_BACKGROUND: Color = Color::Rgb(0x2E, 0x43, 0x6E);
const FAST_FORWARD_RECEIPT_PREFIX: &str = "Fast-forward to ";
const CREATED_RECEIPT_PREFIX: &str = "Created and checked out ";

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

fn git_stdout(repository_path: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
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

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, "src/main.rs", "fn main() {}\n", BASE_SUBJECT);
    run_git(&path, &["checkout", "-q", "-b", LOCAL_BRANCH]);
    commit_file(
        &path,
        "src/local.rs",
        "fn local() {}\n",
        LOCAL_BRANCH_SUBJECT,
    );
    run_git(&path, &["checkout", "-q", CURRENT_BRANCH]);
    path
}

fn branch_terminal(
    repository_path: &Path,
    width: u16,
    height: u16,
    caps: CapsProfile,
) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, caps, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        width,
        height,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal
}

fn open_branches_view(terminal: &mut TestTerminal<App>) {
    terminal.send_keys("b");
    terminal.tick();
    assert_eq!(
        terminal.driver().log_view(),
        LogView::Branches,
        "the b key opens the branches view"
    );
}

fn branch_index(terminal: &TestTerminal<App>, name: &str) -> usize {
    terminal
        .driver()
        .branches()
        .iter()
        .position(|branch| branch.name == name)
        .unwrap_or_else(|| panic!("the branch list holds {name}"))
}

fn branches_list_row(terminal: &TestTerminal<App>, name: &str) -> u16 {
    let layout = terminal.driver().layout();
    let list = gitdeck_tui::branch_list_rows_area(layout.rows_above_bottom_rule(layout.commits));
    let index = branch_index(terminal, name);
    list.y + u16::try_from(index).expect("the branch index fits the screen")
}

fn selected_row_background(terminal: &TestTerminal<App>, name: &str) -> Color {
    let layout = terminal.driver().layout();
    let list = gitdeck_tui::branch_list_rows_area(layout.rows_above_bottom_rule(layout.commits));
    let row = branches_list_row(terminal, name);
    terminal.cell(list.x + 1, row).background
}

#[test]
fn branches_view_starts_on_the_current_branch() {
    for caps in EVERY_CAPS_PROFILE {
        let repository_path = scratch_repository("branches-selection-start");
        let mut terminal = branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, caps);
        open_branches_view(&mut terminal);
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(branch_index(&terminal, CURRENT_BRANCH)),
            "{caps:?} the branches view starts on the current branch"
        );
        assert_eq!(
            selected_row_background(&terminal, CURRENT_BRANCH),
            SELECTED_BACKGROUND,
            "{caps:?} the current branch row carries the selection background"
        );
        assert_ne!(
            selected_row_background(&terminal, LOCAL_BRANCH),
            SELECTED_BACKGROUND,
            "{caps:?} only one row is selected"
        );
        terminal.screenshot(&format!("branches_selection/start-{caps:?}"));
        terminal.assert_no_missing_glyphs();
    }
}

#[test]
fn arrow_keys_move_the_branch_selection() {
    for caps in EVERY_CAPS_PROFILE {
        let repository_path = scratch_repository("branches-selection-keys");
        let mut terminal = branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, caps);
        open_branches_view(&mut terminal);
        let current_index = branch_index(&terminal, CURRENT_BRANCH);
        let local_index = branch_index(&terminal, LOCAL_BRANCH);
        terminal.send_keys("down");
        terminal.tick();
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(current_index),
            "{caps:?} the down key stops on the last branch"
        );
        terminal.send_keys("up");
        terminal.tick();
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(local_index),
            "{caps:?} the up key moves the selection"
        );
        assert_eq!(
            selected_row_background(&terminal, LOCAL_BRANCH),
            SELECTED_BACKGROUND,
            "{caps:?} the moved selection repaints the row background"
        );
        terminal.send_keys("j");
        terminal.tick();
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(current_index),
            "{caps:?} j moves the selection down"
        );
        terminal.send_keys("k");
        terminal.tick();
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(local_index),
            "{caps:?} k moves the selection up"
        );
        terminal.send_keys("pageup");
        terminal.tick();
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(local_index),
            "{caps:?} page up stops on the first branch"
        );
        terminal.send_keys("pagedown");
        terminal.tick();
        assert_eq!(
            terminal.driver().selected_branch_index(),
            Some(current_index),
            "{caps:?} page down stops on the last branch"
        );
        terminal.screenshot(&format!("branches_selection/keys-{caps:?}"));
    }
}

#[test]
fn clicking_a_branch_row_selects_it() {
    let repository_path = scratch_repository("branches-selection-click");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_branches_view(&mut terminal);
    let layout = terminal.driver().layout();
    let list = gitdeck_tui::branch_list_rows_area(layout.rows_above_bottom_rule(layout.commits));
    let row = branches_list_row(&terminal, CURRENT_BRANCH);
    terminal.mouse_click(list.x + 1, row);
    terminal.tick();
    assert_eq!(
        terminal.driver().selected_branch_index(),
        Some(branch_index(&terminal, CURRENT_BRANCH)),
        "the clicked row is the selected branch"
    );
    assert_eq!(
        selected_row_background(&terminal, CURRENT_BRANCH),
        SELECTED_BACKGROUND,
        "the clicked row carries the selection background"
    );
    terminal.screenshot("branches_selection/click");
}

#[test]
fn alt_m_merges_the_selected_branch() {
    let repository_path = scratch_repository("branches-selection-merge");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_branches_view(&mut terminal);
    terminal.send_keys("up");
    terminal.tick();
    assert_eq!(
        terminal.driver().selected_branch_index(),
        Some(branch_index(&terminal, LOCAL_BRANCH)),
        "the selection sits on the local branch"
    );
    terminal.send_keys("alt+m");
    terminal.tick();
    assert!(
        terminal
            .screen_text()
            .contains(&format!("{FAST_FORWARD_RECEIPT_PREFIX}{LOCAL_BRANCH}")),
        "the merge names the selected branch, screen text was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        git_stdout(&repository_path, &["log", "-1", "--format=%s"]),
        LOCAL_BRANCH_SUBJECT,
        "the merge moved the head onto the selected branch commit"
    );
    assert_eq!(
        git_stdout(&repository_path, &["branch", "--show-current"]),
        CURRENT_BRANCH,
        "the merge stays on the current branch"
    );
    terminal.screenshot("branches_selection/merge-selected");
}

#[test]
fn alt_n_opens_the_new_branch_input() {
    for (width, height) in [(WIDE_WIDTH, WIDE_HEIGHT), (NARROW_WIDTH, NARROW_HEIGHT)] {
        let repository_path = scratch_repository("branches-selection-new-input");
        let mut terminal = branch_terminal(&repository_path, width, height, CapsProfile::Rich);
        open_branches_view(&mut terminal);
        terminal.send_keys("alt+n");
        terminal.tick();
        assert_eq!(
            terminal.driver().new_branch_dialog_name(),
            Some(""),
            "{width}x{height} the dialog opens with an empty name"
        );
        let screen = terminal.screen_text();
        assert!(
            screen.contains(NEW_BRANCH_DIALOG_TITLE),
            "{width}x{height} the dialog is titled, screen text was:\n{screen}"
        );
        assert!(
            screen.contains(NEW_BRANCH_INPUT_LABEL),
            "{width}x{height} the dialog shows its input label, screen text was:\n{screen}"
        );
        assert!(
            screen.contains(BRANCHES_TITLE),
            "{width}x{height} the dialog floats over the branches view, screen text was:\n{screen}"
        );
        terminal.screenshot(&format!("branches_selection/new-branch-{width}x{height}"));
    }
}

#[test]
fn typing_a_name_and_pressing_enter_creates_the_branch() {
    let repository_path = scratch_repository("branches-selection-create");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_branches_view(&mut terminal);
    terminal.send_keys("alt+n");
    terminal.tick();
    terminal.send_text("feature/typo");
    terminal.send_keys("backspace");
    terminal.tick();
    assert_eq!(
        terminal.driver().new_branch_dialog_name(),
        Some("feature/typ"),
        "the dialog edits the typed name"
    );
    terminal.send_keys("esc");
    terminal.tick();
    assert_eq!(
        terminal.driver().new_branch_dialog_name(),
        None,
        "the escape key closes the dialog"
    );
    terminal.send_keys("alt+n");
    terminal.tick();
    terminal.send_text(CREATED_BRANCH);
    terminal.tick();
    terminal.screenshot("branches_selection/new-branch-typed");
    terminal.send_keys("enter");
    terminal.tick();
    assert!(
        terminal
            .screen_text()
            .contains(&format!("{CREATED_RECEIPT_PREFIX}{CREATED_BRANCH}")),
        "the status bar reports the new branch, screen text was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().new_branch_dialog_name(),
        None,
        "the dialog closes once the branch exists"
    );
    assert_eq!(
        git_stdout(&repository_path, &["branch", "--show-current"]),
        CREATED_BRANCH,
        "the new branch is checked out in the repository"
    );
    assert_eq!(
        terminal.driver().head_info().branch.as_deref(),
        Some(CREATED_BRANCH),
        "the head info follows the new branch"
    );
    assert_eq!(
        terminal.driver().selected_branch_index(),
        Some(branch_index(&terminal, CREATED_BRANCH)),
        "the branch list selects the new branch"
    );
    terminal.screenshot("branches_selection/created");
}

#[test]
fn clicking_the_rail_toggles_between_log_and_branches() {
    let repository_path = scratch_repository("branches-selection-rail");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    let rail = terminal.driver().layout().log_rail;
    assert_eq!(terminal.driver().log_view(), LogView::Log);
    terminal.mouse_click(rail.x, rail.y + 1);
    terminal.tick();
    assert_eq!(
        terminal.driver().log_view(),
        LogView::Branches,
        "clicking the rail opens the branches view"
    );
    terminal.screenshot("branches_selection/rail-branches");
    terminal.mouse_click(rail.x, rail.y + 1);
    terminal.tick();
    assert_eq!(
        terminal.driver().log_view(),
        LogView::Log,
        "clicking the rail again returns to the log view"
    );
    assert!(
        terminal.screen_text().contains(CURRENT_BRANCH_CHIP),
        "the log view keeps the branch chip, screen text was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("branches_selection/rail-log");
}
