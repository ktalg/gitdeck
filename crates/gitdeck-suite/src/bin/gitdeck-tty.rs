use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use gitdeck_testkit::{CapsProfile, PtySession, PtySessionOptions, ThemeId};

const DEFAULT_COLUMNS: u16 = 160;
const DEFAULT_ROWS: u16 = 48;
const DEFAULT_DELAY_MILLIS: u64 = 120;
const STEP_SETTLE_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);
const STARTUP_MARKER: &str = concat!("gitdeck ", env!("CARGO_PKG_VERSION"));
const TERMINAL_TYPE: &str = "xterm-256color";
const POSIX_LOCALE: &str = "C";
const PROGRAM_NAME: &str = "gitdeck";
const PROGRAM_PACKAGE: &str = "gitdeck-cli";
const NAMED_KEYS: [&str; 13] = [
    "enter",
    "return",
    "esc",
    "escape",
    "tab",
    "shift+tab",
    "backtab",
    "space",
    "up",
    "down",
    "left",
    "right",
    "f5",
];
const MODIFIER_PREFIXES: [&str; 3] = ["ctrl+", "alt+", "shift+"];
const STEP_KINDS: [&str; 16] = [
    "key",
    "text",
    "click",
    "dclick",
    "rclick",
    "clicktext",
    "dclicktext",
    "rclicktext",
    "find",
    "drag",
    "wheel",
    "resize",
    "wait",
    "waitfor",
    "waitchanged",
    "cell",
];
const WAIT_FOR_TEXT_TIMEOUT: Duration = Duration::from_secs(5);
const WAIT_FOR_CHANGE_POLL_INTERVAL: Duration = Duration::from_millis(50);

struct ProbeOptions {
    repository: PathBuf,
    sizes: Vec<(u16, u16)>,
    steps: Vec<Step>,
    png_directory: Option<PathBuf>,
    caps: CapsProfile,
    delay: Duration,
    diff: bool,
}

struct Step {
    label: String,
    action: StepAction,
}

struct StepReport<'a> {
    index: usize,
    step: &'a Step,
    size: (u16, u16),
    screen: &'a str,
    previous_screen: Option<&'a str>,
    note: Option<&'a str>,
}

impl Step {
    fn measured_quantity(&self) -> String {
        match self.action {
            StepAction::Wheel(_, _, delta) => {
                format!(", wheel events={}", delta.unsigned_abs())
            }
            _ => String::new(),
        }
    }
}

enum StepAction {
    Key(String),
    Text(String),
    Click(u16, u16),
    DoubleClick(u16, u16),
    RightClick(u16, u16),
    ClickText(String),
    DoubleClickText(String),
    RightClickText(String),
    Find(String),
    Drag((u16, u16), (u16, u16)),
    Wheel(u16, u16, i32),
    Resize(u16, u16),
    Wait(u64),
    WaitFor(String),
    WaitChanged,
    Inspect(u16, u16),
}

