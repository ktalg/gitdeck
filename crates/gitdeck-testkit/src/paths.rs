use std::path::{Component, Path, PathBuf};

const WORKSPACE_MARKER_DIRECTORY: &str = "crates";
const WORKSPACE_MARKER_FILE: &str = "Cargo.toml";
const SOURCE_DIRECTORY_NAMES: [&str; 2] = ["src", "tests"];

pub fn repository_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .find(|candidate| {
            candidate.join(WORKSPACE_MARKER_FILE).is_file()
                && candidate.join(WORKSPACE_MARKER_DIRECTORY).is_dir()
        })
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            panic!(
                "no workspace root holding {} and a {} directory above {}",
                WORKSPACE_MARKER_FILE,
                WORKSPACE_MARKER_DIRECTORY,
                manifest_dir.display()
            )
        })
}

pub(crate) fn artifacts_root() -> PathBuf {
    repository_root().join("artifacts")
}

pub(crate) fn screenshots_root() -> PathBuf {
    artifacts_root().join("screenshots")
}

pub(crate) fn run_stamp_path() -> PathBuf {
    repository_root().join(".tmp").join("suite-run-stamp")
}

pub(crate) fn golden_root() -> PathBuf {
    repository_root().join("crates/gitdeck-suite/tests/suite/golden")
}

pub(crate) fn repository_relative_path(path: &Path) -> PathBuf {
    let root = repository_root();
    path.strip_prefix(&root).unwrap_or(path).to_path_buf()
}

pub(crate) fn module_path_from_source_file(source_file: &str) -> String {
    let mut segments: Vec<String> = Path::new(source_file)
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();
    if let Some(index) = segments
        .iter()
        .position(|segment| SOURCE_DIRECTORY_NAMES.contains(&segment.as_str()))
    {
        segments.drain(..=index);
    }
    if let Some(last_segment) = segments.last_mut() {
        let stem = Path::new(last_segment.as_str())
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| last_segment.clone());
        *last_segment = stem;
    }
    segments.join("__")
}

#[cfg(test)]
mod path_tests {
    use std::path::PathBuf;

    use super::{module_path_from_source_file, repository_relative_path, repository_root};

    #[test]
    fn repository_root_holds_the_workspace_manifest_and_the_crates_directory() {
        let root = repository_root();
        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("crates").is_dir());
        assert!(root.join("crates/gitdeck-testkit/Cargo.toml").is_file());
    }

    #[test]
    fn repository_relative_path_strips_the_workspace_root() {
        let root = repository_root();
        assert_eq!(
            repository_relative_path(&root.join("artifacts/screenshots/suite/step.png")),
            PathBuf::from("artifacts/screenshots/suite/step.png")
        );
    }

    #[test]
    fn module_path_from_source_file_joins_the_segments_after_tests() {
        assert_eq!(
            module_path_from_source_file("crates/gitdeck-suite/tests/suite/harness_smoke.rs"),
            "suite__harness_smoke"
        );
        assert_eq!(
            module_path_from_source_file("crates/gitdeck-testkit/tests/harness_api.rs"),
            "harness_api"
        );
        assert_eq!(
            module_path_from_source_file("crates/gitdeck-testkit/src/terminal.rs"),
            "terminal"
        );
    }
}
