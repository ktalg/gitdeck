use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, LogView};
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{branch_filter_block_rect, theme_palette};

const WIDTH: u16 = 160;
const HEIGHT: u16 = 48;
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const CURRENT_BRANCH: &str = "main";
const LOCAL_BRANCH: &str = "feature/local";
const REMOTE_BRANCH: &str = "origin/main";
const BASE_SUBJECT: &str = "base commit";
const LOCAL_SUBJECT: &str = "local branch commit";
const HEAD_ROW_LABEL: &str = "HEAD";
const CURRENT_BRANCH_ROW_LABEL: &str = "* main";
const EMPTY_BRANCH_LABEL: &str = "Branch: HEAD";

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

fn commit_file(repository_path: &Path, relative_path: &str, content: &str, message: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, content).expect("the scratch file is written");
    run_git(repository_path, &["add", "--all"]);
    run_git(repository_path, &["commit", "-q", "-m", message]);
}

fn branch_filter_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, "src/main.rs", "fn main() {}\n", BASE_SUBJECT);
    run_git(&path, &["checkout", "-q", "-b", LOCAL_BRANCH]);
    commit_file(&path, "src/local.rs", "pub fn local() {}\n", LOCAL_SUBJECT);
    run_git(&path, &["checkout", "-q", CURRENT_BRANCH]);
    run_git(&path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    path
}

fn branch_filter_terminal(name: &str) -> TestTerminal<App> {
    let repository_path = branch_filter_repository(name);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        WIDTH,
        HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(WIDTH, HEIGHT);
    terminal.tick();
    terminal
}

fn branch_chip_rect(terminal: &TestTerminal<App>) -> Rect {
    let filters = terminal.driver().layout().log_filters;
    branch_filter_block_rect(filters, terminal.driver().log_filter_branch()).unwrap_or_else(|| {
        panic!(
            "the filter row {:?} shows no branch chip",
            filter_row(terminal)
        )
    })
}

fn filter_row(terminal: &TestTerminal<App>) -> String {
    let filters = terminal.driver().layout().log_filters;
    row_text(terminal, filters.y, filters.x, filters.right())
}

fn row_text(terminal: &TestTerminal<App>, row: u16, left: u16, right: u16) -> String {
    (left..right)
        .map(|column| terminal.cell(column, row).symbol)
        .collect()
}

fn first_dropdown_row(terminal: &TestTerminal<App>) -> u16 {
    let anchor = branch_chip_rect(terminal);
    anchor.bottom().saturating_add(1)
}

fn dropdown_row_label(terminal: &TestTerminal<App>, row: u16) -> String {
    let chip = branch_chip_rect(terminal);
    row_text(terminal, row, chip.x.saturating_add(1), WIDTH)
        .split('│')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn dropdown_rows(terminal: &TestTerminal<App>, row_count: usize) -> Vec<String> {
    (0..row_count)
        .map(|offset| {
            let row =
                first_dropdown_row(terminal).saturating_add(u16::try_from(offset).unwrap_or(0));
            dropdown_row_label(terminal, row)
        })
        .collect()
}

fn click_branch_chip(terminal: &mut TestTerminal<App>) {
    let chip = branch_chip_rect(terminal);
    terminal.mouse_click(chip.x, chip.y);
    terminal.tick();
}

#[test]
fn branch_chip_opens_a_dropdown_with_head_first() {
    let mut terminal = branch_filter_terminal("branch-filter-open");
    assert!(
        filter_row(&terminal).contains(EMPTY_BRANCH_LABEL),
        "the filter row starts on the unfiltered branch chip, row was {:?}",
        filter_row(&terminal)
    );
    click_branch_chip(&mut terminal);
    let anchor = branch_chip_rect(&terminal);
    let first_row = first_dropdown_row(&terminal);
    assert_eq!(
        dropdown_row_label(&terminal, first_row),
        String::from(HEAD_ROW_LABEL),
        "the dropdown opens right below the chip with HEAD first"
    );
    terminal.screenshot("branch_filter_dropdown/step-1-open");
    let accent = theme_palette(ThemeId::JbDark).accent;
    assert_eq!(
        terminal
            .cell(anchor.x.saturating_add(1), first_row)
            .background,
        accent,
        "the HEAD row carries the accent selection background"
    );
    assert_eq!(
        terminal.cell(anchor.x, first_row).background,
        theme_palette(ThemeId::JbDark).panel,
        "the border column keeps the panel background"
    );
}

#[test]
fn dropdown_lists_local_then_remote_branches_with_the_current_one_marked() {
    let mut terminal = branch_filter_terminal("branch-filter-order");
    click_branch_chip(&mut terminal);
    let rows = dropdown_rows(&terminal, 4);
    assert_eq!(
        rows,
        vec![
            String::from(HEAD_ROW_LABEL),
            String::from(LOCAL_BRANCH),
            String::from(CURRENT_BRANCH_ROW_LABEL),
            String::from(REMOTE_BRANCH),
        ],
        "the dropdown lists HEAD, the local branches, then the remote branches"
    );
    terminal.screenshot("branch_filter_dropdown/step-2-rows");
}

#[test]
fn selecting_a_branch_updates_the_chip_and_reloads_the_log() {
    let mut terminal = branch_filter_terminal("branch-filter-select");
    assert_eq!(
        terminal.driver().commits().len(),
        1,
        "the log starts on the main commit"
    );
    click_branch_chip(&mut terminal);
    terminal.send_keys("down");
    terminal.send_keys("enter");
    assert_eq!(
        terminal.driver().log_filter().branch,
        Some(String::from(LOCAL_BRANCH)),
        "the applied branch reaches the backend filter"
    );
    assert_eq!(terminal.driver().log_filter_branch(), Some(LOCAL_BRANCH));
    assert!(
        filter_row(&terminal).contains("Branch: feature/local"),
        "the chip follows the applied branch, row was {:?}",
        filter_row(&terminal)
    );
    assert_eq!(
        terminal.driver().commits().len(),
        2,
        "the log reloads with the branch revision"
    );
    assert_eq!(
        terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.subject.clone()),
        Some(String::from(LOCAL_SUBJECT)),
        "the selection returns to the newest commit of that branch"
    );
    terminal.screenshot("branch_filter_dropdown/step-3-applied");
}

