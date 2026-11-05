use std::collections::BTreeMap;
use std::fmt;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gitdeck_caps::{detect, CapsProfile, CapsSource, ThemeId};
use gitdeck_core::{
    InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInput, MouseInputKind, Size,
};
use ratatui::crossterm::event::{
    Event, KeyCode as TerminalKeyCode, KeyEvent, KeyEventKind, KeyModifiers as TerminalModifiers,
    MouseButton as TerminalMouseButton, MouseEvent, MouseEventKind,
};

pub const DEFAULT_SCREEN_WIDTH: u16 = 80;
pub const DEFAULT_SCREEN_HEIGHT: u16 = 24;
pub const CAPS_ENVIRONMENT_VARIABLE: &str = "GITDECK_CAPS";
pub const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);
pub const DOUBLE_CLICK_POSITION_TOLERANCE: u16 = 1;
pub const HELP_OPTIONS_SECTION: &str = "usage: gitdeck [options] [repository-path]

options
  --caps <profile>               capability profile to render with: rich,
                                 standard or basic
  --theme <name>                 color theme: jb-dark, jb-light, mono or nord
  --doctor                       print the caps, the theme and the terminal size
  --mouse-debug <path>           append every received mouse event to <path>
  --help                         print this help

keys
";

pub const HELP_MOUSE_SECTION: &str = "
mouse
  click                          select the clicked row
  double click                   open the diff of the clicked file
  drag on a divider              resize the pane
  wheel over a pane              move the hovered selection or scroll the diff
                                 by three
";

pub const HELP_KEY_COLUMN: usize = 33;
pub const HELP_MAX_COLUMNS: usize = 80;

pub fn help_text() -> String {
    let mut text = String::from(HELP_OPTIONS_SECTION);
    for binding in gitdeck_app::KEY_BINDINGS {
        text.push_str(&format_help_binding(binding));
    }
    text.push_str(HELP_MOUSE_SECTION);
    text
}

