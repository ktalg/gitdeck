use gitdeck_app::App;
use gitdeck_core::PaneId;
use gitdeck_testkit::prelude::*;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const CHANGES_ROW_SCAN_LIMIT: usize = 32;
const MODIFIED_APP_PATH: &str = "src/app.rs";
const TWO_HUNK_FILE_PATH: &str = "src/large.txt";
const WORD_HIGHLIGHTS_OFF_LABEL: &str = "Word highlights: off";
const WORD_HIGHLIGHTS_ON_LABEL: &str = "Word highlights: on";
const CHANGES_PANE_HINT: &str = "space check";
const LOG_PANE_HINT: &str = "b branches";
const DIFF_PANE_HINT: &str = "space hunk";
const QUIT_KEYS: [&str; 2] = ["ctrl+q", "ctrl+c"];

fn app_terminal(
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

fn showcase_terminal(caps: CapsProfile) -> TestTerminal<App> {
    app_terminal(
        FixtureRepo::DiffShowcase,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        caps,
    )
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

fn open_selected_file_diff(terminal: &mut TestTerminal<App>, path: &str) {
    select_file(terminal, path);
    terminal.send_keys("enter");
    assert!(
        terminal.driver().file_diff().is_some(),
        "the diff of {path} is open"
    );
}

fn status_message_text(terminal: &TestTerminal<App>) -> Option<String> {
    terminal
        .driver()
        .status_message()
        .map(|(text, _tone)| text.to_string())
}

#[test]
fn escape_closes_the_diff_tab_instead_of_quitting() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(caps);
        open_selected_file_diff(&mut terminal, TWO_HUNK_FILE_PATH);
        terminal.send_keys("esc");
        assert!(
            terminal.driver().file_diff().is_none(),
            "{caps:?} esc closes the diff tab"
        );
        assert_eq!(
            terminal.driver().tab_titles().len(),
            1,
            "{caps:?} the log tab is the only tab left"
        );
        assert_eq!(
            terminal.focused_pane(),
            PaneId::Changes,
            "{caps:?} esc keeps the shell routing keys"
        );
        terminal.send_keys("j");
        assert!(
            terminal.driver().selected_file_path().is_some(),
            "{caps:?} the shell still answers the keys after esc"
        );
    }
}

#[test]
fn escape_closes_a_dialog_before_anything_else() {
    let mut terminal = app_terminal(
        FixtureRepo::Dirty,
        REFERENCE_WIDTH,
        REFERENCE_HEIGHT,
        CapsProfile::Rich,
    );
    open_selected_file_diff(&mut terminal, MODIFIED_APP_PATH);
    terminal.send_keys("alt+d");
    assert!(
        terminal.driver().discard_dialog_visible(),
        "the discard dialog is open on top of the diff tab"
    );
    terminal.send_keys("esc");
    assert!(
        !terminal.driver().discard_dialog_visible(),
        "esc closes the dialog first"
    );
    assert!(
        terminal.driver().file_diff().is_some(),
        "esc leaves the diff tab open while it closes the dialog"
    );
    terminal.send_keys("esc");
    assert!(
        terminal.driver().file_diff().is_none(),
        "the second esc closes the diff tab"
    );
}

#[test]
fn escape_alone_does_not_exit_the_app() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal =
            app_terminal(FixtureRepo::Dirty, REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let before = terminal.screen_text();
        let selected_before = terminal.driver().selected_file_path().map(str::to_string);
        terminal.send_keys("esc");
        terminal.send_keys("esc");
        assert_eq!(
            terminal.screen_text(),
            before,
            "{caps:?} esc alone leaves the screen untouched"
        );
        assert_eq!(
            terminal.driver().selected_file_path().map(str::to_string),
            selected_before,
            "{caps:?} esc alone leaves the selection untouched"
        );
        assert_eq!(
            terminal.driver().tab_titles().len(),
            1,
            "{caps:?} esc alone opens nothing"
        );
    }
}

#[test]
fn ctrl_q_and_ctrl_c_still_exit() {
    let quit_keys: Vec<&str> = QUIT_KEYS
        .iter()
        .copied()
        .filter(|keys| {
            gitdeck_app::key_bindings_for_keys(keys)
                .iter()
                .any(|binding| binding.commands.contains(&gitdeck_app::KeyCommand::Quit))
        })
        .collect();
    assert_eq!(
        quit_keys,
        QUIT_KEYS.to_vec(),
        "ctrl+q and ctrl+c stay registered as the quit keys"
    );
    for binding in gitdeck_app::key_bindings_for_keys("esc") {
        assert!(
            !binding.commands.contains(&gitdeck_app::KeyCommand::Quit),
            "esc is no longer a quit key, it reads {:?}",
            binding
        );
    }
}

#[test]
fn opening_a_diff_moves_the_focus_to_the_diff_pane() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(caps);
        terminal.assert_focused(PaneId::Changes);
        open_selected_file_diff(&mut terminal, TWO_HUNK_FILE_PATH);
        assert_eq!(
            terminal.focused_pane(),
            PaneId::Log,
            "{caps:?} the diff pane takes the focus"
        );
    }
}

