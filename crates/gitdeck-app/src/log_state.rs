use std::collections::HashMap;

use gitdeck_core::{assign_lanes, BranchEntry, CommitSummary, LaneRow};
use gitdeck_git::ChangedFile;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CachedCommitBranches {
    Listed(Vec<BranchEntry>),
    Unavailable,
}

pub(crate) struct LogState {
    commits: Vec<CommitSummary>,
    lanes: Vec<LaneRow>,
    selected_index: Option<usize>,
    scroll: usize,
    files_by_commit_id: HashMap<String, Vec<ChangedFile>>,
    branches_by_commit_id: HashMap<String, CachedCommitBranches>,
    lanes_revision: u64,
}

impl LogState {
    pub(crate) fn new() -> Self {
        Self {
            commits: Vec::new(),
            lanes: Vec::new(),
            selected_index: None,
            scroll: 0,
            files_by_commit_id: HashMap::new(),
            branches_by_commit_id: HashMap::new(),
            lanes_revision: 0,
        }
    }

    pub(crate) fn replace_commits(&mut self, commits: Vec<CommitSummary>) {
        self.lanes = assign_lanes(&commits);
        self.lanes_revision = self.lanes_revision.saturating_add(1);
        self.selected_index = (!commits.is_empty()).then_some(0);
        self.commits = commits;
        self.scroll = 0;
        self.files_by_commit_id.clear();
        self.branches_by_commit_id.clear();
    }

    pub(crate) fn append_commits(&mut self, commits: Vec<CommitSummary>) {
        if commits.is_empty() {
            return;
        }
        self.commits.extend(commits);
        self.lanes = assign_lanes(&self.commits);
        self.lanes_revision = self.lanes_revision.saturating_add(1);
    }

    pub(crate) fn commits(&self) -> &[CommitSummary] {
        &self.commits
    }

    pub(crate) fn lanes(&self) -> &[LaneRow] {
        &self.lanes
    }

