use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_core::{ChangeKind, LogFilter, LogQuery, PaneId};
use gitdeck_git::{CliGit, GitBackend};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    amend_file_notice, amend_untracked_notice, changes_panel_layout, compact_amend_file_notice,
    text_display_width, ChangesPanelLayout, StatusTone, AMEND_COMMIT_BUTTON_LABEL,
    CHANGES_PANEL_CONTENT_TOP, CHECKBOX_COLUMN, COMMIT_BUTTON_LABEL, COMMIT_MESSAGE_BOX_MIN_HEIGHT,
    COMMIT_MESSAGE_HEIGHT_DIVISOR, FILE_NAME_COLUMN, FILE_STATUS_COLUMN,
    FILE_WORKTREE_STATUS_COLUMN, GROUP_ARROW_COLUMN, GROUP_TITLE_COLUMN, NO_CHANGES_LABEL,
};
use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const WIDE_WIDTH: u16 = 220;
const WIDE_HEIGHT: u16 = 70;
const WIDE_PANEL_HEIGHT: u16 = WIDE_HEIGHT - 2;
const AMEND_PREVIEW_WIDTH: u16 = 260;
const AMEND_PREVIEW_HEIGHT: u16 = 60;
const MESSAGE_BOX_DRAG_ROW: u16 = 5;
const MESSAGE_BOX_WIDEN_COLUMNS: u16 = 8;
const LONG_LINE_TAIL_CHARACTERS: usize = 20;
const LONG_MESSAGE_LINE: &str =
    "fix: keep the caret inside the commit message box when the subject runs past the pane";
const CELL_PIXEL_WIDTH: u32 = 16;
const CELL_PIXEL_HEIGHT: u32 = 32;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const PANEL_BACKGROUND: Color = Color::Rgb(0x2B, 0x2D, 0x30);
const TEXT_COLOR: Color = Color::Rgb(0xDF, 0xE1, 0xE5);
const SELECTED_BACKGROUND: Color = Color::Rgb(0x2E, 0x43, 0x6E);
const INACTIVE_SELECTION_BACKGROUND: Color = Color::Rgb(0x39, 0x3B, 0x40);
const MODIFIED_LETTER_COLOR: Color = Color::Rgb(0xC9, 0xA6, 0x3C);
const UNTRACKED_LETTER_COLOR: Color = Color::Rgb(0x86, 0x8A, 0x91);
const CJK_FILE_NAME: &str = "说明.md";
const CJK_SCREEN_MISSING_GLYPHS: [char; 22] = [
    '中', '交', '信', '加', '与', '文', '更', '新', '明', '档', '添', '径', '提', '说', '路', '：',
    '息', '未', '跟', '踪', '件', '增',
];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const SELECT_FILE_ROW_HINT: &str = "Select a file row first";
const SHORT_PANEL_HEIGHT: u16 = 14;

fn changes_terminal(
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
                .expect("fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn changes_left(terminal: &TestTerminal<App>) -> u16 {
    terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible")
        .x
}

fn changes_content_top(terminal: &TestTerminal<App>) -> u16 {
    terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible")
        .y
        + CHANGES_PANEL_CONTENT_TOP
}

fn assert_cells_hold_text(
    terminal: &TestTerminal<App>,
    column: u16,
    row: u16,
    expected: &str,
) -> u16 {
    let mut current_column = column;
    for character in expected.chars() {
        assert_eq!(
            terminal.cell(current_column, row).symbol,
            character.to_string(),
            "cell ({current_column}, {row}) of {expected:?}"
        );
        current_column += u16::try_from(character.width().unwrap_or(1)).unwrap_or(1);
    }
    current_column
}

fn changes_pane_line_text(terminal: &TestTerminal<App>, row: u16) -> String {
    let changes = terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible");
    let mut line = String::new();
    let mut column = changes.x;
    while column < changes.right() {
        let symbol = terminal.cell(column, row).symbol;
        let symbol_width = symbol
            .chars()
            .map(|character| character.width().unwrap_or(1))
            .max()
            .unwrap_or(1);
        line.push_str(&symbol);
        column += u16::try_from(symbol_width.max(1)).unwrap_or(1);
    }
    line
}

fn find_changes_line(terminal: &TestTerminal<App>, needle: &str) -> Option<u16> {
    let screen_row_count = u16::try_from(terminal.screen_lines().len()).unwrap_or(0);
    (0..screen_row_count).find(|row| changes_pane_line_text(terminal, *row).contains(needle))
}

fn expanded_group_arrow(caps: CapsProfile) -> char {
    if caps == CapsProfile::Basic {
        'v'
    } else {
        '▾'
    }
}

fn tracked_change_count(terminal: &TestTerminal<App>) -> usize {
    terminal
        .driver()
        .changes()
        .iter()
        .filter(|entry| entry.kind != ChangeKind::Untracked)
        .count()
}

fn untracked_change_count(terminal: &TestTerminal<App>) -> usize {
    terminal
        .driver()
        .changes()
        .iter()
        .filter(|entry| entry.kind == ChangeKind::Untracked)
        .count()
}

fn select_unversioned_group_header(terminal: &mut TestTerminal<App>) {
    terminal.send_keys("ctrl+k");
    let file_rows = tracked_change_count(terminal);
    for _ in 0..=file_rows {
        terminal.send_keys("down");
    }
}

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, &["init", "-q", "-b", "main"]);
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

