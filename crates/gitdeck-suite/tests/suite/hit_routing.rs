use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::{App, ChangesFocus, LogView};
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    branch_list_rows_area, branch_row_rect, hunk_checkbox_rect, text_filter_block_rect,
    top_bar_branch_chip_rect, BRANCH_NAME_COLUMN, DIFF_HUNK_TEXT_COLUMN,
};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const CURRENT_BRANCH: &str = "main";
const LOCAL_BRANCH: &str = "feature/local";
const BASE_SUBJECT: &str = "base commit";
const LOCAL_SUBJECT: &str = "local branch commit";
const CHECKOUT_REFLOG_MARKER: &str = "checkout: moving from";
const CHECKED_OUT_SUFFIX: &str = " checked out";
const CURRENT_BRANCH_CHIP: &str = "* main";
const LOCAL_BRANCH_CHIP: &str = "* feature/local";
const TEXT_FILTER_BLOCK_LABEL: &str = "Text or hash";
const UNCHECKED_HUNK_LABEL: &str = "[ ]";
const CHECKED_HUNK_LABEL: &str = "[x]";
const TWO_HUNK_FILE_PATH: &str = "src/large.txt";
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const SINGLE_LETTER_SHORTCUTS: [&str; 3] = ["p", "b", "f"];

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

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", CURRENT_BRANCH]);
    commit_file(&path, "src/main.rs", "fn main() {}\n", BASE_SUBJECT);
    run_git(&path, &["checkout", "-q", "-b", LOCAL_BRANCH]);
    commit_file(&path, "src/local.rs", "pub fn local() {}\n", LOCAL_SUBJECT);
    run_git(&path, &["checkout", "-q", CURRENT_BRANCH]);
    path
}

fn scratch_terminal(name: &str, width: u16, height: u16) -> TestTerminal<App> {
    let repository_path = scratch_repository(name);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal.tick();
    terminal
}

fn showcase_terminal(caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::DiffShowcase,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the diff showcase repository opens")
        },
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
}

fn row_text(terminal: &TestTerminal<App>, row: u16, left: u16, right: u16) -> String {
    (left..right)
        .map(|column| terminal.cell(column, row).symbol)
        .collect()
}

fn branch_list_area(terminal: &TestTerminal<App>) -> Rect {
    let layout = terminal.driver().layout();
    branch_list_rows_area(layout.rows_above_bottom_rule(layout.commits))
}

fn diff_area(terminal: &TestTerminal<App>) -> Rect {
    let layout = terminal.driver().layout();
    layout.rows_above_bottom_rule(layout.log_content_rect())
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

fn visible_branch_row(terminal: &TestTerminal<App>, branch_name: &str) -> u16 {
    let list = branch_list_area(terminal);
    let row = terminal.find_line(branch_name).unwrap_or_else(|| {
        panic!(
            "the branch list shows no {branch_name}, screen text was:\n{}",
            terminal.screen_text()
        )
    });
    assert!(
        row >= list.y && row < list.bottom(),
        "{branch_name} sits at row {row}, outside the branch list {list:?}"
    );
    row
}

fn branch_row_click_column(terminal: &TestTerminal<App>, row: u16) -> u16 {
    let list = branch_list_area(terminal);
    branch_row_rect(list, row.saturating_sub(list.y)).x + BRANCH_NAME_COLUMN
}

fn checkout_reflog_count(repository_path: &Path) -> usize {
    fs::read_to_string(repository_path.join(".git").join("logs").join("HEAD"))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains(CHECKOUT_REFLOG_MARKER))
        .count()
}

fn select_file(terminal: &mut TestTerminal<App>, path: &str) {
    terminal.send_keys("ctrl+k");
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        terminal.send_keys("k");
    }
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
    terminal.tick();
    assert!(
        terminal.driver().file_diff().is_some(),
        "the working tree diff opens, screen text was:\n{}",
        terminal.screen_text()
    );
}

fn hunk_header_row(terminal: &TestTerminal<App>) -> u16 {
    let area = diff_area(terminal);
    let row = terminal
        .find_line("@@")
        .expect("the hunk header row is visible");
    assert!(
        row >= area.y && row < area.bottom(),
        "the hunk header sits at row {row}, outside the diff pane {area:?}"
    );
    row
}

