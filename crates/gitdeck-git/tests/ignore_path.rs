use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gitdeck_git::{CliGit, GitBackend, GitError};
use gitdeck_testkit::repository_root;

const SCRATCH_DIRECTORY: &str = "artifacts/tmp/ignore-path";
const IGNORED_PATH: &str = "notes.txt";
const NESTED_PATH: &str = "logs/debug.txt";
const TRACKED_PATH: &str = "src/tracked.txt";
const GITIGNORE_FILE_NAME: &str = ".gitignore";

fn run_git(working_directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=gitdeck-ignore-test",
            "-c",
            "user.email=ignore@gitdeck.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .arg("-C")
        .arg(working_directory)
        .args(arguments)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "git {arguments:?} failed in {}: {}",
        working_directory.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write_file(repository_path: &Path, relative_path: &str, contents: &str) {
    let path = repository_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the parent directory is created");
    }
    fs::write(&path, contents).expect("the repository file is written");
}

fn scratch_repository(name: &str) -> PathBuf {
    let repository_path = repository_root().join(SCRATCH_DIRECTORY).join(name);
    if repository_path.exists() {
        fs::remove_dir_all(&repository_path).expect("the stale scratch repository is removed");
    }
    fs::create_dir_all(&repository_path).expect("the scratch repository is created");
    run_git(&repository_path, &["init", "-q"]);
    write_file(&repository_path, TRACKED_PATH, "tracked\n");
    run_git(&repository_path, &["add", "--all"]);
    run_git(&repository_path, &["commit", "-q", "-m", "base commit"]);
    repository_path
}

fn read_gitignore(repository_path: &Path) -> String {
    fs::read_to_string(repository_path.join(GITIGNORE_FILE_NAME))
        .expect("the gitignore file is readable")
}

fn open(repository_path: &Path) -> CliGit {
    CliGit::open(repository_path).expect("the scratch repository opens")
}

#[test]
fn ignore_path_appends_to_gitignore_and_dedupes() {
    let repository_path = scratch_repository("append");
    write_file(&repository_path, IGNORED_PATH, "scratch\n");
    let repository = open(&repository_path);
    repository
        .ignore_path(IGNORED_PATH)
        .expect("the untracked path is ignored");
    assert_eq!(
        read_gitignore(&repository_path),
        format!("{IGNORED_PATH}\n")
    );
    repository
        .ignore_path(IGNORED_PATH)
        .expect("a second ignore of the same path is harmless");
    assert_eq!(
        read_gitignore(&repository_path),
        format!("{IGNORED_PATH}\n"),
        "the entry is written once"
    );
    assert!(
        run_git(
            &repository_path,
            &["status", "--porcelain", "--", IGNORED_PATH]
        )
        .is_empty(),
        "the ignored path leaves the untracked list"
    );
}

#[test]
fn ignore_path_preserves_crlf_and_avoids_a_double_entry() {
    let repository_path = scratch_repository("crlf");
    write_file(
        &repository_path,
        GITIGNORE_FILE_NAME,
        "target/\r\nexisting.txt\r\n",
    );
    let repository = open(&repository_path);
    repository
        .ignore_path(NESTED_PATH)
        .expect("the nested path is ignored");
    assert_eq!(
        read_gitignore(&repository_path),
        format!("target/\r\nexisting.txt\r\n{NESTED_PATH}\r\n"),
        "the appended entry keeps the CRLF line ending of the file"
    );
    repository
        .ignore_path("existing.txt")
        .expect("an entry that already exists is accepted");
    assert_eq!(
        read_gitignore(&repository_path),
        format!("target/\r\nexisting.txt\r\n{NESTED_PATH}\r\n"),
        "an existing entry is not written twice"
    );
}

#[test]
fn ignore_path_completes_a_missing_trailing_newline() {
    let repository_path = scratch_repository("no-trailing-newline");
    write_file(&repository_path, GITIGNORE_FILE_NAME, "target/");
    open(&repository_path)
        .ignore_path(IGNORED_PATH)
        .expect("the path is appended to the unterminated file");
    assert_eq!(read_gitignore(&repository_path), "target/\nnotes.txt\n");
}

#[test]
fn ignore_path_refuses_unsafe_paths() {
    let repository_path = scratch_repository("unsafe");
    let repository = open(&repository_path);
    for unsafe_path in [
        "",
        "-notes.txt",
        "bad\0name.txt",
        "/etc/passwd",
        "../outside.txt",
    ] {
        let error = repository
            .ignore_path(unsafe_path)
            .expect_err("an unsafe path is refused");
        assert!(
            matches!(error, GitError::UnsafePathspec { .. }),
            "{unsafe_path:?} reports the unsafe pathspec error, got {error:?}"
        );
    }
    assert!(
        !repository_path.join(GITIGNORE_FILE_NAME).exists(),
        "a refused path writes no gitignore file"
    );
}