fn write_file(repository_path: &Path, relative_path: &str, content: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory is created");
    }
    fs::write(&path, content).expect("scratch file is written");
}

#[test]
fn dirty_fixture_shows_changes_and_unversioned_groups() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal =
            changes_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let left = changes_left(&terminal);
        let top = changes_content_top(&terminal);
        let tracked_count = tracked_change_count(&terminal);
        let untracked_count = untracked_change_count(&terminal);
        assert_eq!(tracked_count, 3, "{caps:?} tracked change count");
        assert_eq!(untracked_count, 1, "{caps:?} untracked change count");
        let title_end = assert_cells_hold_text(
            &terminal,
            left + GROUP_ARROW_COLUMN,
            top,
            &format!("{} [x] Changes", expanded_group_arrow(caps)),
        );
        assert!(title_end > left + GROUP_TITLE_COLUMN);
        assert_cells_hold_text(&terminal, title_end + 1, top, "3 files");
        let untracked_row = top + u16::try_from(tracked_count).expect("row offset") + 1;
        let untracked_title_end = assert_cells_hold_text(
            &terminal,
            left + GROUP_ARROW_COLUMN,
            untracked_row,
            &format!("{} [x] Unversioned Files", expanded_group_arrow(caps)),
        );
        assert_cells_hold_text(&terminal, untracked_title_end + 1, untracked_row, "1 file");
    }
}

#[test]
fn file_rows_indent_two_columns_below_the_group_header() {
    let terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let left = changes_left(&terminal);
    let top = changes_content_top(&terminal);
    let file_row = top + 1;
    assert_cells_hold_text(&terminal, left, file_row, "  [x]  D readme.md  docs");
    assert_eq!(
        terminal.cell(left, file_row).symbol,
        " ",
        "file rows indent two columns"
    );
    assert_eq!(
        terminal.cell(left + CHECKBOX_COLUMN, file_row).symbol,
        "[",
        "the checkbox starts at x+2"
    );
    assert_eq!(
        terminal
            .cell(left + FILE_WORKTREE_STATUS_COLUMN, file_row)
            .symbol,
        "D",
        "the worktree status letter sits at x+7"
    );
    assert_eq!(
        terminal.cell(left + FILE_STATUS_COLUMN, file_row).symbol,
        " ",
        "the index status column stays blank for a worktree-only change"
    );
}

#[test]
fn the_unversioned_group_starts_expanded() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal =
            changes_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let untracked_header_start = assert_cells_hold_text(
            &terminal,
            changes_left(&terminal),
            changes_content_top(&terminal) + 4,
            &format!("{} [x] Unversioned Files", expanded_group_arrow(caps)),
        );
        assert!(untracked_header_start > changes_left(&terminal));
        let notes_row = find_changes_line(&terminal, "notes.txt")
            .expect("the untracked file row is visible without expanding the group");
        assert_eq!(
            notes_row,
            changes_content_top(&terminal) + 5,
            "{caps:?} the untracked file sits under its group header"
        );
        assert_cells_hold_text(
            &terminal,
            changes_left(&terminal),
            notes_row,
            "  [x]  ? notes.txt",
        );
        terminal.screenshot("changes_panel/unversioned-expanded");
    }
}

fn changes_list_rows(terminal: &TestTerminal<App>) -> Vec<String> {
    let list = changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        false,
    )
    .expect("the changes pane has a layout")
    .list;
    (list.y..list.bottom())
        .map(|row| changes_pane_line_text(terminal, row))
        .collect()
}