fn hunk_header_label(terminal: &TestTerminal<App>, row: u16) -> String {
    let area = diff_area(terminal);
    row_text(terminal, row, area.x, area.x + DIFF_HUNK_TEXT_COLUMN)
}

#[test]
fn clicking_a_branch_row_selects_and_checks_it_out() {
    let mut terminal = scratch_terminal("hit-branch-click", MEDIUM_WIDTH, MEDIUM_HEIGHT);
    assert!(
        terminal.screen_text().contains(CURRENT_BRANCH_CHIP),
        "the top bar starts on the current branch, screen text was:\n{}",
        terminal.screen_text()
    );
    open_branches_view(&mut terminal);
    let row = visible_branch_row(&terminal, LOCAL_BRANCH);
    terminal.mouse_click(branch_row_click_column(&terminal, row), row);
    terminal.tick();
    let branches = terminal.driver().branches().to_vec();
    let local_index = branches
        .iter()
        .position(|branch| branch.name == LOCAL_BRANCH)
        .expect("the branch list holds the local branch");
    assert_eq!(
        terminal.driver().selected_branch_index(),
        Some(local_index),
        "the clicked row is the selected branch"
    );
    assert_eq!(
        terminal.driver().head_info().branch.as_deref(),
        Some(LOCAL_BRANCH),
        "the clicked row is checked out"
    );
    assert!(
        terminal
            .driver()
            .status_message()
            .is_some_and(|(text, _)| text.ends_with(CHECKED_OUT_SUFFIX)),
        "the status bar reports the checkout, status was {:?}",
        terminal.driver().status_message()
    );
    assert!(
        terminal.screen_text().contains(LOCAL_BRANCH_CHIP),
        "the top bar chip follows the checkout, screen text was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("hit_routing/branch-row-click");
}

#[test]
fn double_clicking_a_branch_row_checks_it_out_once() {
    let repository_path = scratch_repository("hit-branch-double-click");
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        MEDIUM_WIDTH,
        MEDIUM_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(MEDIUM_WIDTH, MEDIUM_HEIGHT);
    terminal.tick();
    open_branches_view(&mut terminal);
    let row = visible_branch_row(&terminal, LOCAL_BRANCH);
    let checkouts_before = checkout_reflog_count(&repository_path);
    terminal.mouse_double_click(branch_row_click_column(&terminal, row), row);
    terminal.tick();
    assert_eq!(
        terminal.driver().head_info().branch.as_deref(),
        Some(LOCAL_BRANCH),
        "the double clicked row is checked out"
    );
    assert_eq!(
        checkout_reflog_count(&repository_path),
        checkouts_before + 1,
        "a double click runs one checkout, the reflog was:\n{}",
        fs::read_to_string(repository_path.join(".git").join("logs").join("HEAD"))
            .unwrap_or_default()
    );
}

#[test]
fn clicking_the_hunk_checkbox_toggles_it() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(caps);
        open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
        let row = hunk_header_row(&terminal);
        let checkbox = hunk_checkbox_rect(diff_area(&terminal), row);
        assert_eq!(
            hunk_header_label(&terminal, row),
            UNCHECKED_HUNK_LABEL,
            "{caps:?} the hunk starts unchecked"
        );
        terminal.mouse_click(checkbox.x, checkbox.y);
        terminal.tick();
        assert_eq!(
            hunk_header_label(&terminal, row),
            CHECKED_HUNK_LABEL,
            "{caps:?} the checkbox click checks the hunk"
        );
        assert_eq!(
            terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH),
            vec![0],
            "{caps:?} the first hunk is the checked one"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("hit_routing/hunk-checkbox-checked");
        }
        terminal.mouse_click(checkbox.x, checkbox.y);
        terminal.tick();
        assert_eq!(
            hunk_header_label(&terminal, row),
            UNCHECKED_HUNK_LABEL,
            "{caps:?} a second click unchecks the hunk"
        );
        assert!(
            terminal
                .driver()
                .checked_hunks_for_path(TWO_HUNK_FILE_PATH)
                .is_empty(),
            "{caps:?} no hunk stays checked"
        );
    }
}

