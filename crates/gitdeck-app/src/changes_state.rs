use std::collections::BTreeSet;

use gitdeck_core::{ChangeEntry, ChangeKind};
use gitdeck_tui::{
    changes_rows, default_collapsed_groups, group_member_indices, ChangesGroup, ChangesRow,
};

pub struct ChangesState {
    entries: Vec<ChangeEntry>,
    collapsed_groups: BTreeSet<ChangesGroup>,
    checked_paths: BTreeSet<String>,
    selected_row: usize,
}

impl Default for ChangesState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangesState {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            collapsed_groups: default_collapsed_groups(),
            checked_paths: BTreeSet::new(),
            selected_row: 0,
        }
    }

    pub fn entries(&self) -> &[ChangeEntry] {
        &self.entries
    }

    pub fn rows(&self) -> Vec<ChangesRow> {
        changes_rows(&self.entries, &self.collapsed_groups, &self.checked_paths)
    }

    pub fn selected_row(&self) -> usize {
        self.selected_row
    }

    pub fn row_at(&self, row_index: usize) -> Option<ChangesRow> {
        self.rows().get(row_index).copied()
    }

    pub fn selected_file_path(&self) -> Option<&str> {
        match self.row_at(self.selected_row)? {
            ChangesRow::File { index, .. } => {
                self.entries.get(index).map(|entry| entry.path.as_str())
            }
            ChangesRow::Group { .. } => None,
        }
    }

    pub fn selected_untracked_path(&self) -> Option<&str> {
        let ChangesRow::File { index, .. } = self.row_at(self.selected_row)? else {
            return None;
        };
        let entry = self.entries.get(index)?;
        (entry.kind == ChangeKind::Untracked).then_some(entry.path.as_str())
    }

    pub fn selected_conflicted_path(&self) -> Option<&str> {
        let ChangesRow::File { index, .. } = self.row_at(self.selected_row)? else {
            return None;
        };
        let entry = self.entries.get(index)?;
        entry.conflict.is_some().then_some(entry.path.as_str())
    }

    pub fn checked_file_paths(&self) -> Vec<String> {
        let mut seen_paths: BTreeSet<&str> = BTreeSet::new();
        self.entries
            .iter()
            .filter(|entry| self.checked_paths.contains(&entry.path))
            .filter(|entry| seen_paths.insert(entry.path.as_str()))
            .map(|entry| entry.path.clone())
            .collect()
    }

    pub fn is_path_checked(&self, path: &str) -> bool {
        self.checked_paths.contains(path)
    }

    pub fn checked_untracked_paths(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == ChangeKind::Untracked)
            .filter(|entry| self.checked_paths.contains(&entry.path))
            .map(|entry| entry.path.clone())
            .collect()
    }

    pub fn select_row(&mut self, row_index: usize) {
        self.selected_row = row_index;
        self.clamp_selected_row();
    }

    pub fn move_selection(&mut self, delta: isize) {
        let row_count = self.rows().len();
        if row_count == 0 {
            self.selected_row = 0;
            return;
        }
        let current_row = isize::try_from(self.selected_row).unwrap_or(isize::MAX);
        let last_row = isize::try_from(row_count - 1).unwrap_or(0);
        let target_row = current_row.saturating_add(delta).clamp(0, last_row);
        self.selected_row = usize::try_from(target_row).unwrap_or(0);
    }

    pub fn toggle_checked_at_cursor(&mut self) {
        self.toggle_checked_at_row(self.selected_row);
    }

    pub fn toggle_checked_at_row(&mut self, row_index: usize) {
        match self.row_at(row_index) {
            Some(ChangesRow::Group { group, .. }) => self.toggle_group_checked(group),
            Some(ChangesRow::File { index, checked, .. }) => {
                self.set_entry_checked(index, !checked);
            }
            None => {}
        }
    }

    pub fn toggle_group_collapsed(&mut self, group: ChangesGroup) {
        if !self.collapsed_groups.remove(&group) {
            self.collapsed_groups.insert(group);
        }
        self.clamp_selected_row();
    }

    pub fn set_selected_group_collapsed(&mut self, collapsed: bool) {
        let group = match self.row_at(self.selected_row) {
            Some(ChangesRow::Group { group, .. }) => group,
            Some(ChangesRow::File { group, .. }) => group,
            None => return,
        };
        if collapsed {
            self.collapsed_groups.insert(group);
            if let Some(header_row) = self.group_header_row(group) {
                self.selected_row = header_row;
            }
        } else {
            self.collapsed_groups.remove(&group);
        }
        self.clamp_selected_row();
    }

    fn group_header_row(&self, group: ChangesGroup) -> Option<usize> {
        self.rows().iter().position(
            |row| matches!(row, ChangesRow::Group { group: row_group, .. } if *row_group == group),
        )
    }

    pub fn replace_entries(&mut self, entries: Vec<ChangeEntry>) {
        self.checked_paths = entries.iter().map(|entry| entry.path.clone()).collect();
        self.entries = entries;
        self.clamp_selected_row();
    }

    fn toggle_group_checked(&mut self, group: ChangesGroup) {
        let member_indices = group_member_indices(&self.entries, group);
        let all_checked = member_indices
            .iter()
            .all(|index| self.checked_paths.contains(&self.entries[*index].path));
        for index in member_indices {
            self.set_entry_checked(index, !all_checked);
        }
    }

    fn set_entry_checked(&mut self, entry_index: usize, checked: bool) {
        let Some(entry) = self.entries.get(entry_index) else {
            return;
        };
        let path = entry.path.clone();
        if checked {
            self.checked_paths.insert(path);
        } else {
            self.checked_paths.remove(&path);
        }
    }

    fn clamp_selected_row(&mut self) {
        let row_count = self.rows().len();
        self.selected_row = self.selected_row.min(row_count.saturating_sub(1));
    }
}