#[test]
fn an_empty_changes_list_renders_no_stray_glyphs() {
    let terminal = changes_terminal(
        FixtureRepo::Empty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let rows = changes_list_rows(&terminal);
    let placeholder_row = rows
        .iter()
        .position(|row| row.trim_end() == NO_CHANGES_LABEL)
        .expect("the empty list shows the placeholder");
    assert_eq!(placeholder_row, 0, "the placeholder sits on the first row");
    for (offset, row) in rows.iter().enumerate().skip(1) {
        assert_eq!(
            row.trim_end(),
            "",
            "row {offset} of the empty list must stay blank, was {row:?}"
        );
    }
}

#[test]
fn no_stray_glyphs_appear_when_the_list_has_rows() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.tick();
    let list = changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        false,
    )
    .expect("the changes pane has a layout")
    .list;
    let stray_glyphs = ['\u{25cf}', '\u{25c6}', '\u{25cb}'];
    for row in list.y..list.bottom() {
        for column in list.x..list.right() {
            let symbol = terminal.cell(column, row).symbol.clone();
            assert!(
                !stray_glyphs.iter().any(|glyph| symbol == glyph.to_string()),
                "cell ({column}, {row}) holds the stray glyph {symbol:?}\n{}",
                terminal.screen_text()
            );
        }
    }
    let rows = changes_list_rows(&terminal);
    assert!(
        rows[0].starts_with(expanded_group_arrow(CapsProfile::Rich)),
        "the first row is the expanded group header, was {:?}",
        rows[0]
    );
    assert_eq!(
        rows[1].chars().take(9).collect::<String>(),
        "  [x]  D ",
        "the first file row keeps the status columns at x+6 and x+7"
    );
    terminal.screenshot("changes_panel/no-stray-glyphs");
}

#[test]
fn group_header_toggles_all_children() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    terminal.assert_focused(PaneId::Changes);
    let every_path = terminal.driver().checked_file_paths();
    assert_eq!(every_path.len(), 4);
    terminal.send_keys("space");
    assert_eq!(
        terminal.driver().checked_file_paths(),
        vec![String::from("notes.txt")]
    );
    terminal.send_keys("space");
    assert_eq!(terminal.driver().checked_file_paths(), every_path);
    select_unversioned_group_header(&mut terminal);
    terminal.send_keys("right");
    terminal.send_keys("space");
    assert_eq!(terminal.driver().checked_file_paths().len(), 3);
    terminal.send_keys("space");
    assert_eq!(terminal.driver().checked_file_paths(), every_path);
    terminal.send_keys("space,k,k,k,k,space");
    assert_eq!(terminal.driver().checked_file_paths(), Vec::<String>::new());
    terminal.send_keys("space");
    let changes_group_path: Vec<String> = every_path
        .iter()
        .filter(|path| path.as_str() != "notes.txt")
        .cloned()
        .collect();
    assert_eq!(terminal.driver().checked_file_paths(), changes_group_path);
}

#[test]
fn space_toggles_one_file_checkbox() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    let every_path = terminal.driver().checked_file_paths();
    terminal.send_keys("down");
    assert_eq!(
        terminal.driver().selected_file_path(),
        Some("docs/readme.md")
    );
    let left = changes_left(&terminal);
    let top = changes_content_top(&terminal);
    terminal.send_keys("space");
    let remaining_path = terminal.driver().checked_file_paths();
    assert_eq!(remaining_path.len(), every_path.len() - 1);
    assert!(!remaining_path.contains(&String::from("docs/readme.md")));
    assert_eq!(
        terminal.cell(left + CHECKBOX_COLUMN + 1, top).symbol,
        "-",
        "the group header turns partial"
    );
    terminal.send_keys("space");
    assert_eq!(terminal.driver().checked_file_paths(), every_path);
    assert_eq!(terminal.cell(left + CHECKBOX_COLUMN + 1, top).symbol, "x");
}

#[test]
fn selection_moves_with_j_k_and_arrows() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    assert_eq!(terminal.driver().selected_file_path(), None);
    terminal.send_keys("j");
    assert_eq!(
        terminal.driver().selected_file_path(),
        Some("docs/readme.md")
    );
    terminal.send_keys("down");
    assert_eq!(terminal.driver().selected_file_path(), Some("src/app.rs"));
    terminal.send_keys("k");
    assert_eq!(
        terminal.driver().selected_file_path(),
        Some("docs/readme.md")
    );
    terminal.send_keys("up");
    assert_eq!(terminal.driver().selected_file_path(), None);
    terminal.send_keys("j,j,j,j");
    assert_eq!(
        terminal.driver().selected_file_path(),
        None,
        "the unversioned group header is still reachable"
    );
    terminal.send_keys("j");
    assert_eq!(
        terminal.driver().selected_file_path(),
        Some("notes.txt"),
        "the expanded unversioned group hands the selection to its child"
    );
    assert!(
        find_changes_line(&terminal, "notes.txt").is_some(),
        "the untracked file row stays on screen"
    );
}

