use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::paths::repository_root;

pub fn scratch_directory(parent: &str, name: &str) -> PathBuf {
    let path = repository_root().join(parent).join(name);
    if path.exists() {
        fs::remove_dir_all(&path).unwrap_or_else(|error| {
            panic!("cannot remove stale scratch {}: {error}", path.display())
        });
    }
    fs::create_dir_all(&path)
        .unwrap_or_else(|error| panic!("cannot create scratch {}: {error}", path.display()));
    path
}

pub fn copy_directory(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod scratch_tests {
    use std::fs;

    use super::{copy_directory, scratch_directory};

    #[test]
    fn scratch_directory_recreates_an_empty_repository_slot() {
        let path = scratch_directory("artifacts/tmp/testkit", "recreated-slot");
        fs::write(path.join("stale.txt"), "stale").expect("stale file is written");
        let recreated = scratch_directory("artifacts/tmp/testkit", "recreated-slot");
        assert_eq!(path, recreated);
        assert!(recreated.is_dir());
        assert!(!recreated.join("stale.txt").exists());
    }

    #[test]
    fn copy_directory_reproduces_nested_file_content() {
        let source = scratch_directory("artifacts/tmp/testkit", "copy-source");
        let target = scratch_directory("artifacts/tmp/testkit", "copy-target");
        fs::create_dir_all(source.join("nested")).expect("nested source directory is created");
        fs::write(source.join("nested/file.txt"), "content").expect("source file is written");
        copy_directory(&source, &target).expect("the directory copies");
        assert_eq!(
            fs::read_to_string(target.join("nested/file.txt")).expect("copied file is readable"),
            "content"
        );
    }
}
