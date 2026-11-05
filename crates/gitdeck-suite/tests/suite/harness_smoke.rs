use std::path::{Path, PathBuf};

use gitdeck_testkit::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::border;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph, Widget};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const SCREEN_WIDTH: u16 = 160;
const SCREEN_HEIGHT: u16 = 48;
const PANES_LEFT_MARGIN: u16 = 4;
const PANES_TOP_MARGIN: u16 = 1;
const COMMIT_PANE_INITIAL_WIDTH: u16 = 34;
const COMMIT_PANE_MIN_WIDTH: u16 = 20;
const COMMIT_PANE_MAX_WIDTH: u16 = 50;
const DETAILS_PANE_WIDTH: u16 = 40;
const FOCUSED_PANE_COLOR: Color = Color::Rgb(0x35, 0x74, 0xF0);
const INACTIVE_PANE_COLOR: Color = Color::Rgb(0x39, 0x3B, 0x40);
const STATUS_LINE_COLOR: Color = Color::Rgb(0xBC, 0xBE, 0xC4);
const STATUS_LINE: &str = "1 file committed";
const CHANGED_FILE_LINES: [&str; 3] = [
    "M  src/main.rs",
    "A  src/new_file.rs",
    "?  修复/中文文件.txt",
];
const LOG_FILTER_LINE: &str = "Text or hash";
const LOG_COMMIT_LINE: &str = "* a1b2c3d 修复: 冲突已解决  demo-user  17/6/26 6:56pm";
const LOG_LINES: [&str; 2] = [LOG_FILTER_LINE, LOG_COMMIT_LINE];
const DETAILS_LINES: [&str; 2] = ["main -> origin/main", "in 12 branches"];
const CJK_SUBJECT: &str = "修复: 冲突已解决";
const CJK_SUBJECT_COLUMN: u16 = 10;
const COMMIT_PANE_TITLE: &str = "Commit";
const LOG_PANE_TITLE: &str = "Log";
const DETAILS_PANE_TITLE: &str = "Details";
const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
    [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];
const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};
const MISSING_GLYPHS: [char; 10] = ['修', '复', '冲', '突', '已', '解', '决', '中', '文', '件'];
const CELL_PIXEL_WIDTH: u32 = 16;
const CELL_PIXEL_HEIGHT: u32 = 32;
const GIT_HEAD_REVISION_LENGTH: usize = 40;
const RICH_UNICODE_BORDERS: [char; 2] = ['┌', '│'];
const BASIC_ASCII_BORDERS: [char; 2] = ['+', '|'];
const DIVIDER_DRAG_COLUMN_OFFSET: u16 = 8;

struct DemoScreen {
    width: u16,
    height: u16,
    focus: PaneId,
    commit_width: u16,
    caps: CapsProfile,
    shows_data: bool,
    dragging_divider: bool,
}

impl DemoScreen {
    fn new(width: u16, height: u16, caps: CapsProfile) -> Self {
        Self {
            width,
            height,
            focus: PaneId::Changes,
            commit_width: COMMIT_PANE_INITIAL_WIDTH,
            caps,
            shows_data: true,
            dragging_divider: false,
        }
    }

    fn chrome_only(width: u16, height: u16, caps: CapsProfile) -> Self {
        Self {
            shows_data: false,
            ..Self::new(width, height, caps)
        }
    }

    fn screen_area(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }

    fn pane_rect_within(&self, area: Rect, pane: PaneId) -> Rect {
        let pane_height = area.height.saturating_sub(PANES_TOP_MARGIN + 1);
        let pane_top = area.y + PANES_TOP_MARGIN;
        let details_left = area.width.saturating_sub(DETAILS_PANE_WIDTH);
        match pane {
            PaneId::Changes => Rect::new(
                area.x + PANES_LEFT_MARGIN,
                pane_top,
                self.commit_width,
                pane_height,
            ),
            PaneId::Log => Rect::new(
                area.x + PANES_LEFT_MARGIN + self.commit_width,
                pane_top,
                details_left.saturating_sub(PANES_LEFT_MARGIN + self.commit_width),
                pane_height,
            ),
            PaneId::Details => Rect::new(
                area.x + details_left,
                pane_top,
                DETAILS_PANE_WIDTH,
                pane_height,
            ),
        }
    }

