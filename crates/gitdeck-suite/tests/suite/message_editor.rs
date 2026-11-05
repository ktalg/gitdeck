use std::fs;
use std::path::PathBuf;

use gitdeck_app::App;
use gitdeck_core::{ChangeKind, PaneId};
use gitdeck_testkit::prelude::*;
use gitdeck_tui::{
    box_glyphs, changes_panel_layout, file_count_label, text_display_width, theme_palette,
    ChangesPanelLayout, StatusTone, COMMIT_MESSAGE_COUNT_SUFFIX,
};
use ratatui::layout::Rect;

const REFERENCE_WIDTH: u16 = 160;
const REFERENCE_HEIGHT: u16 = 48;
const WIDE_WIDTH: u16 = 260;
const WIDE_HEIGHT: u16 = 60;
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/message-editor";
const LONG_MESSAGE: &str = "wrap the commit message so every typed character stays shown";
const CHINESE_MESSAGE: &str = "修复提交消息编辑器里的折行显示问题不能把双宽字符切成两半";

fn message_repository(case: &str, caps: CapsProfile) -> PathBuf {
    let target = repository_root()
        .join(SCRATCH_REPOSITORY_DIRECTORY)
        .join(format!("{case}-{}", caps.as_str()));
    if target.exists() {
        fs::remove_dir_all(&target).expect("the stale scratch repository is removed");
    }
    copy_directory(&fixture_repo(FixtureRepo::Dirty), &target)
        .expect("the dirty fixture repository is copied");
    target
}

fn dirty_terminal(case: &str, width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let repository_path = message_repository(case, caps);
    let mut terminal = TestTerminal::new(
        App::with_repository(&repository_path, caps, ThemeId::JbDark)
            .expect("the scratch repository opens"),
        width,
        height,
        caps,
        ThemeId::JbDark,
    );
    terminal.resize(width, height);
    terminal.tick();
    terminal
}

fn panel_layout(terminal: &TestTerminal<App>) -> ChangesPanelLayout {
    changes_panel_layout(
        terminal
            .pane_rect(PaneId::Changes)
            .expect("the changes pane is visible"),
        false,
    )
    .expect("the changes pane holds the commit block")
}

fn row_text(terminal: &TestTerminal<App>, row: u16, area: Rect) -> String {
    (area.x..area.right())
        .map(|column| terminal.cell(column, row).symbol)
        .collect()
}

fn message_rows(terminal: &TestTerminal<App>, layout: ChangesPanelLayout) -> Vec<String> {
    (layout.message_lines.y..layout.message_lines.bottom())
        .map(|row| {
            row_text(terminal, row, layout.message_lines)
                .trim_end()
                .to_string()
        })
        .collect()
}

fn rows_with_content(rows: &[String]) -> Vec<String> {
    rows.iter()
        .filter(|row| !row.trim().is_empty())
        .cloned()
        .collect()
}