#[test]
fn single_letter_shortcuts_are_ignored_in_the_diff_pane() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    terminal.send_keys("alt+9");
    terminal.tick();
    terminal.assert_focused(PaneId::Log);
    let tab_titles = terminal.driver().tab_titles();
    for shortcut in SINGLE_LETTER_SHORTCUTS {
        terminal.send_keys(shortcut);
        terminal.tick();
        assert!(
            !terminal.driver().pull_dialog_visible(),
            "{shortcut} must not open the pull dialog in the diff pane"
        );
        assert!(
            !terminal.driver().fetch_dialog_visible(),
            "{shortcut} must not open the fetch dialog in the diff pane"
        );
        assert!(
            !terminal.driver().push_dialog_visible(),
            "{shortcut} must not open the push dialog in the diff pane"
        );
        assert!(
            !terminal.driver().branch_filter_dropdown_visible(),
            "{shortcut} must not open the branch filter dropdown in the diff pane"
        );
        assert_eq!(
            terminal.driver().commit_message(),
            "",
            "{shortcut} must not start a commit message"
        );
        assert_eq!(
            terminal.driver().changes_focus(),
            ChangesFocus::List,
            "{shortcut} must leave the changes panel on its list"
        );
        assert_eq!(
            terminal.driver().log_view(),
            LogView::Log,
            "{shortcut} must not switch the log view"
        );
        assert_eq!(
            terminal.driver().focused_pane(),
            PaneId::Log,
            "{shortcut} must keep the diff pane focused"
        );
        assert_eq!(
            terminal.driver().tab_titles(),
            tab_titles,
            "{shortcut} must keep the open tabs"
        );
        assert_eq!(
            terminal.driver().diff_scroll(),
            0,
            "{shortcut} must not scroll the diff"
        );
    }
    terminal.send_keys("i");
    terminal.tick();
    assert_eq!(
        terminal.driver().changes_focus(),
        ChangesFocus::Message,
        "the i key reaches the commit message from the diff pane"
    );
    terminal.send_keys("esc");
    terminal.tick();
    assert_eq!(
        terminal.driver().changes_focus(),
        ChangesFocus::List,
        "esc leaves the message editor"
    );
}

