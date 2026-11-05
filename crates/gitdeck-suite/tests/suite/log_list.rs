use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    commit_details_area, format_commit_date, BOTTOM_RULE_HEIGHT, CHANGED_FILE_DIRECTORY_GAP,
    CHANGED_FILE_INDENT, CHANGED_FILE_STATUS_GAP, COMMIT_DETAIL_SEPARATOR_HEIGHT, DATE_AUTHOR_GAP,
    DATE_COLUMN_WIDTH, GRAPH_COLUMN_WIDTH, NO_COMMITS_LABEL, NO_FILES_LABEL, SUBJECT_COLUMN,
};
use unicode_width::UnicodeWidthChar;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const FIXTURE_AUTHOR_NAME: &str = "gitdeck-fixture";
const FIXTURE_AUTHOR_EMAIL: &str = "fixture@gitdeck.invalid";
const BASE_EPOCH_SECONDS: u64 = 1_577_836_800;
const UTC_OFFSET_MINUTES_SHIFT: i32 = 120;
const SELECTED_BACKGROUND: Color = Color::Rgb(0x2E, 0x43, 0x6E);
const LOCAL_BRANCH_COLOR: Color = Color::Rgb(0x57, 0x96, 0x5C);
const REMOTE_BRANCH_COLOR: Color = Color::Rgb(0x35, 0x74, 0xF0);
const DETAILS_WIDTH: u16 = 44;
const REFS_SCREEN_WIDTH: u16 = 200;
const WHEEL_COLUMN_INSET: u16 = 10;
const WHEEL_STEP_ROWS: u16 = 3;
const DETAIL_AUTHOR_LINE: u16 = 1;
const DETAIL_EMAIL_LINE: u16 = 2;
const DETAIL_DATE_LINE: u16 = 3;
const LOG_PAGE_JUMP: usize = 10;

fn graph_node_glyph(caps: CapsProfile) -> &'static str {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => "●",
        CapsProfile::Basic => "*",
    }
}

fn log_terminal(
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

fn open_log_terminal(repository_path: &Path, width: u16, height: u16) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("scratch repository opens"),
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal
}

fn pane_row_cells(terminal: &TestTerminal<App>, pane: Rect, row: u16) -> Vec<(u16, String)> {
    let mut cells = Vec::new();
    let mut column = pane.x;
    while column < pane.right() {
        let symbol = terminal.cell(column, row).symbol;
        let symbol_width = symbol
            .chars()
            .map(|character| character.width().unwrap_or(1))
            .max()
            .unwrap_or(1);
        cells.push((column, symbol));
        column += u16::try_from(symbol_width.max(1)).unwrap_or(1);
    }
    cells
}

fn pane_row_text(terminal: &TestTerminal<App>, pane: Rect, row: u16) -> String {
    pane_row_cells(terminal, pane, row)
        .into_iter()
        .map(|(_, symbol)| symbol)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn commits_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal.pane_rect(PaneId::Log).expect("the commits pane")
}

fn details_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal
        .pane_rect(PaneId::Details)
        .expect("the details pane")
}

fn commit_row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    pane_row_text(terminal, commits_pane(terminal), row)
}

fn details_row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    pane_row_text(terminal, details_pane(terminal), row)
}

fn commit_details_top(terminal: &TestTerminal<App>) -> u16 {
    let pane = details_pane(terminal);
    let content = Rect::new(
        pane.x,
        pane.y,
        pane.width,
        pane.height.saturating_sub(BOTTOM_RULE_HEIGHT),
    );
    let block = commit_details_area(
        content,
        terminal.driver().selected_commit(),
        terminal.driver().head_info().branch.as_deref(),
        terminal.driver().selected_commit_branches(),
    );
    block.y + COMMIT_DETAIL_SEPARATOR_HEIGHT
}

fn detail_line_text(terminal: &TestTerminal<App>, line: u16) -> String {
    details_row_text(terminal, commit_details_top(terminal) + line)
}

fn find_commit_label_column(terminal: &TestTerminal<App>, row: u16, needle: &str) -> Option<u16> {
    let cells = pane_row_cells(terminal, commits_pane(terminal), row);
    let text: String = cells
        .iter()
        .map(|(_, symbol)| symbol.as_str())
        .collect::<String>();
    let byte_index = text.find(needle)?;
    let char_index = text[..byte_index].chars().count();
    cells.get(char_index).map(|(column, _)| *column)
}

