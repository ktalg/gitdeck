pub(crate) struct NewBranchDialogState {
    name: Option<String>,
}

impl NewBranchDialogState {
    pub(crate) const fn new() -> Self {
        Self { name: None }
    }

    pub(crate) fn open(&mut self) {
        self.name = Some(String::new());
    }

    pub(crate) fn close(&mut self) {
        self.name = None;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.name.is_some()
    }

    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(crate) fn push_char(&mut self, character: char) {
        if let Some(name) = self.name.as_mut() {
            name.push(character);
        }
    }

    pub(crate) fn pop_char(&mut self) {
        if let Some(name) = self.name.as_mut() {
            name.pop();
        }
    }
}

#[cfg(test)]
mod new_branch_dialog_state_tests {
    use super::NewBranchDialogState;

    #[test]
    fn an_opened_dialog_starts_with_an_empty_name() {
        let mut state = NewBranchDialogState::new();
        assert!(!state.is_open());
        state.open();
        assert!(state.is_open());
        assert_eq!(state.name(), Some(""));
    }

    #[test]
    fn typed_characters_append_and_backspace_drops_the_last_one() {
        let mut state = NewBranchDialogState::new();
        state.open();
        for character in "feature/x".chars() {
            state.push_char(character);
        }
        assert_eq!(state.name(), Some("feature/x"));
        state.pop_char();
        assert_eq!(state.name(), Some("feature/"));
        state.pop_char();
        assert_eq!(state.name(), Some("feature"));
    }

    #[test]
    fn closing_drops_the_typed_name() {
        let mut state = NewBranchDialogState::new();
        state.open();
        state.push_char('x');
        state.close();
        assert!(!state.is_open());
        assert_eq!(state.name(), None);
        state.open();
        assert_eq!(state.name(), Some(""));
    }
}
