use std::collections::VecDeque;

use unicode_width::UnicodeWidthChar;

const MESSAGE_UNDO_DEPTH: usize = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChangesFocus {
    List,
    Message,
}

pub(crate) struct CommitState {
    message: String,
    undo_history: VecDeque<String>,
    focus: ChangesFocus,
    amend: bool,
    sign_off: bool,
    options_visible: bool,
}

impl CommitState {
    pub(crate) fn new() -> Self {
        Self {
            message: String::new(),
            undo_history: VecDeque::new(),
            focus: ChangesFocus::List,
            amend: false,
            sign_off: false,
            options_visible: false,
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn focus(&self) -> ChangesFocus {
        self.focus
    }

    pub(crate) fn is_editing(&self) -> bool {
        self.focus == ChangesFocus::Message
    }

    pub(crate) fn amend(&self) -> bool {
        self.amend
    }

    pub(crate) fn sign_off(&self) -> bool {
        self.sign_off
    }

    pub(crate) fn options_visible(&self) -> bool {
        self.options_visible
    }

    pub(crate) fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
        self.undo_history.clear();
    }

    pub(crate) fn enter_message_editor(&mut self) {
        self.focus = ChangesFocus::Message;
    }

    pub(crate) fn leave_message_editor(&mut self) {
        self.focus = ChangesFocus::List;
    }

    pub(crate) fn set_amend(&mut self, amend: bool) {
        self.amend = amend;
    }

    pub(crate) fn toggle_sign_off(&mut self) {
        self.sign_off = !self.sign_off;
    }

    pub(crate) fn set_options_visible(&mut self, visible: bool) {
        self.options_visible = visible;
    }

    pub(crate) fn insert_character(&mut self, character: char) {
        self.record_undo();
        self.message.push(character);
    }

    pub(crate) fn insert_newline(&mut self) {
        self.record_undo();
        self.message.push('\n');
    }

    pub(crate) fn remove_last_grapheme(&mut self) {
        if self.message.is_empty() {
            return;
        }
        self.record_undo();
        pop_last_grapheme(&mut self.message);
    }

    pub(crate) fn undo(&mut self) {
        if let Some(previous) = self.undo_history.pop_back() {
            self.message = previous;
        }
    }

    pub(crate) fn clear_after_commit(&mut self) {
        self.message.clear();
        self.undo_history.clear();
        self.focus = ChangesFocus::List;
        self.amend = false;
    }

    fn record_undo(&mut self) {
        if self.undo_history.len() == MESSAGE_UNDO_DEPTH {
            self.undo_history.pop_front();
        }
        self.undo_history.push_back(self.message.clone());
    }
}

fn pop_last_grapheme(message: &mut String) -> bool {
    loop {
        while message.chars().next_back().is_some_and(is_zero_width) {
            if !pop_last_character(message) {
                return false;
            }
        }
        if !pop_last_character(message) {
            return false;
        }
        if !message.chars().next_back().is_some_and(is_zero_width) {
            return true;
        }
    }
}

fn pop_last_character(message: &mut String) -> bool {
    match message.char_indices().next_back() {
        Some((index, _)) => {
            message.truncate(index);
            true
        }
        None => false,
    }
}

fn is_zero_width(character: char) -> bool {
    matches!(UnicodeWidthChar::width(character), None | Some(0))
}

#[cfg(test)]
mod commit_state_tests {
    use super::{ChangesFocus, CommitState, MESSAGE_UNDO_DEPTH};

    fn state_with_message(message: &str) -> CommitState {
        let mut state = CommitState::new();
        state.set_message(message);
        state
    }

    #[test]
    fn typing_appends_wide_and_combining_characters_verbatim() {
        let mut state = CommitState::new();
        for character in "修复：冲突 📦".chars() {
            state.insert_character(character);
        }
        assert_eq!(state.message(), "修复：冲突 📦");
        state.insert_character('👨');
        state.insert_character('\u{200D}');
        state.insert_character('👩');
        assert_eq!(state.message(), "修复：冲突 📦👨‍👩");
    }

    #[test]
    fn removing_the_last_grapheme_keeps_wide_characters_whole() {
        let mut state = state_with_message("ab中文");
        state.remove_last_grapheme();
        assert_eq!(state.message(), "ab中");
        state.remove_last_grapheme();
        assert_eq!(state.message(), "ab");
    }

    #[test]
    fn removing_the_last_grapheme_consumes_a_joined_emoji_sequence() {
        let mut state = state_with_message("a👨‍👩‍👦");
        state.remove_last_grapheme();
        assert_eq!(state.message(), "a");
        let mut combining = state_with_message("e\u{301}");
        combining.remove_last_grapheme();
        assert_eq!(combining.message(), "");
        combining.remove_last_grapheme();
        assert_eq!(combining.message(), "");
    }

    #[test]
    fn undo_walks_back_through_fifty_edits() {
        let mut state = CommitState::new();
        for _ in 0..MESSAGE_UNDO_DEPTH {
            state.insert_character('x');
        }
        assert_eq!(state.message().len(), MESSAGE_UNDO_DEPTH);
        for _ in 0..MESSAGE_UNDO_DEPTH {
            state.undo();
        }
        assert_eq!(state.message(), "");
        state.undo();
        assert_eq!(state.message(), "");
    }

    #[test]
    fn undo_restores_the_message_removed_by_backspace() {
        let mut state = state_with_message("hello");
        state.remove_last_grapheme();
        assert_eq!(state.message(), "hell");
        state.undo();
        assert_eq!(state.message(), "hello");
    }

    #[test]
    fn setting_the_message_clears_the_undo_history() {
        let mut state = state_with_message("typed");
        state.insert_character('!');
        state.set_message("replaced");
        state.undo();
        assert_eq!(state.message(), "replaced");
    }

    #[test]
    fn focus_amend_sign_off_and_options_are_tracked_separately_from_the_message() {
        let mut state = CommitState::new();
        assert_eq!(state.focus(), ChangesFocus::List);
        assert!(!state.is_editing());
        state.enter_message_editor();
        assert_eq!(state.focus(), ChangesFocus::Message);
        assert!(state.is_editing());
        state.leave_message_editor();
        assert_eq!(state.focus(), ChangesFocus::List);
        state.set_amend(true);
        assert!(state.amend());
        state.toggle_sign_off();
        assert!(state.sign_off());
        state.set_options_visible(true);
        assert!(state.options_visible());
        state.set_message("kept");
        state.clear_after_commit();
        assert_eq!(state.message(), "");
        assert!(!state.amend());
        assert!(
            state.sign_off(),
            "the sign-off preference survives a commit"
        );
    }
}