fn screen_line_with(terminal: &TestTerminal<App>, pane: Rect, needle: &str) -> Option<u16> {
    (pane.y..pane.bottom()).find(|row| pane_row_text(terminal, pane, *row).contains(needle))
}

fn is_digit_run(text: &str, length: usize) -> bool {
    text.len() == length && text.chars().all(|character| character.is_ascii_digit())
}

fn is_commit_date(text: &str) -> bool {
    let Some((date_part, time_part)) = text.split_once(", ") else {
        return false;
    };
    let date_fields: Vec<&str> = date_part.split('/').collect();
    if date_fields.len() != 3 {
        return false;
    }
    let day_matches = (1..=2).contains(&date_fields[0].len())
        && is_digit_run(date_fields[0], date_fields[0].len());
    let month_matches = (1..=2).contains(&date_fields[1].len())
        && is_digit_run(date_fields[1], date_fields[1].len());
    if !day_matches || !month_matches || !is_digit_run(date_fields[2], 2) {
        return false;
    }
    let day_value = date_fields[0].parse::<u8>().unwrap_or(0);
    let month_value = date_fields[1].parse::<u8>().unwrap_or(0);
    if !(1..=31).contains(&day_value) || !(1..=12).contains(&month_value) {
        return false;
    }
    let Some((clock, meridiem)) = time_part.split_once(' ') else {
        return false;
    };
    if meridiem != "am" && meridiem != "pm" {
        return false;
    }
    let Some((hour, minute)) = clock.split_once(':') else {
        return false;
    };
    let hour_matches = (1..=2).contains(&hour.len()) && is_digit_run(hour, hour.len());
    hour_matches && (1..=12).contains(&hour.parse::<u8>().unwrap_or(0)) && is_digit_run(minute, 2)
}

fn minute_of_day(text: &str) -> i32 {
    let (_, time_part) = text.split_once(", ").expect("the date holds a time");
    let (clock, meridiem) = time_part.split_once(' ').expect("the time holds am or pm");
    let (hour, minute) = clock.split_once(':').expect("the clock holds a colon");
    let hour_value = hour.parse::<i32>().expect("the hour is a number");
    let minute_value = minute.parse::<i32>().expect("the minute is a number");
    let hour_24 = match (hour_value, meridiem) {
        (12, "am") => 0,
        (12, "pm") => 12,
        (hour, "pm") => hour + 12,
        (hour, _) => hour,
    };
    hour_24 * 60 + minute_value
}

fn commit_date_text(terminal: &TestTerminal<App>, row: u16, author_width: u16) -> String {
    let commits = commits_pane(terminal);
    let date_cells = pane_row_cells(terminal, commits, row);
    let date_end = commits
        .right()
        .saturating_sub(author_width + DATE_AUTHOR_GAP);
    let date_start = date_end.saturating_sub(DATE_COLUMN_WIDTH);
    date_cells
        .into_iter()
        .filter(|(column, _)| *column >= date_start && *column < date_end)
        .map(|(_, symbol)| symbol)
        .collect::<String>()
        .trim()
        .to_string()
}