    fn divider_column(&self) -> u16 {
        PANES_LEFT_MARGIN + self.commit_width - 1
    }

    fn pane_title(&self, pane: PaneId) -> &'static str {
        match pane {
            PaneId::Changes => COMMIT_PANE_TITLE,
            PaneId::Log => LOG_PANE_TITLE,
            PaneId::Details => DETAILS_PANE_TITLE,
        }
    }

    fn pane_lines(&self, pane: PaneId) -> &'static [&'static str] {
        if !self.shows_data {
            return &[];
        }
        match pane {
            PaneId::Changes => &CHANGED_FILE_LINES,
            PaneId::Log => &LOG_LINES,
            PaneId::Details => &DETAILS_LINES,
        }
    }

    fn handle_key_input(&mut self, key_input: KeyInput) {
        match key_input.code {
            KeyCode::Tab => self.focus = focus_after_tab(self.focus),
            KeyCode::BackTab => self.focus = focus_after_back_tab(self.focus),
            _ => {}
        }
    }

    fn handle_mouse_input(&mut self, mouse_input: MouseInput) {
        if mouse_input.button != MouseButton::Left {
            return;
        }
        match mouse_input.kind {
            MouseInputKind::Down => {
                self.dragging_divider = mouse_input.column == self.divider_column();
            }
            MouseInputKind::Drag => {
                if self.dragging_divider {
                    self.commit_width = commit_width_for_divider_column(mouse_input.column);
                }
            }
            MouseInputKind::Up => self.dragging_divider = false,
            MouseInputKind::Move | MouseInputKind::ScrollUp | MouseInputKind::ScrollDown => {}
        }
    }

    fn border_set(&self) -> border::Set<'static> {
        match self.caps {
            CapsProfile::Rich | CapsProfile::Standard => border::PLAIN,
            CapsProfile::Basic => ASCII_BORDER,
        }
    }

    fn render_pane(&self, area: Rect, buffer: &mut Buffer, pane: PaneId) {
        let accent_color = if pane == self.focus {
            FOCUSED_PANE_COLOR
        } else {
            INACTIVE_PANE_COLOR
        };
        let block = Block::bordered()
            .border_set(self.border_set())
            .border_style(Style::default().fg(accent_color))
            .title(Line::styled(
                self.pane_title(pane),
                Style::default().fg(accent_color),
            ));
        let body: Vec<Line> = self
            .pane_lines(pane)
            .iter()
            .map(|line| Line::from(*line))
            .collect();
        Paragraph::new(body)
            .block(block)
            .render(self.pane_rect_within(area, pane), buffer);
    }

    fn render_status_line(&self, area: Rect, buffer: &mut Buffer) {
        if area.height > PANES_TOP_MARGIN {
            buffer.set_string(
                area.x,
                area.bottom() - 1,
                STATUS_LINE,
                Style::default().fg(STATUS_LINE_COLOR),
            );
        }
    }
}

impl ScreenDriver for DemoScreen {
    fn handle_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key(key_input) => self.handle_key_input(key_input),
            InputEvent::Mouse(mouse_input) => self.handle_mouse_input(mouse_input),
            InputEvent::Resize(size) => {
                self.width = size.width;
                self.height = size.height;
            }
            InputEvent::Paste(_) | InputEvent::Tick => {}
        }
    }

    fn render_frame(&self, area: Rect, buffer: &mut Buffer) {
        for pane in [PaneId::Changes, PaneId::Log, PaneId::Details] {
            self.render_pane(area, buffer, pane);
        }
        self.render_status_line(area, buffer);
    }

    fn focused_pane(&self) -> PaneId {
        self.focus
    }

    fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
        Some(self.pane_rect_within(self.screen_area(), pane))
    }
}

