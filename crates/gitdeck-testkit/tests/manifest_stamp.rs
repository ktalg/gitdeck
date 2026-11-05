use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use gitdeck_testkit::prelude::*;
use sha2::{Digest, Sha256};

const SUITE_RUN_STAMP: &str = "suite-run-stamp";
const SCREENSHOTS_DIRECTORY: &str = "artifacts/screenshots";
const OWNER_FILE_PREFIX: &str = "manifest-owner-";

#[test]
fn manifest_is_truncated_once_per_run_stamp() {
    let scratch_root = prepare_scratch_root("truncated-once");
    let stamp_path = write_stamp(&scratch_root, Duration::from_secs(120));
    let first_artifact = write_artifact(&scratch_root, "suite/first.png", b"first");
    let second_artifact = write_artifact(&scratch_root, "suite/second.png", b"second");
    let screenshots_root = scratch_root.join(SCREENSHOTS_DIRECTORY);
    let manifest = ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path);
    fs::write(manifest.manifest_path(), stale_manifest_line()).expect("stale manifest is writable");

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&first_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![expected_line(&first_artifact)],
        "the first writer of a run stamp must drop the previous manifest content"
    );

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&second_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![
            expected_line(&first_artifact),
            expected_line(&second_artifact)
        ],
        "a later writer of the same run stamp must append instead of truncating"
    );

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&first_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![
            expected_line(&first_artifact),
            expected_line(&second_artifact),
        ],
        "recording an artifact twice must keep a single line for it"
    );
}

#[test]
fn manifest_upserts_duplicate_paths_with_the_latest_sha() {
    let scratch_root = prepare_scratch_root("upsert");
    let stamp_path = write_stamp(&scratch_root, Duration::from_secs(120));
    let screenshots_root = scratch_root.join(SCREENSHOTS_DIRECTORY);
    let manifest = ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path);
    let earlier_artifact = write_artifact(&scratch_root, "suite/upsert.png", b"earlier");
    let earlier_line = expected_line(&earlier_artifact);
    let later_artifact = write_artifact(&scratch_root, "suite/upsert.png", b"later");
    let later_line = expected_line(&later_artifact);
    assert_ne!(
        earlier_line, later_line,
        "the two renders must differ for the upsert to be observable"
    );

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path)
        .record_artifact(&earlier_artifact);
    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&later_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![later_line],
        "one path must keep exactly one line, holding the sha of the latest render"
    );
}

#[test]
fn manifest_lines_are_sorted_by_path() {
    let scratch_root = prepare_scratch_root("sorted");
    let stamp_path = write_stamp(&scratch_root, Duration::from_secs(120));
    let screenshots_root = scratch_root.join(SCREENSHOTS_DIRECTORY);
    let manifest = ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path);
    let zulu_artifact = write_artifact(&scratch_root, "suite/zulu.png", b"zulu");
    let alpha_artifact = write_artifact(&scratch_root, "suite/alpha.png", b"alpha");
    let mike_artifact = write_artifact(&scratch_root, "suite/mike.png", b"mike");

    for artifact_path in [&zulu_artifact, &alpha_artifact, &mike_artifact] {
        ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path)
            .record_artifact(artifact_path);
    }
    let recorded_paths: Vec<String> = manifest_lines(&manifest)
        .iter()
        .map(|line| line.split_once("  ").expect("manifest line").1.to_string())
        .collect();
    assert_eq!(
        recorded_paths,
        vec![
            relative_artifact_path(&alpha_artifact),
            relative_artifact_path(&mike_artifact),
            relative_artifact_path(&zulu_artifact),
        ],
        "the manifest must list its artifacts in path order"
    );
}

#[test]
fn manifest_is_rebuilt_when_the_stamp_changes() {
    let scratch_root = prepare_scratch_root("stamp-changes");
    let stamp_path = write_stamp(&scratch_root, Duration::from_secs(120));
    let first_artifact = write_artifact(&scratch_root, "suite/earlier.png", b"earlier");
    let second_artifact = write_artifact(&scratch_root, "suite/later.png", b"later");
    let screenshots_root = scratch_root.join(SCREENSHOTS_DIRECTORY);
    let manifest = ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path);

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&first_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![expected_line(&first_artifact)]
    );

    write_stamp(&scratch_root, Duration::from_secs(10));
    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&second_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![expected_line(&second_artifact)],
        "a new run stamp must rebuild the manifest from the new run only"
    );
}

