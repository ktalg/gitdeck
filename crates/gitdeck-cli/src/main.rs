use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use gitdeck_app::{App, TICK_INTERVAL};
use gitdeck_cli::{
    doctor_report, help_text, is_quit_key, parse_cli_arguments, render_caps_for_stdout, CliError,
    CliOptions, InputEventMapper, MouseDebugLog, DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH,
};
use gitdeck_core::{InputEvent, Size};
use gitdeck_tui::ScreenDriver;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn app_for_repository(options: &CliOptions) -> App {
    let repository_path = options
        .repository_path
        .clone()
        .unwrap_or_else(current_directory);
    App::open_repository(&repository_path, options.caps, options.theme)
}

fn current_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let options = parse_cli_arguments(arguments).map_err(|error: CliError| error.to_string())?;
    if options.help {
        println!("{}", help_text());
        return Ok(());
    }
    if options.doctor {
        let render_caps = render_caps_for_stdout(&options);
        println!("{}", doctor_report(options, terminal_size(), render_caps));
        return Ok(());
    }
    let mouse_debug_log = MouseDebugLog::open(options.mouse_debug_path.as_deref())?;
    run_shell(options, mouse_debug_log).map_err(|error| format!("terminal error: {error}"))
}

fn terminal_size() -> Size {
    match size() {
        Ok((width, height)) => Size { width, height },
        Err(_) => Size {
            width: DEFAULT_SCREEN_WIDTH,
            height: DEFAULT_SCREEN_HEIGHT,
        },
    }
}

fn run_shell(options: CliOptions, mut mouse_debug_log: MouseDebugLog) -> io::Result<()> {
    let _session = TerminalSession::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = app_for_repository(&options);
    app.handle_event(InputEvent::Resize(terminal_size()));
    let mut input_mapper = InputEventMapper::new();
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            app.render_frame(area, frame.buffer_mut());
        })?;
        if !event::poll(TICK_INTERVAL)? {
            app.handle_event(InputEvent::Tick);
            continue;
        }
        let input_event = input_mapper.map_event(event::read()?);
        mouse_debug_log.record(input_event.as_ref());
        match input_event {
            Some(InputEvent::Key(key_input)) if is_quit_key(&key_input) => {
                app.apply_quit_key(&key_input)
            }
            Some(input_event) => app.handle_event(input_event),
            None => {}
        }
        if app.exit_requested() {
            return Ok(());
        }
    }
}
