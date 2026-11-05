use std::path::Path;

use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    changed_files_area, commit_details_area, commit_details_row_count, file_count_label,
    line_glyphs, BOTTOM_RULE_HEIGHT, COMMIT_DETAIL_SEPARATOR_HEIGHT, FILES_HEADER_HEIGHT,
};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const COMPACT_WIDTH: u16 = 80;
const COMPACT_HEIGHT: u16 = 24;
const SETTLE_TICKS: u32 = gitdeck_app::DETAILS_QUERY_SETTLE_TICKS;
const SELECTION_STEPS: usize = 10;
const SCROLL_STEPS: i16 = -600;
const MAXIMUM_SCROLL_QUERIES: usize = 5;
const WHEEL_COLUMN_INSET: u16 = 10;
const WHEEL_STEP_ROWS: usize = 1;
const CONTAINING_BRANCH_LABEL: &str = "In 1 branch";
const SHORT_PANEL_DETAIL_ROWS: u16 = 2;
const DIVIDER_RESIZE_STEP: u16 = 6;
const BODY_ROW: u16 = 10;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const DIRTY_SUBJECT: &str = "call run twice";
const DIRTY_FILE_COUNT_LABEL: &str = "1 file";

fn details_terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn details_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal
        .pane_rect(PaneId::Details)
        .expect("the details pane is visible")
}

fn details_content(terminal: &TestTerminal<App>) -> Rect {
    let pane = details_pane(terminal);
    Rect::new(
        pane.x,
        pane.y,
        pane.width,
        pane.height.saturating_sub(BOTTOM_RULE_HEIGHT),
    )
}

fn selected_commit_details(terminal: &TestTerminal<App>) -> Rect {
    commit_details_area(
        details_content(terminal),
        terminal.driver().selected_commit(),
        terminal.driver().head_info().branch.as_deref(),
        terminal.driver().selected_commit_branches(),
    )
}

fn row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    let pane = details_pane(terminal);
    (pane.x..pane.right())
        .map(|column| terminal.cell(column, row).symbol.clone())
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[test]
fn details_block_is_pinned_to_the_panel_bottom() {
    let mut terminal = details_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, CapsProfile::Rich);
    let content = details_content(&terminal);
    let block = selected_commit_details(&terminal);
    let commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a commit");
    let expected_rows = commit_details_row_count(
        &commit,
        Some("main"),
        terminal.driver().selected_commit_branches(),
        block.width,
    );
    assert_eq!(
        block.height, expected_rows,
        "the block keeps every detail row"
    );
    assert_eq!(
        block.bottom(),
        content.bottom(),
        "the block hangs from the panel bottom"
    );
    let changed_files = changed_files_area(
        content,
        Some(&commit),
        terminal.driver().head_info().branch.as_deref(),
        terminal.driver().selected_commit_branches(),
    );
    assert_eq!(
        changed_files.y, content.y,
        "the file tree starts at the top"
    );
    assert_eq!(
        changed_files.height,
        content.height - block.height,
        "the file tree takes every row above the block"
    );

    let before_details_width = details_pane(&terminal).width;
    let before_block_height = block.height;
    let divider_column = terminal.driver().layout().commits_details_divider.x;
    terminal.mouse_drag(
        (divider_column, BODY_ROW),
        (divider_column - DIVIDER_RESIZE_STEP, BODY_ROW),
    );
    let widened = selected_commit_details(&terminal);
    assert_eq!(
        details_pane(&terminal).width,
        before_details_width + DIVIDER_RESIZE_STEP,
        "the drag widens the details column"
    );
    assert_eq!(
        widened.height, before_block_height,
        "the block height does not follow the divider drag"
    );
    assert_eq!(
        widened.bottom(),
        details_content(&terminal).bottom(),
        "the block stays on the panel bottom after the drag"
    );
    terminal.screenshot("details/block-pinned");
}

#[test]
fn subject_line_leads_the_details_block() {
    for caps in EVERY_CAPS_PROFILE {
        assert_subject_leads_the_block(caps);
    }
}

