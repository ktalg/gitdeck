use std::path::{Path, PathBuf};

use gitdeck_app::{App, EditorTab};
use gitdeck_core::{ChangeKind, LogFilter, LogQuery, PaneId};
use gitdeck_git::{CliGit, GitBackend};
use gitdeck_testkit::prelude::*;
use gitdeck_testkit::repository_root;
use gitdeck_tui::{ellipsis_glyph, LOG_RAIL_GAP, LOG_RAIL_WIDTH, LOG_TAB_TITLE};

const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const WHEEL_ROW: u16 = 1;
const DIFF_HORIZONTAL_STEP_COLUMNS: u16 = 8;
const ACCENT_COLOR: Color = Color::Rgb(0x35, 0x74, 0xF0);
const BORDER_COLOR: Color = Color::Rgb(0x39, 0x3B, 0x40);
const EDITOR_BACKGROUND: Color = Color::Rgb(0x1E, 0x1F, 0x22);
const CJK_TAB_TITLE: &str = "说明.md (Working Tree)";
const CJK_DIFF_PATH: &str = "文档/说明.md";
const CJK_MISSING_GLYPHS: [char; 25] = [
    '。', '中', '于', '内', '列', '变', '后', '容', '文', '新', '更', '明', '校', '档', '用', '的',
    '表', '说', '：', '验', '未', '跟', '踪', '件', '增',
];
const FRESH_COMMIT_SUBJECT: &str = "call run twice";
const NO_REPOSITORY_LABEL: &str = "no repository";
const BROKEN_GIT_DIRECTORY_FILE: &str = "gitdir: ./missing-gitdir\n";
const NON_REPOSITORY_DIRECTORY_NAME: &str = "non-repository";
const FILTER_BAR_LABEL: &str = "Text or hash";
const HIDDEN_FILTER_BLOCKS: [&str; 2] = ["Date", "Paths"];

fn fixture_terminal(
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
                .expect("the fixture repository opens")
        },
    );
    terminal.resize(width, height);
    terminal
}

fn select_file(terminal: &mut TestTerminal<App>, path: &str) {
    terminal.send_keys("ctrl+k");
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

fn pane_row(terminal: &TestTerminal<App>, pane: PaneId, row: u16) -> String {
    let rect = terminal
        .pane_rect(pane)
        .unwrap_or_else(|| panic!("the {pane:?} pane has no rectangle"));
    let mut text = String::new();
    for column in rect.x..rect.right() {
        text.push_str(&terminal.cell(column, row).symbol);
    }
    text.trim_end().to_string()
}

fn pane_column(terminal: &TestTerminal<App>, pane: PaneId) -> u16 {
    terminal
        .pane_rect(pane)
        .unwrap_or_else(|| panic!("the {pane:?} pane has no rectangle"))
        .x
}

fn non_repository_directory(name: &str) -> PathBuf {
    let directory = repository_root()
        .join("artifacts")
        .join("tmp")
        .join(NON_REPOSITORY_DIRECTORY_NAME)
        .join(name);
    std::fs::create_dir_all(&directory).expect("the non repository directory is created");
    std::fs::write(directory.join(".git"), BROKEN_GIT_DIRECTORY_FILE)
        .expect("the broken git directory file is written");
    directory
}

fn non_repository_terminal(
    name: &str,
    width: u16,
    height: u16,
    caps: CapsProfile,
) -> TestTerminal<App> {
    let directory = non_repository_directory(name);
    terminal_around_path(directory.as_path(), width, height, caps)
}

fn terminal_around_path(
    path: &Path,
    width: u16,
    height: u16,
    caps: CapsProfile,
) -> TestTerminal<App> {
    let mut terminal = TestTerminal::new(
        App::open_repository(path, caps, ThemeId::JbDark),
        width,
        height,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal
}

#[test]
fn cjk_tab_title_is_not_truncated_mid_character() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = fixture_terminal(
            FixtureRepo::CjkPaths,
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            caps,
        );
        select_file(&mut terminal, CJK_DIFF_PATH);
        terminal.send_keys("enter");
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Diff(String::from(CJK_DIFF_PATH)),
            "{caps:?} active tab"
        );
        let tab_row = terminal
            .find_line("Working Tree")
            .expect("the diff tab row is visible");
        let tab_line = terminal.screen_lines()[usize::from(tab_row)].clone();
        assert!(
            tab_line.contains(CJK_TAB_TITLE),
            "{caps:?} tab row {tab_line:?} does not hold {CJK_TAB_TITLE:?}"
        );
        assert!(
            !tab_line.contains("说.md"),
            "{caps:?} tab row {tab_line:?} lost the second wide character"
        );
        terminal.assert_missing_glyphs(&CJK_MISSING_GLYPHS);
        if caps == CapsProfile::Rich {
            terminal.screenshot("acceptance/cjk-tab-title");
        }
    }
}

