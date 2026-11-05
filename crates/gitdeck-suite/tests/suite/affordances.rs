use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    case_sensitive_filter_block_rect, changes_toolbar_entry_rects, divergence_label, theme_palette,
    top_bar_divergence_rect, CHANGES_TOOLBAR_ENTRIES,
};

const MEDIUM_WIDTH: u16 = 120;
const MEDIUM_HEIGHT: u16 = 32;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const UPPERCASE_FILTER: &str = "CALL";
const FILTERED_SUBJECT: &str = "call run twice";
const CASE_SENSITIVE_ON_STATUS: &str = "Case sensitive: on";
const CASE_SENSITIVE_OFF_STATUS: &str = "Case sensitive: off";
const REFRESHED_STATUS: &str = "Refreshed";

fn terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the dirty fixture opens")
        },
    );
    terminal.resize(width, height);
    terminal.tick();
    terminal
}

fn rich_terminal() -> TestTerminal<App> {
    terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, CapsProfile::Rich)
}

fn move_pointer(terminal: &mut TestTerminal<App>, column: u16, row: u16) {
    terminal
        .driver_mut()
        .handle_event(InputEvent::Mouse(MouseInput {
            kind: MouseInputKind::Move,
            button: MouseButton::None,
            column,
            row,
            modifiers: Default::default(),
            click_count: 1,
        }));
    terminal.tick();
}

fn apply_text_filter(terminal: &mut TestTerminal<App>, text: &str) {
    terminal.send_keys("alt+9");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    terminal.send_keys("/");
    terminal.send_text(text);
    terminal.send_keys("enter");
    terminal.tick();
}

fn case_sensitive_block(terminal: &TestTerminal<App>) -> Rect {
    case_sensitive_filter_block_rect(
        terminal.driver().layout().log_filters,
        terminal.driver().log_filter_branch(),
    )
    .expect("the filter bar shows the case sensitivity block")
}

#[test]
fn cc_toggles_case_sensitive_text_filtering() {
    let mut terminal = rich_terminal();
    apply_text_filter(&mut terminal, UPPERCASE_FILTER);
    assert!(
        !terminal.driver().log_filter().case_sensitive,
        "the case sensitivity starts off"
    );
    assert!(
        terminal.screen_text().contains(FILTERED_SUBJECT),
        "an insensitive filter matches the lowercase subject, screen was:\n{}",
        terminal.screen_text()
    );

    terminal.send_keys("alt+c");
    terminal.tick();
    assert!(
        terminal.driver().log_filter().case_sensitive,
        "alt+c turns the case sensitivity on"
    );
    assert!(
        !terminal.screen_text().contains(FILTERED_SUBJECT),
        "a sensitive filter drops the lowercase subject, screen was:\n{}",
        terminal.screen_text()
    );

    terminal.send_keys("alt+c");
    terminal.tick();
    assert!(!terminal.driver().log_filter().case_sensitive);
    assert!(
        terminal.screen_text().contains(FILTERED_SUBJECT),
        "turning the case sensitivity off restores the match"
    );
}

#[test]
fn the_cc_button_shows_its_state_and_reports_it() {
    let mut terminal = rich_terminal();
    let theme = theme_palette(ThemeId::JbDark);
    let block = case_sensitive_block(&terminal);
    assert_eq!(
        terminal.cell(block.x, block.y).foreground,
        theme.muted,
        "the block stays muted while the filtering is insensitive"
    );

    terminal.mouse_click(block.x, block.y);
    terminal.tick();
    assert!(terminal.driver().log_filter().case_sensitive);
    let block = case_sensitive_block(&terminal);
    assert_eq!(
        terminal.cell(block.x, block.y).foreground,
        theme.accent,
        "the block turns accent while the filtering is sensitive"
    );
    assert_eq!(
        terminal.driver().status_message().map(|(text, _)| text),
        Some(CASE_SENSITIVE_ON_STATUS)
    );

    terminal.mouse_click(block.x, block.y);
    terminal.tick();
    assert!(!terminal.driver().log_filter().case_sensitive);
    let block = case_sensitive_block(&terminal);
    assert_eq!(terminal.cell(block.x, block.y).foreground, theme.muted);
    assert_eq!(
        terminal.driver().status_message().map(|(text, _)| text),
        Some(CASE_SENSITIVE_OFF_STATUS)
    );
    terminal.screenshot("affordances/case-sensitive-off");
    terminal.mouse_click(block.x, block.y);
    terminal.tick();
    terminal.screenshot("affordances/case-sensitive-on");
}

#[test]
fn the_changes_toolbar_icons_all_have_a_tooltip() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, caps);
        let changes = terminal.driver().layout().changes;
        for (rect, entry) in changes_toolbar_entry_rects(changes)
            .into_iter()
            .zip(CHANGES_TOOLBAR_ENTRIES)
        {
            assert_eq!(
                terminal.cell(rect.x, rect.y).symbol,
                entry.icon.character(caps).to_string()
            );
            move_pointer(&mut terminal, rect.x, rect.y);
            assert_eq!(
                terminal.driver().status_hint(),
                Some(entry.tooltip),
                "the icon at column {} reports its name",
                rect.x
            );
        }
    }
}

