use gitdeck_core::CommitSummary;

use crate::decoding::decode_text;
use crate::error::GitError;

const RECORD_SEPARATOR: u8 = 0x1e;
const FIELD_SEPARATOR: u8 = 0x1f;
const LOG_FIELD_COUNT: usize = 10;
const LOG_CONTEXT: &str = "log";
const AUTHOR_ROLE: &str = "author";
const COMMITTER_ROLE: &str = "committer";

pub(crate) const LOG_FORMAT_ARGUMENT: &str =
    "--format=%H%x1f%h%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%s%x1e";

pub(crate) fn parse_log_records(output: &[u8]) -> Result<Vec<CommitSummary>, GitError> {
    output
        .split(|byte| *byte == RECORD_SEPARATOR)
        .map(trim_line_breaks)
        .filter(|record| !record.is_empty())
        .map(parse_log_record)
        .collect()
}

fn parse_log_record(record: &[u8]) -> Result<CommitSummary, GitError> {
    let fields: Vec<&[u8]> = record.split(|byte| *byte == FIELD_SEPARATOR).collect();
    if fields.len() != LOG_FIELD_COUNT {
        return Err(GitError::UnexpectedOutput {
            context: LOG_CONTEXT,
            message: format!(
                "record needs {LOG_FIELD_COUNT} unit separated fields, found {}",
                fields.len()
            ),
        });
    }
    Ok(CommitSummary {
        id: decode_text(fields[0]),
        short_id: decode_text(fields[1]),
        parent_ids: decode_text(fields[2])
            .split(' ')
            .filter(|parent_id| !parent_id.is_empty())
            .map(str::to_string)
            .collect(),
        author_name: decode_text(fields[3]),
        author_email: decode_text(fields[4]),
        author_timestamp: parse_timestamp(fields[5], AUTHOR_ROLE)?,
        committer_name: decode_text(fields[6]),
        committer_email: decode_text(fields[7]),
        committer_timestamp: parse_timestamp(fields[8], COMMITTER_ROLE)?,
        subject: decode_text(fields[9]),
        refs: Vec::new(),
    })
}

fn parse_timestamp(field: &[u8], role: &str) -> Result<i64, GitError> {
    let text = decode_text(field);
    text.parse::<i64>()
        .map_err(|error| GitError::UnexpectedOutput {
            context: LOG_CONTEXT,
            message: format!("{role} timestamp {text:?} is not a signed integer: {error}"),
        })
}

fn trim_line_breaks(record: &[u8]) -> &[u8] {
    let start = record
        .iter()
        .position(|byte| !matches!(byte, b'\n' | b'\r'))
        .unwrap_or(record.len());
    let end = record
        .iter()
        .rposition(|byte| !matches!(byte, b'\n' | b'\r'))
        .map_or(start, |index| index + 1);
    &record[start..end]
}

#[cfg(test)]
mod log_record_tests {
    use super::parse_log_records;

    const COMMIT_ID: &str = "1111111111111111111111111111111111111111";
    const PARENT_ID: &str = "2222222222222222222222222222222222222222";
    const SECOND_PARENT_ID: &str = "3333333333333333333333333333333333333333";

    fn record(parents: &str, timestamp: &str, subject: &str) -> String {
        record_with_committer(
            parents,
            timestamp,
            "gitdeck-fixture",
            "fixture@gitdeck.invalid",
            timestamp,
            subject,
        )
    }

    fn record_with_committer(
        parents: &str,
        author_timestamp: &str,
        committer_name: &str,
        committer_email: &str,
        committer_timestamp: &str,
        subject: &str,
    ) -> String {
        format!(
            "{COMMIT_ID}\u{1f}1111111\u{1f}{parents}\u{1f}gitdeck-fixture\u{1f}\
             fixture@gitdeck.invalid\u{1f}{author_timestamp}\u{1f}{committer_name}\u{1f}\
             {committer_email}\u{1f}{committer_timestamp}\u{1f}{subject}\u{1e}\n"
        )
    }