#[test]
fn selected_row_uses_selection_color_when_focused() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    terminal.assert_focused(PaneId::Changes);
    terminal.assert_cell_style(
        changes_left(&terminal) + 1,
        changes_content_top(&terminal),
        TEXT_COLOR,
        SELECTED_BACKGROUND,
    );
}

#[test]
fn unfocused_pane_uses_inactive_selection_color() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    terminal.assert_cell_style(
        changes_left(&terminal) + 1,
        changes_content_top(&terminal),
        TEXT_COLOR,
        INACTIVE_SELECTION_BACKGROUND,
    );
}

#[test]
fn status_letters_use_documented_colors() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let left = changes_left(&terminal);
    let modified_row = find_changes_line(&terminal, "app.rs").expect("the modified file row");
    assert_eq!(
        terminal
            .cell(left + FILE_WORKTREE_STATUS_COLUMN, modified_row)
            .symbol,
        "M"
    );
    terminal.assert_cell_style(
        left + FILE_WORKTREE_STATUS_COLUMN,
        modified_row,
        MODIFIED_LETTER_COLOR,
        PANEL_BACKGROUND,
    );
    select_unversioned_group_header(&mut terminal);
    terminal.send_keys("right");
    let untracked_row = find_changes_line(&terminal, "notes.txt").expect("the untracked file row");
    assert_eq!(
        terminal
            .cell(left + FILE_WORKTREE_STATUS_COLUMN, untracked_row)
            .symbol,
        "?"
    );
    terminal.assert_cell_style(
        left + FILE_WORKTREE_STATUS_COLUMN,
        untracked_row,
        UNTRACKED_LETTER_COLOR,
        PANEL_BACKGROUND,
    );
}

#[test]
fn cjk_file_name_renders_and_missing_glyphs_are_exact() {
    let terminal = changes_terminal(
        FixtureRepo::CjkPaths,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let left = changes_left(&terminal);
    let row = find_changes_line(&terminal, CJK_FILE_NAME).expect("the cjk file row");
    assert_cells_hold_text(
        &terminal,
        left,
        row,
        &format!("  [x]  M {CJK_FILE_NAME}  文档"),
    );
    assert_eq!(
        terminal.cell(left + FILE_NAME_COLUMN, row).symbol,
        "说",
        "the cjk file name starts at x+9"
    );
    terminal.assert_missing_glyphs(&CJK_SCREEN_MISSING_GLYPHS);
}

#[test]
fn chrome_carries_no_missing_glyphs() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal =
            changes_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.assert_no_missing_glyphs();
    }
}

#[test]
fn status_bar_shows_branch_from_head_info() {
    let repository_path = fixture_repo(FixtureRepo::Dirty);
    let repository = CliGit::open(&repository_path).expect("the fixture repository opens");
    let branch = repository
        .head_info()
        .expect("the fixture head info is readable")
        .branch
        .expect("the fixture has a checked out branch");
    let terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    assert_eq!(terminal.driver().status_left_label(), branch);
    assert_eq!(
        terminal.driver().head_info().branch.as_deref(),
        Some(branch.as_str())
    );
    let status_line = terminal
        .screen_lines()
        .last()
        .cloned()
        .expect("status line");
    assert!(
        status_line.starts_with(&format!(" {branch}")),
        "status line {status_line:?} does not start with {branch:?}"
    );
}