#[test]
fn no_dead_icon_remains_in_the_changes_toolbar() {
    let mut terminal = rich_terminal();
    let changes = terminal.driver().layout().changes;
    let rects = changes_toolbar_entry_rects(changes);
    let rendered: String = rects
        .iter()
        .filter(|rect| rect.width > 0)
        .map(|rect| terminal.cell(rect.x, rect.y).symbol.clone())
        .collect::<Vec<String>>()
        .join("");
    assert_eq!(
        rendered,
        String::from("○←▸↑▾"),
        "the toolbar draws only the icons whose actions exist"
    );

    terminal.mouse_click(rects[0].x, rects[0].y);
    terminal.tick();
    assert_eq!(
        terminal.driver().status_message().map(|(text, _)| text),
        Some(REFRESHED_STATUS),
        "the refresh icon reports that it reloaded"
    );

    terminal.send_keys("ctrl+k");
    loop {
        if terminal.driver().selected_file_path().is_some() {
            break;
        }
        terminal.send_keys("j");
    }
    let selected = terminal.driver().selected_file_path().map(str::to_string);
    let before = selected;

    terminal.mouse_click(rects[4].x, rects[4].y);
    terminal.tick();
    let after = terminal.driver().selected_file_path().map(str::to_string);
    assert_ne!(before, after, "the move down icon moves the selection");

    terminal.mouse_click(rects[3].x, rects[3].y);
    terminal.tick();
    assert_eq!(
        terminal.driver().selected_file_path().map(str::to_string),
        before,
        "the move up icon moves the selection back"
    );

    terminal.mouse_click(rects[2].x, rects[2].y);
    terminal.tick();
    assert!(
        terminal.driver().file_diff().is_some(),
        "the show diff icon opens the diff of the selected file"
    );
    terminal.send_keys("ctrl+w");
    terminal.tick();

    terminal.mouse_click(rects[1].x, rects[1].y);
    terminal.tick();
    assert!(
        terminal.driver().discard_dialog_visible(),
        "the rollback icon opens the discard dialog"
    );
    terminal.send_keys("esc");
    terminal.tick();
    terminal.screenshot("affordances/toolbar-icons");
}

#[test]
fn the_ahead_behind_indicator_shows_numbers() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = terminal(MEDIUM_WIDTH, MEDIUM_HEIGHT, caps);
        let top_bar = terminal.driver().layout().top_bar;
        let head_info = terminal.driver().head_info();
        let rect = top_bar_divergence_rect(
            top_bar,
            terminal.driver().project_name().as_deref(),
            head_info.branch.as_deref(),
            head_info.ahead,
            head_info.behind,
            caps,
        )
        .expect("the top bar shows the divergence of the checked out branch");
        let mut row = String::new();
        for column in rect.x..rect.right() {
            row.push_str(&terminal.cell(column, rect.y).symbol);
        }
        assert_eq!(
            row,
            divergence_label(caps, head_info.ahead, head_info.behind)
        );
        assert!(
            row.contains(char::is_numeric),
            "the indicator carries the counts, row was {row:?}"
        );
    }
}

#[test]
fn clicking_the_ahead_behind_indicator_opens_the_pull_dialog() {
    let mut terminal = rich_terminal();
    let rect = top_bar_divergence_rect(
        terminal.driver().layout().top_bar,
        terminal.driver().project_name().as_deref(),
        terminal.driver().head_info().branch.as_deref(),
        0,
        0,
        CapsProfile::Rich,
    )
    .expect("the top bar shows the divergence");
    assert!(!terminal.driver().pull_dialog_visible());
    terminal.mouse_click(rect.x, rect.y);
    terminal.tick();
    assert!(
        terminal.driver().pull_dialog_visible(),
        "the divergence click opens the pull dialog, screen was:\n{}",
        terminal.screen_text()
    );
    assert!(
        !terminal.driver().branch_filter_dropdown_visible(),
        "the divergence is not part of the branch chip"
    );
    terminal.screenshot("affordances/divergence-pull-dialog");
    terminal.send_keys("esc");
    terminal.tick();

    move_pointer(&mut terminal, rect.x, rect.y);
    assert!(
        terminal
            .driver()
            .status_hint()
            .is_some_and(|hint| hint.starts_with("Ahead 0, behind 0")),
        "the hover reports the ahead and behind counts, hint was {:?}",
        terminal.driver().status_hint()
    );
    terminal.screenshot("affordances/divergence-hover");
}

#[test]
fn case_sensitive_filtering_is_case_sensitive_in_git() {
    use gitdeck_core::{LogFilter, LogQuery};
    use gitdeck_git::{CliGit, GitBackend};

    let repository_path = fixture_repo(FixtureRepo::Dirty);
    let git = CliGit::open(&repository_path).expect("the dirty fixture opens");
    let query = LogQuery {
        max_commits: 10,
        skip: 0,
    };
    let insensitive = git
        .log(
            query,
            &LogFilter {
                branch: None,
                text: Some(String::from(UPPERCASE_FILTER)),
                case_sensitive: false,
            },
        )
        .expect("the insensitive query runs");
    assert_eq!(
        insensitive
            .iter()
            .filter(|commit| commit.subject == FILTERED_SUBJECT)
            .count(),
        1,
        "the insensitive query matches the lowercase subject"
    );
    let sensitive = git
        .log(
            query,
            &LogFilter {
                branch: None,
                text: Some(String::from(UPPERCASE_FILTER)),
                case_sensitive: true,
            },
        )
        .expect("the sensitive query runs");
    assert!(
        sensitive
            .iter()
            .all(|commit| commit.subject != FILTERED_SUBJECT),
        "the sensitive query drops the lowercase subject"
    );
}
