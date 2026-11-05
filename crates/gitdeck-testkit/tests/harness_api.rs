use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use gitdeck_core::{InputEvent, KeyCode, PaneId};
use gitdeck_testkit::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use sha2::{Digest, Sha256};

const PROBE_WIDTH: u16 = 24;
const PROBE_HEIGHT: u16 = 4;
const PROBE_FOREGROUND: Color = Color::Rgb(0xCC, 0x78, 0x32);
const JB_DARK_BACKGROUND: [u8; 3] = [0x1E, 0x1F, 0x22];
const CELL_PIXEL_WIDTH: u32 = 16;
const CELL_PIXEL_HEIGHT: u32 = 32;
const ELLIPSIS_DOT_COLUMNS: [u32; 3] = [2, 7, 12];

static MANIFEST_VALIDATION_GUARD: Mutex<()> = Mutex::new(());

fn lock_manifest_validation() -> MutexGuard<'static, ()> {
    MANIFEST_VALIDATION_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct ProbeDriver {
    area: Rect,
    painted_line: String,
    focused: PaneId,
}

impl ProbeDriver {
    fn new(area: Rect, painted_line: &str) -> Self {
        Self {
            area,
            painted_line: painted_line.to_string(),
            focused: PaneId::Changes,
        }
    }
}

impl ScreenDriver for ProbeDriver {
    fn handle_event(&mut self, event: InputEvent) {
        let InputEvent::Key(key_input) = event else {
            return;
        };
        if key_input.code == KeyCode::Tab {
            self.focused = match self.focused {
                PaneId::Changes => PaneId::Log,
                PaneId::Log => PaneId::Details,
                PaneId::Details => PaneId::Changes,
            };
        }
    }

    fn render_frame(&self, area: Rect, buffer: &mut Buffer) {
        buffer.set_string(
            area.x,
            area.y,
            &self.painted_line,
            Style::default().fg(PROBE_FOREGROUND),
        );
    }

    fn focused_pane(&self) -> PaneId {
        self.focused
    }

    fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
        (pane == PaneId::Changes).then_some(self.area)
    }
}

fn probe_terminal(painted_line: &str) -> TestTerminal<ProbeDriver> {
    TestTerminal::new(
        ProbeDriver::new(Rect::new(0, 0, PROBE_WIDTH, PROBE_HEIGHT), painted_line),
        PROBE_WIDTH,
        PROBE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
    )
}

