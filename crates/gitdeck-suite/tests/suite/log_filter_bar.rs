use std::collections::BTreeSet;

use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::theme_palette;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const FILTER_BAR_INSET: u16 = 1;
const EMPTY_FILTER_LABEL: &str = "Text or hash";
const FIRST_PAGE_COMMITS: usize = 500;
const SECOND_PAGE_COMMITS: usize = 1000;

fn log_terminal(fixture: FixtureRepo, width: u16, height: u16) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        fixture,
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
                .expect("fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn focus_log_pane(terminal: &mut TestTerminal<App>) {
    terminal.send_keys("alt+9");
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Log,
        "alt+9 focuses the log pane"
    );
}

fn filter_row_text(terminal: &TestTerminal<App>) -> String {
    let filters = terminal.driver().layout().log_filters;
    let mut row = String::new();
    for column in filters.x..filters.right() {
        row.push_str(&terminal.cell(column, filters.y).symbol);
    }
    row.trim().to_string()
}

fn filter_query_text(terminal: &TestTerminal<App>) -> String {
    let filters = terminal.driver().layout().log_filters;
    let mut row = String::new();
    for column in filters.x + FILTER_BAR_INSET..filters.right() {
        row.push_str(&terminal.cell(column, filters.y).symbol);
    }
    row.trim_end().to_string()
}

fn filter_status_text(terminal: &TestTerminal<App>) -> String {
    terminal
        .driver()
        .log_filter_status()
        .unwrap_or("")
        .to_string()
}

fn caret_column(terminal: &mut TestTerminal<App>) -> u16 {
    let filters = terminal.driver().layout().log_filters;
    let caret_background = theme_palette(ThemeId::JbDark).text;
    (filters.x..filters.right())
        .find(|column| terminal.cell(*column, filters.y).background == caret_background)
        .unwrap_or_else(|| {
            panic!(
                "the filter row {:?} holds no caret",
                filter_row_text(terminal)
            )
        })
}

fn matching_commit_count(terminal: &TestTerminal<App>, query: &str) -> usize {
    let matches: Vec<String> = terminal
        .driver()
        .commits()
        .iter()
        .filter(|commit| commit.subject.to_lowercase().contains(query))
        .map(|commit| commit.id.clone())
        .collect();
    matches.len()
}

fn selected_commit_id(terminal: &TestTerminal<App>) -> String {
    terminal
        .driver()
        .selected_commit()
        .map(|commit| commit.id.clone())
        .unwrap_or_default()
}

#[test]
fn typing_in_the_filter_row_shows_the_query_and_a_caret() {
    let mut terminal = log_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    focus_log_pane(&mut terminal);
    assert!(
        filter_row_text(&terminal).starts_with(EMPTY_FILTER_LABEL),
        "the idle filter row shows the placeholder, got {:?}",
        filter_row_text(&terminal)
    );
    terminal.send_keys("/");
    terminal.send_text("run");
    let filter_bar = terminal.driver().layout().log_filters;
    let caret = caret_column(&mut terminal);
    assert_eq!(
        filter_query_text(&terminal),
        String::from("run"),
        "the filter row shows the typed query"
    );
    assert_eq!(
        caret,
        filter_bar.x + FILTER_BAR_INSET + 3,
        "the caret sits right after the query"
    );
    assert!(
        !filter_row_text(&terminal).contains(EMPTY_FILTER_LABEL),
        "the typed query replaces the placeholder, got {:?}",
        filter_row_text(&terminal)
    );
    terminal.screenshot("log_filter_bar/step-1-typing");
    assert!(terminal.driver().log_filter_editing());
    assert_eq!(terminal.driver().log_filter_text(), None);
}

#[test]
fn enter_applies_the_text_filter_and_resets_the_selection() {
    let mut terminal = log_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    focus_log_pane(&mut terminal);
    let unfiltered_commits = terminal.driver().commits().to_vec();
    assert!(
        unfiltered_commits.len() >= 2,
        "the dirty fixture holds at least two commits"
    );
    terminal.send_keys("j");
    let selected_before = selected_commit_id(&terminal);
    assert_eq!(
        selected_before, unfiltered_commits[1].id,
        "the second commit starts selected after one j"
    );
    let expected_matches = matching_commit_count(&terminal, "initial");
    terminal.send_keys("/");
    terminal.send_text("initial");
    terminal.send_keys("enter");
    let theme = theme_palette(ThemeId::JbDark);
    let filters = terminal.driver().layout().log_filters;
    assert_eq!(
        filter_query_text(&terminal),
        String::from("initial"),
        "the applied query stays in the filter row"
    );
    assert_eq!(
        terminal
            .cell(filters.x + FILTER_BAR_INSET, filters.y)
            .foreground,
        theme.accent,
        "the applied query uses the accent color"
    );
    assert_eq!(
        terminal.driver().commits().len(),
        expected_matches,
        "the log holds only the matching commits"
    );
    assert_eq!(
        terminal.driver().log_filter_text(),
        Some("initial"),
        "the applied query is stored"
    );
    assert!(!terminal.driver().log_filter_editing());
    assert_eq!(
        selected_commit_id(&terminal),
        terminal.driver().commits()[0].id,
        "the selection returns to the newest matching commit"
    );
    assert_eq!(
        filter_status_text(&terminal),
        format!("Filtered {expected_matches} commits")
    );
    terminal.screenshot("log_filter_bar/step-2-applied");
}

