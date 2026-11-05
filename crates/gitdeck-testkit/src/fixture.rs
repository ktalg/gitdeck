use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::paths::artifacts_root;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum FixtureRepo {
    Empty,
    Dirty,
    MergeConflict,
    RebaseDiverged,
    CjkPaths,
    LargeTenThousand,
    DetachedHead,
    DiffShowcase,
}

impl FixtureRepo {
    pub const fn directory_name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Dirty => "dirty",
            Self::MergeConflict => "merge-conflict",
            Self::RebaseDiverged => "rebase-diverged",
            Self::CjkPaths => "cjk-paths",
            Self::LargeTenThousand => "large-10k",
            Self::DetachedHead => "detached-head",
            Self::DiffShowcase => "diff-showcase",
        }
    }
}

const FIXTURE_MARKER_FILE_NAME: &str = "gitdeck-fixture-marker";
const FIXTURE_MARKER_CONTENT: &str = "gitdeck-fixture-spec-4";
const FIXTURE_CONFIG_FILE_NAME: &str = "gitconfig";
pub const FIXTURE_USER_NAME: &str = "gitdeck-fixture";
pub const FIXTURE_USER_EMAIL: &str = "fixture@gitdeck.invalid";
const GIT_IDENTITY_VARIABLES: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", FIXTURE_USER_NAME),
    ("GIT_AUTHOR_EMAIL", FIXTURE_USER_EMAIL),
    ("GIT_COMMITTER_NAME", FIXTURE_USER_NAME),
    ("GIT_COMMITTER_EMAIL", FIXTURE_USER_EMAIL),
];
#[cfg(windows)]
const NULL_DEVICE_PATH: &str = "NUL";
#[cfg(not(windows))]
const NULL_DEVICE_PATH: &str = "/dev/null";
const FIXTURE_BASE_EPOCH_SECONDS: u64 = 1_577_836_800;
const LARGE_FIXTURE_COMMIT_COUNT: u32 = 10_000;
const LARGE_FIXTURE_COMMITS_PER_BLOCK: u32 = 100;
const LARGE_FIXTURE_TIME_LIMIT: Duration = Duration::from_secs(30);
const SHOWCASE_MODIFIED_LINE_COUNT: u32 = 40;
const SHOWCASE_MODIFIED_LINE_NUMBERS: [u32; 3] = [5, 12, 30];
const SHOWCASE_REMOVED_LINE_NUMBER: u32 = 20;
const SHOWCASE_REPLACEMENT_LINE_NUMBER: u32 = 41;
const SHOWCASE_UNTRACKED_LINE_COUNT: u32 = 10;
const SHOWCASE_LARGE_FILE_LINE_COUNT: u32 = 5_000;
const SHOWCASE_LARGE_FILE_CHANGED_EVERY: u32 = 500;
const SHOWCASE_LONG_LINE_LENGTH: usize = 400;
const SHOWCASE_BINARY_PATH: &str = "assets/logo.bin";
const SHOWCASE_BINARY_COMMITTED_CONTENT: &[u8] = b"gitdeck-logo\n\x00\x00\x01\x02\n";
const SHOWCASE_BINARY_MODIFIED_CONTENT: &[u8] = b"gitdeck-logo\n\x00\x00\x03\x04\n";

static FIXTURE_GENERATION_LOCK: Mutex<()> = Mutex::new(());

pub fn fixture_repo(name: FixtureRepo) -> PathBuf {
    let _generation_guard = FIXTURE_GENERATION_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let fixture_path = fixtures_root().join(name.directory_name());
    if fixture_marker_matches(&fixture_path) {
        return fixture_path;
    }
    reset_fixture_directory(&fixture_path);
    populate_fixture(name, &fixture_path);
    verify_fixture_head(&fixture_path, name);
    write_fixture_marker(&fixture_path);
    fixture_path
}

