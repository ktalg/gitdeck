use std::fs;
use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    format_commit_date, DATE_AUTHOR_GAP, DATE_COLUMN_WIDTH, GRAPH_COLUMN_WIDTH, SUBJECT_COLUMN,
};
use unicode_width::UnicodeWidthChar;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/repos";
const BASE_EPOCH_SECONDS: u64 = 1_577_836_800;
const FIRST_LANE_COLOR: Color = Color::Rgb(0x57, 0x96, 0x5C);
const SECOND_LANE_COLOR: Color = Color::Rgb(0x35, 0x74, 0xF0);
const COMMIT_BACKGROUND: Color = Color::Rgb(0x1E, 0x1F, 0x22);
const GRAPH_GLYPH_CHARACTERS: [char; 7] = ['●', '│', '╮', '╯', '┬', '┴', '─'];

fn graph_terminal(
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

fn open_graph_terminal(
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

fn commits_pane(terminal: &TestTerminal<App>) -> Rect {
    terminal.pane_rect(PaneId::Log).expect("the commits pane")
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

fn commit_row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    pane_row_text(terminal, commits_pane(terminal), row)
}

fn graph_row_cells(terminal: &TestTerminal<App>, row: u16) -> Vec<TestCell> {
    let commits = commits_pane(terminal);
    (0..GRAPH_COLUMN_WIDTH)
        .map(|column| terminal.cell(commits.x + column, row))
        .collect()
}

fn is_graph_glyph(symbol: &str) -> bool {
    symbol
        .chars()
        .next()
        .map(|character| GRAPH_GLYPH_CHARACTERS.contains(&character))
        .unwrap_or(false)
}

fn lane_node_glyph(caps: CapsProfile) -> &'static str {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => "●",
        CapsProfile::Basic => "*",
    }
}

fn graph_foreground_colors(terminal: &TestTerminal<App>, row: u16) -> Vec<Color> {
    let mut colors: Vec<Color> = Vec::new();
    for cell in graph_row_cells(terminal, row) {
        if is_graph_glyph(&cell.symbol) && !colors.contains(&cell.foreground) {
            colors.push(cell.foreground);
        }
    }
    colors
}

fn subject_first_character(terminal: &TestTerminal<App>, commit_index: usize) -> String {
    terminal
        .driver()
        .commits()
        .get(commit_index)
        .expect("the commit row exists")
        .subject
        .chars()
        .next()
        .expect("the commit holds a subject")
        .to_string()
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

fn commit_file(repository_path: &Path, relative_path: &str, content: &str, offset_seconds: u64) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory is created");
    }
    fs::write(&path, content).expect("scratch file is written");
    run_git(repository_path, offset_seconds, &["add", "--all"]);
    run_git(
        repository_path,
        offset_seconds,
        &["commit", "-q", "-m", content],
    );
}

fn branching_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    commit_file(&repository_path, "shared.txt", "base\n", 0);
    run_git(&repository_path, 0, &["switch", "-q", "-c", "feature"]);
    commit_file(&repository_path, "feature.txt", "feature\n", 60);
    run_git(&repository_path, 0, &["switch", "-q", "main"]);
    commit_file(&repository_path, "main.txt", "main\n", 120);
    run_git(
        &repository_path,
        180,
        &["merge", "--no-ff", "feature", "-m", "merge feature"],
    );
    repository_path
}

fn octopus_repository(name: &str) -> PathBuf {
    let repository_path = scratch_repository(name);
    commit_file(&repository_path, "shared.txt", "base\n", 0);
    for (index, branch) in ["p1", "p2", "p3"].iter().enumerate() {
        run_git(&repository_path, 0, &["switch", "-q", "-c", branch, "main"]);
        let offset = 60 * u64::try_from(index).unwrap_or(0) + 60;
        commit_file(
            &repository_path,
            &format!("{branch}.txt"),
            &format!("{branch}\n"),
            offset,
        );
    }
    run_git(&repository_path, 0, &["switch", "-q", "main"]);
    commit_file(&repository_path, "main.txt", "main\n", 240);
    run_git(
        &repository_path,
        300,
        &["merge", "--no-ff", "p1", "p2", "p3", "-m", "octopus merge"],
    );
    repository_path
}

