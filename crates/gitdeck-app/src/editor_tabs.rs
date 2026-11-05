use gitdeck_git::FileDiff;
use gitdeck_tui::{LOG_TAB_TITLE, WORKING_TREE_TAB_SUFFIX};

const PATH_SEPARATOR: char = '/';

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum EditorTab {
    Log,
    Diff(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffOrigin {
    WorkingTree,
    Commit,
}

struct OpenDiff {
    path: String,
    origin: DiffOrigin,
    diff: FileDiff,
}

pub struct EditorTabs {
    open_diffs: Vec<OpenDiff>,
    active_index: Option<usize>,
}

impl Default for EditorTabs {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorTabs {
    pub fn new() -> Self {
        Self {
            open_diffs: Vec::new(),
            active_index: None,
        }
    }

    pub fn active_tab(&self) -> EditorTab {
        match self.active_diff() {
            Some(open_diff) => EditorTab::Diff(open_diff.path.clone()),
            None => EditorTab::Log,
        }
    }

    pub fn active_index(&self) -> usize {
        match self.active_index {
            Some(index) => index.saturating_add(1),
            None => 0,
        }
    }

    pub fn is_open(&self, path: &str) -> bool {
        self.open_diffs
            .iter()
            .any(|open_diff| open_diff.path == path)
    }

    pub fn open(&mut self, path: &str, origin: DiffOrigin, diff: FileDiff) {
        match self.index_of(path) {
            Some(index) => self.active_index = Some(index),
            None => {
                self.open_diffs.push(OpenDiff {
                    path: path.to_string(),
                    origin,
                    diff,
                });
                self.active_index = Some(self.open_diffs.len().saturating_sub(1));
            }
        }
    }

    pub fn activate(&mut self, path: &str) {
        if let Some(index) = self.index_of(path) {
            self.active_index = Some(index);
        }
    }

    pub fn activate_log(&mut self) {
        self.active_index = None;
    }

    pub fn activate_diff_at(&mut self, diff_index: usize) {
        if diff_index < self.open_diffs.len() {
            self.active_index = Some(diff_index);
        }
    }

    pub fn close_diff_at(&mut self, diff_index: usize) {
        if diff_index >= self.open_diffs.len() {
            return;
        }
        self.open_diffs.remove(diff_index);
        self.active_index = match self.active_index {
            Some(active) if active == diff_index => None,
            Some(active) if active > diff_index => Some(active.saturating_sub(1)),
            other => other,
        };
    }

    pub fn close_active(&mut self) {
        let Some(index) = self.active_index.take() else {
            return;
        };
        if index < self.open_diffs.len() {
            self.open_diffs.remove(index);
        }
    }

    pub fn file_diff(&self) -> Option<&FileDiff> {
        self.active_diff().map(|open_diff| &open_diff.diff)
    }

    pub fn active_origin(&self) -> Option<DiffOrigin> {
        self.active_diff().map(|open_diff| open_diff.origin)
    }

    pub fn active_path(&self) -> Option<&str> {
        self.active_diff().map(|open_diff| open_diff.path.as_str())
    }

    pub fn titles(&self) -> Vec<String> {
        let mut titles = vec![LOG_TAB_TITLE.to_string()];
        titles.extend(
            self.open_diffs
                .iter()
                .map(|open_diff| working_tree_tab_title(&open_diff.path)),
        );
        titles
    }

    fn active_diff(&self) -> Option<&OpenDiff> {
        self.active_index
            .and_then(|index| self.open_diffs.get(index))
    }

    fn index_of(&self, path: &str) -> Option<usize> {
        self.open_diffs
            .iter()
            .position(|open_diff| open_diff.path == path)
    }
}

fn working_tree_tab_title(path: &str) -> String {
    let file_name = path.rsplit(PATH_SEPARATOR).next().unwrap_or(path);
    format!("{file_name}{WORKING_TREE_TAB_SUFFIX}")
}

#[cfg(test)]
mod editor_tabs_tests {
    use gitdeck_git::FileDiff;

    use super::{DiffOrigin, EditorTab, EditorTabs};

    fn file_diff(path: &str) -> FileDiff {
        FileDiff {
            path: path.to_string(),
            binary: false,
            missing_newline_at_end: false,
            hunks: Vec::new(),
        }
    }

    #[test]
    fn log_tab_is_active_before_any_diff_is_opened() {
        let tabs = EditorTabs::new();
        assert_eq!(tabs.active_tab(), EditorTab::Log);
        assert_eq!(tabs.active_index(), 0);
        assert_eq!(tabs.titles(), vec![String::from("Log: HEAD")]);
        assert_eq!(tabs.file_diff(), None);
        assert_eq!(tabs.active_origin(), None);
        assert_eq!(tabs.active_path(), None);
    }

    #[test]
    fn the_active_diff_reports_where_its_content_comes_from() {
        let mut tabs = EditorTabs::new();
        tabs.open("src/app.rs", DiffOrigin::Commit, file_diff("src/app.rs"));
        assert_eq!(tabs.active_origin(), Some(DiffOrigin::Commit));
        assert_eq!(tabs.active_path(), Some("src/app.rs"));
        tabs.open(
            "docs/readme.md",
            DiffOrigin::WorkingTree,
            file_diff("docs/readme.md"),
        );
        assert_eq!(tabs.active_origin(), Some(DiffOrigin::WorkingTree));
        assert_eq!(tabs.active_path(), Some("docs/readme.md"));
    }

    #[test]
    fn opened_diffs_become_tabs_in_open_order() {
        let mut tabs = EditorTabs::new();
        tabs.open(
            "src/app.rs",
            DiffOrigin::WorkingTree,
            file_diff("src/app.rs"),
        );
        assert_eq!(
            tabs.active_tab(),
            EditorTab::Diff(String::from("src/app.rs"))
        );
        assert_eq!(
            tabs.titles(),
            vec![
                String::from("Log: HEAD"),
                String::from("app.rs (Working Tree)"),
            ]
        );
        tabs.open(
            "docs/readme.md",
            DiffOrigin::WorkingTree,
            file_diff("docs/readme.md"),
        );
        assert_eq!(
            tabs.titles(),
            vec![
                String::from("Log: HEAD"),
                String::from("app.rs (Working Tree)"),
                String::from("readme.md (Working Tree)"),
            ]
        );
        assert_eq!(tabs.active_index(), 2);
    }

    #[test]
    fn reopening_an_open_diff_activates_it_without_adding_a_tab() {
        let mut tabs = EditorTabs::new();
        tabs.open(
            "src/app.rs",
            DiffOrigin::WorkingTree,
            file_diff("src/app.rs"),
        );
        tabs.open(
            "docs/readme.md",
            DiffOrigin::WorkingTree,
            file_diff("docs/readme.md"),
        );
        tabs.activate("src/app.rs");
        assert_eq!(tabs.active_index(), 1);
        assert_eq!(tabs.titles().len(), 3);
        assert_eq!(
            tabs.file_diff().map(|diff| diff.path.as_str()),
            Some("src/app.rs")
        );
    }

    #[test]
    fn closing_the_active_diff_returns_to_the_log_tab() {
        let mut tabs = EditorTabs::new();
        tabs.open(
            "src/app.rs",
            DiffOrigin::WorkingTree,
            file_diff("src/app.rs"),
        );
        tabs.close_active();
        assert_eq!(tabs.active_tab(), EditorTab::Log);
        assert_eq!(tabs.titles(), vec![String::from("Log: HEAD")]);
        assert!(!tabs.is_open("src/app.rs"));
    }

    #[test]
    fn closing_the_log_tab_does_nothing() {
        let mut tabs = EditorTabs::new();
        tabs.close_active();
        assert_eq!(tabs.active_tab(), EditorTab::Log);
        assert_eq!(tabs.titles().len(), 1);
    }

    #[test]
    fn tab_titles_use_the_file_name_without_the_directory() {
        let mut tabs = EditorTabs::new();
        tabs.open(
            "文档/说明.md",
            DiffOrigin::WorkingTree,
            file_diff("文档/说明.md"),
        );
        assert_eq!(
            tabs.titles()[1],
            String::from("说明.md (Working Tree)"),
            "tab title keeps only the file name"
        );
        assert!(!tabs.titles()[1].contains('/'));
    }
}