fn format_help_binding(binding: &gitdeck_app::KeyBinding) -> String {
    let mut line = format!("  {:<width$}", binding.keys, width = HELP_KEY_COLUMN - 2);
    let mut column = HELP_KEY_COLUMN;
    let mut first_word = true;
    for word in binding.action.split_whitespace() {
        let separator_width = usize::from(!first_word);
        let word_width = word.chars().count();
        if !first_word && column + separator_width + word_width > HELP_MAX_COLUMNS {
            line.push('\n');
            line.push_str(&" ".repeat(HELP_KEY_COLUMN));
            column = HELP_KEY_COLUMN;
            first_word = true;
        }
        if first_word {
            line.push_str(word);
            column += word_width;
            first_word = false;
        } else {
            line.push(' ');
            line.push_str(word);
            column += separator_width + word_width;
        }
    }
    line.push('\n');
    line
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliOptions {
    pub caps: CapsProfile,
    pub caps_source: CapsSource,
    pub theme: ThemeId,
    pub doctor: bool,
    pub help: bool,
    pub mouse_debug_path: Option<PathBuf>,
    pub repository_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliError {
    UnknownArgument(String),
    MissingValue(&'static str),
    InvalidCaps(String),
    InvalidEnvCaps(String),
    InvalidTheme(String),
    MultipleRepositoryPaths,
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument {argument}"),
            Self::MissingValue(argument) => write!(formatter, "{argument} requires a value"),
            Self::InvalidCaps(value) => write!(formatter, "unknown capability profile {value}"),
            Self::InvalidEnvCaps(value) => write!(
                formatter,
                "unknown capability profile {value} in {CAPS_ENVIRONMENT_VARIABLE}, expected one of {}",
                caps_profile_names()
            ),
            Self::InvalidTheme(value) => write!(formatter, "unknown theme {value}"),
            Self::MultipleRepositoryPaths => {
                write!(formatter, "only one repository path is accepted")
            }
        }
    }
}

impl std::error::Error for CliError {}

fn caps_profile_names() -> String {
    CapsProfile::EVERY
        .iter()
        .map(|caps_profile| caps_profile.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn process_environment() -> BTreeMap<String, String> {
    std::env::vars().collect()
}

pub fn parse_cli_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> Result<CliOptions, CliError> {
    let options = parse_cli_arguments_with_environment(arguments, &process_environment())?;
    if options.doctor {
        return Ok(options);
    }
    Ok(apply_stdout_terminal_fallback(
        options,
        stdout_is_terminal(),
    ))
}

pub fn render_caps_for_stdout(options: &CliOptions) -> CapsProfile {
    apply_stdout_terminal_fallback(options.clone(), stdout_is_terminal()).caps
}

pub fn apply_stdout_terminal_fallback(
    mut options: CliOptions,
    stdout_is_terminal: bool,
) -> CliOptions {
    if !stdout_is_terminal && options.caps_source == CapsSource::Detected {
        options.caps = CapsProfile::Basic;
    }
    options
}

fn stdout_is_terminal() -> bool {
    std::io::stdout().is_terminal()
}

pub fn parse_cli_arguments_with_environment(
    arguments: impl IntoIterator<Item = String>,
    environment: &BTreeMap<String, String>,
) -> Result<CliOptions, CliError> {
    let mut options = CliOptions {
        caps: detect(environment),
        caps_source: CapsSource::Detected,
        theme: ThemeId::JbDark,
        doctor: false,
        help: false,
        mouse_debug_path: None,
        repository_path: None,
    };
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--" => continue,
            "--caps" => {
                let value = arguments.next().ok_or(CliError::MissingValue("--caps"))?;
                options.caps =
                    CapsProfile::parse_cli(&value).ok_or(CliError::InvalidCaps(value))?;
                options.caps_source = CapsSource::Cli;
            }
            "--theme" => {
                let value = arguments.next().ok_or(CliError::MissingValue("--theme"))?;
                options.theme = ThemeId::parse_cli(&value).ok_or(CliError::InvalidTheme(value))?;
            }
            "--mouse-debug" => {
                let value = arguments
                    .next()
                    .ok_or(CliError::MissingValue("--mouse-debug"))?;
                options.mouse_debug_path = Some(PathBuf::from(value));
            }
            "--doctor" => options.doctor = true,
            "--help" | "-h" => options.help = true,
            other if other.starts_with("--") => {
                return Err(CliError::UnknownArgument(other.to_string()))
            }
            other => options.repository_path = Some(single_repository_path(&options, other)?),
        }
    }
    if options.caps_source != CapsSource::Cli {
        options.apply_environment_caps(environment)?;
    }
    Ok(options)
}

impl CliOptions {
    fn apply_environment_caps(
        &mut self,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), CliError> {
        let caps_value = environment
            .get(CAPS_ENVIRONMENT_VARIABLE)
            .filter(|value| !value.is_empty());
        let Some(caps_value) = caps_value else {
            return Ok(());
        };
        self.caps = CapsProfile::parse_cli(caps_value)
            .ok_or_else(|| CliError::InvalidEnvCaps(caps_value.to_string()))?;
        self.caps_source = CapsSource::Environment;
        Ok(())
    }
}

fn single_repository_path(options: &CliOptions, argument: &str) -> Result<PathBuf, CliError> {
    if options.repository_path.is_some() {
        return Err(CliError::MultipleRepositoryPaths);
    }
    Ok(PathBuf::from(argument))
}

pub fn doctor_report(options: CliOptions, screen: Size, render_caps: CapsProfile) -> String {
    let mut report = format!(
        "caps: {}\ncaps source: {}\ntheme: {}\nsize: {}x{}",
        options.caps.as_str(),
        options.caps_source.as_str(),
        options.theme.as_str(),
        screen.width,
        screen.height
    );
    if render_caps != options.caps {
        report.push_str(&format!(
            "\nrender caps: {} (stdout is not a terminal)",
            render_caps.as_str()
        ));
    }
    report
}

pub fn is_quit_key(key_input: &KeyInput) -> bool {
    gitdeck_app::is_quit_key(key_input)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MousePress {
    at: Instant,
    column: u16,
    row: u16,
}

#[derive(Debug)]
pub struct MouseDebugLog {
    file: Option<std::fs::File>,
}

impl MouseDebugLog {
    pub fn open(path: Option<&std::path::Path>) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Self { file: None });
        };
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| {
                format!(
                    "cannot open the mouse debug file {}: {error}",
                    path.display()
                )
            })?;
        Ok(Self { file: Some(file) })
    }

    pub fn record(&mut self, input_event: Option<&InputEvent>) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let Some(InputEvent::Mouse(mouse_input)) = input_event else {
            return;
        };
        let line = format!(
            "kind={:?} button={:?} column={} row={} click_count={}\n",
            mouse_input.kind,
            mouse_input.button,
            mouse_input.column,
            mouse_input.row,
            mouse_input.click_count
        );
        let _ = std::io::Write::write_all(file, line.as_bytes());
    }
}