#[test]
fn graph_occupies_six_columns() {
    let repository_path = branching_repository("graph-columns");
    let terminal = open_graph_terminal(
        &repository_path,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let merge_row = commits.y;
    assert_eq!(terminal.cell(commits.x, merge_row).symbol, "┬");
    assert_eq!(terminal.cell(commits.x + 1, merge_row).symbol, "─");
    assert_eq!(terminal.cell(commits.x + 2, merge_row).symbol, "╮");
    for column in 3..GRAPH_COLUMN_WIDTH {
        assert_eq!(
            terminal.cell(commits.x + column, merge_row).symbol,
            " ",
            "graph column {column} stays empty on the merge row"
        );
    }
    assert_eq!(
        terminal
            .cell(commits.x + GRAPH_COLUMN_WIDTH, merge_row)
            .symbol,
        "H",
        "the refs label starts at column {GRAPH_COLUMN_WIDTH}"
    );
}

#[test]
fn linear_history_renders_one_vertical_lane() {
    let terminal = graph_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let history = terminal.driver().commits().len();
    assert!(history >= 2, "the dirty fixture holds {history} commits");
    for row_offset in 0..u16::try_from(history).expect("commit row count") {
        let row = commits.y + row_offset;
        let symbol = terminal.cell(commits.x, row).symbol;
        assert!(
            symbol == "●" || symbol == "│",
            "row {row} holds {symbol:?} in the first lane"
        );
        assert_eq!(
            terminal.cell(commits.x, row).foreground,
            FIRST_LANE_COLOR,
            "row {row} keeps the first lane color"
        );
    }
}

#[test]
fn merge_commit_renders_merge_glyph() {
    let repository_path = branching_repository("graph-merge");
    let terminal = open_graph_terminal(
        &repository_path,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let merge_row = commits.y;
    let symbols: Vec<String> = graph_row_cells(&terminal, merge_row)
        .into_iter()
        .map(|cell| cell.symbol)
        .collect();
    assert!(
        symbols.iter().any(|symbol| symbol == "┬"),
        "the merge row {symbols:?} misses the node tee"
    );
    assert!(
        graph_foreground_colors(&terminal, merge_row).len() >= 2,
        "the merge row {symbols:?} carries one lane color only"
    );
    let base_row =
        commits.y + u16::try_from(terminal.driver().commits().len() - 1).expect("commit row count");
    let base_symbols: Vec<String> = graph_row_cells(&terminal, base_row)
        .into_iter()
        .map(|cell| cell.symbol)
        .collect();
    assert!(
        base_symbols.iter().any(|symbol| symbol == "┴")
            || base_symbols.iter().any(|symbol| symbol == "╯"),
        "the meeting row {base_symbols:?} misses the merge glyph"
    );
    assert_eq!(base_symbols[2], "╯", "the second lane closes at the base");
    assert!(
        graph_foreground_colors(&terminal, base_row).len() >= 2,
        "the meeting row {base_symbols:?} carries one lane color only"
    );
}

#[test]
fn branch_row_renders_branch_glyph() {
    let repository_path = branching_repository("graph-branch");
    let terminal = open_graph_terminal(
        &repository_path,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let merge_row = commits_pane(&terminal).y;
    let symbols: Vec<String> = graph_row_cells(&terminal, merge_row)
        .into_iter()
        .map(|cell| cell.symbol)
        .collect();
    assert_eq!(symbols[0], "┬", "the branch leaves the first lane");
    assert_eq!(symbols[1], "─", "the branch links to the second lane");
    assert_eq!(symbols[2], "╮", "the second lane starts below the node");
}

#[test]
fn lane_colors_match_the_documented_palette() {
    let repository_path = branching_repository("graph-palette");
    let terminal = open_graph_terminal(
        &repository_path,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let last_index = terminal.driver().commits().len() - 1;
    let base_row = commits.y + u16::try_from(last_index).expect("commit row count");
    terminal.assert_cell_style(commits.x, base_row, FIRST_LANE_COLOR, COMMIT_BACKGROUND);
    terminal.assert_cell_style(
        commits.x + 2,
        base_row,
        SECOND_LANE_COLOR,
        COMMIT_BACKGROUND,
    );
}

#[test]
fn refs_do_not_repeat_the_head_branch() {
    let repository_path = branching_repository("graph-refs");
    let terminal = open_graph_terminal(
        &repository_path,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let merge_row = commits_pane(&terminal).y;
    let row_text = commit_row_text(&terminal, merge_row);
    assert!(
        row_text.contains("HEAD -> main"),
        "row {row_text:?} misses the head label"
    );
    assert_eq!(
        row_text.matches("main").count(),
        1,
        "row {row_text:?} repeats the head branch"
    );
}

#[test]
fn date_precedes_author_at_row_end() {
    let terminal = graph_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let history = terminal.driver().commits().to_vec();
    for (index, commit) in history.iter().enumerate() {
        let row = commits.y + u16::try_from(index).expect("commit row count");
        let author_width = u16::try_from(commit.author_name.chars().count()).expect("author width");
        let author_start = commits.right() - author_width;
        let author_text: String = (author_start..commits.right())
            .map(|column| terminal.cell(column, row).symbol)
            .collect();
        assert_eq!(author_text, commit.author_name, "row {row} author");
        for column in author_start - DATE_AUTHOR_GAP..author_start {
            assert_eq!(
                terminal.cell(column, row).symbol,
                " ",
                "row {row} column {column} is the author gap"
            );
        }
        let date_end = author_start - DATE_AUTHOR_GAP;
        let date_start = date_end - DATE_COLUMN_WIDTH;
        let date_text: String = (date_start..date_end)
            .map(|column| terminal.cell(column, row).symbol)
            .collect();
        assert_eq!(
            date_text.trim(),
            format_commit_date(commit.author_timestamp, 0),
            "row {row} date field"
        );
    }
}

#[test]
fn subject_still_starts_at_column_six() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = graph_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let commits = commits_pane(&terminal);
        let reference_free_row = commits.y + 1;
        assert!(
            terminal.driver().commits()[1].refs.is_empty(),
            "{caps:?} row 1 carries no refs"
        );
        assert_eq!(
            terminal
                .cell(commits.x + SUBJECT_COLUMN, reference_free_row)
                .symbol,
            subject_first_character(&terminal, 1),
            "{caps:?} subject column"
        );
        assert_eq!(
            terminal.cell(commits.x, reference_free_row).symbol,
            lane_node_glyph(caps),
            "{caps:?} column 0 holds the lane marker"
        );
    }
}

#[test]
fn graph_truncates_beyond_three_lanes() {
    let repository_path = octopus_repository("graph-octopus");
    let terminal = open_graph_terminal(
        &repository_path,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    let commits = commits_pane(&terminal);
    let octopus_row = commits.y;
    for column in 0..GRAPH_COLUMN_WIDTH {
        assert!(
            is_graph_glyph(&terminal.cell(commits.x + column, octopus_row).symbol),
            "graph column {column} of the four parent merge is empty"
        );
    }
    for column in GRAPH_COLUMN_WIDTH..GRAPH_COLUMN_WIDTH + 2 {
        let symbol = terminal.cell(commits.x + column, octopus_row).symbol;
        assert!(
            !is_graph_glyph(&symbol),
            "column {column} draws the truncated lane glyph {symbol:?}"
        );
    }
    assert_eq!(
        terminal.cell(commits.x + 4, octopus_row).symbol,
        "╮",
        "the third lane is still drawn"
    );
}

#[test]
fn graph_screenshots_render_for_three_profiles() {
    let linear_scenarios: [(CapsProfile, &str); 3] = [
        (CapsProfile::Rich, "graph/linear-160x48"),
        (CapsProfile::Standard, "graph/linear-standard"),
        (CapsProfile::Basic, "graph/linear-basic"),
    ];
    for (caps, name) in linear_scenarios {
        let terminal = graph_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let report = terminal.screenshot(name);
        assert!(report.png_path.exists(), "{name} png");
        assert!(report.text_path.exists(), "{name} text");
        assert!(report.ansi_path.exists(), "{name} ansi");
        terminal.assert_no_missing_glyphs();
    }
    let repository_path = branching_repository("graph-screenshots");
    for (width, height, name) in [
        (REFERENCE_WIDTH, REFERENCE_HEIGHT, "graph/merge-160x48"),
        (NARROW_WIDTH, NARROW_HEIGHT, "graph/merge-80x24"),
    ] {
        let terminal = open_graph_terminal(&repository_path, width, height, CapsProfile::Rich);
        let report = terminal.screenshot(name);
        assert!(report.png_path.exists(), "{name} png");
        terminal.assert_no_missing_glyphs();
        let merge_row = commits_pane(&terminal).y;
        assert_eq!(
            terminal.cell(commits_pane(&terminal).x, merge_row).symbol,
            "┬",
            "{name} merge row"
        );
    }
}