fn without_spaces(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn typed_terminal(case: &str, width: u16, height: u16, caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = dirty_terminal(case, width, height, caps);
    terminal.send_keys("ctrl+k");
    terminal.send_keys("i");
    terminal.send_text(LONG_MESSAGE);
    terminal
}

fn untracked_checked_count(terminal: &TestTerminal<App>) -> usize {
    terminal
        .driver()
        .changes()
        .iter()
        .filter(|entry| entry.kind == ChangeKind::Untracked)
        .count()
}

fn uncheck_the_unversioned_group(terminal: &mut TestTerminal<App>) {
    let tracked_count = terminal
        .driver()
        .changes()
        .iter()
        .filter(|entry| entry.kind != ChangeKind::Untracked)
        .count();
    for _ in 0..=tracked_count {
        terminal.send_keys("j");
    }
    terminal.send_keys("space");
}

#[test]
fn the_message_box_wraps_long_messages() {
    assert_eq!(
        LONG_MESSAGE.chars().count(),
        60,
        "the message holds sixty characters"
    );
    for caps in EVERY_CAPS_PROFILE {
        let terminal = typed_terminal("wrap", REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let layout = panel_layout(&terminal);
        assert_eq!(
            layout.message_lines.width,
            terminal
                .pane_rect(PaneId::Changes)
                .expect("the changes pane is visible")
                .width
                - 4,
            "{caps:?} the message box spans the changes pane"
        );
        let rows = rows_with_content(&message_rows(&terminal, layout));
        assert!(
            rows.len() > 1,
            "{caps:?} the message wraps into several rows, rows were {rows:?}"
        );
        assert_eq!(
            without_spaces(&rows.join("")),
            without_spaces(LONG_MESSAGE),
            "{caps:?} every typed character stays visible, rows were {rows:?}"
        );
        assert!(
            !rows.iter().any(|row| row.contains('…')),
            "{caps:?} the message box never ellipsizes the message, rows were {rows:?}"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("message_editor/wrapped-long-message");
        }
    }
}

#[test]
fn wrapping_never_splits_a_wide_character() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = dirty_terminal("wide", REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.send_keys("ctrl+k");
        terminal.send_keys("i");
        terminal.send_text(CHINESE_MESSAGE);
        let layout = panel_layout(&terminal);
        let rows = rows_with_content(&message_rows(&terminal, layout));
        assert!(
            rows.len() > 1,
            "{caps:?} the chinese message wraps, rows were {rows:?}"
        );
        assert_eq!(
            without_spaces(&rows.join("")),
            CHINESE_MESSAGE,
            "{caps:?} no character is dropped or split, rows were {rows:?}"
        );
        for row in &rows {
            let content_width = text_display_width(&without_spaces(row));
            assert!(
                content_width <= layout.message_lines.width,
                "{caps:?} row {row:?} stays inside {} columns",
                layout.message_lines.width
            );
            assert_eq!(
                content_width % 2,
                0,
                "{caps:?} row {row:?} holds whole wide characters only"
            );
        }
        let box_right = layout.message_box.right() - 1;
        let vertical = box_glyphs(caps).vertical.to_string();
        for row in layout.message_lines.y..layout.message_lines.bottom() {
            assert_eq!(
                terminal.cell(box_right, row).symbol,
                vertical,
                "{caps:?} the right border survives row {row}"
            );
        }
        if caps == CapsProfile::Rich {
            terminal.screenshot("message_editor/wrapped-chinese-message");
        }
    }
}

#[test]
fn the_caret_stays_visible_on_a_wrapped_line() {
    for caps in EVERY_CAPS_PROFILE {
        let terminal = typed_terminal("caret", REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let layout = panel_layout(&terminal);
        let theme = theme_palette(ThemeId::JbDark);
        let caret_cells: Vec<(u16, u16)> = (layout.message_lines.y..layout.message_lines.bottom())
            .flat_map(|row| {
                (layout.message_lines.x..layout.message_lines.right())
                    .map(move |column| (column, row))
                    .collect::<Vec<(u16, u16)>>()
            })
            .filter(|(column, row)| {
                let cell = terminal.cell(*column, *row);
                cell.background == theme.text && cell.foreground == theme.panel
            })
            .collect();
        assert_eq!(
            caret_cells.len(),
            1,
            "{caps:?} exactly one caret is on screen, cells were {caret_cells:?}"
        );
        let (caret_column, caret_row) = caret_cells[0];
        let wrapped_row_count = rows_with_content(&message_rows(&terminal, layout)).len();
        assert_eq!(
            caret_row,
            layout.message_lines.y + u16::try_from(wrapped_row_count - 1).unwrap(),
            "{caps:?} the caret sits on the last wrapped row"
        );
        assert!(
            caret_column < layout.message_lines.right(),
            "{caps:?} the caret column stays inside the box"
        );
    }
}

#[test]
fn the_message_box_shows_the_character_count() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = typed_terminal("count", REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        let layout = panel_layout(&terminal);
        let expected = format!("60{COMMIT_MESSAGE_COUNT_SUFFIX}");
        let title_row = row_text(&terminal, layout.message_box.y, layout.message_box);
        assert!(
            title_row.contains(&expected),
            "{caps:?} the title row shows {expected:?}, row was {title_row:?}"
        );
        terminal.send_text("修复");
        let expected = format!("62{COMMIT_MESSAGE_COUNT_SUFFIX}");
        let title_row = row_text(&terminal, layout.message_box.y, layout.message_box);
        assert!(
            title_row.contains(&expected),
            "{caps:?} two wide characters count as two characters, row was {title_row:?}"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("message_editor/character-count");
        }
    }
}

#[test]
fn amend_shows_how_many_files_will_join() {
    let mut terminal = dirty_terminal("amend-notice", WIDE_WIDTH, WIDE_HEIGHT, CapsProfile::Rich);
    terminal.send_keys("ctrl+k");
    uncheck_the_unversioned_group(&mut terminal);
    terminal.send_keys("alt+a");
    let layout = panel_layout(&terminal);
    let checked_file_count = terminal.driver().checked_file_paths().len();
    let row = row_text(&terminal, layout.amend_row.y, layout.amend_row);
    let expected = format!(
        "({} will be amended in)",
        file_count_label(checked_file_count)
    );
    assert!(
        row.contains(&expected),
        "the amend row announces the checked files, row was {row:?}"
    );
    terminal.screenshot("message_editor/amend-file-notice");
}

#[test]
fn amend_warns_about_untracked_files() {
    let mut terminal = dirty_terminal(
        "amend-untracked",
        WIDE_WIDTH,
        WIDE_HEIGHT,
        CapsProfile::Rich,
    );
    terminal.send_keys("ctrl+k");
    let untracked_count = untracked_checked_count(&terminal);
    terminal.send_keys("alt+a");
    let layout = panel_layout(&terminal);
    let row = row_text(&terminal, layout.amend_row.y, layout.amend_row);
    let expected = format!("{untracked_count} untracked");
    assert!(
        row.contains(&expected),
        "the amend row warns about the untracked draft, row was {row:?}"
    );
    let theme = theme_palette(ThemeId::JbDark);
    let warning_cells: Vec<u16> = (layout.amend_row.x..layout.amend_row.right())
        .filter(|column| terminal.cell(*column, layout.amend_row.y).foreground == theme.warning)
        .collect();
    assert!(
        !warning_cells.is_empty(),
        "the untracked warning uses the warning color, row was {row:?}"
    );
    terminal.screenshot("message_editor/amend-untracked-warning");
}

#[test]
fn amend_status_message_reports_the_untracked_count() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = dirty_terminal("amend-receipt", REFERENCE_WIDTH, REFERENCE_HEIGHT, caps);
        terminal.send_keys("ctrl+k");
        let untracked_count = untracked_checked_count(&terminal);
        assert!(untracked_count > 0, "{caps:?} the fixture holds a draft");
        terminal.send_keys("alt+a");
        terminal.send_keys("ctrl+enter");
        let expected = format!("{untracked_count} untracked file amended into the last commit");
        assert_eq!(
            terminal.driver().status_message(),
            Some((expected.as_str(), StatusTone::Warning)),
            "{caps:?} the amend reports the untracked file, screen was:\n{}",
            terminal.screen_text()
        );
        assert!(
            terminal.screen_text().contains(&expected),
            "{caps:?} the warning is on the status bar"
        );
        if caps == CapsProfile::Rich {
            terminal.screenshot("message_editor/amend-untracked-receipt");
        }
    }
}