fn focus_after_tab(pane: PaneId) -> PaneId {
    match pane {
        PaneId::Changes => PaneId::Log,
        PaneId::Log => PaneId::Details,
        PaneId::Details => PaneId::Changes,
    }
}

fn focus_after_back_tab(pane: PaneId) -> PaneId {
    match pane {
        PaneId::Changes => PaneId::Details,
        PaneId::Log => PaneId::Changes,
        PaneId::Details => PaneId::Log,
    }
}

fn commit_width_for_divider_column(column: u16) -> u16 {
    column
        .saturating_sub(PANES_LEFT_MARGIN - 1)
        .clamp(COMMIT_PANE_MIN_WIDTH, COMMIT_PANE_MAX_WIDTH)
}

fn demo_terminal(caps: CapsProfile) -> TestTerminal<DemoScreen> {
    TestTerminal::new(
        DemoScreen::new(SCREEN_WIDTH, SCREEN_HEIGHT, caps),
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        caps,
        ThemeId::JbDark,
    )
}

fn chrome_only_terminal(caps: CapsProfile) -> TestTerminal<DemoScreen> {
    TestTerminal::new(
        DemoScreen::chrome_only(SCREEN_WIDTH, SCREEN_HEIGHT, caps),
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        caps,
        ThemeId::JbDark,
    )
}

fn png_dimensions(png_path: &Path) -> (u32, u32) {
    let header = std::fs::read(png_path).expect("screenshot png is readable");
    (
        u32::from_be_bytes([header[16], header[17], header[18], header[19]]),
        u32::from_be_bytes([header[20], header[21], header[22], header[23]]),
    )
}

fn character_columns(character: char) -> u16 {
    u16::try_from(UnicodeWidthChar::width(character).unwrap_or(1))
        .unwrap_or(1)
        .max(1)
}

fn display_columns(text: &str) -> u16 {
    u16::try_from(text.width()).unwrap_or(u16::MAX)
}

fn assert_rendered_text_columns(
    terminal: &TestTerminal<DemoScreen>,
    row: u16,
    start_column: u16,
    text: &str,
) -> u16 {
    let mut column = start_column;
    for character in text.chars() {
        assert_eq!(
            terminal.cell(column, row).symbol,
            character.to_string(),
            "glyph {character:?} does not sit at column {column}"
        );
        column += character_columns(character);
    }
    column
}

fn assert_profile_border_glyphs(caps: CapsProfile, screen_text: &str) {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => {
            for glyph in RICH_UNICODE_BORDERS {
                assert!(
                    screen_text.contains(glyph),
                    "{} profile must draw the unicode border glyph {glyph:?}",
                    caps.as_str()
                );
            }
        }
        CapsProfile::Basic => {
            for glyph in BASIC_ASCII_BORDERS {
                assert!(
                    screen_text.contains(glyph),
                    "{} profile must draw the ascii border glyph {glyph:?}",
                    caps.as_str()
                );
            }
            for glyph in RICH_UNICODE_BORDERS {
                assert!(
                    !screen_text.contains(glyph),
                    "{} profile must downgrade the unicode border glyph {glyph:?}",
                    caps.as_str()
                );
            }
        }
    }
}

fn log_line_bounds(terminal: &TestTerminal<DemoScreen>, row: u16, log_pane: Rect) -> (u16, u16) {
    let scan_columns = log_pane.x + 1..log_pane.right() - 1;
    let first_column = scan_columns
        .clone()
        .find(|column| terminal.cell(*column, row).symbol != " ")
        .expect("the log line starts inside the log pane");
    let last_column = scan_columns
        .rev()
        .find(|column| terminal.cell(*column, row).symbol != " ")
        .expect("the log line ends inside the log pane");
    (first_column, last_column)
}