#[test]
fn detached_head_status_shows_the_short_commit_id() {
    let repository_path = fixture_repo(FixtureRepo::DetachedHead);
    let repository = CliGit::open(&repository_path).expect("the fixture repository opens");
    let head_commit = repository
        .log(
            LogQuery {
                max_commits: 1,
                skip: 0,
            },
            &LogFilter::default(),
        )
        .expect("the fixture log is readable")
        .into_iter()
        .next()
        .expect("the detached fixture has a head commit");
    let terminal = changes_terminal(
        FixtureRepo::DetachedHead,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let label = terminal.driver().status_left_label();
    let short_id = label
        .strip_prefix("detached at ")
        .unwrap_or_else(|| panic!("label {label:?} does not start with the detached prefix"));
    assert_eq!(short_id.len(), 7);
    assert!(
        head_commit.id.starts_with(short_id),
        "short id {short_id:?} is not the head commit prefix"
    );
    let status_line = terminal
        .screen_lines()
        .last()
        .cloned()
        .expect("status line");
    assert!(
        status_line.starts_with(&format!(" {label}")),
        "status line {status_line:?}"
    );
}

#[test]
fn refresh_picks_up_a_new_untracked_file() {
    let repository_path = scratch_repository(&format!("refresh-untracked-{}", std::process::id()));
    write_file(&repository_path, "first.txt", "first\n");
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    assert_eq!(terminal.driver().changes().len(), 1);
    assert_eq!(terminal.driver().checked_file_paths().len(), 1);
    write_file(&repository_path, "second.txt", "second\n");
    terminal.send_keys("f5");
    assert_eq!(terminal.driver().panel_error(), None);
    assert_eq!(terminal.driver().changes().len(), 2);
    assert_eq!(
        terminal.driver().checked_file_paths(),
        vec![String::from("first.txt"), String::from("second.txt")]
    );
}

#[test]
fn no_repository_shows_placeholder_status() {
    let mut terminal = TestTerminal::new(
        App::new(CapsProfile::Rich, ThemeId::JbDark),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    let status_line = terminal
        .screen_lines()
        .last()
        .cloned()
        .expect("status line");
    assert!(
        status_line.contains("no repository"),
        "status line {status_line:?}"
    );
    assert_eq!(terminal.find_line("No changes"), Some(3));
    assert_eq!(terminal.driver().repository_path(), None);
}

#[test]
fn clicking_the_checkbox_matches_pressing_space() {
    let mut keyboard = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    keyboard.send_keys("ctrl+k,down,space");
    let mut pointer = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    pointer.send_keys("ctrl+k");
    pointer.mouse_click(
        changes_left(&pointer) + CHECKBOX_COLUMN + 1,
        changes_content_top(&pointer) + 1,
    );
    assert_eq!(
        pointer.driver().checked_file_paths(),
        keyboard.driver().checked_file_paths()
    );
    assert_eq!(
        pointer.driver().selected_file_path(),
        keyboard.driver().selected_file_path()
    );
    assert_eq!(pointer.screen_text(), keyboard.screen_text());
}

#[test]
fn clicking_the_group_arrow_matches_pressing_left() {
    let mut keyboard = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    keyboard.send_keys("ctrl+k,left");
    let mut pointer = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    pointer.send_keys("ctrl+k");
    pointer.mouse_click(
        changes_left(&pointer) + GROUP_ARROW_COLUMN,
        changes_content_top(&pointer),
    );
    assert_eq!(find_changes_line(&keyboard, "app.rs"), None);
    assert_eq!(find_changes_line(&pointer, "app.rs"), None);
    assert_eq!(pointer.screen_text(), keyboard.screen_text());
}

fn changes_panel_boxes(terminal: &TestTerminal<App>) -> ChangesPanelLayout {
    changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        false,
    )
    .expect("the changes pane holds the commit block")
}

fn amended_changes_panel_boxes(terminal: &TestTerminal<App>) -> ChangesPanelLayout {
    changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        terminal.driver().amend(),
    )
    .expect("the changes pane holds the commit block")
}

