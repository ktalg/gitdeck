#[path = "e2e/harness.rs"]
mod harness;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gitdeck_testkit::{copy_directory, fixture_git_command, fixture_repo, FixtureRepo};
use gitdeck_tui::{file_count_label, DIFF_MARKER_COLUMN, DIFF_TEXT_COLUMN};
use vt100::Color;

use harness::{artifacts_directory, TerminalSession};

const SCREEN_COLUMNS: u16 = 120;
const SCREEN_ROWS: u16 = 40;
const WIDE_COLUMNS: u16 = 160;
const WIDE_ROWS: u16 = 48;
const NARROW_COLUMNS: u16 = 100;
const NARROW_ROWS: u16 = 30;
const WIDE_LAYOUT_COLUMNS: u16 = 220;
const WIDE_LAYOUT_ROWS: u16 = 70;
const MEDIUM_COLUMNS: u16 = 120;
const MEDIUM_ROWS: u16 = 32;
const COMPACT_COLUMNS: u16 = 80;
const COMPACT_ROWS: u16 = 24;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(40);
const TEXT_TIMEOUT: Duration = Duration::from_secs(20);
const EXIT_TIMEOUT: Duration = Duration::from_secs(20);
const UNSAVED_MESSAGE_QUIT_TEXT: &str = "You have an unsaved commit message.";
const QUIT_PROMPT_CANCEL_LABEL: &str = "[ Cancel ]";
const MERGE_ABORT_PROMPT: &str = "Abort the merge in progress?";
const REBASE_ABORT_PROMPT: &str = "Abort the rebase in progress?";
const QUIT_PROMPT_TIMEOUT: Duration = Duration::from_millis(700);
const FOCUSED_SELECTION_BACKGROUND: Color = Color::Rgb(0x2E, 0x43, 0x6E);
const UNFOCUSED_SELECTION_BACKGROUND: Color = Color::Rgb(0x39, 0x3B, 0x40);
const ACCENT_FOREGROUND: Color = Color::Rgb(0x35, 0x74, 0xF0);
const BORDER_FOREGROUND: Color = Color::Rgb(0x39, 0x3B, 0x40);
const RAIL_ICON_COLUMN: u16 = 1;
const RAIL_ENTRY_ROWS: [u16; 3] = [2, 5, 8];
const CHANGES_PANEL_BACKGROUND: Color = Color::Rgb(0x2B, 0x2D, 0x30);
const CHANGES_FIRST_ROW: u16 = 3;
const CHANGES_GROUP_ROW_COLUMN: u16 = 4;
const CHANGES_FIRST_FILE_ROW: u16 = 4;
const CHANGES_FOURTH_ROW: u16 = 6;
const BODY_ROW: u16 = 10;
const FILE_ROW_CLICK_COLUMN: u16 = 10;
const DETAILS_FILE_CLICK_OFFSET: u16 = 6;
const DIVIDER_GLYPH: char = '│';
const DETAILS_VISIBLE_MIN_COLUMNS: u16 = 100;
const DIVIDER_DRAG_COLUMNS: u16 = 8;
const DIVIDER_MINIMUM_DELTA: u16 = 4;
const DETAILS_DRAG_ROW: u16 = 20;
const DETAILS_DRAG_TARGET_COLUMN: u16 = 55;
const DETAILS_DRAG_STEP_COLUMNS: u16 = 20;
const MEDIUM_DETAILS_DIVIDER_COLUMN: u16 = 79;
const MEDIUM_NARROWEST_DETAILS_DIVIDER_COLUMN: u16 = 95;
const MEDIUM_WIDEST_DETAILS_DIVIDER_COLUMN: u16 = 49;
const WIDE_DETAILS_DIVIDER_COLUMN: u16 = 158;
const WIDE_NARROWEST_DETAILS_DIVIDER_COLUMN: u16 = 195;
const WIDE_WIDEST_DETAILS_DIVIDER_COLUMN: u16 = 87;
const DIRTY_SUBJECT: &str = "call run twice";
const CONTAINING_BRANCH_LABEL: &str = "In 1 branch: main";
const ALTERNATE_SCREEN_ENTER: &str = "\u{1b}[?1049h";
const ALTERNATE_SCREEN_LEAVE: &str = "\u{1b}[?1049l";
const MOUSE_CAPTURE_OFF: [&str; 3] = ["\u{1b}[?1000l", "\u{1b}[?1002l", "\u{1b}[?1006l"];
const WHEEL_COLUMN: u16 = 60;
const WHEEL_NOTCHES: u8 = 3;
const WHEEL_LINES_PER_EVENT: u16 = 3;
const WHEEL_SCROLLED_ROWS: u16 = 9;
const DIFF_FIRST_ROW: u16 = 3;
const CHANGES_PANE_LEFT: u16 = 4;
const DIFF_LINE_NUMBER_WIDTH: u16 = 4;
const DIFF_CAPTURED_ROW_COUNT: u16 = 6;
const EDITOR_TAB_ROW: u16 = 1;
const ERROR_FOREGROUND: Color = Color::Rgb(0xDB, 0x5C, 0x5C);
const MUTED_FOREGROUND: Color = Color::Rgb(0x86, 0x8A, 0x91);
const REFRESH_FAILED_LABEL: &str = "Refresh failed";
const NEXT_STEP_HINT_PREFIX: &str = "→ ";
const MISSING_REMOTE_HINT: &str = "configure a remote first (git remote add)";
const MERGE_CONFLICTS_GROUP_LABEL: &str = "Merge Conflicts";
const CONFLICT_RESOLUTION_HINT: &str = "alt+o ours  alt+t theirs  alt+r resolve";
const CONFLICT_RESOLVED_RECEIPT: &str = "Accepted ours for src/conflict.txt";
const UNMERGED_STATUS_PREFIX: &str = "UU ";
const PUSH_FAILED_LABEL: &str = "Push failed";
const EMPTY_MESSAGE_STATUS: &str = "Commit message is empty";
const COMMIT_RECEIPT_PREFIX: &str = "Committed ";
const FIRST_COMMIT_SUBJECT: &str = "merge 10000";
const SCROLL_BURST_NOTCHES: u16 = 600;
const SCROLL_BURST_BATCH_NOTCHES: u8 = 100;
const SCROLL_BURST_BUDGET: Duration = Duration::from_millis(15_000);
const SCREEN_STABLE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const SCREEN_STABLE_READS: u8 = 3;
const SCREEN_STABLE_TIMEOUT: Duration = Duration::from_secs(10);
const WHEEL_CONFIRMATION_ATTEMPTS: u8 = 3;
const WHEEL_CHANGE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const WHEEL_CHANGE_TIMEOUT: Duration = Duration::from_secs(5);
const INPUT_DISCARD_POLL_INTERVAL: Duration = Duration::from_millis(50);
const INPUT_DISCARD_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSED_INPUT_CHUNK_BYTES: usize = 8 * 1024;
const WHEELED_COMMIT_SUBJECT: &str = "commit 9991";
const APP_FILE_TITLE: &str = "app.rs (Working Tree)";
const CJK_FILE_NAME: &str = "说明.md";
const CJK_TAB_TITLE: &str = "说明.md (Working Tree)";
const LARGE_FILE_TITLE: &str = "large.txt (Working Tree)";
const LONG_LINE_FILE_TITLE: &str = "long_line.txt (Working Tree)";
const LONG_LINE_LENGTH: usize = 400;
const CONTINUATION_MARKER: char = '↓';
const SOFT_WRAP_OFF_STATUS: &str = "Soft wrap: off";
const CLIPPED_LINE_MARKER: &str = ">";
const E2E_REPOSITORY_DIRECTORY: &str = "e2e-repos";
const NOT_A_REPOSITORY_DIRECTORY_NAME: &str = "not-a-repository";
const BROKEN_GIT_DIRECTORY_FILE: &str = "gitdir: ./missing-gitdir\n";
const COMMIT_BUTTON_LABEL: &str = "[ Commit ]";
const AMEND_COMMIT_BUTTON_LABEL: &str = "[ Amend Commit ]";
const AMEND_LABEL: &str = "Amend last commit";
const COMMIT_AND_PUSH_LABEL: &str = "[ Commit and Push… ]";
const COMMIT_AND_PUSH_COMPACT_LABEL: &str = "[ Push… ]";
const COMMIT_MESSAGE: &str = "commit from the pty";
const WRAPPED_MESSAGE: &str =
    "修复提交消息编辑器里的折行显示问题 the wrap keeps every typed character shown";
const MESSAGE_FROM_THE_LOG_PANE: &str = "from the log pane";
const SIGN_OFF_LABEL: &str = "Sign-off commit";
const SIGN_OFF_TRAILER: &str = "Signed-off-by: gitdeck-fixture <fixture@gitdeck.invalid>";
const AMENDED_SUBJECT: &str = "amended message";
const BACKSPACE_KEY: u8 = 0x7f;
const COMMIT_MESSAGE_TITLE: &str = "Commit Message";
const ERROR_BLOCK_TOP_LEFT: char = '┌';
const ERROR_BLOCK_TOP_RIGHT: char = '┐';
const ERROR_BLOCK_BOTTOM_LEFT: char = '└';
const ERROR_BLOCK_OK_LABEL: &str = "[ OK ]";
const TYPED_UPPERCASE_MESSAGE: &str = "Hello World ABC MiXeD123";
const CLEAN_TREE_SUBJECT: &str = "clean work tree";
const ERROR_BODY_ROW_COUNT: u16 = 4;
const AMENDED_RECEIPT: &str = "Amended commit";
const AMEND_ROW_LABEL: &str = "Amend last commit";
const UNVERSIONED_GROUP_LABEL: &str = "Unversioned Files";
const CURRENT_BRANCH: &str = "main";
const OTHER_BRANCH: &str = "feature/other";
const OTHER_BRANCH_SUBJECT: &str = "call run on the other branch";
const CHECKED_OUT_RECEIPT: &str = "feature/other checked out";
const CHECKOUT_FAILED_LABEL: &str = "Checkout failed";
const CHECKOUT_CONFLICT_STDERR_KEY: &str = "git checkout feature/other";
const BRANCHES_TITLE: &str = "Branches";
const NEW_BRANCH_DIALOG_TITLE: &str = "New Branch";
const NEW_BRANCH_INPUT_LABEL: &str = "Branch name: ";
const CREATED_BRANCH: &str = "feature/created";
const CREATED_BRANCH_RECEIPT: &str = "Created and checked out feature/created";
const CREATED_BRANCH_CHIP: &str = "* feature/created";
const FAST_FORWARD_RECEIPT_PREFIX: &str = "Fast-forward to ";
const ALREADY_UP_TO_DATE_STATUS: &str = "Already up to date";
const RAIL_LABEL_LENGTH: u16 = 8;
const DIVIDER_WIDTH: u16 = 1;
const CURRENT_BRANCH_CHIP: &str = "* main";
const OTHER_BRANCH_CHIP: &str = "* feature/other";
const CURRENT_BRANCH_MARKER: &str = "● main";
const OTHER_BRANCH_MARKER: &str = "● feature/other";
const OTHER_BRANCH_SUBJECT_PREFIX: &str = "call run on";
const LOG_FIRST_ROW: u16 = 3;
const TOP_BAR_ROW: u16 = 0;
const REMOTE_FETCH_LABEL: &str = "Fetch";
const REMOTE_PULL_LABEL: &str = "Pull";
const REMOTE_PUSH_LABEL: &str = "Push";
const PUSH_DIALOG_TITLE: &str = "Push";
const PUSH_DIALOG_MESSAGE: &str = "Push main to origin";
const PUSH_FORCE_WITH_LEASE_LABEL: &str = "Force with lease";
const PUSH_RECEIPT: &str = "Pushed main to origin";
const PULL_DIALOG_MESSAGE: &str = "Pull main from origin";
const PULL_FAST_FORWARD_ONLY_LABEL: &str = "Fast-forward only";
const PULL_RECEIPT: &str = "Pulled main from origin";
const PULL_FAILED_LABEL: &str = "Pull failed";
const PULL_CONFLICT_FILE: &str = "conflict.txt";
const FETCH_DIALOG_MESSAGE: &str = "Fetch from origin";
const FETCH_RECEIPT: &str = "Fetched from origin";
const FETCH_FAILED_LABEL: &str = "Fetch failed";
const REMOTE_TRACKING_LABEL: &str = "origin/main";
const PULL_DIALOG_ROW_COUNT: u16 = 8;
const PULL_CONFIRM_LABEL: &str = "[ Pull ]";
const FETCH_CONFIRM_LABEL: &str = "[ Fetch ]";
const FETCH_DIALOG_ROW_COUNT: u16 = 6;
const DISCARD_DIALOG_TITLE: &str = "Discard Changes";
const DISCARD_DIALOG_MESSAGE: &str = "Discard all local changes to src/app.rs?";
const DISCARD_IRREVERSIBLE_LABEL: &str = "This cannot be undone.";
const DISCARD_CONFIRM_LABEL: &str = "[ Discard ]";
const DISCARD_CANCEL_LABEL: &str = "[ Cancel ]";
const DISCARD_RECEIPT: &str = "Discarded changes to src/app.rs";
const DISCARD_CANCELLED_RECEIPT: &str = "src/app.rs kept";
const SELECT_FILE_ROW_HINT: &str = "Select a file row first";
const DISCARD_PATH: &str = "src/app.rs";
const DISCARD_RESTORED_LINE: &str = "run_twice";
const DISCARD_DROPPED_LINE: &str = "shutdown";
const NOTES_PATH: &str = "notes.txt";
const TABS_PATH: &str = "tabs.txt";
const TABS_BASE_SUBJECT: &str = "tabbed base";
const SECOND_PATH: &str = "extra.txt";
const NOTES_TITLE: &str = "notes.txt (Working Tree)";
const HUNK_BASE_SUBJECT: &str = "base notes";
const HUNK_COMMIT_MESSAGE: &str = "commit the selected hunk";
const HUNK_FILE_LINE_COUNT: usize = 20;
const FIRST_HUNK_LINE: usize = 2;
const SECOND_HUNK_LINE: usize = 19;
const SECOND_FILE_CHANGED_LINE: usize = 3;
const STAGED_CHANGED_LINE: usize = 3;
const LINE_BREAK: &str = "\n";
const CHANGES_SELECTION_SCAN_LIMIT: usize = 24;
const UNCHECKED_HUNK_LABEL: &str = "[ ]";
const CHECKED_HUNK_LABEL: &str = "[x]";
const COMMITTED_PREFIX: &str = "Committed ";
const FILTER_BAR_ROW: u16 = 2;
const DROPDOWN_ROW_SCAN_LIMIT: u16 = 12;
const FILTER_PLACEHOLDER_LABEL: &str = "Text or hash";
const FILTER_QUERY: &str = "side commit";
const FILTERED_COMMIT_COUNT: usize = 50;
const FILTERED_SUBJECT_FRAGMENT: &str = "side";
const UNFILTERED_NEIGHBOUR_SUBJECT: &str = "commit 9998";
const FILTERED_STATUS_LABEL: &str = "Filtered 50 commits";
const FILTER_SCAN_WINDOW_COMMITS: usize = 5000;
const MATCHES_BEYOND_THE_SCAN_WINDOW: usize = 100;
const SECOND_PAGE_SUBJECT: &str = "commit 9001";
const SECOND_PAGE_SUBJECT_POSITION: usize = 999;
const ADDED_WORD_HIGHLIGHT_BACKGROUND: Color = Color::Rgb(0x3A, 0x6B, 0x47);
const REMOVED_WORD_HIGHLIGHT_BACKGROUND: Color = Color::Rgb(0x6B, 0x3A, 0x3A);
const ADDED_LINE_BACKGROUND_COLOR: Color = Color::Rgb(0x25, 0x40, 0x2F);
const REMOVED_LINE_BACKGROUND_COLOR: Color = Color::Rgb(0x3A, 0x2A, 0x2A);
const WORD_HIGHLIGHT_ADDED_LINE: &str = "run_once();";
const WORD_HIGHLIGHT_REMOVED_LINE: &str = "run_twice();";
const WORD_HIGHLIGHT_CHANGED_ADDED_OFFSET: u16 = 4;
const WORD_HIGHLIGHT_CHANGED_REMOVED_OFFSET: u16 = 4;
const WORD_HIGHLIGHTS_OFF_STATUS: &str = "Word highlights: off";
const DIFF_ROW_SCAN_LIMIT: u16 = 24;
const LOG_TEXT_COLOR: Color = Color::Rgb(0xDF, 0xE1, 0xE5);
const END_KEY: &[u8] = b"\x1b[F";
const BRANCH_FILTER_CHIP_LABEL: &str = "Branch: HEAD";
const BRANCH_FILTER_APPLIED_CHIP_LABEL: &str = "Branch: feature/other";
const BRANCH_FILTER_HEAD_ROW_LABEL: &str = "HEAD";
const BRANCH_FILTER_TEXT_QUERY: &str = "clean";
const BRANCH_FILTER_COMBINED_STATUS: &str = "Filtered 1 commits";
const BRANCH_REF_PREFIX: &str = "feature/o";
const HEAD_REF_PREFIX_LABEL: &str = "HEAD -> ";
const DETAILS_SUBJECT: &str = "call run twice";
const DETAILS_FILE_COUNT: &str = "1 file";
const DETAILS_SEPARATOR: &str = "─";
const DEEP_PATH_FILE_NAME: &str = "note_extra.go";
const DEEP_PATH_ELLIPSIS_MARKER: &str = "…/library";
const LONG_DETAILS_SUBJECT_HEAD: &str = "restore the portable capability probe";
const LONG_DETAILS_SUBJECT_CONTINUATION: &str = "before the terminal matrix runs";

fn dirty_repository() -> PathBuf {
    fixture_repo(FixtureRepo::Dirty)
}

fn deep_path_repository_copy(name: &str, long_subject: &str) -> PathBuf {
    let repository = dirty_repository_copy(name);
    let deep_directory = repository.join("internal").join("library");
    std::fs::create_dir_all(&deep_directory).expect("the deep directory is created");
    std::fs::write(
        deep_directory.join(DEEP_PATH_FILE_NAME),
        "package library\n",
    )
    .expect("the deep file is written");
    commit_in_repository(&repository, long_subject);
    repository
}

fn deep_path_row(session: &mut TerminalSession) -> u16 {
    let divider = divider_column(session);
    let row_count = u16::try_from(screen_lines(session).len()).unwrap_or(0);
    (0..row_count)
        .find(|row| {
            session
                .row_text(*row, CHANGES_PANE_LEFT, divider)
                .contains(DEEP_PATH_FILE_NAME)
        })
        .unwrap_or_else(|| {
            panic!(
                "the changes list never shows {DEEP_PATH_FILE_NAME}, screen text was:\n{}",
                session.screen_text()
            )
        })
}