fn assert_subject_leads_the_block(caps: CapsProfile) {
    let terminal = details_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
    let block = selected_commit_details(&terminal);
    let subject = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a commit");
    assert_eq!(subject.subject, DIRTY_SUBJECT);
    let subject_row = block.y + COMMIT_DETAIL_SEPARATOR_HEIGHT;
    assert_eq!(row_text(&terminal, subject_row), DIRTY_SUBJECT);
    assert_eq!(
        row_text(&terminal, block.y),
        line_glyphs(caps)
            .horizontal
            .to_string()
            .repeat(usize::from(block.width)),
        "the separator sits directly above the subject"
    );
    assert_eq!(
        row_text(&terminal, subject_row + 1),
        format!("{} {}", &subject.short_id[..7], subject.author_name),
        "the short id and the author follow the subject"
    );
    assert_eq!(
        terminal
            .cell(details_pane(&terminal).x, subject_row)
            .foreground,
        Color::Rgb(0x35, 0x74, 0xF0),
        "the subject uses the accent color"
    );
}

#[test]
fn files_header_shows_the_count() {
    for caps in EVERY_CAPS_PROFILE {
        assert_files_header_shows_the_count(caps);
    }
}

fn assert_files_header_shows_the_count(caps: CapsProfile) {
    let terminal = details_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
    let changed_files = terminal.driver().selected_commit_files();
    assert_eq!(changed_files.len(), 1, "{changed_files:?}");
    let content = details_content(&terminal);
    assert_eq!(
        row_text(&terminal, content.y),
        DIRTY_FILE_COUNT_LABEL,
        "the file tree opens with the changed file count"
    );
    assert_eq!(
        row_text(&terminal, content.y + FILES_HEADER_HEIGHT),
        "M  app.rs  src",
        "the first file row sits under the count header"
    );
    assert_eq!(
        DIRTY_FILE_COUNT_LABEL,
        file_count_label(changed_files.len()),
        "the header reuses the shared count wording"
    );
}

#[test]
fn separator_spans_the_details_width() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = details_terminal(REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let pane = details_pane(&terminal);
        let separator_row = selected_commit_details(&terminal).y;
        let expected = line_glyphs(caps).horizontal;
        for column in pane.x..pane.right() {
            assert_eq!(
                terminal.cell(column, separator_row).symbol,
                expected.to_string(),
                "{caps:?} separator column {column}"
            );
        }
    }
}

#[test]
fn short_panel_drops_rows_in_priority_order() {
    let mut terminal = details_terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, CapsProfile::Rich);
    let content = details_content(&terminal);
    let short_content_height = COMMIT_DETAIL_SEPARATOR_HEIGHT + SHORT_PANEL_DETAIL_ROWS;
    let shrink = content.height - short_content_height;
    let short_height = MEDIUM_HEIGHT - shrink;
    terminal.resize(MEDIUM_WIDTH, short_height);
    let content = details_content(&terminal);
    assert_eq!(content.height, short_content_height, "the squeezed panel");
    let block = selected_commit_details(&terminal);
    assert_eq!(
        block.height, short_content_height,
        "the block clamps to the squeezed panel"
    );
    let subject_row = block.y + COMMIT_DETAIL_SEPARATOR_HEIGHT;
    let commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a commit");
    assert_eq!(row_text(&terminal, subject_row), commit.subject);
    assert_eq!(
        row_text(&terminal, subject_row + 1),
        format!("{} {}", &commit.short_id[..7], commit.author_name),
        "the subject and the short id survive the squeeze"
    );
    assert!(
        !row_text(&terminal, content.y).contains(&commit.author_email),
        "the email row is dropped before the subject and the short id"
    );
    assert_eq!(
        changed_files_area(
            content,
            Some(&commit),
            terminal.driver().head_info().branch.as_deref(),
            terminal.driver().selected_commit_branches(),
        )
        .height,
        0,
        "the file tree keeps no row once the block fills the panel"
    );
    terminal.screenshot("details/short-panel");

    let compact = details_terminal(COMPACT_WIDTH, COMPACT_HEIGHT, CapsProfile::Rich);
    assert_eq!(
        compact.pane_rect(PaneId::Details),
        None,
        "eighty columns hide the details pane"
    );
    assert!(!compact.screen_text().is_empty());
}

fn ten_thousand_terminal() -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::LargeTenThousand,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
                .expect("fixture repository opens")
        },
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal.send_keys("alt+9");
    terminal
}

fn log_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal.pane_rect(PaneId::Log).expect("the log pane")
}

fn selected_commit_id(terminal: &TestTerminal<App>) -> String {
    terminal
        .driver()
        .selected_commit()
        .map(|commit| commit.id.clone())
        .expect("a commit is selected")
}

