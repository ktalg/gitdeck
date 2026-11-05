use gitdeck_app::App;
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    activity_rail_entry_rects, branch_filter_block_rect, case_sensitive_filter_block_rect,
    changes_panel_layout, changes_toolbar_entry_rects, remote_tool_bar_rects,
    text_filter_block_rect, top_bar_branch_chevron_rect, top_bar_branch_chip_rect,
    top_bar_divergence_rect, top_bar_help_button_rect, CHANGES_TOOLBAR_ENTRIES,
};

const WIDTH: u16 = 160;
const HEIGHT: u16 = 48;
const SCANNED_SAMPLES: [(u16, u16); 3] = [(120, 32), (160, 48), (220, 70)];
const ALREADY_ACTIVE_CONTROLS: [(&str, &str); 1] = [(
    "the Commit tab of the changes pane",
    "the tab already carries the active state, so a click restates the current view",
)];

fn terminal(width: u16, height: u16) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        width,
        height,
        CapsProfile::Rich,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, CapsProfile::Rich, ThemeId::JbDark)
                .expect("the dirty fixture opens")
        },
    );
    terminal.resize(width, height);
    terminal.tick();
    terminal
}

fn screen_signature(terminal: &TestTerminal<App>) -> String {
    let mut signature = terminal.screen_text();
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            let cell = terminal.cell(column, row);
            signature.push_str(&cell.symbol);
            signature.push_str(&color_key(cell.foreground));
            signature.push_str(&color_key(cell.background));
            signature.push_str(&format!("{:?}", cell.modifier));
        }
    }
    signature
}

fn color_key(color: ratatui::style::Color) -> String {
    format!("{color:?}")
}

fn head_branch(terminal: &TestTerminal<App>) -> Option<String> {
    terminal.driver().head_info().branch.clone()
}

fn project_name(terminal: &TestTerminal<App>) -> Option<String> {
    terminal.driver().project_name()
}

fn first_cell(rect: ratatui::layout::Rect) -> (u16, u16) {
    (rect.x, rect.y)
}

fn click_and_diff(
    prepare: impl FnOnce(&mut TestTerminal<App>),
    point: impl Fn(&TestTerminal<App>) -> Option<(u16, u16)>,
) -> Option<(String, String)> {
    let mut terminal = terminal(WIDTH, HEIGHT);
    prepare(&mut terminal);
    terminal.tick();
    let (column, row) = point(&terminal)?;
    let before = screen_signature(&terminal);
    terminal.mouse_click(column, row);
    terminal.tick();
    let after = screen_signature(&terminal);
    (before == after).then_some((before, after))
}

fn assert_no_dead_control(
    label: &str,
    precondition: &str,
    prepare: impl FnOnce(&mut TestTerminal<App>),
    point: impl Fn(&TestTerminal<App>) -> Option<(u16, u16)>,
) {
    assert!(
        !precondition.is_empty(),
        "{label} names the state it needs before the click"
    );
    if let Some((before, after)) = click_and_diff(prepare, point) {
        panic!(
            "{label} must react to a click in the state {precondition:?}, \
             the screen stayed at\n{before}\nthe second sample read\n{after}"
        );
    }
}

fn open_a_file_diff(terminal: &mut TestTerminal<App>) {
    terminal.send_keys("ctrl+k");
    for _ in 0..40 {
        if terminal.driver().selected_file_path().is_some() {
            break;
        }
        terminal.send_keys("j");
    }
    terminal.send_keys("enter");
    assert!(
        terminal.driver().file_diff().is_some(),
        "the fixture opens a file diff"
    );
}

