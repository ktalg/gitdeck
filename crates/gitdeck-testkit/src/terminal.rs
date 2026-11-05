use std::path::Path;

use gitdeck_caps::{CapsProfile, ThemeId};
use gitdeck_core::{
    InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInput, MouseInputKind, PaneId,
    Size,
};
use gitdeck_tui::ScreenDriver;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

use crate::fixture::{fixture_repo, FixtureRepo};
use crate::key_sequence::parse_key_sequence;
use crate::paths::{module_path_from_source_file, repository_root};
use crate::pixel_baseline::{compare_with_golden, PixelDiff, PixelTolerance};
use crate::raster::{missing_glyphs, write_screenshot};
use crate::screen_text::{buffer_lines, buffer_text};
use crate::screenshot_report::ScreenshotReport;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCell {
    pub symbol: String,
    pub foreground: Color,
    pub background: Color,
    pub modifier: Modifier,
}

pub struct TestTerminal<D: ScreenDriver> {
    driver: D,
    width: u16,
    height: u16,
    caps: CapsProfile,
    theme: ThemeId,
    buffer: Buffer,
    input_clock: u64,
}

impl<D: ScreenDriver> TestTerminal<D> {
    pub fn new(driver: D, width: u16, height: u16, caps: CapsProfile, theme: ThemeId) -> Self {
        let mut terminal = Self {
            driver,
            width,
            height,
            caps,
            theme,
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            input_clock: 0,
        };
        terminal.render_into_buffer();
        terminal
    }

    pub fn for_fixture(
        fixture: FixtureRepo,
        width: u16,
        height: u16,
        caps: CapsProfile,
        theme: ThemeId,
        build_driver: impl FnOnce(&Path) -> D,
    ) -> Self {
        let fixture_path = fixture_repo(fixture);
        let driver = build_driver(&fixture_path);
        Self::new(driver, width, height, caps, theme)
    }

    pub fn driver(&self) -> &D {
        &self.driver
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    pub fn caps(&self) -> CapsProfile {
        self.caps
    }

    pub fn send_keys(&mut self, keys: &str) {
        for key_input in parse_key_sequence(keys) {
            self.deliver_event_to_driver(InputEvent::Key(key_input));
        }
        self.render_into_buffer();
    }

    pub fn send_text(&mut self, text: &str) {
        for character in text.chars() {
            self.deliver_event_to_driver(InputEvent::Key(KeyInput {
                code: KeyCode::Char(character),
                modifiers: KeyModifiers::default(),
            }));
        }
        self.render_into_buffer();
    }

    pub fn mouse_click(&mut self, column: u16, row: u16) {
        self.deliver_click(column, row, 1);
        self.render_into_buffer();
    }

    pub fn mouse_double_click(&mut self, column: u16, row: u16) {
        self.deliver_click(column, row, 1);
        self.deliver_click(column, row, 2);
        self.render_into_buffer();
    }

    pub fn mouse_right_click(&mut self, column: u16, row: u16) {
        self.deliver_mouse(MouseInputKind::Down, MouseButton::Right, column, row);
        self.deliver_mouse(MouseInputKind::Up, MouseButton::Right, column, row);
        self.render_into_buffer();
    }

    pub fn mouse_drag(&mut self, from: (u16, u16), to: (u16, u16)) {
        self.deliver_left_press(from.0, from.1, 1);
        for (column, row) in drag_path(from, to) {
            self.deliver_mouse(MouseInputKind::Drag, MouseButton::Left, column, row);
        }
        self.deliver_mouse(MouseInputKind::Up, MouseButton::Left, to.0, to.1);
        self.render_into_buffer();
    }

    pub fn mouse_wheel(&mut self, column: u16, row: u16, delta: i16) {
        let kind = if delta >= 0 {
            MouseInputKind::ScrollUp
        } else {
            MouseInputKind::ScrollDown
        };
        for _ in 0..delta.unsigned_abs() {
            self.deliver_mouse(kind, MouseButton::None, column, row);
        }
        self.render_into_buffer();
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.deliver_event_to_driver(InputEvent::Resize(Size { width, height }));
        self.render_into_buffer();
    }

    pub fn tick(&mut self) {
        self.input_clock += 1;
        self.deliver_event_to_driver(InputEvent::Tick);
        self.render_into_buffer();
    }

    pub fn screen_text(&self) -> String {
        buffer_text(&self.buffer)
    }

    pub fn screen_lines(&self) -> Vec<String> {
        buffer_lines(&self.buffer)
    }

    pub fn cell(&self, column: u16, row: u16) -> TestCell {
        let cell = self.buffer.cell((column, row)).unwrap_or_else(|| {
            panic!(
                "cell ({column}, {row}) is outside the {}x{} screen",
                self.width, self.height
            )
        });
        TestCell {
            symbol: cell.symbol().to_string(),
            foreground: cell.fg,
            background: cell.bg,
            modifier: cell.modifier,
        }
    }

    pub fn focused_pane(&self) -> PaneId {
        self.driver.focused_pane()
    }

    pub fn find_pane(&self, pane: PaneId) -> Option<Rect> {
        self.driver.pane_rect(pane)
    }

    pub fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
        self.find_pane(pane)
    }