#[test]
fn manifest_appends_without_a_stamp() {
    let scratch_root = prepare_scratch_root("without-stamp");
    let stamp_path = scratch_root.join(".tmp").join(SUITE_RUN_STAMP);
    assert!(!stamp_path.exists(), "the scratch root must hold no stamp");
    let first_artifact = write_artifact(&scratch_root, "suite/no-stamp-first.png", b"first");
    let second_artifact = write_artifact(&scratch_root, "suite/no-stamp-second.png", b"second");
    let screenshots_root = scratch_root.join(SCREENSHOTS_DIRECTORY);
    let manifest = ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path);

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&first_artifact);
    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&second_artifact);
    assert_eq!(
        manifest_lines(&manifest),
        vec![
            expected_line(&first_artifact),
            expected_line(&second_artifact)
        ],
        "without a run stamp every writer must append to the manifest"
    );
}

#[test]
fn manifest_owner_file_is_per_stamp() {
    let scratch_root = prepare_scratch_root("owner-file");
    let stamp_path = write_stamp(&scratch_root, Duration::from_secs(120));
    let artifact_path = write_artifact(&scratch_root, "suite/owner.png", b"owner");
    let screenshots_root = scratch_root.join(SCREENSHOTS_DIRECTORY);
    let earlier_owner_file_name =
        ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).owner_file_name();
    let repeated_owner_file_name =
        ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).owner_file_name();

    assert_eq!(
        earlier_owner_file_name, repeated_owner_file_name,
        "one run stamp must map to exactly one owner file name"
    );
    assert!(
        earlier_owner_file_name.starts_with(OWNER_FILE_PREFIX),
        "owner file name {earlier_owner_file_name:?} misses the {OWNER_FILE_PREFIX:?} prefix"
    );
    assert_eq!(
        earlier_owner_file_name.len(),
        OWNER_FILE_PREFIX.len() + 16,
        "owner file name {earlier_owner_file_name:?} must carry 16 identity digits"
    );

    ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).record_artifact(&artifact_path);
    assert!(
        scratch_root
            .join(".tmp")
            .join(&earlier_owner_file_name)
            .is_file(),
        "the owner file must sit next to the suite run stamp"
    );

    write_stamp(&scratch_root, Duration::from_secs(10));
    let later_owner_file_name =
        ScreenshotManifest::rooted_at(&screenshots_root, &stamp_path).owner_file_name();
    assert_ne!(
        earlier_owner_file_name, later_owner_file_name,
        "a changed run stamp must claim a different owner file name"
    );
}

fn prepare_scratch_root(case: &str) -> PathBuf {
    let scratch_root = repository_root()
        .join(".tmp")
        .join(format!("manifest-stamp-{case}"));
    if scratch_root.exists() {
        fs::remove_dir_all(&scratch_root).expect("stale manifest scratch directory is removable");
    }
    fs::create_dir_all(scratch_root.join(".tmp")).expect("manifest scratch stamp directory");
    fs::create_dir_all(scratch_root.join(SCREENSHOTS_DIRECTORY))
        .expect("manifest scratch screenshot directory");
    scratch_root
}

fn write_stamp(scratch_root: &Path, age: Duration) -> PathBuf {
    let stamp_path = scratch_root.join(".tmp").join(SUITE_RUN_STAMP);
    fs::write(&stamp_path, b"").expect("manifest scratch stamp is writable");
    let stamp = fs::File::options()
        .write(true)
        .open(&stamp_path)
        .expect("manifest scratch stamp is openable");
    stamp
        .set_modified(SystemTime::now() - age)
        .expect("manifest scratch stamp mtime is settable");
    stamp_path
}

fn write_artifact(scratch_root: &Path, relative_path: &str, content: &[u8]) -> PathBuf {
    let artifact_path = scratch_root.join(SCREENSHOTS_DIRECTORY).join(relative_path);
    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent).expect("manifest scratch artifact directory");
    }
    fs::write(&artifact_path, content).expect("manifest scratch artifact is writable");
    artifact_path
}

fn expected_line(artifact_path: &Path) -> String {
    let sha256 = Sha256::digest(fs::read(artifact_path).expect("recorded artifact is readable"))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{sha256}  {}", relative_artifact_path(artifact_path))
}

fn relative_artifact_path(artifact_path: &Path) -> String {
    artifact_path
        .strip_prefix(repository_root())
        .unwrap_or(artifact_path)
        .display()
        .to_string()
}

fn stale_manifest_line() -> String {
    format!(
        "{}  {SCREENSHOTS_DIRECTORY}/suite/stale.png\n",
        "0".repeat(64)
    )
}

fn manifest_lines(manifest: &ScreenshotManifest) -> Vec<String> {
    fs::read_to_string(manifest.manifest_path())
        .expect("screenshot manifest is readable")
        .lines()
        .map(str::to_string)
        .collect()
}