fn sha256_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn relative_to_repository(path: &Path) -> String {
    path.strip_prefix(repository_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn pixel_matches(pixels: &[u8], width: u32, column: u32, row: u32, color: Color) -> bool {
    let Color::Rgb(red, green, blue) = color else {
        return false;
    };
    let index = usize::try_from((row * width + column) * 3).unwrap_or(usize::MAX);
    pixels.get(index..index + 3) == Some(&[red, green, blue][..])
}

fn decode_rgb_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = fs::File::open(path).expect("screenshot png opens");
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().expect("screenshot png header");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .expect("screenshot png frame");
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

#[test]
fn ctrl_enter_is_encoded_as_the_csi_u_control_enter_sequence() {
    assert_eq!(
        gitdeck_testkit::key_token_bytes("ctrl+enter"),
        b"\x1b[13;5u".to_vec()
    );
    assert_eq!(
        gitdeck_testkit::key_token_bytes("CTRL+RETURN"),
        b"\x1b[13;5u".to_vec()
    );
    assert_eq!(gitdeck_testkit::key_token_bytes("ctrl+k"), vec![0x0b]);
}

#[test]
fn missing_glyph_is_reported_for_cjk() {
    let terminal = probe_terminal("修 a");
    let report = terminal.screenshot("harness-api/missing-glyph-cjk");
    assert_eq!(report.missing_glyphs, vec!['修']);
    terminal.assert_missing_glyphs(&['修']);
}

#[test]
fn ellipsis_glyph_renders_without_missing() {
    let terminal = probe_terminal("ab…");
    terminal.assert_no_missing_glyphs();
    let report = terminal.screenshot("harness-api/ellipsis-glyph");
    assert!(report.missing_glyphs.is_empty());
    assert_eq!(
        terminal.cell(2, 0).symbol,
        "…",
        "the glyph cell keeps the ellipsis symbol"
    );
    let (width, _, pixels) = decode_rgb_png(&report.png_path);
    let dot_row = CELL_PIXEL_HEIGHT / 2;
    let painted_dots = ELLIPSIS_DOT_COLUMNS
        .iter()
        .filter(|dot_column| {
            let column = 2 * CELL_PIXEL_WIDTH + *dot_column;
            pixel_matches(&pixels, width, column + 1, dot_row, PROBE_FOREGROUND)
        })
        .count();
    assert_eq!(painted_dots, 3, "three ellipsis dots are painted");
}

#[test]
fn assert_missing_glyphs_accepts_the_exact_set() {
    let terminal = probe_terminal("修 复");
    terminal.assert_missing_glyphs(&['修', '复']);
    terminal.assert_missing_glyphs(&['复', '修']);
}

#[test]
#[should_panic(expected = "missing glyphs differ")]
fn assert_missing_glyphs_requires_exact_set() {
    let terminal = probe_terminal("修 复");
    terminal.assert_missing_glyphs(&['修', '复', '中']);
}

#[test]
#[should_panic(expected = "expected foreground")]
fn assert_cell_style_reports_mismatch() {
    let terminal = probe_terminal("ok");
    terminal.assert_cell_style(0, 0, Color::Rgb(1, 2, 3), Color::Reset);
}

#[test]
fn assert_cell_style_matches_the_rendered_cell() {
    let terminal = probe_terminal("ok");
    terminal.assert_cell_style(0, 0, PROBE_FOREGROUND, Color::Reset);
}

#[test]
fn screenshot_rerenders_after_driver_mutation() {
    let _guard = lock_manifest_validation();
    let mut terminal = probe_terminal("█");
    let painted = terminal.screenshot("harness-api/rerender-target");
    terminal.driver_mut().painted_line = String::new();
    let cleared = terminal.screenshot("harness-api/rerender-target");
    assert_ne!(
        painted.bytes_sha256, cleared.bytes_sha256,
        "the png must follow the mutated driver, not the cached buffer"
    );
    let text = fs::read_to_string(&cleared.text_path).expect("screenshot text is readable");
    assert!(
        !text.contains('█'),
        "the text still holds the pre-mutation frame"
    );
    assert!(
        terminal.screen_text().contains('█'),
        "the cached screen must keep the pre-mutation frame until tick"
    );
    let (width, height, pixels) = decode_rgb_png(&cleared.png_path);
    assert_eq!(
        (width, height),
        (
            u32::from(PROBE_WIDTH) * CELL_PIXEL_WIDTH,
            u32::from(PROBE_HEIGHT) * CELL_PIXEL_HEIGHT
        )
    );
    assert!(
        pixels.chunks(3).all(|pixel| pixel == JB_DARK_BACKGROUND),
        "the png still holds the pre-mutation glyph pixels"
    );
}

#[test]
fn manifest_lists_every_written_screenshot_with_matching_sha() {
    let _guard = lock_manifest_validation();
    let terminal = probe_terminal("manifest");
    let report = terminal.screenshot("harness-api/manifest-entry");
    let manifest_path = repository_root().join("artifacts/screenshots/manifest.txt");
    let manifest = fs::read_to_string(&manifest_path).expect("screenshot manifest is readable");
    let expected_paths = [
        relative_to_repository(&report.png_path),
        relative_to_repository(&report.text_path),
        relative_to_repository(&report.ansi_path),
    ];
    let mut listed_entries = 0;
    for manifest_line in manifest.lines() {
        let (sha256, relative_path) = manifest_line
            .split_once("  ")
            .unwrap_or_else(|| panic!("manifest line {manifest_line:?} is not '<sha256>  <path>'"));
        assert_eq!(sha256.len(), 64, "manifest line {manifest_line:?}");
        let artifact_path = repository_root().join(relative_path);
        let artifact_bytes = fs::read(&artifact_path).unwrap_or_else(|error| {
            panic!("cannot read manifest artifact {relative_path}: {error}")
        });
        assert_eq!(
            sha256,
            sha256_of(&artifact_bytes),
            "manifest sha256 of {relative_path} does not match the file"
        );
        if expected_paths.contains(&relative_path.to_string()) {
            listed_entries += 1;
        }
    }
    assert_eq!(
        listed_entries,
        expected_paths.len(),
        "the manifest must hold the png, txt and ansi entries of this screenshot"
    );
    assert_eq!(
        report.bytes_sha256,
        sha256_of(&fs::read(&report.png_path).expect("screenshot png is readable"))
    );
}

#[test]
fn find_pane_matches_pane_rect() {
    let mut terminal = probe_terminal("pane");
    for pane in [PaneId::Changes, PaneId::Log, PaneId::Details] {
        assert_eq!(terminal.find_pane(pane), terminal.pane_rect(pane));
    }
    terminal.assert_focused(PaneId::Changes);
    terminal.send_keys("tab");
    terminal.assert_focused(PaneId::Log);
}

#[test]
fn for_fixture_builds_driver_with_repo_path() {
    let terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        PROBE_WIDTH,
        PROBE_HEIGHT,
        CapsProfile::Rich,
        ThemeId::JbDark,
        |repo_path| {
            assert!(
                repo_path.is_dir(),
                "fixture driver path {} is not a directory",
                repo_path.display()
            );
            assert!(
                repo_path.join(".git").is_dir(),
                "fixture driver path {} holds no .git directory",
                repo_path.display()
            );
            ProbeDriver::new(
                Rect::new(0, 0, PROBE_WIDTH, PROBE_HEIGHT),
                &relative_to_repository(repo_path),
            )
        },
    );
    assert!(terminal.driver().painted_line.contains("dirty"));
}

#[test]
fn collect_script_collects_fresh_manifest() {
    let scratch_root = prepare_gate_root("fresh");
    write_stamp(&scratch_root);
    let png_sha256 = write_artifact(
        &scratch_root,
        "artifacts/screenshots/suite/fresh.png",
        b"png",
    );
    let txt_sha256 = write_artifact(
        &scratch_root,
        "artifacts/screenshots/suite/fresh.txt",
        b"txt",
    );
    write_manifest(
        &scratch_root,
        &[
            (
                png_sha256,
                String::from("artifacts/screenshots/suite/fresh.png"),
            ),
            (
                txt_sha256,
                String::from("artifacts/screenshots/suite/fresh.txt"),
            ),
        ],
    );
    let output = run_collect_script(&scratch_root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the fresh manifest must pass: {stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("collected 1 screenshot(s)"), "{stdout}");
    assert!(
        stdout.contains("artifacts/screenshots/suite/fresh.png"),
        "{stdout}"
    );
}

#[test]
fn collect_script_fails_on_stale_or_missing_png() {
    assert_stale_artifact_is_rejected();
    assert_missing_artifact_is_rejected();
    assert_unlisted_screenshot_is_rejected();
}

fn assert_stale_artifact_is_rejected() {
    let scratch_root = prepare_gate_root("stale");
    let artifact_relative_path = "artifacts/screenshots/suite/stale.png";
    let artifact_sha256 = write_artifact(&scratch_root, artifact_relative_path, b"stale");
    backdate_artifact(&scratch_root.join(artifact_relative_path));
    write_stamp(&scratch_root);
    write_manifest(
        &scratch_root,
        &[(artifact_sha256, String::from(artifact_relative_path))],
    );
    assert_gate_fails(&scratch_root, "is stale");
}

fn assert_missing_artifact_is_rejected() {
    let scratch_root = prepare_gate_root("missing");
    write_stamp(&scratch_root);
    write_manifest(
        &scratch_root,
        &[(
            sha256_of(b"absent"),
            String::from("artifacts/screenshots/suite/absent.png"),
        )],
    );
    assert_gate_fails(&scratch_root, "is missing");
}

fn assert_unlisted_screenshot_is_rejected() {
    let scratch_root = prepare_gate_root("unlisted");
    write_stamp(&scratch_root);
    let listed_relative_path = "artifacts/screenshots/suite/listed.png";
    let listed_sha256 = write_artifact(&scratch_root, listed_relative_path, b"listed");
    write_artifact(
        &scratch_root,
        "artifacts/screenshots/suite/unlisted.png",
        b"unlisted",
    );
    write_manifest(
        &scratch_root,
        &[(listed_sha256, String::from(listed_relative_path))],
    );
    assert_gate_fails(&scratch_root, "is missing from the manifest");
}

fn prepare_gate_root(case: &str) -> PathBuf {
    let scratch_root = repository_root()
        .join(".tmp")
        .join(format!("collect-gate-{case}"));
    if scratch_root.exists() {
        fs::remove_dir_all(&scratch_root).expect("stale gate scratch directory is removable");
    }
    fs::create_dir_all(scratch_root.join("scripts")).expect("gate scripts directory");
    fs::create_dir_all(scratch_root.join("artifacts/screenshots")).expect("gate screenshot root");
    fs::create_dir_all(scratch_root.join(".tmp")).expect("gate stamp directory");
    fs::copy(
        repository_root().join("scripts/collect-screenshots.sh"),
        scratch_root.join("scripts/collect-screenshots.sh"),
    )
    .expect("collect script is copyable");
    scratch_root
}

fn write_stamp(scratch_root: &Path) {
    fs::write(scratch_root.join(".tmp/suite-run-stamp"), b"").expect("gate stamp is writable");
}

fn write_artifact(scratch_root: &Path, relative_path: &str, content: &[u8]) -> String {
    let artifact_path = scratch_root.join(relative_path);
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).expect("gate artifact directory");
    }
    fs::write(&artifact_path, content).expect("gate artifact is writable");
    sha256_of(content)
}