fn git_name_only_paths(repository_path: &Path, commit_id: &str) -> Vec<String> {
    let output = fixture_git_command(repository_path)
        .args(["show", "--name-only", "--format=", commit_id])
        .output()
        .expect("git show runs");
    assert!(
        output.status.success(),
        "git show {commit_id} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn details_queries_wait_until_the_selection_settles() {
    let mut terminal = ten_thousand_terminal();
    let startup_commit_id = selected_commit_id(&terminal);
    for _ in 0..SELECTION_STEPS {
        terminal.send_keys("j");
    }
    for _ in 0..SELECTION_STEPS {
        assert!(
            terminal.driver().selected_commit_files().is_empty(),
            "a selection change alone queries no commit details"
        );
        terminal.send_keys("k");
    }
    for _ in 0..SELECTION_STEPS {
        terminal.send_keys("j");
    }
    let settled_commit_id = selected_commit_id(&terminal);
    terminal.tick();
    assert!(
        terminal.driver().selected_commit_files().is_empty(),
        "the first tick is still inside the settle window"
    );
    terminal.tick();
    assert!(
        !terminal.driver().selected_commit_files().is_empty(),
        "the settled selection is queried"
    );
    assert_eq!(SETTLE_TICKS as usize, 2, "the settle window is two ticks");
    let mut queried_commit_ids = Vec::new();
    for _ in 0..=SELECTION_STEPS {
        if !terminal.driver().selected_commit_files().is_empty() {
            queried_commit_ids.push(selected_commit_id(&terminal));
        }
        terminal.send_keys("k");
    }
    assert_eq!(
        queried_commit_ids,
        vec![settled_commit_id, startup_commit_id],
        "the settled selection joins the commit queried at startup"
    );
}

#[test]
fn details_are_correct_after_scrolling_stops() {
    let mut terminal = ten_thousand_terminal();
    let commits = log_pane(&terminal);
    terminal.mouse_wheel(
        commits.x + WHEEL_COLUMN_INSET,
        commits.y + 1,
        -i16::try_from(WHEEL_STEP_ROWS).expect("one wheel row"),
    );
    for _ in 0..SETTLE_TICKS {
        terminal.tick();
    }
    let commit_id = selected_commit_id(&terminal);
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the fixture repository path")
        .to_path_buf();
    let selected_paths: Vec<String> = terminal
        .driver()
        .selected_commit_files()
        .iter()
        .map(|file| file.path.clone())
        .collect();
    assert!(!selected_paths.is_empty(), "the scrolled commit has files");
    assert_eq!(
        selected_paths,
        git_name_only_paths(&repository_path, &commit_id),
        "the scrolled commit details match git name-status"
    );
    let screen = terminal.screen_text();
    assert!(
        screen.contains(CONTAINING_BRANCH_LABEL),
        "the settled details show the containing branch count, screen was:\n{screen}"
    );
    assert!(
        screen.contains(file_count_label(selected_paths.len()).as_str()),
        "the settled details show the file count, screen was:\n{screen}"
    );
    for path in &selected_paths {
        let file_name = path.rsplit('/').next().expect("a file name");
        assert!(
            screen.contains(file_name),
            "the settled details show {path:?}, screen was:\n{screen}"
        );
    }
    terminal.screenshot("details/after-scrolling");
}

#[test]
fn rapid_scrolling_does_not_query_per_step() {
    let mut terminal = ten_thousand_terminal();
    let commits = log_pane(&terminal);
    terminal.mouse_wheel(commits.x + WHEEL_COLUMN_INSET, commits.y + 1, SCROLL_STEPS);
    assert!(
        terminal.driver().selected_commit_files().is_empty(),
        "a scroll burst queries nothing until the selection settles"
    );
    for _ in 0..SETTLE_TICKS {
        terminal.tick();
    }
    let mut queried_commit_count = 0;
    for _ in 0..SCROLL_STEPS.unsigned_abs() {
        if !terminal.driver().selected_commit_files().is_empty() {
            queried_commit_count += 1;
        }
        terminal.send_keys("k");
    }
    assert!(queried_commit_count >= 1, "the settled commit is queried");
    assert!(
        queried_commit_count <= MAXIMUM_SCROLL_QUERIES,
        "{SCROLL_STEPS} scroll events queried {queried_commit_count} commits"
    );
}
