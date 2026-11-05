use gitdeck_tui::{PullDialogButton, PullDialogFocus, PullStrategyOption, PULL_STRATEGY_OPTIONS};

const PULL_DIALOG_BUTTON_ORDER: [PullDialogButton; 2] =
    [PullDialogButton::Pull, PullDialogButton::Cancel];
const PULL_DIALOG_FOCUS_ORDER: [PullDialogFocus; 2] =
    [PullDialogFocus::Strategy, PullDialogFocus::Buttons];

pub struct PullDialogState {
    open: bool,
    strategy: PullStrategyOption,
    focus: PullDialogFocus,
    button: PullDialogButton,
}

impl PullDialogState {
    pub const fn new() -> Self {
        Self {
            open: false,
            strategy: PullStrategyOption::FastForwardOnly,
            focus: PullDialogFocus::Strategy,
            button: PullDialogButton::Pull,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.focus = PullDialogFocus::Strategy;
        self.button = PullDialogButton::Pull;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub const fn strategy(&self) -> PullStrategyOption {
        self.strategy
    }

    pub const fn focus(&self) -> PullDialogFocus {
        self.focus
    }

    pub const fn button(&self) -> PullDialogButton {
        self.button
    }

    pub fn select_strategy(&mut self, strategy: PullStrategyOption) {
        self.strategy = strategy;
        self.focus = PullDialogFocus::Strategy;
    }

    pub fn step_strategy_by(&mut self, step: isize) {
        self.select_strategy(step_through(&PULL_STRATEGY_OPTIONS, self.strategy, step));
    }

    pub fn focus_by(&mut self, step: isize) {
        self.focus = step_through(&PULL_DIALOG_FOCUS_ORDER, self.focus, step);
    }

    pub fn select_button(&mut self, button: PullDialogButton) {
        self.button = button;
        self.focus = PullDialogFocus::Buttons;
    }

    pub fn select_button_by(&mut self, step: isize) {
        self.button = step_through(&PULL_DIALOG_BUTTON_ORDER, self.button, step);
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
mod pull_dialog_state_tests {
    use gitdeck_tui::{PullDialogButton, PullDialogFocus, PullStrategyOption};

    use super::PullDialogState;

    #[test]
    fn a_new_dialog_starts_on_fast_forward_only() {
        let mut state = PullDialogState::new();
        state.open();
        assert_eq!(state.strategy(), PullStrategyOption::FastForwardOnly);
        assert_eq!(state.focus(), PullDialogFocus::Strategy);
        assert_eq!(state.button(), PullDialogButton::Pull);
    }

    #[test]
    fn the_strategy_steps_through_the_three_options_and_wraps() {
        let mut state = PullDialogState::new();
        state.open();
        state.step_strategy_by(1);
        assert_eq!(state.strategy(), PullStrategyOption::Merge);
        state.step_strategy_by(1);
        assert_eq!(state.strategy(), PullStrategyOption::Rebase);
        state.step_strategy_by(1);
        assert_eq!(state.strategy(), PullStrategyOption::FastForwardOnly);
        state.step_strategy_by(-1);
        assert_eq!(state.strategy(), PullStrategyOption::Rebase);
        assert_eq!(state.focus(), PullDialogFocus::Strategy);
    }

    #[test]
    fn the_focus_steps_between_the_strategy_list_and_the_button_row() {
        let mut state = PullDialogState::new();
        state.open();
        state.focus_by(1);
        assert_eq!(state.focus(), PullDialogFocus::Buttons);
        state.focus_by(1);
        assert_eq!(state.focus(), PullDialogFocus::Strategy);
        state.focus_by(-1);
        assert_eq!(state.focus(), PullDialogFocus::Buttons);
    }

    #[test]
    fn reopening_resets_the_focus_and_the_button_row_only() {
        let mut state = PullDialogState::new();
        state.open();
        state.select_strategy(PullStrategyOption::Rebase);
        state.select_button(PullDialogButton::Cancel);
        state.close();
        state.open();
        assert_eq!(state.focus(), PullDialogFocus::Strategy);
        assert_eq!(state.button(), PullDialogButton::Pull);
        assert_eq!(state.strategy(), PullStrategyOption::Rebase);
    }
}