fn reset_fixture_directory(fixture_path: &Path) {
    ensure_fixture_git_config();
    if fixture_path.exists() {
        fs::remove_dir_all(fixture_path).unwrap_or_else(|error| {
            panic!(
                "cannot remove stale fixture {}: {error}",
                fixture_path.display()
            )
        });
    }
    fs::create_dir_all(fixture_path).unwrap_or_else(|error| {
        panic!("cannot create fixture {}: {error}", fixture_path.display())
    });
    git_checked(fixture_path, 0, &["init", "-q", "-b", "main"]);
    write_fixture_identity(fixture_path);
}

fn write_fixture_identity(fixture_path: &Path) {
    git_checked(fixture_path, 0, &["config", "user.name", FIXTURE_USER_NAME]);
    git_checked(
        fixture_path,
        0,
        &["config", "user.email", FIXTURE_USER_EMAIL],
    );
    git_checked(fixture_path, 0, &["config", "commit.gpgsign", "false"]);
    git_checked(fixture_path, 0, &["config", "tag.gpgsign", "false"]);
    git_checked(fixture_path, 0, &["config", "core.hooksPath", "/dev/null"]);
}

fn populate_fixture(name: FixtureRepo, fixture_path: &Path) {
    match name {
        FixtureRepo::Empty => {}
        FixtureRepo::Dirty => populate_dirty(fixture_path),
        FixtureRepo::MergeConflict => populate_merge_conflict(fixture_path),
        FixtureRepo::RebaseDiverged => populate_rebase_diverged(fixture_path),
        FixtureRepo::CjkPaths => populate_cjk_paths(fixture_path),
        FixtureRepo::LargeTenThousand => populate_large_ten_thousand(fixture_path),
        FixtureRepo::DetachedHead => populate_detached_head(fixture_path),
        FixtureRepo::DiffShowcase => populate_diff_showcase(fixture_path),
    }
}

fn write_fixture_marker(fixture_path: &Path) {
    let marker_path = fixture_marker_path(fixture_path);
    fs::write(&marker_path, FIXTURE_MARKER_CONTENT).unwrap_or_else(|error| {
        panic!(
            "cannot write fixture marker {}: {error}",
            marker_path.display()
        )
    });
}

fn fixture_marker_path(fixture_path: &Path) -> PathBuf {
    fixture_path.join(".git").join(FIXTURE_MARKER_FILE_NAME)
}

fn fixtures_root() -> PathBuf {
    artifacts_root().join("fixtures")
}

fn fixture_git_config_path() -> PathBuf {
    fixtures_root().join(FIXTURE_CONFIG_FILE_NAME)
}

fn ensure_fixture_git_config() {
    let config_path = fixture_git_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "cannot create fixture config directory {}: {error}",
                parent.display()
            )
        });
    }
    let config = format!(
        "[user]\n\tname = {FIXTURE_USER_NAME}\n\temail = {FIXTURE_USER_EMAIL}\n\
         [init]\n\tdefaultBranch = main\n\
         [core]\n\tautocrlf = false\n\tquotePath = false\n\thooksPath = /dev/null\n\
         [commit]\n\tgpgsign = false\n\
         [tag]\n\tgpgsign = false\n"
    );
    fs::write(&config_path, config).unwrap_or_else(|error| {
        panic!(
            "cannot write fixture git config {}: {error}",
            config_path.display()
        )
    });
}

fn fixture_marker_matches(fixture_path: &Path) -> bool {
    if !fixture_path.join(".git").exists() {
        return false;
    }
    fs::read_to_string(fixture_marker_path(fixture_path))
        .map(|content| content == FIXTURE_MARKER_CONTENT)
        .unwrap_or(false)
}

pub fn fixture_git_command(repository_path: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repository_path)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_DEVICE_PATH)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .envs(GIT_IDENTITY_VARIABLES)
        .stdin(Stdio::null());
    command
}

fn git_command(fixture_path: &Path, offset_seconds: u64) -> Command {
    let mut command = fixture_git_command(fixture_path);
    command
        .env("GIT_CONFIG_GLOBAL", fixture_git_config_path())
        .env("GIT_AUTHOR_DATE", git_date(offset_seconds))
        .env("GIT_COMMITTER_DATE", git_date(offset_seconds))
        .env("TZ", "UTC");
    command
}