fn assert_commit_row_is_contiguous(
    terminal: &TestTerminal<DemoScreen>,
    row: u16,
    first_column: u16,
    last_column: u16,
) {
    assert_eq!(
        assert_rendered_text_columns(terminal, row, first_column, LOG_COMMIT_LINE),
        last_column + 1,
        "the rendered commit row is contiguous over the whole log line"
    );
    assert_eq!(
        last_column - first_column + 1,
        display_columns(LOG_COMMIT_LINE),
        "the rendered row occupies the display width of its characters"
    );
}

fn assert_cjk_subject_cells(terminal: &TestTerminal<DemoScreen>, row: u16, first_column: u16) {
    assert!(
        terminal.screen_lines()[usize::from(row)].contains(CJK_SUBJECT),
        "the commit row carries the cjk subject"
    );
    let subject_column = first_column + CJK_SUBJECT_COLUMN;
    assert_eq!(terminal.cell(subject_column, row).symbol, "修");
    assert_eq!(
        terminal.cell(subject_column + 1, row).symbol,
        " ",
        "the second column of a wide glyph holds no symbol"
    );
    assert_eq!(terminal.cell(subject_column + 2, row).symbol, "复");
}

fn fixture_git_config_path() -> PathBuf {
    repository_root().join("artifacts/fixtures/gitconfig")
}

fn read_head_revision(fixture_path: &Path) -> String {
    let output = fixture_git_command(fixture_path)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .expect("git rev-parse runs inside the sandbox");
    assert!(
        output.status.success(),
        "git rev-parse failed in {}: {}",
        fixture_path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn three_capability_profiles_render_borders() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = demo_terminal(caps);
        terminal.tick();
        let screen_text = terminal.screen_text();
        let report = terminal.screenshot(&format!(
            "suite/three-capability-profiles-{}",
            caps.as_str()
        ));
        assert!(
            report.png_path.is_file(),
            "{} profile produced no png",
            caps.as_str()
        );
        assert_profile_border_glyphs(caps, &screen_text);
    }
}

#[test]
fn chrome_carries_no_missing_glyphs() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = chrome_only_terminal(caps);
        terminal.tick();
        terminal.assert_no_missing_glyphs();
    }
}

#[test]
fn every_capability_profile_matches_its_demo_snapshot() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = demo_terminal(caps);
        terminal.tick();
        terminal.assert_snapshot(&format!("demo_screen_{}", caps.as_str()));
    }
}

#[test]
fn mouse_drag_on_divider_resizes_commit_pane() {
    let mut terminal = demo_terminal(CapsProfile::Rich);
    terminal.tick();
    let commit_pane_before = terminal.find_pane(PaneId::Changes).expect("changes pane");
    let log_pane_before = terminal.find_pane(PaneId::Log).expect("log pane");
    let divider_column = commit_pane_before.right() - 1;
    terminal.mouse_drag(
        (divider_column, 3),
        (divider_column + DIVIDER_DRAG_COLUMN_OFFSET, 3),
    );
    let commit_pane_after = terminal.find_pane(PaneId::Changes).expect("changes pane");
    let log_pane_after = terminal.find_pane(PaneId::Log).expect("log pane");
    assert_eq!(
        commit_pane_after.width,
        commit_pane_before.width + DIVIDER_DRAG_COLUMN_OFFSET,
        "dragging the divider right widens the commit pane"
    );
    assert_eq!(
        log_pane_after.x,
        log_pane_before.x + DIVIDER_DRAG_COLUMN_OFFSET,
        "log pane left edge follows the divider"
    );
    assert_eq!(
        commit_pane_after.right(),
        log_pane_after.x,
        "panes stay adjacent after the drag"
    );
    terminal.screenshot("suite/mouse-drag-commit-pane-resized");
}

