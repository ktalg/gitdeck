use gitdeck_tui::BRANCH_FILTER_DROPDOWN_MAX_VISIBLE_ROWS;

pub(crate) struct BranchFilterDropdownState {
    open: bool,
    selected_index: usize,
    scroll: usize,
}

impl BranchFilterDropdownState {
    pub(crate) const fn new() -> Self {
        Self {
            open: false,
            selected_index: 0,
            scroll: 0,
        }
    }

    pub(crate) fn open(&mut self, selected_index: usize) {
        self.open = true;
        self.selected_index = selected_index;
        self.scroll = 0;
        self.reveal_selected_row();
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) const fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub(crate) const fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn select_by(&mut self, delta: isize, item_count: usize) {
        let Some(last_index) = item_count.checked_sub(1) else {
            return;
        };
        let last_index = isize::try_from(last_index).unwrap_or(isize::MAX);
        let current_index = isize::try_from(self.selected_index).unwrap_or(isize::MAX);
        let target_index = current_index.saturating_add(delta).clamp(0, last_index);
        self.selected_index = usize::try_from(target_index).unwrap_or(0);
        self.reveal_selected_row();
    }

    pub(crate) fn select_index(&mut self, index: usize) {
        self.selected_index = index;
        self.reveal_selected_row();
    }

    fn reveal_selected_row(&mut self) {
        if self.selected_index < self.scroll {
            self.scroll = self.selected_index;
            return;
        }
        let first_hidden_row = self
            .scroll
            .saturating_add(BRANCH_FILTER_DROPDOWN_MAX_VISIBLE_ROWS);
        if self.selected_index >= first_hidden_row {
            self.scroll = self
                .selected_index
                .saturating_add(1)
                .saturating_sub(BRANCH_FILTER_DROPDOWN_MAX_VISIBLE_ROWS);
        }
    }
}

#[cfg(test)]
mod branch_filter_dropdown_state_tests {
    use super::BranchFilterDropdownState;

    #[test]
    fn the_selection_stops_at_the_first_and_last_entry() {
        let mut state = BranchFilterDropdownState::new();
        state.open(1);
        state.select_by(-5, 3);
        assert_eq!(state.selected_index(), 0);
        state.select_by(5, 3);
        assert_eq!(state.selected_index(), 2);
        state.select_by(1, 0);
        assert_eq!(
            state.selected_index(),
            2,
            "an empty list keeps the selection"
        );
    }

    #[test]
    fn the_scroll_follows_the_selection_past_the_visible_rows() {
        let mut state = BranchFilterDropdownState::new();
        state.open(0);
        state.select_by(12, 15);
        assert_eq!(state.selected_index(), 12);
        assert_eq!(state.scroll(), 3);
        state.select_by(-12, 15);
        assert_eq!(state.scroll(), 0);
    }
}