    pub fn find_line(&self, needle: &str) -> Option<u16> {
        self.screen_lines()
            .iter()
            .position(|line| line.contains(needle))
            .and_then(|index| u16::try_from(index).ok())
    }

    pub fn screenshot(&self, name: &str) -> ScreenshotReport {
        let frame = self.render_current_frame();
        write_screenshot(&frame, self.theme, name)
    }

    #[track_caller]
    pub fn assert_snapshot(&self, name: &str) {
        let caller = std::panic::Location::caller();
        let screen_text = self.screen_text();
        let assertion = insta::_macro_support::assert_snapshot(
            (name.to_string(), screen_text.as_str()).into(),
            &repository_root(),
            "",
            &module_path_from_source_file(caller.file()),
            caller.file(),
            caller.line(),
            "self.screen_text()",
        );
        if let Err(error) = assertion {
            panic!("snapshot {name} failed: {error}");
        }
    }

    pub fn assert_cell_style(&self, column: u16, row: u16, foreground: Color, background: Color) {
        let cell = self.cell(column, row);
        assert!(
            cell.foreground == foreground && cell.background == background,
            "cell ({column}, {row}) holds {:?} with foreground {:?} and background {:?}, expected foreground {:?} and background {:?}",
            cell.symbol,
            cell.foreground,
            cell.background,
            foreground,
            background
        );
    }

    pub fn assert_focused(&self, pane: PaneId) {
        let focused = self.focused_pane();
        assert!(
            focused == pane,
            "pane {pane:?} must be focused, {focused:?} is focused"
        );
    }

    pub fn assert_missing_glyphs(&self, expected: &[char]) {
        let actual = self.current_missing_glyphs();
        let mut expected_sorted = expected.to_vec();
        expected_sorted.sort_unstable();
        expected_sorted.dedup();
        let mut actual_sorted = actual.clone();
        actual_sorted.sort_unstable();
        actual_sorted.dedup();
        assert!(
            actual_sorted == expected_sorted,
            "missing glyphs differ: expected {expected_sorted:?}, actual {actual:?}"
        );
    }

    pub fn assert_no_missing_glyphs(&self) {
        self.assert_missing_glyphs(&[]);
    }

    pub fn assert_pixels_close(&self, golden: &str, tolerance: PixelTolerance) -> PixelDiff {
        let frame = self.render_current_frame();
        compare_with_golden(&frame, self.theme, golden, tolerance)
    }

    fn current_missing_glyphs(&self) -> Vec<char> {
        missing_glyphs(&self.buffer)
    }

    fn deliver_event_to_driver(&mut self, event: InputEvent) {
        self.driver.handle_event(event);
    }

    fn deliver_click(&mut self, column: u16, row: u16, click_count: u8) {
        self.deliver_left_press(column, row, click_count);
        self.deliver_mouse(MouseInputKind::Up, MouseButton::Left, column, row);
    }

    fn deliver_left_press(&mut self, column: u16, row: u16, click_count: u8) {
        self.deliver_event_to_driver(InputEvent::Mouse(MouseInput {
            kind: MouseInputKind::Down,
            button: MouseButton::Left,
            column,
            row,
            modifiers: KeyModifiers::default(),
            click_count,
        }));
    }

    fn deliver_mouse(&mut self, kind: MouseInputKind, button: MouseButton, column: u16, row: u16) {
        self.deliver_event_to_driver(InputEvent::Mouse(MouseInput {
            kind,
            button,
            column,
            row,
            modifiers: KeyModifiers::default(),
            click_count: 0,
        }));
    }

    fn render_current_frame(&self) -> Buffer {
        let area = Rect::new(0, 0, self.width, self.height);
        let mut buffer = Buffer::empty(area);
        self.driver.render_frame(area, &mut buffer);
        buffer
    }

    fn render_into_buffer(&mut self) {
        self.buffer = self.render_current_frame();
    }
}

fn drag_path(from: (u16, u16), to: (u16, u16)) -> Vec<(u16, u16)> {
    let mut path = vec![from];
    let mut column = from.0;
    while column != to.0 {
        column = step_coordinate(column, to.0);
        path.push((column, from.1));
    }
    let mut row = from.1;
    while row != to.1 {
        row = step_coordinate(row, to.1);
        path.push((to.0, row));
    }
    path
}

