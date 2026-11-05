use gitdeck_app::{App, ChangesFocus, EditorTab};
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    changed_files_area, changes_panel_layout, changes_toolbar_entry_rects, editor_tab_hit_at,
    help_overlay_layout, hunk_checkbox_rect, top_bar_help_button_rect, EditorTabHit,
    DIFF_HUNK_CHECKBOX_COLUMN, FILES_HEADER_HEIGHT, LOG_TAB_TITLE,
};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const TWO_HUNK_FILE_PATH: &str = "src/large.txt";
const DETAILS_FIRST_ROW_OFFSET: u16 = 0;
const CHANGES_ROW_SCAN_LIMIT: usize = 16;
const HELP_OVERLAY_OUTSIDE_COLUMN: u16 = 2;
const HELP_OVERLAY_OUTSIDE_ROW: u16 = 5;
const HELP_WHEEL_DELTA: i16 = -8;
const DETAILS_COMMIT_SUBJECT: &str = "diff showcase base";

fn terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::DiffShowcase,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the diff showcase repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn dirty_terminal(caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the dirty repository opens")
        },
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
}

fn open_showcase_diff(terminal: &mut TestTerminal<App>) {
    let result = terminal.driver_mut().open_diff(TWO_HUNK_FILE_PATH);
    assert!(result.is_ok(), "the showcase diff opens: {result:?}");
    terminal.tick();
}

fn diff_area(terminal: &TestTerminal<App>) -> Rect {
    terminal
        .driver()
        .layout()
        .rows_above_bottom_rule(terminal.driver().layout().log_content_rect())
}

fn log_tabs_rect(terminal: &TestTerminal<App>) -> Rect {
    terminal.driver().layout().log_tabs
}

fn cell_row_text(terminal: &TestTerminal<App>, row: u16, columns: Rect) -> String {
    (columns.x..columns.right())
        .map(|column| terminal.cell(column, row).symbol.clone())
        .collect()
}

#[test]
fn clicking_the_log_tab_row_never_panics() {
    let mut terminal = terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, CapsProfile::Rich);
    open_showcase_diff(&mut terminal);
    let tabs = log_tabs_rect(&terminal);
    let caps = terminal.caps();
    for column in 0..tabs.width {
        if terminal.driver().file_diff().is_none() {
            open_showcase_diff(&mut terminal);
        }
        let before_titles = terminal.driver().tab_titles();
        let before_active = terminal.driver().active_tab();
        let before_hunks = terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH);
        let hit = editor_tab_hit_at(
            tabs,
            &before_titles,
            terminal.driver().active_tab_index(),
            caps,
            column,
            tabs.y,
        );
        terminal.mouse_click(column, tabs.y);
        let after_titles = terminal.driver().tab_titles();
        match hit {
            None => {
                assert_eq!(
                    terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH),
                    before_hunks,
                    "column {column} sits outside every tab and checks no hunk"
                );
                assert_eq!(
                    after_titles, before_titles,
                    "column {column} sits outside every tab and changes no tab"
                );
                assert_eq!(
                    terminal.driver().active_tab(),
                    before_active,
                    "column {column} sits outside every tab and keeps the active tab"
                );
            }
            Some(EditorTabHit::Log) => {
                assert_eq!(
                    terminal.driver().active_tab(),
                    EditorTab::Log,
                    "column {column} lands on the log tab"
                );
                assert_eq!(after_titles.len(), before_titles.len());
            }
            Some(EditorTabHit::Diff(_)) => {
                assert_eq!(after_titles.len(), before_titles.len());
                assert!(
                    terminal.driver().file_diff().is_some(),
                    "column {column} lands on the open diff tab"
                );
            }
            Some(EditorTabHit::CloseDiff(_)) => {
                assert_eq!(
                    after_titles.len() + 1,
                    before_titles.len(),
                    "column {column} closes exactly one diff tab"
                );
            }
        }
    }
}

#[test]
fn hunk_checkbox_hits_only_inside_the_row_area() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, caps);
        open_showcase_diff(&mut terminal);
        let area = diff_area(&terminal);
        let before_screen = terminal.screen_text();
        let checkbox_column = area.x + DIFF_HUNK_CHECKBOX_COLUMN;
        terminal.mouse_click(checkbox_column, area.y.saturating_sub(1));
        terminal.mouse_click(checkbox_column, log_tabs_rect(&terminal).y);
        assert_eq!(
            terminal.screen_text(),
            before_screen,
            "{caps:?} a click above the diff area changes nothing"
        );
        assert!(
            terminal
                .driver()
                .checked_hunks_for_path(TWO_HUNK_FILE_PATH)
                .is_empty(),
            "{caps:?} a click above the diff area checks no hunk"
        );
        let hunk_row = terminal
            .find_line("@@")
            .expect("the first hunk header row is visible");
        let checkbox = hunk_checkbox_rect(area, hunk_row);
        terminal.mouse_click(checkbox.x, checkbox.y);
        assert_eq!(
            terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH),
            vec![0],
            "{caps:?} a click inside the checkbox column checks the hunk under the pointer"
        );
    }
}

#[test]
fn the_help_overlay_closes_on_a_second_question_mark_click() {
    let mut terminal = terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    terminal.send_keys("?");
    assert!(terminal.driver().help_overlay_visible());
    let help_button = top_bar_help_button_rect(terminal.driver().layout().top_bar);
    terminal.mouse_click(help_button.x, help_button.y);
    assert!(
        !terminal.driver().help_overlay_visible(),
        "clicking the ? button closes the open help overlay"
    );
}

