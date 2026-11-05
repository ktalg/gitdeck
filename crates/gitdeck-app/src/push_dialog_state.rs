use gitdeck_tui::{PushDialogButton, PushDialogFocus};

const PUSH_DIALOG_FOCUS_ORDER: [PushDialogFocus; 3] = [
    PushDialogFocus::SetUpstream,
    PushDialogFocus::ForceWithLease,
    PushDialogFocus::Buttons,
];

const PUSH_DIALOG_BUTTON_ORDER: [PushDialogButton; 2] =
    [PushDialogButton::Push, PushDialogButton::Cancel];

pub struct PushDialogState {
    open: bool,
    focus: PushDialogFocus,
    button: PushDialogButton,
    set_upstream: bool,
    force_with_lease: bool,
}

impl PushDialogState {
    pub const fn new() -> Self {
        Self {
            open: false,
            focus: PushDialogFocus::SetUpstream,
            button: PushDialogButton::Push,
            set_upstream: false,
            force_with_lease: false,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.focus = PushDialogFocus::SetUpstream;
        self.button = PushDialogButton::Push;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn focus(&self) -> PushDialogFocus {
        self.focus
    }

    pub const fn button(&self) -> PushDialogButton {
        self.button
    }

    pub const fn set_upstream(&self) -> bool {
        self.set_upstream
    }

    pub const fn force_with_lease(&self) -> bool {
        self.force_with_lease
    }

    pub fn focus_by(&mut self, step: isize) {
        self.focus = step_through(&PUSH_DIALOG_FOCUS_ORDER, self.focus, step);
    }

    pub fn select_button(&mut self, button: PushDialogButton) {
        self.button = button;
        self.focus = PushDialogFocus::Buttons;
    }

    pub fn select_button_by(&mut self, step: isize) {
        self.button = step_through(&PUSH_DIALOG_BUTTON_ORDER, self.button, step);
    }

    pub fn select_option(&mut self, focus: PushDialogFocus) {
        self.focus = focus;
        self.toggle_focused_option();
    }

    pub fn toggle_focused_option(&mut self) {
        match self.focus {
            PushDialogFocus::SetUpstream => self.set_upstream = !self.set_upstream,
            PushDialogFocus::ForceWithLease => self.force_with_lease = !self.force_with_lease,
            PushDialogFocus::Buttons => {}
        }
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
mod push_dialog_state_tests {
    use gitdeck_tui::{PushDialogButton, PushDialogFocus};

    use super::PushDialogState;

    #[test]
    fn focus_steps_through_the_two_options_and_the_button_row() {
        let mut state = PushDialogState::new();
        state.open();
        assert_eq!(state.focus(), PushDialogFocus::SetUpstream);
        state.focus_by(1);
        assert_eq!(state.focus(), PushDialogFocus::ForceWithLease);
        state.focus_by(1);
        assert_eq!(state.focus(), PushDialogFocus::Buttons);
        state.focus_by(1);
        assert_eq!(state.focus(), PushDialogFocus::SetUpstream);
        state.focus_by(-1);
        assert_eq!(state.focus(), PushDialogFocus::Buttons);
    }

    #[test]
    fn only_the_focused_option_toggles() {
        let mut state = PushDialogState::new();
        state.open();
        state.toggle_focused_option();
        assert!(state.set_upstream());
        assert!(!state.force_with_lease());
        state.focus_by(1);
        state.toggle_focused_option();
        assert!(state.force_with_lease());
        state.focus_by(1);
        state.toggle_focused_option();
        assert!(state.set_upstream(), "the button row keeps the options");
        assert!(state.force_with_lease(), "the button row keeps the options");
    }

    #[test]
    fn reopening_resets_the_focus_but_keeps_the_options() {
        let mut state = PushDialogState::new();
        state.open();
        state.toggle_focused_option();
        state.select_button(PushDialogButton::Cancel);
        state.close();
        assert!(!state.is_open());
        state.open();
        assert_eq!(state.focus(), PushDialogFocus::SetUpstream);
        assert_eq!(state.button(), PushDialogButton::Push);
        assert!(state.set_upstream());
    }

    #[test]
    fn the_button_selection_steps_between_push_and_cancel() {
        let mut state = PushDialogState::new();
        state.open();
        state.select_button_by(1);
        assert_eq!(state.button(), PushDialogButton::Cancel);
        state.select_button_by(1);
        assert_eq!(state.button(), PushDialogButton::Push);
        state.select_button_by(-1);
        assert_eq!(state.button(), PushDialogButton::Cancel);
    }
}