#[test]
fn tab_and_backtab_cycle_focus() {
    let mut terminal = demo_terminal(CapsProfile::Standard);
    terminal.tick();
    terminal.assert_focused(PaneId::Changes);
    for expected_pane in [PaneId::Log, PaneId::Details, PaneId::Changes] {
        terminal.send_keys("tab");
        terminal.assert_focused(expected_pane);
    }
    terminal.screenshot("suite/focus-cycle-forward");
    for expected_pane in [PaneId::Details, PaneId::Log, PaneId::Changes] {
        terminal.send_keys("backtab");
        terminal.assert_focused(expected_pane);
    }
    terminal.screenshot("suite/focus-cycle-backward");
}

#[test]
fn cjk_subject_occupies_two_columns() {
    let mut terminal = demo_terminal(CapsProfile::Rich);
    terminal.tick();
    let report = terminal.screenshot("suite/cjk-subject-columns");
    let screenshot_text =
        std::fs::read_to_string(&report.text_path).expect("screenshot text is readable");
    assert_eq!(screenshot_text, terminal.screen_text());
    let row = terminal
        .find_line(CJK_SUBJECT)
        .expect("the cjk subject is on screen");
    let log_pane = terminal.find_pane(PaneId::Log).expect("log pane");
    let (first_column, last_column) = log_line_bounds(&terminal, row, log_pane);
    assert_commit_row_is_contiguous(&terminal, row, first_column, last_column);
    assert_cjk_subject_cells(&terminal, row, first_column);
    assert_eq!(
        display_columns(CJK_SUBJECT),
        16,
        "seven wide glyphs plus a colon and a space"
    );
}

#[test]
fn every_profile_writes_png_txt_and_ansi() {
    for caps in EVERY_CAPS_PROFILE {
        let mut terminal = demo_terminal(caps);
        terminal.tick();
        let report = terminal.screenshot(&format!("suite/every-profile-{}", caps.as_str()));
        for artifact_path in [&report.png_path, &report.text_path, &report.ansi_path] {
            assert!(
                artifact_path.is_file(),
                "{} profile is missing {}",
                caps.as_str(),
                artifact_path.display()
            );
        }
        assert_eq!(
            png_dimensions(&report.png_path),
            (
                u32::from(SCREEN_WIDTH) * CELL_PIXEL_WIDTH,
                u32::from(SCREEN_HEIGHT) * CELL_PIXEL_HEIGHT
            ),
            "{} profile png covers every cell at the raster scale",
            caps.as_str()
        );
        assert_eq!(
            report.missing_glyphs.len(),
            MISSING_GLYPHS.len(),
            "{} profile must report every cjk glyph the bundled font lacks",
            caps.as_str()
        );
        terminal.assert_missing_glyphs(&MISSING_GLYPHS);
    }
}

#[test]
fn rich_demo_screen_matches_its_golden_baseline() {
    let mut terminal = demo_terminal(CapsProfile::Rich);
    terminal.tick();
    terminal.assert_pixels_close(
        "harness_smoke/demo-screen-rich",
        PixelTolerance {
            max_channel_delta: 0,
            max_pixel_ratio: 0.0,
        },
    );
}

#[test]
fn dirty_fixture_repo_is_usable() {
    let mut terminal = demo_terminal(CapsProfile::Rich);
    terminal.tick();
    let fixture_path = fixture_repo(FixtureRepo::Dirty);
    assert!(
        fixture_path.is_dir(),
        "dirty fixture directory {} is missing",
        fixture_path.display()
    );
    let git_config_global = fixture_git_config_path();
    assert!(
        git_config_global.is_file(),
        "fixture git config {} is missing",
        git_config_global.display()
    );
    let head_revision = read_head_revision(&fixture_path);
    assert_eq!(head_revision.len(), GIT_HEAD_REVISION_LENGTH);
    assert!(
        head_revision
            .chars()
            .all(|character| character.is_ascii_hexdigit()),
        "HEAD {head_revision} is not a hexadecimal object name"
    );
    terminal.screenshot("suite/harness-demo-screen");
}