#[test]
fn no_repository_shell_shows_the_placeholder_branch_chip() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = non_repository_terminal("branch-chip", 120, 32, caps);
        let top_bar = terminal.screen_lines()[0].clone();
        assert!(
            top_bar.contains(NO_REPOSITORY_LABEL),
            "{caps:?} top bar {top_bar:?} does not name the missing repository"
        );
        assert!(
            !top_bar.contains("master") && !top_bar.contains("* "),
            "{caps:?} top bar {top_bar:?} claims a branch"
        );
        let status_line = terminal
            .screen_lines()
            .last()
            .cloned()
            .expect("the status row");
        assert!(
            status_line.starts_with(&format!(" {NO_REPOSITORY_LABEL}")),
            "{caps:?} status row {status_line:?}"
        );
        assert!(terminal.driver().open_error().is_some());
    }
    non_repository_terminal("branch-chip", 120, 32, CapsProfile::Rich)
        .screenshot("acceptance/non-repository-error");
}

#[test]
fn open_error_reason_wraps_over_three_rows() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = non_repository_terminal("wrapped-error", 120, 32, caps);
        let label_row = terminal
            .find_line("Cannot open repository")
            .expect("the label row is visible");
        let reason_rows: Vec<String> = (label_row + 1..label_row + 4)
            .map(|row| pane_row(&terminal, PaneId::Changes, row))
            .collect();
        assert!(
            reason_rows[0].contains("git rev-parse --show-toplevel"),
            "{caps:?} first reason row {:?}",
            reason_rows[0]
        );
        let joined = reason_rows.join(" ");
        assert!(
            joined.contains("failed in"),
            "{caps:?} the reason continues on the second row, rows {reason_rows:?}"
        );
        assert!(
            reason_rows[2].ends_with(ellipsis_glyph(caps)),
            "{caps:?} the third reason row {reason_rows:?} ends with the ellipsis"
        );
        assert_eq!(
            terminal
                .cell(pane_column(&terminal, PaneId::Changes), label_row + 1)
                .foreground,
            Color::Rgb(0x86, 0x8A, 0x91),
            "{caps:?} the reason keeps the muted color"
        );
        terminal.assert_no_missing_glyphs();
    }
}

#[test]
fn diff_tab_reclaims_the_filter_bar_row() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            fixture_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let filters_row = terminal.driver().layout().log_filters;
        let commits_row = terminal.driver().layout().commits;
        assert_eq!(
            filters_row.height, 1,
            "{caps:?} the log tab shows the filters"
        );
        select_file(&mut terminal, "src/app.rs");
        terminal.send_keys("enter");
        let diff_layout = terminal.driver().layout();
        assert_eq!(
            diff_layout.log_filters.height, 0,
            "{caps:?} a diff tab drops the filter bar"
        );
        assert_eq!(diff_layout.commits.y, filters_row.y);
        assert_eq!(diff_layout.commits.height, commits_row.height + 1);
        assert!(
            !terminal.screen_text().contains(FILTER_BAR_LABEL),
            "{caps:?} the filter labels are gone with the filter row"
        );
        terminal.send_keys("ctrl+w");
        assert_eq!(
            terminal.driver().layout().log_filters,
            filters_row,
            "{caps:?} closing the diff restores the filter bar"
        );
        assert!(terminal.screen_text().contains(FILTER_BAR_LABEL));
    }
}

