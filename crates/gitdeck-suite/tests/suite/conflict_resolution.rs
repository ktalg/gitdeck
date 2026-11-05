use std::path::{Path, PathBuf};

use gitdeck_app::App;
use gitdeck_testkit::{fixture_git_command, fixture_repo, prelude::*, scratch_directory};
use gitdeck_tui::{StatusTone, CONFLICT_RESOLUTION_HINT};

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/conflict-repos";
const CONFLICT_PATH: &str = "src/conflict.txt";
const CONFLICT_GROUP_TITLE: &str = "Merge Conflicts";
const ACCEPTED_OURS_RECEIPT: &str = "Accepted ours for src/conflict.txt";
const UNMERGED_STATUS_PREFIX: &str = "UU ";

fn scratch_conflict_repository(name: &str) -> PathBuf {
    let target = scratch_directory(SCRATCH_REPOSITORY_DIRECTORY, name);
    copy_directory(&fixture_repo(FixtureRepo::MergeConflict), &target)
        .expect("the merge conflict fixture is copied");
    target
}

fn conflict_terminal(name: &str, caps: CapsProfile) -> TestTerminal<App> {
    let repository_path = scratch_conflict_repository(name);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, caps, ThemeId::JbDark)
            .expect("the conflicted repository opens"),
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(REFERENCE_WIDTH, REFERENCE_HEIGHT);
    terminal
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

fn status_bar_text(terminal: &TestTerminal<App>) -> String {
    (0..REFERENCE_WIDTH)
        .map(|column| terminal.cell(column, REFERENCE_HEIGHT - 1).symbol)
        .collect()
}

fn run_git(repository_path: &Path, arguments: &[&str]) -> String {
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

#[test]
fn conflict_items_show_the_resolution_hint() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = conflict_terminal("hint", caps);
        terminal.tick();
        assert!(
            !status_bar_text(&terminal).contains(CONFLICT_RESOLUTION_HINT),
            "{caps:?} the hint stays hidden while no conflicted file is selected: {}",
            status_bar_text(&terminal)
        );
        select_file(&mut terminal, CONFLICT_PATH);
        terminal.screenshot("conflict_resolution/step-1-hint");
        let status_bar = status_bar_text(&terminal);
        assert!(
            status_bar.contains(CONFLICT_RESOLUTION_HINT),
            "{caps:?} the conflicted selection shows the resolution hint, status bar was {status_bar:?}"
        );
        assert!(
            status_bar.trim_end().ends_with(env!("CARGO_PKG_VERSION")),
            "{caps:?} the hint keeps the version label on the right edge: {status_bar:?}"
        );
    }
}

#[test]
fn alt_o_resolves_the_selected_conflict_and_reports_it() {
    let mut terminal = conflict_terminal("resolve-ours", CapsProfile::Rich);
    terminal.tick();
    assert!(
        terminal.screen_text().contains(CONFLICT_GROUP_TITLE),
        "the fixture starts with the merge conflicts group:\n{}",
        terminal.screen_text()
    );
    select_file(&mut terminal, CONFLICT_PATH);
    terminal.send_keys("alt+o");
    terminal.tick();
    terminal.screenshot("conflict_resolution/step-2-resolved");
    assert_eq!(
        terminal.driver().status_message(),
        Some((ACCEPTED_OURS_RECEIPT, StatusTone::Info))
    );
    assert!(
        status_bar_text(&terminal).contains(ACCEPTED_OURS_RECEIPT),
        "the status bar carries the receipt: {:?}",
        status_bar_text(&terminal)
    );
    assert!(
        !terminal.screen_text().contains(CONFLICT_GROUP_TITLE),
        "the resolved file leaves the merge conflicts group:\n{}",
        terminal.screen_text()
    );
    let repository_path = terminal
        .driver()
        .repository_path()
        .expect("the scratch repository is open")
        .to_path_buf();
    let status = run_git(&repository_path, &["status", "--porcelain"]);
    assert!(
        !status.contains(UNMERGED_STATUS_PREFIX),
        "git reports no unmerged path any more: {status}"
    );
    assert!(
        !status.contains(CONFLICT_PATH),
        "our side matches the head commit, so the path leaves the change list: {status}"
    );
}
