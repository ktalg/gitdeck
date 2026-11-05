use gitdeck_core::{InputEvent, KeyCode, PaneId};
use gitdeck_testkit::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Widget};

const PANE_TITLES: [&str; 3] = ["Commit", "Log", "Details"];
const MISSING_GLYPHS: [char; 10] = ['修', '复', '冲', '突', '已', '解', '决', '中', '文', '件'];

struct ThreePaneDriver {
    area: Rect,
    focused: PaneId,
}

impl ThreePaneDriver {
    fn new(area: Rect) -> Self {
        Self {
            area,
            focused: PaneId::Changes,
        }
    }
}

fn pane_areas(area: Rect) -> [Rect; 3] {
    Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Fill(1),
        Constraint::Percentage(30),
    ])
    .areas(area)
}

fn pane_index(pane: PaneId) -> usize {
    match pane {
        PaneId::Changes => 0,
        PaneId::Log => 1,
        PaneId::Details => 2,
    }
}

impl ScreenDriver for ThreePaneDriver {
    fn handle_event(&mut self, event: InputEvent) {
        let InputEvent::Key(key_input) = event else {
            return;
        };
        if key_input.code != KeyCode::Tab {
            return;
        }
        self.focused = match self.focused {
            PaneId::Changes => PaneId::Log,
            PaneId::Log => PaneId::Details,
            PaneId::Details => PaneId::Changes,
        };
    }

    fn render_frame(&self, area: Rect, buffer: &mut Buffer) {
        for (index, pane_area) in pane_areas(area).into_iter().enumerate() {
            let block = Block::bordered().title(PANE_TITLES[index]);
            block.render(pane_area, buffer);
        }
        let [changes_area, log_area, details_area] = pane_areas(area);
        buffer.set_string(
            changes_area.x + 1,
            changes_area.y + 1,
            "修复: 冲突已解决",
            Style::default().fg(Color::Rgb(0xCC, 0x78, 0x32)),
        );
        buffer.set_string(
            log_area.x + 1,
            log_area.y + 1,
            "├─ src/中文文件.txt",
            Style::default().fg(Color::Rgb(0x9C, 0xDC, 0xFE)),
        );
        buffer.set_string(
            details_area.x + 1,
            details_area.y + 1,
            "main -> origin/main",
            Style::default().fg(Color::Indexed(110)),
        );
    }

    fn focused_pane(&self) -> PaneId {
        self.focused
    }

    fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
        Some(pane_areas(self.area)[pane_index(pane)])
    }
}

fn screenshot_pipeline_terminal() -> TestTerminal<ThreePaneDriver> {
    TestTerminal::new(
        ThreePaneDriver::new(Rect::new(0, 0, 120, 32)),
        120,
        32,
        CapsProfile::Rich,
        ThemeId::JbDark,
    )
}

fn png_dimensions(png_path: &std::path::Path) -> (u32, u32) {
    let header = std::fs::read(png_path).expect("screenshot png is readable");
    (
        u32::from_be_bytes([header[16], header[17], header[18], header[19]]),
        u32::from_be_bytes([header[20], header[21], header[22], header[23]]),
    )
}

#[test]
fn three_pane_render_writes_png_txt_and_ansi_side_by_side() {
    let mut terminal = screenshot_pipeline_terminal();
    terminal.tick();
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    let report = terminal.screenshot("testkit/screenshot-pipeline");

    assert!(report.png_path.exists(), "png exists");
    assert!(report.text_path.exists(), "txt exists");
    assert!(report.ansi_path.exists(), "ansi exists");
    assert_eq!(png_dimensions(&report.png_path), (120 * 16, 32 * 32));

    let text = std::fs::read_to_string(&report.text_path).expect("txt is readable");
    assert_eq!(text, terminal.screen_text());
    assert!(text.contains("Commit"), "commit pane title missing");
    assert!(text.contains("Log"), "log pane title missing");
    assert!(text.contains("Details"), "details pane title missing");
    assert!(
        text.contains("├─ src/"),
        "box drawing glyphs missing from text"
    );

    let ansi = std::fs::read_to_string(&report.ansi_path).expect("ansi is readable");
    assert!(ansi.contains("\u{1b}[38;2;204;120;50m"));
}

#[test]
fn box_drawing_glyphs_are_rendered_and_never_reported_missing() {
    let terminal = screenshot_pipeline_terminal();
    let report = terminal.screenshot("testkit/screenshot-pipeline-box-drawing");
    let box_drawing_characters = ['┌', '┐', '└', '┘', '─', '│', '├', '┤'];
    for character in box_drawing_characters {
        assert!(
            !report.missing_glyphs.contains(&character),
            "box drawing glyph {character:?} has no bundled bitmap"
        );
    }
}

#[test]
fn cjk_glyphs_are_missing_in_the_exact_expected_set() {
    let terminal = screenshot_pipeline_terminal();
    let report = terminal.screenshot("testkit/screenshot-pipeline-cjk");
    assert_eq!(
        report.missing_glyphs.len(),
        MISSING_GLYPHS.len(),
        "cjk text must be reported as missing glyphs while font8x8 has no cjk bitmaps"
    );
    terminal.assert_missing_glyphs(&MISSING_GLYPHS);
}

#[test]
fn tab_key_cycles_focus_across_the_three_panes() {
    let mut terminal = screenshot_pipeline_terminal();
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Log);
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Details);
    terminal.send_keys("tab");
    assert_eq!(terminal.focused_pane(), PaneId::Changes);
}

#[test]
fn pane_rects_partition_the_screen_horizontally() {
    let terminal = screenshot_pipeline_terminal();
    let changes = terminal.pane_rect(PaneId::Changes).expect("changes pane");
    let log = terminal.pane_rect(PaneId::Log).expect("log pane");
    let details = terminal.pane_rect(PaneId::Details).expect("details pane");
    assert_eq!(changes.x, 0);
    assert_eq!(changes.right(), log.x);
    assert_eq!(log.right(), details.x);
    assert_eq!(details.right(), 120);
}

#[test]
fn rendered_screen_matches_the_text_snapshot() {
    let mut terminal = screenshot_pipeline_terminal();
    terminal.tick();
    terminal.send_keys("tab");
    terminal.assert_snapshot("three_pane_screen");
}

#[test]
fn wide_character_cells_occupy_two_columns_on_screen() {
    let mut terminal = screenshot_pipeline_terminal();
    terminal.tick();
    let row = terminal.find_line("修复").expect("cjk line is on screen");
    let cell = terminal.cell(1, row);
    assert_eq!(cell.symbol.chars().count(), 1);
    assert_eq!(terminal.cell(2, row).symbol, " ");
}
