use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use crate::command::{
    run_git_command, run_write_git_command_with_index, run_write_git_command_with_index_and_input,
};
use crate::decoding::decode_line;
use crate::error::GitError;

const GIT_DIRECTORY_ARGUMENTS: [&str; 2] = ["rev-parse", "--absolute-git-dir"];
const READ_TREE_EMPTY_ARGUMENTS: [&str; 2] = ["read-tree", "--empty"];
const RESET_PATH_ARGUMENTS: [&str; 3] = ["reset", "-q", "--"];
const ADD_ARGUMENT_PREFIX: [&str; 2] = ["add", "--"];
const APPLY_PATCH_ARGUMENTS: [&str; 4] = ["apply", "--cached", "--whitespace=nowarn", "-"];
const COMMIT_ARGUMENT_PREFIX: [&str; 3] = ["commit", "-F", "-"];
const COMMIT_AMEND_ARGUMENT: &str = "--amend";
const COMMIT_SIGN_OFF_ARGUMENT: &str = "--signoff";
const INDEX_FILE_NAME: &str = "index";
const INDEX_FILE_NAME_PREFIX: &str = "gitdeck-commit-index-";

pub(crate) struct CommitArguments<'a> {
    pub(crate) message: &'a str,
    pub(crate) amend: bool,
    pub(crate) sign_off: bool,
}

pub(crate) struct TemporaryIndex {
    path: PathBuf,
}

impl TemporaryIndex {
    pub(crate) fn create(repository: &Path) -> Result<Self, GitError> {
        let path =
            git_directory(repository)?.join(format!("{INDEX_FILE_NAME_PREFIX}{}", process::id()));
        if path.exists() {
            fs::remove_file(&path).map_err(|source| GitError::TemporaryIndexFile {
                path: path.clone(),
                source,
            })?;
        }
        Ok(Self { path })
    }

    pub(crate) fn seed_from_real_index(&self, repository: &Path) -> Result<(), GitError> {
        let real_index_path = git_directory(repository)?.join(INDEX_FILE_NAME);
        if !real_index_path.exists() {
            return self.read_empty_tree(repository);
        }
        fs::copy(&real_index_path, &self.path)
            .map(|_| ())
            .map_err(|source| GitError::TemporaryIndexFile {
                path: self.path.clone(),
                source,
            })
    }

    pub(crate) fn reset_path(&self, repository: &Path, path: &str) -> Result<(), GitError> {
        let mut arguments = RESET_PATH_ARGUMENTS.to_vec();
        arguments.push(path);
        run_write_git_command_with_index(repository, &self.path, &arguments).map(|_| ())
    }

    pub(crate) fn read_empty_tree(&self, repository: &Path) -> Result<(), GitError> {
        run_write_git_command_with_index(repository, &self.path, &READ_TREE_EMPTY_ARGUMENTS)
            .map(|_| ())
    }

    pub(crate) fn add_path(&self, repository: &Path, path: &str) -> Result<(), GitError> {
        let mut arguments = ADD_ARGUMENT_PREFIX.to_vec();
        arguments.push(path);
        run_write_git_command_with_index(repository, &self.path, &arguments).map(|_| ())
    }

    pub(crate) fn apply_patch(&self, repository: &Path, patch: &[u8]) -> Result<(), GitError> {
        run_write_git_command_with_index_and_input(
            repository,
            &self.path,
            &APPLY_PATCH_ARGUMENTS,
            patch,
        )
        .map(|_| ())
    }

    pub(crate) fn commit(
        &self,
        repository: &Path,
        request: CommitArguments<'_>,
    ) -> Result<(), GitError> {
        let mut arguments = COMMIT_ARGUMENT_PREFIX.to_vec();
        if request.amend {
            arguments.push(COMMIT_AMEND_ARGUMENT);
        }
        if request.sign_off {
            arguments.push(COMMIT_SIGN_OFF_ARGUMENT);
        }
        run_write_git_command_with_index_and_input(
            repository,
            &self.path,
            &arguments,
            request.message.as_bytes(),
        )
        .map(|_| ())
    }

    pub(crate) fn remove_file(&self) -> Result<(), GitError> {
        fs::remove_file(&self.path).map_err(|source| GitError::TemporaryIndexFile {
            path: self.path.clone(),
            source,
        })
    }
}

fn git_directory(repository: &Path) -> Result<PathBuf, GitError> {
    let git_directory = run_git_command(repository, &GIT_DIRECTORY_ARGUMENTS)?;
    Ok(PathBuf::from(decode_line(&git_directory)))
}

impl Drop for TemporaryIndex {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