fn git_date(offset_seconds: u64) -> String {
    format!("{} +0000", FIXTURE_BASE_EPOCH_SECONDS + offset_seconds)
}

fn run_git(fixture_path: &Path, offset_seconds: u64, arguments: &[&str]) -> Output {
    git_command(fixture_path, offset_seconds)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"))
}

fn git_checked(fixture_path: &Path, offset_seconds: u64, arguments: &[&str]) -> String {
    let output = run_git(fixture_path, offset_seconds, arguments);
    assert!(
        output.status.success(),
        "git {arguments:?} failed in {}: {}",
        fixture_path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn write_fixture_file(fixture_path: &Path, relative_path: &str, content: &str) {
    write_fixture_bytes(fixture_path, relative_path, content.as_bytes());
}

fn write_fixture_bytes(fixture_path: &Path, relative_path: &str, content: &[u8]) {
    let path = fixture_path.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "cannot create fixture directory {}: {error}",
                parent.display()
            )
        });
    }
    fs::write(&path, content)
        .unwrap_or_else(|error| panic!("cannot write fixture file {}: {error}", path.display()));
}

fn remove_fixture_file(fixture_path: &Path, relative_path: &str) {
    let path = fixture_path.join(relative_path);
    fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("cannot remove fixture file {}: {error}", path.display()));
}

fn commit_everything(fixture_path: &Path, message: &str, offset_seconds: u64) {
    git_checked(fixture_path, offset_seconds, &["add", "--all"]);
    git_checked(
        fixture_path,
        offset_seconds,
        &["commit", "-q", "-m", message],
    );
}

fn populate_dirty(fixture_path: &Path) {
    write_fixture_file(fixture_path, "src/app.rs", "fn main() {\n    run();\n}\n");
    write_fixture_file(fixture_path, "docs/readme.md", "# readme\n");
    commit_everything(fixture_path, "initial import", 0);

    write_fixture_file(
        fixture_path,
        "src/app.rs",
        "fn main() {\n    run_twice();\n}\n",
    );
    commit_everything(fixture_path, "call run twice", 60);

    write_fixture_file(
        fixture_path,
        "src/app.rs",
        "fn main() {\n    run_twice();\n    report();\n}\n",
    );
    git_checked(fixture_path, 0, &["add", "src/app.rs"]);
    write_fixture_file(
        fixture_path,
        "src/app.rs",
        "fn main() {\n    run_twice();\n    report();\n    shutdown();\n}\n",
    );
    write_fixture_file(fixture_path, "notes.txt", "untracked notes\n");
    write_fixture_file(fixture_path, "src/module.rs", "pub fn added() {}\n");
    git_checked(fixture_path, 0, &["add", "src/module.rs"]);
    remove_fixture_file(fixture_path, "docs/readme.md");
}

fn populate_merge_conflict(fixture_path: &Path) {
    write_fixture_file(
        fixture_path,
        "src/conflict.txt",
        "first line\nshared line\nlast line\n",
    );
    commit_everything(fixture_path, "base commit", 0);

    git_checked(fixture_path, 0, &["switch", "-q", "-c", "feature"]);
    write_fixture_file(
        fixture_path,
        "src/conflict.txt",
        "first line\nfeature line\nlast line\n",
    );
    commit_everything(fixture_path, "feature branch change", 60);

    git_checked(fixture_path, 0, &["switch", "-q", "main"]);
    write_fixture_file(
        fixture_path,
        "src/conflict.txt",
        "first line\nmain line\nlast line\n",
    );
    commit_everything(fixture_path, "main branch change", 120);

    let merge_output = run_git(fixture_path, 0, &["merge", "--no-edit", "feature"]);
    assert!(
        !merge_output.status.success(),
        "merge-conflict fixture merge unexpectedly succeeded"
    );
    let conflicted = fs::read_to_string(fixture_path.join("src/conflict.txt"))
        .unwrap_or_else(|error| panic!("cannot read conflicted file: {error}"));
    assert!(
        conflicted.contains("<<<<<<<") && conflicted.contains(">>>>>>>"),
        "conflict markers are missing: {conflicted}"
    );
}