fn scratch_dirty_changes_terminal(name: &str) -> TestTerminal<App> {
    let source = fixture_repo(FixtureRepo::Dirty);
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&source, &target).expect("the dirty fixture is copied");
    let mut terminal = TestTerminal::new(
        App::with_repository(&target, CapsProfile::Rich, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
}

#[test]
fn remaining_files_stay_checked_after_a_partial_commit() {
    let mut terminal = scratch_dirty_changes_terminal("partial-commit-remaining");
    terminal.send_keys("ctrl+k");
    terminal.send_keys("space");
    assert_eq!(
        terminal.driver().checked_file_paths(),
        vec![String::from("notes.txt")],
        "the unversioned file stays the only checked entry"
    );
    terminal.send_keys("i");
    terminal.send_text("partial commit");
    terminal.send_keys("enter");
    terminal.send_keys("ctrl+enter");
    let remaining_paths: Vec<String> = terminal
        .driver()
        .changes()
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    assert!(
        !remaining_paths.is_empty(),
        "the partial commit leaves files behind"
    );
    assert_eq!(
        terminal.driver().checked_file_paths(),
        remaining_paths,
        "the remaining files start checked again"
    );
    let top = changes_content_top(&terminal);
    let left = changes_left(&terminal);
    assert_eq!(terminal.cell(left + CHECKBOX_COLUMN + 1, top).symbol, "x");
    terminal.screenshot("changes_panel/partial-commit-remaining-checked");
}

#[test]
fn message_box_takes_a_quarter_of_a_tall_panel() {
    let reference = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let wide = changes_terminal(
        FixtureRepo::Dirty,
        WIDE_WIDTH,
        WIDE_HEIGHT,
        CapsProfile::Rich,
    );
    let narrow = changes_terminal(
        FixtureRepo::Dirty,
        NARROW_WIDTH,
        NARROW_HEIGHT,
        CapsProfile::Rich,
    );
    let reference_box = changes_panel_boxes(&reference).message_box;
    let wide_box = changes_panel_boxes(&wide).message_box;
    let narrow_box = changes_panel_boxes(&narrow).message_box;
    assert_eq!(
        wide_box.height,
        WIDE_PANEL_HEIGHT / COMMIT_MESSAGE_HEIGHT_DIVISOR,
        "220x70 gives the message box a quarter of the pane: {} rows",
        wide_box.height
    );
    assert_eq!(wide_box.height, 17);
    assert_eq!(wide_box.height, reference_box.height + 6);
    assert!(
        narrow_box.height < reference_box.height,
        "a shorter panel holds a shorter message box: {} vs {}",
        narrow_box.height,
        reference_box.height
    );
    for (terminal, message_box) in [
        (&reference, reference_box),
        (&wide, wide_box),
        (&narrow, narrow_box),
    ] {
        let changes_width = terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible")
            .width;
        assert_eq!(message_box.width, changes_width);
        assert_eq!(
            changes_panel_boxes(terminal).message_lines.width,
            changes_width - 4,
            "the message text keeps a two column inset per side"
        );
    }
    let mut widened = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let divider_column = widened.driver().layout().changes_log_divider.x;
    widened.mouse_drag(
        (divider_column, MESSAGE_BOX_DRAG_ROW),
        (
            divider_column + MESSAGE_BOX_WIDEN_COLUMNS,
            MESSAGE_BOX_DRAG_ROW,
        ),
    );
    let widened_box = changes_panel_boxes(&widened).message_box;
    assert_eq!(
        widened_box.width,
        reference_box.width + MESSAGE_BOX_WIDEN_COLUMNS,
        "the message box follows a widened changes pane"
    );
    assert_eq!(widened_box.height, reference_box.height);
    wide.screenshot("layout_polish/message-box-220x70");
}

#[test]
fn message_box_keeps_the_buttons_on_a_short_panel() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        NARROW_WIDTH,
        NARROW_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.tick();
    let panel = terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible");
    let layout = changes_panel_boxes(&terminal);
    assert_eq!(
        layout.message_box.height,
        panel.height / COMMIT_MESSAGE_HEIGHT_DIVISOR,
        "80x24 keeps a quarter of the pane above the button row"
    );
    assert_eq!(layout.message_box.height, 5);
    assert_eq!(layout.amend_row.bottom(), layout.message_box.y);
    assert_eq!(layout.button_row.y, layout.message_box.bottom());
    assert_eq!(layout.button_row.bottom() + 1, panel.bottom());
    let amend_row = layout.amend_row;
    let mut amend_text = String::new();
    for column in amend_row.x..amend_row.right() {
        amend_text.push_str(&terminal.cell(column, amend_row.y).symbol);
    }
    assert!(
        amend_text.contains("Amend last commit"),
        "amend row {amend_text:?}"
    );
    let button_row = layout.button_row;
    let mut button_text = String::new();
    for column in button_row.x..button_row.right() {
        button_text.push_str(&terminal.cell(column, button_row.y).symbol);
    }
    assert!(
        button_text.contains("[ Commit ]"),
        "button row {button_text:?}"
    );
    terminal.screenshot("layout_polish/button-row-at-80x24");
}

#[test]
fn message_box_keeps_three_rows_on_a_short_panel() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.resize(REFERENCE_WIDTH, SHORT_PANEL_HEIGHT);
    let layout = changes_panel_boxes(&terminal);
    assert_eq!(layout.message_box.height, COMMIT_MESSAGE_BOX_MIN_HEIGHT);
    assert_eq!(layout.message_lines.height, 1);
    assert_eq!(layout.button_row.height, 1);
    assert!(
        layout.button_row.bottom() < layout.message_box.y + layout.message_box.height + 2,
        "the button row stays below the message box"
    );
    let button_row = layout.button_row;
    let mut text = String::new();
    for column in button_row.x..button_row.right() {
        text.push_str(&terminal.cell(column, button_row.y).symbol);
    }
    assert!(text.contains("[ Commit ]"), "button row {text:?}");
    let amend_row = layout.amend_row;
    let mut amend_text = String::new();
    for column in amend_row.x..amend_row.right() {
        amend_text.push_str(&terminal.cell(column, amend_row.y).symbol);
    }
    assert!(
        amend_text.contains("Amend last commit"),
        "amend row {amend_text:?}"
    );
    terminal.screenshot("layout_polish/short-panel-buttons");
}

