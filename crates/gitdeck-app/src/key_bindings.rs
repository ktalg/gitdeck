#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCommand {
    FocusNextPane,
    FocusPreviousPane,
    FocusChangesPane,
    FocusLogPane,
    FocusDetailsPane,
    ToggleHelp,
    ToggleLogView,
    OpenBranchFilterDropdown,
    OpenFetchDialog,
    OpenPullDialog,
    CloseActiveTab,
    CloseTopmostLayer,
    ToggleWordHighlights,
    ToggleSoftWrap,
    ToggleCaseSensitiveFilter,
    AcceptOurs,
    AcceptTheirs,
    MarkResolved,
    MergeBranch,
    CreateBranch,
    DeleteBranch,
    DiscardChanges,
    IgnorePath,
    AbortOperation,
    ContinueOperation,
    ToggleAmend,
    ToggleSignOff,
    CommitCheckedFiles,
    Refresh,
    Quit,
    MoveSelection,
    PageSelection,
    JumpToNewestOrOldest,
    CollapseOrExpandGroup,
    ToggleCheckbox,
    OpenDiff,
    EditLogFilter,
    EditCommitMessage,
}

impl KeyCommand {
    pub const EVERY: [Self; 38] = [
        Self::FocusNextPane,
        Self::FocusPreviousPane,
        Self::FocusChangesPane,
        Self::FocusLogPane,
        Self::FocusDetailsPane,
        Self::ToggleHelp,
        Self::ToggleLogView,
        Self::OpenBranchFilterDropdown,
        Self::OpenFetchDialog,
        Self::OpenPullDialog,
        Self::CloseActiveTab,
        Self::CloseTopmostLayer,
        Self::ToggleWordHighlights,
        Self::ToggleSoftWrap,
        Self::ToggleCaseSensitiveFilter,
        Self::AcceptOurs,
        Self::AcceptTheirs,
        Self::MarkResolved,
        Self::MergeBranch,
        Self::CreateBranch,
        Self::DeleteBranch,
        Self::DiscardChanges,
        Self::IgnorePath,
        Self::AbortOperation,
        Self::ContinueOperation,
        Self::ToggleAmend,
        Self::ToggleSignOff,
        Self::CommitCheckedFiles,
        Self::Refresh,
        Self::Quit,
        Self::MoveSelection,
        Self::PageSelection,
        Self::JumpToNewestOrOldest,
        Self::CollapseOrExpandGroup,
        Self::ToggleCheckbox,
        Self::OpenDiff,
        Self::EditLogFilter,
        Self::EditCommitMessage,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyBinding {
    pub keys: &'static str,
    pub action: &'static str,
    pub commands: &'static [KeyCommand],
}

pub const KEY_BINDINGS: &[KeyBinding] = &[
    KeyBinding {
        keys: "tab, shift+tab",
        action: "move the focus through Changes, Log and Details",
        commands: &[KeyCommand::FocusNextPane, KeyCommand::FocusPreviousPane],
    },
    KeyBinding {
        keys: "ctrl+k, alt+1",
        action: "focus the Changes pane",
        commands: &[KeyCommand::FocusChangesPane],
    },
    KeyBinding {
        keys: "alt+9, alt+2",
        action: "focus the Log pane",
        commands: &[KeyCommand::FocusLogPane],
    },
    KeyBinding {
        keys: "alt+3",
        action: "focus the Details pane",
        commands: &[KeyCommand::FocusDetailsPane],
    },
    KeyBinding {
        keys: "?",
        action: "open or close the key list",
        commands: &[KeyCommand::ToggleHelp],
    },
    KeyBinding {
        keys: "b",
        action: "switch the Log tab between the commits and the branches view, ignored while a file diff tab shows",
        commands: &[KeyCommand::ToggleLogView],
    },
    KeyBinding {
        keys: "j, down / k, up",
        action: "move the changes selection, move the commit selection while the Log tab shows, scroll the diff while a diff tab shows",
        commands: &[KeyCommand::MoveSelection],
    },
    KeyBinding {
        keys: "pageup, pagedown",
        action: "jump ten commits, or scroll the diff by one page",
        commands: &[KeyCommand::PageSelection],
    },
    KeyBinding {
        keys: "home, end",
        action: "select the newest or the oldest commit",
        commands: &[KeyCommand::JumpToNewestOrOldest],
    },
    KeyBinding {
        keys: "left, right",
        action: "collapse or expand the selected group",
        commands: &[KeyCommand::CollapseOrExpandGroup],
    },
    KeyBinding {
        keys: "space",
        action: "toggle the checkbox of the selected row, check the hunk under the cursor while a diff tab shows",
        commands: &[KeyCommand::ToggleCheckbox],
    },
    KeyBinding {
        keys: "enter",
        action: "open the diff of the selected file",
        commands: &[KeyCommand::OpenDiff],
    },
    KeyBinding {
        keys: "/",
        action: "type a log filter, enter applies it, esc cancels",
        commands: &[KeyCommand::EditLogFilter],
    },
    KeyBinding {
        keys: "i",
        action: "focus the commit message from any pane, esc leaves the editor, the editor takes characters, enter, backspace and ctrl+z",
        commands: &[KeyCommand::EditCommitMessage],
    },
    KeyBinding {
        keys: "alt+a",
        action: "amend the last commit",
        commands: &[KeyCommand::ToggleAmend],
    },
    KeyBinding {
        keys: "alt+s",
        action: "toggle the sign-off trailer",
        commands: &[KeyCommand::ToggleSignOff],
    },
    KeyBinding {
        keys: "alt+m",
        action: "merge the selected branch into the current one",
        commands: &[KeyCommand::MergeBranch],
    },
    KeyBinding {
        keys: "alt+n",
        action: "create a branch from the selected one",
        commands: &[KeyCommand::CreateBranch],
    },
    KeyBinding {
        keys: "alt+shift+d",
        action: "delete the selected local branch",
        commands: &[KeyCommand::DeleteBranch],
    },
    KeyBinding {
        keys: "alt+d",
        action: "discard the local changes of the selected file",
        commands: &[KeyCommand::DiscardChanges],
    },
    KeyBinding {
        keys: "alt+i",
        action: "add the selected untracked file to .gitignore",
        commands: &[KeyCommand::IgnorePath],
    },
    KeyBinding {
        keys: "alt+o",
        action: "accept our side of the selected conflict",
        commands: &[KeyCommand::AcceptOurs],
    },
    KeyBinding {
        keys: "alt+t",
        action: "accept their side of the selected conflict",
        commands: &[KeyCommand::AcceptTheirs],
    },
    KeyBinding {
        keys: "alt+r",
        action: "mark the selected conflict as resolved",
        commands: &[KeyCommand::MarkResolved],
    },
    KeyBinding {
        keys: "alt+x",
        action: "abort the merge, rebase, cherry-pick or revert in progress",
        commands: &[KeyCommand::AbortOperation],
    },
    KeyBinding {
        keys: "alt+u",
        action: "continue the operation in progress",
        commands: &[KeyCommand::ContinueOperation],
    },
    KeyBinding {
        keys: "alt+w",
        action: "toggle the word-level diff highlight",
        commands: &[KeyCommand::ToggleWordHighlights],
    },
    KeyBinding {
        keys: "alt+z",
        action: "toggle the soft wrap of the diff lines",
        commands: &[KeyCommand::ToggleSoftWrap],
    },
    KeyBinding {
        keys: "alt+c",
        action: "toggle the case sensitivity of the log text filter",
        commands: &[KeyCommand::ToggleCaseSensitiveFilter],
    },
    KeyBinding {
        keys: "ctrl+b",
        action: "open the branch filter dropdown",
        commands: &[KeyCommand::OpenBranchFilterDropdown],
    },
    KeyBinding {
        keys: "p",
        action: "open the pull dialog",
        commands: &[KeyCommand::OpenPullDialog],
    },
    KeyBinding {
        keys: "f",
        action: "open the fetch dialog",
        commands: &[KeyCommand::OpenFetchDialog],
    },
    KeyBinding {
        keys: "ctrl+enter",
        action: "commit the checked files",
        commands: &[KeyCommand::CommitCheckedFiles],
    },
    KeyBinding {
        keys: "f5",
        action: "refresh the repository",
        commands: &[KeyCommand::Refresh],
    },
    KeyBinding {
        keys: "ctrl+w",
        action: "close the active diff tab",
        commands: &[KeyCommand::CloseActiveTab],
    },
    KeyBinding {
        keys: "ctrl+q, ctrl+c",
        action: "quit",
        commands: &[KeyCommand::Quit],
    },
    KeyBinding {
        keys: "esc",
        action: "close the error block or the active diff tab, never quits",
        commands: &[KeyCommand::CloseTopmostLayer],
    },
];

pub fn key_bindings_for_keys(combination: &str) -> Vec<&'static KeyBinding> {
    KEY_BINDINGS
        .iter()
        .filter(|binding| {
            binding.keys == combination
                || binding
                    .keys
                    .split([',', '/'])
                    .map(str::trim)
                    .any(|token| token == combination)
        })
        .collect()
}

pub fn key_bindings_for(command: KeyCommand) -> Vec<&'static KeyBinding> {
    KEY_BINDINGS
        .iter()
        .filter(|binding| binding.commands.contains(&command))
        .collect()
}