    #[test]
    fn log_records_carry_the_committer_fields() {
        let commits = parse_log_records(
            record_with_committer(
                PARENT_ID,
                "1577836800",
                "Someone Else",
                "else@gitdeck.invalid",
                "1577836999",
                "amended subject",
            )
            .as_bytes(),
        )
        .expect("record parses");
        assert_eq!(commits[0].author_name, "gitdeck-fixture");
        assert_eq!(commits[0].author_email, "fixture@gitdeck.invalid");
        assert_eq!(commits[0].author_timestamp, 1_577_836_800);
        assert_eq!(commits[0].committer_name, "Someone Else");
        assert_eq!(commits[0].committer_email, "else@gitdeck.invalid");
        assert_eq!(commits[0].committer_timestamp, 1_577_836_999);
        assert_eq!(commits[0].subject, "amended subject");
    }

    #[test]
    fn parse_log_records_keeps_a_tab_inside_the_subject() {
        let commits =
            parse_log_records(record(PARENT_ID, "1577836800", "fix:\tconflict").as_bytes())
                .expect("record parses");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "fix:\tconflict");
    }

    #[test]
    fn parse_log_records_keeps_wide_characters_inside_the_subject() {
        let commits =
            parse_log_records(record(PARENT_ID, "1577836800", "更新：说明文档").as_bytes())
                .expect("record parses");
        assert_eq!(commits[0].subject, "更新：说明文档");
    }

    #[test]
    fn parse_log_records_keeps_percent_signs_inside_the_subject() {
        let commits =
            parse_log_records(record(PARENT_ID, "1577836800", "reduce 50% allocations").as_bytes())
                .expect("record parses");
        assert_eq!(commits[0].subject, "reduce 50% allocations");
    }

    #[test]
    fn parse_log_records_accepts_an_empty_subject_and_an_empty_parent_list() {
        let commits =
            parse_log_records(record("", "1577836800", "").as_bytes()).expect("record parses");
        assert_eq!(commits[0].subject, "");
        assert!(commits[0].parent_ids.is_empty());
        assert_eq!(commits[0].author_email, "fixture@gitdeck.invalid");
        assert_eq!(commits[0].author_timestamp, 1_577_836_800);
        assert_eq!(commits[0].short_id, "1111111");
    }

    #[test]
    fn parse_log_records_keeps_every_parent_of_a_merge_commit() {
        let parents = format!("{PARENT_ID} {SECOND_PARENT_ID}");
        let commits = parse_log_records(record(&parents, "1577836800", "merge side-0").as_bytes())
            .expect("record parses");
        assert_eq!(
            commits[0].parent_ids,
            vec![String::from(PARENT_ID), String::from(SECOND_PARENT_ID)]
        );
    }

    #[test]
    fn parse_log_records_splits_records_on_the_record_separator_not_on_lines() {
        let mut output = record(PARENT_ID, "1577836900", "subject with space");
        output.push_str(&record(
            PARENT_ID,
            "1577836800",
            "second\nsubject kept as one",
        ));
        let commits = parse_log_records(output.as_bytes()).expect("records parse");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "subject with space");
        assert_eq!(commits[0].author_timestamp, 1_577_836_900);
        assert_eq!(commits[1].author_timestamp, 1_577_836_800);
    }

    #[test]
    fn parse_log_records_returns_nothing_for_empty_output() {
        assert!(parse_log_records(b"")
            .expect("empty output parses")
            .is_empty());
        assert!(parse_log_records(b"\n")
            .expect("blank output parses")
            .is_empty());
    }

    #[test]
    fn parse_log_records_rejects_records_with_a_wrong_field_count() {
        assert!(parse_log_records(format!("{COMMIT_ID}\u{1f}1111111\u{1e}\n").as_bytes()).is_err());
        assert!(parse_log_records(b"only-one-field\x1e\n").is_err());
    }

    #[test]
    fn parse_log_records_rejects_a_timestamp_that_is_not_an_integer() {
        assert!(parse_log_records(record(PARENT_ID, "not-a-time", "subject").as_bytes()).is_err());
    }
}
