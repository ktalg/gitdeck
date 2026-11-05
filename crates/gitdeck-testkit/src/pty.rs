use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gitdeck_caps::ThemeId;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use vt100::{Color, Parser};

use crate::raster::write_screenshot_into;
use crate::screenshot_report::ScreenshotReport;

const STABLE_POLL_INTERVAL: Duration = Duration::from_millis(20);
const INPUT_RESPONSE_TIMEOUT: Duration = Duration::from_millis(250);
const QUIET_POLLS_AFTER_INPUT: u32 = 2;
const QUIET_POLLS_WITHOUT_INPUT: u32 = 1;
const READ_CHUNK_BYTES: usize = 4096;
const INPUT_OUTPUT_ERROR_CODE: i32 = 5;
const BROKEN_PIPE_ERROR_CODE: i32 = 32;
const LEFT_BUTTON_CODE: u8 = 0;
const RIGHT_BUTTON_CODE: u8 = 2;
const DRAG_BUTTON_CODE: u8 = 32;
const WHEEL_UP_BUTTON_CODE: u8 = 64;
const WHEEL_DOWN_BUTTON_CODE: u8 = 65;

pub struct PtySessionOptions<'a> {
    pub program: &'a Path,
    pub working_directory: &'a Path,
    pub columns: u16,
    pub rows: u16,
    pub environment: &'a [(String, String)],
    pub arguments: &'a [&'a str],
    pub settle_timeout: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PtyScreenCell {
    pub symbol: String,
    pub foreground: Color,
    pub background: Color,
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    pending_bytes: Arc<Mutex<Vec<u8>>>,
    frame_version: Arc<AtomicU64>,
    frame_cache: ScreenFrameCache,
    awaiting_input_response: bool,
    reader_thread: Option<JoinHandle<()>>,
    parser: Parser,
    raw_bytes: Vec<u8>,
    settle_timeout: Duration,
    input_discarded_after_exit: bool,
}

struct ScreenFrameCache {
    settled_version: Option<u64>,
}

impl ScreenFrameCache {
    fn new() -> Self {
        Self {
            settled_version: None,
        }
    }

    fn is_current(&self, frame_version: u64) -> bool {
        self.settled_version == Some(frame_version)
    }

    fn note_settled(&mut self, frame_version: u64) {
        self.settled_version = Some(frame_version);
    }

