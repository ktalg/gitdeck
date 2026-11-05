use gitdeck_tui::{DiscardDialogButton, DISCARD_DIALOG_BUTTON_ORDER};

pub struct DiscardDialogState {
    path: Option<String>,
    button: DiscardDialogButton,
}

impl DiscardDialogState {
    pub const fn new() -> Self {
        Self {
            path: None,
            button: DiscardDialogButton::Cancel,
        }
    }

    pub fn open(&mut self, path: &str) {
        self.path = Some(path.to_string());
        self.button = DiscardDialogButton::Cancel;
    }

    pub fn close(&mut self) {
        self.path = None;
    }

    pub fn is_open(&self) -> bool {
        self.path.is_some()
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub const fn button(&self) -> DiscardDialogButton {
        self.button
    }

    pub fn select_button(&mut self, button: DiscardDialogButton) {
        self.button = button;
    }

    pub fn select_button_by(&mut self, step: isize) {
        let index = DISCARD_DIALOG_BUTTON_ORDER
            .iter()
            .position(|candidate| *candidate == self.button)
            .unwrap_or(0);
        let next_index =
            (index as isize + step).rem_euclid(DISCARD_DIALOG_BUTTON_ORDER.len() as isize);
        self.button = DISCARD_DIALOG_BUTTON_ORDER[usize::try_from(next_index).unwrap_or(0)];
    }
}

#[cfg(test)]
mod discard_dialog_state_tests {
    use gitdeck_tui::DiscardDialogButton;

    use super::DiscardDialogState;

    #[test]
    fn a_new_dialog_holds_the_path_and_focuses_cancel() {
        let mut state = DiscardDialogState::new();
        assert!(!state.is_open());
        state.open("src/app.rs");
        assert!(state.is_open());
        assert_eq!(state.path(), Some("src/app.rs"));
        assert_eq!(state.button(), DiscardDialogButton::Cancel);
    }

    #[test]
    fn the_button_selection_steps_between_discard_and_cancel() {
        let mut state = DiscardDialogState::new();
        state.open("src/app.rs");
        state.select_button_by(1);
        assert_eq!(state.button(), DiscardDialogButton::Discard);
        state.select_button_by(-1);
        assert_eq!(state.button(), DiscardDialogButton::Cancel);
    }

    #[test]
    fn closing_drops_the_pending_path() {
        let mut state = DiscardDialogState::new();
        state.open("src/app.rs");
        state.select_button(DiscardDialogButton::Discard);
        state.close();
        assert!(!state.is_open());
        assert_eq!(state.path(), None);
        state.open("src/other.rs");
        assert_eq!(state.button(), DiscardDialogButton::Cancel);
    }
}
