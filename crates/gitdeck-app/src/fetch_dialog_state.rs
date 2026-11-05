use gitdeck_tui::{FetchDialogButton, FETCH_DIALOG_BUTTON_ORDER};

pub struct FetchDialogState {
    open: bool,
    button: FetchDialogButton,
}

impl FetchDialogState {
    pub const fn new() -> Self {
        Self {
            open: false,
            button: FetchDialogButton::Fetch,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.button = FetchDialogButton::Fetch;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn button(&self) -> FetchDialogButton {
        self.button
    }

    pub fn select_button(&mut self, button: FetchDialogButton) {
        self.button = button;
    }

    pub fn select_button_by(&mut self, step: isize) {
        self.button = step_through(&FETCH_DIALOG_BUTTON_ORDER, self.button, step);
    }
}

fn step_through<T: Copy + PartialEq>(order: &[T], current: T, step: isize) -> T {
    let index = order
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);
    let next_index = (index as isize + step).rem_euclid(order.len() as isize);
    order[usize::try_from(next_index).unwrap_or(0)]
}

#[cfg(test)]
mod fetch_dialog_state_tests {
    use gitdeck_tui::FetchDialogButton;

    use super::FetchDialogState;

    #[test]
    fn a_new_dialog_starts_on_the_fetch_button() {
        let mut state = FetchDialogState::new();
        state.open();
        assert_eq!(state.button(), FetchDialogButton::Fetch);
    }

    #[test]
    fn the_button_selection_steps_between_fetch_and_cancel() {
        let mut state = FetchDialogState::new();
        state.open();
        state.select_button_by(1);
        assert_eq!(state.button(), FetchDialogButton::Cancel);
        state.select_button_by(1);
        assert_eq!(state.button(), FetchDialogButton::Fetch);
        state.select_button_by(-1);
        assert_eq!(state.button(), FetchDialogButton::Cancel);
    }

    #[test]
    fn reopening_resets_the_button_selection() {
        let mut state = FetchDialogState::new();
        state.open();
        state.select_button(FetchDialogButton::Cancel);
        state.close();
        state.open();
        assert_eq!(state.button(), FetchDialogButton::Fetch);
    }
}