#[cfg(test)]
mod key_bindings_tests {
    use std::collections::BTreeSet;

    use super::{key_bindings_for, key_bindings_for_keys, KeyBinding, KeyCommand, KEY_BINDINGS};

    const MAIN_ACTION_KEYS: [&str; 47] = [
        "tab",
        "shift+tab",
        "ctrl+k",
        "alt+1",
        "alt+2",
        "alt+3",
        "alt+9",
        "?",
        "b",
        "j",
        "down",
        "k",
        "up",
        "pageup",
        "pagedown",
        "home",
        "end",
        "left",
        "right",
        "space",
        "enter",
        "/",
        "i",
        "alt+a",
        "alt+s",
        "alt+m",
        "alt+n",
        "alt+shift+d",
        "alt+d",
        "alt+i",
        "alt+o",
        "alt+t",
        "alt+r",
        "alt+x",
        "alt+u",
        "alt+w",
        "alt+z",
        "alt+c",
        "ctrl+b",
        "p",
        "f",
        "ctrl+enter",
        "f5",
        "ctrl+w",
        "ctrl+q",
        "ctrl+c",
        "esc",
    ];

    #[test]
    fn the_binding_registry_lists_every_main_action() {
        for keys in MAIN_ACTION_KEYS {
            let rows = key_bindings_for_keys(keys);
            assert!(
                !rows.is_empty(),
                "{keys} is wired in apply_key but missing from the key list"
            );
            assert!(
                rows.iter().any(|row| !row.action.trim().is_empty()),
                "{keys} has no readable action"
            );
        }
    }

    #[test]
    fn every_command_is_reachable_through_the_registry() {
        for command in KeyCommand::EVERY {
            assert!(
                !key_bindings_for(command).is_empty(),
                "{command:?} has no key binding"
            );
        }
    }

    #[test]
    fn the_command_list_and_the_registry_hold_no_duplicates() {
        let listed: BTreeSet<String> = KeyCommand::EVERY
            .iter()
            .map(|command| format!("{command:?}"))
            .collect();
        assert_eq!(
            listed.len(),
            KeyCommand::EVERY.len(),
            "the command list repeats an entry"
        );
        let keys: BTreeSet<&str> = KEY_BINDINGS.iter().map(|binding| binding.keys).collect();
        assert_eq!(
            keys.len(),
            KEY_BINDINGS.len(),
            "the registry repeats a key row"
        );
    }

    #[test]
    fn every_row_carries_a_key_row_and_an_action() {
        for KeyBinding {
            keys,
            action,
            commands,
        } in KEY_BINDINGS
        {
            assert!(!keys.trim().is_empty(), "a row has no keys");
            assert!(!action.trim().is_empty(), "{keys} has no action");
            assert!(!commands.is_empty(), "{keys} drives no command");
        }
    }
}
