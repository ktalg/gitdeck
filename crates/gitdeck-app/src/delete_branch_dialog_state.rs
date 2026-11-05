use gitdeck_tui::{DeleteBranchDialogButton, DELETE_BRANCH_DIALOG_BUTTON_ORDER};

pub struct DeleteBranchDialogState {
    name: Option<String>,
    button: DeleteBranchDialogButton,
}

impl DeleteBranchDialogState {
    pub const fn new() -> Self {
        Self {
            name: None,
            button: DeleteBranchDialogButton::Cancel,
        }
    }

    pub fn open(&mut self, name: &str) {
        self.name = Some(name.to_string());
        self.button = DeleteBranchDialogButton::Cancel;
    }

    pub fn close(&mut self) {
        self.name = None;
    }

    pub fn is_open(&self) -> bool {
        self.name.is_some()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub const fn button(&self) -> DeleteBranchDialogButton {
        self.button
    }

    pub fn select_button_by(&mut self, step: isize) {
        let index = DELETE_BRANCH_DIALOG_BUTTON_ORDER
            .iter()
            .position(|candidate| *candidate == self.button)
            .unwrap_or(0);
        let next_index =
            (index as isize + step).rem_euclid(DELETE_BRANCH_DIALOG_BUTTON_ORDER.len() as isize);
        self.button = DELETE_BRANCH_DIALOG_BUTTON_ORDER[usize::try_from(next_index).unwrap_or(0)];
    }
}

#[cfg(test)]
mod delete_branch_dialog_state_tests {
    use gitdeck_tui::DeleteBranchDialogButton;

    use super::DeleteBranchDialogState;

    #[test]
    fn a_new_dialog_holds_the_branch_name_and_focuses_cancel() {
        let mut state = DeleteBranchDialogState::new();
        assert!(!state.is_open());
        state.open("feature/gone");
        assert!(state.is_open());
        assert_eq!(state.name(), Some("feature/gone"));
        assert_eq!(state.button(), DeleteBranchDialogButton::Cancel);
    }

    #[test]
    fn the_button_selection_steps_between_delete_and_cancel() {
        let mut state = DeleteBranchDialogState::new();
        state.open("feature/gone");
        state.select_button_by(1);
        assert_eq!(state.button(), DeleteBranchDialogButton::Delete);
        state.select_button_by(-1);
        assert_eq!(state.button(), DeleteBranchDialogButton::Cancel);
    }

    #[test]
    fn closing_drops_the_pending_name() {
        let mut state = DeleteBranchDialogState::new();
        state.open("feature/gone");
        state.select_button_by(1);
        state.close();
        assert!(!state.is_open());
        assert_eq!(state.name(), None);
        state.open("feature/other");
        assert_eq!(state.button(), DeleteBranchDialogButton::Cancel);
    }
}