fn populate_rebase_diverged(fixture_path: &Path) {
    write_fixture_file(
        fixture_path,
        "src/service.rs",
        "pub fn version() -> u8 {\n    1\n}\n",
    );
    commit_everything(fixture_path, "base commit", 0);
    write_fixture_file(
        fixture_path,
        "src/service.rs",
        "pub fn version() -> u8 {\n    2\n}\n",
    );
    commit_everything(fixture_path, "bump version", 60);

    git_checked(fixture_path, 0, &["switch", "-q", "-c", "feature"]);
    write_fixture_file(fixture_path, "src/feature.rs", "pub fn feature() {}\n");
    commit_everything(fixture_path, "feature work", 120);
    write_fixture_file(
        fixture_path,
        "src/feature.rs",
        "pub fn feature() {}\n\npub fn helper() {}\n",
    );
    commit_everything(fixture_path, "more feature work", 180);

    git_checked(fixture_path, 0, &["switch", "-q", "main"]);
    write_fixture_file(
        fixture_path,
        "src/service.rs",
        "pub fn version() -> u8 {\n    3\n}\n",
    );
    commit_everything(fixture_path, "main work", 240);
    write_fixture_file(fixture_path, "src/main.rs", "pub fn main_entry() {}\n");
    commit_everything(fixture_path, "more main work", 300);

    git_checked(fixture_path, 0, &["switch", "-q", "feature"]);
}

fn populate_cjk_paths(fixture_path: &Path) {
    write_fixture_file(
        fixture_path,
        "文档/说明.md",
        "# 说明\n\n这是中文路径样例。\n",
    );
    write_fixture_file(fixture_path, "中文文件.txt", "中文内容\n");
    write_fixture_file(fixture_path, "emoji-📦.txt", "emoji 文件名\n");
    commit_everything(fixture_path, "添加中文路径与中文提交信息", 0);

    write_fixture_file(
        fixture_path,
        "文档/说明.md",
        "# 说明\n\n更新后的中文内容。\n",
    );
    commit_everything(fixture_path, "更新：说明文档", 60);

    write_fixture_file(
        fixture_path,
        "文档/说明.md",
        "# 说明\n\n更新后的中文内容。\n变更：用于变更列表校验\n",
    );
    write_fixture_file(
        fixture_path,
        "新增/未跟踪 文件.txt",
        "未跟踪的中文文件内容\n",
    );
}