fn step_coordinate(current: u16, target: u16) -> u16 {
    if current < target {
        current + 1
    } else {
        current - 1
    }
}

#[cfg(test)]
mod terminal_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use gitdeck_core::{
        InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInput, MouseInputKind,
        PaneId, Size,
    };
    use gitdeck_tui::ScreenDriver;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};

    use super::{drag_path, TestTerminal};

    #[derive(Default)]
    struct RecordingDriver {
        events: Vec<InputEvent>,
        focused: Option<PaneId>,
        painted_line: Option<String>,
    }

    impl ScreenDriver for RecordingDriver {
        fn handle_event(&mut self, event: InputEvent) {
            self.events.push(event);
        }

        fn render_frame(&self, area: Rect, buffer: &mut Buffer) {
            if let Some(line) = self.painted_line.as_deref() {
                buffer.set_string(
                    area.x,
                    area.y,
                    line,
                    Style::default().fg(Color::Rgb(7, 8, 9)),
                );
            }
        }

        fn focused_pane(&self) -> PaneId {
            self.focused.unwrap_or(PaneId::Changes)
        }

        fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
            (self.focused == Some(pane)).then(|| Rect::new(1, 1, 20, 10))
        }
    }

    fn new_terminal(driver: RecordingDriver) -> TestTerminal<RecordingDriver> {
        TestTerminal::new(driver, 20, 6, CapsProfile::Rich, ThemeId::JbDark)
    }

    fn mouse_event(
        kind: MouseInputKind,
        button: MouseButton,
        column: u16,
        row: u16,
        click_count: u8,
    ) -> InputEvent {
        InputEvent::Mouse(MouseInput {
            kind,
            button,
            column,
            row,
            modifiers: KeyModifiers::default(),
            click_count,
        })
    }

    #[test]
    fn click_delivers_down_then_up_at_the_same_cell() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_click(3, 2);
        assert_eq!(
            terminal.driver().events,
            vec![
                mouse_event(MouseInputKind::Down, MouseButton::Left, 3, 2, 1),
                mouse_event(MouseInputKind::Up, MouseButton::Left, 3, 2, 0)
            ]
        );
    }

    #[test]
    fn double_click_delivers_two_down_up_pairs() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_double_click(4, 1);
        assert_eq!(
            terminal.driver().events,
            vec![
                mouse_event(MouseInputKind::Down, MouseButton::Left, 4, 1, 1),
                mouse_event(MouseInputKind::Up, MouseButton::Left, 4, 1, 0),
                mouse_event(MouseInputKind::Down, MouseButton::Left, 4, 1, 2),
                mouse_event(MouseInputKind::Up, MouseButton::Left, 4, 1, 0)
            ]
        );
    }

    #[test]
    fn right_click_delivers_right_button_down_and_up() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_right_click(2, 5);
        assert_eq!(
            terminal.driver().events,
            vec![
                mouse_event(MouseInputKind::Down, MouseButton::Right, 2, 5, 0),
                mouse_event(MouseInputKind::Up, MouseButton::Right, 2, 5, 0)
            ]
        );
    }

    #[test]
    fn drag_steps_horizontally_then_vertically_between_endpoints() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_drag((1, 1), (4, 3));
        assert_eq!(
            terminal.driver().events,
            vec![
                mouse_event(MouseInputKind::Down, MouseButton::Left, 1, 1, 1),
                mouse_event(MouseInputKind::Drag, MouseButton::Left, 1, 1, 0),
                mouse_event(MouseInputKind::Drag, MouseButton::Left, 2, 1, 0),
                mouse_event(MouseInputKind::Drag, MouseButton::Left, 3, 1, 0),
                mouse_event(MouseInputKind::Drag, MouseButton::Left, 4, 1, 0),
                mouse_event(MouseInputKind::Drag, MouseButton::Left, 4, 2, 0),
                mouse_event(MouseInputKind::Drag, MouseButton::Left, 4, 3, 0),
                mouse_event(MouseInputKind::Up, MouseButton::Left, 4, 3, 0)
            ]
        );
    }

    #[test]
    fn drag_path_contains_both_endpoints_and_no_duplicate_steps() {
        assert_eq!(
            drag_path((2, 4), (2, 4)),
            vec![(2, 4)],
            "a zero-length drag visits the start point once"
        );
        assert_eq!(
            drag_path((3, 4), (1, 2)),
            vec![(3, 4), (2, 4), (1, 4), (1, 3), (1, 2)]
        );
    }

    #[test]
    fn wheel_up_repeats_scroll_up_events() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_wheel(6, 3, 2);
        assert_eq!(
            terminal.driver().events,
            vec![
                mouse_event(MouseInputKind::ScrollUp, MouseButton::None, 6, 3, 0),
                mouse_event(MouseInputKind::ScrollUp, MouseButton::None, 6, 3, 0)
            ]
        );
    }

    #[test]
    fn wheel_down_repeats_scroll_down_events() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_wheel(6, 3, -3);
        assert_eq!(
            terminal.driver().events,
            vec![
                mouse_event(MouseInputKind::ScrollDown, MouseButton::None, 6, 3, 0),
                mouse_event(MouseInputKind::ScrollDown, MouseButton::None, 6, 3, 0),
                mouse_event(MouseInputKind::ScrollDown, MouseButton::None, 6, 3, 0)
            ]
        );
    }

    #[test]
    fn zero_wheel_delta_delivers_no_event() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.mouse_wheel(1, 1, 0);
        assert!(terminal.driver().events.is_empty());
    }

    #[test]
    fn tick_delivers_tick_event() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.tick();
        assert_eq!(terminal.driver().events, vec![InputEvent::Tick]);
    }

    #[test]
    fn resize_delivers_resize_event_and_updates_screen_size() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.resize(8, 3);
        assert_eq!(
            terminal.driver().events,
            vec![InputEvent::Resize(Size {
                width: 8,
                height: 3
            })]
        );
        assert_eq!(terminal.screen_lines().len(), 3);
    }

    #[test]
    fn send_keys_delivers_one_event_per_parsed_token() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.send_keys("ctrl+k,alt+g");
        assert_eq!(
            terminal.driver().events,
            vec![
                InputEvent::Key(KeyInput {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers {
                        ctrl: true,
                        alt: false,
                        shift: false
                    }
                }),
                InputEvent::Key(KeyInput {
                    code: KeyCode::Char('g'),
                    modifiers: KeyModifiers {
                        ctrl: false,
                        alt: true,
                        shift: false
                    }
                })
            ]
        );
    }

    #[test]
    fn send_text_delivers_one_char_event_per_character() {
        let mut terminal = new_terminal(RecordingDriver::default());
        terminal.send_text("中a");
        assert_eq!(
            terminal.driver().events,
            vec![
                InputEvent::Key(KeyInput {
                    code: KeyCode::Char('中'),
                    modifiers: KeyModifiers::default()
                }),
                InputEvent::Key(KeyInput {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::default()
                })
            ]
        );
    }

    #[test]
    fn rendered_text_is_visible_through_screen_text_and_find_line() {
        let driver = RecordingDriver {
            painted_line: Some(String::from("hello")),
            ..RecordingDriver::default()
        };
        let terminal = new_terminal(driver);
        assert_eq!(terminal.screen_lines()[0], "hello");
        assert_eq!(terminal.find_line("ell"), Some(0));
        assert_eq!(terminal.find_line("absent"), None);
    }

    #[test]
    fn cell_reports_symbol_and_colors_of_the_rendered_buffer() {
        let driver = RecordingDriver {
            painted_line: Some(String::from("ok")),
            ..RecordingDriver::default()
        };
        let terminal = new_terminal(driver);
        let cell = terminal.cell(0, 0);
        assert_eq!(cell.symbol, "o");
        assert_eq!(cell.foreground, Color::Rgb(7, 8, 9));
        assert_eq!(cell.background, Color::Reset);
    }

    #[test]
    fn focused_pane_and_pane_rect_come_from_the_driver() {
        let driver = RecordingDriver {
            focused: Some(PaneId::Log),
            ..RecordingDriver::default()
        };
        let terminal = new_terminal(driver);
        assert_eq!(terminal.focused_pane(), PaneId::Log);
        assert_eq!(
            terminal.find_pane(PaneId::Log),
            Some(Rect::new(1, 1, 20, 10))
        );
        assert_eq!(
            terminal.pane_rect(PaneId::Log),
            Some(Rect::new(1, 1, 20, 10))
        );
        assert_eq!(terminal.pane_rect(PaneId::Changes), None);
    }

    #[test]
    fn caps_reports_the_profile_passed_to_new() {
        let terminal = TestTerminal::new(
            RecordingDriver::default(),
            4,
            2,
            CapsProfile::Basic,
            ThemeId::Mono,
        );
        assert_eq!(terminal.caps(), CapsProfile::Basic);
    }

    #[test]
    fn driver_accessors_expose_the_underlying_driver() {
        let mut terminal = new_terminal(RecordingDriver::default());
        assert!(terminal.driver().events.is_empty());
        terminal.driver_mut().painted_line = Some(String::from("mutated"));
        terminal.tick();
        assert_eq!(terminal.screen_lines()[0], "mutated");
    }
}
