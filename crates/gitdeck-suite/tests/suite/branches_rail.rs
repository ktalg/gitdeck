use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, LogView};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{BOTTOM_RULE_HEIGHT, BRANCHES_TITLE, LOG_RAIL_GAP, LOG_RAIL_WIDTH};
use unicode_width::UnicodeWidthChar;

const WIDE_WIDTH: u16 = 220;
const WIDE_HEIGHT: u16 = 70;
const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const BASE_EPOCH_SECONDS: u64 = 1_577_836_800;
const CURRENT_BRANCH: &str = "main";
const LOCAL_BRANCH: &str = "feature/local";
const REMOTE_BRANCH: &str = "origin/main";
const RAIL_LABEL: &str = "Branches";
const CURRENT_BRANCH_ROW: &str = "● main";
const LOCAL_BRANCH_ROW: &str = "  feature/local";
const REMOTE_BRANCH_ROW: &str = "  origin/main";
const CHECKOUT_FAILED_LABEL: &str = "Checkout failed";
const SELECTED_BACKGROUND: Color = Color::Rgb(0x2E, 0x43, 0x6E);
const ACCENT_FOREGROUND: Color = Color::Rgb(0x35, 0x74, 0xF0);

fn scratch_branch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, "src/main.rs", "fn main() {}\n", "first commit", 0);
    run_git(&path, &["branch", LOCAL_BRANCH]);
    run_git(&path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
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

fn commit_file(
    repository_path: &Path,
    relative_path: &str,
    content: &str,
    message: &str,
    offset_seconds: u64,
) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory is created");
    }
    fs::write(&path, content).expect("scratch file is written");
    let git_date = format!("{} +0000", BASE_EPOCH_SECONDS + offset_seconds);
    let output = fixture_git_command(repository_path)
        .args(["add", "--all"])
        .output()
        .expect("git add runs");
    assert!(output.status.success(), "git add failed");
    let output = fixture_git_command(repository_path)
        .args(["commit", "-q", "-m", message])
        .env("GIT_AUTHOR_DATE", &git_date)
        .env("GIT_COMMITTER_DATE", &git_date)
        .output()
        .expect("git commit runs");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn branch_terminal(
    repository_path: &Path,
    width: u16,
    height: u16,
    caps: CapsProfile,
) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, caps, ThemeId::JbDark)
            .expect("scratch repository opens"),
        width,
        height,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal
}

fn pane_row_text(terminal: &TestTerminal<App>, pane: Rect, row: u16) -> String {
    let mut text = String::new();
    let mut column = pane.x;
    while column < pane.right() {
        let symbol = terminal.cell(column, row).symbol;
        let symbol_width = symbol
            .chars()
            .map(|character| character.width().unwrap_or(1))
            .max()
            .unwrap_or(1);
        text.push_str(&symbol);
        column += u16::try_from(symbol_width.max(1)).unwrap_or(1);
    }
    text.trim_end().to_string()
}

fn commits_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal.pane_rect(PaneId::Log).expect("the log pane")
}

