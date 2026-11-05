use gitdeck_core::BranchEntry;

pub(crate) struct BranchState {
    branches: Vec<BranchEntry>,
    selected_index: Option<usize>,
    scroll: usize,
}

impl BranchState {
    pub(crate) fn new() -> Self {
        Self {
            branches: Vec::new(),
            selected_index: None,
            scroll: 0,
        }
    }

    pub(crate) fn replace_branches(&mut self, branches: Vec<BranchEntry>) {
        let selected_name = self
            .selected_branch()
            .map(|branch| branch.name.clone())
            .or_else(|| {
                branches
                    .iter()
                    .find(|branch| branch.is_head)
                    .map(|branch| branch.name.clone())
            });
        self.selected_index = selected_name
            .and_then(|name| branches.iter().position(|branch| branch.name == name))
            .or_else(|| (!branches.is_empty()).then_some(0));
        self.branches = branches;
        self.scroll = 0;
    }

    pub(crate) fn branches(&self) -> &[BranchEntry] {
        &self.branches
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub(crate) fn selected_branch(&self) -> Option<&BranchEntry> {
        self.selected_index
            .and_then(|index| self.branches.get(index))
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn select_by(&mut self, delta: isize) -> bool {
        let Some(current) = self.selected_index else {
            return false;
        };
        let last_index = self.branches.len().saturating_sub(1);
        let last_index = isize::try_from(last_index).unwrap_or(isize::MAX);
        let current_index = isize::try_from(current).unwrap_or(isize::MAX);
        let target_index = current_index.saturating_add(delta).clamp(0, last_index);
        let target_index = usize::try_from(target_index).unwrap_or(0);
        if target_index == current {
            return false;
        }
        self.selected_index = Some(target_index);
        true
    }

    pub(crate) fn keep_selected_branch_visible(&mut self, visible_rows: usize) {
        let Some(selected_index) = self.selected_index else {
            self.scroll = 0;
            return;
        };
        if visible_rows == 0 {
            return;
        }
        if selected_index < self.scroll {
            self.scroll = selected_index;
            return;
        }
        let first_hidden_index = self.scroll.saturating_add(visible_rows);
        if selected_index >= first_hidden_index {
            self.scroll = selected_index
                .saturating_add(1)
                .saturating_sub(visible_rows);
        }
    }

    pub(crate) fn scroll_viewport_by(&mut self, delta: isize, visible_rows: usize) -> bool {
        let Some(last_index) = self.branches.len().checked_sub(1) else {
            self.scroll = 0;
            return false;
        };
        let last_index = isize::try_from(last_index).unwrap_or(isize::MAX);
        let current_scroll = isize::try_from(self.scroll).unwrap_or(isize::MAX);
        let target_scroll = current_scroll.saturating_add(delta).clamp(0, last_index);
        let scroll = usize::try_from(target_scroll).unwrap_or(0);
        if scroll == self.scroll {
            return false;
        }
        self.scroll = scroll;
        let selection_before = self.selected_index;
        self.keep_selection_inside_viewport(visible_rows);
        selection_before != self.selected_index
    }

    fn keep_selection_inside_viewport(&mut self, visible_rows: usize) {
        let Some(selected_index) = self.selected_index else {
            return;
        };
        let last_index = self.branches.len().saturating_sub(1);
        let last_visible_index = self
            .scroll
            .saturating_add(visible_rows.max(1))
            .saturating_sub(1)
            .min(last_index);
        self.selected_index = Some(selected_index.clamp(self.scroll, last_visible_index));
    }

    pub(crate) fn branch_index_at_visible_row(&self, row_offset: usize) -> Option<usize> {
        let branch_index = self.scroll.checked_add(row_offset)?;
        (branch_index < self.branches.len()).then_some(branch_index)
    }

    pub(crate) fn select_branch_named(&mut self, name: &str) -> bool {
        let Some(index) = self.branches.iter().position(|branch| branch.name == name) else {
            return false;
        };
        if self.selected_index == Some(index) {
            return false;
        }
        self.selected_index = Some(index);
        true
    }

    pub(crate) fn select_index(&mut self, index: usize) {
        if index < self.branches.len() {
            self.selected_index = Some(index);
        }
    }
}

#[cfg(test)]
mod branch_state_tests {
    use gitdeck_core::{BranchEntry, RefKind};

    use super::BranchState;

    fn branch(name: &str, is_head: bool) -> BranchEntry {
        BranchEntry {
            name: name.to_string(),
            kind: RefKind::LocalBranch,
            is_head,
            upstream: None,
        }
    }

    fn state_with_branches(names: &[(&str, bool)]) -> BranchState {
        let mut state = BranchState::new();
        state.replace_branches(
            names
                .iter()
                .map(|(name, is_head)| branch(name, *is_head))
                .collect(),
        );
        state
    }

    #[test]
    fn a_new_list_selects_the_head_branch() {
        let state = state_with_branches(&[("feature", false), ("main", true)]);
        assert_eq!(
            state.selected_branch().map(|branch| branch.name.clone()),
            Some(String::from("main"))
        );
        assert_eq!(state.scroll(), 0);
    }

    #[test]
    fn reloading_keeps_the_selected_branch_by_name() {
        let mut state = state_with_branches(&[("feature", false), ("main", true)]);
        state.select_by(1);
        state.replace_branches(vec![branch("feature", false), branch("main", true)]);
        assert_eq!(
            state.selected_branch().map(|branch| branch.name.clone()),
            Some(String::from("main"))
        );
    }

    #[test]
    fn selection_stops_at_both_ends_and_reports_movement() {
        let mut state = state_with_branches(&[("a", true), ("b", false), ("c", false)]);
        assert!(!state.select_by(-1));
        assert!(state.select_by(5));
        assert_eq!(state.selected_index(), Some(2));
        assert!(!state.select_by(1));
    }

    #[test]
    fn the_viewport_keeps_the_selection_visible_and_stops_at_the_ends() {
        let names: Vec<(String, bool)> = (0..30)
            .map(|index| (format!("branch{index}"), index == 0))
            .collect();
        let names: Vec<(&str, bool)> = names
            .iter()
            .map(|(name, is_head)| (name.as_str(), *is_head))
            .collect();
        let mut state = state_with_branches(&names);
        state.select_by(15);
        state.keep_selected_branch_visible(5);
        assert_eq!(state.scroll(), 11);
        state.scroll_viewport_by(3, 5);
        assert_eq!(state.scroll(), 14);
        assert_eq!(
            state.selected_index(),
            Some(15),
            "the selected branch stays inside the scrolled window"
        );
        state.scroll_viewport_by(200, 5);
        assert_eq!(state.scroll(), 29);
        assert_eq!(
            state.selected_index(),
            Some(29),
            "the selection follows the viewport to the last branch"
        );
        state.scroll_viewport_by(-200, 5);
        assert_eq!(state.scroll(), 0);
        assert_eq!(state.selected_index(), Some(4));
    }

    #[test]
    fn visible_rows_map_to_branches_after_the_scroll_offset() {
        let mut state =
            state_with_branches(&[("a", true), ("b", false), ("c", false), ("d", false)]);
        assert_eq!(state.branch_index_at_visible_row(0), Some(0));
        state.scroll_viewport_by(2, 2);
        assert_eq!(state.branch_index_at_visible_row(0), Some(2));
        assert_eq!(state.branch_index_at_visible_row(2), None);
    }

    #[test]
    fn an_empty_list_holds_no_selection() {
        let mut state = BranchState::new();
        state.replace_branches(Vec::new());
        assert_eq!(state.selected_index(), None);
        assert!(!state.select_by(1));
        assert!(!state.scroll_viewport_by(3, 4));
        assert_eq!(state.scroll(), 0);
        assert_eq!(state.branch_index_at_visible_row(0), None);
    }

    #[test]
    fn selecting_an_out_of_range_index_keeps_the_current_selection() {
        let mut state = state_with_branches(&[("a", true), ("b", false)]);
        state.select_index(9);
        assert_eq!(state.selected_index(), Some(0));
        state.select_index(1);
        assert_eq!(state.selected_index(), Some(1));
    }
}