fn head_short_id(repository_path: &Path) -> String {
    let output = fixture_git_command(repository_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse HEAD runs");
    assert!(output.status.success(), "git rev-parse HEAD failed");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .chars()
        .take(7)
        .collect()
}

fn scratch_repository(name: &str) -> PathBuf {
    let path = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    run_git(&path, 0, &["init", "-q", "-b", "main"]);
    path
}

fn run_git(repository_path: &Path, offset_seconds: u64, arguments: &[&str]) {
    let git_date = format!("{} +0000", BASE_EPOCH_SECONDS + offset_seconds);
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .env("GIT_AUTHOR_DATE", &git_date)
        .env("GIT_COMMITTER_DATE", &git_date)
        .env("TZ", "UTC")
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

fn commit_file(repository_path: &Path, relative_path: &str, content: &str, offset_seconds: u64) {
    write_file(repository_path, relative_path, content);
    run_git(repository_path, offset_seconds, &["add", "--all"]);
    run_git(
        repository_path,
        offset_seconds,
        &["commit", "-q", "-m", content],
    );
}

#[test]
fn log_lists_commits_newest_first() {
    let repository_path = fixture_repo(FixtureRepo::Dirty);
    let terminal = log_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let expected_short_id = head_short_id(&repository_path);
    let newest_commit = terminal
        .driver()
        .commits()
        .first()
        .expect("the dirty fixture holds commits");
    assert_eq!(newest_commit.short_id, expected_short_id);
    let commits = commits_pane(&terminal);
    let first_row = commit_row_text(&terminal, commits.y);
    assert!(
        first_row.contains(&newest_commit.subject),
        "the first commit row {first_row:?} does not hold {:?}",
        newest_commit.subject
    );
    let oldest_commit = terminal
        .driver()
        .commits()
        .last()
        .expect("the dirty fixture holds commits");
    assert_ne!(oldest_commit.short_id, expected_short_id);
    assert!(!first_row.contains(&oldest_commit.subject));
}

#[test]
fn subject_starts_at_column_six() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = log_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let commits = commits_pane(&terminal);
        let history = terminal.driver().commits();
        assert!(history.len() >= 2, "{caps:?} commit count");
        let reference_free_row = commits.y + 1;
        let commit = &history[1];
        assert!(commit.refs.is_empty(), "{caps:?} refs {:?}", commit.refs);
        assert_eq!(
            terminal.cell(commits.x, reference_free_row).symbol,
            graph_node_glyph(caps).to_string(),
            "{caps:?} graph node column"
        );
        let first_subject_character = commit
            .subject
            .chars()
            .next()
            .expect("the commit holds a subject");
        assert_eq!(
            terminal
                .cell(commits.x + SUBJECT_COLUMN, reference_free_row)
                .symbol,
            first_subject_character.to_string(),
            "{caps:?} subject column"
        );
    }
}

#[test]
fn date_is_right_aligned_and_formatted() {
    let terminal = log_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let history = terminal.driver().commits();
    for row_offset in 0..u16::try_from(history.len()).expect("commit row count") {
        let row = commits.y + row_offset;
        let commit = &history[usize::from(row_offset)];
        let author_width = u16::try_from(commit.author_name.chars().count()).expect("author width");
        let date_text = commit_date_text(&terminal, row, author_width);
        assert!(
            is_commit_date(&date_text),
            "row {row} date {date_text:?} does not match the documented format"
        );
        let expected = format_commit_date(commit.author_timestamp, 0);
        assert_eq!(date_text, expected, "row {row} date");
        let date_width = u16::try_from(expected.chars().count()).expect("date width");
        assert!(
            date_width <= DATE_COLUMN_WIDTH,
            "the date {expected:?} overflows the {DATE_COLUMN_WIDTH} column field"
        );
        let date_end = commits.right() - author_width - DATE_AUTHOR_GAP;
        assert_eq!(
            terminal.cell(date_end - 1, row).symbol,
            expected
                .chars()
                .last()
                .expect("the date ends with a letter")
                .to_string(),
            "row {row} date field ends {DATE_AUTHOR_GAP} columns before the author"
        );
        let author_text = pane_row_text(&terminal, commits, row);
        assert!(
            author_text.ends_with(&commit.author_name),
            "row {row} {author_text:?} does not end with the author {:?}",
            commit.author_name
        );
    }
}

#[test]
fn utc_offset_shifts_displayed_time() {
    let mut terminal = log_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let row = commits.y;
    let author_width = u16::try_from(
        terminal
            .driver()
            .commits()
            .first()
            .expect("a commit")
            .author_name
            .chars()
            .count(),
    )
    .expect("author width");
    let utc_date = commit_date_text(&terminal, row, author_width);
    terminal
        .driver_mut()
        .set_utc_offset_minutes(UTC_OFFSET_MINUTES_SHIFT);
    terminal.tick();
    let shifted_date = commit_date_text(&terminal, row, author_width);
    assert!(is_commit_date(&shifted_date), "{shifted_date:?}");
    assert_ne!(utc_date, shifted_date);
    let expected_hour = (minute_of_day(&utc_date) + UTC_OFFSET_MINUTES_SHIFT).rem_euclid(24 * 60);
    assert_eq!(
        minute_of_day(&shifted_date),
        expected_hour,
        "the shifted date {shifted_date:?} does not move {utc_date:?} by two hours"
    );
    let timestamp = terminal
        .driver()
        .commits()
        .first()
        .expect("a commit")
        .author_timestamp;
    assert_eq!(
        shifted_date,
        format_commit_date(timestamp, UTC_OFFSET_MINUTES_SHIFT)
    );
}