#[test]
fn clicking_the_branch_chip_opens_the_filter_dropdown() {
    let mut terminal = scratch_terminal("hit-chip-click", MEDIUM_WIDTH, MEDIUM_HEIGHT);
    let chip = top_bar_branch_chip_rect(
        terminal.driver().layout().top_bar,
        terminal.driver().project_name().as_deref(),
        terminal.driver().head_info().branch.as_deref(),
        terminal.driver().head_info().ahead,
        terminal.driver().head_info().behind,
        CapsProfile::Rich,
    )
    .expect("the top bar shows a branch chip");
    assert!(
        !terminal.driver().branch_filter_dropdown_visible(),
        "the dropdown starts closed"
    );
    terminal.mouse_click(chip.x, chip.y);
    terminal.tick();
    assert!(
        terminal.driver().branch_filter_dropdown_visible(),
        "the chip click opens the branch filter dropdown, screen text was:\n{}",
        terminal.screen_text()
    );
    assert!(
        terminal.screen_text().contains(LOCAL_BRANCH),
        "the dropdown lists the branches of the same source, screen text was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("hit_routing/branch-chip-dropdown");
    terminal.send_keys("esc");
    terminal.tick();
    assert!(
        !terminal.driver().branch_filter_dropdown_visible(),
        "esc closes the dropdown"
    );
}

#[test]
fn clicking_the_text_filter_focuses_it_for_typing() {
    let mut terminal = scratch_terminal("hit-text-filter", MEDIUM_WIDTH, MEDIUM_HEIGHT);
    let filters = terminal.driver().layout().log_filters;
    let block = text_filter_block_rect(filters, terminal.driver().log_filter_branch())
        .expect("the filter bar shows the text block");
    assert!(
        !terminal.driver().log_filter_editing(),
        "the text filter starts idle"
    );
    terminal.mouse_click(block.x, block.y);
    terminal.tick();
    assert!(
        terminal.driver().log_filter_editing(),
        "the click starts the filter editor"
    );
    terminal.assert_focused(PaneId::Log);
    terminal.send_text("base");
    terminal.tick();
    let bar = row_text(&terminal, filters.y, filters.x, filters.right());
    assert!(
        bar.contains("base"),
        "the typed filter reaches the bar, row was {bar:?}"
    );
    terminal.screenshot("hit_routing/text-filter-typing");
}

#[test]
fn hit_areas_match_the_rendered_rects() {
    let mut terminal = scratch_terminal("hit-areas", REFERENCE_WIDTH, REFERENCE_HEIGHT);
    let top_bar = terminal.driver().layout().top_bar;
    let chip = top_bar_branch_chip_rect(
        top_bar,
        terminal.driver().project_name().as_deref(),
        terminal.driver().head_info().branch.as_deref(),
        terminal.driver().head_info().ahead,
        terminal.driver().head_info().behind,
        CapsProfile::Rich,
    )
    .expect("the top bar shows a branch chip");
    assert!(
        row_text(&terminal, chip.y, chip.x, chip.right()).starts_with(CURRENT_BRANCH_CHIP),
        "the chip rect covers the rendered chip, row was {:?}",
        row_text(&terminal, chip.y, chip.x, chip.right())
    );
    terminal.mouse_click(chip.x.saturating_sub(1), chip.y);
    terminal.tick();
    assert!(
        !terminal.driver().branch_filter_dropdown_visible(),
        "the cell before the chip rect is not part of the chip"
    );
    terminal.mouse_click(chip.x, chip.y);
    terminal.tick();
    assert!(
        terminal.driver().branch_filter_dropdown_visible(),
        "the first cell of the chip rect opens the dropdown"
    );
    terminal.send_keys("esc");
    terminal.tick();

    let filters = terminal.driver().layout().log_filters;
    let text_block = text_filter_block_rect(filters, terminal.driver().log_filter_branch())
        .expect("the filter bar shows the text block");
    assert_eq!(
        row_text(&terminal, text_block.y, text_block.x, text_block.right()),
        TEXT_FILTER_BLOCK_LABEL,
        "the text block rect covers the rendered label"
    );
    terminal.mouse_click(text_block.right(), text_block.y);
    terminal.tick();
    assert!(
        !terminal.driver().log_filter_editing(),
        "the cell after the text block rect belongs to the next block"
    );
    terminal.mouse_click(text_block.x, text_block.y);
    terminal.tick();
    assert!(
        terminal.driver().log_filter_editing(),
        "the first cell of the text block rect starts the editor"
    );
    terminal.send_keys("esc");
    terminal.tick();

    open_branches_view(&mut terminal);
    let list = branch_list_area(&terminal);
    let row = visible_branch_row(&terminal, LOCAL_BRANCH);
    let row_rect = branch_row_rect(list, row.saturating_sub(list.y));
    assert!(
        row_text(&terminal, row_rect.y, row_rect.x, row_rect.right()).contains(LOCAL_BRANCH),
        "the branch row rect covers the rendered row"
    );
    terminal.mouse_click(row_rect.x + BRANCH_NAME_COLUMN, list.y.saturating_sub(1));
    terminal.tick();
    assert_eq!(
        terminal.driver().head_info().branch.as_deref(),
        Some(CURRENT_BRANCH),
        "the title row above the list rect is not a branch row"
    );
    terminal.mouse_click(row_rect.x + BRANCH_NAME_COLUMN, row_rect.y);
    terminal.tick();
    assert_eq!(
        terminal.driver().head_info().branch.as_deref(),
        Some(LOCAL_BRANCH),
        "the first cell of the branch row rect checks the branch out"
    );

    let mut diff_terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_diff(&mut diff_terminal, TWO_HUNK_FILE_PATH);
    let area = diff_area(&diff_terminal);
    let hunk_row = hunk_header_row(&diff_terminal);
    let checkbox = hunk_checkbox_rect(area, hunk_row);
    assert_eq!(
        row_text(&diff_terminal, hunk_row, checkbox.x, checkbox.right()),
        UNCHECKED_HUNK_LABEL,
        "the checkbox rect covers the rendered marker"
    );
    diff_terminal.mouse_click(checkbox.right(), hunk_row);
    diff_terminal.tick();
    assert_eq!(
        hunk_header_label(&diff_terminal, hunk_row),
        UNCHECKED_HUNK_LABEL,
        "the header text after the checkbox rect is not clickable"
    );
    diff_terminal.mouse_click(checkbox.x, hunk_row);
    diff_terminal.tick();
    assert_eq!(
        hunk_header_label(&diff_terminal, hunk_row),
        CHECKED_HUNK_LABEL,
        "the first cell of the checkbox rect toggles the hunk"
    );
}