#[test]
fn every_rendered_control_reacts_to_a_click() {
    assert_no_dead_control(
        "the help button",
        "the app is idle",
        |_| {},
        |terminal| {
            let top_bar = terminal.driver().layout().top_bar;
            Some(first_cell(top_bar_help_button_rect(top_bar)))
        },
    );
    assert_no_dead_control(
        "the branch chip",
        "a branch is checked out",
        |_| {},
        |terminal| {
            let top_bar = terminal.driver().layout().top_bar;
            top_bar_branch_chip_rect(
                top_bar,
                project_name(terminal).as_deref(),
                head_branch(terminal).as_deref(),
                terminal.driver().head_info().ahead,
                terminal.driver().head_info().behind,
                terminal.caps(),
            )
            .map(first_cell)
        },
    );
    assert_no_dead_control(
        "the branch chevron",
        "a branch is checked out",
        |_| {},
        |terminal| {
            let top_bar = terminal.driver().layout().top_bar;
            top_bar_branch_chevron_rect(
                top_bar,
                project_name(terminal).as_deref(),
                head_branch(terminal).as_deref(),
                terminal.driver().head_info().ahead,
                terminal.driver().head_info().behind,
                terminal.caps(),
            )
            .map(first_cell)
        },
    );
    assert_no_dead_control(
        "the divergence chip",
        "a branch is checked out",
        |_| {},
        |terminal| {
            let layout = terminal.driver().layout();
            let head_info = terminal.driver().head_info();
            top_bar_divergence_rect(
                layout.top_bar,
                project_name(terminal).as_deref(),
                head_branch(terminal).as_deref(),
                head_info.ahead,
                head_info.behind,
                terminal.caps(),
            )
            .map(first_cell)
        },
    );
    for index in 0..3 {
        let label = format!("the remote tool label {index}");
        assert_no_dead_control(
            &label,
            "a branch is checked out",
            |_| {},
            move |terminal| {
                let layout = terminal.driver().layout();
                let head_info = terminal.driver().head_info();
                let rects = remote_tool_bar_rects(
                    layout.top_bar,
                    project_name(terminal).as_deref(),
                    head_branch(terminal).as_deref(),
                    head_info.ahead,
                    head_info.behind,
                    terminal.caps(),
                );
                Some(first_cell(rects[index]))
            },
        );
    }
    for (index, prepare_keys) in [(0, "alt+9"), (1, "ctrl+k"), (2, "ctrl+k")] {
        let label = format!("the activity rail entry {index}");
        assert_no_dead_control(
            &label,
            "another pane holds the focus",
            |terminal| {
                terminal.send_keys(prepare_keys);
            },
            move |terminal| {
                let rail = terminal.driver().layout().activity_rail;
                Some(first_cell(activity_rail_entry_rects(rail)[index]))
            },
        );
    }
    for (index, entry) in CHANGES_TOOLBAR_ENTRIES.iter().enumerate() {
        let label = format!("the {} toolbar icon", entry.tooltip);
        assert_no_dead_control(
            &label,
            "the changes pane holds rows and, for the up icon, a selection below the first row",
            |terminal| {
                terminal.send_keys("ctrl+k");
                if index == 3 {
                    terminal.send_keys("j");
                }
            },
            move |terminal| {
                let changes = terminal.driver().layout().changes;
                Some(first_cell(changes_toolbar_entry_rects(changes)[index]))
            },
        );
    }
    for (label, rect_of_panel) in [
        (
            "the commit button",
            (|panel: &gitdeck_tui::ChangesPanelLayout| panel.commit_button)
                as fn(&gitdeck_tui::ChangesPanelLayout) -> ratatui::layout::Rect,
        ),
        ("the commit-and-push button", |panel| panel.push_button),
        ("the commit options gear", |panel| panel.options_button),
        ("the amend row", |panel| panel.amend_row),
        ("the commit message box", |panel| panel.message_box),
    ] {
        assert_no_dead_control(
            label,
            "the commit block is on screen",
            |terminal| {
                terminal.send_keys("ctrl+k");
            },
            move |terminal| {
                let layout = terminal.driver().layout();
                let panel = changes_panel_layout(layout.changes, terminal.driver().amend())?;
                Some(first_cell(rect_of_panel(&panel)))
            },
        );
    }
    assert_no_dead_control(
        "the log rail",
        "the log pane is focused",
        |terminal| {
            terminal.send_keys("alt+9");
        },
        |terminal| Some(first_cell(terminal.driver().layout().log_rail)),
    );
    assert_no_dead_control(
        "the log tab title",
        "a diff tab is open beside the log tab",
        |terminal| {
            open_a_file_diff(terminal);
        },
        |terminal| {
            let tabs = terminal.driver().layout().log_tabs;
            (tabs.width > 2).then(|| (tabs.x + 1, tabs.y))
        },
    );
    assert_no_dead_control(
        "the text filter block",
        "the filter bar shows its blocks",
        |terminal| {
            terminal.send_keys("alt+9");
        },
        |terminal| {
            text_filter_block_rect(terminal.driver().layout().log_filters, None).map(first_cell)
        },
    );
    assert_no_dead_control(
        "the case-sensitive filter block",
        "the filter bar shows its blocks",
        |terminal| {
            terminal.send_keys("alt+9");
        },
        |terminal| {
            case_sensitive_filter_block_rect(terminal.driver().layout().log_filters, None)
                .map(first_cell)
        },
    );
    assert_no_dead_control(
        "the branch filter block",
        "the filter bar shows its blocks",
        |terminal| {
            terminal.send_keys("alt+9");
        },
        |terminal| {
            branch_filter_block_rect(terminal.driver().layout().log_filters, None).map(first_cell)
        },
    );
}

