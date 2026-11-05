use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use crate::error::GitError;

const CONFIG_OVERRIDE_ARGUMENTS: [&str; 6] = [
    "-c",
    "core.quotepath=false",
    "-c",
    "color.ui=false",
    "-c",
    "core.fsmonitor=false",
];

const GIT_INDEX_FILE_VARIABLE: &str = "GIT_INDEX_FILE";

const REMOVED_INHERITED_VARIABLES: [&str; 5] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_NAMESPACE",
];

#[cfg(windows)]
const NULL_DEVICE_PATH: &str = "NUL";
#[cfg(not(windows))]
const NULL_DEVICE_PATH: &str = "/dev/null";

pub(crate) struct GitCommandResult {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub(crate) fn git_command(repository: &Path) -> Command {
    let mut command = base_git_command(repository);
    command.env("GIT_CONFIG_GLOBAL", NULL_DEVICE_PATH);
    command
}

pub(crate) fn write_git_command(repository: &Path) -> Command {
    let mut command = base_git_command(repository);
    command.env("GIT_MERGE_AUTOEDIT", "no");
    command
}

fn base_git_command(repository: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .args(CONFIG_OVERRIDE_ARGUMENTS)
        .arg("--no-pager")
        .arg("-C")
        .arg(repository)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C");
    for variable in REMOVED_INHERITED_VARIABLES {
        command.env_remove(variable);
    }
    command
}

pub(crate) fn capture_git_command<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
) -> Result<GitCommandResult, GitError> {
    let output = git_command(repository)
        .args(arguments)
        .output()
        .map_err(|source| GitError::Spawn {
            path: repository.to_path_buf(),
            source,
        })?;
    Ok(GitCommandResult {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

pub(crate) fn run_git_command<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
) -> Result<Vec<u8>, GitError> {
    let result = capture_git_command(repository, arguments)?;
    if !result.status.success() {
        return Err(command_failed_error(repository, arguments, &result.stderr));
    }
    Ok(result.stdout)
}

pub(crate) fn run_write_git_command<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
) -> Result<Vec<u8>, GitError> {
    run_command_with_status_check(write_git_command(repository), repository, arguments)
}

pub(crate) fn run_write_git_command_with_environment<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
    environment: &[(&str, &str)],
) -> Result<Vec<u8>, GitError> {
    let mut command = write_git_command(repository);
    for (name, value) in environment {
        command.env(name, value);
    }
    run_command_with_status_check(command, repository, arguments)
}

pub(crate) fn run_write_git_command_with_index<S: AsRef<OsStr>>(
    repository: &Path,
    index_file: &Path,
    arguments: &[S],
) -> Result<Vec<u8>, GitError> {
    let mut command = write_git_command(repository);
    command.env(GIT_INDEX_FILE_VARIABLE, index_file);
    run_command_with_status_check(command, repository, arguments)
}

fn run_command_with_status_check<S: AsRef<OsStr>>(
    mut command: Command,
    repository: &Path,
    arguments: &[S],
) -> Result<Vec<u8>, GitError> {
    let output = command
        .args(arguments)
        .output()
        .map_err(|source| GitError::Spawn {
            path: repository.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(command_failed_error(repository, arguments, &output.stderr));
    }
    Ok(output.stdout)
}

pub(crate) fn run_remote_git_command<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
) -> Result<Vec<u8>, GitError> {
    let output = write_git_command(repository)
        .args(arguments)
        .output()
        .map_err(|source| GitError::Spawn {
            path: repository.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            path: repository.to_path_buf(),
            arguments: describe_arguments(arguments),
            message: remote_failure_message(&output.stderr, &output.stdout),
        });
    }
    Ok(output.stdout)
}

pub(crate) fn run_write_git_command_with_input<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
    input: &[u8],
) -> Result<Vec<u8>, GitError> {
    run_command_with_input(write_git_command(repository), repository, arguments, input)
}

pub(crate) fn run_write_git_command_with_index_and_input<S: AsRef<OsStr>>(
    repository: &Path,
    index_file: &Path,
    arguments: &[S],
    input: &[u8],
) -> Result<Vec<u8>, GitError> {
    let mut command = write_git_command(repository);
    command.env(GIT_INDEX_FILE_VARIABLE, index_file);
    run_command_with_input(command, repository, arguments, input)
}

fn run_command_with_input<S: AsRef<OsStr>>(
    mut command: Command,
    repository: &Path,
    arguments: &[S],
    input: &[u8],
) -> Result<Vec<u8>, GitError> {
    let mut child = command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| GitError::Spawn {
            path: repository.to_path_buf(),
            source,
        })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| GitError::UnexpectedOutput {
            context: "git stdin",
            message: String::from("the child process has no input pipe"),
        })?;
    stdin
        .write_all(input)
        .map_err(|source| GitError::WriteInput {
            path: repository.to_path_buf(),
            source,
        })?;
    drop(stdin);
    let output = child.wait_with_output().map_err(|source| GitError::Spawn {
        path: repository.to_path_buf(),
        source,
    })?;
    if !output.status.success() {
        return Err(command_failed_error(repository, arguments, &output.stderr));
    }
    Ok(output.stdout)
}