pub struct InputEventMapper {
    last_press: Option<MousePress>,
}

impl Default for InputEventMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl InputEventMapper {
    pub fn new() -> Self {
        Self { last_press: None }
    }

    pub fn map_event(&mut self, event: Event) -> Option<InputEvent> {
        self.map_event_at(event, Instant::now())
    }

    pub fn map_event_at(&mut self, event: Event, now: Instant) -> Option<InputEvent> {
        match event {
            Event::Key(key) => key_input_from_terminal_key(key).map(InputEvent::Key),
            Event::Mouse(mouse) => self
                .mouse_input_from_terminal_mouse(mouse, now)
                .map(InputEvent::Mouse),
            Event::Resize(width, height) => Some(InputEvent::Resize(Size { width, height })),
            Event::Paste(text) => Some(InputEvent::Paste(text)),
            Event::FocusGained | Event::FocusLost => None,
        }
    }

    fn mouse_input_from_terminal_mouse(
        &mut self,
        mouse: MouseEvent,
        now: Instant,
    ) -> Option<MouseInput> {
        let (kind, button) = match mouse.kind {
            MouseEventKind::Down(button) => (MouseInputKind::Down, mouse_button(button)),
            MouseEventKind::Up(button) => (MouseInputKind::Up, mouse_button(button)),
            MouseEventKind::Drag(button) => (MouseInputKind::Drag, mouse_button(button)),
            MouseEventKind::Moved => (MouseInputKind::Move, MouseButton::None),
            MouseEventKind::ScrollUp => (MouseInputKind::ScrollUp, MouseButton::None),
            MouseEventKind::ScrollDown => (MouseInputKind::ScrollDown, MouseButton::None),
            MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => return None,
        };
        let click_count = match (kind, button) {
            (MouseInputKind::Down, MouseButton::Left) => {
                self.click_count(mouse.column, mouse.row, now)
            }
            _ => 0,
        };
        Some(MouseInput {
            kind,
            button,
            column: mouse.column,
            row: mouse.row,
            modifiers: modifiers_from_terminal_modifiers(mouse.modifiers),
            click_count,
        })
    }

    fn click_count(&mut self, column: u16, row: u16, now: Instant) -> u8 {
        let repeats_last_press = self.last_press.is_some_and(|press| {
            now.duration_since(press.at) <= DOUBLE_CLICK_INTERVAL
                && press.column.abs_diff(column) <= DOUBLE_CLICK_POSITION_TOLERANCE
                && press.row.abs_diff(row) <= DOUBLE_CLICK_POSITION_TOLERANCE
        });
        self.last_press = if repeats_last_press {
            None
        } else {
            Some(MousePress {
                at: now,
                column,
                row,
            })
        };
        if repeats_last_press {
            2
        } else {
            1
        }
    }
}

fn key_input_from_terminal_key(key: KeyEvent) -> Option<KeyInput> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    Some(KeyInput {
        code: key_code_from_terminal_key_code(key.code)?,
        modifiers: modifiers_from_terminal_modifiers(key.modifiers),
    })
}

fn key_code_from_terminal_key_code(code: TerminalKeyCode) -> Option<KeyCode> {
    let key_code = match code {
        TerminalKeyCode::Char(' ') => KeyCode::Space,
        TerminalKeyCode::Char(character) => KeyCode::Char(character),
        TerminalKeyCode::Enter => KeyCode::Enter,
        TerminalKeyCode::Esc => KeyCode::Esc,
        TerminalKeyCode::Tab => KeyCode::Tab,
        TerminalKeyCode::BackTab => KeyCode::BackTab,
        TerminalKeyCode::Backspace => KeyCode::Backspace,
        TerminalKeyCode::Delete => KeyCode::Delete,
        TerminalKeyCode::Insert => KeyCode::Insert,
        TerminalKeyCode::Home => KeyCode::Home,
        TerminalKeyCode::End => KeyCode::End,
        TerminalKeyCode::PageUp => KeyCode::PageUp,
        TerminalKeyCode::PageDown => KeyCode::PageDown,
        TerminalKeyCode::Up => KeyCode::Up,
        TerminalKeyCode::Down => KeyCode::Down,
        TerminalKeyCode::Left => KeyCode::Left,
        TerminalKeyCode::Right => KeyCode::Right,
        TerminalKeyCode::F(number) => KeyCode::F(number),
        _ => return None,
    };
    Some(key_code)
}