#[test]
fn esc_clears_the_query_and_restores_the_unfiltered_log() {
    let mut terminal = log_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    focus_log_pane(&mut terminal);
    let unfiltered_commit_ids: Vec<String> = terminal
        .driver()
        .commits()
        .iter()
        .map(|commit| commit.id.clone())
        .collect();
    terminal.send_keys("/");
    terminal.send_text("initial");
    terminal.send_keys("enter");
    assert!(terminal.driver().log_filter_text().is_some());
    terminal.send_keys("/");
    terminal.send_keys("esc");
    assert_eq!(
        terminal.driver().log_filter_text(),
        None,
        "esc drops the applied query"
    );
    assert!(!terminal.driver().log_filter_editing());
    let restored_commit_ids: Vec<String> = terminal
        .driver()
        .commits()
        .iter()
        .map(|commit| commit.id.clone())
        .collect();
    assert_eq!(
        restored_commit_ids, unfiltered_commit_ids,
        "the unfiltered log comes back in its original order"
    );
    assert!(
        filter_row_text(&terminal).starts_with(EMPTY_FILTER_LABEL),
        "the filter row returns to the placeholder, got {:?}",
        filter_row_text(&terminal)
    );
    assert_eq!(filter_status_text(&terminal), String::new());
    terminal.screenshot("log_filter_bar/step-3-cleared");
}

#[test]
fn backspace_edits_the_query() {
    let mut terminal = log_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    focus_log_pane(&mut terminal);
    terminal.send_keys("/");
    terminal.send_text("runs");
    assert_eq!(filter_query_text(&terminal), String::from("runs"));
    terminal.send_keys("backspace");
    assert_eq!(
        filter_query_text(&terminal),
        String::from("run"),
        "backspace drops the last character"
    );
    assert_eq!(caret_column(&mut terminal), {
        let filters = terminal.driver().layout().log_filters;
        filters.x + FILTER_BAR_INSET + 3
    });
    terminal.send_keys("backspace,backspace,backspace");
    assert_eq!(
        filter_query_text(&terminal),
        String::new(),
        "backspace empties the query"
    );
    assert_eq!(
        caret_column(&mut terminal),
        terminal.driver().layout().log_filters.x + FILTER_BAR_INSET
    );
    assert!(terminal.driver().log_filter_editing());
}

#[test]
fn reaching_the_end_loads_the_next_page() {
    let mut terminal = log_terminal(
        FixtureRepo::LargeTenThousand,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
    );
    focus_log_pane(&mut terminal);
    assert_eq!(
        terminal.driver().commits().len(),
        FIRST_PAGE_COMMITS,
        "the first page holds one page of commits"
    );
    assert!(!terminal.driver().log_reached_end());
    let first_page_ids: Vec<String> = terminal
        .driver()
        .commits()
        .iter()
        .map(|commit| commit.id.clone())
        .collect();
    terminal.send_keys("end");
    assert_eq!(
        terminal.driver().commits().len(),
        SECOND_PAGE_COMMITS,
        "the second page appends to the first"
    );
    assert_eq!(
        selected_commit_id(&terminal),
        first_page_ids[FIRST_PAGE_COMMITS - 1],
        "the appended page keeps the selected commit"
    );
    let unique_ids: BTreeSet<String> = terminal
        .driver()
        .commits()
        .iter()
        .map(|commit| commit.id.clone())
        .collect();
    assert_eq!(
        unique_ids.len(),
        SECOND_PAGE_COMMITS,
        "the appended page repeats no commit"
    );
    for (index, commit_id) in first_page_ids.iter().enumerate() {
        assert_eq!(
            terminal.driver().commits()[index].id,
            *commit_id,
            "the appended page keeps the first page in place at row {index}"
        );
    }
    terminal.send_keys("end");
    assert_eq!(
        terminal.driver().commits().len(),
        1500,
        "another page appends when the selection reaches the new end"
    );
    assert_eq!(
        filter_status_text(&terminal),
        String::new(),
        "no text filter means no filter status"
    );
    terminal.screenshot("log_filter_bar/step-4-second-page");
}