#[test]
fn only_the_already_active_tab_ignores_a_click() {
    for (label, condition) in ALREADY_ACTIVE_CONTROLS {
        assert!(
            !label.is_empty() && !condition.is_empty(),
            "every ignored control names its condition"
        );
    }
    let mut terminal = terminal(WIDTH, HEIGHT);
    let layout = terminal.driver().layout();
    let before = screen_signature(&terminal);
    terminal.mouse_click(layout.changes.x + 1, layout.changes.y);
    terminal.tick();
    assert_eq!(
        before,
        screen_signature(&terminal),
        "{} has no state to change",
        ALREADY_ACTIVE_CONTROLS[0].0
    );
}

#[test]
fn removed_affordances_stay_removed() {
    for (width, height) in SCANNED_SAMPLES {
        let terminal = terminal(width, height);
        let layout = terminal.driver().layout();
        let changes_row = terminal.screen_lines()[usize::from(layout.changes.y)].clone();
        assert!(
            !changes_row.contains("Shelf"),
            "{width}x{height} the shelf tab is gone, row {changes_row:?}"
        );
        let log_tabs_row = terminal.screen_lines()[usize::from(layout.log_tabs.y)].clone();
        let log_tabs_area: String = (layout.log_tabs.x..layout.log_tabs.right())
            .map(|column| terminal.cell(column, layout.log_tabs.y).symbol)
            .collect();
        assert!(
            !log_tabs_area.contains('+'),
            "{width}x{height} the add-tab button is gone, row {log_tabs_row:?}"
        );
        assert!(
            log_tabs_row.contains("Log: HEAD"),
            "{width}x{height} the log tab keeps its title, row {log_tabs_row:?}"
        );
        let top_bar: String = (layout.top_bar.x..layout.top_bar.right())
            .map(|column| terminal.cell(column, layout.top_bar.y).symbol)
            .collect();
        assert!(
            !top_bar.contains('≡'),
            "{width}x{height} the menu glyph is gone, row {top_bar:?}"
        );
        let expected_project_name =
            project_name(&terminal).expect("the dirty fixture names its directory");
        assert!(
            top_bar.contains(&expected_project_name),
            "{width}x{height} the project name {expected_project_name:?} stays, row {top_bar:?}"
        );
        let filter_row: String = (layout.log_filters.x..layout.log_filters.right())
            .map(|column| terminal.cell(column, layout.log_filters.y).symbol)
            .collect();
        for removed in [".*", "User", "Date", "Paths"] {
            assert!(
                !filter_row.contains(removed),
                "{width}x{height} the {removed:?} block is gone, row {filter_row:?}"
            );
        }
        for kept in ["Text or hash", "Cc", "Branch: HEAD"] {
            assert!(
                filter_row.contains(kept),
                "{width}x{height} the {kept:?} block stays, row {filter_row:?}"
            );
        }
        let options_button = changes_panel_layout(layout.changes, terminal.driver().amend())
            .map(|panel| panel.options_button);
        assert!(
            options_button.is_some_and(|rect| rect.width > 0),
            "{width}x{height} the commit options gear keeps its seat in the button row"
        );
    }
}