fn backdate_artifact(artifact_path: &Path) {
    let artifact = fs::File::options()
        .write(true)
        .open(artifact_path)
        .expect("gate artifact is openable");
    artifact
        .set_modified(SystemTime::now() - Duration::from_secs(600))
        .expect("gate artifact mtime is settable");
}

fn write_manifest(scratch_root: &Path, entries: &[(String, String)]) {
    let manifest: String = entries
        .iter()
        .map(|(sha256, relative_path)| format!("{sha256}  {relative_path}\n"))
        .collect();
    fs::write(
        scratch_root.join("artifacts/screenshots/manifest.txt"),
        manifest,
    )
    .expect("gate manifest is writable");
}

fn run_collect_script(scratch_root: &Path) -> Output {
    Command::new("bash")
        .arg(scratch_root.join("scripts/collect-screenshots.sh"))
        .output()
        .expect("collect script runs")
}

fn assert_gate_fails(scratch_root: &Path, expected_reason: &str) {
    let output = run_collect_script(scratch_root);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the gate must reject {}: {stdout}{stderr}",
        scratch_root.display()
    );
    assert!(
        stdout.contains(expected_reason),
        "the gate output {stdout} misses {expected_reason:?}"
    );
    assert!(
        stdout.contains("screenshot gate failed"),
        "the gate output {stdout} misses the failure summary"
    );
}
