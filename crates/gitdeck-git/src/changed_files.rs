use gitdeck_core::ChangeKind;

use crate::decoding::decode_text;
use crate::error::GitError;

const NAME_STATUS_CONTEXT: &str = "show --name-status";
const FIELD_TERMINATOR: u8 = 0;
const RENAME_MARKER: u8 = b'R';
const COPY_MARKER: u8 = b'C';

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedFile {
    pub path: String,
    pub previous_path: Option<String>,
    pub kind: ChangeKind,
}

pub(crate) fn parse_name_status(output: &[u8]) -> Result<Vec<ChangedFile>, GitError> {
    let mut fields = output
        .split(|byte| *byte == FIELD_TERMINATOR)
        .filter(|field| !field.is_empty());
    let mut changed_files = Vec::new();
    while let Some(status_field) = fields.next() {
        let status = status_field[0];
        if status == RENAME_MARKER || status == COPY_MARKER {
            let previous = take_path_field(&mut fields, status)?;
            let path = take_path_field(&mut fields, status)?;
            changed_files.push(ChangedFile {
                path,
                previous_path: (status == RENAME_MARKER).then_some(previous),
                kind: change_kind_of_status(status),
            });
            continue;
        }
        let path = take_path_field(&mut fields, status)?;
        changed_files.push(ChangedFile {
            path,
            previous_path: None,
            kind: change_kind_of_status(status),
        });
    }
    Ok(changed_files)
}

fn take_path_field<'a>(
    fields: &mut impl Iterator<Item = &'a [u8]>,
    status: u8,
) -> Result<String, GitError> {
    fields
        .next()
        .map(decode_text)
        .ok_or_else(|| GitError::UnexpectedOutput {
            context: NAME_STATUS_CONTEXT,
            message: format!("status {} holds no path field", char::from(status)),
        })
}

fn change_kind_of_status(status: u8) -> ChangeKind {
    match status {
        b'A' | COPY_MARKER => ChangeKind::Added,
        b'D' => ChangeKind::Deleted,
        RENAME_MARKER => ChangeKind::Renamed,
        b'U' => ChangeKind::Conflicted,
        _ => ChangeKind::Modified,
    }
}

#[cfg(test)]
mod name_status_tests {
    use gitdeck_core::ChangeKind;

    use super::{parse_name_status, ChangedFile};

    fn changed_file(path: &str, previous_path: Option<&str>, kind: ChangeKind) -> ChangedFile {
        ChangedFile {
            path: path.to_string(),
            previous_path: previous_path.map(str::to_string),
            kind,
        }
    }

    #[test]
    fn plain_status_letters_map_to_change_kinds() {
        let output = b"A\0added.rs\0M\0modified.rs\0D\0deleted.rs\0T\0type-changed.rs\0";
        assert_eq!(
            parse_name_status(output).expect("plain statuses parse"),
            vec![
                changed_file("added.rs", None, ChangeKind::Added),
                changed_file("modified.rs", None, ChangeKind::Modified),
                changed_file("deleted.rs", None, ChangeKind::Deleted),
                changed_file("type-changed.rs", None, ChangeKind::Modified),
            ]
        );
    }

    #[test]
    fn rename_status_carries_the_previous_path() {
        let output = b"R100\0src/old.rs\0src/new.rs\0";
        assert_eq!(
            parse_name_status(output).expect("rename status parses"),
            vec![changed_file(
                "src/new.rs",
                Some("src/old.rs"),
                ChangeKind::Renamed
            )]
        );
    }

    #[test]
    fn copy_status_counts_as_an_addition_without_a_previous_path() {
        let output = b"C75\0src/source.rs\0src/copy.rs\0";
        assert_eq!(
            parse_name_status(output).expect("copy status parses"),
            vec![changed_file("src/copy.rs", None, ChangeKind::Added)]
        );
    }

    #[test]
    fn combined_merge_status_uses_its_first_letter() {
        let output = b"MM\0src/merged.rs\0";
        assert_eq!(
            parse_name_status(output).expect("combined status parses"),
            vec![changed_file("src/merged.rs", None, ChangeKind::Modified)]
        );
    }

    #[test]
    fn wide_character_paths_are_decoded_verbatim() {
        let output = "A\0文档/说明.md\0".as_bytes();
        assert_eq!(
            parse_name_status(output).expect("wide character path parses"),
            vec![changed_file("文档/说明.md", None, ChangeKind::Added)]
        );
    }

    #[test]
    fn empty_output_reports_no_changed_files() {
        assert!(parse_name_status(b"")
            .expect("empty output parses")
            .is_empty());
    }

    #[test]
    fn status_without_a_path_field_is_rejected() {
        assert!(parse_name_status(b"M\0").is_err());
        assert!(parse_name_status(b"R100\0only-old.rs\0").is_err());
    }
}