#[test]
fn enter_on_a_commit_opens_the_first_changed_file() {
    let repository_path = fixture_repo(FixtureRepo::Dirty);
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
        .expect("the fixture has a head commit");
    let head_files = repository
        .commit_changed_files(&head_commit.id)
        .expect("the head commit files are readable");
    let first_file = head_files
        .first()
        .expect("the head commit changes a file")
        .path
        .clone();
    let commit_diff = repository
        .diff_commit_file(&head_commit.id, &first_file)
        .expect("the head commit diff is readable");
    assert!(
        !commit_diff.hunks.is_empty(),
        "the head commit changes {first_file}"
    );
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            fixture_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.send_keys("alt+9");
        assert_eq!(terminal.focused_pane(), PaneId::Log);
        terminal.send_keys("enter");
        assert_eq!(
            terminal.driver().active_tab(),
            EditorTab::Diff(first_file.clone()),
            "{caps:?} active tab"
        );
        let titles = terminal.driver().tab_titles();
        let file_name = first_file.rsplit('/').next().unwrap_or(&first_file);
        assert_eq!(titles.first(), Some(&String::from(LOG_TAB_TITLE)));
        assert!(
            titles
                .iter()
                .any(|title| title.starts_with(file_name) && title.contains("(Working Tree)")),
            "{caps:?} tab titles {titles:?} name the commit file {file_name:?}"
        );
        assert_eq!(
            terminal
                .driver()
                .file_diff()
                .map(|file_diff| file_diff.path.clone()),
            Some(first_file.clone()),
            "{caps:?} the open diff belongs to the commit file"
        );
    }
}

#[test]
fn right_scrolls_the_diff_horizontally() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = fixture_terminal(
            FixtureRepo::DiffShowcase,
            REFERENCE_WIDTH,
            REFERENCE_HEIGHT,
            caps,
        );
        select_file(&mut terminal, "src/long_line.txt");
        terminal.send_keys("enter");
        terminal.send_keys("alt+z");
        assert_eq!(terminal.driver().diff_horizontal_offset(), 0);
        let hunk_row = terminal
            .find_line("@@ -1,1 +1,1 @@")
            .expect("the hunk header row is visible");
        let hunk_line = terminal.screen_lines()[usize::from(hunk_row)].clone();
        terminal.send_keys("right");
        assert_eq!(
            terminal.driver().diff_horizontal_offset(),
            DIFF_HORIZONTAL_STEP_COLUMNS
        );
        let shifted_line = terminal
            .screen_lines()
            .get(usize::from(hunk_row))
            .cloned()
            .unwrap_or_else(|| panic!("{caps:?} the hunk header row stays visible"));
        assert_eq!(
            shifted_line, hunk_line,
            "{caps:?} the hunk header ignores the horizontal offset"
        );
        let scrolled_body_row = terminal
            .screen_lines()
            .iter()
            .skip(usize::from(hunk_row) + 1)
            .find(|line| line.trim_end().ends_with('>'))
            .cloned();
        assert!(
            scrolled_body_row.is_some(),
            "{caps:?} a diff body row scrolled horizontally and ends with the marker"
        );
        terminal.send_keys("right");
        assert_eq!(
            terminal.driver().diff_horizontal_offset(),
            DIFF_HORIZONTAL_STEP_COLUMNS * 2
        );
        for _ in 0..4 {
            terminal.send_keys("left");
        }
        assert_eq!(
            terminal.driver().diff_horizontal_offset(),
            0,
            "{caps:?} the offset stops at the first column"
        );
        terminal.send_keys("right");
        terminal.send_keys("right");
        if caps == CapsProfile::Rich {
            terminal.screenshot("acceptance/diff-horizontal-offset");
        }
    }
}