    #[cfg(test)]
    pub(crate) fn lanes_revision(&self) -> u64 {
        self.lanes_revision
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub(crate) fn selected_commit(&self) -> Option<&CommitSummary> {
        self.selected_index
            .and_then(|index| self.commits.get(index))
    }

    pub(crate) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(crate) fn select_by(&mut self, delta: isize) -> bool {
        let Some(current) = self.selected_index else {
            return false;
        };
        let last_index = self.commits.len().saturating_sub(1);
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

    pub(crate) fn keep_selected_commit_visible(&mut self, visible_rows: usize) {
        let Some(selected_index) = self.selected_index else {
            self.scroll = 0;
            return;
        };
        let lowest_scroll = selected_index
            .saturating_add(1)
            .saturating_sub(visible_rows.max(1));
        self.scroll = self.scroll.clamp(lowest_scroll, selected_index);
    }

    pub(crate) fn scroll_viewport_by(&mut self, delta: isize, visible_rows: usize) -> bool {
        let Some(last_index) = self.commits.len().checked_sub(1) else {
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
        let last_index = self.commits.len().saturating_sub(1);
        let last_visible_index = self
            .scroll
            .saturating_add(visible_rows.max(1))
            .saturating_sub(1)
            .min(last_index);
        self.selected_index = Some(selected_index.clamp(self.scroll, last_visible_index));
    }

    pub(crate) fn commit_index_at_visible_row(&self, row_offset: usize) -> Option<usize> {
        let commit_index = self.scroll.checked_add(row_offset)?;
        (commit_index < self.commits.len()).then_some(commit_index)
    }

    pub(crate) fn selected_commit_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub(crate) fn cached_files(&self, commit_id: &str) -> Option<&[ChangedFile]> {
        self.files_by_commit_id.get(commit_id).map(Vec::as_slice)
    }

    pub(crate) fn cache_files(&mut self, commit_id: String, files: Vec<ChangedFile>) {
        self.files_by_commit_id.insert(commit_id, files);
    }

    pub(crate) fn files_at_selected_commit(&self) -> &[ChangedFile] {
        self.selected_commit()
            .and_then(|commit| self.cached_files(&commit.id))
            .unwrap_or(&[])
    }

    pub(crate) fn selected_commit_branches(&self) -> Option<&CachedCommitBranches> {
        let commit = self.selected_commit()?;
        self.branches_by_commit_id.get(&commit.id)
    }

    pub(crate) fn selected_commit_details_are_cached(&self) -> bool {
        let Some(commit) = self.selected_commit() else {
            return true;
        };
        self.files_by_commit_id.contains_key(&commit.id)
            && self.branches_by_commit_id.contains_key(&commit.id)
    }

    pub(crate) fn commit_branches_query_target(&self) -> Option<String> {
        let commit = self.selected_commit()?;
        (!self.branches_by_commit_id.contains_key(&commit.id)).then(|| commit.id.clone())
    }

    pub(crate) fn cache_commit_branches(
        &mut self,
        commit_id: String,
        branches: CachedCommitBranches,
    ) {
        self.branches_by_commit_id.insert(commit_id, branches);
    }
}

#[cfg(test)]
mod log_state_tests {
    use gitdeck_core::{CommitSummary, RefKind, RefName};
    use gitdeck_git::ChangedFile;

    use super::{CachedCommitBranches, LogState};

    fn commit(index: usize) -> CommitSummary {
        CommitSummary {
            id: format!("{index:040}"),
            short_id: format!("{index:07}"),
            subject: format!("commit {index}"),
            author_name: String::from("gitdeck-fixture"),
            author_email: String::from("fixture@gitdeck.invalid"),
            author_timestamp: 1_577_836_800 + i64::try_from(index).unwrap_or(0),
            committer_name: String::from("gitdeck-fixture"),
            committer_email: String::from("fixture@gitdeck.invalid"),
            committer_timestamp: 1_577_836_800 + i64::try_from(index).unwrap_or(0),
            parent_ids: Vec::new(),
            refs: vec![RefName {
                full: String::from("HEAD"),
                short: String::from("HEAD"),
                kind: RefKind::Head,
            }],
        }
    }

    fn state_with_commits(commit_count: usize) -> LogState {
        let mut state = LogState::new();
        state.replace_commits((0..commit_count).map(commit).collect());
        state
    }

    #[test]
    fn replacing_commits_selects_the_newest_commit_and_clears_the_file_cache() {
        let mut state = LogState::new();
        state.replace_commits(vec![commit(0), commit(1)]);
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(
            state
                .selected_commit()
                .map(|commit| commit.short_id.clone()),
            Some(String::from("0000000"))
        );
        state.cache_files(format!("{:040}", 0), Vec::new());
        assert!(state.cached_files(&format!("{:040}", 0)).is_some());
        state.replace_commits(vec![commit(2)]);
        assert!(state.cached_files(&format!("{:040}", 0)).is_none());
        assert_eq!(state.scroll(), 0);
    }

    #[test]
    fn selection_stops_at_both_ends_and_reports_movement() {
        let mut state = state_with_commits(3);
        assert!(
            !state.select_by(-1),
            "the selection already sits at the top"
        );
        assert_eq!(state.selected_index(), Some(0));
        assert!(state.select_by(2));
        assert_eq!(state.selected_index(), Some(2));
        assert!(!state.select_by(1), "the selection already sits at the end");
        assert_eq!(state.selected_index(), Some(2));
        assert!(!state.select_by(0));
    }

    #[test]
    fn scrolling_keeps_the_selected_commit_inside_the_visible_window() {
        let mut state = state_with_commits(100);
        state.keep_selected_commit_visible(10);
        assert_eq!(state.scroll(), 0);
        state.select_by(50);
        state.keep_selected_commit_visible(10);
        assert_eq!(state.scroll(), 41);
        state.select_by(-45);
        state.keep_selected_commit_visible(10);
        assert_eq!(state.scroll(), 5);
        state.select_by(-10);
        state.keep_selected_commit_visible(10);
        assert_eq!(state.scroll(), 0);
    }

    #[test]
    fn visible_rows_map_to_commits_after_the_scroll_offset() {
        let mut state = state_with_commits(4);
        assert_eq!(state.commit_index_at_visible_row(0), Some(0));
        state.select_by(3);
        state.keep_selected_commit_visible(2);
        assert_eq!(state.scroll(), 2);
        assert_eq!(state.commit_index_at_visible_row(0), Some(2));
        assert_eq!(state.commit_index_at_visible_row(1), Some(3));
        assert_eq!(state.commit_index_at_visible_row(2), None);
    }

    #[test]
    fn empty_history_holds_no_selection_and_no_scroll() {
        let mut state = LogState::new();
        state.replace_commits(Vec::new());
        assert_eq!(state.selected_commit(), None);
        assert!(!state.select_by(1));
        state.keep_selected_commit_visible(10);
        assert_eq!(state.scroll(), 0);
        assert!(state.files_at_selected_commit().is_empty());
        assert!(!state.scroll_viewport_by(3, 10));
        assert_eq!(state.scroll(), 0);
    }

    #[test]
    fn scrolling_the_viewport_keeps_the_selection_inside_the_window() {
        let mut state = state_with_commits(100);
        assert_eq!(state.scroll(), 0);
        assert!(
            state.scroll_viewport_by(3, 10),
            "scrolling past the selected row drags the selection along"
        );
        assert_eq!(state.scroll(), 3);
        assert_eq!(state.selected_index(), Some(3));
        state.select_by(2);
        assert!(
            !state.scroll_viewport_by(1, 10),
            "scrolling inside the window keeps the selected commit"
        );
        assert_eq!(state.scroll(), 4);
        assert_eq!(state.selected_index(), Some(5));
        state.scroll_viewport_by(200, 10);
        assert_eq!(
            state.scroll(),
            99,
            "the viewport offset stops at the oldest commit"
        );
        assert_eq!(state.selected_index(), Some(99));
        state.scroll_viewport_by(-200, 10);
        assert_eq!(
            state.scroll(),
            0,
            "the viewport offset stops at the newest commit"
        );
        assert_eq!(state.selected_index(), Some(9));
    }

    #[test]
    fn scrolling_does_not_rebuild_the_graph_on_every_event() {
        let mut state = state_with_commits(200);
        let revision = state.lanes_revision();
        for _ in 0..40 {
            state.scroll_viewport_by(3, 20);
            state.select_by(1);
            state.keep_selected_commit_visible(20);
        }
        assert_eq!(
            state.lanes_revision(),
            revision,
            "scrolling and selecting reuse the graph layout"
        );
        state.append_commits(vec![commit(200)]);
        assert_eq!(
            state.lanes_revision(),
            revision + 1,
            "appending a page rebuilds the graph once"
        );
        state.replace_commits(vec![commit(0)]);
        assert_eq!(
            state.lanes_revision(),
            revision + 2,
            "a reload rebuilds the graph once"
        );
    }

    #[test]
    fn lanes_follow_the_loaded_commits() {
        let mut state = state_with_commits(3);
        assert_eq!(state.lanes().len(), 3);
        assert!(
            state.lanes().iter().all(|row| row.commit_lane == 0),
            "unrelated roots reuse the first lane"
        );
        state.replace_commits(Vec::new());
        assert!(state.lanes().is_empty());
    }

    fn query_selected_commit_branches(state: &mut LogState, queries: &mut Vec<String>) {
        let Some(commit_id) = state.commit_branches_query_target() else {
            return;
        };
        queries.push(commit_id.clone());
        state.cache_commit_branches(commit_id, CachedCommitBranches::Listed(Vec::new()));
    }

    #[test]
    fn branches_query_runs_once_per_selection_change() {
        let mut state = state_with_commits(3);
        let mut queries: Vec<String> = Vec::new();
        for _ in 0..4 {
            query_selected_commit_branches(&mut state, &mut queries);
        }
        assert_eq!(queries, vec![format!("{:040}", 0)], "four redraws ask once");
        state.select_by(1);
        for _ in 0..3 {
            query_selected_commit_branches(&mut state, &mut queries);
        }
        assert_eq!(
            queries,
            vec![format!("{:040}", 0), format!("{:040}", 1)],
            "the selection change asks for the new commit only"
        );
        state.select_by(-1);
        for _ in 0..3 {
            query_selected_commit_branches(&mut state, &mut queries);
        }
        assert_eq!(
            queries.len(),
            2,
            "returning to a cached commit asks nothing, queries {queries:?}"
        );
        assert_eq!(
            state.selected_commit_branches(),
            Some(&CachedCommitBranches::Listed(Vec::new()))
        );
        state.replace_commits(vec![commit(0)]);
        assert_eq!(
            state.selected_commit_branches(),
            None,
            "a reloaded log drops the cached branches"
        );
    }

    #[test]
    fn cached_files_are_returned_only_for_the_selected_commit() {
        let mut state = state_with_commits(2);
        let newest_files = vec![ChangedFile {
            path: String::from("src/app.rs"),
            previous_path: None,
            kind: gitdeck_core::ChangeKind::Modified,
        }];
        state.cache_files(format!("{:040}", 0), newest_files.clone());
        assert_eq!(state.files_at_selected_commit(), newest_files.as_slice());
        state.select_by(1);
        assert!(state.files_at_selected_commit().is_empty());
    }
}
