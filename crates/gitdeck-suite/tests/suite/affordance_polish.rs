use gitdeck_app::{App, KEY_BINDINGS};
use gitdeck_core::{ChangeKind, PaneId};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    amend_summary_notice, changes_panel_layout, commit_and_push_compact_label,
    commit_and_push_label, compact_amend_summary_notice, text_display_width, wrap_help_rows,
    ChangesPanelLayout, StatusTone, AMEND_COMMIT_BUTTON_COMPACT_LABEL, AMEND_COMMIT_BUTTON_LABEL,
    COMMIT_AND_PUSH_ASCII_LABEL, COMMIT_BUTTON_LABEL, HELP_OVERLAY_MORE_ASCII_LABEL,
    HELP_OVERLAY_MORE_LABEL,
};

const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SUPPORTED_WIDTHS: [(u16, u16); 3] = [(120, 32), (160, 48), (220, 70)];
const NARROW_WIDTH: u16 = 120;
const NARROW_HEIGHT: u16 = 32;
const WIDE_WIDTH: u16 = 160;
const WIDE_HEIGHT: u16 = 48;
const DETAILS_HIDDEN_STATUS: &str = "Details is hidden while a diff tab shows";
const DIFF_TAB_CLOSE_HINT: &str = "ctrl+w closes the tab and returns the file list";
const SOFT_WRAP_ON_STATUS: &str = "Soft wrap: on";
const SOFT_WRAP_OFF_STATUS: &str = "Soft wrap: off";
const CHANGES_ROW_SCAN_LIMIT: usize = 40;

fn showcase_terminal(width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::DiffShowcase,
        width,
        height,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the showcase repository opens")
        },
    );
    terminal.resize(width, height);
    terminal.tick();
    terminal
}

fn changes_layout(terminal: &TestTerminal<App>) -> ChangesPanelLayout {
    changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        terminal.driver().amend(),
    )
    .expect("the changes pane holds the commit block")
}

fn pane_row_text(terminal: &TestTerminal<App>, row: u16) -> String {
    terminal
        .screen_lines()
        .get(usize::from(row))
        .cloned()
        .unwrap_or_default()
}

fn untracked_count(terminal: &TestTerminal<App>) -> usize {
    terminal
        .driver()
        .changes()
        .iter()
        .filter(|entry| entry.kind == ChangeKind::Untracked)
        .count()
}

fn open_the_first_file_diff(terminal: &mut TestTerminal<App>) {
    terminal.send_keys("ctrl+k");
    for _ in 0..CHANGES_ROW_SCAN_LIMIT {
        if terminal.driver().selected_file_path().is_some() {
            break;
        }
        terminal.send_keys("j");
    }
    terminal.send_keys("enter");
    assert!(
        terminal.driver().file_diff().is_some(),
        "enter opens the file diff, screen was:\n{}",
        terminal.screen_text()
    );
}

#[test]
fn the_amend_button_row_fits_every_supported_width() {
    for (width, height) in SUPPORTED_WIDTHS {
        for caps in EVERY_CAPS_PROFILE {
            let mut terminal = showcase_terminal(width, height, caps);
            terminal.send_keys("alt+a");
            assert!(
                terminal.driver().amend(),
                "{width}x{height} {caps:?}: alt+a checks amend"
            );
            let layout = changes_layout(&terminal);
            let row_text = pane_row_text(&terminal, layout.button_row.y);
            let expected_commit_label = match layout.buttons.commit_label(true) {
                label if label == AMEND_COMMIT_BUTTON_LABEL => AMEND_COMMIT_BUTTON_LABEL,
                _ => AMEND_COMMIT_BUTTON_COMPACT_LABEL,
            };
            assert_eq!(
                layout.commit_button.width,
                text_display_width(expected_commit_label),
                "{width}x{height} {caps:?}: the commit button hugs its label, row {row_text:?}"
            );
            assert!(
                row_text.contains(expected_commit_label),
                "{width}x{height} {caps:?}: the commit button is printed in full, row {row_text:?}"
            );
            if let Some(push_label) = layout.buttons.push_label(caps) {
                assert!(
                    layout.push_button.width >= text_display_width(&push_label),
                    "{width}x{height} {caps:?}: the push button is never truncated, row {row_text:?}"
                );
                assert!(
                    row_text.contains(&push_label),
                    "{width}x{height} {caps:?}: the push label {push_label:?} is printed in full, row {row_text:?}"
                );
            }
            let gear_column = layout.options_button.x;
            assert_eq!(
                terminal.cell(gear_column - 1, layout.button_row.y).symbol,
                String::from(" "),
                "{width}x{height} {caps:?}: one blank column separates the buttons from the gear, row {row_text:?}"
            );
            let changes_right = terminal
                .pane_rect(PaneId::Changes)
                .expect("the changes pane is visible")
                .right();
            assert!(
                gear_column + 1 < changes_right,
                "{width}x{height} {caps:?}: the gear stays off the pane divider, gear at {gear_column}, pane right {changes_right}"
            );
            assert_ne!(
                terminal.cell(gear_column, layout.button_row.y).symbol,
                String::from("│"),
                "{width}x{height} {caps:?}: the gear column holds the gear, not the divider"
            );
            terminal.screenshot(&format!(
                "affordance_polish/amend-buttons-{width}x{height}-{caps:?}"
            ));
        }
    }
}