fn main() -> ExitCode {
    match run_probe(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("gitdeck-tty: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run_probe(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let options = parse_options(arguments.into_iter().collect())?;
    let program = gitdeck_program_path()?;
    let mut session = spawn_session(&program, &options, options.sizes[0])?;
    for (index, size) in options.sizes.iter().enumerate() {
        if index > 0 {
            session.resize(size.0, size.1);
        }
        run_steps(&mut session, &options, *size)?;
    }
    finish_session(&mut session)
}

fn spawn_session(
    program: &Path,
    options: &ProbeOptions,
    size: (u16, u16),
) -> Result<PtySession, String> {
    let environment = session_environment();
    let program_arguments = program_arguments(options.caps);
    let mut session = PtySession::spawn(&PtySessionOptions {
        program,
        working_directory: &options.repository,
        columns: size.0,
        rows: size.1,
        environment: &environment,
        arguments: &program_arguments,
        settle_timeout: STEP_SETTLE_TIMEOUT,
    });
    if !session.wait_for_text(STARTUP_MARKER, STARTUP_TIMEOUT) {
        return Err(format!(
            "{} did not render within {} ms, screen was:\n{}",
            program.display(),
            STARTUP_TIMEOUT.as_millis(),
            trim_trailing_blank_lines(&session.screen_text())
        ));
    }
    Ok(session)
}

fn session_environment() -> Vec<(String, String)> {
    [("TERM", TERMINAL_TYPE), ("LC_ALL", POSIX_LOCALE)]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn program_arguments(caps: CapsProfile) -> [&'static str; 2] {
    ["--caps", caps.as_str()]
}

fn run_steps(
    session: &mut PtySession,
    options: &ProbeOptions,
    start_size: (u16, u16),
) -> Result<(), String> {
    let mut current_size = start_size;
    let mut previous_screen: Option<String> = None;
    for (index, step) in options.steps.iter().enumerate() {
        let note = apply_step(session, &step.action, &mut current_size)?;
        std::thread::sleep(options.delay);
        let screen = session.screen_text();
        print_step_dump(
            session,
            options,
            StepReport {
                index: index + 1,
                step,
                size: current_size,
                screen: &screen,
                previous_screen: previous_screen.as_deref(),
                note: note.as_deref(),
            },
        );
        previous_screen = Some(screen);
    }
    Ok(())
}

fn apply_step(
    session: &mut PtySession,
    action: &StepAction,
    current_size: &mut (u16, u16),
) -> Result<Option<String>, String> {
    let note = match action {
        StepAction::Key(name) => {
            session.send_keys(name);
            None
        }
        StepAction::Text(text) => {
            session.send_bytes(text.as_bytes());
            None
        }
        StepAction::Click(column, row) => {
            session.mouse_click(*column, *row);
            None
        }
        StepAction::DoubleClick(column, row) => {
            session.mouse_double_click(*column, *row);
            None
        }
        StepAction::RightClick(column, row) => {
            session.mouse_right_click(*column, *row);
            None
        }
        StepAction::ClickText(target) => {
            Some(click_text(session, target, PtySession::mouse_click)?)
        }
        StepAction::DoubleClickText(target) => {
            Some(click_text(session, target, PtySession::mouse_double_click)?)
        }
        StepAction::RightClickText(target) => {
            Some(click_text(session, target, PtySession::mouse_right_click)?)
        }
        StepAction::Find(target) => {
            let (needle, _) = parse_text_target(target);
            let matches = session.find_text_cells(&needle);
            if matches.is_empty() {
                return Err(format!("text {needle:?} is not on the current screen"));
            }
            let listed: Vec<String> = matches
                .iter()
                .map(|(column, row)| format!("({column},{row})"))
                .collect();
            Some(format!("matches for {needle:?}: {}", listed.join(" ")))
        }
        StepAction::Drag(from, to) => {
            session.mouse_drag(*from, *to);
            None
        }
        StepAction::Wheel(column, row, delta) => {
            session.mouse_wheel(*column, *row, *delta);
            None
        }
        StepAction::Resize(columns, rows) => {
            session.resize(*columns, *rows);
            *current_size = (*columns, *rows);
            None
        }
        StepAction::Wait(millis) => {
            std::thread::sleep(Duration::from_millis(*millis));
            None
        }
        StepAction::WaitFor(needle) => {
            if !session.wait_for_text(needle, WAIT_FOR_TEXT_TIMEOUT) {
                return Err(format!("text {needle:?} did not appear on screen"));
            }
            None
        }
        StepAction::WaitChanged => {
            let before = session.screen_text();
            let deadline = std::time::Instant::now() + WAIT_FOR_TEXT_TIMEOUT;
            loop {
                std::thread::sleep(WAIT_FOR_CHANGE_POLL_INTERVAL);
                if session.screen_text() != before {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    return Err("the screen did not change".to_string());
                }
            }
            None
        }
        StepAction::Inspect(_, _) => None,
    };
    Ok(note)
}

fn print_step_dump(session: &mut PtySession, options: &ProbeOptions, report: StepReport<'_>) {
    println!(
        "=== step {}: {} (size {}x{}{}) ===",
        report.index,
        report.step.label,
        report.size.0,
        report.size.1,
        report.step.measured_quantity()
    );
    if let Some(note) = report.note {
        println!("{note}");
    }
    match (options.diff, report.previous_screen) {
        (true, Some(previous)) => print_screen_diff(previous, report.screen),
        _ => println!("{}", trim_trailing_blank_lines(report.screen)),
    }
    if let StepAction::Inspect(column, row) = report.step.action {
        let inspected = session.cell(column, row);
        println!(
            "cell ({column},{row}) = {:?} fg={:?} bg={:?}",
            inspected.symbol, inspected.foreground, inspected.background
        );
    }
    if let Some(directory) = options.png_directory.as_deref() {
        let name = format!(
            "{}x{}-step-{:02}-{}",
            report.size.0,
            report.size.1,
            report.index,
            sanitized_label(&report.step.label)
        );
        session.screenshot_into(directory, &name, ThemeId::JbDark);
    }
}

fn finish_session(session: &mut PtySession) -> Result<(), String> {
    if let Some(exit_code) = session.try_exit_code() {
        println!("exit={exit_code}");
        return Ok(());
    }
    session.send_keys("ctrl+q");
    match session.wait_for_exit(EXIT_TIMEOUT) {
        Some(exit_code) => {
            println!("exit={exit_code}");
            if exit_code == 0 {
                Ok(())
            } else {
                Err(format!("{PROGRAM_NAME} exited with status {exit_code}"))
            }
        }
        None => Err(format!(
            "{PROGRAM_NAME} did not exit within {} ms and was killed",
            EXIT_TIMEOUT.as_millis()
        )),
    }
}

fn print_screen_diff(previous: &str, current: &str) {
    let old_rows: Vec<&str> = previous.split('\n').collect();
    let new_rows: Vec<&str> = current.split('\n').collect();
    let row_count = old_rows.len().max(new_rows.len());
    let mut changed_rows = 0;
    for row_index in 0..row_count {
        let old_row = old_rows.get(row_index).copied().unwrap_or("");
        let new_row = new_rows.get(row_index).copied().unwrap_or("");
        if old_row == new_row {
            continue;
        }
        changed_rows += 1;
        if !old_row.is_empty() {
            println!("-{row_index:02} {old_row}");
        }
        if !new_row.is_empty() {
            println!("+{row_index:02} {new_row}");
        }
    }
    if changed_rows == 0 {
        println!("(no screen change)");
    }
}

fn trim_trailing_blank_lines(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    while lines.last().is_some_and(|line| line.trim_end().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn sanitized_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn gitdeck_program_path() -> Result<PathBuf, String> {
    let sibling = std::env::current_exe()
        .map_err(|error| format!("cannot locate this probe binary: {error}"))?
        .parent()
        .ok_or_else(|| "this probe binary has no parent directory".to_string())?
        .join(PROGRAM_NAME);
    build_gitdeck_program()?;
    if sibling.is_file() {
        return Ok(sibling);
    }
    Err(format!(
        "{} is still missing after building {PROGRAM_PACKAGE}",
        sibling.display()
    ))
}

fn build_gitdeck_program() -> Result<(), String> {
    let status = Command::new("cargo")
        .args(["build", "-q", "-p", PROGRAM_PACKAGE, "--bin", PROGRAM_NAME])
        .status()
        .map_err(|error| format!("cannot start cargo to build {PROGRAM_NAME}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "building {PROGRAM_NAME} failed with status {status}"
        ))
    }
}

fn parse_options(arguments: Vec<String>) -> Result<ProbeOptions, String> {
    let mut repository = None;
    let mut size = None;
    let mut sizes = None;
    let mut steps = Vec::new();
    let mut steps_source: Option<&'static str> = None;
    let mut png_directory = None;
    let mut caps = CapsProfile::Rich;
    let mut delay = Duration::from_millis(DEFAULT_DELAY_MILLIS);
    let mut diff = false;
    let mut remaining = arguments.into_iter();
    while let Some(argument) = remaining.next() {
        match argument.as_str() {
            "--repo" => repository = Some(PathBuf::from(next_value("--repo", &mut remaining)?)),
            "--size" => size = Some(parse_size(&next_value("--size", &mut remaining)?)?),
            "--sizes" => sizes = Some(parse_sizes(&next_value("--sizes", &mut remaining)?)?),
            "--steps" => {
                claimed_steps_source(&mut steps_source, "--steps")?;
                steps = parse_steps(&next_value("--steps", &mut remaining)?)?;
            }
            "--steps-file" => {
                claimed_steps_source(&mut steps_source, "--steps-file")?;
                let path = next_value("--steps-file", &mut remaining)?;
                steps = parse_steps_file(Path::new(&path))?;
            }
            "--caps" => {
                let value = next_value("--caps", &mut remaining)?;
                caps = CapsProfile::parse_cli(&value).ok_or_else(|| {
                    format!("--caps expects rich, standard or basic, got {value:?}")
                })?;
            }
            "--png" => png_directory = Some(PathBuf::from(next_value("--png", &mut remaining)?)),
            "--diff" => diff = true,
            "--delay-ms" => {
                let milliseconds = next_value("--delay-ms", &mut remaining)?;
                delay =
                    Duration::from_millis(milliseconds.parse().map_err(|_| {
                        format!("--delay-ms expects a number, got {milliseconds:?}")
                    })?);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let repository = repository.ok_or_else(|| "--repo <path> is required".to_string())?;
    let sizes = match (size, sizes) {
        (Some(single), None) => vec![single],
        (None, Some(many)) => many,
        (None, None) => vec![(DEFAULT_COLUMNS, DEFAULT_ROWS)],
        (Some(_), Some(_)) => return Err("--size and --sizes are exclusive".to_string()),
    };
    Ok(ProbeOptions {
        repository,
        sizes,
        steps,
        png_directory,
        caps,
        delay,
        diff,
    })
}

fn claimed_steps_source(
    current: &mut Option<&'static str>,
    name: &'static str,
) -> Result<(), String> {
    match *current {
        Some(previous) if previous != name => {
            Err(format!("{} and {} are exclusive", previous, name))
        }
        _ => {
            *current = Some(name);
            Ok(())
        }
    }
}

fn next_value(name: &str, remaining: &mut std::vec::IntoIter<String>) -> Result<String, String> {
    remaining
        .next()
        .ok_or_else(|| format!("{name} needs a value"))
}

fn parse_size(text: &str) -> Result<(u16, u16), String> {
    let (columns, rows) = text
        .split_once('x')
        .ok_or_else(|| format!("size {text:?} is not <columns>x<rows>"))?;
    Ok((parse_number(columns)?, parse_number(rows)?))
}

fn parse_sizes(text: &str) -> Result<Vec<(u16, u16)>, String> {
    let sizes: Vec<(u16, u16)> = text
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(parse_size)
        .collect::<Result<_, _>>()?;
    if sizes.is_empty() {
        return Err("--sizes needs at least one <columns>x<rows> entry".to_string());
    }
    Ok(sizes)
}

fn parse_number(text: &str) -> Result<u16, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("{text:?} is not a terminal dimension"))
}

fn parse_position(text: &str) -> Result<(u16, u16), String> {
    let (column, row) = text
        .split_once(',')
        .ok_or_else(|| format!("position {text:?} is not <column>,<row>"))?;
    Ok((parse_number(column)?, parse_number(row)?))
}

fn parse_steps(script: &str) -> Result<Vec<Step>, String> {
    split_step_tokens(script)
        .into_iter()
        .map(|token| parse_step(&token))
        .collect()
}

fn parse_steps_file(path: &Path) -> Result<Vec<Step>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read the steps file {}: {error}", path.display()))?;
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(parse_step)
        .collect()
}

fn split_step_tokens(script: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    for segment in script.split(',') {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }
        match tokens.last_mut() {
            Some(current) if !starts_step(trimmed) => {
                current.push(',');
                current.push_str(trimmed);
            }
            _ => tokens.push(trimmed.to_string()),
        }
    }
    tokens
}

fn starts_step(segment: &str) -> bool {
    segment
        .split_once(':')
        .is_some_and(|(kind, _)| STEP_KINDS.contains(&kind))
}

fn parse_step(token: &str) -> Result<Step, String> {
    let (kind, payload) = token
        .split_once(':')
        .ok_or_else(|| format!("step {token:?} is not <kind>:<payload>"))?;
    let action = match kind {
        "key" => StepAction::Key(known_key(payload)?),
        "text" => StepAction::Text(payload.to_string()),
        "click" => {
            let (column, row) = parse_position(payload)?;
            StepAction::Click(column, row)
        }
        "dclick" => {
            let (column, row) = parse_position(payload)?;
            StepAction::DoubleClick(column, row)
        }
        "rclick" => {
            let (column, row) = parse_position(payload)?;
            StepAction::RightClick(column, row)
        }
        "clicktext" => StepAction::ClickText(payload.to_string()),
        "dclicktext" => StepAction::DoubleClickText(payload.to_string()),
        "rclicktext" => StepAction::RightClickText(payload.to_string()),
        "find" => StepAction::Find(payload.to_string()),
        "drag" => {
            let (from, to) = payload
                .split_once("->")
                .ok_or_else(|| format!("drag {payload:?} is not <c1>,<r1>-><c2>,<r2>"))?;
            StepAction::Drag(parse_position(from)?, parse_position(to)?)
        }
        "wheel" => {
            let parts: Vec<&str> = payload.split(',').collect();
            let [column, row, delta] = parts.as_slice() else {
                return Err(format!("wheel {payload:?} is not <col>,<row>,<delta>"));
            };
            StepAction::Wheel(
                parse_number(column)?,
                parse_number(row)?,
                delta
                    .trim()
                    .parse()
                    .map_err(|_| format!("wheel delta {delta:?} is not a number"))?,
            )
        }
        "resize" => {
            let (columns, rows) = parse_size(payload)?;
            StepAction::Resize(columns, rows)
        }
        "waitfor" => StepAction::WaitFor(payload.to_string()),
        "waitchanged" => StepAction::WaitChanged,
        "cell" => {
            let (column, row) = parse_position(payload)?;
            StepAction::Inspect(column, row)
        }
        "wait" => StepAction::Wait(
            payload
                .trim()
                .parse()
                .map_err(|_| format!("wait {payload:?} is not a number of milliseconds"))?,
        ),
        other => return Err(format!("unknown step kind {other:?}")),
    };
    Ok(Step {
        label: token.to_string(),
        action,
    })
}

fn click_text(
    session: &mut PtySession,
    target: &str,
    click: fn(&mut PtySession, u16, u16),
) -> Result<String, String> {
    let (needle, required_row) = parse_text_target(target);
    let matches = session.find_text_cells(&needle);
    if matches.is_empty() {
        return Err(format!("text {needle:?} is not on the current screen"));
    }
    let chosen = match required_row {
        Some(row) => matches.iter().find(|(_, match_row)| *match_row == row),
        None => matches.first(),
    };
    let Some((column, row)) = chosen else {
        let listed: Vec<String> = matches
            .iter()
            .map(|(column, row)| format!("({column},{row})"))
            .collect();
        return Err(format!(
            "text {needle:?} is not on row {required_row:?}, it appears at {}",
            listed.join(" ")
        ));
    };
    click(session, *column, *row);
    Ok(format!("clicked {needle:?} at ({column},{row})"))
}

fn parse_text_target(target: &str) -> (String, Option<u16>) {
    match target.rsplit_once('@') {
        Some((needle, row)) if !needle.is_empty() => match row.trim().parse::<u16>() {
            Ok(row) => (needle.to_string(), Some(row)),
            Err(_) => (target.to_string(), None),
        },
        _ => (target.to_string(), None),
    }
}

fn known_key(name: &str) -> Result<String, String> {
    let lowered = name.to_ascii_lowercase();
    if NAMED_KEYS.contains(&lowered.as_str()) || name.chars().count() == 1 {
        return Ok(name.to_string());
    }
    let modified = MODIFIER_PREFIXES.iter().any(|prefix| {
        lowered
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.chars().count() == 1 || NAMED_KEYS.contains(&rest))
    });
    if modified {
        Ok(name.to_string())
    } else {
        Err(format!("unrecognized key name {name:?}"))
    }
}
