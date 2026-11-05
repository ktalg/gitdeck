#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HelpOverlayState {
    open: bool,
    scroll: usize,
}

impl HelpOverlayState {
    pub const fn new() -> Self {
        Self {
            open: false,
            scroll: 0,
        }
    }

    pub const fn is_open(self) -> bool {
        self.open
    }

    pub const fn scroll(self) -> usize {
        self.scroll
    }

    pub fn toggle(&mut self) {
        if self.open {
            self.close();
            return;
        }
        self.open = true;
        self.scroll = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.scroll = 0;
    }

    pub fn scroll_by(&mut self, delta: isize, max_scroll: usize) {
        if !self.open {
            return;
        }
        let current = isize::try_from(self.scroll).unwrap_or(isize::MAX);
        let next = current
            .saturating_add(delta)
            .clamp(0, isize::try_from(max_scroll).unwrap_or(isize::MAX));
        self.scroll = usize::try_from(next).unwrap_or(0);
    }
}

#[cfg(test)]
mod help_overlay_state_tests {
    use super::HelpOverlayState;

    #[test]
    fn toggling_opens_at_the_first_row_and_closes_again() {
        let mut state = HelpOverlayState::new();
        assert!(!state.is_open());
        state.toggle();
        assert!(state.is_open());
        assert_eq!(state.scroll(), 0);
        state.scroll_by(3, 10);
        assert_eq!(state.scroll(), 3);
        state.toggle();
        assert!(!state.is_open());
        assert_eq!(state.scroll(), 0, "closing forgets the scroll offset");
        state.toggle();
        assert_eq!(state.scroll(), 0, "reopening starts at the first row");
    }

    #[test]
    fn scrolling_stays_between_the_first_row_and_the_last_page() {
        let mut state = HelpOverlayState::new();
        state.toggle();
        state.scroll_by(-4, 6);
        assert_eq!(state.scroll(), 0);
        state.scroll_by(100, 6);
        assert_eq!(state.scroll(), 6);
        state.scroll_by(-2, 6);
        assert_eq!(state.scroll(), 4);
    }

    #[test]
    fn a_closed_overlay_ignores_scroll_requests() {
        let mut state = HelpOverlayState::new();
        state.scroll_by(2, 10);
        assert_eq!(state.scroll(), 0);
    }
}
