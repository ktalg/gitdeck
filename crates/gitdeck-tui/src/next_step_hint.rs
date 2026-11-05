use gitdeck_caps::CapsProfile;

pub const NEXT_STEP_HINT_PREFIX: &str = "→ ";
pub const NEXT_STEP_HINT_ASCII_PREFIX: &str = "-> ";

pub const LOCKED_INDEX_HINT: &str = "another git process holds the index, retry in a moment";
pub const NOT_FAST_FORWARD_PULL_HINT: &str = "pull with the Merge or Rebase strategy";
pub const DIRTY_CHECKOUT_HINT: &str = "commit or discard those changes, then switch the branch";
pub const REJECTED_PUSH_HINT: &str = "pull before pushing, or use Force with lease";
pub const MISSING_REMOTE_HINT: &str = "configure a remote first (git remote add)";
pub const UNKNOWN_BRANCH_HINT: &str = "check the branch name (fetch again, then retry)";
pub const MERGE_CONFLICT_HINT: &str =
    "resolve the conflicts and continue (alt+u), or abort (alt+x)";

pub const NEXT_STEP_HINT_TEXTS: [&str; 7] = [
    LOCKED_INDEX_HINT,
    NOT_FAST_FORWARD_PULL_HINT,
    DIRTY_CHECKOUT_HINT,
    REJECTED_PUSH_HINT,
    MISSING_REMOTE_HINT,
    UNKNOWN_BRANCH_HINT,
    MERGE_CONFLICT_HINT,
];

pub fn next_step_hints() -> &'static [&'static str] {
    &NEXT_STEP_HINT_TEXTS
}

const NEXT_STEP_HINTS: [(&[&str], &str); 7] = [
    (&["index.lock", "unable to create"], LOCKED_INDEX_HINT),
    (
        &["not possible to fast-forward"],
        NOT_FAST_FORWARD_PULL_HINT,
    ),
    (&["would be overwritten by checkout"], DIRTY_CHECKOUT_HINT),
    (&["non-fast-forward", "[rejected]"], REJECTED_PUSH_HINT),
    (&["has no remote"], MISSING_REMOTE_HINT),
    (
        &["did not match any file(s) known to git"],
        UNKNOWN_BRANCH_HINT,
    ),
    (
        &["conflict", "automatic merge failed", "could not apply"],
        MERGE_CONFLICT_HINT,
    ),
];

pub fn next_step_hint(message: &str) -> Option<&'static str> {
    let lowered = message.to_lowercase();
    NEXT_STEP_HINTS.iter().find_map(|(fragments, hint)| {
        fragments
            .iter()
            .any(|fragment| lowered.contains(fragment))
            .then_some(*hint)
    })
}

pub fn next_step_hint_prefix(caps: CapsProfile) -> &'static str {
    match caps {
        CapsProfile::Basic => NEXT_STEP_HINT_ASCII_PREFIX,
        CapsProfile::Rich | CapsProfile::Standard => NEXT_STEP_HINT_PREFIX,
    }
}

#[cfg(test)]
mod next_step_hint_tests {
    use gitdeck_caps::CapsProfile;

    use super::{
        next_step_hint, next_step_hint_prefix, DIRTY_CHECKOUT_HINT, LOCKED_INDEX_HINT,
        MERGE_CONFLICT_HINT, MISSING_REMOTE_HINT, NEXT_STEP_HINT_ASCII_PREFIX,
        NEXT_STEP_HINT_PREFIX, NOT_FAST_FORWARD_PULL_HINT, REJECTED_PUSH_HINT, UNKNOWN_BRANCH_HINT,
    };

    const KNOWN_FAILURES: [(&str, &str); 9] = [
        (
            "git add -A -- src/app.rs failed in /repo: fatal: Unable to create '/repo/.git/index.lock': File exists.",
            LOCKED_INDEX_HINT,
        ),
        (
            "fatal: Unable to create '/repo/.git/index.lock': File exists.",
            LOCKED_INDEX_HINT,
        ),
        (
            "Pull failed: fatal: Not possible to fast-forward, aborting.",
            NOT_FAST_FORWARD_PULL_HINT,
        ),
        (
            "error: Your local changes to the following files would be overwritten by checkout: src/app.rs",
            DIRTY_CHECKOUT_HINT,
        ),
        (
            "! [rejected] main -> main (non-fast-forward)",
            REJECTED_PUSH_HINT,
        ),
        (
            "the repository has no remote to push to",
            MISSING_REMOTE_HINT,
        ),
        (
            "error: pathspec 'feature/other' did not match any file(s) known to git",
            UNKNOWN_BRANCH_HINT,
        ),
        (
            "CONFLICT (content): Merge conflict in src/app.rs; Automatic merge failed; fix conflicts and then commit the result.",
            MERGE_CONFLICT_HINT,
        ),
        (
            "Pull failed: Rebasing (1/1) error: could not apply 8e3ed63... local commit",
            MERGE_CONFLICT_HINT,
        ),
    ];

    #[test]
    fn next_step_hint_matches_the_known_failures() {
        for (message, expected) in KNOWN_FAILURES {
            assert_eq!(
                next_step_hint(message),
                Some(expected),
                "the failure {message:?} names the next step"
            );
        }
        assert_eq!(
            next_step_hint("GIT ADD FAILED IN /REPO: INDEX.LOCK IS HELD"),
            Some(LOCKED_INDEX_HINT),
            "the match ignores the case of the git text"
        );
    }

    #[test]
    fn next_step_hint_returns_none_for_unknown_failures() {
        for message in [
            "",
            "Refresh failed",
            "git status --porcelain failed in /repo: fatal: not a git repository",
            "the pre-commit hook refused the commit",
        ] {
            assert_eq!(
                next_step_hint(message),
                None,
                "the unknown failure {message:?} names no next step"
            );
        }
    }

    #[test]
    fn basic_caps_use_the_ascii_hint_prefix() {
        assert_eq!(
            next_step_hint_prefix(CapsProfile::Basic),
            NEXT_STEP_HINT_ASCII_PREFIX
        );
        assert_eq!(
            next_step_hint_prefix(CapsProfile::Rich),
            NEXT_STEP_HINT_PREFIX
        );
        assert_eq!(
            next_step_hint_prefix(CapsProfile::Standard),
            NEXT_STEP_HINT_PREFIX
        );
        assert!(NEXT_STEP_HINT_ASCII_PREFIX.is_ascii());
    }
}