#[test]
fn refs_prefix_uses_documented_colors() {
    let repository_path = scratch_repository("log-refs");
    commit_file(&repository_path, "tracked.txt", "shared base\n", 0);
    run_git(&repository_path, 0, &["branch", "feature"]);
    run_git(
        &repository_path,
        0,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let mut terminal = open_log_terminal(&repository_path, REFS_SCREEN_WIDTH, REFERENCE_HEIGHT);
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    let row = commits_pane(&terminal).y;
    let row_text = commit_row_text(&terminal, row);
    assert!(
        row_text.contains("HEAD -> main, feature, origin/main"),
        "row {row_text:?} must list the head branch once"
    );
    let local_branch_column =
        find_commit_label_column(&terminal, row, "feature").expect("the feature label column");
    terminal.assert_cell_style(
        local_branch_column,
        row,
        LOCAL_BRANCH_COLOR,
        SELECTED_BACKGROUND,
    );
    let remote_branch_column =
        find_commit_label_column(&terminal, row, "origin/main").expect("the remote label column");
    terminal.assert_cell_style(
        remote_branch_column,
        row,
        REMOTE_BRANCH_COLOR,
        SELECTED_BACKGROUND,
    );
}

#[test]
fn selection_moves_and_updates_details() {
    let mut terminal = log_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    let newest_commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a selected commit");
    assert!(
        detail_line_text(&terminal, DETAIL_AUTHOR_LINE).starts_with(&newest_commit.short_id[..7])
    );
    terminal.send_keys("j");
    let moved_commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a selected commit");
    assert_ne!(moved_commit.id, newest_commit.id);
    let author_line = detail_line_text(&terminal, DETAIL_AUTHOR_LINE);
    assert!(
        author_line.starts_with(&moved_commit.short_id[..7]),
        "details row {author_line:?} does not hold {:?}",
        &moved_commit.short_id[..7]
    );
    assert!(!author_line.starts_with(&newest_commit.short_id[..7]));
    let commits = commits_pane(&terminal);
    let selected_row = commits.y + 1;
    let subject_cell = terminal.cell(commits.x + SUBJECT_COLUMN, selected_row);
    assert_eq!(
        subject_cell.foreground,
        Color::Rgb(0xDF, 0xE1, 0xE5),
        "the selected subject keeps the text color"
    );
    assert_eq!(subject_cell.background, SELECTED_BACKGROUND);
    for column in 0..GRAPH_COLUMN_WIDTH {
        assert_eq!(
            terminal.cell(commits.x + column, selected_row).background,
            SELECTED_BACKGROUND,
            "graph column {column} keeps the selection background"
        );
    }
}

#[test]
fn selection_does_not_move_when_changes_pane_is_focused() {
    let mut terminal = log_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    terminal.assert_focused(PaneId::Changes);
    let selected_commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a selected commit");
    terminal.send_keys("j");
    let selected_file = terminal
        .driver()
        .selected_file_path()
        .map(str::to_string)
        .expect("a selected changes row");
    terminal.send_keys("j");
    assert_ne!(
        terminal.driver().selected_file_path().map(str::to_string),
        Some(selected_file)
    );
    let selected_file = terminal
        .driver()
        .selected_file_path()
        .map(str::to_string)
        .expect("a selected changes row");
    terminal.send_keys("pagedown");
    terminal.send_keys("end");
    assert_eq!(
        terminal.driver().selected_file_path().map(str::to_string),
        Some(selected_file),
        "log keys do not move the changes selection"
    );
    assert_eq!(
        terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.id.clone()),
        Some(selected_commit.id),
        "log keys stay inert while the changes pane holds the focus"
    );
}

#[test]
fn details_pane_shows_author_email_and_date() {
    let terminal = log_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let details = details_pane(&terminal);
    let commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("a selected commit");
    let author_line = detail_line_text(&terminal, DETAIL_AUTHOR_LINE);
    assert_eq!(
        author_line,
        format!("{} {}", &commit.short_id[..7], commit.author_name)
    );
    assert!(author_line.contains(FIXTURE_AUTHOR_NAME));
    assert_eq!(
        detail_line_text(&terminal, DETAIL_EMAIL_LINE),
        String::from(FIXTURE_AUTHOR_EMAIL)
    );
    assert_eq!(
        detail_line_text(&terminal, DETAIL_DATE_LINE),
        format_commit_date(commit.author_timestamp, 0)
    );
    assert!(is_commit_date(&detail_line_text(
        &terminal,
        DETAIL_DATE_LINE
    )));
    assert_eq!(details.width, DETAILS_WIDTH);
}