fn mouse_button(button: TerminalMouseButton) -> MouseButton {
    match button {
        TerminalMouseButton::Left => MouseButton::Left,
        TerminalMouseButton::Right => MouseButton::Right,
        TerminalMouseButton::Middle => MouseButton::Middle,
    }
}

fn modifiers_from_terminal_modifiers(modifiers: TerminalModifiers) -> KeyModifiers {
    KeyModifiers {
        ctrl: modifiers.contains(TerminalModifiers::CONTROL),
        alt: modifiers.contains(TerminalModifiers::ALT),
        shift: modifiers.contains(TerminalModifiers::SHIFT),
    }
}

#[cfg(test)]
mod cli_tests {
    use gitdeck_caps::{CapsProfile, CapsSource, ThemeId};
    use gitdeck_core::{
        InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInputKind, Size,
    };
    use ratatui::crossterm::event::{
        Event, KeyCode as TerminalKeyCode, KeyEvent, KeyEventKind, KeyEventState,
        KeyModifiers as TerminalModifiers, MouseButton as TerminalMouseButton, MouseEvent,
        MouseEventKind,
    };

    use std::collections::BTreeMap;
    use std::io::IsTerminal;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use super::{
        apply_stdout_terminal_fallback, detect, doctor_report, help_text, is_quit_key,
        parse_cli_arguments, parse_cli_arguments_with_environment, process_environment,
        render_caps_for_stdout, CliError, CliOptions, InputEventMapper, MouseDebugLog,
        CAPS_ENVIRONMENT_VARIABLE, DOUBLE_CLICK_INTERVAL,
    };

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn environment(variables: &[(&str, &str)]) -> BTreeMap<String, String> {
        variables
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn defaults_detect_the_caps_and_keep_jb_dark_without_doctor() {
        assert_eq!(
            parse_cli_arguments_with_environment(arguments(&[]), &environment(&[])),
            Ok(CliOptions {
                caps: CapsProfile::Standard,
                caps_source: CapsSource::Detected,
                theme: ThemeId::JbDark,
                doctor: false,
                help: false,
                mouse_debug_path: None,
                repository_path: None,
            })
        );
    }

    #[test]
    fn cli_caps_override_wins_over_env_and_detection() {
        let terminal_environment = environment(&[
            ("COLORTERM", "truecolor"),
            (CAPS_ENVIRONMENT_VARIABLE, "basic"),
        ]);
        let options = parse_cli_arguments_with_environment(
            arguments(&["--caps", "standard"]),
            &terminal_environment,
        )
        .expect("the argument list parses");
        assert_eq!(options.caps, CapsProfile::Standard);
        assert_eq!(options.caps_source, CapsSource::Cli);
    }

    #[test]
    fn a_piped_stdout_defaults_to_basic() {
        let detected = parse_cli_arguments_with_environment(
            arguments(&[]),
            &environment(&[("COLORTERM", "truecolor")]),
        )
        .expect("the argument list parses");
        assert_eq!(detected.caps, CapsProfile::Rich);
        let piped = apply_stdout_terminal_fallback(detected.clone(), false);
        assert_eq!(piped.caps, CapsProfile::Basic, "a pipe has no colors");
        assert_eq!(
            piped.caps_source,
            CapsSource::Detected,
            "the fallback keeps the detected source"
        );
        assert_eq!(
            apply_stdout_terminal_fallback(detected, true).caps,
            CapsProfile::Rich,
            "a terminal keeps the detected profile"
        );
        let explicit =
            parse_cli_arguments_with_environment(arguments(&["--caps", "rich"]), &environment(&[]))
                .expect("the argument list parses");
        assert_eq!(
            apply_stdout_terminal_fallback(explicit, false).caps,
            CapsProfile::Rich,
            "the explicit flag wins over the pipe fallback"
        );
        let from_environment = parse_cli_arguments_with_environment(
            arguments(&[]),
            &environment(&[(CAPS_ENVIRONMENT_VARIABLE, "standard")]),
        )
        .expect("the argument list parses");
        assert_eq!(
            apply_stdout_terminal_fallback(from_environment, false).caps,
            CapsProfile::Standard,
            "the environment override wins over the pipe fallback"
        );
    }

    #[test]
    fn env_caps_is_used_when_the_flag_is_absent() {
        let options = parse_cli_arguments_with_environment(
            arguments(&[]),
            &environment(&[
                ("COLORTERM", "truecolor"),
                (CAPS_ENVIRONMENT_VARIABLE, "basic"),
            ]),
        )
        .expect("the argument list parses");
        assert_eq!(options.caps, CapsProfile::Basic);
        assert_eq!(options.caps_source, CapsSource::Environment);
        let detected = parse_cli_arguments_with_environment(
            arguments(&[]),
            &environment(&[("COLORTERM", "truecolor"), (CAPS_ENVIRONMENT_VARIABLE, "")]),
        )
        .expect("the argument list parses");
        assert_eq!(detected.caps, CapsProfile::Rich);
        assert_eq!(
            detected.caps_source,
            CapsSource::Detected,
            "an empty environment value falls back to the detection"
        );
    }

    #[test]
    fn invalid_env_caps_reports_an_error_listing_the_valid_values() {
        let error = parse_cli_arguments_with_environment(
            arguments(&[]),
            &environment(&[(CAPS_ENVIRONMENT_VARIABLE, "dumb")]),
        )
        .expect_err("an unknown environment profile is rejected");
        assert_eq!(error, CliError::InvalidEnvCaps(String::from("dumb")));
        let message = error.to_string();
        assert!(
            message.contains("GITDECK_CAPS") && message.contains("dumb"),
            "the message names the variable and the value, message was {message:?}"
        );
        for valid_name in ["rich", "standard", "basic"] {
            assert!(
                message.contains(valid_name),
                "the message lists {valid_name}, message was {message:?}"
            );
        }
    }

    #[test]
    fn mouse_debug_flag_records_events() {
        let options = parse_cli_arguments_with_environment(
            arguments(&["--mouse-debug", "artifacts/tmp/mouse-debug.log"]),
            &environment(&[]),
        )
        .expect("the mouse debug flag parses");
        assert_eq!(
            options.mouse_debug_path,
            Some(PathBuf::from("artifacts/tmp/mouse-debug.log"))
        );
        let root = gitdeck_testkit::repository_root()
            .join("artifacts")
            .join("tmp");
        std::fs::create_dir_all(&root).expect("the scratch directory is created");
        let path = root.join("mouse-debug-records-events.log");
        let _ = std::fs::remove_file(&path);
        let mut log = MouseDebugLog::open(Some(&path)).expect("the mouse debug file opens");
        log.record(None);
        log.record(Some(&InputEvent::Key(KeyInput {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::default(),
        })));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "",
            "only mouse events reach the debug file"
        );
        log.record(Some(&InputEvent::Mouse(gitdeck_core::MouseInput {
            kind: MouseInputKind::Down,
            button: MouseButton::Left,
            column: 12,
            row: 7,
            modifiers: KeyModifiers::default(),
            click_count: 1,
        })));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "kind=Down button=Left column=12 row=7 click_count=1\n"
        );
        let _ = std::fs::remove_file(&path);
        assert!(
            MouseDebugLog::open(None)
                .expect("a missing flag opens no file")
                .file
                .is_none(),
            "the flag stays off when it is not passed"
        );
        let error = MouseDebugLog::open(Some(&root.join("missing-directory").join("mouse.log")))
            .expect_err("an unwritable path is reported");
        assert!(
            error.contains("cannot open the mouse debug file"),
            "the error names the flag, the message reads {error:?}"
        );
    }

    #[test]
    fn bare_double_dash_is_ignored() {
        assert_eq!(
            parse_cli_arguments_with_environment(arguments(&["--", "--doctor"]), &environment(&[])),
            Ok(CliOptions {
                caps: CapsProfile::Standard,
                caps_source: CapsSource::Detected,
                theme: ThemeId::JbDark,
                doctor: true,
                help: false,
                mouse_debug_path: None,
                repository_path: None,
            })
        );
    }

    #[test]
    fn double_dash_does_not_swallow_the_next_argument() {
        let options = parse_cli_arguments_with_environment(
            arguments(&["--", "--caps", "basic"]),
            &environment(&[]),
        )
        .expect("the argument list parses");
        assert_eq!(options.caps, CapsProfile::Basic);
    }

    #[test]
    fn double_dash_before_path_keeps_the_path() {
        let options =
            parse_cli_arguments_with_environment(arguments(&["--", "/tmp/x"]), &environment(&[]))
                .expect("the argument list parses");
        assert_eq!(options.repository_path, Some(PathBuf::from("/tmp/x")));
    }

    #[test]
    fn flags_override_caps_theme_and_doctor() {
        assert_eq!(
            parse_cli_arguments_with_environment(
                arguments(&["--caps", "basic", "--theme", "mono", "--doctor"]),
                &environment(&[])
            ),
            Ok(CliOptions {
                caps: CapsProfile::Basic,
                caps_source: CapsSource::Cli,
                theme: ThemeId::Mono,
                doctor: true,
                help: false,
                mouse_debug_path: None,
                repository_path: None,
            })
        );
    }

    #[test]
    fn invalid_and_incomplete_arguments_are_rejected() {
        assert_eq!(
            parse_cli_arguments_with_environment(arguments(&["--caps", "dumb"]), &environment(&[])),
            Err(CliError::InvalidCaps(String::from("dumb")))
        );
        assert_eq!(
            parse_cli_arguments_with_environment(
                arguments(&["--theme", "gruvbox"]),
                &environment(&[])
            ),
            Err(CliError::InvalidTheme(String::from("gruvbox")))
        );
        assert_eq!(
            parse_cli_arguments_with_environment(arguments(&["--caps"]), &environment(&[])),
            Err(CliError::MissingValue("--caps"))
        );
        assert_eq!(
            parse_cli_arguments_with_environment(arguments(&["--verbose"]), &environment(&[])),
            Err(CliError::UnknownArgument(String::from("--verbose")))
        );
    }

    #[test]
    fn a_positional_argument_becomes_the_repository_path() {
        let options = parse_cli_arguments_with_environment(
            arguments(&["/work/repo", "--theme", "nord"]),
            &environment(&[]),
        )
        .expect("the argument list parses");
        assert_eq!(options.repository_path, Some(PathBuf::from("/work/repo")));
        assert_eq!(options.theme, ThemeId::Nord);
        assert_eq!(
            parse_cli_arguments_with_environment(arguments(&["one", "two"]), &environment(&[])),
            Err(CliError::MultipleRepositoryPaths)
        );
    }

    #[test]
    fn doctor_report_lists_caps_caps_source_theme_and_size() {
        let options = CliOptions {
            caps: CapsProfile::Basic,
            caps_source: CapsSource::Environment,
            theme: ThemeId::JbLight,
            doctor: true,
            help: false,
            mouse_debug_path: None,
            repository_path: None,
        };
        assert_eq!(
            doctor_report(
                options,
                Size {
                    width: 120,
                    height: 32
                },
                CapsProfile::Basic
            ),
            "caps: basic\ncaps source: env\ntheme: jb-light\nsize: 120x32"
        );
    }

    #[test]
    fn doctor_reports_the_environment_profile_without_a_tty() {
        let detected = detect(&process_environment());
        let options =
            parse_cli_arguments(arguments(&["--doctor"])).expect("the doctor arguments parse");
        assert_eq!(
            options.caps, detected,
            "the doctor keeps the profile the environment detection yields"
        );
        assert_eq!(options.caps_source, CapsSource::Detected);

        let detected_options = parse_cli_arguments_with_environment(
            arguments(&["--doctor"]),
            &environment(&[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]),
        )
        .expect("the doctor arguments parse with the colourful environment");
        assert_eq!(detected_options.caps, CapsProfile::Rich);
        assert_eq!(
            doctor_report(
                detected_options,
                Size {
                    width: 120,
                    height: 32
                },
                CapsProfile::Basic
            ),
            "caps: rich\ncaps source: detected\ntheme: jb-dark\nsize: 120x32\nrender caps: basic (stdout is not a terminal)"
        );
    }

    #[test]
    fn a_piped_stdout_still_downgrades_the_render_profile() {
        let options = parse_cli_arguments_with_environment(
            arguments(&[]),
            &environment(&[("COLORTERM", "truecolor")]),
        )
        .expect("the arguments parse");
        assert_eq!(options.caps, CapsProfile::Rich);
        assert_eq!(
            apply_stdout_terminal_fallback(options.clone(), false).caps,
            CapsProfile::Basic,
            "a piped stdout renders with the basic profile"
        );
        assert_eq!(
            apply_stdout_terminal_fallback(options, true).caps,
            CapsProfile::Rich,
            "a terminal stdout keeps the detected profile"
        );
        if !std::io::stdout().is_terminal() {
            assert_eq!(
                render_caps_for_stdout(
                    &parse_cli_arguments_with_environment(
                        arguments(&[]),
                        &environment(&[("COLORTERM", "truecolor")])
                    )
                    .expect("the arguments parse")
                ),
                CapsProfile::Basic,
                "the render profile of a piped stdout is basic"
            );
        }
    }

    #[test]
    fn ctrl_q_and_ctrl_c_are_the_only_quit_keys() {
        let ctrl = KeyModifiers {
            ctrl: true,
            alt: false,
            shift: false,
        };
        for code in [KeyCode::Char('q'), KeyCode::Char('c')] {
            assert!(is_quit_key(&KeyInput {
                code,
                modifiers: ctrl,
            }));
        }
        assert!(
            !is_quit_key(&KeyInput {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::default(),
            }),
            "escape never quits the shell"
        );
        assert!(!is_quit_key(&KeyInput {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::default()
        }));
        assert!(!is_quit_key(&KeyInput {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::default()
        }));
    }

    fn mouse_down(column: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(TerminalMouseButton::Left),
            column,
            row,
            modifiers: TerminalModifiers::NONE,
        })
    }

    fn mouse_up(column: u16, row: u16) -> Event {
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Up(TerminalMouseButton::Left),
            column,
            row,
            modifiers: TerminalModifiers::NONE,
        })
    }

    fn mapped_click_count(mapper: &mut InputEventMapper, event: Event, now: Instant) -> u8 {
        match mapper.map_event_at(event, now) {
            Some(InputEvent::Mouse(mouse)) => mouse.click_count,
            other => panic!("the mouse event maps to a mouse input, got {other:?}"),
        }
    }

    #[test]
    fn help_flag_and_short_form_request_the_help_text() {
        for argument in ["--help", "-h"] {
            let options =
                parse_cli_arguments_with_environment(arguments(&[argument]), &environment(&[]))
                    .expect("the help argument parses");
            assert!(options.help, "{argument} requests the help text");
            assert!(!options.doctor);
        }
        let help = help_text();
        assert!(help.contains("--caps"));
        assert!(help.contains("--theme"));
        assert!(help.contains("--doctor"));
        assert!(help.contains("wheel over a pane"));
    }

    #[test]
    fn help_text_matches_the_binding_registry() {
        let help = help_text();
        let flat_help = collapse_whitespace(&help);
        for binding in gitdeck_app::KEY_BINDINGS {
            assert!(
                help.contains(binding.keys),
                "the help text names the {:?} binding",
                binding.keys
            );
            assert!(
                flat_help.contains(&collapse_whitespace(binding.action)),
                "the help text carries the {:?} action",
                binding.action
            );
        }
        for command in gitdeck_app::KeyCommand::EVERY {
            assert!(
                !gitdeck_app::key_bindings_for(command).is_empty(),
                "{command:?} is bound to a key"
            );
        }
        for combination in key_combinations_in_the_keys_column(&help) {
            assert!(
                !gitdeck_app::key_bindings_for_keys(&combination).is_empty(),
                "the help text lists the {combination:?} combination, the registry does not"
            );
        }
        for line in help.lines() {
            assert!(
                line.chars().count() <= super::HELP_MAX_COLUMNS,
                "the help line {line:?} stays inside eighty columns"
            );
        }
    }

    fn collapse_whitespace(text: &str) -> String {
        text.split_whitespace().collect::<Vec<&str>>().join(" ")
    }

    fn key_combinations_in_the_keys_column(help: &str) -> Vec<String> {
        let mut combinations: Vec<String> = Vec::new();
        for line in help.lines() {
            let keys_column: String = line.chars().take(super::HELP_KEY_COLUMN).collect();
            for word in
                keys_column.split(|character: char| character.is_whitespace() || character == ',')
            {
                if !(word.starts_with("alt+") || word.starts_with("ctrl+")) {
                    continue;
                }
                if !combinations.iter().any(|candidate| candidate == word) {
                    combinations.push(word.to_string());
                }
            }
        }
        combinations
    }

    #[test]
    fn a_space_keystroke_maps_to_the_space_key_code() {
        let space = Event::Key(KeyEvent::new_with_kind(
            TerminalKeyCode::Char(' '),
            TerminalModifiers::NONE,
            KeyEventKind::Press,
        ));
        assert_eq!(
            InputEventMapper::new().map_event(space),
            Some(InputEvent::Key(KeyInput {
                code: KeyCode::Space,
                modifiers: KeyModifiers::default(),
            }))
        );
    }

    #[test]
    fn key_events_map_to_input_events_and_releases_are_dropped() {
        let pressed = Event::Key(KeyEvent::new_with_kind(
            TerminalKeyCode::Char('k'),
            TerminalModifiers::CONTROL,
            KeyEventKind::Press,
        ));
        assert_eq!(
            InputEventMapper::new().map_event(pressed),
            Some(InputEvent::Key(KeyInput {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers {
                    ctrl: true,
                    alt: false,
                    shift: false
                }
            }))
        );
        let released = Event::Key(KeyEvent {
            code: TerminalKeyCode::Char('k'),
            modifiers: TerminalModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });
        assert_eq!(InputEventMapper::new().map_event(released), None);
    }

    #[test]
    fn mouse_and_resize_events_map_to_input_events() {
        assert_eq!(
            InputEventMapper::new().map_event(mouse_down(12, 7)),
            Some(InputEvent::Mouse(gitdeck_core::MouseInput {
                kind: MouseInputKind::Down,
                button: MouseButton::Left,
                column: 12,
                row: 7,
                modifiers: KeyModifiers::default(),
                click_count: 1,
            }))
        );
        assert_eq!(
            InputEventMapper::new().map_event(Event::Resize(160, 48)),
            Some(InputEvent::Resize(Size {
                width: 160,
                height: 48
            }))
        );
    }

    #[test]
    fn a_second_press_nearby_within_the_interval_counts_as_a_double_click() {
        let mut mapper = InputEventMapper::new();
        let first = Instant::now();
        assert_eq!(mapped_click_count(&mut mapper, mouse_down(12, 7), first), 1);
        assert_eq!(
            mapped_click_count(
                &mut mapper,
                mouse_down(13, 8),
                first + Duration::from_millis(120)
            ),
            2
        );
    }

    #[test]
    fn presses_apart_in_time_or_place_count_as_single_clicks() {
        let mut mapper = InputEventMapper::new();
        let first = Instant::now();
        assert_eq!(mapped_click_count(&mut mapper, mouse_down(12, 7), first), 1);
        assert_eq!(
            mapped_click_count(
                &mut mapper,
                mouse_down(12, 7),
                first + DOUBLE_CLICK_INTERVAL + Duration::from_millis(1)
            ),
            1,
            "a press after the interval starts a new click"
        );
        assert_eq!(
            mapped_click_count(
                &mut mapper,
                mouse_down(20, 7),
                first + DOUBLE_CLICK_INTERVAL + Duration::from_millis(2)
            ),
            1,
            "a press two columns away starts a new click"
        );
    }

    #[test]
    fn releases_and_wheel_events_carry_no_click_count() {
        let mut mapper = InputEventMapper::new();
        let now = Instant::now();
        assert_eq!(mapped_click_count(&mut mapper, mouse_up(12, 7), now), 0);
        let wheel = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 12,
            row: 7,
            modifiers: TerminalModifiers::NONE,
        });
        assert_eq!(mapped_click_count(&mut mapper, wheel, now), 0);
    }
}