#[test]
fn clicking_outside_the_help_overlay_closes_it() {
    let mut terminal = terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    terminal.send_keys("?");
    assert!(terminal.driver().help_overlay_visible());
    terminal.mouse_click(HELP_OVERLAY_OUTSIDE_COLUMN, HELP_OVERLAY_OUTSIDE_ROW);
    assert!(
        !terminal.driver().help_overlay_visible(),
        "a click outside the help frame closes the overlay"
    );
}

#[test]
fn the_wheel_scrolls_the_help_overlay() {
    let mut terminal = terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    terminal.send_keys("?");
    let frame = help_overlay_layout(Rect::new(0, 0, REFERENCE_WIDTH, REFERENCE_HEIGHT))
        .expect("the help overlay frame is visible");
    let first_body_row = frame.rows.y;
    let before = cell_row_text(&terminal, first_body_row, frame.rows);
    terminal.mouse_wheel(frame.frame.x + 1, frame.frame.y + 1, HELP_WHEEL_DELTA);
    assert!(terminal.driver().help_overlay_visible());
    let after = cell_row_text(&terminal, first_body_row, frame.rows);
    assert_ne!(
        before, after,
        "the wheel scrolls the help overlay body, both rows read {before:?}"
    );
}

#[test]
fn clicking_the_diff_tab_close_icon_closes_the_tab() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_showcase_diff(&mut terminal);
        let tabs = log_tabs_rect(&terminal);
        let row = cell_row_text(&terminal, tabs.y, tabs);
        let close_column = (tabs.x..tabs.right())
            .filter(|column| *column > tabs.x)
            .rev()
            .find(|column| {
                let symbol = terminal.cell(*column, tabs.y).symbol.clone();
                symbol == "✗" || symbol == "x"
            })
            .unwrap_or_else(|| panic!("{caps:?} the active diff tab shows a close icon: {row:?}"));
        let hit = editor_tab_hit_at(
            tabs,
            &terminal.driver().tab_titles(),
            terminal.driver().active_tab_index(),
            caps,
            close_column,
            tabs.y,
        );
        assert!(
            matches!(hit, Some(EditorTabHit::CloseDiff(_))),
            "{caps:?} column {close_column} is the close icon, the hit is {hit:?} and the row reads {row:?}"
        );
        terminal.mouse_click(close_column, tabs.y);
        assert_eq!(
            terminal.driver().tab_titles(),
            vec![String::from(LOG_TAB_TITLE)],
            "{caps:?} the close icon closes the diff tab"
        );
        assert_eq!(terminal.driver().active_tab(), EditorTab::Log);
    }
}

#[test]
fn clicking_the_log_tab_returns_to_the_commit_list() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        open_showcase_diff(&mut terminal);
        assert!(terminal.driver().file_diff().is_some());
        let tabs = log_tabs_rect(&terminal);
        terminal.mouse_click(tabs.x + 1, tabs.y);
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Log,
            "{caps:?} the log tab becomes active"
        );
        assert!(
            terminal.driver().file_diff().is_none(),
            "{caps:?} the diff content is hidden behind the log tab"
        );
        assert!(
            terminal.find_line(DETAILS_COMMIT_SUBJECT).is_some(),
            "{caps:?} the commit list is visible again"
        );
    }
}

#[test]
fn clicking_the_rollback_icon_opens_the_discard_dialog() {
    let mut terminal = dirty_terminal(CapsProfile::Rich);
    terminal.send_keys("ctrl+k");
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        if terminal.driver().selected_file_path().is_some() {
            break;
        }
        terminal.send_keys("j");
    }
    let selected_path = terminal
        .driver()
        .selected_file_path()
        .expect("a file row is selected")
        .to_string();
    let rollback = changes_toolbar_entry_rects(terminal.driver().layout().changes)[1];
    terminal.mouse_click(rollback.x, rollback.y);
    assert!(
        terminal.driver().discard_dialog_visible(),
        "the rollback icon opens the discard dialog"
    );
    assert_eq!(
        terminal.driver().discard_dialog_path(),
        Some(selected_path.as_str())
    );
}

#[test]
fn clicking_the_message_box_focuses_the_editor() {
    let mut terminal = dirty_terminal(CapsProfile::Rich);
    let panel = changes_panel_layout(terminal.driver().layout().changes, false)
        .expect("the changes panel lays out");
    assert_eq!(terminal.driver().changes_focus(), ChangesFocus::List);
    terminal.mouse_click(panel.message_box.x, panel.message_box.y);
    assert_eq!(
        terminal.driver().changes_focus(),
        ChangesFocus::Message,
        "a click inside the message box enters the message editor"
    );
    terminal.send_text("typed after the click");
    assert_eq!(
        terminal.driver().commit_message(),
        "typed after the click",
        "the message editor receives the typed text"
    );
}

#[test]
fn double_clicking_a_details_file_opens_its_diff() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = dirty_terminal(caps);
        let pane = terminal
            .pane_rect(PaneId::Details)
            .expect("the details pane is visible");
        let files = changed_files_area(
            terminal.driver().layout().rows_above_bottom_rule(pane),
            terminal.driver().selected_commit(),
            terminal.driver().head_info().branch.as_deref(),
            terminal.driver().selected_commit_branches(),
        );
        let file_row = files.y + FILES_HEADER_HEIGHT + DETAILS_FIRST_ROW_OFFSET;
        let path = terminal.driver().selected_commit_files()[0].path.clone();
        terminal.mouse_double_click(files.x + 2, file_row);
        assert_eq!(
            terminal.driver().file_diff().map(|diff| diff.path.as_str()),
            Some(path.as_str()),
            "{caps:?} the double clicked details row opens its commit diff"
        );
    }
}
