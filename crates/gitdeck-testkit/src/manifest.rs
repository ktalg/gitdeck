use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::hashing::{sha256_hex, sha256_hex_of_file};
use crate::paths::{repository_relative_path, run_stamp_path, screenshots_root};

const MANIFEST_FILE_NAME: &str = "manifest.txt";
const MANIFEST_TEMPORARY_EXTENSION: &str = "txt.tmp";
const LOCK_FILE_NAME: &str = "manifest-lock";
const OWNER_FILE_PREFIX: &str = "manifest-owner-";
const OWNER_IDENTITY_DIGITS: usize = 16;
const MISSING_STAMP_IDENTITY: &str = "no-stamp";
const LOCK_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(5);

pub struct ScreenshotManifest {
    screenshots_root: PathBuf,
    run_stamp_path: PathBuf,
}

impl ScreenshotManifest {
    pub fn for_repository() -> Self {
        Self::rooted_at(&screenshots_root(), &run_stamp_path())
    }

    pub fn rooted_at(screenshots_root: &Path, run_stamp_path: &Path) -> Self {
        Self {
            screenshots_root: screenshots_root.to_path_buf(),
            run_stamp_path: run_stamp_path.to_path_buf(),
        }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.screenshots_root.join(MANIFEST_FILE_NAME)
    }

    pub fn owner_file_name(&self) -> String {
        let identity = self.run_stamp_identity();
        format!("{OWNER_FILE_PREFIX}{}", &identity[..OWNER_IDENTITY_DIGITS])
    }

    pub fn record_artifact(&self, artifact_path: &Path) {
        let relative_path = repository_relative_path(artifact_path)
            .display()
            .to_string();
        let sha256 = sha256_hex_of_file(artifact_path);
        let manifest_path = self.manifest_path();
        let _lock = self.lock_run_directory();
        let mut entries = if self.claim_first_writer_marker() {
            BTreeMap::new()
        } else {
            read_manifest_entries(&manifest_path)
        };
        entries.insert(relative_path, sha256);
        write_manifest_entries(&manifest_path, &entries);
    }

    fn run_directory(&self) -> PathBuf {
        self.run_stamp_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.screenshots_root.clone())
    }

    fn owner_file_path(&self) -> PathBuf {
        self.run_directory().join(self.owner_file_name())
    }

    fn claim_first_writer_marker(&self) -> bool {
        let owner_file_path = self.owner_file_path();
        if let Some(parent) = owner_file_path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "cannot create the screenshot manifest owner directory {}: {error}",
                    parent.display()
                )
            });
        }
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&owner_file_path)
        {
            Ok(_owner_marker) => true,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => false,
            Err(error) => panic!(
                "cannot create the screenshot manifest owner marker {}: {error}",
                owner_file_path.display()
            ),
        }
    }

    fn lock_run_directory(&self) -> File {
        let lock_path = self.run_directory().join(LOCK_FILE_NAME);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "cannot create the screenshot manifest lock directory {}: {error}",
                    parent.display()
                )
            });
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .unwrap_or_else(|error| {
                panic!(
                    "cannot open the screenshot manifest lock {}: {error}",
                    lock_path.display()
                )
            });
        let deadline = Instant::now() + LOCK_WAIT_TIMEOUT;
        loop {
            match lock.try_lock() {
                Ok(()) => return lock,
                Err(TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        panic!(
                            "cannot lock {} within {LOCK_WAIT_TIMEOUT:?}: another writer keeps it busy",
                            lock_path.display()
                        );
                    }
                    thread::sleep(LOCK_RETRY_INTERVAL);
                }
                Err(TryLockError::Error(error)) => {
                    panic!("cannot lock {}: {error}", lock_path.display())
                }
            }
        }
    }

    fn run_stamp_identity(&self) -> String {
        let run_stamp = match fs::metadata(&self.run_stamp_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return sha256_hex(MISSING_STAMP_IDENTITY.as_bytes())
            }
            Err(error) => panic!(
                "cannot read the suite run stamp {}: {error}",
                self.run_stamp_path.display()
            ),
        };
        let modified_nanos = run_stamp
            .modified()
            .unwrap_or_else(|error| {
                panic!(
                    "the suite run stamp {} holds no modification time: {error}",
                    self.run_stamp_path.display()
                )
            })
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|error| {
                panic!(
                    "the suite run stamp {} predates the unix epoch: {error}",
                    self.run_stamp_path.display()
                )
            })
            .as_nanos();
        sha256_hex(format!("{modified_nanos}-{}", run_stamp.len()).as_bytes())
    }
}

fn read_manifest_entries(manifest_path: &Path) -> BTreeMap<String, String> {
    let content = match fs::read_to_string(manifest_path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return BTreeMap::new(),
        Err(error) => panic!(
            "cannot read the screenshot manifest {}: {error}",
            manifest_path.display()
        ),
    };
    content
        .lines()
        .filter(|line| !line.is_empty())
        .map(parse_manifest_entry)
        .collect()
}

fn parse_manifest_entry(manifest_line: &str) -> (String, String) {
    let (sha256, relative_path) = manifest_line
        .split_once("  ")
        .unwrap_or_else(|| panic!("manifest line {manifest_line:?} is not '<sha256>  <path>'"));
    (relative_path.to_string(), sha256.to_string())
}

fn write_manifest_entries(manifest_path: &Path, entries: &BTreeMap<String, String>) {
    let content: String = entries
        .iter()
        .map(|(relative_path, sha256)| format!("{sha256}  {relative_path}\n"))
        .collect();
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "cannot create the screenshot directory {}: {error}",
                parent.display()
            )
        });
    }
    let temporary_path = manifest_path.with_extension(MANIFEST_TEMPORARY_EXTENSION);
    fs::write(&temporary_path, content).unwrap_or_else(|error| {
        panic!(
            "cannot write the screenshot manifest {}: {error}",
            temporary_path.display()
        )
    });
    fs::rename(&temporary_path, manifest_path).unwrap_or_else(|error| {
        panic!(
            "cannot move the screenshot manifest {} onto {}: {error}",
            temporary_path.display(),
            manifest_path.display()
        )
    });
}

pub(crate) fn record_screenshot_artifact(artifact_path: &Path) {
    ScreenshotManifest::for_repository().record_artifact(artifact_path);
}