fn start_wide_shell(repository: &Path) -> TerminalSession {
    let mut session = TerminalSession::spawn(Some(repository), WIDE_COLUMNS, WIDE_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the wide shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session
}

fn start_wide_layout_shell(repository: &Path) -> TerminalSession {
    let mut session =
        TerminalSession::spawn(Some(repository), WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the wide layout shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session
}

fn start_shell(repository: &Path) -> TerminalSession {
    let mut session = TerminalSession::spawn(Some(repository), SCREEN_COLUMNS, SCREEN_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session
}

fn screen_lines(session: &mut TerminalSession) -> Vec<String> {
    session.screen_text().lines().map(str::to_string).collect()
}

fn screen_line(session: &mut TerminalSession, row: u16) -> String {
    screen_lines(session)
        .get(usize::from(row))
        .cloned()
        .unwrap_or_default()
}

fn raw_log_text(session: &TerminalSession) -> String {
    String::from_utf8_lossy(session.raw_log()).to_string()
}

fn finish_with_ctrl_q(session: &mut TerminalSession) {
    session.send_keys("ctrl+q");
    if session.wait_for_text(QUIT_PROMPT_CANCEL_LABEL, QUIT_PROMPT_TIMEOUT) {
        session.send_keys("tab");
        session.send_keys("enter");
    }
    assert_eq!(
        session.wait_for_exit(EXIT_TIMEOUT),
        Some(0),
        "the shell exits with the success code"
    );
}

fn dump_screen(session: &mut TerminalSession, name: &str) {
    let directory = artifacts_directory().join("e2e");
    std::fs::create_dir_all(&directory).expect("the e2e screen directory is created");
    let text = session.screen_text();
    std::fs::write(directory.join(name), format!("{text}\n"))
        .expect("the e2e screen dump is written");
}

fn fixture_repository_copy(fixture: FixtureRepo, name: &str) -> PathBuf {
    let target = artifacts_directory()
        .join("tmp")
        .join(E2E_REPOSITORY_DIRECTORY)
        .join(name);
    if target.exists() {
        std::fs::remove_dir_all(&target).expect("the stale repository copy is removed");
    }
    copy_directory(&fixture_repo(fixture), &target).expect("the fixture repository is copied");
    target
}

fn repository_directory_name(repository: &Path) -> String {
    repository
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .expect("every e2e repository has a directory name")
}

fn dirty_repository_copy(name: &str) -> PathBuf {
    fixture_repository_copy(FixtureRepo::Dirty, name)
}

fn changes_row_text(session: &mut TerminalSession, row: u16) -> String {
    let divider = divider_column(session);
    session.row_text(row, CHANGES_PANE_LEFT, divider)
}

fn log_row_text(session: &mut TerminalSession, row: u16) -> String {
    let divider = divider_column(session);
    let right = log_pane_right(session);
    session.row_text(row, divider + 1, right)
}

fn log_pane_right(session: &mut TerminalSession) -> u16 {
    let divider = divider_column(session);
    let columns = u16::try_from(screen_line(session, BODY_ROW).chars().count())
        .expect("the screen width fits a terminal column");
    let shows_details = columns >= DETAILS_VISIBLE_MIN_COLUMNS;
    match details_divider_column(session) {
        Some(column) if shows_details && column > divider => column,
        _ => SCREEN_COLUMNS,
    }
}

fn details_divider_column(session: &mut TerminalSession) -> Option<u16> {
    let body_line = screen_line(session, BODY_ROW);
    body_line
        .chars()
        .enumerate()
        .filter(|(_, character)| *character == DIVIDER_GLYPH)
        .map(|(column, _)| column)
        .last()
        .and_then(|column| u16::try_from(column).ok())
}

fn log_row_of(session: &mut TerminalSession, needle: &str) -> u16 {
    for row in LOG_FIRST_ROW..SCREEN_ROWS - 1 {
        if log_row_text(session, row).contains(needle) {
            return row;
        }
    }
    panic!(
        "the log never shows {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

fn changes_row_of(session: &mut TerminalSession, needle: &str) -> u16 {
    for row in CHANGES_FIRST_ROW..SCREEN_ROWS - 1 {
        if changes_row_text(session, row).contains(needle) {
            return row;
        }
    }
    panic!(
        "the changes pane never shows {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

fn branch_selection_column(session: &mut TerminalSession) -> u16 {
    log_pane_right(session).saturating_sub(1)
}

fn wait_for_selection_row(
    session: &mut TerminalSession,
    column: u16,
    expected: u16,
    timeout: Duration,
) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if selection_row(session, column) == Some(expected) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn selection_row(session: &mut TerminalSession, column: u16) -> Option<u16> {
    (1..SCREEN_ROWS - 1)
        .find(|row| session.cell(column, *row).background == FOCUSED_SELECTION_BACKGROUND)
}

fn not_a_repository_directory() -> PathBuf {
    let directory = artifacts_directory()
        .join("tmp")
        .join(NOT_A_REPOSITORY_DIRECTORY_NAME);
    std::fs::create_dir_all(&directory).expect("the non repository directory is created");
    std::fs::write(directory.join(".git"), BROKEN_GIT_DIRECTORY_FILE)
        .expect("the broken git directory file is written");
    directory
}

#[test]
fn starts_on_the_alternate_screen_and_shows_the_shell() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(Some(&repository), SCREEN_COLUMNS, SCREEN_ROWS, &[]);
    assert!(
        session.wait_for_text("Text or hash", STARTUP_TIMEOUT),
        "the filter bar renders, screen text was:\n{}",
        session.screen_text()
    );
    let screen = session.screen_text();
    assert!(screen.contains("Commit"));
    assert!(screen.contains("Log: HEAD"));
    assert!(screen.contains("Text or hash"));
    assert!(raw_log_text(&session).contains(ALTERNATE_SCREEN_ENTER));
    finish_with_ctrl_q(&mut session);
}

#[test]
fn ctrl_q_exits_and_restores_the_terminal() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_bytes(b"\x11");
    assert_eq!(session.wait_for_exit(EXIT_TIMEOUT), Some(0));
    assert_restored_terminal(&session);
}

#[test]
fn ctrl_c_exits_and_restores_the_terminal() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_bytes(b"\x03");
    assert_eq!(session.wait_for_exit(EXIT_TIMEOUT), Some(0));
    assert_restored_terminal(&session);
}

#[test]
fn escape_closes_the_diff_tab_instead_of_quitting() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, "app.rs");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "escape-before.txt");
    session.send_bytes(b"\x1b");
    assert!(
        wait_for_text_absence(&mut session, APP_FILE_TITLE, TEXT_TIMEOUT),
        "the escape closes the diff tab, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "escape-after.txt");
    assert_eq!(
        session.try_exit_code(),
        None,
        "the escape leaves the shell running, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains("Log: HEAD"),
        "the log tab is back, screen text was:\n{}",
        session.screen_text()
    );
    session.send_bytes(b"\x1b");
    assert_eq!(
        session.try_exit_code(),
        None,
        "a second escape leaves the shell running"
    );
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

fn assert_restored_terminal(session: &TerminalSession) {
    let raw_log = raw_log_text(session);
    assert!(
        raw_log.contains(ALTERNATE_SCREEN_LEAVE),
        "the shell leaves the alternate screen"
    );
    for sequence in MOUSE_CAPTURE_OFF {
        assert!(
            raw_log.contains(sequence),
            "the shell disables mouse capture with {sequence:?}"
        );
    }
}

fn caps_detection_environment(term: &str, colorterm: &str) -> Vec<(String, String)> {
    harness::overridden_environment(&[
        ("TERM", term),
        ("COLORTERM", colorterm),
        ("TERM_PROGRAM", ""),
        ("NO_COLOR", ""),
        ("GITDECK_CAPS", ""),
    ])
}

fn start_doctor_session(environment: &[(String, String)], arguments: &[&str]) -> TerminalSession {
    TerminalSession::spawn_with_environment(
        None,
        COMPACT_COLUMNS,
        COMPACT_ROWS,
        environment,
        arguments,
    )
}

#[test]
fn caps_detection_matches_the_terminal_environment() {
    let cases = [
        ("xterm-256color", "", "standard"),
        ("xterm-256color", "truecolor", "rich"),
    ];
    for (term, colorterm, expected_caps) in cases {
        let environment = caps_detection_environment(term, colorterm);
        let mut session = start_doctor_session(&environment, &["--doctor"]);
        let expected_caps_line = format!("caps: {expected_caps}");
        assert!(
            session.wait_for_text(&expected_caps_line, TEXT_TIMEOUT),
            "TERM {term} with COLORTERM {colorterm:?} reports {expected_caps_line}, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.wait_for_text("caps source: detected", TEXT_TIMEOUT),
            "TERM {term} with COLORTERM {colorterm:?} reports the detected source, screen text was:\n{}",
            session.screen_text()
        );
        assert_eq!(session.wait_for_exit(EXIT_TIMEOUT), Some(0));
    }
}

#[test]
fn caps_flag_still_overrides_the_environment() {
    let environment = caps_detection_environment("xterm-256color", "truecolor");
    let mut session = start_doctor_session(&environment, &["--caps", "basic", "--doctor"]);
    assert!(
        session.wait_for_text("caps: basic", TEXT_TIMEOUT),
        "the flag wins over the truecolor environment, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text("caps source: cli", TEXT_TIMEOUT),
        "the report names the flag as the caps source, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(session.wait_for_exit(EXIT_TIMEOUT), Some(0));
}

#[test]
fn tab_cycles_focus_and_ctrl_k_returns_to_changes() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    let selected_row = (CHANGES_GROUP_ROW_COLUMN, CHANGES_FIRST_ROW);
    assert_eq!(
        session.cell(selected_row.0, selected_row.1).background,
        FOCUSED_SELECTION_BACKGROUND
    );
    session.send_keys("tab");
    assert_eq!(
        session.cell(selected_row.0, selected_row.1).background,
        UNFOCUSED_SELECTION_BACKGROUND,
        "the changes selection loses its focus highlight on tab"
    );
    session.send_keys("tab");
    assert_eq!(
        session.cell(selected_row.0, selected_row.1).background,
        UNFOCUSED_SELECTION_BACKGROUND
    );
    session.send_keys("tab");
    assert_eq!(
        session.cell(selected_row.0, selected_row.1).background,
        FOCUSED_SELECTION_BACKGROUND,
        "three tabs return the focus to the changes pane"
    );
    session.send_keys("alt+9");
    assert_eq!(
        session.cell(selected_row.0, selected_row.1).background,
        UNFOCUSED_SELECTION_BACKGROUND
    );
    session.send_keys("ctrl+k");
    assert_eq!(
        session.cell(selected_row.0, selected_row.1).background,
        FOCUSED_SELECTION_BACKGROUND,
        "ctrl+k focuses the changes pane"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn mouse_click_selects_a_file_row() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    dump_screen(&mut session, "mouse-click-screen-start.txt");
    assert_eq!(
        session
            .cell(CHANGES_GROUP_ROW_COLUMN, CHANGES_FIRST_ROW)
            .background,
        FOCUSED_SELECTION_BACKGROUND
    );
    session.mouse_click(FILE_ROW_CLICK_COLUMN, CHANGES_FOURTH_ROW);
    let clicked_row = screen_line(&mut session, CHANGES_FOURTH_ROW);
    assert!(
        clicked_row.contains("    [x] A  module.rs  src"),
        "the clicked row is the added src/module.rs file row, got {clicked_row:?}"
    );
    assert_eq!(
        session
            .cell(FILE_ROW_CLICK_COLUMN, CHANGES_FOURTH_ROW)
            .background,
        FOCUSED_SELECTION_BACKGROUND,
        "the clicked row carries the selection background"
    );
    assert_eq!(
        session
            .cell(CHANGES_GROUP_ROW_COLUMN, CHANGES_FIRST_ROW)
            .background,
        CHANGES_PANEL_BACKGROUND,
        "the previously selected row drops its selection background"
    );
    assert!(
        screen_line(&mut session, SCREEN_ROWS - 1).contains("main"),
        "the status bar keeps reporting the branch"
    );
    dump_screen(&mut session, "screen-after-click.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn mouse_drag_on_divider_resizes_the_changes_pane() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    let divider_before = divider_column(&mut session);
    dump_screen(&mut session, "divider-drag-screen-start.txt");
    session.mouse_drag(
        (divider_before, BODY_ROW),
        (divider_before + DIVIDER_DRAG_COLUMNS, BODY_ROW),
    );
    let divider_after = divider_column(&mut session);
    dump_screen(&mut session, "screen-after-drag.txt");
    assert_eq!(
        divider_after,
        divider_before + DIVIDER_DRAG_COLUMNS,
        "the drag follows the pointer by {DIVIDER_DRAG_COLUMNS} columns"
    );
    assert!(
        divider_after >= divider_before + DIVIDER_MINIMUM_DELTA,
        "the drag moves the divider from {divider_before} to {divider_after}"
    );
    assert!(
        screen_line(&mut session, BODY_ROW).contains(DIVIDER_GLYPH),
        "the divider column is still rendered"
    );
    finish_with_ctrl_q(&mut session);
}

fn divider_column(session: &mut TerminalSession) -> u16 {
    let body_line = screen_line(session, BODY_ROW);
    let column = body_line
        .chars()
        .position(|character| character == DIVIDER_GLYPH)
        .unwrap_or_else(|| panic!("no divider glyph in body row {body_line:?}"));
    u16::try_from(column).expect("the divider column fits a terminal column")
}

fn dragged_details_divider_column(session: &mut TerminalSession) -> u16 {
    details_divider_column(session).unwrap_or_else(|| panic!("the details divider sits on screen"))
}

fn drag_details_divider(session: &mut TerminalSession, target_column: u16) -> u16 {
    let mut divider_column = dragged_details_divider_column(session);
    while divider_column != target_column {
        let next_column = divider_column_one_drag_step_toward(divider_column, target_column);
        session.mouse_drag(
            (divider_column, DETAILS_DRAG_ROW),
            (next_column, DETAILS_DRAG_ROW),
        );
        let reached_column = dragged_details_divider_column(session);
        if reached_column == divider_column {
            break;
        }
        divider_column = reached_column;
    }
    session.mouse_drag(
        (divider_column, DETAILS_DRAG_ROW),
        (target_column, DETAILS_DRAG_ROW),
    );
    dragged_details_divider_column(session)
}

fn divider_column_one_drag_step_toward(divider_column: u16, target_column: u16) -> u16 {
    let step = DETAILS_DRAG_STEP_COLUMNS.min(divider_column.abs_diff(target_column));
    if target_column > divider_column {
        divider_column + step
    } else {
        divider_column - step
    }
}

#[test]
fn dragging_the_details_divider_works_in_a_real_terminal() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the medium shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    let divider_before = dragged_details_divider_column(&mut session);
    assert_eq!(
        divider_before, MEDIUM_DETAILS_DIVIDER_COLUMN,
        "the medium details divider starts at the default width"
    );
    dump_screen(&mut session, "details-resize-120x32-default.txt");
    session.mouse_drag(
        (divider_before, DETAILS_DRAG_ROW),
        (DETAILS_DRAG_TARGET_COLUMN, DETAILS_DRAG_ROW),
    );
    let divider_after = dragged_details_divider_column(&mut session);
    assert_eq!(
        divider_after, DETAILS_DRAG_TARGET_COLUMN,
        "the details divider follows the pointer from {divider_before} to \
         {DETAILS_DRAG_TARGET_COLUMN}"
    );
    assert!(
        divider_after < divider_before,
        "the details left edge moves left instead of staying at {divider_before}"
    );
    let screen = session.screen_text();
    assert!(
        screen.contains(DIRTY_SUBJECT),
        "the widened details column still shows the commit subject, screen text was:\n{screen}"
    );
    assert!(
        screen.contains(CONTAINING_BRANCH_LABEL),
        "the widened details column still shows its branch label, screen text was:\n{screen}"
    );
    dump_screen(&mut session, "details-resize-120x32-dragged.txt");
    assert_eq!(
        drag_details_divider(&mut session, MEDIUM_COLUMNS - 1),
        MEDIUM_NARROWEST_DETAILS_DIVIDER_COLUMN,
        "the medium drag shrinks the details column to its minimum"
    );
    dump_screen(&mut session, "details-resize-120x32-narrowest.txt");
    assert_eq!(
        drag_details_divider(&mut session, 0),
        MEDIUM_WIDEST_DETAILS_DIVIDER_COLUMN,
        "the medium drag grows the details column to its maximum"
    );
    dump_screen(&mut session, "details-resize-120x32-widest.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn dragging_the_details_divider_clamps_on_the_wide_layout() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(
        Some(&repository),
        WIDE_LAYOUT_COLUMNS,
        WIDE_LAYOUT_ROWS,
        &[],
    );
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the wide shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        dragged_details_divider_column(&mut session),
        WIDE_DETAILS_DIVIDER_COLUMN,
        "the wide details divider starts at the proportional width"
    );
    dump_screen(&mut session, "details-resize-220x70-default.txt");
    assert_eq!(
        drag_details_divider(&mut session, WIDE_LAYOUT_COLUMNS - 1),
        WIDE_NARROWEST_DETAILS_DIVIDER_COLUMN,
        "the wide drag shrinks the details column to its minimum"
    );
    dump_screen(&mut session, "details-resize-220x70-narrowest.txt");
    assert_eq!(
        drag_details_divider(&mut session, 0),
        WIDE_WIDEST_DETAILS_DIVIDER_COLUMN,
        "the wide drag grows the details column to its maximum"
    );
    dump_screen(&mut session, "details-resize-220x70-widest.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn space_checks_and_enter_opens_diff_and_ctrl_w_closes() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_keys("down");
    session.send_keys("space");
    let group_row = screen_line(&mut session, CHANGES_FIRST_ROW);
    assert!(
        group_row.contains("[-]"),
        "the unchecked file leaves the group partially checked, got {group_row:?}"
    );
    session.send_keys("enter");
    assert!(
        session.wait_for_text("(Working Tree)", TEXT_TIMEOUT),
        "the diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+w");
    let screen = session.screen_text();
    assert!(
        !screen.contains("(Working Tree)"),
        "the diff tab closes, screen text was:\n{screen}"
    );
    assert!(screen.contains("Log: HEAD"));
    finish_with_ctrl_q(&mut session);
}

#[test]
fn opening_a_diff_with_tabs_does_not_crash() {
    let repository = tabbed_repository("control-characters-tabs");
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    session.send_keys("down");
    session.send_keys("enter");
    assert!(
        session.wait_for_text("(Working Tree)", TEXT_TIMEOUT),
        "the diff tab opens for a file with tabs, screen text was:\n{}",
        session.screen_text()
    );
    let screen = session.screen_text();
    for line in screen.lines() {
        assert!(
            !line.contains(char::is_control),
            "a control character reached the screen: {line:?}"
        );
    }
    assert!(
        !screen.contains("panicked"),
        "the tabbed diff does not panic, screen text was:\n{screen}"
    );
    assert!(
        screen.contains("a   b"),
        "the removed tab expands to the next eight column stop, screen text was:\n{screen}"
    );
    assert!(
        screen.contains("a   B"),
        "the added tab expands to the next eight column stop, screen text was:\n{screen}"
    );
    dump_screen(&mut session, "control-characters-tabs-diff.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn resize_updates_the_screen_model_dimensions() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.resize(COMPACT_COLUMNS, COMPACT_ROWS);
    assert_eq!(
        screen_lines(&mut session).len(),
        usize::from(COMPACT_ROWS),
        "the screen model keeps one line per resized row"
    );
    assert_every_row_fills_the_columns(&mut session, COMPACT_COLUMNS);
    session.resize(WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS);
    assert_eq!(
        screen_lines(&mut session).len(),
        usize::from(WIDE_LAYOUT_ROWS),
        "the screen model grows back to the wide row count"
    );
    assert_every_row_fills_the_columns(&mut session, WIDE_LAYOUT_COLUMNS);
    finish_with_ctrl_q(&mut session);
}

#[test]
fn input_after_the_child_exited_is_discarded_without_a_panic() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+q");
    assert_eq!(session.wait_for_exit(EXIT_TIMEOUT), Some(0));
    session.send_bytes(b"\t");
    if !fill_closed_input_until_the_terminal_reports_it(&mut session) {
        assert_no_panic_reached_the_screen(&mut session);
    }
    assert_eq!(
        session.wait_for_exit(EXIT_TIMEOUT),
        Some(0),
        "the exited shell keeps its success code"
    );
}

fn fill_closed_input_until_the_terminal_reports_it(session: &mut TerminalSession) -> bool {
    let deadline = Instant::now() + INPUT_DISCARD_TIMEOUT;
    let closed_input_chunk = vec![b'a'; CLOSED_INPUT_CHUNK_BYTES];
    loop {
        session.send_bytes(&closed_input_chunk);
        if session.input_was_discarded_after_exit() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(INPUT_DISCARD_POLL_INTERVAL);
    }
}

fn assert_no_panic_reached_the_screen(session: &mut TerminalSession) {
    let screen = session.screen_text();
    assert!(
        !screen.contains("panicked"),
        "writing to the closed pseudo terminal does not panic, screen text was:\n{screen}"
    );
}

#[test]
fn resize_reflows_the_layout() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.resize(NARROW_COLUMNS, NARROW_ROWS);
    let lines = screen_lines(&mut session);
    assert_eq!(lines.len(), usize::from(NARROW_ROWS));
    assert!(
        lines[0].contains(&repository_directory_name(&repository)),
        "the top bar keeps the project name on one row, got {:?}",
        lines[0]
    );
    assert!(
        lines[usize::from(NARROW_ROWS - 1)].contains("gitdeck"),
        "the status bar is the last row, got {:?}",
        lines[usize::from(NARROW_ROWS - 1)]
    );
    assert!(
        lines.iter().any(|line| line.contains("Log: HEAD")),
        "the log tab survives the resize"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn resize_between_three_widths_keeps_the_layout_consistent() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(
        Some(&repository),
        WIDE_LAYOUT_COLUMNS,
        WIDE_LAYOUT_ROWS,
        &[],
    );
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    let sizes = [
        (WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS),
        (COMPACT_COLUMNS, COMPACT_ROWS),
        (MEDIUM_COLUMNS, MEDIUM_ROWS),
    ];
    for (columns, rows) in sizes {
        session.resize(columns, rows);
        assert_resized_shell(&mut session, &repository, columns, rows);
    }
    finish_with_ctrl_q(&mut session);
}

#[test]
fn the_top_bar_names_the_opened_repository_directory() {
    let repository = dirty_repository_copy("project-name-label");
    let mut session = start_shell(&repository);
    let lines = screen_lines(&mut session);
    let top_bar = lines.first().cloned().unwrap_or_default();
    assert!(
        top_bar.contains("project-name-label"),
        "the top bar names the repository directory, got {top_bar:?}"
    );
    assert!(
        !top_bar.contains("no repository"),
        "a real repository is named, not the placeholder, got {top_bar:?}"
    );
    dump_screen(&mut session, "project-name-label.txt");
    finish_with_ctrl_q(&mut session);
}

fn assert_resized_shell(session: &mut TerminalSession, repository: &Path, columns: u16, rows: u16) {
    let lines = screen_lines(session);
    assert_eq!(
        lines.len(),
        usize::from(rows),
        "{columns}x{rows} keeps one line per row"
    );
    assert!(
        lines[0].contains(&repository_directory_name(repository)),
        "{columns}x{rows} keeps the project name on the first row, got {:?}",
        lines[0]
    );
    let status_line = lines.last().cloned().unwrap_or_default();
    assert!(
        status_line.contains(concat!("gitdeck ", env!("CARGO_PKG_VERSION"))),
        "{columns}x{rows} keeps the status bar on the last row, got {status_line:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("Log: HEAD")),
        "{columns}x{rows} keeps the log tab, screen text was:\n{}",
        lines.join("\n")
    );
    assert_every_row_fills_the_columns(session, columns);
}

fn assert_every_row_fills_the_columns(session: &mut TerminalSession, columns: u16) {
    for (index, line) in screen_lines(session).iter().enumerate() {
        assert_eq!(
            line.chars().count(),
            usize::from(columns),
            "row {index} holds {line:?} instead of {columns} columns"
        );
    }
}

#[test]
fn resize_during_diff_keeps_the_tab_and_content() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(
        Some(&repository),
        WIDE_LAYOUT_COLUMNS,
        WIDE_LAYOUT_ROWS,
        &[],
    );
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, "app.rs");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    let line_numbers_before = diff_line_numbers(&mut session);
    assert!(
        line_numbers_before
            .iter()
            .any(|number_column| number_column.trim() == "1"),
        "the diff pane shows a right aligned first line number, got {line_numbers_before:?}"
    );
    session.resize(COMPACT_COLUMNS, COMPACT_ROWS);
    assert!(
        screen_line(&mut session, EDITOR_TAB_ROW).contains(APP_FILE_TITLE),
        "the diff tab survives the resize, got {:?}",
        screen_line(&mut session, EDITOR_TAB_ROW)
    );
    assert_eq!(
        diff_line_numbers(&mut session),
        line_numbers_before,
        "the line number columns keep their alignment after the resize"
    );
    assert_every_row_fills_the_columns(&mut session, COMPACT_COLUMNS);
    finish_with_ctrl_q(&mut session);
}

fn diff_line_numbers(session: &mut TerminalSession) -> Vec<String> {
    let left = divider_column(session) + DIVIDER_WIDTH;
    (DIFF_FIRST_ROW..DIFF_FIRST_ROW + DIFF_CAPTURED_ROW_COUNT)
        .map(|row| session.row_text(row, left, left + DIFF_LINE_NUMBER_WIDTH))
        .collect()
}

#[test]
fn commit_block_and_dividers_scale_with_the_terminal() {
    let repository = dirty_repository();
    let sizes: [(u16, u16, u16, u16, u16); 3] = [
        (WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS, 52, 158, 17),
        (MEDIUM_COLUMNS, MEDIUM_ROWS, 38, 79, 7),
        (COMPACT_COLUMNS, COMPACT_ROWS, 38, 0, 5),
    ];
    for (columns, rows, changes_divider, details_divider, message_rows) in sizes {
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "the {columns}x{rows} shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        let (_, title_row) = changes_position_of(&mut session, COMMIT_MESSAGE_TITLE);
        let button_row = commit_button_row(&mut session);
        assert_eq!(
            button_row - title_row,
            message_rows,
            "the {columns}x{rows} message box keeps its rows, screen text was:\n{}",
            session.screen_text()
        );
        let amend_row = changes_position_of(&mut session, AMEND_LABEL).1;
        assert_eq!(
            amend_row + 1,
            title_row,
            "the {columns}x{rows} amend row sits above the message box"
        );
        let body = screen_line(&mut session, BODY_ROW);
        let divider_columns: Vec<u16> = body
            .chars()
            .enumerate()
            .filter(|(_, character)| *character == DIVIDER_GLYPH)
            .map(|(column, _)| u16::try_from(column).expect("the divider column"))
            .collect();
        let expected: Vec<u16> = if details_divider == 0 {
            vec![changes_divider]
        } else {
            vec![changes_divider, details_divider]
        };
        assert!(
            divider_columns.starts_with(&expected),
            "the {columns}x{rows} dividers sit at {divider_columns:?} instead of {expected:?}"
        );
        dump_screen(&mut session, &format!("layout-ratios-{columns}x{rows}.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn non_repository_path_shows_a_visible_error() {
    let directory = not_a_repository_directory();
    let mut session = TerminalSession::spawn(Some(&directory), SCREEN_COLUMNS, SCREEN_ROWS, &[]);
    assert!(
        session.wait_for_text("Cannot open repository", STARTUP_TIMEOUT),
        "the shell reports the unusable repository, screen text was:\n{}",
        session.screen_text()
    );
    let screen = session.screen_text();
    assert!(screen.contains("Cannot open repository"));
    assert!(screen.contains("no repository"));
    assert_eq!(
        session
            .cell(CHANGES_GROUP_ROW_COLUMN, CHANGES_FIRST_ROW)
            .foreground,
        Color::Rgb(0xDB, 0x5C, 0x5C),
        "the label uses the error color"
    );
    let error_row = screen_line(&mut session, CHANGES_FIRST_FILE_ROW);
    assert!(
        error_row.contains("git rev-parse"),
        "the failure reason is visible under the label, got {error_row:?}"
    );
    assert_eq!(
        session
            .cell(CHANGES_GROUP_ROW_COLUMN, CHANGES_FIRST_FILE_ROW)
            .foreground,
        Color::Rgb(0x86, 0x8A, 0x91),
        "the failure reason uses the muted color"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn wheel_moves_the_log_viewport_by_three_rows() {
    let repository = fixture_repo(FixtureRepo::LargeTenThousand);
    let mut session = TerminalSession::spawn(Some(&repository), WIDE_COLUMNS, WIDE_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text(FIRST_COMMIT_SUBJECT, STARTUP_TIMEOUT),
        "the first log page arrives before the keys land, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+9");
    for _ in 0..10 {
        session.send_keys("j");
    }
    assert!(
        wait_for_selection_row(&mut session, WHEEL_COLUMN, 3 + 10, TEXT_TIMEOUT),
        "the j keys walk the selection to the eleventh row, screen text was:\n{}",
        session.screen_text()
    );
    let selected_row_before = selection_row(&mut session, WHEEL_COLUMN)
        .expect("the selected commit carries the selection background");
    assert_eq!(selected_row_before, 3 + 10);
    let selected_commit_row_before = log_row_text(&mut session, selected_row_before);
    let first_row_before = log_row_text(&mut session, 3);
    let fourth_row_before = log_row_text(&mut session, 3 + WHEEL_LINES_PER_EVENT);
    assert!(
        first_row_before.contains(FIRST_COMMIT_SUBJECT),
        "the first log row holds {FIRST_COMMIT_SUBJECT:?}, got {first_row_before:?}"
    );
    dump_screen(&mut session, "wheel-viewport-before.txt");
    session.mouse_wheel_down(WHEEL_COLUMN, BODY_ROW, 1);
    let first_row_after = log_row_text(&mut session, 3);
    assert_eq!(
        first_row_after, fourth_row_before,
        "one wheel event scrolls the viewport three rows"
    );
    assert_ne!(first_row_after, first_row_before);
    let selected_row_after = selection_row(&mut session, WHEEL_COLUMN)
        .expect("the selection background survives the wheel");
    assert_eq!(
        selected_row_after,
        selected_row_before - WHEEL_LINES_PER_EVENT,
        "the selected commit rides up with the viewport"
    );
    assert_eq!(
        log_row_text(&mut session, selected_row_after),
        selected_commit_row_before,
        "the wheel keeps the selected commit"
    );
    dump_screen(&mut session, "wheel-viewport-after.txt");
    session.mouse_wheel_up(WHEEL_COLUMN, BODY_ROW, 1);
    assert_eq!(
        log_row_text(&mut session, 3),
        first_row_before,
        "a wheel event upwards returns to the first row"
    );
    assert_eq!(
        selection_row(&mut session, WHEEL_COLUMN),
        Some(selected_row_before),
        "the selected commit returns to its first row"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn wheel_scrolls_the_commit_list() {
    let repository = fixture_repo(FixtureRepo::LargeTenThousand);
    let mut session = TerminalSession::spawn(Some(&repository), WIDE_COLUMNS, WIDE_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+9");
    let selected_before = selection_row(&mut session, WHEEL_COLUMN);
    assert_eq!(
        selected_before,
        Some(3),
        "the newest commit starts selected"
    );
    assert!(
        log_row_text(&mut session, 3).contains(FIRST_COMMIT_SUBJECT),
        "the first commit row holds {FIRST_COMMIT_SUBJECT:?}, got {:?}",
        log_row_text(&mut session, 3)
    );
    dump_screen(&mut session, "wheel-commits-before.txt");
    session.mouse_wheel_down(WHEEL_COLUMN, BODY_ROW, WHEEL_NOTCHES);
    let selected_after = selection_row(&mut session, WHEEL_COLUMN);
    assert_eq!(
        selected_after,
        Some(3),
        "the selection follows the viewport back to its first row"
    );
    assert_eq!(
        session.cell(WHEEL_COLUMN, 3).background,
        FOCUSED_SELECTION_BACKGROUND
    );
    assert!(
        log_row_text(&mut session, 3).contains(WHEELED_COMMIT_SUBJECT),
        "the scrolled first row holds {WHEELED_COMMIT_SUBJECT:?}, got {:?}",
        log_row_text(&mut session, 3)
    );
    dump_screen(&mut session, "wheel-commits-after.txt");
    session.mouse_wheel_up(WHEEL_COLUMN, BODY_ROW, WHEEL_NOTCHES);
    assert!(
        log_row_text(&mut session, 3).contains(FIRST_COMMIT_SUBJECT),
        "the first row shows the newest commit again, got {:?}",
        log_row_text(&mut session, 3)
    );
    assert_eq!(
        selection_row(&mut session, WHEEL_COLUMN),
        Some(3 + WHEEL_SCROLLED_ROWS),
        "the selected commit keeps its place in the list"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn wheel_scrolls_the_diff_view() {
    let repository = fixture_repo(FixtureRepo::DiffShowcase);
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, "large.txt");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(LARGE_FILE_TITLE, TEXT_TIMEOUT),
        "the large diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    let captured_row_count = WHEEL_SCROLLED_ROWS * 2;
    let before: Vec<String> = (DIFF_FIRST_ROW..DIFF_FIRST_ROW + captured_row_count)
        .map(|row| log_row_text(&mut session, row))
        .collect();
    dump_screen(&mut session, "wheel-diff-before.txt");
    session.mouse_wheel_down(WHEEL_COLUMN, BODY_ROW, WHEEL_NOTCHES);
    dump_screen(&mut session, "wheel-diff-after.txt");
    for offset in 0..WHEEL_SCROLLED_ROWS {
        let row = DIFF_FIRST_ROW + offset;
        assert_eq!(
            log_row_text(&mut session, row),
            before[usize::from(WHEEL_SCROLLED_ROWS + offset)],
            "row {row} shows the diff row that three wheel notches scrolled in"
        );
    }
    finish_with_ctrl_q(&mut session);
}

fn count_diff_pane_symbol(session: &mut TerminalSession, needle: char) -> usize {
    count_diff_column_symbol(session, needle, DIFF_TEXT_COLUMN)
}

fn count_diff_column_symbol(
    session: &mut TerminalSession,
    needle: char,
    column_offset: u16,
) -> usize {
    let divider = usize::from(divider_column(session));
    let right = usize::from(log_pane_right(session));
    let text = session.screen_text();
    let mut counted = 0;
    for (index, line) in text.lines().enumerate() {
        if row_index(index) < usize::from(DIFF_FIRST_ROW) {
            continue;
        }
        for (column, symbol) in line.chars().enumerate() {
            if column < divider + 1 + usize::from(column_offset) || column >= right {
                continue;
            }
            if symbol == needle {
                counted += 1;
            }
        }
    }
    counted
}

fn row_index(line_index: usize) -> usize {
    line_index
}

fn details_band_texts(session: &mut TerminalSession) -> Vec<String> {
    let Some(left) = details_divider_column(session) else {
        return Vec::new();
    };
    let detail_left = usize::from(left) + 1;
    screen_lines(session)
        .into_iter()
        .map(|line| {
            line.chars()
                .skip(detail_left)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn changed_file_count_of_head(repository_path: &Path) -> usize {
    git_output(
        repository_path,
        &["show", "--name-only", "--format=", "HEAD"],
    )
    .lines()
    .filter(|line| !line.trim().is_empty())
    .count()
}

#[test]
fn a_diff_tab_widens_the_text_area_in_a_real_terminal() {
    let repository = fixture_repo(FixtureRepo::DiffShowcase);
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the medium shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text("diff showcase base", STARTUP_TIMEOUT),
        "the log lists the base commit, screen text was:\n{}",
        session.screen_text()
    );
    let details_header = file_count_label(changed_file_count_of_head(&repository));
    assert!(
        details_band_texts(&mut session)
            .iter()
            .any(|line| line.contains(&details_header)),
        "the details column shows its {details_header:?} header before the diff opens, screen text was:\n{}",
        session.screen_text()
    );
    let changes_divider = divider_column(&mut session);
    let details_divider =
        details_divider_column(&mut session).expect("the details divider sits on screen");
    let right_before = log_pane_right(&mut session);
    dump_screen(&mut session, "diff-width-120x32-before.txt");

    select_changes_file(&mut session, "long_line.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(LONG_LINE_FILE_TITLE, TEXT_TIMEOUT),
        "the long line diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "diff-width-120x32-after.txt");

    let body = screen_line(&mut session, BODY_ROW);
    assert_eq!(
        body.matches(DIVIDER_GLYPH).count(),
        1,
        "the diff screen keeps only the changes divider, row {body:?}"
    );
    let right_after = log_pane_right(&mut session);
    assert_eq!(
        right_after, MEDIUM_COLUMNS,
        "the diff text reaches the right screen edge, row {body:?}"
    );
    assert!(
        right_after >= right_before + DIVIDER_MINIMUM_DELTA,
        "the diff gains the details width, from {right_before} to {right_after}"
    );
    assert_eq!(
        divider_column(&mut session),
        changes_divider,
        "the changes divider stays at its column, screen text was:\n{}",
        session.screen_text()
    );
    assert_ne!(
        session.cell(details_divider, DIFF_FIRST_ROW).symbol,
        DIVIDER_GLYPH.to_string(),
        "the details divider column carries diff text, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !details_band_texts(&mut session)
            .iter()
            .any(|line| line.contains(&details_header)),
        "the details header leaves with the column, screen text was:\n{}",
        session.screen_text()
    );

    session.resize(WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS);
    dump_screen(&mut session, "diff-width-220x70-open.txt");
    let wide_body = screen_line(&mut session, BODY_ROW);
    assert_eq!(
        wide_body.matches(DIVIDER_GLYPH).count(),
        1,
        "the wide diff screen keeps only the changes divider, row {wide_body:?}"
    );
    session.send_keys("ctrl+w");
    assert!(
        session.wait_for_text(&details_header, TEXT_TIMEOUT),
        "closing the diff brings the details header back, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "diff-width-220x70-closed.txt");
    let closed_body = screen_line(&mut session, BODY_ROW);
    assert_eq!(
        closed_body.matches(DIVIDER_GLYPH).count(),
        2,
        "the closed diff screen draws both dividers again, row {closed_body:?}"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn the_amend_button_label_changes_in_a_real_terminal() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    assert!(
        session.wait_for_text(COMMIT_BUTTON_LABEL, TEXT_TIMEOUT),
        "the commit block names its commit button, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+a");
    assert!(
        session.wait_for_text(AMEND_COMMIT_BUTTON_LABEL, TEXT_TIMEOUT),
        "the checked amend row renames the button, screen text was:\n{}",
        session.screen_text()
    );
    let amended_screen = session.screen_text();
    assert!(
        !amended_screen.contains(COMMIT_BUTTON_LABEL),
        "the plain commit label leaves the button row, screen text was:\n{amended_screen}"
    );
    dump_screen(&mut session, "amend-button-120x40.txt");
    session.send_keys("alt+a");
    assert!(
        session.wait_for_text(COMMIT_BUTTON_LABEL, TEXT_TIMEOUT),
        "the unchecked amend row restores the commit label, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(AMEND_COMMIT_BUTTON_LABEL),
        "the amended label leaves the button row, screen text was:\n{}",
        session.screen_text()
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn a_long_diff_line_is_readable_at_120_columns() {
    let repository = fixture_repo(FixtureRepo::DiffShowcase);
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the medium shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    select_changes_file(&mut session, "long_line.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(LONG_LINE_FILE_TITLE, TEXT_TIMEOUT),
        "the long line diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "soft-wrap-on-120x32.txt");
    let visible_columns = count_diff_pane_symbol(&mut session, 'b');
    assert!(
        visible_columns + 8 >= LONG_LINE_LENGTH,
        "the wrapped diff shows the long added line across continuation rows, counted {visible_columns} of\n{}",
        session.screen_text()
    );
    assert!(
        count_diff_column_symbol(&mut session, CONTINUATION_MARKER, DIFF_MARKER_COLUMN) >= 2,
        "the wrapped rows carry the continuation marker, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+z");
    assert!(
        session.wait_for_text(SOFT_WRAP_OFF_STATUS, TEXT_TIMEOUT),
        "the status bar reports the soft wrap change, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "soft-wrap-off-120x32.txt");
    let unwrapped_characters = count_diff_pane_symbol(&mut session, 'b');
    assert!(
        unwrapped_characters > 0 && unwrapped_characters < LONG_LINE_LENGTH / 2,
        "the unwrapped diff truncates the long line instead of wrapping it, got {unwrapped_characters}"
    );
    assert_eq!(
        count_diff_column_symbol(&mut session, CONTINUATION_MARKER, DIFF_MARKER_COLUMN),
        0,
        "the unwrapped diff drops the continuation markers"
    );
    let added_row = diff_row_of(&mut session, "bbb");
    let pane_right = log_pane_right(&mut session);
    assert_eq!(
        session.cell(pane_right - 1, added_row).symbol,
        CLIPPED_LINE_MARKER,
        "the unwrapped long line marks its truncated last column, screen text was:\n{}",
        session.screen_text()
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn resize_to_narrow_hides_the_details_pane() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    let wide_body = screen_line(&mut session, BODY_ROW);
    assert_eq!(
        wide_body.matches(DIVIDER_GLYPH).count(),
        2,
        "a wide shell draws the changes and the details dividers, row {wide_body:?}"
    );
    session.resize(COMPACT_COLUMNS, COMPACT_ROWS);
    let narrow_body = screen_line(&mut session, BODY_ROW);
    assert_eq!(
        narrow_body.matches(DIVIDER_GLYPH).count(),
        1,
        "eighty columns drop the details divider, row {narrow_body:?}"
    );
    let commits_line = screen_line(&mut session, CHANGES_FIRST_ROW);
    let newest_subject = git_output(&repository, &["log", "-1", "--format=%s"]);
    let newest_row = screen_line(&mut session, LOG_FIRST_ROW);
    assert!(
        newest_row.contains(newest_subject.trim()),
        "the narrowed commit row shows the whole subject, row {newest_row:?}"
    );
    assert!(
        !commits_line.contains("1/1/20"),
        "the narrowed commit row drops the date column, row {commits_line:?}"
    );
    assert_eq!(
        screen_lines(&mut session).len(),
        usize::from(COMPACT_ROWS),
        "the screen model keeps one line per row"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn enter_on_a_commit_opens_the_first_file_diff() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_keys("alt+9");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the commit diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    let screen = session.screen_text();
    assert!(
        screen.contains("run_twice();"),
        "the diff pane shows the committed line, screen text was:\n{screen}"
    );
    assert!(
        !screen.contains("Text or hash"),
        "the diff tab reclaims the filter row, screen text was:\n{screen}"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn cjk_tab_title_is_not_truncated_mid_character() {
    let repository = fixture_repo(FixtureRepo::CjkPaths);
    let mut session = TerminalSession::spawn(Some(&repository), WIDE_COLUMNS, WIDE_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, CJK_FILE_NAME);
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(CJK_TAB_TITLE, TEXT_TIMEOUT),
        "the cjk diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    let tab_line = screen_line(&mut session, EDITOR_TAB_ROW);
    assert!(
        tab_line.contains(CJK_TAB_TITLE),
        "the tab row keeps every wide character, row {tab_line:?}"
    );
    assert!(
        !tab_line.contains("说.md"),
        "the tab row lost a wide character, row {tab_line:?}"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn double_click_opens_the_diff_tab() {
    let repository = dirty_repository();
    let mut single_click_session = start_shell(&repository);
    single_click_session.send_keys("ctrl+k");
    let row = changes_row_of(&mut single_click_session, "app.rs");
    single_click_session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    let clicked_screen = single_click_session.screen_text();
    assert!(
        !clicked_screen.contains(APP_FILE_TITLE),
        "a single click leaves the diff closed, screen text was:\n{clicked_screen}"
    );
    dump_screen(&mut single_click_session, "single-click-changes.txt");
    finish_with_ctrl_q(&mut single_click_session);
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, "app.rs");
    session.mouse_double_click(FILE_ROW_CLICK_COLUMN, row);
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "two fast presses on a file row open its diff, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "double-click-diff.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn refresh_failure_is_visible_after_dot_git_is_removed() {
    let repository = dirty_repository_copy("dot-git-removed");
    let mut session = start_shell(&repository);
    dump_screen(&mut session, "refresh-before.txt");
    std::fs::remove_dir_all(repository.join(".git")).expect("the git directory is removed");
    session.send_keys("f5");
    assert!(
        session.wait_for_text(REFRESH_FAILED_LABEL, TEXT_TIMEOUT),
        "the failed refresh is reported on screen, screen text was:\n{}",
        session.screen_text()
    );
    let lines = screen_lines(&mut session);
    let title_row = u16::try_from(
        lines
            .iter()
            .position(|line| line.contains(REFRESH_FAILED_LABEL))
            .expect("the title row exists"),
    )
    .expect("the title row fits a screen row");
    let title_line = screen_line(&mut session, title_row);
    assert_eq!(
        title_line.chars().nth(usize::from(CHANGES_PANE_LEFT)),
        Some(ERROR_BLOCK_TOP_LEFT),
        "the error block opens with its top border, row {title_line:?}"
    );
    assert!(
        title_line.contains(ERROR_BLOCK_TOP_RIGHT),
        "the error block closes its top border, row {title_line:?}"
    );
    assert_eq!(
        session.cell(CHANGES_PANE_LEFT + 2, title_row).foreground,
        ERROR_FOREGROUND,
        "the title uses the error color"
    );
    let reason_row = title_row + 1;
    let reason_line = screen_line(&mut session, reason_row);
    assert!(
        reason_line.contains("git status"),
        "the failure reason is visible under the title, got {reason_line:?}"
    );
    assert_eq!(
        session.cell(CHANGES_PANE_LEFT + 2, reason_row).foreground,
        MUTED_FOREGROUND,
        "the failure reason uses the muted color"
    );
    let ok_row = title_row + ERROR_BODY_ROW_COUNT + 1;
    let ok_line = screen_line(&mut session, ok_row);
    assert!(
        ok_line.contains(ERROR_BLOCK_OK_LABEL),
        "the error block carries the ok button, row {ok_line:?}"
    );
    let bottom_line = screen_line(&mut session, ok_row + 1);
    assert_eq!(
        bottom_line.chars().nth(usize::from(CHANGES_PANE_LEFT)),
        Some(ERROR_BLOCK_BOTTOM_LEFT),
        "the error block closes with its bottom border, row {bottom_line:?}"
    );
    assert!(
        screen_line(&mut session, ok_row + 2).contains(AMEND_ROW_LABEL),
        "the error block stops above the amend row"
    );
    dump_screen(&mut session, "refresh-failed.txt");
    assert!(
        session.screen_text().contains("Unversioned Files"),
        "the changes list stays visible above the error"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn enter_dismisses_the_refresh_error_block() {
    let repository = dirty_repository_copy("dismiss-refresh-error");
    let mut session = start_shell(&repository);
    std::fs::remove_dir_all(repository.join(".git")).expect("the git directory is removed");
    session.send_keys("f5");
    assert!(
        session.wait_for_text(REFRESH_FAILED_LABEL, TEXT_TIMEOUT),
        "the failed refresh is reported on screen, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("enter");
    let screen = session.screen_text();
    assert!(
        !screen.contains(REFRESH_FAILED_LABEL),
        "the enter key dismisses the error block, screen text was:\n{screen}"
    );
    assert!(
        !screen.contains(APP_FILE_TITLE),
        "the enter key opens no diff while it dismisses the error, screen text was:\n{screen}"
    );
    assert!(
        screen.contains(COMMIT_BUTTON_LABEL),
        "the commit block survives the dismissal, screen text was:\n{screen}"
    );
    dump_screen(&mut session, "dismiss-refresh-error.txt");
    finish_with_ctrl_q(&mut session);
}

fn git_output(repository_path: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn commit_count(repository_path: &Path) -> usize {
    git_output(repository_path, &["rev-list", "--count", "HEAD"])
        .trim()
        .parse()
        .expect("the commit count is a number")
}

fn changed_path_count(repository_path: &Path) -> usize {
    git_output(repository_path, &["status", "--porcelain"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn head_subject(repository_path: &Path) -> String {
    git_output(repository_path, &["log", "-1", "--format=%s"])
        .trim()
        .to_string()
}

fn changes_position_of_either(session: &mut TerminalSession, needles: &[&str]) -> (u16, u16) {
    for needle in needles {
        if let Some(position) = try_changes_position_of(session, needle) {
            return position;
        }
    }
    panic!(
        "the changes pane never shows any of {needles:?}, screen text was:\n{}",
        session.screen_text()
    );
}

fn try_changes_position_of(session: &mut TerminalSession, needle: &str) -> Option<(u16, u16)> {
    let lines = screen_lines(session);
    lines.iter().enumerate().skip(1).find_map(|(index, line)| {
        line.find(needle).map(|column| {
            (
                u16::try_from(column).expect("the needle starts inside the row"),
                u16::try_from(index).expect("the row index fits a screen row"),
            )
        })
    })
}

fn changes_position_of(session: &mut TerminalSession, needle: &str) -> (u16, u16) {
    let lines = screen_lines(session);
    for (index, line) in lines.iter().enumerate().skip(1) {
        if let Some(column) = line.find(needle) {
            return (
                u16::try_from(column).expect("the needle starts inside the row"),
                u16::try_from(index).expect("the row index fits a screen row"),
            );
        }
    }
    panic!(
        "the changes pane never shows {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

fn commit_button_row(session: &mut TerminalSession) -> u16 {
    changes_position_of(session, COMMIT_BUTTON_LABEL).1
}

fn message_box_rows(session: &mut TerminalSession) -> Vec<String> {
    let (_, title_row) = changes_position_of(session, COMMIT_MESSAGE_TITLE);
    (title_row + 1..title_row + 4)
        .map(|row| screen_line(session, row))
        .collect()
}

fn wait_for_new_unversioned_file(session: &mut TerminalSession) {
    assert!(
        session.wait_for_text(UNVERSIONED_GROUP_LABEL, TEXT_TIMEOUT),
        "the refreshed list shows the new unversioned file, screen text was:\n{}",
        session.screen_text()
    );
}

fn type_message(session: &mut TerminalSession, message: &str) {
    session.send_keys("i");
    session.send_bytes(message.as_bytes());
    assert!(
        session.wait_for_text(message, TEXT_TIMEOUT),
        "the typed message reaches the message box, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn committed_message_matches_the_typed_text() {
    let repository = dirty_repository_copy("commit-wrapped-message");
    assert_eq!(
        WRAPPED_MESSAGE.chars().count(),
        60,
        "the wrapped message holds sixty characters"
    );
    let mut session = start_wide_shell(&repository);
    session.send_keys("ctrl+k");
    session.send_keys("i");
    session.send_bytes(WRAPPED_MESSAGE.as_bytes());
    assert!(
        session.wait_for_text("typed character shown", TEXT_TIMEOUT),
        "the wrapped tail reaches the message box, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "wrapped-message-box.txt");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(COMMIT_RECEIPT_PREFIX, TEXT_TIMEOUT),
        "the commit reports its receipt, screen text was:\n{}",
        session.screen_text()
    );
    let committed = git_output(&repository, &["log", "-1", "--format=%B"]);
    assert_eq!(
        committed.trim_end(),
        WRAPPED_MESSAGE,
        "the commit carries the typed message verbatim"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn typing_a_message_and_committing_adds_a_commit_to_the_log() {
    let repository = dirty_repository_copy("commit-message");
    let commits_before = commit_count(&repository);
    let mut session = start_wide_layout_shell(&repository);
    session.send_keys("ctrl+k");
    let button_row = commit_button_row(&mut session);
    session.send_keys("ctrl+enter");
    assert_eq!(
        commit_count(&repository),
        commits_before,
        "an empty message must not create a commit"
    );
    assert!(
        !session.screen_text().contains(REFRESH_FAILED_LABEL),
        "an empty message reports no error, screen text was:\n{}",
        session.screen_text()
    );
    type_message(&mut session, COMMIT_MESSAGE);
    dump_screen(&mut session, "commit-before.txt");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the changes list empties after the commit, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(commit_count(&repository), commits_before + 1);
    assert_eq!(head_subject(&repository), COMMIT_MESSAGE);
    let newest_row = log_row_text(&mut session, CHANGES_FIRST_ROW);
    assert!(
        newest_row.contains(COMMIT_MESSAGE),
        "the log shows the new commit on its first row, row {newest_row:?}"
    );
    assert_eq!(
        changes_position_of(&mut session, COMMIT_BUTTON_LABEL).1,
        button_row,
        "the commit button keeps its row"
    );
    dump_screen(&mut session, "commit-after.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn shift_typed_uppercase_letters_reach_the_message_box() {
    let repository = dirty_repository_copy("commit-uppercase");
    let mut session = start_wide_shell(&repository);
    session.send_keys("ctrl+k");
    session.send_keys("i");
    session.send_bytes(TYPED_UPPERCASE_MESSAGE.as_bytes());
    assert!(
        session.wait_for_text(TYPED_UPPERCASE_MESSAGE, TEXT_TIMEOUT),
        "the message box holds {TYPED_UPPERCASE_MESSAGE:?}, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "uppercase-message.txt");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the commit empties the changes list, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(head_subject(&repository), TYPED_UPPERCASE_MESSAGE);
    finish_with_ctrl_q(&mut session);
}

#[test]
fn empty_message_shows_a_status_message() {
    let repository = dirty_repository_copy("commit-empty-status");
    let mut session = start_wide_shell(&repository);
    session.send_keys("ctrl+k");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(EMPTY_MESSAGE_STATUS, TEXT_TIMEOUT),
        "the status bar reports the empty message, screen text was:\n{}",
        session.screen_text()
    );
    let status_row = u16::try_from(screen_lines(&mut session).len()).unwrap_or(0) - 1;
    let status_line = screen_line(&mut session, status_row);
    let label_column = status_line
        .find(EMPTY_MESSAGE_STATUS)
        .unwrap_or_else(|| panic!("the status row holds the message, row {status_line:?}"));
    assert!(
        label_column > usize::from(CHANGES_PANE_LEFT),
        "the message sits beside the branch label, row {status_line:?}"
    );
    dump_screen(&mut session, "empty-message-status.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn successful_commit_shows_a_receipt() {
    let repository = dirty_repository_copy("commit-receipt");
    let checked_file_count = changed_path_count(&repository);
    let mut session = start_wide_shell(&repository);
    session.send_keys("ctrl+k");
    type_message(&mut session, COMMIT_MESSAGE);
    session.send_keys("ctrl+enter");
    let receipt = format!("{COMMIT_RECEIPT_PREFIX}{checked_file_count} files to main");
    assert!(
        session.wait_for_text(&receipt, TEXT_TIMEOUT),
        "the status bar reports {receipt:?}, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "commit-receipt.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn amend_message_only_rewrites_head() {
    let repository = dirty_repository_copy("commit-amend-message-only");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", CLEAN_TREE_SUBJECT]);
    let commits_before = commit_count(&repository);
    let mut session = start_wide_shell(&repository);
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the cleaned work tree shows no changes, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    session.send_keys("alt+a");
    session.send_keys("i");
    session.send_bytes(&vec![BACKSPACE_KEY; CLEAN_TREE_SUBJECT.len()]);
    session.send_bytes(AMENDED_SUBJECT.as_bytes());
    assert!(
        session.wait_for_text(AMENDED_SUBJECT, TEXT_TIMEOUT),
        "the message box holds the amended message, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "amend-message-only-before.txt");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(AMENDED_RECEIPT, TEXT_TIMEOUT),
        "the message-only amend reports an amended commit receipt, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        commit_count(&repository),
        commits_before,
        "the message-only amend adds no commit"
    );
    assert_eq!(head_subject(&repository), AMENDED_SUBJECT);
    assert!(!session.screen_text().contains(REFRESH_FAILED_LABEL));
    dump_screen(&mut session, "amend-message-only-after.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn amend_rewrites_the_head_commit() {
    let repository = dirty_repository_copy("commit-amend");
    let commits_before = commit_count(&repository);
    let mut session = start_wide_shell(&repository);
    session.send_keys("ctrl+k");
    type_message(&mut session, "first commit");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the first commit empties the changes list, screen text was:\n{}",
        session.screen_text()
    );
    std::fs::write(repository.join("second.txt"), "second change\n")
        .expect("the second change is written");
    session.send_keys("f5");
    wait_for_new_unversioned_file(&mut session);
    type_message(&mut session, "second commit");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the second commit empties the changes list, screen text was:\n{}",
        session.screen_text()
    );
    let commits_after_second = commit_count(&repository);
    assert_eq!(commits_after_second, commits_before + 2);
    std::fs::write(repository.join("amended.txt"), "amended change\n")
        .expect("the amended change is written");
    session.send_keys("f5");
    wait_for_new_unversioned_file(&mut session);
    session.send_keys("alt+a");
    let prefilled_rows = message_box_rows(&mut session);
    assert!(
        prefilled_rows
            .iter()
            .any(|row| row.contains("second commit")),
        "amend prefills the last commit message, box rows were {prefilled_rows:?}"
    );
    session.send_keys("i");
    session.send_bytes(&vec![BACKSPACE_KEY; "second commit".len()]);
    session.send_bytes(AMENDED_SUBJECT.as_bytes());
    let amended_rows = message_box_rows(&mut session);
    assert!(
        amended_rows.iter().any(|row| row.contains(AMENDED_SUBJECT)),
        "the amended message replaces the prefilled one, box rows were {amended_rows:?}"
    );
    dump_screen(&mut session, "amend-before.txt");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the amended commit empties the changes list, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        commit_count(&repository),
        commits_after_second,
        "amend rewrites HEAD instead of adding a commit"
    );
    assert_eq!(head_subject(&repository), AMENDED_SUBJECT);
    let newest_row = log_row_text(&mut session, CHANGES_FIRST_ROW);
    assert!(
        newest_row.contains(AMENDED_SUBJECT),
        "the log shows the amended subject on its first row, row {newest_row:?}"
    );
    dump_screen(&mut session, "amend-after.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn sign_off_appends_the_trailer() {
    let repository = dirty_repository_copy("commit-sign-off");
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    session.send_keys("alt+s");
    type_message(&mut session, "signed commit");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the signed commit empties the changes list, screen text was:\n{}",
        session.screen_text()
    );
    let message = git_output(&repository, &["log", "-1", "--format=%B"]);
    assert_eq!(message.trim_end().lines().next(), Some("signed commit"));
    assert!(
        message.contains(SIGN_OFF_TRAILER),
        "the signed off commit carries the trailer, message {message:?}"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn commit_and_push_without_a_remote_shows_an_error() {
    let repository = dirty_repository_copy("commit-push");
    let commits_before = commit_count(&repository);
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    type_message(&mut session, "push attempt");
    let (push_column, push_row) = changes_position_of_either(
        &mut session,
        &[COMMIT_AND_PUSH_LABEL, COMMIT_AND_PUSH_COMPACT_LABEL],
    );
    session.mouse_click(push_column + 2, push_row);
    assert!(
        session.wait_for_text(PUSH_FAILED_LABEL, TEXT_TIMEOUT),
        "the failed push is reported on screen, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        commit_count(&repository),
        commits_before + 1,
        "the commit lands before the push fails"
    );
    assert_eq!(head_subject(&repository), "push attempt");
    assert!(
        !session.screen_text().contains(REFRESH_FAILED_LABEL),
        "the push failure keeps its own title, screen text was:\n{}",
        session.screen_text()
    );
    let reason_row = changes_position_of(&mut session, PUSH_FAILED_LABEL).1 + 1;
    let reason = screen_line(&mut session, reason_row);
    assert!(
        reason.contains("no remote"),
        "the push failure names the missing remote, row {reason:?}"
    );
    assert!(
        session.screen_text().contains("No changes"),
        "the committed files leave the list, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "commit-push-failed.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn gear_popover_toggles_sign_off() {
    let repository = dirty_repository_copy("commit-gear");
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    let divider = divider_column(&mut session);
    let row = commit_button_row(&mut session);
    session.mouse_click(divider - 2, row);
    assert!(
        session.wait_for_text(SIGN_OFF_LABEL, TEXT_TIMEOUT),
        "the gear opens the sign-off popover, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "gear-popover-open.txt");
    let (label_column, label_row) = changes_position_of(&mut session, SIGN_OFF_LABEL);
    session.mouse_click(label_column, label_row);
    assert!(
        !session.screen_text().contains(SIGN_OFF_LABEL),
        "clicking the sign-off row closes the popover, screen text was:\n{}",
        session.screen_text()
    );
    type_message(&mut session, "popped over commit");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the commit empties the changes list, screen text was:\n{}",
        session.screen_text()
    );
    let message = git_output(&repository, &["log", "-1", "--format=%B"]);
    assert!(
        message.contains(SIGN_OFF_TRAILER),
        "the popover sign-off reaches the commit, message {message:?}"
    );
    dump_screen(&mut session, "gear-popover-commit.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn escape_leaves_the_message_editor_without_quitting() {
    let repository = dirty_repository_copy("commit-escape");
    let mut session = start_shell(&repository);
    session.send_keys("ctrl+k");
    type_message(&mut session, "kept message");
    session.send_keys("esc");
    dump_screen(&mut session, "escape-leaves-editor.txt");
    assert_eq!(
        session.try_exit_code(),
        None,
        "escape leaves the message editor instead of quitting the shell"
    );
    assert_eq!(
        head_subject(&repository),
        "call run twice",
        "escape commits nothing"
    );
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text("No changes", TEXT_TIMEOUT),
        "the kept message still commits, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(head_subject(&repository), "kept message");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn staged_then_modified_shows_two_letters() {
    let repository = dirty_repository_copy("dual-state-staged-then-modified");
    git_output(&repository, &["reset", "--quiet", "--", "src/app.rs"]);
    git_output(&repository, &["add", "src/app.rs"]);
    std::fs::write(repository.join("src/app.rs"), "staged then modified\n")
        .expect("the worktree change is written");
    assert_eq!(
        git_output(&repository, &["status", "--porcelain", "src/app.rs"]).trim(),
        "MM src/app.rs",
        "the repository stages one change and keeps another in the worktree"
    );
    let mut session = start_shell(&repository);
    let row = changes_row_of(&mut session, "app.rs");
    let row_text = changes_row_text(&mut session, row);
    assert!(
        row_text.contains("MM app.rs"),
        "the staged and worktree letters share the status column, row was {row_text:?}"
    );
    dump_screen(&mut session, "dual-state-staged-then-modified.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn discard_a_change_from_the_ui() {
    for (columns, rows, label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "120x32"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let repository = dirty_repository_copy(&format!("discard-changes-{label}"));
        let mut session = start_shell(&repository);
        session.resize(columns, rows);
        assert!(
            session.wait_for_text("Log: HEAD", TEXT_TIMEOUT),
            "{label} the shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        let discard_row = changes_row_of(&mut session, "app.rs");
        session.mouse_click(FILE_ROW_CLICK_COLUMN, discard_row);
        session.send_keys("alt+d");
        assert!(
            session.wait_for_text(DISCARD_DIALOG_MESSAGE, TEXT_TIMEOUT),
            "{label} alt+d opens the discard confirmation, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.screen_text().contains(DISCARD_DIALOG_TITLE),
            "{label} the confirmation keeps its title, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.screen_text().contains(DISCARD_IRREVERSIBLE_LABEL),
            "{label} the confirmation warns that it cannot be undone, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.screen_text().contains(DISCARD_CONFIRM_LABEL)
                && session.screen_text().contains(DISCARD_CANCEL_LABEL),
            "{label} the confirmation offers both buttons, screen text was:\n{}",
            session.screen_text()
        );
        dump_screen(&mut session, &format!("discard-dialog-{label}.txt"));
        session.send_keys("enter");
        assert!(
            !session.screen_text().contains(DISCARD_DIALOG_MESSAGE),
            "{label} enter closes the confirmation because Cancel holds the focus, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.wait_for_text(DISCARD_CANCELLED_RECEIPT, TEXT_TIMEOUT),
            "{label} the status bar reports that the file stays, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            git_output(&repository, &["status", "--porcelain"]).contains(DISCARD_PATH),
            "{label} the cancelled confirmation keeps the change: {}",
            git_output(&repository, &["status", "--porcelain"])
        );
        session.send_keys("alt+d");
        assert!(
            session.wait_for_text(DISCARD_DIALOG_MESSAGE, TEXT_TIMEOUT),
            "{label} the confirmation opens again, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("esc");
        assert!(
            !session.screen_text().contains(DISCARD_DIALOG_MESSAGE),
            "{label} esc closes the confirmation, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            git_output(&repository, &["status", "--porcelain"]).contains(DISCARD_PATH),
            "{label} the escaped confirmation keeps the change: {}",
            git_output(&repository, &["status", "--porcelain"])
        );
        session.send_keys("alt+d");
        assert!(
            session.wait_for_text(DISCARD_DIALOG_MESSAGE, TEXT_TIMEOUT),
            "{label} the confirmation opens a third time, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("tab");
        session.send_keys("enter");
        assert!(
            session.wait_for_text(DISCARD_RECEIPT, TEXT_TIMEOUT),
            "{label} the status bar reports the discard, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            !session.screen_text().contains(DISCARD_DIALOG_MESSAGE),
            "{label} the confirmation closes after the discard, screen text was:\n{}",
            session.screen_text()
        );
        let restored = std::fs::read_to_string(repository.join(DISCARD_PATH))
            .expect("the discarded file is readable");
        assert!(
            restored.contains(DISCARD_RESTORED_LINE) && !restored.contains(DISCARD_DROPPED_LINE),
            "{label} the discarded file holds the head version again, file was {restored:?}"
        );
        let status = git_output(&repository, &["status", "--porcelain"]);
        assert!(
            !status.contains(DISCARD_PATH),
            "{label} git no longer reports the discarded file: {status}"
        );
        dump_screen(
            &mut session,
            &format!("discard-dialog-{label}-discarded.txt"),
        );
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn a_file_action_on_a_group_row_reports_the_hint() {
    for (columns, rows, label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "120x32"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let repository = dirty_repository_copy(&format!("group-row-hint-{label}"));
        let mut session = start_shell(&repository);
        session.resize(columns, rows);
        assert!(
            session.wait_for_text("Log: HEAD", TEXT_TIMEOUT),
            "{label} the shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("ctrl+k");
        session.send_keys("up");
        session.send_keys("alt+d");
        assert!(
            session.wait_for_text(SELECT_FILE_ROW_HINT, TEXT_TIMEOUT),
            "{label} alt+d on a group row reports the hint, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            !session.screen_text().contains(DISCARD_DIALOG_TITLE),
            "{label} alt+d on a group row opens no confirmation, screen text was:\n{}",
            session.screen_text()
        );
        dump_screen(&mut session, &format!("group-row-hint-{label}.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn merge_conflict_fixture_shows_the_merge_conflicts_group() {
    let repository = fixture_repository_copy(FixtureRepo::MergeConflict, "merge-conflicts-group");
    let mut session = start_shell(&repository);
    assert!(
        session.wait_for_text("Merge Conflicts", TEXT_TIMEOUT),
        "the conflicted fixture shows its merge conflicts group, screen text was:\n{}",
        session.screen_text()
    );
    let group_row = changes_row_of(&mut session, "Merge Conflicts");
    assert_eq!(
        group_row, CHANGES_FIRST_ROW,
        "the merge conflicts group sits above the changes group"
    );
    let group_text = changes_row_text(&mut session, group_row);
    assert!(
        group_text.contains("Merge Conflicts 1 file"),
        "the group header carries its count, row was {group_text:?}"
    );
    let conflict_row = changes_row_of(&mut session, "conflict.txt");
    let conflict_text = changes_row_text(&mut session, conflict_row);
    assert!(
        conflict_text.contains("UU conflict.txt"),
        "the conflicted row shows its two character code, row was {conflict_text:?}"
    );
    dump_screen(&mut session, "merge-conflicts-group.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn resolve_a_conflict_from_the_ui() {
    for (columns, rows, label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "120x32"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let repository = fixture_repository_copy(
            FixtureRepo::MergeConflict,
            &format!("resolve-conflict-{label}"),
        );
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text(MERGE_CONFLICTS_GROUP_LABEL, STARTUP_TIMEOUT),
            "{label} the fixture starts with the merge conflicts group, screen text was:\n{}",
            session.screen_text()
        );
        let conflict_row = CHANGES_FIRST_FILE_ROW;
        assert!(
            screen_line(&mut session, conflict_row).contains("conflict.txt"),
            "{label} the conflicted file row sits below its group header, row was {:?}",
            screen_line(&mut session, conflict_row)
        );
        session.mouse_click(FILE_ROW_CLICK_COLUMN, conflict_row);
        let status_row = rows - 1;
        assert!(
            screen_line(&mut session, status_row).contains(CONFLICT_RESOLUTION_HINT),
            "{label} the selected conflict shows the resolution hint, status row was {:?}",
            screen_line(&mut session, status_row)
        );
        dump_screen(
            &mut session,
            &format!("resolve-conflict-{label}-1-conflicted.txt"),
        );
        session.send_keys("alt+o");
        assert!(
            session.wait_for_text(CONFLICT_RESOLVED_RECEIPT, TEXT_TIMEOUT),
            "{label} the status bar reports the accepted side, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            !session.screen_text().contains(MERGE_CONFLICTS_GROUP_LABEL),
            "{label} the resolved file leaves the merge conflicts group, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            !screen_line(&mut session, status_row).contains(CONFLICT_RESOLUTION_HINT),
            "{label} the hint disappears once no conflicted file is selected, status row was {:?}",
            screen_line(&mut session, status_row)
        );
        dump_screen(
            &mut session,
            &format!("resolve-conflict-{label}-2-resolved.txt"),
        );
        let status = git_output(&repository, &["status", "--porcelain"]);
        assert!(
            !status.contains(UNMERGED_STATUS_PREFIX),
            "{label} git reports no unmerged path any more: {status}"
        );
        finish_with_ctrl_q(&mut session);
    }
}

fn repository_with_a_second_branch(name: &str) -> PathBuf {
    let repository = dirty_repository_copy(name);
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", CLEAN_TREE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", "-b", OTHER_BRANCH]);
    std::fs::write(
        repository.join("src/app.rs"),
        "fn main() {\n    run_on_the_other_branch();\n}\n",
    )
    .expect("the other branch change is written");
    git_output(
        &repository,
        &["commit", "-q", "-a", "-m", OTHER_BRANCH_SUBJECT],
    );
    git_output(&repository, &["checkout", "-q", CURRENT_BRANCH]);
    repository
}

#[test]
fn branch_switch_from_the_branches_rail() {
    let repository = repository_with_a_second_branch("branch-checkout");
    let mut session = start_wide_layout_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the main commit, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(CURRENT_BRANCH_CHIP),
        "the top bar starts on the current branch, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        log_row_text(&mut session, LOG_FIRST_ROW).contains(CLEAN_TREE_SUBJECT),
        "the log starts on the main commit, row was {:?}",
        log_row_text(&mut session, LOG_FIRST_ROW)
    );
    dump_screen(&mut session, "branch-checkout-before.txt");
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    let other_row = log_row_of(&mut session, OTHER_BRANCH);
    assert!(
        log_row_text(&mut session, other_row + 1).contains(CURRENT_BRANCH_MARKER),
        "the current branch is marked below the other branch, rows were {:?} and {:?}",
        log_row_text(&mut session, other_row),
        log_row_text(&mut session, other_row + 1)
    );
    session.send_keys("up");
    let branch_column = branch_selection_column(&mut session);
    assert_eq!(
        selection_row(&mut session, branch_column),
        Some(other_row),
        "the up key selects the other branch, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-checkout-list.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(CHECKED_OUT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the checkout, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text(OTHER_BRANCH_CHIP, TEXT_TIMEOUT),
        "the top bar chip follows the checkout, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(BRANCHES_TITLE),
        "a successful checkout keeps the branches view open, screen text was:\n{}",
        session.screen_text()
    );
    let checked_out_row = log_row_of(&mut session, OTHER_BRANCH);
    let checked_out_text = log_row_text(&mut session, checked_out_row);
    assert!(
        checked_out_text.contains(OTHER_BRANCH_MARKER),
        "the checkout moves the current branch marker, row was {checked_out_text:?}"
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(OTHER_BRANCH_SUBJECT_PREFIX, TEXT_TIMEOUT),
        "the log first row follows the checkout, screen text was:\n{}",
        session.screen_text()
    );
    let first_log_row = log_row_text(&mut session, LOG_FIRST_ROW);
    assert!(
        first_log_row.contains(OTHER_BRANCH_SUBJECT_PREFIX)
            && first_log_row.contains("HEAD -> feature/other"),
        "the log lists the other branch commit, row was {first_log_row:?}"
    );
    dump_screen(&mut session, "branch-checkout-after.txt");
    assert!(
        !session.screen_text().contains(CHECKOUT_FAILED_LABEL),
        "the checkout reports no failure, screen text was:\n{}",
        session.screen_text()
    );
    finish_with_ctrl_q(&mut session);
    assert_eq!(
        git_output(&repository, &["branch", "--show-current"]).trim(),
        OTHER_BRANCH,
        "the checked out branch is the repository head"
    );
    assert_eq!(head_subject(&repository), OTHER_BRANCH_SUBJECT);
}

#[test]
fn branches_rail_is_visible_at_medium_and_compact_sizes() {
    for (columns, rows, name) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "medium"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "compact"),
    ] {
        let repository = dirty_repository();
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "the {name} shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("b");
        assert!(
            session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
            "the b key opens the branches view on the {name} shell, screen text was:\n{}",
            session.screen_text()
        );
        assert_eq!(
            rail_letters(&mut session, columns, rows),
            "Branches",
            "the {name} shell stacks the rail letters"
        );
        dump_screen(
            &mut session,
            &format!("branches-rail-{name}-branches-view.txt"),
        );
        session.send_keys("b");
        assert!(
            !session.screen_text().contains(BRANCHES_TITLE),
            "the log view drops the branches title, screen text was:\n{}",
            session.screen_text()
        );
        assert_eq!(
            rail_letters(&mut session, columns, rows),
            "Branches",
            "the {name} shell keeps the rail while the log view is open"
        );
        dump_screen(&mut session, &format!("branches-rail-{name}-log-view.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn create_a_branch_from_the_ui() {
    let repository = repository_with_a_second_branch("branch-create-from-ui");
    let mut session = start_wide_layout_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the clean commit, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(CURRENT_BRANCH_CHIP),
        "the top bar starts on the current branch, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+n");
    assert!(
        session.wait_for_text(NEW_BRANCH_DIALOG_TITLE, TEXT_TIMEOUT),
        "alt+n opens the new branch dialog, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(NEW_BRANCH_INPUT_LABEL),
        "the dialog shows its input label, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-create-dialog.txt");
    session.send_keys("f e a t u r e / c r e a t e d");
    assert!(
        session.wait_for_text(
            &format!("{NEW_BRANCH_INPUT_LABEL}{CREATED_BRANCH}"),
            TEXT_TIMEOUT
        ),
        "the dialog echoes the typed name, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-create-typed.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(CREATED_BRANCH_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the created branch, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text(CREATED_BRANCH_CHIP, TEXT_TIMEOUT),
        "the top bar chip follows the new branch, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(NEW_BRANCH_DIALOG_TITLE),
        "the dialog closes once the branch exists, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-create-after.txt");
    finish_with_ctrl_q(&mut session);
    assert_eq!(
        git_output(&repository, &["branch", "--show-current"]).trim(),
        CREATED_BRANCH,
        "the created branch is checked out in the repository"
    );
}

#[test]
fn merge_a_selected_branch_from_the_ui() {
    let repository = repository_with_a_second_branch("branch-merge-selected-ui");
    let mut session = start_wide_layout_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the clean commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    let other_row = log_row_of(&mut session, OTHER_BRANCH);
    let column = branch_selection_column(&mut session);
    session.send_keys("up");
    assert!(
        wait_for_selection_row(&mut session, column, other_row, TEXT_TIMEOUT),
        "the up key selects the other branch row {other_row}, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-merge-selected-before.txt");
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(
            &format!("{FAST_FORWARD_RECEIPT_PREFIX}{OTHER_BRANCH}"),
            TEXT_TIMEOUT
        ),
        "the merge names the selected branch, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(ALREADY_UP_TO_DATE_STATUS),
        "the merge does not fall back to the current branch, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-merge-selected-after.txt");
    finish_with_ctrl_q(&mut session);
    assert_eq!(
        head_subject(&repository),
        OTHER_BRANCH_SUBJECT,
        "the merge moved the head onto the selected branch commit"
    );
    assert_eq!(
        git_output(&repository, &["branch", "--show-current"]).trim(),
        CURRENT_BRANCH,
        "the merge stays on the current branch"
    );
}

fn rail_letters(session: &mut TerminalSession, columns: u16, rows: u16) -> String {
    let column = divider_column(session) + DIVIDER_WIDTH;
    assert!(
        column < columns,
        "the log column fits the {columns} column screen"
    );
    for top in 2..rows {
        let first = session.row_text(top, column, column + 1);
        if first.trim() != "B" {
            continue;
        }
        let letters: String = (top..top + RAIL_LABEL_LENGTH)
            .map(|row| session.row_text(row, column, column + 1))
            .collect();
        if letters == BRANCHES_TITLE {
            return letters;
        }
    }
    panic!(
        "the rail is missing from column {column}, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn dirty_worktree_checkout_shows_the_error_block() {
    let repository = repository_with_a_second_branch("branch-checkout-dirty");
    std::fs::write(
        repository.join("src/app.rs"),
        "fn main() {\n    dirty_worktree();\n}\n",
    )
    .expect("the uncommitted change is written");
    let mut session = start_wide_shell(&repository);
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    let other_row = log_row_of(&mut session, OTHER_BRANCH);
    session.send_keys("up");
    let branch_column = branch_selection_column(&mut session);
    assert_eq!(
        selection_row(&mut session, branch_column),
        Some(other_row),
        "the up key selects the other branch, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("enter");
    assert!(
        session.wait_for_text(CHECKOUT_FAILED_LABEL, TEXT_TIMEOUT),
        "the dirty worktree blocks the checkout, screen text was:\n{}",
        session.screen_text()
    );
    let failure_row = changes_row_of(&mut session, CHECKOUT_FAILED_LABEL);
    let reason_rows: Vec<String> = (failure_row + 1..failure_row + 4)
        .map(|row| changes_row_text(&mut session, row))
        .collect();
    assert!(
        reason_rows
            .iter()
            .any(|row| row.contains(CHECKOUT_CONFLICT_STDERR_KEY)),
        "the error block keeps the git command text, rows were {reason_rows:?}"
    );
    assert!(
        session.screen_text().contains(BRANCHES_TITLE),
        "the failed checkout keeps the branches view open, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-checkout-dirty-failed.txt");
    finish_with_ctrl_q(&mut session);
    assert_eq!(
        git_output(&repository, &["branch", "--show-current"]).trim(),
        CURRENT_BRANCH,
        "the failed checkout leaves the repository head alone"
    );
}

fn bare_remote(name: &str) -> PathBuf {
    let target = artifacts_directory()
        .join("tmp")
        .join(E2E_REPOSITORY_DIRECTORY)
        .join(format!("{name}.git"));
    if target.exists() {
        std::fs::remove_dir_all(&target).expect("the stale bare remote is removed");
    }
    std::fs::create_dir_all(&target).expect("the bare remote directory is created");
    git_output(&target, &["init", "--bare", "--quiet"]);
    target
}

fn top_bar_position_of(session: &mut TerminalSession, needle: &str) -> (u16, u16) {
    let top_bar = screen_line(session, 0);
    let Some(byte_index) = top_bar.find(needle) else {
        panic!(
            "the top bar never shows {needle:?}, row was {top_bar:?}\n{}",
            session.screen_text()
        );
    };
    let column = top_bar[..byte_index].chars().count();
    (
        u16::try_from(column).expect("the needle starts inside the top bar"),
        0,
    )
}

fn add_origin_remote(repository: &Path, remote: &Path) {
    git_output(
        repository,
        &[
            "remote",
            "add",
            "origin",
            remote
                .to_str()
                .expect("the bare remote path is valid utf-8"),
        ],
    );
}

#[test]
fn a_deep_path_is_readable_in_a_real_terminal() {
    for (columns, rows) in EVIDENCE_SIZES {
        let repository = deep_path_repository_copy(
            &format!("deep-path-{columns}x{rows}"),
            &format!("{LONG_DETAILS_SUBJECT_HEAD} {LONG_DETAILS_SUBJECT_CONTINUATION}"),
        );
        let mut session = start_sized_shell(&repository, columns, rows);
        let row = deep_path_row(&mut session);
        let line = changes_row_text(&mut session, row);
        assert!(
            line.contains(DEEP_PATH_FILE_NAME),
            "{columns}x{rows} keeps the file name, row was {line:?}"
        );
        assert!(
            line.contains(DEEP_PATH_ELLIPSIS_MARKER),
            "{columns}x{rows} keeps the last directory level behind the ellipsis, row was {line:?}"
        );
        assert!(
            session.screen_text().contains("Unversioned Files"),
            "{columns}x{rows} keeps the unversioned group header, screen text was:\n{}",
            session.screen_text()
        );
        dump_screen(&mut session, &format!("deep-path-{columns}x{rows}.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn the_details_subject_is_wrapped_in_a_real_terminal() {
    let repository = deep_path_repository_copy(
        "details-wrap",
        &format!("{LONG_DETAILS_SUBJECT_HEAD} {LONG_DETAILS_SUBJECT_CONTINUATION}"),
    );
    let mut session = start_sized_shell(&repository, MEDIUM_COLUMNS, MEDIUM_ROWS);
    assert!(
        session.wait_for_text(LONG_DETAILS_SUBJECT_HEAD, TEXT_TIMEOUT),
        "the details pane shows the wrapped subject, screen text was:\n{}",
        session.screen_text()
    );
    let screen = session.screen_text();
    let subject_rows: Vec<&str> = screen
        .lines()
        .filter(|line| line.contains(LONG_DETAILS_SUBJECT_HEAD))
        .collect();
    assert_eq!(
        subject_rows.len(),
        1,
        "the full subject head only fits the details row, screen text was:\n{screen}"
    );
    assert!(
        !subject_rows[0].contains('…'),
        "the wrapped subject row is not truncated, row was {:?}",
        subject_rows[0]
    );
    assert!(
        screen.contains(LONG_DETAILS_SUBJECT_CONTINUATION),
        "the wrapped subject continues on the next row, screen text was:\n{screen}"
    );
    dump_screen(&mut session, "details-wrapped-subject.txt");
    finish_with_ctrl_q(&mut session);
}

fn commit_in_repository(repository: &Path, subject: &str) {
    git_output(
        repository,
        &["commit", "--allow-empty", "--quiet", "-m", subject],
    );
}

fn open_push_dialog_from_the_top_bar(session: &mut TerminalSession) {
    let (column, row) = top_bar_position_of(session, REMOTE_PUSH_LABEL);
    session.mouse_click(column, row);
    assert!(
        session.wait_for_text(PUSH_DIALOG_MESSAGE, TEXT_TIMEOUT),
        "the top bar push label opens the dialog, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn push_from_the_dialog_publishes_the_branch() {
    let repository = dirty_repository_copy("push-dialog");
    let remote = bare_remote("push-dialog");
    add_origin_remote(&repository, &remote);
    let mut session = start_shell(&repository);
    open_push_dialog_from_the_top_bar(&mut session);
    dump_screen(&mut session, "push-dialog-open.txt");
    assert!(
        !session.screen_text().contains(PUSH_FORCE_WITH_LEASE_LABEL)
            || session.screen_text().contains(PUSH_DIALOG_TITLE),
        "the dialog is the only overlay, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("enter");
    assert!(
        session.wait_for_text(PUSH_RECEIPT, TEXT_TIMEOUT),
        "the dialog reports the pushed branch, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(PUSH_FORCE_WITH_LEASE_LABEL),
        "the dialog closes after the push, screen text was:\n{}",
        session.screen_text()
    );
    let remote_branches = git_output(&remote, &["branch", "--list", CURRENT_BRANCH]);
    assert!(
        remote_branches.contains(CURRENT_BRANCH),
        "the bare remote holds the branch, branches were {remote_branches:?}"
    );
    assert_eq!(
        git_output(&remote, &["rev-parse", CURRENT_BRANCH]).trim(),
        git_output(&repository, &["rev-parse", CURRENT_BRANCH]).trim(),
        "the remote branch matches the local head"
    );
    dump_screen(&mut session, "push-dialog-pushed.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn push_dialog_reports_a_non_fast_forward_rejection() {
    let repository = dirty_repository_copy("push-rejected");
    let remote_side = dirty_repository_copy("push-rejected-remote-side");
    let remote = bare_remote("push-rejected");
    add_origin_remote(&repository, &remote);
    commit_in_repository(&remote_side, "remote advance");
    git_output(
        &remote_side,
        &[
            "push",
            remote
                .to_str()
                .expect("the bare remote path is valid utf-8"),
            CURRENT_BRANCH,
        ],
    );
    commit_in_repository(&repository, "local advance");
    let mut session = start_shell(&repository);
    open_push_dialog_from_the_top_bar(&mut session);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(PUSH_FAILED_LABEL, TEXT_TIMEOUT),
        "the rejected push lands in the error block, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(PUSH_DIALOG_MESSAGE),
        "the failed push closes the dialog, screen text was:\n{}",
        session.screen_text()
    );
    let failure_row = changes_position_of(&mut session, PUSH_FAILED_LABEL).1;
    let reason_rows: Vec<String> = (failure_row + 1..failure_row + ERROR_BODY_ROW_COUNT + 1)
        .map(|row| screen_line(&mut session, row))
        .collect();
    assert!(
        reason_rows.iter().any(|row| row.contains("rejected")
            || row.contains("non-fast-forward")
            || row.contains("fetch first")),
        "the error block keeps the git rejection text, rows were {reason_rows:?}"
    );
    assert!(
        !session.screen_text().contains(REFRESH_FAILED_LABEL),
        "the push rejection keeps its own title, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "push-dialog-rejected.txt");
    finish_with_ctrl_q(&mut session);
}

fn open_top_bar_dialog(session: &mut TerminalSession, label: &str, dialog_text: &str) {
    let (column, row) = top_bar_position_of(session, label);
    session.mouse_click(column, row);
    assert!(
        session.wait_for_text(dialog_text, TEXT_TIMEOUT),
        "clicking the {label} label opens its dialog, screen text was:\n{}",
        session.screen_text()
    );
}

fn published_repository(name: &str) -> (PathBuf, PathBuf) {
    let repository = dirty_repository_copy(name);
    let remote = bare_remote(name);
    add_origin_remote(&repository, &remote);
    git_output(&repository, &["add", "-A"]);
    git_output(
        &repository,
        &["commit", "--quiet", "-m", "publish the fixture"],
    );
    git_output(
        &repository,
        &[
            "push",
            "--quiet",
            "--set-upstream",
            "origin",
            CURRENT_BRANCH,
        ],
    );
    (repository, remote)
}

fn remote_side_copy(name: &str, source: &Path) -> PathBuf {
    let target = artifacts_directory()
        .join("tmp")
        .join(E2E_REPOSITORY_DIRECTORY)
        .join(format!("{name}-remote-side"));
    if target.exists() {
        std::fs::remove_dir_all(&target).expect("the stale remote side copy is removed");
    }
    copy_directory(source, &target).expect("the published repository is copied");
    target
}

fn push_from_the_remote_side(remote_side: &Path) {
    git_output(remote_side, &["push", "--quiet", "origin", CURRENT_BRANCH]);
}

fn dialog_rows(session: &mut TerminalSession, top_text: &str, height: u16) -> Vec<String> {
    let lines = screen_lines(session);
    let top_row = lines
        .iter()
        .position(|line| line.contains(top_text))
        .unwrap_or_else(|| {
            panic!(
                "the screen never shows {top_text:?}, screen text was:\n{}",
                session.screen_text()
            )
        });
    lines
        .into_iter()
        .skip(top_row.saturating_sub(1))
        .take(usize::from(height))
        .collect()
}

#[test]
fn remote_dialogs_fit_a_medium_and_a_compact_terminal() {
    let (repository, _remote) = published_repository("dialog-sizes");
    for (columns, rows, size) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "120x32"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let mut session = start_shell(&repository);
        session.resize(columns, rows);
        assert!(
            session.wait_for_text(REMOTE_PULL_LABEL, TEXT_TIMEOUT),
            "the {size} screen shows the remote tool bar, screen text was:\n{}",
            session.screen_text()
        );
        open_top_bar_dialog(&mut session, REMOTE_PULL_LABEL, PULL_DIALOG_MESSAGE);
        let pull_rows = dialog_rows(&mut session, PULL_DIALOG_MESSAGE, PULL_DIALOG_ROW_COUNT);
        assert_eq!(
            pull_rows.len(),
            usize::from(PULL_DIALOG_ROW_COUNT),
            "{size} the pull dialog keeps every row, rows were {pull_rows:?}"
        );
        assert!(
            pull_rows
                .iter()
                .any(|row| row.contains(PULL_FAST_FORWARD_ONLY_LABEL)),
            "{size} the pull dialog keeps its strategy rows, rows were {pull_rows:?}"
        );
        assert!(
            pull_rows.iter().any(|row| row.contains(PULL_CONFIRM_LABEL)),
            "{size} the pull dialog keeps its buttons, rows were {pull_rows:?}"
        );
        dump_screen(&mut session, &format!("pull-dialog-{size}.txt"));
        session.send_keys("esc");
        open_top_bar_dialog(&mut session, REMOTE_FETCH_LABEL, FETCH_DIALOG_MESSAGE);
        let fetch_rows = dialog_rows(&mut session, FETCH_DIALOG_MESSAGE, FETCH_DIALOG_ROW_COUNT);
        assert_eq!(
            fetch_rows.len(),
            usize::from(FETCH_DIALOG_ROW_COUNT),
            "{size} the fetch dialog keeps every row, rows were {fetch_rows:?}"
        );
        assert!(
            fetch_rows
                .iter()
                .any(|row| row.contains(FETCH_CONFIRM_LABEL)),
            "{size} the fetch dialog keeps its buttons, rows were {fetch_rows:?}"
        );
        dump_screen(&mut session, &format!("fetch-dialog-{size}.txt"));
        session.send_keys("esc");
        assert_eq!(
            session.try_exit_code(),
            None,
            "the shell stays alive after closing the {size} dialogs"
        );
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn pull_fast_forward_from_the_dialog_advances_the_branch() {
    let (repository, remote) = published_repository("pull-fast-forward");
    let remote_side = remote_side_copy("pull-fast-forward", &repository);
    commit_in_repository(&remote_side, "remote advance");
    push_from_the_remote_side(&remote_side);
    let commits_before = commit_count(&repository);
    let mut session = start_shell(&repository);
    open_top_bar_dialog(&mut session, REMOTE_PULL_LABEL, PULL_DIALOG_MESSAGE);
    assert!(
        session.screen_text().contains(PULL_FAST_FORWARD_ONLY_LABEL),
        "the pull dialog offers the fast-forward strategy, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "pull-dialog-open.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(PULL_RECEIPT, TEXT_TIMEOUT),
        "the dialog reports the pulled branch, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(PULL_FAST_FORWARD_ONLY_LABEL),
        "the dialog closes after the pull, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        commit_count(&repository),
        commits_before + 1,
        "the fast-forward pull takes the remote commit"
    );
    assert_eq!(
        git_output(&repository, &["branch", "--show-current"]).trim(),
        CURRENT_BRANCH,
        "the pull keeps the branch checked out"
    );
    assert_eq!(
        git_output(&repository, &["rev-parse", "HEAD"]).trim(),
        git_output(&remote, &["rev-parse", CURRENT_BRANCH]).trim(),
        "the local head matches the remote head after the fast-forward"
    );
    dump_screen(&mut session, "pull-dialog-fast-forward.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn pull_rebase_from_the_dialog_replays_local_commits() {
    let (repository, _remote) = published_repository("pull-rebase");
    let remote_side = remote_side_copy("pull-rebase", &repository);
    commit_in_repository(&repository, "local work");
    commit_in_repository(&remote_side, "remote advance");
    push_from_the_remote_side(&remote_side);
    let mut session = start_shell(&repository);
    open_top_bar_dialog(&mut session, REMOTE_PULL_LABEL, PULL_DIALOG_MESSAGE);
    session.send_keys("down");
    session.send_keys("down");
    dump_screen(&mut session, "pull-dialog-rebase.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(PULL_RECEIPT, TEXT_TIMEOUT),
        "the rebasing pull reports its receipt, screen text was:\n{}",
        session.screen_text()
    );
    let subjects = git_output(&repository, &["log", "--format=%s", "-n", "3"]);
    assert!(
        subjects.contains("local work") && subjects.contains("remote advance"),
        "the rebase keeps both commits, subjects were {subjects:?}"
    );
    assert_eq!(
        git_output(&repository, &["log", "--format=%s", "-n", "1"]).trim(),
        "local work",
        "the rebase replays the local commit on top"
    );
    assert_eq!(
        git_output(&repository, &["log", "--format=%s", "-n", "1", "HEAD~1"]).trim(),
        "remote advance",
        "the rebase puts the remote commit underneath the local one"
    );
    assert_eq!(
        git_output(&repository, &["log", "--format=%p", "-n", "1"])
            .split_whitespace()
            .count(),
        1,
        "the rebase leaves no merge commit behind"
    );
    dump_screen(&mut session, "pull-dialog-rebase.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn fetch_from_the_dialog_updates_the_behind_count() {
    let (repository, remote) = published_repository("fetch-dialog");
    let remote_side = remote_side_copy("fetch-dialog", &repository);
    commit_in_repository(&remote_side, "remote advance");
    push_from_the_remote_side(&remote_side);
    let mut session = start_shell(&repository);
    assert!(
        session.screen_text().contains(REMOTE_TRACKING_LABEL),
        "the stale remote-tracking label sits on the local head, screen text was:\n{}",
        session.screen_text()
    );
    open_top_bar_dialog(&mut session, REMOTE_FETCH_LABEL, FETCH_DIALOG_MESSAGE);
    dump_screen(&mut session, "fetch-dialog-open.txt");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(FETCH_RECEIPT, TEXT_TIMEOUT),
        "the fetch dialog reports its receipt, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        git_output(&repository, &["rev-parse", REMOTE_TRACKING_LABEL]).trim(),
        git_output(&remote, &["rev-parse", CURRENT_BRANCH]).trim(),
        "the fetch moves the remote-tracking ref"
    );
    assert_eq!(
        git_output(&repository, &["rev-list", "--count", "HEAD..origin/main"]).trim(),
        "1",
        "the fetch makes the repository one commit behind"
    );
    let head_row_text = log_row_text(&mut session, LOG_FIRST_ROW);
    assert!(
        !head_row_text.contains(REMOTE_TRACKING_LABEL),
        "the refreshed log drops the stale remote label from the local head, log row was {head_row_text:?}"
    );
    dump_screen(&mut session, "fetch-dialog-fetched.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn pull_conflict_reports_the_error_block() {
    let (repository, _remote) = published_repository("pull-conflict");
    let remote_side = remote_side_copy("pull-conflict", &repository);
    std::fs::write(repository.join(PULL_CONFLICT_FILE), "local side\n")
        .expect("the local conflict file is written");
    git_output(&repository, &["add", PULL_CONFLICT_FILE]);
    git_output(
        &repository,
        &["commit", "--quiet", "-m", "local conflict change"],
    );
    std::fs::write(remote_side.join(PULL_CONFLICT_FILE), "remote side\n")
        .expect("the remote conflict file is written");
    git_output(&remote_side, &["add", PULL_CONFLICT_FILE]);
    git_output(
        &remote_side,
        &["commit", "--quiet", "-m", "remote conflict change"],
    );
    push_from_the_remote_side(&remote_side);
    let mut session = start_shell(&repository);
    open_top_bar_dialog(&mut session, REMOTE_PULL_LABEL, PULL_DIALOG_MESSAGE);
    session.send_keys("down");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(PULL_FAILED_LABEL, TEXT_TIMEOUT),
        "the conflicting pull lands in the error block, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(PULL_FAST_FORWARD_ONLY_LABEL),
        "the failed pull closes the dialog, screen text was:\n{}",
        session.screen_text()
    );
    let failure_row = changes_position_of(&mut session, PULL_FAILED_LABEL).1;
    let reason_rows: Vec<String> = (failure_row + 1..failure_row + ERROR_BODY_ROW_COUNT + 1)
        .map(|row| screen_line(&mut session, row))
        .collect();
    assert!(
        reason_rows
            .iter()
            .any(|row| row.to_lowercase().contains("conflict")),
        "the error block keeps the git conflict text, rows were {reason_rows:?}"
    );
    assert!(
        !session.screen_text().contains(FETCH_FAILED_LABEL),
        "the pull conflict keeps its own title, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !git_output(&repository, &["ls-files", "--unmerged"])
            .trim()
            .is_empty(),
        "the repository keeps the unmerged paths the pull reported"
    );
    dump_screen(&mut session, "pull-dialog-conflict.txt");
    finish_with_ctrl_q(&mut session);
}

fn hunk_staging_repository(name: &str) -> PathBuf {
    let repository = artifacts_directory()
        .join("tmp")
        .join(E2E_REPOSITORY_DIRECTORY)
        .join(name);
    if repository.exists() {
        std::fs::remove_dir_all(&repository).expect("the stale repository is removed");
    }
    std::fs::create_dir_all(&repository).expect("the repository directory is created");
    git_output(&repository, &["init", "-b", CURRENT_BRANCH]);
    git_output(&repository, &["config", "user.name", "gitdeck-fixture"]);
    git_output(
        &repository,
        &["config", "user.email", "fixture@gitdeck.invalid"],
    );
    std::fs::write(repository.join(NOTES_PATH), numbered_lines(&[], LINE_BREAK))
        .expect("the notes file is written");
    std::fs::write(
        repository.join(SECOND_PATH),
        numbered_lines(&[], LINE_BREAK),
    )
    .expect("the second file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", HUNK_BASE_SUBJECT]);
    std::fs::write(
        repository.join(NOTES_PATH),
        numbered_lines(&[FIRST_HUNK_LINE, SECOND_HUNK_LINE], LINE_BREAK),
    )
    .expect("the changed notes are written");
    std::fs::write(
        repository.join(SECOND_PATH),
        numbered_lines(&[SECOND_FILE_CHANGED_LINE], LINE_BREAK),
    )
    .expect("the changed second file is written");
    repository
}

fn modified_and_staged_repository(name: &str) -> PathBuf {
    let repository = artifacts_directory()
        .join("tmp")
        .join(E2E_REPOSITORY_DIRECTORY)
        .join(name);
    if repository.exists() {
        std::fs::remove_dir_all(&repository).expect("the stale repository is removed");
    }
    std::fs::create_dir_all(&repository).expect("the repository directory is created");
    git_output(&repository, &["init", "-b", CURRENT_BRANCH]);
    git_output(&repository, &["config", "user.name", "gitdeck-fixture"]);
    git_output(
        &repository,
        &["config", "user.email", "fixture@gitdeck.invalid"],
    );
    std::fs::write(repository.join(NOTES_PATH), numbered_lines(&[], LINE_BREAK))
        .expect("the notes file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", HUNK_BASE_SUBJECT]);
    std::fs::write(
        repository.join(NOTES_PATH),
        numbered_lines(&[STAGED_CHANGED_LINE], LINE_BREAK),
    )
    .expect("the staged notes are written");
    git_output(&repository, &["add", "--", NOTES_PATH]);
    std::fs::write(
        repository.join(NOTES_PATH),
        numbered_lines(
            &[STAGED_CHANGED_LINE, FIRST_HUNK_LINE, SECOND_HUNK_LINE],
            LINE_BREAK,
        ),
    )
    .expect("the modified notes are written");
    repository
}

fn tabbed_repository(name: &str) -> PathBuf {
    let repository = artifacts_directory()
        .join("tmp")
        .join(E2E_REPOSITORY_DIRECTORY)
        .join(name);
    if repository.exists() {
        std::fs::remove_dir_all(&repository).expect("the stale repository is removed");
    }
    std::fs::create_dir_all(&repository).expect("the repository directory is created");
    git_output(&repository, &["init", "-b", CURRENT_BRANCH]);
    git_output(&repository, &["config", "user.name", "gitdeck-fixture"]);
    git_output(
        &repository,
        &["config", "user.email", "fixture@gitdeck.invalid"],
    );
    std::fs::write(repository.join(TABS_PATH), "a\tb\nc\n").expect("the tabbed base is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", TABS_BASE_SUBJECT]);
    std::fs::write(repository.join(TABS_PATH), "a\tB\nC\n").expect("the tabbed change is written");
    repository
}

fn numbered_lines(changed_lines: &[usize], line_break: &str) -> String {
    (1..=HUNK_FILE_LINE_COUNT)
        .map(|line_number| {
            if changed_lines.contains(&line_number) {
                format!("line {line_number:02} changed{line_break}")
            } else {
                format!("line {line_number:02}{line_break}")
            }
        })
        .collect()
}

fn select_changes_file(session: &mut TerminalSession, needle: &str) {
    session.send_keys("ctrl+k");
    for _ in 0..CHANGES_SELECTION_SCAN_LIMIT {
        session.send_keys("k");
    }
    for _ in 0..CHANGES_SELECTION_SCAN_LIMIT {
        let row = selection_row(session, CHANGES_PANE_LEFT + 2).unwrap_or(BODY_ROW);
        if changes_row_text(session, row).contains(needle) {
            return;
        }
        session.send_keys("j");
    }
    panic!(
        "the changes cursor never reached {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn space_after_opening_a_diff_checks_the_first_hunk() {
    let repository = hunk_staging_repository("diff-focus-hunk");
    let mut session = start_wide_shell(&repository);
    open_notes_diff(&mut session);
    let changes_row = changes_row_of(&mut session, NOTES_PATH);
    let changes_row_before = screen_line(&mut session, changes_row);
    session.send_keys("space");
    assert!(
        session.wait_for_text(CHECKED_HUNK_LABEL, TEXT_TIMEOUT),
        "the first hunk takes the check mark, screen text was:\n{}",
        session.screen_text()
    );
    let row = hunk_header_row(&mut session);
    assert!(
        screen_line(&mut session, row).contains(CHECKED_HUNK_LABEL),
        "the hunk header carries the check mark, row {:?}",
        screen_line(&mut session, row)
    );
    assert_eq!(
        screen_line(&mut session, changes_row),
        changes_row_before,
        "the space key leaves the file row alone"
    );
    dump_screen(&mut session, "diff-focus-hunk-checked.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn b_keeps_the_log_list_while_a_diff_tab_shows() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    open_app_diff(&mut session);
    session.send_keys("b");
    session.send_keys("p");
    session.send_keys("f");
    assert!(
        session.screen_text().contains(APP_FILE_TITLE),
        "the single letters leave the diff tab alone, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+w");
    assert!(
        !session.screen_text().contains(BRANCHES_TITLE),
        "the log tab still lists the commits, screen text was:\n{}",
        session.screen_text()
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn i_focuses_the_commit_message_from_the_log_pane() {
    let repository = dirty_repository();
    let mut session = start_shell(&repository);
    session.send_keys("alt+9");
    type_message(&mut session, MESSAGE_FROM_THE_LOG_PANE);
    assert!(
        session.screen_text().contains(MESSAGE_FROM_THE_LOG_PANE),
        "the message typed from the log pane reaches the message box, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "message-from-log-pane.txt");
    session.send_keys("ctrl+q");
    assert!(
        session.wait_for_text(UNSAVED_MESSAGE_QUIT_TEXT, TEXT_TIMEOUT),
        "the shell asks before dropping the typed message, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("tab");
    session.send_keys("enter");
    assert_eq!(
        session.wait_for_exit(EXIT_TIMEOUT),
        Some(0),
        "the confirmed quit leaves the shell with the success code"
    );
    let pending_message = git_output(&repository, &["log", "-1", "--format=%B"]);
    assert!(
        !pending_message.contains(MESSAGE_FROM_THE_LOG_PANE),
        "the typed message stays uncommitted, message {pending_message:?}"
    );
}

fn open_app_diff(session: &mut TerminalSession) {
    session.send_keys("ctrl+k");
    let row = changes_row_of(session, "app.rs");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
}

fn check_the_first_hunk(session: &mut TerminalSession) -> String {
    session.send_keys("alt+9");
    session.send_keys("space");
    assert!(
        session.wait_for_text(CHECKED_HUNK_LABEL, TEXT_TIMEOUT),
        "the hunk header shows its checked marker, screen text was:\n{}",
        session.screen_text()
    );
    let row = hunk_header_row(session);
    screen_line(session, row)
}

fn hunk_header_row(session: &mut TerminalSession) -> u16 {
    for row in 1..DIFF_FIRST_ROW + DIFF_CAPTURED_ROW_COUNT {
        if log_row_text(session, row).contains(UNCHECKED_HUNK_LABEL)
            || log_row_text(session, row).contains(CHECKED_HUNK_LABEL)
        {
            return row;
        }
    }
    panic!(
        "no hunk header row is visible, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn commit_only_the_selected_hunk() {
    let repository = hunk_staging_repository("hunk-only-selection");
    let mut session = start_wide_shell(&repository);
    open_notes_diff(&mut session);
    select_changes_file(&mut session, SECOND_PATH);
    session.send_keys("space");
    type_message(&mut session, HUNK_COMMIT_MESSAGE);
    let header = check_the_first_hunk(&mut session);
    assert!(
        header.contains(CHECKED_HUNK_LABEL),
        "the first hunk header carries the checkbox, row {header:?}"
    );
    dump_screen(&mut session, "hunk-only-selection-checked.txt");
    session.send_keys("ctrl+k");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(COMMITTED_PREFIX, TEXT_TIMEOUT),
        "the commit reports its receipt, screen text was:\n{}",
        session.screen_text()
    );
    let stat = git_output(&repository, &["show", "--stat", "--format=", "HEAD"]);
    assert_eq!(
        stat.matches('|').count(),
        1,
        "the commit holds one file, stat was:\n{stat}"
    );
    assert!(stat.contains(NOTES_PATH), "stat was:\n{stat}");
    let head_content = git_output(&repository, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(
        head_content.contains("line 02 changed"),
        "the checked hunk reaches the commit:\n{head_content}"
    );
    assert!(
        !head_content.contains("line 19 changed"),
        "the unchecked hunk stays out of the commit:\n{head_content}"
    );
    let worktree_diff = git_output(&repository, &["diff", "HEAD", "--", NOTES_PATH]);
    assert!(
        worktree_diff.contains("-line 19") && worktree_diff.contains("+line 19 changed"),
        "the second hunk survives in the work tree:\n{worktree_diff}"
    );
    assert!(
        !worktree_diff.contains("line 02"),
        "the committed hunk leaves the work tree diff:\n{worktree_diff}"
    );
    let untouched = git_output(&repository, &["diff", "HEAD", "--", SECOND_PATH]);
    assert!(
        untouched.contains("+line 03 changed"),
        "the unchecked file keeps its change in the work tree:\n{untouched}"
    );
    assert_eq!(
        git_output(&repository, &["diff", "--cached"]).trim(),
        "",
        "the index is empty after the commit"
    );
    dump_screen(&mut session, "hunk-only-selection-committed.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn hunk_selection_and_file_selection_combine() {
    let repository = hunk_staging_repository("hunk-and-file");
    let mut session = start_wide_shell(&repository);
    open_notes_diff(&mut session);
    type_message(&mut session, HUNK_COMMIT_MESSAGE);
    check_the_first_hunk(&mut session);
    session.send_keys("ctrl+k");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(COMMITTED_PREFIX, TEXT_TIMEOUT),
        "the commit reports its receipt, screen text was:\n{}",
        session.screen_text()
    );
    let stat = git_output(&repository, &["show", "--stat", "--format=", "HEAD"]);
    assert_eq!(
        stat.matches('|').count(),
        2,
        "the commit holds the hunk file and the whole file, stat was:\n{stat}"
    );
    let second_content = git_output(&repository, &["show", &format!("HEAD:{SECOND_PATH}")]);
    assert!(
        second_content.contains("line 03 changed"),
        "the whole second file reaches the commit:\n{second_content}"
    );
    let notes_content = git_output(&repository, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(notes_content.contains("line 02 changed"));
    assert!(
        !notes_content.contains("line 19 changed"),
        "the unchecked hunk stays out of the commit:\n{notes_content}"
    );
    assert_eq!(
        git_output(&repository, &["diff", "--cached"]).trim(),
        "",
        "the index is empty after the commit"
    );
    dump_screen(&mut session, "hunk-and-file-committed.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn pre_staged_content_stays_out_of_a_hunk_commit() {
    let repository = hunk_staging_repository("hunk-pre-staged-index");
    git_output(&repository, &["add", "--", SECOND_PATH]);
    let commits_before = commit_count(&repository);
    let mut session = start_wide_shell(&repository);
    open_notes_diff(&mut session);
    select_changes_file(&mut session, SECOND_PATH);
    session.send_keys("space");
    type_message(&mut session, HUNK_COMMIT_MESSAGE);
    check_the_first_hunk(&mut session);
    session.send_keys("ctrl+k");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(COMMITTED_PREFIX, TEXT_TIMEOUT),
        "the hunk commit reports its receipt, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        commit_count(&repository),
        commits_before + 1,
        "the hunk commit adds one commit"
    );
    let committed_paths = git_output(&repository, &["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(
        committed_paths.trim(),
        NOTES_PATH,
        "the commit holds the hunk file only, stat was:\n{committed_paths}"
    );
    let notes_content = git_output(&repository, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(
        notes_content.contains("line 02 changed"),
        "the selected hunk reaches the commit:\n{notes_content}"
    );
    assert!(
        !notes_content.contains("line 19 changed"),
        "the unchecked hunk stays out of the commit:\n{notes_content}"
    );
    let staged_diff = git_output(&repository, &["diff", "--cached", "--name-only"]);
    assert_eq!(
        staged_diff.trim(),
        SECOND_PATH,
        "the index keeps only the content that was staged before"
    );
    dump_screen(&mut session, "hunk-pre-staged-index.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn hunk_commit_on_a_modified_staged_file_succeeds() {
    let repository = modified_and_staged_repository("hunk-modified-staged-file");
    let mut session = start_wide_shell(&repository);
    open_notes_diff(&mut session);
    type_message(&mut session, HUNK_COMMIT_MESSAGE);
    check_the_first_hunk(&mut session);
    session.send_keys("ctrl+k");
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(COMMITTED_PREFIX, TEXT_TIMEOUT),
        "the staged-and-modified file commits its hunk, screen text was:\n{}",
        session.screen_text()
    );
    let stat = git_output(&repository, &["show", "--stat", "--format=", "HEAD"]);
    assert_eq!(
        stat.matches('|').count(),
        1,
        "the commit holds one file, stat was:\n{stat}"
    );
    assert!(stat.contains(NOTES_PATH), "stat was:\n{stat}");
    let head_content = git_output(&repository, &["show", &format!("HEAD:{NOTES_PATH}")]);
    assert!(
        head_content.contains("line 03 changed"),
        "the staged part of the file reaches the commit:\n{head_content}"
    );
    assert!(
        head_content.contains("line 02 changed"),
        "the checked hunk reaches the commit:\n{head_content}"
    );
    assert!(
        !head_content.contains("line 19 changed"),
        "the unchecked hunk stays out of the commit:\n{head_content}"
    );
    let worktree_diff = git_output(&repository, &["diff", "--", NOTES_PATH]);
    assert!(
        worktree_diff.contains("+line 19 changed"),
        "the unchecked hunk survives in the work tree:\n{worktree_diff}"
    );
    dump_screen(&mut session, "hunk-modified-staged-file-committed.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn hunk_checkbox_is_visible_at_medium_and_compact_sizes() {
    let repository = hunk_staging_repository("hunk-checkbox-sizes");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the medium shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    open_notes_diff(&mut session);
    session.send_keys("alt+9");
    let unchecked_row = hunk_header_row(&mut session);
    assert!(
        log_row_text(&mut session, unchecked_row).contains(UNCHECKED_HUNK_LABEL),
        "the unchecked hunk header fits 120 columns, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "hunk-checkbox-medium-unchecked.txt");
    session.send_keys("space");
    assert!(
        log_row_text(&mut session, unchecked_row).contains(CHECKED_HUNK_LABEL),
        "the checked hunk header fits 120 columns, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "hunk-checkbox-medium-checked.txt");
    session.resize(COMPACT_COLUMNS, COMPACT_ROWS);
    assert_every_row_fills_the_columns(&mut session, COMPACT_COLUMNS);
    let compact_row = hunk_header_row(&mut session);
    assert!(
        log_row_text(&mut session, compact_row).contains(CHECKED_HUNK_LABEL),
        "the checked hunk header survives 80 columns, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "hunk-checkbox-compact-checked.txt");
    finish_with_ctrl_q(&mut session);
}

fn open_notes_diff(session: &mut TerminalSession) {
    select_changes_file(session, NOTES_PATH);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(NOTES_TITLE, TEXT_TIMEOUT),
        "the working tree diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
}

fn filter_bar_row_text(session: &mut TerminalSession, columns: u16) -> String {
    session
        .row_text(FILTER_BAR_ROW, 0, columns)
        .trim()
        .to_string()
}

fn filter_caret_columns(session: &mut TerminalSession, columns: u16) -> Vec<u16> {
    (0..columns)
        .filter(|column| session.cell(*column, FILTER_BAR_ROW).background == LOG_TEXT_COLOR)
        .collect()
}

fn dump_filter_state(session: &mut TerminalSession, columns: u16, name: &str) {
    let directory = artifacts_directory().join("e2e");
    std::fs::create_dir_all(&directory).expect("the e2e screen directory is created");
    let screen = session.screen_text();
    let caret_line = format!(
        "filter caret columns: {:?}\n",
        filter_caret_columns(session, columns)
    );
    std::fs::write(directory.join(name), format!("{screen}\n{caret_line}"))
        .expect("the e2e screen dump is written");
}

#[test]
fn text_filter_narrows_the_log_on_screen() {
    let repository = fixture_repository_copy(FixtureRepo::LargeTenThousand, "log-filter-text");
    let scan_window_argument = format!("--max-count={FILTER_SCAN_WINDOW_COMMITS}");
    let scanned_subjects = git_output(
        &repository,
        &["log", scan_window_argument.as_str(), "--format=%s", "HEAD"],
    );
    let scanned_matches = scanned_subjects
        .lines()
        .filter(|subject| subject.contains(FILTER_QUERY))
        .count();
    assert_eq!(
        scanned_matches, FILTERED_COMMIT_COUNT,
        "git counts the commits the query matches inside the scan window"
    );
    let history_matches = git_output(
        &repository,
        &["rev-list", "--count", "HEAD", "--grep=side commit"],
    );
    assert_eq!(
        history_matches.trim(),
        MATCHES_BEYOND_THE_SCAN_WINDOW.to_string(),
        "the whole history holds more matches than the scan window reaches"
    );
    for (columns, rows, size_label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "medium"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "compact"),
    ] {
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "{size_label} shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("alt+9");
        assert!(
            session.wait_for_text(UNFILTERED_NEIGHBOUR_SUBJECT, TEXT_TIMEOUT),
            "{size_label} log shows the newest commits, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            filter_bar_row_text(&mut session, columns).contains(FILTER_PLACEHOLDER_LABEL),
            "{size_label} idle filter bar shows the placeholder, got {:?}",
            filter_bar_row_text(&mut session, columns)
        );
        dump_filter_state(
            &mut session,
            columns,
            &format!("log-filter-{size_label}-1-empty.txt"),
        );
        session.send_bytes(b"/");
        session.send_bytes(FILTER_QUERY.as_bytes());
        assert!(
            session.wait_for_text(FILTER_QUERY, TEXT_TIMEOUT),
            "{size_label} filter bar holds the typed query, screen text was:\n{}",
            session.screen_text()
        );
        let caret_columns = filter_caret_columns(&mut session, columns);
        assert_eq!(
            caret_columns.len(),
            1,
            "{size_label} the filter row draws one caret, got {caret_columns:?}"
        );
        let caret = caret_columns[0];
        let query_width = u16::try_from(FILTER_QUERY.chars().count()).expect("the query width");
        assert_eq!(
            session.row_text(FILTER_BAR_ROW, caret - query_width, caret),
            FILTER_QUERY,
            "{size_label} the caret sits right after the typed query"
        );
        assert_eq!(
            session.row_text(FILTER_BAR_ROW, caret + 1, columns).trim(),
            "",
            "{size_label} nothing follows the caret"
        );
        dump_filter_state(
            &mut session,
            columns,
            &format!("log-filter-{size_label}-2-editing.txt"),
        );
        session.send_keys("enter");
        assert!(
            session.wait_for_text(FILTERED_STATUS_LABEL, TEXT_TIMEOUT),
            "{size_label} status bar reports the applied filter, screen text was:\n{}",
            session.screen_text()
        );
        let first_row = log_row_text(&mut session, LOG_FIRST_ROW);
        assert!(
            first_row.contains(FILTERED_SUBJECT_FRAGMENT),
            "{size_label} the first log row holds a matching subject, got {first_row:?}"
        );
        assert!(
            !first_row.contains(UNFILTERED_NEIGHBOUR_SUBJECT),
            "{size_label} the first log row drops the unfiltered commits, got {first_row:?}"
        );
        assert!(
            !session.screen_text().contains(UNFILTERED_NEIGHBOUR_SUBJECT),
            "{size_label} the unfiltered commits leave the screen"
        );
        let status_row = rows - 1;
        assert!(
            session
                .row_text(status_row, 0, columns)
                .contains(FILTERED_STATUS_LABEL),
            "{size_label} status bar reports {FILTERED_STATUS_LABEL:?}, got {:?}",
            session.row_text(status_row, 0, columns)
        );
        dump_filter_state(
            &mut session,
            columns,
            &format!("log-filter-{size_label}-3-applied.txt"),
        );
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn scrolling_past_the_first_page_appends_more_commits() {
    let repository = fixture_repository_copy(FixtureRepo::LargeTenThousand, "log-filter-paging");
    let log_subjects = git_output(&repository, &["log", "--format=%s", "HEAD"]);
    let second_page_position = log_subjects
        .lines()
        .position(|subject| subject == SECOND_PAGE_SUBJECT)
        .expect("the fixture holds the second page subject");
    assert_eq!(
        second_page_position, SECOND_PAGE_SUBJECT_POSITION,
        "git places {SECOND_PAGE_SUBJECT:?} behind the first page"
    );
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+9");
    assert!(
        session.wait_for_text(UNFILTERED_NEIGHBOUR_SUBJECT, TEXT_TIMEOUT),
        "the log shows the newest commits, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(SECOND_PAGE_SUBJECT),
        "the first page stops before {SECOND_PAGE_SUBJECT:?}"
    );
    session.send_bytes(END_KEY);
    session.send_bytes(END_KEY);
    assert!(
        session.wait_for_text(SECOND_PAGE_SUBJECT, TEXT_TIMEOUT),
        "the appended page shows {SECOND_PAGE_SUBJECT:?}, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !log_row_text(&mut session, LOG_FIRST_ROW).contains(UNFILTERED_NEIGHBOUR_SUBJECT),
        "the viewport leaves the newest commits behind, got {:?}",
        log_row_text(&mut session, LOG_FIRST_ROW)
    );
    dump_screen(&mut session, "log-filter-second-page.txt");
    let oldest_visible_row = (LOG_FIRST_ROW..MEDIUM_ROWS - 1)
        .find(|row| log_row_text(&mut session, *row).contains(SECOND_PAGE_SUBJECT))
        .expect("the second page subject sits on the screen");
    assert!(
        oldest_visible_row > LOG_FIRST_ROW,
        "the second page subject is not the first row"
    );
    finish_with_ctrl_q(&mut session);
}

fn filter_row_cells(session: &mut TerminalSession, columns: u16) -> Vec<String> {
    (0..columns)
        .map(|column| session.cell(column, FILTER_BAR_ROW).symbol)
        .collect()
}

fn log_subject_rows(session: &mut TerminalSession) -> Vec<String> {
    (LOG_FIRST_ROW..WIDE_ROWS - 2)
        .map(|row| log_row_text(session, row))
        .filter(|row| row.contains("gitdeck-fixture"))
        .collect()
}

fn branch_chip_column(session: &mut TerminalSession, columns: u16) -> u16 {
    let cells = filter_row_cells(session, columns);
    let label: Vec<String> = BRANCH_FILTER_CHIP_LABEL
        .chars()
        .map(|character| character.to_string())
        .collect();
    let label_length = u16::try_from(label.len()).expect("the chip label width");
    (0..columns.saturating_sub(label_length))
        .find(|column| {
            let start = usize::from(*column);
            cells.get(start..start + label.len()) == Some(label.as_slice())
        })
        .unwrap_or_else(|| {
            panic!(
                "the filter row shows no branch chip, row was {:?}",
                cells.concat()
            )
        })
}

fn apply_branch_filter_from_the_chip(session: &mut TerminalSession, columns: u16) {
    let chip_column = branch_chip_column(session, columns);
    session.mouse_click(chip_column, FILTER_BAR_ROW);
    assert!(
        session.wait_for_text(BRANCH_FILTER_HEAD_ROW_LABEL, TEXT_TIMEOUT),
        "the chip opens a dropdown with the HEAD row, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text(OTHER_BRANCH, TEXT_TIMEOUT),
        "the dropdown lists the other branch, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("down");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(BRANCH_FILTER_APPLIED_CHIP_LABEL, TEXT_TIMEOUT),
        "the chip follows the applied branch, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn branch_filter_narrows_the_log_to_that_branch() {
    let repository = repository_with_a_second_branch("branch-filter-narrow");
    let newest_on_the_branch = git_output(
        &repository,
        &["log", OTHER_BRANCH, "--format=%s", "--max-count=1"],
    );
    assert_eq!(newest_on_the_branch.trim(), OTHER_BRANCH_SUBJECT);
    assert!(
        newest_on_the_branch
            .trim()
            .starts_with(OTHER_BRANCH_SUBJECT_PREFIX),
        "git reports the branch subject the screen shows"
    );
    let main_commit_count = git_output(&repository, &["rev-list", "--count", CURRENT_BRANCH]);
    let branch_commit_count = git_output(&repository, &["rev-list", "--count", OTHER_BRANCH]);
    let mut session = start_wide_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the main commit, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        log_row_text(&mut session, LOG_FIRST_ROW).contains(CLEAN_TREE_SUBJECT),
        "the first row holds the main subject, row was {:?}",
        log_row_text(&mut session, LOG_FIRST_ROW)
    );
    let unfiltered_rows = log_subject_rows(&mut session);
    assert_eq!(
        unfiltered_rows.len(),
        main_commit_count
            .trim()
            .parse::<usize>()
            .expect("the main commit count"),
        "the log lists every commit of the checked out branch, rows were {unfiltered_rows:?}"
    );
    apply_branch_filter_from_the_chip(&mut session, WIDE_COLUMNS);
    let filtered_rows = log_subject_rows(&mut session);
    assert_eq!(
        filtered_rows.len(),
        branch_commit_count
            .trim()
            .parse::<usize>()
            .expect("the branch commit count"),
        "the log lists exactly the commits of the filtered branch, rows were {filtered_rows:?}"
    );
    assert!(
        filtered_rows[0].contains(OTHER_BRANCH_SUBJECT_PREFIX),
        "the first row holds the branch commit, row was {:?}",
        filtered_rows[0]
    );
    assert!(
        !filtered_rows[0].contains(CLEAN_TREE_SUBJECT),
        "the main commit leaves the first row, row was {:?}",
        filtered_rows[0]
    );
    dump_screen(&mut session, "branch-filter-wide-applied.txt");
    finish_with_ctrl_q(&mut session);
    assert!(
        session.screen_text().is_empty(),
        "the shell leaves the alternate screen behind"
    );
    for (columns, rows, size_label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "medium"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "compact"),
    ] {
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "{size_label} shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            log_row_text(&mut session, LOG_FIRST_ROW).contains(CURRENT_BRANCH),
            "{size_label} log starts on the main branch, row was {:?}",
            log_row_text(&mut session, LOG_FIRST_ROW)
        );
        assert!(
            !log_row_text(&mut session, LOG_FIRST_ROW).contains(HEAD_REF_PREFIX_LABEL),
            "{size_label} the compact row drops the head prefix, row was {:?}",
            log_row_text(&mut session, LOG_FIRST_ROW)
        );
        let chip_column = branch_chip_column(&mut session, columns);
        session.mouse_click(chip_column, FILTER_BAR_ROW);
        assert!(
            session.wait_for_text(OTHER_BRANCH, TEXT_TIMEOUT),
            "{size_label} dropdown lists {OTHER_BRANCH:?}, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.wait_for_text(BRANCH_FILTER_HEAD_ROW_LABEL, TEXT_TIMEOUT),
            "{size_label} dropdown starts on the HEAD row, screen text was:\n{}",
            session.screen_text()
        );
        dump_screen(
            &mut session,
            &format!("branch-filter-{size_label}-1-dropdown.txt"),
        );
        session.send_keys("down");
        session.send_keys("enter");
        assert!(
            session.wait_for_text(BRANCH_FILTER_APPLIED_CHIP_LABEL, TEXT_TIMEOUT),
            "{size_label} chip shows {BRANCH_FILTER_APPLIED_CHIP_LABEL:?}, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            log_row_text(&mut session, LOG_FIRST_ROW).contains(BRANCH_REF_PREFIX),
            "{size_label} the log first row belongs to the filtered branch, row was {:?}",
            log_row_text(&mut session, LOG_FIRST_ROW)
        );
        dump_screen(
            &mut session,
            &format!("branch-filter-{size_label}-2-applied.txt"),
        );
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn branch_filter_with_text_filter_narrows_further() {
    let repository = repository_with_a_second_branch("branch-filter-combined");
    let matching_commits = git_output(
        &repository,
        &[
            "rev-list",
            "--count",
            OTHER_BRANCH,
            "--grep=clean work tree",
        ],
    );
    assert_eq!(matching_commits.trim(), "1");
    let mut session = start_wide_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the main commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+9");
    apply_branch_filter_from_the_chip(&mut session, WIDE_COLUMNS);
    assert!(
        log_row_text(&mut session, LOG_FIRST_ROW).contains(OTHER_BRANCH_SUBJECT_PREFIX),
        "the branch filter alone keeps the branch commit on screen"
    );
    session.send_bytes(b"/");
    session.send_bytes(BRANCH_FILTER_TEXT_QUERY.as_bytes());
    session.send_keys("enter");
    assert!(
        session.wait_for_text(BRANCH_FILTER_COMBINED_STATUS, TEXT_TIMEOUT),
        "the status bar counts the commits that pass both filters, screen text was:\n{}",
        session.screen_text()
    );
    let first_row = log_row_text(&mut session, LOG_FIRST_ROW);
    assert!(
        first_row.contains(CLEAN_TREE_SUBJECT),
        "the text filter narrows the branch log, row was {first_row:?}"
    );
    assert!(
        !session.screen_text().contains(OTHER_BRANCH_SUBJECT_PREFIX),
        "the branch commit no longer matches the text filter, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-filter-3-combined.txt");
    finish_with_ctrl_q(&mut session);
}

fn diff_row_of(session: &mut TerminalSession, needle: &str) -> u16 {
    for row in DIFF_FIRST_ROW..DIFF_FIRST_ROW + DIFF_ROW_SCAN_LIMIT {
        if session.row_text(row, 0, WIDE_COLUMNS).contains(needle) {
            return row;
        }
    }
    panic!(
        "the diff pane never shows {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

fn column_of(session: &mut TerminalSession, row: u16, needle: &str) -> u16 {
    let line = session.row_text(row, 0, WIDE_COLUMNS);
    let byte_index = line.find(needle).unwrap_or_else(|| {
        panic!("the diff row {row} does not hold {needle:?}, the row was {line:?}")
    });
    let column = line[..byte_index].chars().count();
    u16::try_from(column).expect("the needle starts inside the row")
}

#[test]
fn word_highlights_appear_in_a_real_terminal() {
    let repository = dirty_repository_copy("word-highlight");
    git_output(&repository, &["add", "-A"]);
    git_output(&repository, &["commit", "-qm", "word highlight base"]);
    std::fs::write(
        repository.join("src/app.rs"),
        "fn main() {\n    run_once();\n    report();\n    shutdown();\n}\n",
    )
    .expect("the single line change is written");
    let mut session = TerminalSession::spawn(Some(&repository), WIDE_COLUMNS, WIDE_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, "app.rs");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    let added_row = diff_row_of(&mut session, WORD_HIGHLIGHT_ADDED_LINE);
    let removed_row = diff_row_of(&mut session, WORD_HIGHLIGHT_REMOVED_LINE);
    let added_column = column_of(&mut session, added_row, WORD_HIGHLIGHT_ADDED_LINE)
        + WORD_HIGHLIGHT_CHANGED_ADDED_OFFSET;
    let removed_column = column_of(&mut session, removed_row, WORD_HIGHLIGHT_REMOVED_LINE)
        + WORD_HIGHLIGHT_CHANGED_REMOVED_OFFSET;
    assert_eq!(
        session.cell(added_column, added_row).background,
        ADDED_WORD_HIGHLIGHT_BACKGROUND,
        "the changed token of the added line is highlighted"
    );
    assert_eq!(
        session.cell(added_column + 1, added_row).background,
        ADDED_WORD_HIGHLIGHT_BACKGROUND,
        "the second changed character of the added line is highlighted"
    );
    assert_eq!(
        session.cell(added_column + 2, added_row).background,
        ADDED_LINE_BACKGROUND_COLOR,
        "the shared tail of the added line keeps the line background"
    );
    assert_eq!(
        session.cell(removed_column, removed_row).background,
        REMOVED_WORD_HIGHLIGHT_BACKGROUND,
        "the changed token of the removed line is highlighted"
    );
    assert_eq!(
        session.cell(removed_column + 3, removed_row).background,
        REMOVED_LINE_BACKGROUND_COLOR,
        "the shared tail of the removed line keeps the line background"
    );
    dump_screen(&mut session, "word-highlight-160x48.txt");
    session.send_keys("alt+w");
    assert!(
        session.wait_for_text(WORD_HIGHLIGHTS_OFF_STATUS, TEXT_TIMEOUT),
        "the toggle reports its state, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        session.cell(added_column, added_row).background,
        ADDED_LINE_BACKGROUND_COLOR,
        "the added line falls back to its line background"
    );
    assert_eq!(
        session.cell(removed_column, removed_row).background,
        REMOVED_LINE_BACKGROUND_COLOR,
        "the removed line falls back to its line background"
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn details_panel_shows_the_subject_and_file_count_in_a_real_terminal() {
    let repository = dirty_repository_copy("details-block");
    for (columns, rows) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS),
        (WIDE_LAYOUT_COLUMNS, WIDE_LAYOUT_ROWS),
    ] {
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "the shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        let details_column = details_column_start(&mut session, columns) + 1;
        let subject_row =
            details_row_of(&mut session, details_column, columns, rows, DETAILS_SUBJECT);
        let count_row = details_row_of(
            &mut session,
            details_column,
            columns,
            rows,
            DETAILS_FILE_COUNT,
        );
        assert!(
            count_row < subject_row,
            "the file count sits above the subject, {count_row} vs {subject_row}"
        );
        let separator_row = subject_row - 1;
        let separator_text = session.row_text(separator_row, details_column, columns);
        assert!(
            separator_text.starts_with(DETAILS_SEPARATOR.repeat(4).as_str()),
            "the block opens with a separator, row {separator_text:?}"
        );
        let details_lines = (subject_row..rows - 1)
            .map(|row| {
                session
                    .row_text(row, details_column, columns)
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<String>>();
        println!(
            "details pane {columns}x{rows}:\n{}",
            details_lines.join("\n")
        );
        dump_screen(&mut session, &format!("details-block-{columns}x{rows}.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn details_panel_shows_the_branch_count_in_a_real_terminal() {
    let repository = dirty_repository_copy("details-branches");
    git_output(&repository, &["branch", "alpha"]);
    git_output(&repository, &["branch", "beta"]);
    git_output(
        &repository,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let head_short_id = git_output(&repository, &["rev-parse", "--short=7", "HEAD"]);
    let containing_output = git_output(
        &repository,
        &["branch", "-a", "--contains", head_short_id.trim()],
    );
    let containing_count = containing_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    assert!(
        containing_count >= 4,
        "the scratch repository holds several containing branches, git said:\n{containing_output}"
    );
    let expected_label = format!("In {containing_count} branches: ");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text(expected_label.as_str(), TEXT_TIMEOUT),
        "the details block reports the containing branch count, screen text was:\n{}",
        session.screen_text()
    );
    let details_column = details_column_start(&mut session, MEDIUM_COLUMNS) + 1;
    let branches_row = details_row_of(
        &mut session,
        details_column,
        MEDIUM_COLUMNS,
        MEDIUM_ROWS,
        expected_label.as_str(),
    );
    let subject_row = details_row_of(
        &mut session,
        details_column,
        MEDIUM_COLUMNS,
        MEDIUM_ROWS,
        DETAILS_SUBJECT,
    );
    assert!(
        branches_row > subject_row,
        "the branches row sits under the subject, {branches_row} vs {subject_row}"
    );
    let details_lines = (subject_row..MEDIUM_ROWS - 1)
        .map(|row| {
            session
                .row_text(row, details_column, MEDIUM_COLUMNS)
                .trim_end()
                .to_string()
        })
        .collect::<Vec<String>>();
    println!(
        "details pane {MEDIUM_COLUMNS}x{MEDIUM_ROWS}:\n{}",
        details_lines.join("\n")
    );
    dump_screen(&mut session, "details-branches-120x32.txt");
    finish_with_ctrl_q(&mut session);
}

fn details_column_start(session: &mut TerminalSession, columns: u16) -> u16 {
    let body = session.row_text(BODY_ROW, 0, columns);
    body.chars()
        .enumerate()
        .filter(|(_, character)| *character == DIVIDER_GLYPH)
        .map(|(column, _)| u16::try_from(column).unwrap_or(u16::MAX))
        .last()
        .unwrap_or_else(|| panic!("the body row {body:?} carries no details divider"))
}

fn details_row_of(
    session: &mut TerminalSession,
    details_column: u16,
    columns: u16,
    rows: u16,
    needle: &str,
) -> u16 {
    for row in LOG_FIRST_ROW..rows - 1 {
        if session
            .row_text(row, details_column, columns)
            .contains(needle)
        {
            return row;
        }
    }
    panic!(
        "the details pane never shows {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

const MERGE_SOURCE_BRANCH: &str = "feature/local";
const MERGE_MAIN_PATH: &str = "src/merge_main.rs";
const MERGE_SOURCE_PATH: &str = "src/merge_feature.rs";
const MERGE_CONFLICT_PATH: &str = "src/merge_conflict.rs";
const MERGE_SOURCE_SUBJECT: &str = "branch side change";
const MERGE_TARGET_SUBJECT: &str = "main side change";
const MERGED_RECEIPT: &str = "Merged feature/local into main";
const MERGE_CONFLICT_RECEIPT: &str = "Merge of feature/local stopped with conflicts";
const MERGE_FAILED_LABEL: &str = "Merge failed";
const MERGING_LABEL: &str = "MERGING";
const MERGE_CONFLICT_RECEIPT_PATH: &str = "Accepted ours for src/merge_conflict.rs";
const MERGE_PREFILLED_MESSAGE: &str = "Merge branch 'feature/local'";
const MERGE_COMMIT_RECEIPT: &str = "main merged and committed";
const PARENT_COUNT_TWO: usize = 2;
const MERGE_DEFAULT_SUBJECT: &str = "Merge branch 'feature/local'";
const MERGE_EXTRA_PATH: &str = "src/merge_extra.rs";
const MERGE_EXTRA_NEEDLE: &str = "merge_extra.rs";
const UNCHECKED_CHANGE_MARKER: &str = "[ ]";

fn repository_with_a_diverged_branch(name: &str) -> PathBuf {
    let repository = dirty_repository_copy(name);
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", CLEAN_TREE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", "-b", MERGE_SOURCE_BRANCH]);
    std::fs::write(repository.join(MERGE_SOURCE_PATH), "fn feature() {}\n")
        .expect("the branch side file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", MERGE_SOURCE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", CURRENT_BRANCH]);
    std::fs::write(repository.join(MERGE_MAIN_PATH), "fn current() {}\n")
        .expect("the main side file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", MERGE_TARGET_SUBJECT]);
    repository
}

fn repository_with_a_conflicting_branch(name: &str) -> PathBuf {
    let repository = dirty_repository_copy(name);
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", CLEAN_TREE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", "-b", MERGE_SOURCE_BRANCH]);
    std::fs::write(repository.join(MERGE_CONFLICT_PATH), "fn branch() {}\n")
        .expect("the branch side file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", MERGE_SOURCE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", CURRENT_BRANCH]);
    std::fs::write(repository.join(MERGE_CONFLICT_PATH), "fn current() {}\n")
        .expect("the main side file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", MERGE_TARGET_SUBJECT]);
    repository
}

fn repository_with_a_conflict_and_a_branch_only_change(name: &str) -> PathBuf {
    let repository = dirty_repository_copy(name);
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", CLEAN_TREE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", "-b", MERGE_SOURCE_BRANCH]);
    std::fs::write(repository.join(MERGE_CONFLICT_PATH), "fn branch() {}\n")
        .expect("the branch side conflict file is written");
    std::fs::write(repository.join(MERGE_EXTRA_PATH), "fn extra() {}\n")
        .expect("the branch only file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", MERGE_SOURCE_SUBJECT]);
    git_output(&repository, &["checkout", "-q", CURRENT_BRANCH]);
    std::fs::write(repository.join(MERGE_CONFLICT_PATH), "fn current() {}\n")
        .expect("the main side conflict file is written");
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", MERGE_TARGET_SUBJECT]);
    repository
}

fn select_branch_row(session: &mut TerminalSession, branch: &str) {
    let target = log_row_of(session, branch);
    let column = branch_selection_column(session);
    let current = selection_row(session, column).unwrap_or_else(|| {
        panic!(
            "a branch row is selected, screen text was:\n{}",
            session.screen_text()
        )
    });
    let key = if current > target { "up" } else { "down" };
    for _ in 0..current.abs_diff(target) {
        session.send_keys(key);
    }
    assert_eq!(
        selection_row(session, column),
        Some(target),
        "the selection sits on {branch}, screen text was:\n{}",
        session.screen_text()
    );
}

fn wait_for_text_absence(session: &mut TerminalSession, needle: &str, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if !session.screen_text().contains(needle) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn head_parent_count(repository: &Path) -> usize {
    git_output(repository, &["log", "-1", "--format=%P"])
        .split_whitespace()
        .count()
}

#[test]
fn merge_from_the_branches_view_creates_a_merge_commit() {
    let repository = repository_with_a_diverged_branch("merge-from-branches");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(MERGE_TARGET_SUBJECT, STARTUP_TIMEOUT),
        "the session starts on the merge target commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, MERGE_SOURCE_BRANCH);
    dump_screen(&mut session, "merge-branches-before-120x32.txt");
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(MERGED_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the merge, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(MERGING_LABEL),
        "a finished merge leaves no merging marker, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        head_parent_count(&repository),
        PARENT_COUNT_TWO,
        "the merge commit has two parents"
    );
    assert!(
        git_output(&repository, &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "the merge leaves a clean work tree"
    );
    assert_eq!(head_subject(&repository), MERGE_DEFAULT_SUBJECT);
    dump_screen(&mut session, "merge-branches-after-120x32.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn conflicted_merge_from_the_ui_can_be_resolved_and_committed() {
    let repository = repository_with_a_conflicting_branch("merge-conflict-from-ui");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(MERGE_TARGET_SUBJECT, STARTUP_TIMEOUT),
        "the session starts on the merge target commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, MERGE_SOURCE_BRANCH);
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(MERGE_CONFLICT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the stopped merge, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(MERGING_LABEL),
        "the footer reports the merge in progress, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(MERGE_CONFLICTS_GROUP_LABEL),
        "the conflicted file lands in the merge conflicts group, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(MERGE_FAILED_LABEL),
        "the conflict block is titled Merge failed, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "merge-conflict-120x32-conflicted.txt");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, CHANGES_FIRST_FILE_ROW);
    session.send_keys("alt+o");
    assert!(
        session.wait_for_text(MERGE_CONFLICT_RECEIPT_PATH, TEXT_TIMEOUT),
        "the status bar reports the accepted side, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    assert!(
        session.wait_for_text(MERGE_PREFILLED_MESSAGE, TEXT_TIMEOUT),
        "the message box carries the prefilled merge message, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(MERGE_COMMIT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the concluded merge, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        wait_for_text_absence(&mut session, MERGING_LABEL, TEXT_TIMEOUT),
        "the merging marker disappears after the merge commit, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        head_parent_count(&repository),
        PARENT_COUNT_TWO,
        "the concluding commit has two parents"
    );
    assert_eq!(head_subject(&repository), MERGE_PREFILLED_MESSAGE);
    let status = git_output(&repository, &["status", "--porcelain"]);
    assert!(
        !status.contains(UNMERGED_STATUS_PREFIX),
        "no unmerged path is left: {status}"
    );
    dump_screen(&mut session, "merge-conflict-120x32-committed.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn merging_with_one_path_unchecked_still_commits_the_merge() {
    let repository = repository_with_a_conflict_and_a_branch_only_change("merge-unchecked-path");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(MERGE_TARGET_SUBJECT, STARTUP_TIMEOUT),
        "the session starts on the merge target commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, MERGE_SOURCE_BRANCH);
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(MERGE_CONFLICT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the stopped merge, screen text was:\n{}",
        session.screen_text()
    );
    select_changes_file(&mut session, MERGE_EXTRA_NEEDLE);
    session.send_keys("space");
    let unchecked_row = changes_row_of(&mut session, MERGE_EXTRA_NEEDLE);
    let unchecked_text = changes_row_text(&mut session, unchecked_row);
    assert!(
        unchecked_text.contains(UNCHECKED_CHANGE_MARKER),
        "the merge brought path is unchecked, row was {unchecked_text:?}"
    );
    dump_screen(&mut session, "merge-unchecked-120x32-unchecked.txt");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, CHANGES_FIRST_FILE_ROW);
    session.send_keys("alt+t");
    let accepted_theirs = format!("Accepted theirs for {MERGE_CONFLICT_PATH}");
    assert!(
        session.wait_for_text(&accepted_theirs, TEXT_TIMEOUT),
        "the status bar reports the accepted side, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    assert!(
        session.wait_for_text(MERGE_PREFILLED_MESSAGE, TEXT_TIMEOUT),
        "the message box carries the prefilled merge message, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+enter");
    assert!(
        session.wait_for_text(MERGE_COMMIT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the concluded merge, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        wait_for_text_absence(&mut session, MERGING_LABEL, TEXT_TIMEOUT),
        "the merging marker disappears after the merge commit, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        head_parent_count(&repository),
        PARENT_COUNT_TWO,
        "the concluding commit has two parents"
    );
    let committed_paths = git_output(
        &repository,
        &["show", "--name-only", "--first-parent", "--format=", "HEAD"],
    );
    assert!(
        committed_paths.contains(MERGE_CONFLICT_PATH),
        "the resolved conflict reaches the merge commit, paths were:\n{committed_paths}"
    );
    assert!(
        committed_paths.contains(MERGE_EXTRA_PATH),
        "the unchecked path still reaches the merge commit, paths were:\n{committed_paths}"
    );
    let status = git_output(&repository, &["status", "--porcelain"]);
    assert!(
        status.trim().is_empty(),
        "the concluded merge leaves no staged change behind: {status}"
    );
    dump_screen(&mut session, "merge-unchecked-120x32-committed.txt");
    finish_with_ctrl_q(&mut session);
}

const MERGE_ABORTED_RECEIPT: &str = "Aborted the merge";
const REBASE_ABORTED_RECEIPT: &str = "Aborted the rebase";
const REBASING_LABEL: &str = "REBASING";

fn git_output_expecting_failure(repository_path: &Path, arguments: &[&str]) -> String {
    let output = fixture_git_command(repository_path)
        .args(arguments)
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        !output.status.success(),
        "git {arguments:?} was expected to stop in {}",
        repository_path.display()
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn repository_stopped_in_a_rebase(name: &str) -> PathBuf {
    let repository = repository_with_a_conflicting_branch(name);
    git_output(&repository, &["checkout", "-q", MERGE_SOURCE_BRANCH]);
    git_output_expecting_failure(&repository, &["rebase", CURRENT_BRANCH]);
    assert!(
        repository.join(".git").join("rebase-merge").exists(),
        "the rebase stops with conflicts"
    );
    assert!(
        std::fs::read_to_string(repository.join(MERGE_CONFLICT_PATH))
            .expect("the conflicted file reads")
            .contains("<<<<<<<"),
        "the rebase leaves conflict markers"
    );
    repository
}

#[test]
fn the_danger_guards_show_their_prompts_in_a_real_terminal() {
    let repository = repository_with_a_conflicting_branch("danger-guard-prompts");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(MERGE_TARGET_SUBJECT, STARTUP_TIMEOUT),
        "the session starts on the merge target commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, MERGE_SOURCE_BRANCH);
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(MERGE_CONFLICT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the stopped merge, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+x");
    assert!(
        session.wait_for_text(MERGE_ABORT_PROMPT, TEXT_TIMEOUT),
        "the abort prompt appears before the merge is discarded, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(MERGING_LABEL),
        "the merge survives the unanswered prompt, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "abort-prompt-120x32.txt");
    session.send_keys("esc");
    assert!(
        wait_for_text_absence(&mut session, MERGE_ABORT_PROMPT, TEXT_TIMEOUT),
        "escape closes the abort prompt, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+q");
    assert!(
        session.wait_for_text(UNSAVED_MESSAGE_QUIT_TEXT, TEXT_TIMEOUT),
        "the quit prompt names the prefilled merge message, screen text was:\n{}",
        session.screen_text()
    );
    assert_eq!(
        session.try_exit_code(),
        None,
        "the shell stays open while the quit prompt waits"
    );
    dump_screen(&mut session, "quit-prompt-120x32.txt");
    session.send_keys("esc");
    assert!(
        wait_for_text_absence(&mut session, UNSAVED_MESSAGE_QUIT_TEXT, TEXT_TIMEOUT),
        "escape closes the quit prompt, screen text was:\n{}",
        session.screen_text()
    );
    finish_with_ctrl_q(&mut session);
}

#[test]
fn merge_conflict_can_be_aborted_from_the_ui() {
    let repository = repository_with_a_conflicting_branch("merge-abort-from-ui");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(MERGE_TARGET_SUBJECT, STARTUP_TIMEOUT),
        "the session starts on the merge target commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, MERGE_SOURCE_BRANCH);
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(MERGE_CONFLICT_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the stopped merge, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(MERGING_LABEL),
        "the footer reports the merge in progress, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "merge-abort-120x32-conflicted.txt");
    session.send_keys("alt+x");
    assert!(
        session.wait_for_text(MERGE_ABORT_PROMPT, TEXT_TIMEOUT),
        "the abort asks before discarding the merge, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("tab");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(MERGE_ABORTED_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the aborted merge, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        wait_for_text_absence(&mut session, MERGING_LABEL, TEXT_TIMEOUT),
        "the merging marker disappears after the abort, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        git_output(&repository, &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "the aborted merge leaves a clean work tree"
    );
    assert_eq!(head_subject(&repository), MERGE_TARGET_SUBJECT);
    dump_screen(&mut session, "merge-abort-120x32-aborted.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn rebase_conflict_shows_the_state_and_can_be_aborted() {
    let repository = repository_stopped_in_a_rebase("rebase-abort-from-ui");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(REBASING_LABEL, STARTUP_TIMEOUT),
        "the footer reports the rebase in progress, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains(MERGING_LABEL),
        "a rebase is not reported as a merge, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "rebase-abort-120x32-rebasing.txt");
    session.send_keys("alt+x");
    assert!(
        session.wait_for_text(REBASE_ABORT_PROMPT, TEXT_TIMEOUT),
        "the abort asks before discarding the rebase, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("tab");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(REBASE_ABORTED_RECEIPT, TEXT_TIMEOUT),
        "the status bar reports the aborted rebase, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        wait_for_text_absence(&mut session, REBASING_LABEL, TEXT_TIMEOUT),
        "the rebasing marker disappears after the abort, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        git_output(&repository, &["status", "--porcelain"])
            .trim()
            .is_empty(),
        "the aborted rebase leaves a clean work tree"
    );
    assert!(
        !repository.join(".git").join("rebase-merge").exists(),
        "the rebase directory is gone"
    );
    dump_screen(&mut session, "rebase-abort-120x32-aborted.txt");
    finish_with_ctrl_q(&mut session);
}

const COMMIT_FAILED_LABEL: &str = "Commit failed";
const LOCKED_INDEX_HINT_FRAGMENT: &str = "another git process holds the index";
const NOT_FAST_FORWARD_HINT_FRAGMENT: &str = "pull with the Merge or Rebase strategy";
const OVERFLOW_MARKER_FRAGMENT: &str = "more lines)";
const NOT_FAST_FORWARD_TAIL_FRAGMENT: &str = "aborting.";
const ERROR_BLOCK_ROW_COUNT: u16 = 6;
const EVIDENCE_SIZES: [(u16, u16); 2] = [(120, 32), (80, 24)];

fn start_sized_shell(repository: &Path, columns: u16, rows: u16) -> TerminalSession {
    let mut session = TerminalSession::spawn(Some(repository), columns, rows, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the {columns}x{rows} shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session
}

fn error_block_rows(session: &mut TerminalSession, title: &str) -> Vec<String> {
    let title_row = changes_position_of(session, title).1;
    (title_row..title_row + ERROR_BLOCK_ROW_COUNT)
        .map(|row| screen_line(session, row))
        .collect()
}

#[test]
fn index_lock_failure_shows_the_reason_and_the_hint() {
    for (columns, rows) in EVIDENCE_SIZES {
        let repository = dirty_repository_copy(&format!("index-lock-{columns}x{rows}"));
        std::fs::write(repository.join(".git").join("index.lock"), "")
            .expect("the index lock is written");
        let mut session = start_sized_shell(&repository, columns, rows);
        session.send_keys("ctrl+k");
        type_message(&mut session, COMMIT_MESSAGE);
        session.send_keys("ctrl+enter");
        assert!(
            session.wait_for_text(COMMIT_FAILED_LABEL, TEXT_TIMEOUT),
            "the locked index fails the commit, screen text was:\n{}",
            session.screen_text()
        );
        let block_rows = error_block_rows(&mut session, COMMIT_FAILED_LABEL);
        let block_text = block_rows.join(" ");
        assert!(
            block_text.contains("index.lock") || block_text.contains(LOCKED_INDEX_HINT_FRAGMENT),
            "the block names the locked index, rows were {block_rows:?}"
        );
        assert!(
            block_text.contains(LOCKED_INDEX_HINT_FRAGMENT),
            "the block names the next step, rows were {block_rows:?}"
        );
        assert!(
            block_text.contains(OVERFLOW_MARKER_FRAGMENT),
            "the block reports the lines it hides instead of stopping mid sentence, rows were {block_rows:?}"
        );
        dump_screen(&mut session, &format!("index-lock-{columns}x{rows}.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn diverged_pull_shows_the_rebase_hint() {
    for (columns, rows) in EVIDENCE_SIZES {
        let name = format!("diverged-pull-{columns}x{rows}");
        let (repository, _remote) = published_repository(&name);
        let remote_side = remote_side_copy(&name, &repository);
        commit_in_repository(&repository, "local advance");
        commit_in_repository(&remote_side, "remote advance");
        push_from_the_remote_side(&remote_side);
        let head_before = git_output(&repository, &["rev-parse", "HEAD"]);
        let mut session = start_sized_shell(&repository, columns, rows);
        session.send_keys("p");
        assert!(
            session.wait_for_text(PULL_DIALOG_MESSAGE, TEXT_TIMEOUT),
            "the p key opens the pull dialog, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("enter");
        assert!(
            session.wait_for_text(PULL_FAILED_LABEL, TEXT_TIMEOUT),
            "the diverged fast-forward pull fails, screen text was:\n{}",
            session.screen_text()
        );
        let block_rows = error_block_rows(&mut session, PULL_FAILED_LABEL);
        let block_text = block_rows.join(" ");
        assert!(
            block_text.contains(NOT_FAST_FORWARD_HINT_FRAGMENT),
            "the block names the next step, rows were {block_rows:?}"
        );
        assert!(
            block_text.contains(OVERFLOW_MARKER_FRAGMENT)
                || block_text.contains(NOT_FAST_FORWARD_TAIL_FRAGMENT),
            "the block ends on a whole line instead of stopping mid sentence, rows were {block_rows:?}"
        );
        dump_screen(&mut session, &format!("diverged-pull-{columns}x{rows}.txt"));
        finish_with_ctrl_q(&mut session);
        assert_eq!(
            git_output(&repository, &["rev-parse", "HEAD"]),
            head_before,
            "the refused pull leaves the head alone"
        );
    }
}

const HUNK_CHECKBOX_CLICK_COLUMN_OFFSET: u16 = 2;
const BRANCH_ROW_CLICK_COLUMN_OFFSET: u16 = 6;

#[test]
fn branch_row_click_switches_branches_in_a_real_terminal() {
    for (columns, rows, label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "120x32"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let repository = repository_with_a_second_branch("branch-row-click");
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "{label} the shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.screen_text().contains(CURRENT_BRANCH_CHIP),
            "{label} the top bar starts on the current branch, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("b");
        assert!(
            session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
            "{label} the b key opens the branches view, screen text was:\n{}",
            session.screen_text()
        );
        let other_row = log_row_of(&mut session, OTHER_BRANCH);
        let click_column = divider_column(&mut session) + BRANCH_ROW_CLICK_COLUMN_OFFSET;
        session.mouse_click(click_column, other_row);
        assert!(
            session.wait_for_text(CHECKED_OUT_RECEIPT, TEXT_TIMEOUT),
            "{label} the branch row click checks the branch out, screen text was:\n{}",
            session.screen_text()
        );
        assert!(
            session.wait_for_text(OTHER_BRANCH_CHIP, TEXT_TIMEOUT),
            "{label} the top bar chip follows the click, screen text was:\n{}",
            session.screen_text()
        );
        dump_screen(
            &mut session,
            &format!("hit-routing-branch-click-{label}.txt"),
        );
        finish_with_ctrl_q(&mut session);
        assert_eq!(
            git_output(&repository, &["branch", "--show-current"]).trim(),
            OTHER_BRANCH,
            "{label} the click leaves the branch checked out"
        );
    }
}

#[test]
fn hunk_checkbox_click_works_in_a_real_terminal() {
    for (columns, rows, label) in [
        (MEDIUM_COLUMNS, MEDIUM_ROWS, "120x32"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let repository = hunk_staging_repository("hunk-checkbox-click");
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "{label} the shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        open_notes_diff(&mut session);
        let hunk_row = hunk_header_row(&mut session);
        assert!(
            log_row_text(&mut session, hunk_row).contains(UNCHECKED_HUNK_LABEL),
            "{label} the hunk starts unchecked, screen text was:\n{}",
            session.screen_text()
        );
        let divider = divider_column(&mut session);
        let checkbox_column = divider + HUNK_CHECKBOX_CLICK_COLUMN_OFFSET;
        session.mouse_click(checkbox_column, hunk_row);
        assert!(
            log_row_text(&mut session, hunk_row).contains(CHECKED_HUNK_LABEL),
            "{label} the checkbox click checks the hunk, screen text was:\n{}",
            session.screen_text()
        );
        dump_screen(&mut session, &format!("hit-routing-hunk-click-{label}.txt"));
        session.mouse_click(checkbox_column, hunk_row);
        assert!(
            log_row_text(&mut session, hunk_row).contains(UNCHECKED_HUNK_LABEL),
            "{label} a second click unchecks the hunk, screen text was:\n{}",
            session.screen_text()
        );
        finish_with_ctrl_q(&mut session);
    }
}

#[test]
fn p_in_the_diff_pane_does_not_open_the_pull_dialog() {
    let repository = hunk_staging_repository("diff-pane-shortcuts");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    open_notes_diff(&mut session);
    session.send_keys("alt+9");
    session.send_keys("p");
    session.send_keys("b");
    session.send_keys("f");
    assert!(
        wait_for_text_absence(&mut session, PULL_DIALOG_MESSAGE, TEXT_TIMEOUT),
        "the p key opens no pull dialog in the diff pane, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        wait_for_text_absence(&mut session, FETCH_DIALOG_MESSAGE, TEXT_TIMEOUT),
        "the f key opens no fetch dialog in the diff pane, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(NOTES_TITLE),
        "the diff tab stays open, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "hit-routing-diff-pane-shortcuts.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn scrolling_ten_thousand_commits_stays_responsive() {
    let repository = fixture_repo(FixtureRepo::LargeTenThousand);
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text(FIRST_COMMIT_SUBJECT, STARTUP_TIMEOUT),
        "the first log page arrives before the scroll burst, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+9");
    let elapsed = burst_wheel_notches(&mut session, SCROLL_BURST_NOTCHES);
    println!("scroll burst of {SCROLL_BURST_NOTCHES} notches took {elapsed:?}");
    let screen = session.screen_text();
    assert!(
        !screen.contains(FIRST_COMMIT_SUBJECT),
        "the viewport left the newest commit, screen text was:\n{screen}"
    );
    assert!(
        session.wait_for_text(" branches", TEXT_TIMEOUT),
        "the details settle on the scrolled commit, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        !session.screen_text().contains("In ? branches"),
        "the deferred branches query resolves, screen text was:\n{}",
        session.screen_text()
    );
    assert_wheel_keeps_moving_three_rows_after_the_burst(&mut session);
    if scroll_burst_budget_applies_outside_ci() {
        assert!(
            elapsed < SCROLL_BURST_BUDGET,
            "the scroll burst took {elapsed:?}, the budget is {SCROLL_BURST_BUDGET:?}"
        );
    }
    dump_screen(&mut session, "scroll-burst.txt");
    finish_with_ctrl_q(&mut session);
}

fn burst_wheel_notches(session: &mut TerminalSession, notches: u16) -> Duration {
    let mut elapsed = Duration::ZERO;
    let mut remaining = notches;
    while remaining > 0 {
        let batch = remaining.min(u16::from(SCROLL_BURST_BATCH_NOTCHES));
        remaining -= batch;
        let started = Instant::now();
        session.mouse_wheel_down(
            WHEEL_COLUMN,
            BODY_ROW,
            u8::try_from(batch).expect("the wheel batch fits one wheel event count"),
        );
        elapsed += started.elapsed();
        wait_for_a_stable_screen(session);
    }
    elapsed
}

fn wait_for_a_stable_screen(session: &mut TerminalSession) -> String {
    let deadline = Instant::now() + SCREEN_STABLE_TIMEOUT;
    let mut previous = String::new();
    let mut stable_reads = 0;
    loop {
        let current = session.screen_text();
        if current == previous {
            stable_reads += 1;
            if stable_reads >= SCREEN_STABLE_READS {
                return current;
            }
        } else {
            stable_reads = 0;
            previous = current;
        }
        if Instant::now() >= deadline {
            return previous;
        }
        std::thread::sleep(SCREEN_STABLE_POLL_INTERVAL);
    }
}

fn assert_wheel_keeps_moving_three_rows_after_the_burst(session: &mut TerminalSession) {
    for attempt in 1..=WHEEL_CONFIRMATION_ATTEMPTS {
        wait_for_a_stable_screen(session);
        let first_row = log_row_text(session, LOG_FIRST_ROW);
        let row_three_lines_down = log_row_text(session, LOG_FIRST_ROW + WHEEL_LINES_PER_EVENT);
        session.mouse_wheel_down(WHEEL_COLUMN, BODY_ROW, 1);
        assert!(
            wait_for_log_row_to_change(session, LOG_FIRST_ROW, &first_row, WHEEL_CHANGE_TIMEOUT),
            "the extra wheel event moves the viewport instead of stalling after \
             {SCROLL_BURST_NOTCHES} notches, screen text was:\n{}",
            session.screen_text()
        );
        let moved_row = log_row_text(session, LOG_FIRST_ROW);
        if moved_row == row_three_lines_down {
            return;
        }
        assert!(
            attempt < WHEEL_CONFIRMATION_ATTEMPTS,
            "one wheel event still moves the viewport three rows after {SCROLL_BURST_NOTCHES} \
             notches, expected {row_three_lines_down:?} and got {moved_row:?}"
        );
    }
}

fn wait_for_log_row_to_change(
    session: &mut TerminalSession,
    row: u16,
    previous: &str,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if log_row_text(session, row) != previous {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(WHEEL_CHANGE_POLL_INTERVAL);
    }
}

fn scroll_burst_budget_applies_outside_ci() -> bool {
    std::env::var_os("CI").is_none()
}

fn dropdown_row_of(session: &mut TerminalSession, needle: &str) -> u16 {
    for row in FILTER_BAR_ROW..FILTER_BAR_ROW + DROPDOWN_ROW_SCAN_LIMIT {
        if session.row_text(row, 0, WIDE_COLUMNS).contains(needle) {
            return row;
        }
    }
    panic!(
        "the branch dropdown never shows {needle:?}, screen text was:\n{}",
        session.screen_text()
    );
}

#[test]
fn branch_filter_dropdown_applies_in_a_real_terminal() {
    let repository = repository_with_a_second_branch("branch-filter-click");
    let mut session = start_wide_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the main commit, screen text was:\n{}",
        session.screen_text()
    );
    let chip_column = branch_chip_column(&mut session, WIDE_COLUMNS);
    session.mouse_click(chip_column, FILTER_BAR_ROW);
    assert!(
        session.wait_for_text(OTHER_BRANCH, TEXT_TIMEOUT),
        "the chip opens a dropdown that lists the other branch, screen text was:\n{}",
        session.screen_text()
    );
    let branch_row = dropdown_row_of(&mut session, OTHER_BRANCH);
    let branch_column = column_of(&mut session, branch_row, OTHER_BRANCH);
    dump_screen(&mut session, "branch-filter-click-1-dropdown.txt");
    session.mouse_click(branch_column, branch_row);
    assert!(
        session.wait_for_text(BRANCH_FILTER_APPLIED_CHIP_LABEL, TEXT_TIMEOUT),
        "clicking the row applies the branch filter, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        log_row_text(&mut session, LOG_FIRST_ROW).contains(OTHER_BRANCH_SUBJECT_PREFIX),
        "the log reloads with the clicked branch, row was {:?}",
        log_row_text(&mut session, LOG_FIRST_ROW)
    );
    dump_screen(&mut session, "branch-filter-click-2-applied.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn the_branch_chevron_opens_the_branch_filter_dropdown() {
    let repository = repository_with_a_second_branch("branch-chevron-click");
    let mut session = start_wide_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the main commit, screen text was:\n{}",
        session.screen_text()
    );
    let top_row = screen_line(&mut session, TOP_BAR_ROW);
    let chevron_column =
        u16::try_from(top_row.find('▾').unwrap_or_else(|| {
            panic!("the top bar shows the branch chevron, row was {top_row:?}")
        }))
        .expect("the chevron column fits the screen");
    session.mouse_click(chevron_column, TOP_BAR_ROW);
    assert!(
        session.wait_for_text(OTHER_BRANCH, TEXT_TIMEOUT),
        "the chevron opens the branch dropdown, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-chevron-1-dropdown.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn merge_commit_message_is_prefilled_in_a_real_terminal() {
    let repository = repository_with_a_conflicting_branch("merge-message-prefill");
    let mut session = start_wide_shell(&repository);
    assert!(
        session.wait_for_text(CLEAN_TREE_SUBJECT, TEXT_TIMEOUT),
        "the log starts on the clean tree commit, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, MERGE_SOURCE_BRANCH);
    session.send_keys("alt+m");
    assert!(
        session.wait_for_text(MERGE_CONFLICT_RECEIPT, TEXT_TIMEOUT),
        "the merge stops with conflicts, screen text was:\n{}",
        session.screen_text()
    );
    let prefilled = format!("Merge branch '{MERGE_SOURCE_BRANCH}'");
    assert!(
        session.wait_for_text(&prefilled, TEXT_TIMEOUT),
        "the commit message box carries the git generated merge message, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.screen_text().contains(COMMIT_MESSAGE_TITLE),
        "the message box keeps its title above the prefilled message, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "merge-message-prefill.txt");
    finish_with_ctrl_q(&mut session);
}

#[test]
fn error_dialog_fits_wide_and_compact_terminals() {
    for (columns, rows, label) in [
        (WIDE_COLUMNS, WIDE_ROWS, "160x48"),
        (COMPACT_COLUMNS, COMPACT_ROWS, "80x24"),
    ] {
        let repository = dirty_repository_copy(&format!("error-dialog-{label}"));
        let mut session = TerminalSession::spawn(Some(&repository), columns, rows, &[]);
        assert!(
            session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
            "{label} the shell shows its log tab, screen text was:\n{}",
            session.screen_text()
        );
        session.send_keys("p");
        session.send_keys("enter");
        assert!(
            session.wait_for_text(PULL_FAILED_LABEL, TEXT_TIMEOUT),
            "{label} the failed pull is reported on screen, screen text was:\n{}",
            session.screen_text()
        );
        let mut block_row = None;
        for row in 0..rows {
            if screen_line(&mut session, row).contains(PULL_FAILED_LABEL) {
                block_row = Some(row);
                break;
            }
        }
        let block_row = block_row.unwrap_or_else(|| {
            panic!(
                "{label} the error block has a title row, screen text was:\n{}",
                session.screen_text()
            )
        });
        let title_line = screen_line(&mut session, block_row);
        let block_width = title_line.trim_end().chars().count();
        if columns == WIDE_COLUMNS {
            assert!(
                block_width > 36,
                "{label} the error block grows past the changes pane, title row was {title_line:?}"
            );
        }
        assert!(
            block_width <= usize::from(columns),
            "{label} the error block stays inside the screen, title row was {title_line:?}"
        );
        let hint_row = block_row + ERROR_BODY_ROW_COUNT;
        let hint_line = screen_line(&mut session, hint_row);
        assert!(
            hint_line.contains(NEXT_STEP_HINT_PREFIX),
            "{label} the hint row is on screen, row was {hint_line:?}"
        );
        assert!(
            hint_line.contains(MISSING_REMOTE_HINT),
            "{label} the hint row is complete instead of cut, row was {hint_line:?}"
        );
        dump_screen(&mut session, &format!("error-dialog-{label}.txt"));
        finish_with_ctrl_q(&mut session);
    }
}

const DELETED_BRANCH: &str = "feature/gone";
const DELETED_BRANCH_RECEIPT: &str = "Deleted feature/gone";
const DELETE_BRANCH_DIALOG_TITLE: &str = "Delete Branch";
const DELETE_BRANCH_MESSAGE: &str = "Delete branch feature/gone?";
const DELETE_BRANCH_CANCEL_LABEL: &str = "[ Cancel ]";
const ALT_SHIFT_D_KEY: &[u8] = b"\x1bD";
const IGNORED_PATH: &str = "notes.txt";
const IGNORED_RECEIPT: &str = "Ignored notes.txt";
const GITIGNORE_FILE_NAME: &str = ".gitignore";
const CHANGES_GROUP_SCAN_ROWS: usize = 12;

fn repository_with_a_merged_branch(name: &str) -> PathBuf {
    let repository = dirty_repository_copy(name);
    git_output(&repository, &["add", "--all"]);
    git_output(&repository, &["commit", "-q", "-m", CLEAN_TREE_SUBJECT]);
    git_output(&repository, &["branch", DELETED_BRANCH]);
    repository
}

#[test]
fn delete_a_branch_from_the_ui() {
    let repository = repository_with_a_merged_branch("branch-delete");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell starts, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("b");
    assert!(
        session.wait_for_text(BRANCHES_TITLE, TEXT_TIMEOUT),
        "the b key opens the branches view, screen text was:\n{}",
        session.screen_text()
    );
    select_branch_row(&mut session, DELETED_BRANCH);
    session.send_bytes(ALT_SHIFT_D_KEY);
    assert!(
        session.wait_for_text(DELETE_BRANCH_DIALOG_TITLE, TEXT_TIMEOUT),
        "alt+shift+d opens the delete confirmation, screen text was:\n{}",
        session.screen_text()
    );
    let screen_text = session.screen_text();
    assert!(
        screen_text.contains(DELETE_BRANCH_MESSAGE),
        "the confirmation names the branch, screen text was:\n{screen_text}"
    );
    assert!(
        screen_text.contains(DELETE_BRANCH_CANCEL_LABEL),
        "the confirmation offers the cancel button, screen text was:\n{screen_text}"
    );
    dump_screen(&mut session, "branch-delete-confirm.txt");
    session.send_keys("tab");
    session.send_keys("enter");
    assert!(
        session.wait_for_text(DELETED_BRANCH_RECEIPT, TEXT_TIMEOUT),
        "the footer reports the deleted branch, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "branch-delete-after.txt");
    finish_with_ctrl_q(&mut session);
    assert_eq!(
        git_output(&repository, &["branch", "--list", DELETED_BRANCH]),
        "",
        "the deleted branch is gone from the repository"
    );
}

#[test]
fn ignore_an_untracked_file_from_the_ui() {
    let repository = dirty_repository_copy("ignore-untracked");
    std::fs::write(repository.join(IGNORED_PATH), "scratch\n")
        .expect("the untracked file is written");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell starts, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("ctrl+k");
    for _ in 0..CHANGES_GROUP_SCAN_ROWS {
        session.send_keys("k");
    }
    for _ in 0..CHANGES_GROUP_SCAN_ROWS {
        session.send_keys("right");
        session.send_keys("j");
    }
    assert!(
        session.wait_for_text(IGNORED_PATH, TEXT_TIMEOUT),
        "the expanded unversioned group lists the scratch file, screen text was:\n{}",
        session.screen_text()
    );
    let file_row = changes_row_of(&mut session, IGNORED_PATH);
    session.mouse_click(FILE_ROW_CLICK_COLUMN, file_row);
    dump_screen(&mut session, "ignore-before.txt");
    session.send_keys("alt+i");
    assert!(
        session.wait_for_text(IGNORED_RECEIPT, TEXT_TIMEOUT),
        "the footer reports the ignored file, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "ignore-after.txt");
    finish_with_ctrl_q(&mut session);
    let gitignore = std::fs::read_to_string(repository.join(GITIGNORE_FILE_NAME))
        .expect("the ignore file is written");
    assert_eq!(gitignore, format!("{IGNORED_PATH}\n"));
    assert_eq!(
        git_output(&repository, &["status", "--porcelain", "--", IGNORED_PATH]),
        "",
        "the ignored file leaves the untracked list"
    );
}

#[test]
fn the_rail_focuses_panes_in_a_real_terminal() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the medium shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "rail-start.txt");
    let changes_divider = divider_column(&mut session);
    assert_eq!(
        session.cell(changes_divider, BODY_ROW).foreground,
        ACCENT_FOREGROUND,
        "the changes divider carries the accent while the changes pane holds the focus"
    );
    assert_eq!(
        session
            .cell(RAIL_ICON_COLUMN, RAIL_ENTRY_ROWS[0])
            .foreground,
        ACCENT_FOREGROUND,
        "the changes entry is the selected one"
    );
    session.mouse_click(RAIL_ICON_COLUMN, RAIL_ENTRY_ROWS[1]);
    assert_eq!(
        session
            .cell(CHANGES_GROUP_ROW_COLUMN, CHANGES_FIRST_ROW)
            .background,
        UNFOCUSED_SELECTION_BACKGROUND,
        "clicking the log entry moves the focus off the changes pane"
    );
    assert_eq!(
        session
            .cell(RAIL_ICON_COLUMN, RAIL_ENTRY_ROWS[1])
            .foreground,
        ACCENT_FOREGROUND,
        "the log entry turns into the selected one"
    );
    assert_eq!(
        session
            .cell(RAIL_ICON_COLUMN, RAIL_ENTRY_ROWS[0])
            .foreground,
        MUTED_FOREGROUND,
        "the changes entry drops back to the muted colour"
    );
    dump_screen(&mut session, "rail-log-focused.txt");
    session.mouse_click(RAIL_ICON_COLUMN, RAIL_ENTRY_ROWS[2]);
    assert_eq!(
        session.cell(changes_divider, BODY_ROW).foreground,
        BORDER_FOREGROUND,
        "the changes divider loses the accent while the details pane holds the focus"
    );
    let details_divider = details_divider_column(&mut session)
        .expect("the medium screen keeps the commits and details divider");
    assert_eq!(
        session.cell(details_divider, BODY_ROW).foreground,
        ACCENT_FOREGROUND,
        "the details divider carries the accent instead"
    );
    session.send_keys("alt+2");
    assert_eq!(
        session.cell(changes_divider, BODY_ROW).foreground,
        ACCENT_FOREGROUND,
        "alt+2 returns the focus to the log pane"
    );
    assert_eq!(
        session
            .cell(RAIL_ICON_COLUMN, RAIL_ENTRY_ROWS[1])
            .foreground,
        ACCENT_FOREGROUND
    );
    dump_screen(&mut session, "rail-alt-2.txt");
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

#[test]
fn the_help_overlay_opens_from_the_top_bar() {
    let repository = dirty_repository();
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the medium shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    let top_bar = screen_line(&mut session, 0);
    assert!(
        top_bar.trim_end().ends_with('?'),
        "the top bar carries the help button, line reads {top_bar:?}"
    );
    session.mouse_click(MEDIUM_COLUMNS - 1, 0);
    assert!(
        session.wait_for_text("open or close the key list", TEXT_TIMEOUT),
        "clicking the question mark opens the key list, screen text was:\n{}",
        session.screen_text()
    );
    assert!(session.screen_text().contains("Keys"));
    dump_screen(&mut session, "help-overlay.txt");
    session.send_bytes(b"\x1b");
    assert!(
        wait_for_text_absence(&mut session, "open or close the key list", TEXT_TIMEOUT),
        "escape closes the key list, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "help-overlay-closed.txt");
    assert_eq!(
        session.try_exit_code(),
        None,
        "the shell keeps running after the overlay closes"
    );
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

const CHANGES_TOOLBAR_ROW: u16 = 2;
const CASE_SENSITIVE_UPPERCASE_QUERY: &str = "CALL";
const CASE_SENSITIVE_LOWERCASE_SUBJECT: &str = "call run twice";
const CASE_SENSITIVE_ON_LABEL: &str = "Case sensitive: on";
const CASE_SENSITIVE_OFF_LABEL: &str = "Case sensitive: off";
const REFRESH_ICON_GLYPH: &str = "○";
const ROLLBACK_ICON_GLYPH: &str = "←";
const REFRESH_ICON_TOOLTIP: &str = "Refresh the repository (f5)";
const ROLLBACK_ICON_TOOLTIP: &str = "Roll back the selected changes (alt+d)";

fn move_pointer(session: &mut TerminalSession, column: u16, row: u16) {
    session.send_bytes(format!("\u{1b}[<35;{};{}M", column + 1, row + 1).as_bytes());
}

fn changes_toolbar_icon_column(
    session: &mut TerminalSession,
    columns: u16,
    glyph: &str,
) -> Option<u16> {
    (0..columns).find(|column| session.cell(*column, CHANGES_TOOLBAR_ROW).symbol == glyph)
}

#[test]
fn case_sensitive_filter_narrows_differently() {
    let repository = fixture_repository_copy(FixtureRepo::Dirty, "case-sensitive-filter");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+9");
    session.send_bytes(b"/");
    session.send_bytes(CASE_SENSITIVE_UPPERCASE_QUERY.as_bytes());
    session.send_keys("enter");
    assert!(
        session.wait_for_text(CASE_SENSITIVE_LOWERCASE_SUBJECT, TEXT_TIMEOUT),
        "an insensitive filter matches the lowercase subject, screen text was:\n{}",
        session.screen_text()
    );
    session.send_keys("alt+c");
    assert!(
        session.wait_for_text(CASE_SENSITIVE_ON_LABEL, TEXT_TIMEOUT),
        "alt+c reports the case sensitivity, screen text was:\n{}",
        session.screen_text()
    );
    let sensitive_screen = session.screen_text();
    assert!(
        !sensitive_screen.contains(CASE_SENSITIVE_LOWERCASE_SUBJECT),
        "a sensitive filter drops the lowercase subject, screen text was:\n{sensitive_screen}"
    );
    session.send_keys("alt+c");
    assert!(
        session.wait_for_text(CASE_SENSITIVE_OFF_LABEL, TEXT_TIMEOUT),
        "a second alt+c reports the insensitive state, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text(CASE_SENSITIVE_LOWERCASE_SUBJECT, TEXT_TIMEOUT),
        "the insensitive filter matches again, screen text was:\n{}",
        session.screen_text()
    );
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

#[test]
fn hovering_a_toolbar_icon_shows_its_name() {
    let repository = fixture_repository_copy(FixtureRepo::Dirty, "toolbar-hover");
    let mut session = TerminalSession::spawn(Some(&repository), MEDIUM_COLUMNS, MEDIUM_ROWS, &[]);
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    let refresh_column =
        changes_toolbar_icon_column(&mut session, MEDIUM_COLUMNS, REFRESH_ICON_GLYPH)
            .unwrap_or_else(|| {
                panic!(
                    "the changes toolbar draws the refresh icon, screen text was:\n{}",
                    session.screen_text()
                )
            });
    move_pointer(&mut session, refresh_column, CHANGES_TOOLBAR_ROW);
    assert!(
        session.wait_for_text(REFRESH_ICON_TOOLTIP, TEXT_TIMEOUT),
        "hovering the refresh icon names it in the status bar, screen text was:\n{}",
        session.screen_text()
    );
    let rollback_column =
        changes_toolbar_icon_column(&mut session, MEDIUM_COLUMNS, ROLLBACK_ICON_GLYPH)
            .expect("the changes toolbar draws the rollback icon");
    move_pointer(&mut session, rollback_column, CHANGES_TOOLBAR_ROW);
    assert!(
        session.wait_for_text(ROLLBACK_ICON_TOOLTIP, TEXT_TIMEOUT),
        "hovering the rollback icon names it in the status bar, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "affordances-toolbar-hover.txt");
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

#[test]
fn the_top_bar_reports_the_ahead_and_behind_counts() {
    let (repository, _remote) = published_repository("divergence-counts");
    git_output(
        &repository,
        &["commit", "--quiet", "--allow-empty", "-m", "ahead by one"],
    );
    let mut session = start_shell(&repository);
    assert!(
        session.wait_for_text("\u{2191}1 \u{2193}0", STARTUP_TIMEOUT),
        "the top bar reports the divergence counts, screen text was:\n{}",
        session.screen_text()
    );
    let top_bar = session.row_text(0, 0, MEDIUM_COLUMNS);
    assert!(
        top_bar.contains("\u{2191}1 \u{2193}0"),
        "the counts sit in the top bar, row was {top_bar:?}"
    );
    let (column, row) = top_bar_position_of(&mut session, "\u{2191}1");
    move_pointer(&mut session, column, row);
    assert!(
        session.wait_for_text("Ahead 1, behind 0", TEXT_TIMEOUT),
        "hovering the counts names them, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "affordances-divergence-counts.txt");
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

fn log_tab_column(session: &mut TerminalSession, columns: u16) -> (u16, u16) {
    let tab_row = screen_lines(session)
        .iter()
        .position(|line| line.contains("Log: HEAD"))
        .and_then(|index| u16::try_from(index).ok())
        .unwrap_or_else(|| {
            panic!(
                "the shell shows its log tab, screen text was:\n{}",
                session.screen_text()
            )
        });
    let column = (0..columns)
        .find(|column| session.cell(*column, tab_row).symbol == "L")
        .unwrap_or_else(|| {
            panic!(
                "the log tab title starts on row {tab_row}, screen text was:\n{}",
                session.screen_text()
            )
        });
    (column, tab_row)
}

#[test]
fn clicking_the_log_tab_with_a_diff_open_does_not_crash() {
    let repository = dirty_repository();
    let mut session = start_sized_shell(&repository, MEDIUM_COLUMNS, MEDIUM_ROWS);
    session.send_keys("ctrl+k");
    let row = changes_row_of(&mut session, "app.rs");
    session.mouse_click(FILE_ROW_CLICK_COLUMN, row);
    session.send_keys("enter");
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the diff tab opens, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "mouse-log-tab-before.txt");
    let (column, tab_row) = log_tab_column(&mut session, MEDIUM_COLUMNS);
    session.mouse_click(column, tab_row);
    assert_eq!(
        session.try_exit_code(),
        None,
        "clicking the log tab row at {column},{tab_row} keeps the shell alive, screen text was:\n{}",
        session.screen_text()
    );
    assert!(
        session.wait_for_text("Log: HEAD", TEXT_TIMEOUT),
        "the log tab stays rendered, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "mouse-log-tab-after.txt");
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

#[test]
fn the_mouse_clicks_are_recorded_with_the_debug_flag() {
    let repository = dirty_repository();
    let directory = artifacts_directory().join("e2e");
    std::fs::create_dir_all(&directory).expect("the e2e screen directory is created");
    let log_path = directory.join("mouse-debug.log");
    let _ = std::fs::remove_file(&log_path);
    let log_argument = log_path
        .to_str()
        .expect("the mouse debug path is valid utf-8");
    let mut session = TerminalSession::spawn(
        Some(&repository),
        SCREEN_COLUMNS,
        SCREEN_ROWS,
        &["--mouse-debug", log_argument],
    );
    assert!(
        session.wait_for_text("Log: HEAD", STARTUP_TIMEOUT),
        "the shell shows its log tab, screen text was:\n{}",
        session.screen_text()
    );
    session.mouse_click(FILE_ROW_CLICK_COLUMN, CHANGES_FOURTH_ROW);
    let deadline = Instant::now() + TEXT_TIMEOUT;
    let mut recorded = String::new();
    while Instant::now() < deadline {
        recorded = std::fs::read_to_string(&log_path).unwrap_or_default();
        if recorded.contains("kind=Down button=Left") {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        recorded.contains(&format!(
            "column={FILE_ROW_CLICK_COLUMN} row={CHANGES_FOURTH_ROW}"
        )),
        "the mouse debug file records the click, the file reads {recorded:?}"
    );
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

fn help_button_column(session: &mut TerminalSession, columns: u16) -> u16 {
    let top_bar = session.row_text(0, 0, columns);
    top_bar
        .rfind('?')
        .and_then(|column| u16::try_from(column).ok())
        .unwrap_or_else(|| panic!("the top bar shows the help button, row was {top_bar:?}"))
}

#[test]
fn clicking_the_help_button_closes_the_help_overlay() {
    let repository = dirty_repository();
    let mut session = start_sized_shell(&repository, MEDIUM_COLUMNS, MEDIUM_ROWS);
    session.send_keys("?");
    assert!(
        session.wait_for_text("Keys", TEXT_TIMEOUT),
        "the help overlay opens, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "mouse-help-overlay-open.txt");
    let column = help_button_column(&mut session, MEDIUM_COLUMNS);
    session.mouse_click(column, 0);
    assert!(
        wait_for_text_absence(&mut session, "Keys", TEXT_TIMEOUT),
        "the second click on ? closes the help overlay, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "mouse-help-overlay-closed.txt");
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}

#[test]
fn double_clicking_a_details_file_opens_its_diff() {
    let repository = dirty_repository();
    let mut session = start_sized_shell(&repository, WIDE_COLUMNS, WIDE_ROWS);
    let body_line = screen_line(&mut session, BODY_ROW);
    let details_left = body_line
        .char_indices()
        .filter(|(_, character)| *character == DIVIDER_GLYPH)
        .nth(1)
        .and_then(|(column, _)| u16::try_from(column + 1).ok())
        .expect("the details pane sits right of the second divider");
    let row = (CHANGES_FIRST_ROW..WIDE_ROWS - 1)
        .find(|row| {
            session
                .row_text(*row, details_left, WIDE_COLUMNS)
                .contains("app.rs")
        })
        .unwrap_or_else(|| {
            panic!(
                "the details pane lists app.rs, screen text was:\n{}",
                session.screen_text()
            )
        });
    dump_screen(&mut session, "mouse-details-double-click-before.txt");
    session.mouse_double_click(details_left + DETAILS_FILE_CLICK_OFFSET, row);
    assert!(
        session.wait_for_text(APP_FILE_TITLE, TEXT_TIMEOUT),
        "the details double click opens the diff tab, screen text was:\n{}",
        session.screen_text()
    );
    dump_screen(&mut session, "mouse-details-double-click-after.txt");
    finish_with_ctrl_q(&mut session);
    assert_restored_terminal(&session);
}