#[test]
fn selecting_head_clears_the_branch_filter() {
    let mut terminal = branch_filter_terminal("branch-filter-head");
    click_branch_chip(&mut terminal);
    terminal.send_keys("down,enter");
    assert_eq!(terminal.driver().log_filter_branch(), Some(LOCAL_BRANCH));
    click_branch_chip(&mut terminal);
    terminal.send_keys("up");
    terminal.send_keys("enter");
    assert_eq!(
        terminal.driver().log_filter_branch(),
        None,
        "HEAD drops the branch filter"
    );
    assert!(
        filter_row(&terminal).contains(EMPTY_BRANCH_LABEL),
        "the chip returns to HEAD, row was {:?}",
        filter_row(&terminal)
    );
    assert_eq!(
        terminal.driver().commits().len(),
        1,
        "the log reloads without a branch revision"
    );
    terminal.screenshot("branch_filter_dropdown/step-4-cleared");
}

#[test]
fn dropdown_swallows_keys_while_open() {
    let mut terminal = branch_filter_terminal("branch-filter-swallow");
    terminal.send_keys("alt+9");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    click_branch_chip(&mut terminal);
    terminal.send_keys("/");
    assert!(
        !terminal.driver().log_filter_editing(),
        "the slash key does not open the text filter while the dropdown is open"
    );
    terminal.send_keys("b");
    assert_eq!(
        terminal.driver().log_view(),
        LogView::Log,
        "the b key does not switch to the branches view while the dropdown is open"
    );
    assert_eq!(
        dropdown_row_label(&terminal, first_dropdown_row(&terminal)),
        String::from(HEAD_ROW_LABEL),
        "the dropdown stays open after the swallowed keys"
    );
    assert!(terminal.driver().branch_filter_dropdown_visible());
    terminal.send_keys("esc");
    assert!(
        !terminal.driver().branch_filter_dropdown_visible(),
        "esc closes the dropdown"
    );
    assert_eq!(
        terminal.driver().log_filter_branch(),
        None,
        "esc leaves the branch filter alone"
    );
}

#[test]
fn branch_and_text_filters_combine() {
    let mut terminal = branch_filter_terminal("branch-filter-combine");
    click_branch_chip(&mut terminal);
    terminal.send_keys("down,enter");
    terminal.send_keys("alt+9");
    terminal.send_keys("/");
    terminal.send_text("base");
    terminal.send_keys("enter");
    let filter = terminal.driver().log_filter();
    assert_eq!(
        filter.branch,
        Some(String::from(LOCAL_BRANCH)),
        "the branch filter stays applied while the text filter loads"
    );
    assert_eq!(filter.text, Some(String::from("base")));
    let subjects: Vec<String> = terminal
        .driver()
        .commits()
        .iter()
        .map(|commit| commit.subject.clone())
        .collect();
    assert_eq!(
        subjects,
        vec![String::from(BASE_SUBJECT)],
        "the log holds only commits on that branch that match the text"
    );
    terminal.screenshot("branch_filter_dropdown/step-5-combined");
}

#[test]
fn clicking_the_leading_column_of_a_dropdown_row_applies_the_filter() {
    let mut terminal = branch_filter_terminal("branch-filter-click-edge");
    click_branch_chip(&mut terminal);
    let chip = branch_chip_rect(&terminal);
    let local_branch_row = first_dropdown_row(&terminal).saturating_add(1);
    terminal.mouse_click(chip.x, local_branch_row);
    terminal.tick();
    assert_eq!(
        terminal.driver().log_filter().branch,
        Some(String::from(LOCAL_BRANCH)),
        "the whole dropdown row applies the branch filter, screen was:\n{}",
        terminal.screen_text()
    );
}

#[test]
fn selecting_a_branch_in_the_dropdown_applies_the_filter() {
    let mut terminal = branch_filter_terminal("branch-filter-click-select");
    click_branch_chip(&mut terminal);
    let chip = branch_chip_rect(&terminal);
    let local_branch_row = first_dropdown_row(&terminal).saturating_add(1);
    terminal.mouse_click(chip.x.saturating_add(2), local_branch_row);
    terminal.tick();
    assert_eq!(
        terminal.driver().log_filter().branch,
        Some(String::from(LOCAL_BRANCH)),
        "clicking a dropdown row applies the branch filter, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        filter_row(&terminal).contains("Branch: feature/local"),
        "the chip follows the clicked branch, row was {:?}",
        filter_row(&terminal)
    );
    assert!(
        !terminal.driver().branch_filter_dropdown_visible(),
        "the click closes the dropdown"
    );
    terminal.screenshot("branch_filter_dropdown/step-6-click-applied");
}