    fn invalidate(&mut self) {
        self.settled_version = None;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettlePoll {
    KeepWaiting,
    Settled,
}

struct ScreenSettleTracker {
    awaiting_input_response: bool,
    input_response_seen: bool,
    input_version: u64,
    quiet_polls: u32,
    previous_screen: String,
    settled_version: u64,
}

impl ScreenSettleTracker {
    fn new(awaiting_input_response: bool, frame_version: u64, previous_screen: String) -> Self {
        Self {
            awaiting_input_response,
            input_response_seen: false,
            input_version: frame_version,
            quiet_polls: 0,
            previous_screen,
            settled_version: frame_version,
        }
    }

    fn after_poll(
        &mut self,
        received_bytes: bool,
        frame_version: u64,
        screen: String,
    ) -> SettlePoll {
        self.settled_version = frame_version;
        let screen_changed = screen != self.previous_screen;
        self.previous_screen = screen;
        if frame_version != self.input_version {
            self.input_response_seen = true;
        }
        if received_bytes || screen_changed {
            self.quiet_polls = 0;
            return SettlePoll::KeepWaiting;
        }
        self.quiet_polls += 1;
        if self.still_waiting_for_the_input_response()
            || self.quiet_polls < self.required_quiet_polls()
        {
            return SettlePoll::KeepWaiting;
        }
        SettlePoll::Settled
    }

    fn still_waiting_for_the_input_response(&self) -> bool {
        self.awaiting_input_response && !self.input_response_seen
    }

    fn stop_awaiting_input_response(&mut self) {
        self.awaiting_input_response = false;
    }

    fn required_quiet_polls(&self) -> u32 {
        if self.awaiting_input_response {
            QUIET_POLLS_AFTER_INPUT
        } else {
            QUIET_POLLS_WITHOUT_INPUT
        }
    }

    fn settled_version(&self) -> u64 {
        self.settled_version
    }
}

impl PtySession {
    pub fn spawn(options: &PtySessionOptions<'_>) -> Self {
        let pair = native_pty_system()
            .openpty(pty_size(options.columns, options.rows))
            .expect("a pseudo terminal is allocated");
        let mut command = CommandBuilder::new(options.program);
        command.cwd(options.working_directory);
        for (name, value) in options.environment {
            command.env(name, value);
        }
        command.args(options.arguments.iter().copied());
        let child = pair
            .slave
            .spawn_command(command)
            .expect("the program starts inside the pseudo terminal");
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("the pseudo terminal master is readable");
        let writer = pair
            .master
            .take_writer()
            .expect("the pseudo terminal master is writable");
        let pending_bytes = Arc::new(Mutex::new(Vec::new()));
        let frame_version = Arc::new(AtomicU64::new(0));
        let reader_thread = {
            let pending_bytes = Arc::clone(&pending_bytes);
            let frame_version = Arc::clone(&frame_version);
            std::thread::spawn(move || {
                let mut chunk = [0u8; READ_CHUNK_BYTES];
                loop {
                    match reader.read(&mut chunk) {
                        Ok(0) => return,
                        Ok(count) => {
                            pending_bytes
                                .lock()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .extend_from_slice(&chunk[..count]);
                            frame_version.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(_) => return,
                    }
                }
            })
        };
        Self {
            master: pair.master,
            writer,
            child,
            pending_bytes,
            frame_version,
            frame_cache: ScreenFrameCache::new(),
            awaiting_input_response: false,
            reader_thread: Some(reader_thread),
            parser: Parser::new(options.rows, options.columns, 0),
            raw_bytes: Vec::new(),
            settle_timeout: options.settle_timeout,
            input_discarded_after_exit: false,
        }
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        self.frame_cache.invalidate();
        self.awaiting_input_response = true;
        let written = self
            .writer
            .write_all(bytes)
            .and_then(|()| self.writer.flush());
        match written {
            Ok(()) => {}
            Err(error) if terminal_closed_the_input(&error) => {
                self.input_discarded_after_exit = true;
            }
            Err(error) => panic!("the pseudo terminal accepts input: {error}"),
        }
    }

    pub fn input_was_discarded_after_exit(&self) -> bool {
        self.input_discarded_after_exit
    }

    pub fn try_exit_code(&mut self) -> Option<i32> {
        self.pump_pending_bytes();
        match self.child.try_wait() {
            Ok(Some(status)) => Some(i32::try_from(status.exit_code()).unwrap_or(-1)),
            Ok(None) | Err(_) => None,
        }
    }

    pub fn send_keys(&mut self, keys: &str) {
        let mut bytes = Vec::new();
        for token in keys
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|token| !token.is_empty())
        {
            bytes.extend_from_slice(&key_token_bytes(token));
        }
        self.send_bytes(&bytes);
    }

    pub fn mouse_click(&mut self, column: u16, row: u16) {
        self.send_bytes(&sgr_mouse_report(LEFT_BUTTON_CODE, column, row, 'M'));
        self.send_bytes(&sgr_mouse_report(LEFT_BUTTON_CODE, column, row, 'm'));
    }

    pub fn mouse_right_click(&mut self, column: u16, row: u16) {
        self.send_bytes(&sgr_mouse_report(RIGHT_BUTTON_CODE, column, row, 'M'));
        self.send_bytes(&sgr_mouse_report(RIGHT_BUTTON_CODE, column, row, 'm'));
    }

    pub fn mouse_double_click(&mut self, column: u16, row: u16) {
        self.mouse_click(column, row);
        self.mouse_click(column, row);
    }

    pub fn mouse_drag(&mut self, from: (u16, u16), to: (u16, u16)) {
        self.send_bytes(&sgr_mouse_report(LEFT_BUTTON_CODE, from.0, from.1, 'M'));
        for column in drag_columns(from.0, to.0) {
            self.send_bytes(&sgr_mouse_report(DRAG_BUTTON_CODE, column, to.1, 'M'));
        }
        self.send_bytes(&sgr_mouse_report(LEFT_BUTTON_CODE, to.0, to.1, 'm'));
    }

    pub fn mouse_wheel(&mut self, column: u16, row: u16, delta: i32) {
        let button_code = if delta >= 0 {
            WHEEL_UP_BUTTON_CODE
        } else {
            WHEEL_DOWN_BUTTON_CODE
        };
        for _ in 0..delta.unsigned_abs() {
            self.send_bytes(&sgr_mouse_report(button_code, column, row, 'M'));
        }
    }

    pub fn resize(&mut self, columns: u16, rows: u16) {
        self.master
            .resize(pty_size(columns, rows))
            .expect("the pseudo terminal is resized");
        self.parser.set_size(rows, columns);
        self.frame_cache.invalidate();
        self.awaiting_input_response = true;
        self.settle();
    }

    pub fn settle(&mut self) {
        let awaiting_input_response = std::mem::take(&mut self.awaiting_input_response);
        let mut tracker = ScreenSettleTracker::new(
            awaiting_input_response,
            self.received_frame_version(),
            self.parser.screen().contents(),
        );
        let deadline = Instant::now() + self.settle_timeout;
        let input_response_deadline = Instant::now() + INPUT_RESPONSE_TIMEOUT;
        while Instant::now() < deadline {
            std::thread::sleep(STABLE_POLL_INTERVAL);
            let version_before_pump = self.received_frame_version();
            let received_bytes = self.pump_pending_bytes();
            let screen = self.parser.screen().contents();
            if tracker.after_poll(received_bytes, version_before_pump, screen)
                == SettlePoll::Settled
            {
                break;
            }
            if Instant::now() >= input_response_deadline {
                tracker.stop_awaiting_input_response();
            }
        }
        self.frame_cache.note_settled(tracker.settled_version());
    }

    fn received_frame_version(&self) -> u64 {
        self.frame_version.load(Ordering::SeqCst)
    }

    fn settle_unless_frame_is_current(&mut self) {
        if !self.awaiting_input_response
            && self.frame_cache.is_current(self.received_frame_version())
        {
            return;
        }
        self.settle();
    }

    pub fn screen_text(&mut self) -> String {
        self.settle_unless_frame_is_current();
        self.parser.screen().contents()
    }

    pub fn row_text(&mut self, row: u16, from_column: u16, to_column: u16) -> String {
        self.settle_unless_frame_is_current();
        let mut text = String::new();
        for column in from_column..to_column {
            if let Some(cell) = self.parser.screen().cell(row, column) {
                text.push_str(&cell.contents());
            }
        }
        text.trim_end().to_string()
    }

    pub fn find_text_cells(&mut self, needle: &str) -> Vec<(u16, u16)> {
        self.settle_unless_frame_is_current();
        let screen = self.parser.screen();
        let (rows, columns) = screen.size();
        let mut matches = Vec::new();
        if needle.is_empty() {
            return matches;
        }
        for row in 0..rows {
            let mut line = String::new();
            let mut cell_starts: Vec<(usize, u16)> = Vec::new();
            for column in 0..columns {
                if let Some(cell) = screen.cell(row, column) {
                    cell_starts.push((line.len(), column));
                    line.push_str(&cell.contents());
                }
            }
            let mut search_from = 0usize;
            while let Some(relative) = line[search_from..].find(needle) {
                let byte_index = search_from + relative;
                let column = cell_starts
                    .iter()
                    .rev()
                    .find(|(byte_start, _)| *byte_start <= byte_index)
                    .map(|(_, column)| *column)
                    .unwrap_or(0);
                matches.push((column, row));
                search_from = byte_index + needle.len();
            }
        }
        matches
    }

    pub fn cell(&mut self, column: u16, row: u16) -> PtyScreenCell {
        self.settle_unless_frame_is_current();
        let cell = self
            .parser
            .screen()
            .cell(row, column)
            .unwrap_or_else(|| panic!("no cell at column {column}, row {row}"));
        PtyScreenCell {
            symbol: cell.contents(),
            foreground: cell.fgcolor(),
            background: cell.bgcolor(),
        }
    }

    pub fn screenshot_into(
        &mut self,
        directory: &Path,
        name: &str,
        theme: ThemeId,
    ) -> ScreenshotReport {
        self.settle_unless_frame_is_current();
        let buffer = buffer_from_screen(self.parser.screen());
        write_screenshot_into(directory, name, &buffer, theme)
    }

    pub fn raw_log(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub fn wait_for_text(&mut self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.parser.screen().contents().contains(needle) {
                return true;
            }
            self.pump_until_stable();
            if self.parser.screen().contents().contains(needle) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
        }
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<i32> {
        let deadline = Instant::now() + timeout;
        loop {
            self.pump_pending_bytes();
            if let Ok(Some(status)) = self.child.try_wait() {
                self.settle();
                return Some(i32::try_from(status.exit_code()).unwrap_or(-1));
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return None;
            }
            std::thread::sleep(STABLE_POLL_INTERVAL);
        }
    }

    fn pump_pending_bytes(&mut self) -> bool {
        let pending = {
            let mut pending_bytes = self
                .pending_bytes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            std::mem::take(&mut *pending_bytes)
        };
        if pending.is_empty() {
            return false;
        }
        self.raw_bytes.extend_from_slice(&pending);
        self.parser.process(&pending);
        true
    }

    fn pump_until_stable(&mut self) {
        let mut previous_screen = self.parser.screen().contents();
        let deadline = Instant::now() + self.settle_timeout;
        while Instant::now() < deadline {
            std::thread::sleep(STABLE_POLL_INTERVAL);
            let received_bytes = self.pump_pending_bytes();
            let current_screen = self.parser.screen().contents();
            if !received_bytes && current_screen == previous_screen {
                return;
            }
            previous_screen = current_screen;
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
    }
}

pub fn key_token_bytes(token: &str) -> Vec<u8> {
    let lowered = token.to_ascii_lowercase();
    match lowered.as_str() {
        "enter" | "return" => return vec![b'\r'],
        "esc" | "escape" => return vec![0x1b],
        "tab" => return vec![b'\t'],
        "shift+tab" | "backtab" => return b"\x1b[Z".to_vec(),
        "space" => return vec![b' '],
        "up" => return b"\x1b[A".to_vec(),
        "down" => return b"\x1b[B".to_vec(),
        "right" => return b"\x1b[C".to_vec(),
        "left" => return b"\x1b[D".to_vec(),
        "f5" => return b"\x1b[15~".to_vec(),
        "ctrl+enter" | "ctrl+return" => return b"\x1b[13;5u".to_vec(),
        _ => {}
    }
    if let Some(letter) = lowered.strip_prefix("ctrl+") {
        return vec![control_code(letter)];
    }
    if let Some(letter) = lowered.strip_prefix("alt+") {
        return b"\x1b".iter().copied().chain(letter.bytes()).collect();
    }
    if let Some(letter) = lowered.strip_prefix("shift+") {
        return letter.bytes().collect();
    }
    assert_eq!(token.chars().count(), 1, "unrecognized key token {token:?}");
    token.bytes().collect()
}

fn control_code(letter: &str) -> u8 {
    let mut characters = letter.chars();
    let character = characters.next().expect("a control key names a character");
    assert!(
        characters.next().is_none() && character.is_ascii_lowercase(),
        "control keys are single ascii letters, got {letter:?}"
    );
    (character as u8) & 0x1f
}

fn buffer_from_screen(screen: &vt100::Screen) -> ratatui::buffer::Buffer {
    let (rows, columns) = screen.size();
    let mut buffer =
        ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, columns, rows));
    for row in 0..rows {
        for column in 0..columns {
            let (Some(source), Some(target)) =
                (screen.cell(row, column), buffer.cell_mut((column, row)))
            else {
                continue;
            };
            target.set_symbol(&source.contents());
            target.set_fg(screen_color(source.fgcolor()));
            target.set_bg(screen_color(source.bgcolor()));
        }
    }
    buffer
}

fn screen_color(color: Color) -> ratatui::style::Color {
    use ratatui::style::Color as TerminalColor;
    match color {
        Color::Default => TerminalColor::Reset,
        Color::Idx(index) => TerminalColor::Indexed(index),
        Color::Rgb(red, green, blue) => TerminalColor::Rgb(red, green, blue),
    }
}

fn terminal_closed_the_input(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::NotFound
    ) || matches!(
        error.raw_os_error(),
        Some(INPUT_OUTPUT_ERROR_CODE) | Some(BROKEN_PIPE_ERROR_CODE)
    )
}

fn pty_size(columns: u16, rows: u16) -> PtySize {
    PtySize {
        rows,
        cols: columns,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn sgr_mouse_report(button_code: u8, column: u16, row: u16, final_byte: char) -> Vec<u8> {
    format!("\x1b[<{button_code};{};{}{final_byte}", column + 1, row + 1).into_bytes()
}

fn drag_columns(from: u16, to: u16) -> Vec<u16> {
    if to >= from {
        (from + 1..=to).collect()
    } else {
        (to..from).rev().collect()
    }
}

#[cfg(test)]
mod screen_frame_cache {
    use super::ScreenFrameCache;

    struct FakeScreenReader {
        cache: ScreenFrameCache,
        frame_version: u64,
        awaiting_input_response: bool,
        settle_calls: u32,
    }

    fn fake_screen_reader() -> FakeScreenReader {
        FakeScreenReader {
            cache: ScreenFrameCache::new(),
            frame_version: 0,
            awaiting_input_response: false,
            settle_calls: 0,
        }
    }

    impl FakeScreenReader {
        fn read_cell(&mut self) {
            let cache_is_current =
                !self.awaiting_input_response && self.cache.is_current(self.frame_version);
            if cache_is_current {
                return;
            }
            self.awaiting_input_response = false;
            self.settle_calls += 1;
            self.cache.note_settled(self.frame_version);
        }

        fn receive_bytes(&mut self) {
            self.frame_version += 1;
        }

        fn send_input(&mut self) {
            self.cache.invalidate();
            self.awaiting_input_response = true;
        }
    }

    #[test]
    fn a_hundred_cell_reads_of_one_frame_settle_once() {
        let mut reader = fake_screen_reader();
        for _ in 0..100 {
            reader.read_cell();
        }
        assert_eq!(reader.settle_calls, 1);
    }

    #[test]
    fn new_bytes_make_the_cached_frame_stale() {
        let mut reader = fake_screen_reader();
        reader.read_cell();
        reader.receive_bytes();
        reader.read_cell();
        assert_eq!(reader.settle_calls, 2);
    }

    #[test]
    fn the_first_cell_read_after_sent_input_settles_again() {
        let mut reader = fake_screen_reader();
        reader.read_cell();
        reader.send_input();
        reader.read_cell();
        assert_eq!(reader.settle_calls, 2);
        reader.read_cell();
        assert_eq!(reader.settle_calls, 2);
    }

    #[test]
    fn a_stale_settled_version_does_not_hide_the_pending_input() {
        let mut reader = fake_screen_reader();
        reader.read_cell();
        reader.cache.note_settled(reader.frame_version);
        reader.send_input();
        reader.cache.note_settled(reader.frame_version);
        reader.read_cell();
        assert_eq!(reader.settle_calls, 2);
    }
}

#[cfg(test)]
mod screen_settle_tracker {
    use super::{ScreenSettleTracker, SettlePoll};

    fn tracker_without_input() -> ScreenSettleTracker {
        ScreenSettleTracker::new(false, 7, "commit 1".to_string())
    }

    fn tracker_after_input() -> ScreenSettleTracker {
        ScreenSettleTracker::new(true, 7, "commit 1".to_string())
    }

    #[test]
    fn a_quiet_poll_without_input_settles_the_frame() {
        let mut tracker = tracker_without_input();
        assert_eq!(
            tracker.after_poll(false, 7, "commit 1".to_string()),
            SettlePoll::Settled
        );
        assert_eq!(tracker.settled_version(), 7);
    }

    #[test]
    fn a_changed_screen_keeps_waiting_for_the_next_poll() {
        let mut tracker = tracker_without_input();
        assert_eq!(
            tracker.after_poll(false, 7, "commit 2".to_string()),
            SettlePoll::KeepWaiting
        );
        assert_eq!(
            tracker.after_poll(false, 7, "commit 2".to_string()),
            SettlePoll::Settled
        );
    }

    #[test]
    fn a_pending_input_keeps_waiting_while_no_frame_arrives() {
        let mut tracker = tracker_after_input();
        for _ in 0..20 {
            assert_eq!(
                tracker.after_poll(false, 7, "commit 1".to_string()),
                SettlePoll::KeepWaiting
            );
        }
        assert_eq!(tracker.settled_version(), 7);
    }

    #[test]
    fn the_input_response_settles_only_after_two_quiet_polls() {
        let mut tracker = tracker_after_input();
        assert_eq!(
            tracker.after_poll(true, 8, "commit 2".to_string()),
            SettlePoll::KeepWaiting
        );
        assert_eq!(
            tracker.after_poll(false, 8, "commit 2".to_string()),
            SettlePoll::KeepWaiting
        );
        assert_eq!(
            tracker.after_poll(false, 8, "commit 2".to_string()),
            SettlePoll::Settled
        );
        assert_eq!(tracker.settled_version(), 8);
    }

    #[test]
    fn a_silent_input_settles_once_the_response_window_closed() {
        let mut tracker = tracker_after_input();
        assert_eq!(
            tracker.after_poll(false, 7, "commit 1".to_string()),
            SettlePoll::KeepWaiting
        );
        tracker.stop_awaiting_input_response();
        assert_eq!(
            tracker.after_poll(false, 7, "commit 1".to_string()),
            SettlePoll::Settled
        );
    }
}

#[cfg(test)]
mod closed_input_detection {
    use super::terminal_closed_the_input;

    #[test]
    fn the_input_output_error_from_a_closed_pty_counts_as_closed_input() {
        assert!(terminal_closed_the_input(
            &std::io::Error::from_raw_os_error(5)
        ));
    }

    #[test]
    fn the_broken_pipe_error_counts_as_closed_input() {
        assert!(terminal_closed_the_input(
            &std::io::Error::from_raw_os_error(32)
        ));
    }

    #[test]
    fn an_unrelated_write_error_is_not_a_closed_input() {
        assert!(!terminal_closed_the_input(
            &std::io::Error::from_raw_os_error(13)
        ));
    }
}