#[test]
fn eighty_columns_hide_details_and_keep_the_subject_readable() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = fixture_terminal(FixtureRepo::Dirty, NARROW_WIDTH, NARROW_HEIGHT, caps);
        let layout = terminal.driver().layout();
        assert_eq!(
            layout.details.width, 0,
            "{caps:?} the details column is hidden"
        );
        assert_eq!(
            layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
            layout.log_area.width
        );
        assert_eq!(terminal.pane_rect(PaneId::Details), None);
        let commit_row = terminal
            .find_line(FRESH_COMMIT_SUBJECT)
            .map(|row| (row, true))
            .or_else(|| {
                terminal
                    .find_line(ellipsis_glyph(caps).encode_utf8(&mut [0u8; 4]))
                    .map(|row| (row, false))
            })
            .map(|(row, _)| row)
            .expect("a commit row is visible");
        let commit_line = terminal.screen_lines()[usize::from(commit_row)].clone();
        assert!(
            commit_line.contains(FRESH_COMMIT_SUBJECT),
            "{caps:?} commit row {commit_line:?} does not show the whole subject"
        );
        assert!(
            !commit_line.contains("1/1/20"),
            "{caps:?} commit row {commit_line:?} keeps the date column"
        );
        let filter_row = terminal
            .find_line(FILTER_BAR_LABEL)
            .expect("the filter bar is visible");
        let filter_line = terminal.screen_lines()[usize::from(filter_row)].clone();
        for hidden_block in HIDDEN_FILTER_BLOCKS {
            assert!(
                !filter_line.contains(hidden_block),
                "{caps:?} filter row {filter_line:?} keeps the {hidden_block:?} block"
            );
        }
        if caps == CapsProfile::Rich {
            terminal.screenshot("acceptance/narrow-80x24");
        }
    }
}

#[test]
fn focused_pane_divider_uses_the_accent_color() {
    for caps in EVERY_CAPS_PROFILE {
        assert_focused_pane_divider_colors(caps);
    }
}

fn assert_focused_pane_divider_colors(caps: CapsProfile) {
    let mut terminal =
        fixture_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
    let layout = terminal.driver().layout();
    let divider_row = layout.commits.y + WHEEL_ROW;
    terminal.assert_cell_style(
        layout.changes_log_divider.x,
        divider_row,
        ACCENT_COLOR,
        EDITOR_BACKGROUND,
    );
    assert_eq!(
        terminal
            .cell(layout.commits_details_divider.x, divider_row)
            .foreground,
        BORDER_COLOR,
        "{caps:?} the divider away from the focused pane keeps the border color"
    );
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    terminal.assert_cell_style(
        layout.commits_details_divider.x,
        divider_row,
        ACCENT_COLOR,
        EDITOR_BACKGROUND,
    );
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Details);
    terminal.assert_cell_style(
        layout.commits_details_divider.x,
        divider_row,
        ACCENT_COLOR,
        EDITOR_BACKGROUND,
    );
    assert_eq!(
        terminal
            .cell(layout.changes_log_divider.x, divider_row)
            .foreground,
        BORDER_COLOR
    );
}

#[test]
fn tab_skips_the_hidden_details_pane() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = fixture_terminal(FixtureRepo::Dirty, NARROW_WIDTH, NARROW_HEIGHT, caps);
        assert_eq!(terminal.focused_pane(), PaneId::Changes);
        terminal.send_keys("tab");
        assert_eq!(terminal.focused_pane(), PaneId::Log, "{caps:?}");
        terminal.send_keys("tab");
        assert_eq!(
            terminal.focused_pane(),
            PaneId::Changes,
            "{caps:?} the hidden details pane is skipped"
        );
        terminal.send_keys("shift+tab");
        assert_eq!(terminal.focused_pane(), PaneId::Log, "{caps:?}");
    }
}

#[test]
fn commit_changed_kinds_are_readable_from_the_shared_backend() {
    let repository_path = fixture_repo(FixtureRepo::Dirty);
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
        .expect("the fixture has a head commit");
    let files = repository
        .commit_changed_files(&head_commit.id)
        .expect("the head commit files are readable");
    assert!(
        !files.is_empty(),
        "the head commit changes at least one file"
    );
    for file in &files {
        assert!(
            matches!(file.kind, ChangeKind::Added | ChangeKind::Modified),
            "the head commit changes {} with {:?}",
            file.path,
            file.kind
        );
        let file_diff = repository
            .diff_commit_file(&head_commit.id, &file.path)
            .expect("the commit file diff is readable");
        assert_eq!(file_diff.path, file.path);
        assert!(
            !file_diff.hunks.is_empty(),
            "{} has hunks in the head commit",
            file.path
        );
        assert!(
            file_diff
                .hunks
                .iter()
                .flat_map(|hunk| hunk.lines.iter())
                .any(|line| line.kind == gitdeck_git::DiffLineKind::Added),
            "{} holds added lines in the commit diff",
            file.path
        );
    }
}