#[test]
fn the_full_push_label_still_fits_a_wide_pane() {
    let terminal = showcase_terminal(220, 70, CapsProfile::Rich);
    let layout = changes_layout(&terminal);
    assert!(
        layout.push_button.width >= text_display_width(&commit_and_push_label(CapsProfile::Rich)),
        "the wide pane keeps the long push label"
    );
    assert_eq!(
        layout.buttons,
        gitdeck_tui::CommitButtonRow::FullLabels,
        "the wide pane keeps the full label pair"
    );
    assert!(
        commit_and_push_compact_label(CapsProfile::Rich).len()
            < commit_and_push_label(CapsProfile::Rich).len(),
        "the compact push label is shorter than the long one"
    );
    assert_eq!(
        layout.buttons.commit_label(false),
        COMMIT_BUTTON_LABEL,
        "the plain row keeps the commit label"
    );
    assert!(
        text_display_width(COMMIT_AND_PUSH_ASCII_LABEL)
            >= text_display_width(&commit_and_push_label(CapsProfile::Rich)),
        "the rich glyph is no wider than the ascii fallback the layout measures"
    );
    terminal.screenshot("affordance_polish/amend-buttons-wide");
}

#[test]
fn every_next_step_hint_is_english() {
    for hint in gitdeck_tui::next_step_hints() {
        assert!(hint.is_ascii(), "the hint stays english: {hint:?}");
        assert!(!hint.trim().is_empty(), "the hint says something");
    }
}

#[test]
fn the_help_overlay_lists_the_commit_close_and_escape_keys() {
    let rows = wrap_help_rows(
        &KEY_BINDINGS
            .iter()
            .map(|binding| (binding.keys, binding.action))
            .collect::<Vec<_>>(),
        60,
    );
    for keys in ["ctrl+enter", "ctrl+w", "esc"] {
        assert!(
            rows.iter().any(|row| row.contains(keys)),
            "the help table lists {keys}"
        );
    }
}

#[test]
fn alt_z_reports_the_soft_wrap_state() {
    let mut terminal = showcase_terminal(WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    let wrap_before = terminal.driver().soft_wrap();
    terminal.send_keys("alt+z");
    let expected_first = if wrap_before {
        SOFT_WRAP_OFF_STATUS
    } else {
        SOFT_WRAP_ON_STATUS
    };
    assert_eq!(
        terminal.driver().status_message(),
        Some((expected_first, StatusTone::Info)),
        "alt+z reports the wrap it just switched, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.send_keys("alt+z");
    let expected_second = if wrap_before {
        SOFT_WRAP_ON_STATUS
    } else {
        SOFT_WRAP_OFF_STATUS
    };
    assert_eq!(
        terminal.driver().status_message(),
        Some((expected_second, StatusTone::Info)),
        "alt+z reports the opposite state on the second press, screen was:\n{}",
        terminal.screen_text()
    );
    assert_eq!(terminal.driver().soft_wrap(), wrap_before);
    terminal.screenshot("affordance_polish/soft-wrap-status");
}

#[test]
fn alt_3_reports_that_the_details_pane_is_hidden_by_a_diff_tab() {
    let mut terminal = showcase_terminal(WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_the_first_file_diff(&mut terminal);
    terminal.send_keys("alt+3");
    assert_eq!(
        terminal.driver().status_message(),
        Some((DETAILS_HIDDEN_STATUS, StatusTone::Info)),
        "alt+3 explains why the details pane refuses the focus, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("affordance_polish/details-hidden-by-diff");
}

#[test]
fn the_footer_names_the_key_that_returns_to_the_file_list() {
    let mut terminal = showcase_terminal(WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    open_the_first_file_diff(&mut terminal);
    assert!(
        terminal.screen_text().contains(DIFF_TAB_CLOSE_HINT),
        "the footer names the way back to the file list, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.send_keys("ctrl+w");
    assert!(
        !terminal.screen_text().contains(DIFF_TAB_CLOSE_HINT),
        "the hint leaves with the tab, screen was:\n{}",
        terminal.screen_text()
    );
    terminal.screenshot("affordance_polish/diff-tab-close-hint");
}

#[test]
fn the_help_overlay_shows_a_more_marker_when_the_rows_overflow() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = showcase_terminal(WIDE_WIDTH, WIDE_HEIGHT, caps);
        terminal.send_keys("?");
        let screen_text = terminal.screen_text();
        let marker = match caps {
            CapsProfile::Basic => HELP_OVERLAY_MORE_ASCII_LABEL,
            CapsProfile::Rich | CapsProfile::Standard => HELP_OVERLAY_MORE_LABEL,
        };
        assert!(
            screen_text.contains(marker),
            "{caps:?}: the overflowing help table marks the rows below with {marker:?}, screen was:\n{screen_text}"
        );
        terminal.screenshot(&format!("affordance_polish/help-more-{caps:?}"));
    }
}

#[test]
fn the_amend_preview_stays_visible_at_120_columns() {
    let mut terminal = showcase_terminal(NARROW_WIDTH, NARROW_HEIGHT, CapsProfile::Rich);
    terminal.send_keys("alt+a");
    assert!(terminal.driver().amend(), "alt+a checks amend");
    let file_count = terminal.driver().checked_file_paths().len();
    let untracked = untracked_count(&terminal);
    let layout = changes_layout(&terminal);
    let amend_row = pane_row_text(&terminal, layout.amend_row.y);
    assert!(
        amend_row.contains(&amend_summary_notice(file_count, untracked))
            || amend_row.contains(&compact_amend_summary_notice(file_count, untracked)),
        "the narrow amend row previews {file_count} files and {untracked} untracked, got {amend_row:?}"
    );
    terminal.screenshot("affordance_polish/amend-preview-120");
}
