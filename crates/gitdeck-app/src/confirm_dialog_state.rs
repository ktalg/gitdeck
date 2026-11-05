use gitdeck_tui::{ConfirmDialogButton, CONFIRM_DIALOG_BUTTON_ORDER};

pub struct ConfirmDialogState {
    is_open: bool,
    button: ConfirmDialogButton,
}

impl ConfirmDialogState {
    pub const fn new() -> Self {
        Self {
            is_open: false,
            button: ConfirmDialogButton::Cancel,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.button = ConfirmDialogButton::Cancel;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.button = ConfirmDialogButton::Cancel;
    }

    pub const fn is_open(&self) -> bool {
        self.is_open
    }

    pub const fn button(&self) -> ConfirmDialogButton {
        self.button
    }

    pub fn select_button(&mut self, button: ConfirmDialogButton) {
        self.button = button;
    }

    pub fn select_button_by(&mut self, step: isize) {
        let index = CONFIRM_DIALOG_BUTTON_ORDER
            .iter()
            .position(|candidate| *candidate == self.button)
            .unwrap_or(0);
        let next_index =
            (index as isize + step).rem_euclid(CONFIRM_DIALOG_BUTTON_ORDER.len() as isize);
        self.button = CONFIRM_DIALOG_BUTTON_ORDER[usize::try_from(next_index).unwrap_or(0)];
    }
}

#[cfg(test)]
mod confirm_dialog_state_tests {
    use gitdeck_tui::ConfirmDialogButton;

    use super::ConfirmDialogState;

    #[test]
    fn a_confirmation_opens_on_cancel_and_steps_to_the_confirmation() {
        let mut state = ConfirmDialogState::new();
        assert!(!state.is_open());
        state.open();
        assert!(state.is_open());
        assert_eq!(state.button(), ConfirmDialogButton::Cancel);
        state.select_button_by(1);
        assert_eq!(state.button(), ConfirmDialogButton::Confirm);
        state.select_button_by(-1);
        assert_eq!(state.button(), ConfirmDialogButton::Cancel);
    }

    #[test]
    fn closing_and_reopening_returns_the_focus_to_cancel() {
        let mut state = ConfirmDialogState::new();
        state.open();
        state.select_button(ConfirmDialogButton::Confirm);
        state.close();
        assert!(!state.is_open());
        state.open();
        assert_eq!(state.button(), ConfirmDialogButton::Cancel);
    }
}