fn rail_rect(terminal: &TestTerminal<App>) -> Rect {
    terminal.driver().layout().log_rail
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

#[test]
fn branches_rail_is_rendered_with_stacked_letters() {
    for (width, height) in [
        (WIDE_WIDTH, WIDE_HEIGHT),
        (MEDIUM_WIDTH, MEDIUM_HEIGHT),
        (NARROW_WIDTH, NARROW_HEIGHT),
    ] {
        for caps in EVERY_CAPS_PROFILE {
            let repository_path = scratch_branch_repository("branches-rail");
            let terminal = branch_terminal(&repository_path, width, height, caps);
            let rail = rail_rect(&terminal);
            assert_eq!(rail.width, LOG_RAIL_WIDTH, "{width}x{height} rail width");
            assert_eq!(
                rail.right() + LOG_RAIL_GAP,
                commits_pane(&terminal).x,
                "{width}x{height} the rail keeps one column before the list"
            );
            let label_rows = u16::try_from(RAIL_LABEL.len()).unwrap();
            let rail_rows = rail.height - BOTTOM_RULE_HEIGHT;
            let top = rail.y + (rail_rows - label_rows) / 2;
            let stacked: String = RAIL_LABEL
                .chars()
                .enumerate()
                .map(|(offset, _)| {
                    terminal
                        .cell(rail.x, top + u16::try_from(offset).unwrap())
                        .symbol
                })
                .collect();
            assert_eq!(
                stacked, RAIL_LABEL,
                "{width}x{height} {caps:?} rail letters"
            );
            assert_eq!(
                terminal.cell(rail.x + 1, top).symbol,
                " ",
                "{width}x{height} {caps:?} the rail writes one letter per row"
            );
            terminal.screenshot(&format!("branches_rail/rail-{width}x{height}-{caps:?}"));
            terminal.assert_no_missing_glyphs();
        }
    }
}

#[test]
fn pressing_b_switches_between_log_and_branches() {
    let repository_path = scratch_branch_repository("branches-toggle");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    let commits = commits_pane(&terminal);
    let log_first_row = pane_row_text(&terminal, commits, commits.y);
    assert!(
        !log_first_row.starts_with(BRANCHES_TITLE),
        "the log view starts on a commit row, row was {log_first_row:?}"
    );
    open_branches_view(&mut terminal);
    assert_eq!(
        pane_row_text(&terminal, commits, commits.y),
        BRANCHES_TITLE,
        "the branches view titles its list"
    );
    assert_eq!(
        terminal
            .cell(rail_rect(&terminal).x, rail_rect(&terminal).y + 1)
            .foreground,
        ACCENT_FOREGROUND,
        "the open branches view lights the rail with the accent color"
    );
    terminal.screenshot("branches_rail/view-branches");
    terminal.send_keys("b");
    terminal.tick();
    assert_eq!(terminal.driver().log_view(), LogView::Log);
    assert_eq!(
        pane_row_text(&terminal, commits, commits.y),
        log_first_row,
        "the second b key returns to the log view"
    );
    terminal.screenshot("branches_rail/view-log");
}

#[test]
fn branches_list_marks_the_current_branch() {
    let repository_path = scratch_branch_repository("branches-current");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_branches_view(&mut terminal);
    let commits = commits_pane(&terminal);
    let names: Vec<String> = terminal
        .driver()
        .branches()
        .iter()
        .map(|branch| branch.name.clone())
        .collect();
    assert_eq!(
        names,
        vec![
            String::from(LOCAL_BRANCH),
            String::from(CURRENT_BRANCH),
            String::from(REMOTE_BRANCH)
        ],
        "local branches come before the remote branches"
    );
    let current_row = commits.y
        + 1
        + u16::try_from(
            names
                .iter()
                .position(|name| name == CURRENT_BRANCH)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        pane_row_text(&terminal, commits, current_row),
        CURRENT_BRANCH_ROW
    );
    assert_eq!(
        terminal.cell(commits.x, current_row).foreground,
        ACCENT_FOREGROUND,
        "the current branch marker uses the accent color"
    );
    let local_row = commits.y + 1;
    assert_eq!(
        pane_row_text(&terminal, commits, local_row),
        LOCAL_BRANCH_ROW
    );
    let remote_row = current_row + 1;
    assert_eq!(
        pane_row_text(&terminal, commits, remote_row),
        REMOTE_BRANCH_ROW
    );
    assert_eq!(
        terminal.cell(commits.x, current_row).background,
        SELECTED_BACKGROUND,
        "the selected branch row carries the selection background"
    );
    terminal.screenshot("branches_rail/list-current-branch");
}

#[test]
fn enter_on_a_remote_branch_shows_the_checkout_error() {
    let repository_path = scratch_branch_repository("branches-remote-checkout");
    let mut terminal =
        branch_terminal(&repository_path, WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_branches_view(&mut terminal);
    let remote_row = terminal
        .find_line(REMOTE_BRANCH_ROW)
        .expect("the remote branch is listed");
    terminal.send_keys("down");
    terminal.tick();
    assert_eq!(
        terminal.driver().selected_branch_index(),
        Some(2),
        "the selection moved onto the remote branch"
    );
    let commits = commits_pane(&terminal);
    let selected_row = remote_row;
    assert_eq!(
        terminal.cell(commits.x, selected_row).background,
        SELECTED_BACKGROUND,
        "the remote branch row is selected before the checkout"
    );
    terminal.send_keys("enter");
    terminal.tick();
    assert!(
        terminal.find_line(CHECKOUT_FAILED_LABEL).is_some(),
        "the failed remote checkout reports its own title, screen text was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(
        terminal.driver().log_view(),
        LogView::Branches,
        "a failed checkout keeps the branches view open"
    );
    terminal.screenshot("branches_rail/remote-checkout-failed");
}

#[test]
fn short_terminal_keeps_the_rail_without_overflow() {
    for caps in EVERY_CAPS_PROFILE {
        let repository_path = scratch_branch_repository("branches-short");
        let mut terminal = branch_terminal(&repository_path, NARROW_WIDTH, NARROW_HEIGHT, caps);
        open_branches_view(&mut terminal);
        let commits = commits_pane(&terminal);
        assert_eq!(rail_rect(&terminal).width, LOG_RAIL_WIDTH, "{caps:?} rail");
        assert_eq!(
            rail_rect(&terminal).x,
            terminal.driver().layout().log_area.x,
            "{caps:?} the rail sits at the log column edge"
        );
        assert_eq!(
            commits.right(),
            NARROW_WIDTH,
            "{caps:?} the list ends at the screen edge"
        );
        for line in terminal.screen_lines() {
            let line_width: u16 = line
                .chars()
                .map(|character| u16::try_from(character.width().unwrap_or(1)).unwrap_or(1))
                .sum();
            assert!(
                line_width <= NARROW_WIDTH,
                "{caps:?} a row of the short terminal stays inside the screen, line {line:?}"
            );
        }
        terminal.screenshot(&format!("branches_rail/short-{caps:?}"));
    }
}