pub(crate) fn command_failed_error<S: AsRef<OsStr>>(
    repository: &Path,
    arguments: &[S],
    stderr: &[u8],
) -> GitError {
    GitError::CommandFailed {
        path: repository.to_path_buf(),
        arguments: describe_arguments(arguments),
        message: first_stderr_line(stderr),
    }
}

fn describe_arguments<S: AsRef<OsStr>>(arguments: &[S]) -> String {
    let mut described = String::new();
    for argument in arguments {
        if !described.is_empty() {
            described.push(' ');
        }
        described.push_str(&argument.as_ref().to_string_lossy());
    }
    described
}

fn first_stderr_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| String::from("git reported no error text"))
}

fn remote_failure_message(stderr: &[u8], stdout: &[u8]) -> String {
    let stderr_text = String::from_utf8_lossy(stderr);
    let stdout_text = String::from_utf8_lossy(stdout);
    let mut reported = reported_lines(&stderr_text);
    let carries_a_failure_reason = reported
        .iter()
        .any(|line| line.contains("fatal:") || line.contains("error:"));
    if !carries_a_failure_reason {
        reported.extend(reported_lines(&stdout_text));
    }
    if reported.is_empty() {
        return String::from("git reported no error text");
    }
    reported.join("; ")
}

fn reported_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("hint:"))
        .collect()
}

#[cfg(test)]
mod command_tests {
    use std::path::Path;

    use super::{first_stderr_line, remote_failure_message};

    #[test]
    fn first_stderr_line_skips_leading_blank_lines_and_keeps_the_first_message() {
        assert_eq!(
            first_stderr_line(b"\nfatal: not a git repository\nhint: more text\n"),
            "fatal: not a git repository"
        );
    }

    #[test]
    fn remote_failure_message_keeps_the_rejection_line_and_drops_hint_lines() {
        assert_eq!(
            remote_failure_message(
                b"To /tmp/remote.git\n ! [rejected] main -> main (fetch first)\nhint: Updates were rejected.\nerror: failed to push some refs\n",
                b""
            ),
            "To /tmp/remote.git; ! [rejected] main -> main (fetch first); error: failed to push some refs"
        );
    }

    #[test]
    fn remote_failure_message_falls_back_to_stdout_when_stderr_only_carries_progress() {
        assert_eq!(
            remote_failure_message(
                b"From /tmp/remote\n   1a2b3c4..5d6e7f8  main       -> origin/main\n",
                b"Auto-merging src/app.rs\nCONFLICT (content): Merge conflict in src/app.rs\nAutomatic merge failed; fix conflicts and then commit the result.\n"
            ),
            "From /tmp/remote; 1a2b3c4..5d6e7f8  main       -> origin/main; Auto-merging src/app.rs; CONFLICT (content): Merge conflict in src/app.rs; Automatic merge failed; fix conflicts and then commit the result."
        );
    }

    #[test]
    fn first_stderr_line_reports_missing_text_explicitly() {
        assert_eq!(first_stderr_line(b"\n \n"), "git reported no error text");
    }

    #[test]
    fn git_command_redirects_configuration_away_from_the_user_environment() {
        let command = super::git_command(Path::new("/nonexistent"));
        let environment: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().to_string(),
                    value.map(|value| value.to_string_lossy().to_string()),
                )
            })
            .collect();
        let lookup = |name: &str| {
            environment
                .iter()
                .find(|(variable, _)| variable == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(lookup("GIT_OPTIONAL_LOCKS"), Some(Some(String::from("0"))));
        assert_eq!(lookup("GIT_TERMINAL_PROMPT"), Some(Some(String::from("0"))));
        assert_eq!(lookup("GIT_CONFIG_NOSYSTEM"), Some(Some(String::from("1"))));
        assert_eq!(
            lookup("GIT_CONFIG_GLOBAL"),
            Some(Some(super::NULL_DEVICE_PATH.to_string()))
        );
        assert_eq!(lookup("LC_ALL"), Some(Some(String::from("C"))));
        assert_eq!(lookup("GIT_DIR"), Some(None));
    }

    #[test]
    fn write_git_command_lets_git_resolve_the_user_identity() {
        let command = super::write_git_command(Path::new("/nonexistent"));
        let environment: Vec<(String, Option<String>)> = command
            .get_envs()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().to_string(),
                    value.map(|value| value.to_string_lossy().to_string()),
                )
            })
            .collect();
        let lookup = |name: &str| {
            environment
                .iter()
                .find(|(variable, _)| variable == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(
            lookup("GIT_CONFIG_GLOBAL"),
            None,
            "the write path inherits the user configuration instead of nulling it"
        );
        assert_eq!(lookup("GIT_CONFIG_NOSYSTEM"), Some(Some(String::from("1"))));
        assert_eq!(lookup("GIT_TERMINAL_PROMPT"), Some(Some(String::from("0"))));
    }
}