#[test]
fn space_after_opening_a_diff_toggles_the_hunk_not_the_file() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_file_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    let checked_files_before = terminal.driver().checked_file_paths();
    terminal.send_keys("space");
    assert_eq!(
        terminal.driver().checked_hunks_for_path(TWO_HUNK_FILE_PATH),
        vec![0],
        "space checks the hunk under the cursor"
    );
    assert_eq!(
        terminal.driver().checked_file_paths(),
        checked_files_before,
        "space leaves the file checkboxes alone"
    );
}

#[test]
fn closing_the_diff_restores_the_previous_focus() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_file_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    terminal.send_keys("ctrl+w");
    assert!(
        terminal.driver().file_diff().is_none(),
        "ctrl+w closes the diff tab"
    );
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Changes,
        "the focus returns to the pane that opened the diff"
    );

    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    terminal.send_keys("enter");
    assert!(
        terminal.driver().file_diff().is_some(),
        "enter opens the diff of the selected commit"
    );
    terminal.send_keys("esc");
    assert!(
        terminal.driver().file_diff().is_none(),
        "esc closes the commit diff tab"
    );
    assert_eq!(
        terminal.focused_pane(),
        PaneId::Log,
        "the focus stays on the log pane that opened the diff"
    );
}

#[test]
fn b_does_not_switch_tabs_while_a_diff_tab_is_focused() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    open_selected_file_diff(&mut terminal, TWO_HUNK_FILE_PATH);
    assert_eq!(
        terminal.driver().log_view(),
        gitdeck_app::LogView::Log,
        "the log tab shows the commits"
    );
    for keys in ["b", "p", "f"] {
        terminal.send_keys(keys);
        assert_eq!(
            terminal.driver().log_view(),
            gitdeck_app::LogView::Log,
            "{keys} does not switch the log view while the diff tab is focused"
        );
        assert!(
            !terminal.driver().pull_dialog_visible(),
            "{keys} does not open the pull dialog while the diff tab is focused"
        );
        assert!(
            !terminal.driver().fetch_dialog_visible(),
            "{keys} does not open the fetch dialog while the diff tab is focused"
        );
    }
}

#[test]
fn alt_w_toggles_the_highlight_from_the_log_pane() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    terminal.send_keys("alt+w");
    assert_eq!(
        status_message_text(&terminal),
        Some(String::from(WORD_HIGHLIGHTS_OFF_LABEL)),
        "alt+w toggles the word highlight from the log pane"
    );
    terminal.send_keys("alt+w");
    assert_eq!(
        status_message_text(&terminal),
        Some(String::from(WORD_HIGHLIGHTS_ON_LABEL)),
        "alt+w toggles the word highlight back"
    );
}

#[test]
fn i_focuses_the_commit_message_from_any_pane() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(caps);
        for pane in [PaneId::Log, PaneId::Details] {
            terminal.send_keys("ctrl+k");
            while terminal.focused_pane() != pane {
                terminal.send_keys("tab");
            }
            terminal.send_keys("i");
            assert_eq!(
                terminal.focused_pane(),
                PaneId::Changes,
                "{caps:?} the i key brings the focus to the commit message"
            );
            assert_eq!(
                terminal.driver().changes_focus(),
                gitdeck_app::ChangesFocus::Message,
                "{caps:?} the i key starts the message editor from {pane:?}"
            );
            assert_eq!(
                terminal.driver().commit_message(),
                "",
                "{caps:?} the i key types no character from {pane:?}"
            );
            terminal.send_keys("esc");
            assert_eq!(
                terminal.driver().changes_focus(),
                gitdeck_app::ChangesFocus::List,
                "{caps:?} esc leaves the message editor"
            );
        }
    }
}

#[test]
fn the_footer_hint_lists_the_keys_for_the_focused_pane() {
    let mut terminal = showcase_terminal(CapsProfile::Rich);
    terminal.assert_focused(PaneId::Changes);
    let changes_screen = terminal.screen_text();
    assert!(
        changes_screen.contains(CHANGES_PANE_HINT),
        "the status bar names the changes keys, screen was:\n{changes_screen}"
    );
    terminal.send_keys("alt+9");
    terminal.assert_focused(PaneId::Log);
    let log_screen = terminal.screen_text();
    assert!(
        log_screen.contains(LOG_PANE_HINT),
        "the status bar names the log keys, screen was:\n{log_screen}"
    );
    assert!(
        !log_screen.contains(CHANGES_PANE_HINT),
        "the changes hint disappears while the log pane holds the focus"
    );
    terminal.send_keys("enter");
    assert!(
        terminal.driver().file_diff().is_some(),
        "enter opens the diff tab"
    );
    let diff_screen = terminal.screen_text();
    assert!(
        diff_screen.contains(DIFF_PANE_HINT),
        "the status bar names the diff keys, screen was:\n{diff_screen}"
    );
}