#[test]
fn changed_files_tree_groups_by_directory() {
    let terminal = log_terminal(
        FixtureRepo::DiffShowcase,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let details = details_pane(&terminal);
    let changed_files = terminal.driver().selected_commit_files();
    assert_eq!(changed_files.len(), 6, "{changed_files:?}");
    let directory_row =
        screen_line_with(&terminal, details, "src").expect("the src directory header row");
    let directory_text = details_row_text(&terminal, directory_row);
    assert!(
        directory_text.contains("src") && directory_text.contains("4 files"),
        "directory header {directory_text:?}"
    );
    let first_file_row = directory_row + 1;
    let indent_width = CHANGED_FILE_INDENT;
    for column in 0..indent_width {
        assert_eq!(
            terminal.cell(details.x + column, first_file_row).symbol,
            " ",
            "indented file column {column}"
        );
    }
    assert_eq!(
        terminal
            .cell(details.x + indent_width, first_file_row)
            .symbol,
        "A",
        "the first indented file row holds the added status letter"
    );
    let status_gap = CHANGED_FILE_STATUS_GAP;
    let file_row_text = details_row_text(&terminal, first_file_row);
    assert!(
        file_row_text.trim_start().starts_with('A'),
        "file row {file_row_text:?}"
    );
    assert!(
        terminal
            .cell(details.x + indent_width + 1 + status_gap, first_file_row)
            .symbol
            .chars()
            .next()
            .expect("the file name starts here")
            .is_ascii_alphabetic(),
        "the file name follows the status letter"
    );
    let directory_gap = CHANGED_FILE_DIRECTORY_GAP;
    assert!(directory_gap >= 1);
    let wide_character_row =
        screen_line_with(&terminal, details, "说明.md").expect("the wide character file row");
    assert!(details_row_text(&terminal, wide_character_row).contains("文档"));
    let ungrouped_single_file_row =
        screen_line_with(&terminal, details, "logo.bin").expect("the assets file row");
    assert!(
        !details_row_text(&terminal, ungrouped_single_file_row).contains("files"),
        "a single file directory carries no group header"
    );
}

#[test]
fn merge_commit_with_no_changed_files_shows_placeholder() {
    let repository_path = scratch_repository("log-empty-merge");
    commit_file(&repository_path, "shared.txt", "base\n", 0);
    run_git(&repository_path, 0, &["switch", "-q", "-c", "side"]);
    commit_file(&repository_path, "shared.txt", "same\n", 60);
    run_git(&repository_path, 0, &["switch", "-q", "main"]);
    commit_file(&repository_path, "shared.txt", "same\n", 120);
    run_git(
        &repository_path,
        0,
        &["merge", "--no-ff", "side", "-m", "merge side"],
    );
    let terminal = open_log_terminal(&repository_path, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    let commit = terminal
        .driver()
        .selected_commit()
        .cloned()
        .expect("the merge commit is selected");
    assert_eq!(commit.parent_ids.len(), 2);
    assert!(terminal.driver().selected_commit_files().is_empty());
    let details = details_pane(&terminal);
    let placeholder_row =
        screen_line_with(&terminal, details, NO_FILES_LABEL).expect("the no files placeholder");
    assert!(details_row_text(&terminal, placeholder_row).starts_with(NO_FILES_LABEL));
}

#[test]
fn wheel_scrolls_the_commit_viewport_without_moving_the_selection() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = log_terminal(
            FixtureRepo::LargeTenThousand,
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            caps,
        );
        terminal.send_keys("alt+9");
        for _ in 0..10 {
            terminal.send_keys("j");
        }
        let commits = commits_pane(&terminal);
        let history = terminal.driver().commits().to_vec();
        let selected_before = terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.id.clone());
        assert_eq!(
            selected_before,
            history.get(10).map(|commit| commit.id.clone()),
            "{caps:?} the tenth commit starts selected"
        );
        let top_row_before = commit_row_text(&terminal, commits.y);
        let fourth_row_before = commit_row_text(&terminal, commits.y + WHEEL_STEP_ROWS);
        let log_column = commits.x + WHEEL_COLUMN_INSET;
        terminal.mouse_wheel(log_column, commits.y + 1, -1);
        let top_row_after = commit_row_text(&terminal, commits.y);
        assert_ne!(
            top_row_before, top_row_after,
            "{caps:?} one wheel event scrolls the viewport"
        );
        assert_eq!(
            top_row_after, fourth_row_before,
            "{caps:?} the viewport scrolled three rows"
        );
        assert_eq!(
            terminal
                .driver()
                .selected_commit()
                .map(|commit| commit.id.clone()),
            selected_before,
            "{caps:?} the wheel leaves the selection alone"
        );
        terminal.mouse_wheel(log_column, commits.y + 1, 1);
        assert_eq!(
            commit_row_text(&terminal, commits.y),
            top_row_before,
            "{caps:?} a wheel event upwards restores the first row"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("log/wheel-viewport");
        }
    }
}