#[cfg(test)]
mod changes_state_tests {
    use gitdeck_core::{ChangeEntry, ChangeKind};
    use gitdeck_tui::ChangesGroup;

    use super::ChangesState;

    fn entry(path: &str, kind: ChangeKind) -> ChangeEntry {
        ChangeEntry::unstaged(path.to_string(), None, kind)
    }

    fn dirty_state() -> ChangesState {
        let mut state = ChangesState::new();
        state.replace_entries(vec![
            entry("docs/readme.md", ChangeKind::Deleted),
            entry("src/app.rs", ChangeKind::Modified),
            entry("notes.txt", ChangeKind::Untracked),
        ]);
        state
    }

    #[test]
    fn new_state_checks_every_entry_and_expands_every_group() {
        let state = dirty_state();
        assert_eq!(
            state.checked_file_paths(),
            vec![
                String::from("docs/readme.md"),
                String::from("src/app.rs"),
                String::from("notes.txt"),
            ]
        );
        assert_eq!(state.rows().len(), 5, "two group rows plus three files");
    }

    #[test]
    fn selection_walks_visible_rows_and_stops_at_the_last_one() {
        let mut state = dirty_state();
        assert_eq!(state.selected_file_path(), None);
        state.move_selection(1);
        assert_eq!(state.selected_file_path(), Some("docs/readme.md"));
        state.move_selection(1);
        assert_eq!(state.selected_file_path(), Some("src/app.rs"));
        state.move_selection(1);
        assert_eq!(state.selected_file_path(), None);
        state.move_selection(1);
        assert_eq!(state.selected_file_path(), Some("notes.txt"));
        assert_eq!(state.selected_row(), 4);
        state.move_selection(-3);
        assert_eq!(state.selected_row(), 1);
        state.move_selection(-1);
        assert_eq!(state.selected_row(), 0);
        state.move_selection(-1);
        assert_eq!(state.selected_row(), 0);
    }

    #[test]
    fn toggling_a_group_row_clears_and_restores_every_child() {
        let mut state = dirty_state();
        state.toggle_checked_at_cursor();
        assert_eq!(state.checked_file_paths(), vec![String::from("notes.txt")]);
        state.toggle_checked_at_cursor();
        assert_eq!(state.checked_file_paths().len(), 3);
    }

    #[test]
    fn toggling_a_file_row_checks_only_that_file() {
        let mut state = dirty_state();
        state.select_row(1);
        state.toggle_checked_at_cursor();
        assert_eq!(
            state.checked_file_paths(),
            vec![String::from("src/app.rs"), String::from("notes.txt")]
        );
        state.toggle_checked_at_cursor();
        assert_eq!(state.checked_file_paths().len(), 3);
    }

    #[test]
    fn collapsing_the_selected_group_hides_its_children_and_clamps_the_cursor() {
        let mut state = dirty_state();
        state.select_row(2);
        state.set_selected_group_collapsed(true);
        assert_eq!(state.selected_row(), 0);
        assert_eq!(state.rows().len(), 3);
        state.set_selected_group_collapsed(false);
        assert_eq!(state.rows().len(), 5);
    }

    #[test]
    fn toggling_a_group_header_collapse_flips_its_group_only() {
        let mut state = dirty_state();
        state.toggle_group_collapsed(ChangesGroup::Changes);
        assert_eq!(state.rows().len(), 3);
        state.toggle_group_collapsed(ChangesGroup::Changes);
        assert_eq!(state.rows().len(), 5);
    }

    #[test]
    fn refreshing_checks_every_remaining_entry() {
        let mut state = dirty_state();
        state.select_row(1);
        state.toggle_checked_at_cursor();
        assert_eq!(
            state.checked_file_paths(),
            vec![String::from("src/app.rs"), String::from("notes.txt")]
        );
        state.replace_entries(vec![
            entry("docs/readme.md", ChangeKind::Deleted),
            entry("notes.txt", ChangeKind::Untracked),
        ]);
        assert_eq!(
            state.checked_file_paths(),
            vec![String::from("docs/readme.md"), String::from("notes.txt")],
            "a refreshed list starts with every remaining entry checked"
        );
    }

    #[test]
    fn entry_without_slashes_has_no_directory() {
        let mut state = ChangesState::new();
        state.replace_entries(vec![entry("notes.txt", ChangeKind::Untracked)]);
        assert_eq!(state.checked_file_paths(), vec![String::from("notes.txt")]);
        assert_eq!(state.selected_file_path(), None);
    }
}