#[test]
fn long_message_line_wraps_and_keeps_the_caret_visible() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    terminal.send_keys("i");
    terminal.send_text(LONG_MESSAGE_LINE);
    terminal.tick();
    assert_eq!(terminal.driver().commit_message(), LONG_MESSAGE_LINE);
    let layout = changes_panel_boxes(&terminal);
    let lines = layout.message_lines;
    assert!(
        usize::from(lines.width) < LONG_MESSAGE_LINE.chars().count(),
        "the typed line is wider than the message box"
    );
    let filled_rows: Vec<u16> = (lines.y..lines.bottom())
        .filter(|row| !message_line_text(&terminal, lines, *row).trim().is_empty())
        .collect();
    assert!(
        filled_rows.len() > 1,
        "the over wide line wraps onto several rows, rows were {filled_rows:?}"
    );
    let wrapped_text: String = filled_rows
        .iter()
        .map(|row| {
            message_line_text(&terminal, lines, *row)
                .trim_end()
                .to_string()
        })
        .collect();
    assert_eq!(
        wrapped_text.replace(' ', ""),
        LONG_MESSAGE_LINE.replace(' ', ""),
        "wrapping keeps every typed character"
    );
    let caret = message_caret_column(&terminal, lines).expect("the editing caret is rendered");
    assert_eq!(
        caret.1,
        *filled_rows.last().expect("the message fills a row"),
        "the caret follows the last wrapped row"
    );
    assert!(
        caret.0 >= lines.x && caret.0 < lines.right(),
        "caret column {} stays inside {}..{}",
        caret.0,
        lines.x,
        lines.right()
    );
    let tail: String = LONG_MESSAGE_LINE
        .chars()
        .skip(LONG_MESSAGE_LINE.chars().count() - LONG_LINE_TAIL_CHARACTERS)
        .collect();
    let visible_row = message_line_text(&terminal, lines, caret.1);
    assert!(
        visible_row.contains(&tail),
        "the visible window keeps the caret end of the line: {visible_row:?}"
    );
    terminal.screenshot("layout_polish/long-message-caret");
}

fn message_line_text(terminal: &TestTerminal<App>, lines: Rect, row: u16) -> String {
    let mut text = String::new();
    let mut column = lines.x;
    while column < lines.right() {
        let cell = terminal.cell(column, row);
        column += u16::try_from(cell.symbol.chars().count())
            .unwrap_or(1)
            .max(1);
        text.push_str(&cell.symbol);
    }
    text
}

fn message_caret_column(terminal: &TestTerminal<App>, lines: Rect) -> Option<(u16, u16)> {
    (lines.y..lines.bottom()).find_map(|row| {
        (lines.x..lines.right())
            .find(|column| {
                let cell = terminal.cell(*column, row);
                cell.background == TEXT_COLOR && cell.foreground == PANEL_BACKGROUND
            })
            .map(|column| (column, row))
    })
}

#[test]
fn changes_panel_screenshots_render_without_missing_glyphs() {
    let scenarios: [(u16, u16, CapsProfile, &str); 3] = [
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Rich,
            "changes_panel/dirty-160x48-rich",
        ),
        (
            NARROW_WIDTH,
            NARROW_HEIGHT,
            CapsProfile::Rich,
            "changes_panel/dirty-80x24-rich",
        ),
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Basic,
            "changes_panel/dirty-160x48-basic",
        ),
    ];
    for (width, height, caps, name) in scenarios {
        let terminal = changes_terminal(FixtureRepo::Dirty, width, height, caps);
        let report = terminal.screenshot(name);
        assert_eq!(
            (
                png_dimensions(&report.png_path).0,
                png_dimensions(&report.png_path).1
            ),
            (
                u32::from(width) * CELL_PIXEL_WIDTH,
                u32::from(height) * CELL_PIXEL_HEIGHT
            ),
            "{name} png dimensions"
        );
        assert!(report.missing_glyphs.is_empty(), "{name} missing glyphs");
    }
}

fn png_dimensions(png_path: &Path) -> (u32, u32) {
    let header = fs::read(png_path).expect("screenshot png is readable");
    (
        u32::from_be_bytes([header[16], header[17], header[18], header[19]]),
        u32::from_be_bytes([header[20], header[21], header[22], header[23]]),
    )
}

#[test]
fn group_rows_render_with_the_selection_background() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    assert_eq!(
        terminal.driver().selected_file_path(),
        None,
        "the first row is a group header"
    );
    let left = changes_left(&terminal);
    let top = changes_content_top(&terminal);
    let group_title: String = (left + GROUP_TITLE_COLUMN..left + GROUP_TITLE_COLUMN + 7)
        .map(|column| terminal.cell(column, top).symbol.clone())
        .collect();
    assert!(
        group_title.starts_with("Changes"),
        "the selected row is the group header, row {group_title:?}"
    );
    let pane = terminal
        .pane_rect(PaneId::Changes)
        .expect("the changes pane is visible");
    for column in pane.x + 1..pane.right() - 1 {
        assert_eq!(
            terminal.cell(column, top).background,
            SELECTED_BACKGROUND,
            "column {column} of the selected group row"
        );
    }
    terminal.screenshot("changes_panel/group-row-selected");
}