#[test]
fn virtualized_list_renders_only_the_visible_window() {
    let mut terminal = log_terminal(
        FixtureRepo::LargeTenThousand,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    let commits = commits_pane(&terminal);
    let history = terminal.driver().commits().to_vec();
    assert!(
        history.len() >= 200,
        "the loaded window is {}",
        history.len()
    );
    let viewport_rows = usize::from(commits.height - BOTTOM_RULE_HEIGHT);
    let newest_subject = history[0].subject.clone();
    let top_row_text = commit_row_text(&terminal, commits.y);
    assert!(top_row_text.contains(&newest_subject));
    terminal.send_keys("end");
    let oldest_visible_subject = history[history.len() - 1].subject.clone();
    let bottom_row_text = commit_row_text(&terminal, commits.bottom() - 1 - BOTTOM_RULE_HEIGHT);
    assert!(
        bottom_row_text.contains(&oldest_visible_subject),
        "the last viewport row {bottom_row_text:?} does not hold {oldest_visible_subject:?}"
    );
    let scrolled_top_row_text = commit_row_text(&terminal, commits.y);
    assert_ne!(scrolled_top_row_text, top_row_text);
    assert!(!scrolled_top_row_text.contains(&newest_subject));
    let expected_top_subject = history[history.len() - viewport_rows].subject.clone();
    assert!(
        scrolled_top_row_text.contains(&expected_top_subject),
        "the scrolled viewport starts at {scrolled_top_row_text:?} instead of {expected_top_subject:?}"
    );
    for row in commits.y..commits.bottom() - BOTTOM_RULE_HEIGHT {
        assert!(
            !commit_row_text(&terminal, row).is_empty(),
            "row {row} of the viewport is empty"
        );
    }
    let rendered_row_count = u16::try_from(viewport_rows).expect("viewport rows");
    assert_eq!(rendered_row_count, commits.height - BOTTOM_RULE_HEIGHT);
    terminal.send_keys("home");
    assert_eq!(commit_row_text(&terminal, commits.y), top_row_text);
    for _ in 0..LOG_PAGE_JUMP {
        terminal.send_keys("j");
    }
    for _ in 0..LOG_PAGE_JUMP {
        terminal.send_keys("k");
    }
    assert_eq!(commit_row_text(&terminal, commits.y), top_row_text);
}

#[test]
fn empty_repository_shows_no_commits() {
    let repository_path = scratch_repository("log-empty-repository");
    let terminal = open_log_terminal(&repository_path, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    assert!(terminal.driver().commits().is_empty());
    assert_eq!(terminal.driver().selected_commit(), None);
    let commits = commits_pane(&terminal);
    let first_row = pane_row_text(&terminal, commits, commits.y);
    assert!(first_row.starts_with(NO_COMMITS_LABEL), "row {first_row:?}");
    assert_eq!(
        terminal.cell(commits.x, commits.y).foreground,
        Color::Rgb(0x86, 0x8A, 0x91),
        "the placeholder uses the muted color"
    );
}

#[test]
fn commit_list_screenshots_render_at_every_size_and_profile() {
    let scenarios: [(u16, u16, CapsProfile, &str); 3] = [
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Rich,
            "log/list-160x48",
        ),
        (
            NARROW_WIDTH,
            NARROW_HEIGHT,
            CapsProfile::Rich,
            "log/list-80x24",
        ),
        (
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            CapsProfile::Basic,
            "log/list-basic",
        ),
    ];
    for (width, height, caps, name) in scenarios {
        let terminal = log_terminal(FixtureRepo::Dirty, width, height, caps);
        let report = terminal.screenshot(name);
        assert!(report.png_path.exists(), "{name} png");
        assert!(report.text_path.exists(), "{name} text");
        assert!(report.ansi_path.exists(), "{name} ansi");
        let commits = commits_pane(&terminal);
        assert!(commits.width > 0, "{name} commits column");
        assert_no_wide_character_broken_rows(&terminal, commits);
    }
}

fn assert_no_wide_character_broken_rows(terminal: &TestTerminal<App>, pane: Rect) {
    let width = usize::from(pane.width);
    for row in pane.y..pane.bottom() {
        let line = pane_row_text(terminal, pane, row);
        assert!(
            line.chars().count() <= width,
            "row {row} of {line:?} holds more symbols than the {width} columns"
        );
    }
}

#[test]
fn scrolled_list_and_details_screenshots_render() {
    let mut scrolled = log_terminal(
        FixtureRepo::LargeTenThousand,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    scrolled.send_keys("alt+9, end");
    scrolled.screenshot("log/list-scrolled");
    assert!(!commit_row_text(&scrolled, commits_pane(&scrolled).y).is_empty());
    let repository_path = scratch_repository("log-details-only");
    commit_file(&repository_path, "src/app.rs", "first\n", 0);
    commit_file(&repository_path, "src/lib.rs", "second\n", 60);
    commit_file(&repository_path, "docs/readme.md", "third\n", 120);
    run_git(&repository_path, 0, &["tag", "v1"]);
    run_git(
        &repository_path,
        0,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let details_only = open_log_terminal(&repository_path, REFERENCE_WIDTH, REFERENCE_HEIGHT);
    let report = details_only.screenshot("log/details-only");
    assert!(report.png_path.exists());
    assert!(detail_line_text(&details_only, DETAIL_AUTHOR_LINE).contains(FIXTURE_AUTHOR_NAME));
    assert_eq!(
        details_only.driver().selected_commit_files().len(),
        1,
        "the newest commit changes one file"
    );
}

#[test]
fn arrow_and_page_keys_move_the_selection_within_the_log_pane() {
    let mut terminal = log_terminal(
        FixtureRepo::LargeTenThousand,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    let history = terminal.driver().commits().to_vec();
    let selected_id = |terminal: &TestTerminal<App>| {
        terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.id.clone())
    };
    terminal.send_keys("down");
    assert_eq!(selected_id(&terminal), Some(history[1].id.clone()));
    terminal.send_keys("up");
    assert_eq!(selected_id(&terminal), Some(history[0].id.clone()));
    terminal.send_keys("pagedown");
    assert_eq!(
        selected_id(&terminal),
        Some(history[LOG_PAGE_JUMP].id.clone())
    );
    terminal.send_keys("pageup");
    assert_eq!(selected_id(&terminal), Some(history[0].id.clone()));
    terminal.send_keys("end");
    assert_eq!(
        selected_id(&terminal),
        Some(history[history.len() - 1].id.clone())
    );
    terminal.send_keys("home");
    assert_eq!(selected_id(&terminal), Some(history[0].id.clone()));
}

#[test]
fn clicking_a_commit_row_selects_that_commit() {
    let mut terminal = log_terminal(
        FixtureRepo::LargeTenThousand,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    let commits = commits_pane(&terminal);
    let history = terminal.driver().commits().to_vec();
    terminal.mouse_click(commits.x + SUBJECT_COLUMN, commits.y + 2);
    assert_eq!(
        terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.id.clone()),
        Some(history[2].id.clone())
    );
    assert_eq!(
        terminal.cell(commits.x, commits.y + 2).background,
        SELECTED_BACKGROUND
    );
    terminal.mouse_click(commits.x + SUBJECT_COLUMN, commits.y + 1);
    assert_eq!(
        terminal
            .driver()
            .selected_commit()
            .map(|commit| commit.id.clone()),
        Some(history[1].id.clone())
    );
    let details = details_pane(&terminal);
    assert!(
        detail_line_text(&terminal, DETAIL_AUTHOR_LINE).starts_with(&history[1].short_id[..7]),
        "the details column follows the clicked commit"
    );
    assert!(details.width > 0);
}

const MEDIUM_COLUMNS: u16 = 120;
const MEDIUM_ROWS: u16 = 32;
const WIDE_LAYOUT_COLUMNS: u16 = 220;
const WIDE_LAYOUT_ROWS: u16 = 70;
const READABLE_SUBJECT_COLUMNS: usize = 20;
const LONG_SUBJECT: &str = "checkout the readable subject line";
const HEAD_REF_LABEL_PREFIX: &str = gitdeck_tui::HEAD_REF_LABEL_PREFIX;
const CURRENT_BRANCH_LABEL: &str = "main";

#[test]
fn medium_width_uses_compact_log_rows() {
    let mut terminal = log_terminal(
        FixtureRepo::LargeTenThousand,
        MEDIUM_COLUMNS,
        MEDIUM_ROWS,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    let commits = commits_pane(&terminal);
    let commit = terminal
        .driver()
        .commits()
        .first()
        .cloned()
        .expect("the loaded log holds commits");
    let row_text = commit_row_text(&terminal, commits.y);
    let date_text = format_commit_date(commit.author_timestamp, 0);
    assert!(
        !row_text.contains(&date_text),
        "the medium row drops the date column, row {row_text:?} still holds {date_text:?}"
    );
    assert!(
        !row_text.contains(&commit.author_name),
        "the medium row gives up the author column, row {row_text:?}"
    );
    assert!(
        row_text.contains(&commit.subject),
        "the medium row keeps the subject, row {row_text:?}"
    );
    terminal.screenshot("log/medium-compact-rows");
}

#[test]
fn medium_width_shows_a_readable_subject() {
    let repository_path = scratch_repository("medium-readable-subject");
    run_git(
        &repository_path,
        0,
        &["commit", "-q", "--allow-empty", "-m", LONG_SUBJECT],
    );
    let terminal = open_log_terminal(&repository_path, MEDIUM_COLUMNS, MEDIUM_ROWS);
    let commits = commits_pane(&terminal);
    let row_text = commit_row_text(&terminal, commits.y);
    let readable_prefix: String = LONG_SUBJECT
        .chars()
        .take(READABLE_SUBJECT_COLUMNS)
        .collect();
    assert!(
        row_text.contains(&readable_prefix),
        "the medium row shows at least {READABLE_SUBJECT_COLUMNS} subject columns, row {row_text:?}"
    );
}

#[test]
fn compact_refs_drop_the_head_prefix() {
    let repository_path = scratch_repository("medium-compact-refs");
    run_git(
        &repository_path,
        0,
        &["commit", "-q", "--allow-empty", "-m", LONG_SUBJECT],
    );
    let terminal = open_log_terminal(&repository_path, MEDIUM_COLUMNS, MEDIUM_ROWS);
    let commits = commits_pane(&terminal);
    let row_text = commit_row_text(&terminal, commits.y);
    let readable_prefix: String = LONG_SUBJECT
        .chars()
        .take(READABLE_SUBJECT_COLUMNS)
        .collect();
    assert!(
        !row_text.contains(HEAD_REF_LABEL_PREFIX),
        "the compact row drops the head prefix, row {row_text:?}"
    );
    assert!(
        row_text.contains(CURRENT_BRANCH_LABEL),
        "the compact row keeps the branch name, row {row_text:?}"
    );
    assert!(
        row_text.contains(&readable_prefix),
        "the compact row spends the freed columns on the subject, row {row_text:?}"
    );
}

#[test]
fn wide_refs_keep_the_head_prefix() {
    let repository_path = scratch_repository("wide-head-prefix");
    run_git(
        &repository_path,
        0,
        &["commit", "-q", "--allow-empty", "-m", LONG_SUBJECT],
    );
    let terminal = open_log_terminal(&repository_path, WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS);
    let commits = commits_pane(&terminal);
    let row_text = commit_row_text(&terminal, commits.y);
    assert!(
        row_text.contains(HEAD_REF_LABEL_PREFIX),
        "the wide row keeps the head prefix, row {row_text:?}"
    );
}

#[test]
fn wide_layout_keeps_the_full_log_columns() {
    let mut terminal = log_terminal(
        FixtureRepo::LargeTenThousand,
        WIDE_LAYOUT_COLUMNS,
        WIDE_LAYOUT_ROWS,
        CapsProfile::Rich,
    );
    terminal.send_keys("alt+9");
    let commits = commits_pane(&terminal);
    let commit = terminal
        .driver()
        .commits()
        .first()
        .cloned()
        .expect("the loaded log holds commits");
    let row_text = commit_row_text(&terminal, commits.y);
    let date_text = format_commit_date(commit.author_timestamp, 0);
    assert!(
        row_text.contains(&date_text),
        "the wide layout keeps the date column, row {row_text:?} is missing {date_text:?}"
    );
    assert!(
        row_text.contains(&commit.author_name),
        "the wide layout keeps the author column, row {row_text:?}"
    );
    terminal.screenshot("log/wide-layout-rows");
}
