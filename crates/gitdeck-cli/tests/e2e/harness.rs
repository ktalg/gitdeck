use std::path::{Path, PathBuf};
use std::time::Duration;

use gitdeck_testkit::repository_root;
use gitdeck_testkit::{
    PtyScreenCell, PtySession, PtySessionOptions, FIXTURE_USER_EMAIL, FIXTURE_USER_NAME,
};

const TERMINAL_TYPE: &str = "xterm-256color";
const POSIX_LOCALE: &str = "C";
const SETTLE_TIMEOUT: Duration = Duration::from_secs(3);
const PINNED_CAPS_ARGUMENTS: [&str; 2] = ["--caps", "rich"];

pub struct TerminalSession {
    session: PtySession,
}

impl TerminalSession {
    pub fn spawn(
        repository: Option<&Path>,
        columns: u16,
        rows: u16,
        extra_arguments: &[&str],
    ) -> Self {
        let mut arguments = PINNED_CAPS_ARGUMENTS.to_vec();
        arguments.extend_from_slice(extra_arguments);
        Self::spawn_with_environment(
            repository,
            columns,
            rows,
            &session_environment(),
            &arguments,
        )
    }

    pub fn spawn_with_environment(
        repository: Option<&Path>,
        columns: u16,
        rows: u16,
        environment: &[(String, String)],
        arguments: &[&str],
    ) -> Self {
        let working_directory = working_directory(repository);
        let session = PtySession::spawn(&PtySessionOptions {
            program: Path::new(env!("CARGO_BIN_EXE_gitdeck")),
            working_directory: &working_directory,
            columns,
            rows,
            environment,
            arguments,
            settle_timeout: SETTLE_TIMEOUT,
        });
        Self { session }
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        self.session.send_bytes(bytes);
    }

    pub fn send_keys(&mut self, keys: &str) {
        self.session.send_keys(keys);
    }

    pub fn mouse_click(&mut self, column: u16, row: u16) {
        self.session.mouse_click(column, row);
    }

    pub fn mouse_double_click(&mut self, column: u16, row: u16) {
        self.session.mouse_double_click(column, row);
    }

    pub fn mouse_drag(&mut self, from: (u16, u16), to: (u16, u16)) {
        self.session.mouse_drag(from, to);
    }

    pub fn mouse_wheel_down(&mut self, column: u16, row: u16, notches: u8) {
        self.session.mouse_wheel(column, row, -i32::from(notches));
    }

    pub fn mouse_wheel_up(&mut self, column: u16, row: u16, notches: u8) {
        self.session.mouse_wheel(column, row, i32::from(notches));
    }

    pub fn resize(&mut self, columns: u16, rows: u16) {
        self.session.resize(columns, rows);
    }

    pub fn screen_text(&mut self) -> String {
        self.session.screen_text()
    }

    pub fn row_text(&mut self, row: u16, from_column: u16, to_column: u16) -> String {
        self.session.row_text(row, from_column, to_column)
    }

    pub fn cell(&mut self, column: u16, row: u16) -> PtyScreenCell {
        self.session.cell(column, row)
    }

    pub fn raw_log(&self) -> &[u8] {
        self.session.raw_log()
    }

    pub fn wait_for_text(&mut self, needle: &str, timeout: Duration) -> bool {
        self.session.wait_for_text(needle, timeout)
    }

    pub fn wait_for_exit(&mut self, timeout: Duration) -> Option<i32> {
        self.session.wait_for_exit(timeout)
    }

    pub fn try_exit_code(&mut self) -> Option<i32> {
        self.session.try_exit_code()
    }

    pub fn input_was_discarded_after_exit(&self) -> bool {
        self.session.input_was_discarded_after_exit()
    }
}

fn session_environment() -> Vec<(String, String)> {
    overridden_environment(&[])
}

pub fn overridden_environment(variables: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut environment = base_environment();
    for (name, value) in variables {
        environment.retain(|(existing_name, _)| existing_name != name);
        environment.push(((*name).to_string(), (*value).to_string()));
    }
    environment
}

fn base_environment() -> Vec<(String, String)> {
    [
        ("TERM".to_string(), TERMINAL_TYPE.to_string()),
        ("LC_ALL".to_string(), POSIX_LOCALE.to_string()),
        ("GIT_CONFIG_NOSYSTEM".to_string(), "1".to_string()),
        (
            "GIT_CONFIG_GLOBAL".to_string(),
            git_config_path().display().to_string(),
        ),
        (
            "GIT_CONFIG_SYSTEM".to_string(),
            git_config_path().display().to_string(),
        ),
        (
            "GIT_CEILING_DIRECTORIES".to_string(),
            temporary_root().display().to_string(),
        ),
        ("GIT_AUTHOR_NAME".to_string(), FIXTURE_USER_NAME.to_string()),
        (
            "GIT_AUTHOR_EMAIL".to_string(),
            FIXTURE_USER_EMAIL.to_string(),
        ),
        (
            "GIT_COMMITTER_NAME".to_string(),
            FIXTURE_USER_NAME.to_string(),
        ),
        (
            "GIT_COMMITTER_EMAIL".to_string(),
            FIXTURE_USER_EMAIL.to_string(),
        ),
    ]
    .into_iter()
    .collect()
}

fn working_directory(repository: Option<&Path>) -> PathBuf {
    match repository {
        Some(repository_path) => repository_path.to_path_buf(),
        None => {
            let directory = artifacts_directory().join("tmp").join("e2e-cwd");
            std::fs::create_dir_all(&directory).expect("the e2e working directory is created");
            directory
        }
    }
}

fn temporary_root() -> PathBuf {
    let directory = artifacts_directory().join("tmp");
    std::fs::create_dir_all(&directory).expect("the e2e temporary directory is created");
    directory
}

fn git_config_path() -> PathBuf {
    let directory = artifacts_directory().join("tmp");
    std::fs::create_dir_all(&directory).expect("the e2e git config directory is created");
    directory.join("e2e-gitconfig")
}

pub fn artifacts_directory() -> PathBuf {
    repository_root().join("artifacts")
}