#[test]
fn file_actions_warn_when_a_group_row_is_selected() {
    let mut terminal = changes_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    assert_eq!(terminal.driver().selected_file_path(), None);
    for key in ["alt+d", "alt+i", "alt+o", "alt+t", "alt+r", "enter"] {
        terminal.send_keys(key);
        assert_eq!(
            terminal.driver().status_message(),
            Some((SELECT_FILE_ROW_HINT, StatusTone::Warning)),
            "{key} reports that the row holds no file, screen was:\n{}",
            terminal.screen_text()
        );
        assert!(
            !terminal.driver().discard_dialog_visible(),
            "{key} opens no confirmation on a group row"
        );
        assert!(
            terminal.driver().status_message().is_some(),
            "{key} keeps the hint on the status bar"
        );
    }
    terminal.screenshot("changes_panel/group-row-action-hint");
}

#[test]
fn the_commit_button_renames_itself_when_amend_is_checked() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            changes_terminal(FixtureRepo::DiffShowcase, WIDE_WIDTH, WIDE_HEIGHT, caps);
        let layout = amended_changes_panel_boxes(&terminal);
        let plain_row = changes_pane_line_text(&terminal, layout.commit_button.y);
        assert!(
            plain_row.contains(COMMIT_BUTTON_LABEL),
            "{caps:?} the plain button row {plain_row:?}"
        );
        assert!(
            !plain_row.contains(AMEND_COMMIT_BUTTON_LABEL),
            "{caps:?} the plain button row {plain_row:?}"
        );
        assert_eq!(
            layout.commit_button.width,
            text_display_width(COMMIT_BUTTON_LABEL),
            "{caps:?} the plain button hugs its label"
        );

        terminal.send_keys("alt+a");
        assert!(terminal.driver().amend(), "{caps:?} alt+a checks amend");
        let layout = amended_changes_panel_boxes(&terminal);
        let amended_button_row = changes_pane_line_text(&terminal, layout.commit_button.y);
        assert!(
            amended_button_row.contains(AMEND_COMMIT_BUTTON_LABEL),
            "{caps:?} the amended button row {amended_button_row:?}"
        );
        assert!(
            !amended_button_row.contains(COMMIT_BUTTON_LABEL),
            "{caps:?} the amended button row {amended_button_row:?}"
        );
        assert_eq!(
            layout.commit_button.width,
            text_display_width(AMEND_COMMIT_BUTTON_LABEL),
            "{caps:?} the amended button hugs its wider label"
        );
        assert_eq!(
            layout.push_button.x,
            layout.commit_button.right() + 2,
            "{caps:?} the push button follows the wider commit button"
        );
        terminal.screenshot(&format!("changes_panel/amend-button-{caps:?}"));

        terminal.send_keys("alt+a");
        assert!(!terminal.driver().amend(), "{caps:?} alt+a unchecks amend");
        assert!(
            changes_pane_line_text(&terminal, layout.commit_button.y).contains(COMMIT_BUTTON_LABEL),
            "{caps:?} the button names itself commit again"
        );
    }
}

#[test]
fn the_amend_preview_counts_files_and_untracked_before_committing() {
    let mut terminal = changes_terminal(
        FixtureRepo::DiffShowcase,
        AMEND_PREVIEW_WIDTH,
        AMEND_PREVIEW_HEIGHT,
        CapsProfile::Rich,
    );
    let layout = amended_changes_panel_boxes(&terminal);
    let plain_row = changes_pane_line_text(&terminal, layout.amend_row.y);
    assert!(
        !plain_row.contains("untracked"),
        "an unchecked amend row promises nothing, got {plain_row:?}"
    );

    terminal.send_keys("alt+a");
    assert!(terminal.driver().amend());
    let checked_file_count = terminal.driver().checked_file_paths().len();
    let untracked_file_count = untracked_change_count(&terminal);
    let layout = amended_changes_panel_boxes(&terminal);
    let amend_row = changes_pane_line_text(&terminal, layout.amend_row.y);
    assert!(
        amend_row.contains(&amend_file_notice(checked_file_count))
            || amend_row.contains(&compact_amend_file_notice(checked_file_count)),
        "the amend row counts {checked_file_count} files, got {amend_row:?}"
    );
    assert!(
        amend_row.contains(&amend_untracked_notice(untracked_file_count)),
        "the amend row counts {untracked_file_count} untracked files, got {amend_row:?}"
    );
    assert!(
        !terminal.screen_text().contains("Amended commit"),
        "the preview lands before any commit runs"
    );
    terminal.screenshot("changes_panel/amend-preview");
}