fn populate_detached_head(fixture_path: &Path) {
    write_fixture_file(fixture_path, "src/detached.rs", "pub fn first() {}\n");
    commit_everything(fixture_path, "first commit", 0);
    write_fixture_file(
        fixture_path,
        "src/detached.rs",
        "pub fn first() {}\n\npub fn second() {}\n",
    );
    commit_everything(fixture_path, "second commit", 60);
    git_checked(fixture_path, 0, &["switch", "--detach", "HEAD"]);
    let head = git_checked(fixture_path, 0, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(
        head.trim(),
        "HEAD",
        "detached-head fixture did not detach HEAD"
    );
}

fn populate_diff_showcase(fixture_path: &Path) {
    write_fixture_file(
        fixture_path,
        "src/modified.rs",
        &showcase_modified_lines(false),
    );
    write_fixture_file(
        fixture_path,
        "src/no_newline.txt",
        "first line\nlast line\n",
    );
    write_fixture_file(fixture_path, "src/long_line.txt", &showcase_long_line("a"));
    write_fixture_file(
        fixture_path,
        "文档/说明.md",
        &showcase_markdown("第一行内容"),
    );
    write_fixture_file(fixture_path, "src/large.txt", &showcase_large_file(false));
    write_fixture_bytes(
        fixture_path,
        SHOWCASE_BINARY_PATH,
        SHOWCASE_BINARY_COMMITTED_CONTENT,
    );
    commit_everything(fixture_path, "diff showcase base", 0);

    write_fixture_file(
        fixture_path,
        "src/modified.rs",
        &showcase_modified_lines(true),
    );
    write_fixture_file(
        fixture_path,
        "src/untracked_new.rs",
        &showcase_untracked_lines(),
    );
    write_fixture_bytes(
        fixture_path,
        SHOWCASE_BINARY_PATH,
        SHOWCASE_BINARY_MODIFIED_CONTENT,
    );
    write_fixture_file(
        fixture_path,
        "src/no_newline.txt",
        "first line\nlast line without newline",
    );
    write_fixture_file(fixture_path, "src/long_line.txt", &showcase_long_line("b"));
    write_fixture_file(
        fixture_path,
        "文档/说明.md",
        &showcase_markdown("第二行内容"),
    );
    write_fixture_file(fixture_path, "src/large.txt", &showcase_large_file(true));
}

fn showcase_modified_lines(modified: bool) -> String {
    let mut content = String::new();
    for line_number in 1..=SHOWCASE_MODIFIED_LINE_COUNT {
        if modified && line_number == SHOWCASE_REMOVED_LINE_NUMBER {
            continue;
        }
        let value = if modified && SHOWCASE_MODIFIED_LINE_NUMBERS.contains(&line_number) {
            format!("{line_number}0")
        } else {
            line_number.to_string()
        };
        content.push_str(&format!(
            "pub const STEP_{line_number:02}: u32 = {value};\n"
        ));
    }
    if modified {
        content.push_str(&format!(
            "pub const STEP_{SHOWCASE_REPLACEMENT_LINE_NUMBER:02}: u32 = {SHOWCASE_REPLACEMENT_LINE_NUMBER};\n"
        ));
    }
    content
}

fn showcase_untracked_lines() -> String {
    let mut content = String::new();
    for line_number in 1..=SHOWCASE_UNTRACKED_LINE_COUNT {
        content.push_str(&format!("pub fn extra_{line_number:02}() {{}}\n"));
    }
    content
}

fn showcase_long_line(letter: &str) -> String {
    format!("{}\n", letter.repeat(SHOWCASE_LONG_LINE_LENGTH))
}

fn showcase_markdown(last_line_content: &str) -> String {
    format!("# 说明\n\n{last_line_content}\n")
}

fn showcase_large_file(modified: bool) -> String {
    let mut content = String::new();
    for line_number in 1..=SHOWCASE_LARGE_FILE_LINE_COUNT {
        let changed = modified && line_number % SHOWCASE_LARGE_FILE_CHANGED_EVERY == 0;
        let suffix = if changed { " changed" } else { "" };
        content.push_str(&format!("row {line_number:04}{suffix}\n"));
    }
    content
}

fn populate_large_ten_thousand(fixture_path: &Path) {
    let started_at = Instant::now();
    run_fast_import(fixture_path, &large_fixture_fast_import_stream());
    let elapsed = started_at.elapsed();
    println!(
        "large-10k fixture generated in {:.2}s",
        elapsed.as_secs_f64()
    );
    assert!(
        elapsed < LARGE_FIXTURE_TIME_LIMIT,
        "large-10k fixture took {elapsed:?}, limit is {LARGE_FIXTURE_TIME_LIMIT:?}"
    );
    let commit_count = git_checked(fixture_path, 0, &["rev-list", "--count", "refs/heads/main"]);
    assert_eq!(
        commit_count.trim(),
        LARGE_FIXTURE_COMMIT_COUNT.to_string(),
        "large-10k fixture must contain exactly {LARGE_FIXTURE_COMMIT_COUNT} commits"
    );
}

fn run_fast_import(fixture_path: &Path, stream: &str) {
    let mut child = git_command(fixture_path, 0)
        .args(["fast-import", "--quiet", "--force"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("cannot start git fast-import: {error}"));
    {
        let mut stdin = child
            .stdin
            .take()
            .expect("git fast-import stdin must be piped");
        stdin
            .write_all(stream.as_bytes())
            .unwrap_or_else(|error| panic!("cannot write fast-import stream: {error}"));
    }
    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("cannot wait for git fast-import: {error}"));
    assert!(
        output.status.success(),
        "git fast-import failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[derive(Default)]
struct FastImportState {
    mark: u32,
    main_head: Option<u32>,
}

fn large_fixture_fast_import_stream() -> String {
    let mut stream = String::new();
    let block_count = LARGE_FIXTURE_COMMIT_COUNT / LARGE_FIXTURE_COMMITS_PER_BLOCK;
    let mut state = FastImportState::default();
    for block in 0..block_count {
        append_fast_import_block(&mut stream, block, &mut state);
    }
    stream.push_str("done\n");
    stream
}

fn append_fast_import_block(stream: &mut String, block: u32, state: &mut FastImportState) {
    let block_base = state.main_head;
    append_plain_main_commits(stream, LARGE_FIXTURE_COMMITS_PER_BLOCK - 2, state);
    let side_mark = append_side_commit(stream, block, block_base, state);
    append_merge_commit(stream, side_mark, state);
}

fn append_plain_main_commits(stream: &mut String, commit_count: u32, state: &mut FastImportState) {
    for _ in 0..commit_count {
        state.mark += 1;
        let mark = state.mark;
        append_fast_import_commit(
            stream,
            FastImportCommit {
                reference: String::from("refs/heads/main"),
                mark,
                first_parent: state.main_head,
                additional_parent: None,
                message: format!("commit {mark}"),
                file_path: String::from("tracked.txt"),
                file_content: format!("commit {mark}\n"),
            },
        );
        state.main_head = Some(mark);
    }
}

fn append_side_commit(
    stream: &mut String,
    block: u32,
    block_base: Option<u32>,
    state: &mut FastImportState,
) -> u32 {
    state.mark += 1;
    let side_mark = state.mark;
    append_fast_import_commit(
        stream,
        FastImportCommit {
            reference: format!("refs/heads/side-{block}"),
            mark: side_mark,
            first_parent: block_base,
            additional_parent: None,
            message: format!("side commit {side_mark}"),
            file_path: String::from("side.txt"),
            file_content: format!("side {side_mark}\n"),
        },
    );
    side_mark
}

fn append_merge_commit(stream: &mut String, side_mark: u32, state: &mut FastImportState) {
    state.mark += 1;
    let merge_mark = state.mark;
    append_fast_import_commit(
        stream,
        FastImportCommit {
            reference: String::from("refs/heads/main"),
            mark: merge_mark,
            first_parent: state.main_head,
            additional_parent: Some(side_mark),
            message: format!("merge {merge_mark}"),
            file_path: String::from("tracked.txt"),
            file_content: format!("merge {merge_mark}\n"),
        },
    );
    state.main_head = Some(merge_mark);
}

struct FastImportCommit {
    reference: String,
    mark: u32,
    first_parent: Option<u32>,
    additional_parent: Option<u32>,
    message: String,
    file_path: String,
    file_content: String,
}

fn append_fast_import_commit(stream: &mut String, commit: FastImportCommit) {
    stream.push_str(&format!("commit {}\n", commit.reference));
    stream.push_str(&format!("mark :{}\n", commit.mark));
    stream.push_str(&format!(
        "committer {FIXTURE_USER_NAME} <{FIXTURE_USER_EMAIL}> {} +0000\n",
        FIXTURE_BASE_EPOCH_SECONDS + u64::from(commit.mark) * 60
    ));
    stream.push_str(&fast_import_data(&format!("{}\n", commit.message)));
    if let Some(parent) = commit.first_parent {
        stream.push_str(&format!("from :{parent}\n"));
    }
    if let Some(parent) = commit.additional_parent {
        stream.push_str(&format!("merge :{parent}\n"));
    }
    stream.push_str(&format!("M 100644 inline {}\n", commit.file_path));
    stream.push_str(&fast_import_data(&commit.file_content));
}

fn fast_import_data(content: &str) -> String {
    format!("data {}\n{content}", content.len())
}

fn verify_fixture_head(fixture_path: &Path, name: FixtureRepo) {
    let output = run_git(fixture_path, 0, &["rev-parse", "--verify", "HEAD"]);
    if name == FixtureRepo::Empty {
        assert!(
            !output.status.success(),
            "empty fixture {} unexpectedly has a HEAD commit",
            fixture_path.display()
        );
        return;
    }
    assert!(
        output.status.success(),
        "fixture {} has no HEAD commit: {}",
        fixture_path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(head.len(), 40, "fixture HEAD is not a full commit hash");
}

#[cfg(test)]
mod fixture_tests {
    use std::fs;

    use super::{
        fixture_repo, fixtures_root, git_checked, FixtureRepo, FIXTURE_USER_EMAIL,
        FIXTURE_USER_NAME,
    };

    #[test]
    fn fixtures_are_generated_inside_the_repository_artifacts() {
        let fixture_path = fixture_repo(FixtureRepo::Empty);
        assert!(
            fixture_path.starts_with(super::artifacts_root()),
            "fixture {} escapes the artifacts root",
            fixture_path.display()
        );
        assert!(fixture_path.starts_with(fixtures_root()));
    }

    #[test]
    fn fixtures_carry_a_repository_local_commit_identity() {
        let fixture_path = fixture_repo(FixtureRepo::Dirty);
        assert_eq!(
            git_checked(&fixture_path, 0, &["config", "--local", "user.name"]).trim(),
            FIXTURE_USER_NAME
        );
        assert_eq!(
            git_checked(&fixture_path, 0, &["config", "--local", "user.email"]).trim(),
            FIXTURE_USER_EMAIL
        );
        assert_eq!(
            git_checked(&fixture_path, 0, &["config", "--local", "commit.gpgsign"]).trim(),
            "false"
        );
    }

    #[test]
    fn empty_fixture_has_a_git_directory_and_no_commits() {
        let fixture_path = fixture_repo(FixtureRepo::Empty);
        assert!(fixture_path.join(".git").is_dir());
        assert!(super::fixture_marker_path(&fixture_path).is_file());
        let commit_count = git_checked(&fixture_path, 0, &["rev-list", "--all", "--count"]);
        assert_eq!(commit_count.trim(), "0");
    }

    #[test]
    fn dirty_fixture_reports_staged_unstaged_and_untracked_changes() {
        let fixture_path = fixture_repo(FixtureRepo::Dirty);
        let status = git_checked(&fixture_path, 0, &["status", "--porcelain"]);
        assert!(
            status.contains("MM src/app.rs"),
            "staged plus unstaged modification missing: {status}"
        );
        assert!(
            status.contains("?? notes.txt"),
            "untracked file missing: {status}"
        );
        assert!(
            status.contains("A  src/module.rs"),
            "staged addition missing: {status}"
        );
        assert!(
            status.contains(" D docs/readme.md"),
            "unstaged deletion missing: {status}"
        );
    }

    #[test]
    fn merge_conflict_fixture_leaves_conflict_markers_in_the_worktree() {
        let fixture_path = fixture_repo(FixtureRepo::MergeConflict);
        let conflicted = fs::read_to_string(fixture_path.join("src/conflict.txt"))
            .expect("conflicted file is readable");
        assert!(conflicted.contains("<<<<<<< HEAD"));
        assert!(conflicted.contains("======="));
        assert!(conflicted.contains(">>>>>>> feature"));
    }

    #[test]
    fn rebase_diverged_fixture_has_two_branches_with_diverging_commits() {
        let fixture_path = fixture_repo(FixtureRepo::RebaseDiverged);
        let branches = git_checked(&fixture_path, 0, &["branch", "--format=%(refname:short)"]);
        assert!(branches.contains("main"), "main branch missing: {branches}");
        assert!(
            branches.contains("feature"),
            "feature branch missing: {branches}"
        );
        let merge_base = git_checked(&fixture_path, 0, &["merge-base", "main", "feature"]);
        let main_head = git_checked(&fixture_path, 0, &["rev-parse", "main"]);
        let feature_head = git_checked(&fixture_path, 0, &["rev-parse", "feature"]);
        assert_ne!(merge_base.trim(), main_head.trim());
        assert_ne!(merge_base.trim(), feature_head.trim());
    }

    #[test]
    fn cjk_paths_fixture_commits_wide_character_paths_and_messages() {
        let fixture_path = fixture_repo(FixtureRepo::CjkPaths);
        assert!(fixture_path.join("中文文件.txt").is_file());
        assert!(fixture_path.join("文档/说明.md").is_file());
        let tracked_files = git_checked(&fixture_path, 0, &["ls-files"]);
        assert!(tracked_files.contains("文档/说明.md"));
        assert!(tracked_files.contains("emoji-📦.txt"));
        let last_message = git_checked(&fixture_path, 0, &["log", "-1", "--pretty=%s"]);
        assert_eq!(last_message.trim(), "更新：说明文档");
    }

    #[test]
    fn fixture_repo_reuses_the_existing_repository_on_second_call() {
        let first_path = fixture_repo(FixtureRepo::Dirty);
        let first_head = git_checked(&first_path, 0, &["rev-parse", "HEAD"]);
        let first_marker = fs::read_to_string(super::fixture_marker_path(&first_path))
            .expect("first fixture marker is readable");
        let second_path = fixture_repo(FixtureRepo::Dirty);
        let second_head = git_checked(&second_path, 0, &["rev-parse", "HEAD"]);
        let second_marker = fs::read_to_string(super::fixture_marker_path(&second_path))
            .expect("second fixture marker is readable");
        assert_eq!(first_path, second_path);
        assert_eq!(first_head, second_head);
        assert_eq!(first_marker, second_marker);
    }

    #[test]
    fn large_ten_thousand_fixture_has_exactly_ten_thousand_commits() {
        let fixture_path = fixture_repo(FixtureRepo::LargeTenThousand);
        let commit_count = git_checked(
            &fixture_path,
            0,
            &["rev-list", "--count", "refs/heads/main"],
        );
        assert_eq!(commit_count.trim(), "10000");
        let merge_count = git_checked(
            &fixture_path,
            0,
            &["rev-list", "--merges", "--count", "refs/heads/main"],
        );
        assert_eq!(merge_count.trim(), "100");
    }

    #[test]
    fn diff_showcase_fixture_leaves_exactly_the_documented_worktree_changes() {
        let fixture_path = fixture_repo(FixtureRepo::DiffShowcase);
        let status = git_checked(&fixture_path, 0, &["status", "--porcelain=v1"]);
        let mut status_lines: Vec<&str> = status.lines().collect();
        status_lines.sort_unstable();
        assert_eq!(
            status_lines,
            vec![
                " M assets/logo.bin",
                " M src/large.txt",
                " M src/long_line.txt",
                " M src/modified.rs",
                " M src/no_newline.txt",
                " M 文档/说明.md",
                "?? src/untracked_new.rs",
            ],
            "diff-showcase worktree status"
        );
        let logo_bytes = fs::read(fixture_path.join("assets/logo.bin")).expect("logo is readable");
        assert!(
            logo_bytes.contains(&0),
            "the logo binary holds no null byte"
        );
        let long_line = fs::read_to_string(fixture_path.join("src/long_line.txt"))
            .expect("long line file is readable");
        assert_eq!(long_line.trim_end_matches('\n').chars().count(), 400);
        let no_newline = fs::read_to_string(fixture_path.join("src/no_newline.txt"))
            .expect("no newline file is readable");
        assert!(!no_newline.ends_with('\n'));
        let large_file =
            fs::read_to_string(fixture_path.join("src/large.txt")).expect("large file is readable");
        assert_eq!(large_file.lines().count(), 5_000);
        let modified = fs::read_to_string(fixture_path.join("src/modified.rs"))
            .expect("modified file is readable");
        assert_eq!(modified.lines().count(), 40);
        let untracked = fs::read_to_string(fixture_path.join("src/untracked_new.rs"))
            .expect("untracked file is readable");
        assert_eq!(untracked.lines().count(), 10);
    }
}
