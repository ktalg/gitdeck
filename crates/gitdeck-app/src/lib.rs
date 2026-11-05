mod branch_filter_dropdown_state;
mod branch_state;
mod changes_state;
mod commit_state;
mod confirm_dialog_state;
mod delete_branch_dialog_state;
mod diff_rows;
mod discard_dialog_state;
mod editor_tabs;
mod fetch_dialog_state;
mod help_overlay_state;
mod key_bindings;
mod log_state;
mod new_branch_dialog_state;
mod pull_dialog_state;
mod push_dialog_state;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use gitdeck_caps::{CapsProfile, ThemeId};
use gitdeck_core::{
    BranchEntry, ChangeEntry, CommitSummary, HeadInfo, InputEvent, KeyCode, KeyInput, KeyModifiers,
    LogFilter, LogQuery, MouseButton, MouseInput, MouseInputKind, PaneId, RefKind, RepoOperation,
    Size,
};
use gitdeck_git::{
    ChangedFile, CliGit, CommitOutcome, CommitRequest, ConflictResolution, FileDiff, GitBackend,
    GitError, MergeOutcome, PullStrategy, PushRequest,
};
use gitdeck_tui::{
    activity_rail_entry_at, activity_rail_footer_label, assign_word_highlights,
    branch_filter_block_rect, branch_filter_dropdown_layout, branch_list_rows_area,
    branch_row_rect, case_sensitive_filter_block_rect, changed_file_index_at, changes_panel_layout,
    changes_toolbar_entry_at, changes_toolbar_tooltip, changes_width_for_divider_column,
    compute_pane_layout, confirm_dialog_layout, default_changes_width, default_details_width,
    details_width_for_divider_column, diff_display_rows, discard_dialog_layout, editor_tab_hit_at,
    error_body_line_count, fetch_dialog_layout, file_count_label, help_overlay_layout,
    help_overlay_visible_row_count, hunk_checkbox_rect, next_step_hint, panel_error_layout,
    pull_dialog_layout, push_dialog_layout, rect_contains_cell, remote_tool_bar_rects,
    render_branch_filter_dropdown, render_branch_list, render_branches_rail, render_changes_panel,
    render_commit_details, render_commit_list, render_confirm_dialog, render_delete_branch_dialog,
    render_diff_pane, render_discard_dialog, render_fetch_dialog, render_help_overlay,
    render_new_branch_dialog, render_panel_error, render_pull_dialog, render_push_dialog,
    render_shell, sign_off_popover, text_display_width, text_filter_block_rect, theme_palette,
    top_bar_branch_chevron_rect, top_bar_branch_chip_rect, top_bar_divergence_rect,
    top_bar_help_button_rect, wrap_help_rows, BranchFilterDropdownLayout, BranchFilterDropdownView,
    BranchListView, ChangesPanelLayout, ChangesPanelView, ChangesRow, ChangesToolbarAction,
    CommitDetailsView, CommitListView, CommitPanelView, ConfirmDialogButton, ConfirmDialogView,
    ContainingBranches, DeleteBranchDialogButton, DeleteBranchDialogView, DiffPaneView,
    DiffRowKind, DiscardDialogButton, DiscardDialogView, EditorTabHit, EditorTabStrip,
    FetchDialogButton, FetchDialogView, HelpOverlayView, LogFilterBarView, NewBranchDialogView,
    PanelErrorSource, PanelErrorView, PullDialogButton, PullDialogFocus, PullDialogView,
    PullStrategyOption, PushDialogButton, PushDialogFocus, PushDialogView, ScreenDriver, ShellView,
    StatusMessageView, StatusTone, ABORT_CONFIRM_BUTTON_LABEL, BRANCH_FILTER_CURRENT_BRANCH_PREFIX,
    BRANCH_FILTER_HEAD_LABEL, CHECKBOX_COLUMN, CHECKBOX_WIDTH, CONFLICT_RESOLUTION_HINT,
    DETAILS_WIDTH_HIDDEN, DIFF_TEXT_COLUMN, HELP_OVERLAY_PAGE_ROWS, NO_REPOSITORY_LABEL,
    ORIGIN_REMOTE_NAME, PUSH_DETACHED_HEAD_LABEL, QUIT_CONFIRM_BUTTON_LABEL,
    REMOTE_FETCH_TOOL_LABEL, REMOTE_PULL_TOOL_LABEL, REMOTE_PUSH_TOOL_LABEL,
    REMOTE_TOOL_BAR_LABELS,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::branch_filter_dropdown_state::BranchFilterDropdownState;
use crate::branch_state::BranchState;
use crate::changes_state::ChangesState;
use crate::commit_state::CommitState;
use crate::confirm_dialog_state::ConfirmDialogState;
use crate::delete_branch_dialog_state::DeleteBranchDialogState;
use crate::diff_rows::diff_rows;
use crate::discard_dialog_state::DiscardDialogState;
use crate::editor_tabs::{DiffOrigin, EditorTabs};
use crate::fetch_dialog_state::FetchDialogState;
use crate::help_overlay_state::HelpOverlayState;
use crate::log_state::{CachedCommitBranches, LogState};
use crate::new_branch_dialog_state::NewBranchDialogState;
use crate::pull_dialog_state::PullDialogState;
use crate::push_dialog_state::PushDialogState;

pub use crate::commit_state::ChangesFocus;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogView {
    Log,
    Branches,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeyScope {
    Changes,
    LogList,
    Branches,
    Diff,
    Details,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SingleLetterCommand {
    ToggleLogView,
    OpenFetchDialog,
    OpenPullDialog,
    EditCommitMessage,
}

impl KeyScope {
    fn accepts_single_letter_command(self, command: SingleLetterCommand) -> bool {
        match command {
            SingleLetterCommand::ToggleLogView => {
                matches!(
                    self,
                    KeyScope::Changes | KeyScope::LogList | KeyScope::Branches
                )
            }
            SingleLetterCommand::OpenFetchDialog | SingleLetterCommand::OpenPullDialog => {
                self == KeyScope::Changes
            }
            SingleLetterCommand::EditCommitMessage => true,
        }
    }

    fn key_hint(self) -> &'static str {
        match self {
            KeyScope::Changes => CHANGES_PANE_KEY_HINT,
            KeyScope::LogList => LOG_LIST_KEY_HINT,
            KeyScope::Branches => BRANCHES_KEY_HINT,
            KeyScope::Diff => DIFF_PANE_KEY_HINT,
            KeyScope::Details => DETAILS_PANE_KEY_HINT,
        }
    }
}

pub use crate::key_bindings::{
    key_bindings_for, key_bindings_for_keys, KeyBinding, KeyCommand, KEY_BINDINGS,
};

pub use crate::editor_tabs::EditorTab;
pub use gitdeck_tui::{DividerId, PaneLayout};

const DETACHED_AT_FORMAT: &str = "detached at ";
const DETACHED_HEAD_BRANCH_LABEL: &str = "HEAD";
pub const DETACHED_SHORT_ID_LENGTH: usize = 7;
const LOG_MAX_COMMITS: usize = 500;
const LOG_PAGE_JUMP: isize = 10;
const LOG_PAGE_LOAD_AHEAD_COMMITS: usize = 20;
const WHEEL_STEP_ROWS: isize = 3;
const DIFF_HORIZONTAL_STEP_COLUMNS: u16 = 8;
const DOUBLE_CLICK_COUNT: u8 = 2;
const SINGLE_CLICK_COUNT: u8 = 1;
static NO_CHECKED_HUNKS: BTreeSet<usize> = BTreeSet::new();
const COMMIT_DIFF_HUNK_STAGING_STATUS: &str = "hunk staging works on working tree changes only";
const CHANGES_PANE_KEY_HINT: &str = "space check  enter diff  i message";
const LOG_LIST_KEY_HINT: &str = "j/k select  enter diff  b branches";
const BRANCHES_KEY_HINT: &str = "j/k select  enter checkout  alt+n new";
const DIFF_PANE_KEY_HINT: &str =
    "ctrl+w closes the tab and returns the file list  space hunk  alt+w words";
const DETAILS_PANE_KEY_HINT: &str = "tab focus  ctrl+q quit";
const WORD_HIGHLIGHTS_ON_STATUS: &str = "Word highlights: on";
const WORD_HIGHLIGHTS_OFF_STATUS: &str = "Word highlights: off";
const SOFT_WRAP_ON_STATUS: &str = "Soft wrap: on";
const SOFT_WRAP_OFF_STATUS: &str = "Soft wrap: off";
const DETAILS_HIDDEN_BY_DIFF_STATUS: &str = "Details is hidden while a diff tab shows";
const EMPTY_COMMIT_MESSAGE_STATUS: &str = "Commit message is empty";
const NOTHING_TO_COMMIT_STATUS: &str = "Nothing to commit";
const PUSH_FAILED_STATUS_PREFIX: &str = "Push failed: ";
const GIT_REMOTE_HEADER_PREFIXES: [&str; 2] = ["To ", "From "];
const GIT_REPORTED_LINE_SEPARATOR: &str = "; ";
const AMENDED_COMMIT_STATUS: &str = "Amended commit";
const GIT_DIRECTORY_NAME: &str = ".git";
const MERGE_MESSAGE_FILE_NAME: &str = "MERGE_MSG";
const CHECKED_OUT_STATUS_SUFFIX: &str = " checked out";
const CREATED_BRANCH_STATUS_PREFIX: &str = "Created and checked out ";
const ACCEPTED_OURS_STATUS_PREFIX: &str = "Accepted ours for ";
const ACCEPTED_THEIRS_STATUS_PREFIX: &str = "Accepted theirs for ";
const DISCARDED_STATUS_PREFIX: &str = "Discarded changes to ";
const DISCARD_CANCELLED_STATUS_SUFFIX: &str = " kept";
const SELECT_FILE_ROW_HINT: &str = "Select a file row first";
const REFRESHED_STATUS: &str = "Refreshed";
const CASE_SENSITIVE_ON_STATUS: &str = "Case sensitive: on";
const CASE_SENSITIVE_OFF_STATUS: &str = "Case sensitive: off";
const DELETED_BRANCH_STATUS_PREFIX: &str = "Deleted ";
const IGNORED_PATH_STATUS_PREFIX: &str = "Ignored ";
const MARKED_RESOLVED_STATUS_PREFIX: &str = "Marked ";
const MARKED_RESOLVED_STATUS_SUFFIX: &str = " as resolved";
const ALREADY_UP_TO_DATE_STATUS: &str = "Already up to date";
const FAST_FORWARD_STATUS_PREFIX: &str = "Fast-forward to ";
const MERGED_STATUS_PREFIX: &str = "Merged ";
const MERGED_STATUS_INFIX: &str = " into ";
const MERGE_CONFLICTS_STATUS_PREFIX: &str = "Merge of ";
const OPERATION_CONTINUATION_HINT_PREFIX: &str = "Resolve the conflicts and continue the ";
const MERGE_CONFLICTS_STATUS_SUFFIX: &str = " stopped with conflicts";
const REMOTE_MERGE_UNSUPPORTED_SUFFIX: &str = " is a remote branch: check it out before merging";
const REMOTE_CHECKOUT_UNSUPPORTED_SUFFIX: &str =
    " is a remote branch; check out a local branch instead";
const ABORT_DIALOG_TITLE_PREFIX: &str = "Abort ";
const ABORT_DIALOG_MESSAGE_TEMPLATE: &str = "Abort the {noun} in progress?";
const ABORT_DIALOG_DETAIL: &str = "This discards the conflict resolutions you already made.";
const ABORT_CANCELLED_STATUS: &str = "Abort cancelled";
const QUIT_CANCELLED_STATUS: &str = "Quit cancelled";
const QUIT_DIALOG_TITLE: &str = "Quit";
const UNSAVED_COMMIT_MESSAGE_QUIT_MESSAGE: &str = "You have an unsaved commit message.";
const MODIFIED_SELECTION_QUIT_MESSAGE: &str = "You have modified the commit selection.";
const UNSAVED_DRAFT_QUIT_MESSAGE: &str =
    "You have an unsaved commit message and a modified selection.";
const REFRESH_FAILED_STATUS_PREFIX: &str = "Operation finished; refresh failed: ";
pub const TICK_INTERVAL: Duration = Duration::from_millis(100);
pub const STATUS_MESSAGE_LIFETIME_TICKS: u16 = 30;
pub const DETAILS_QUERY_SETTLE_TICKS: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfirmDialogTarget {
    Abort,
    Quit,
}

impl ConfirmDialogTarget {
    fn state_mut(self, app: &mut App) -> &mut ConfirmDialogState {
        match self {
            Self::Abort => &mut app.abort_dialog,
            Self::Quit => &mut app.quit_dialog,
        }
    }
}

pub struct App {
    caps: CapsProfile,
    theme: ThemeId,
    screen: Size,
    focus: PaneId,
    changes_width: Option<u16>,
    details_width: Option<u16>,
    dragged_divider: Option<DividerId>,
    dragged_divider_offset: i16,
    repository: Option<Box<dyn GitBackend + Send>>,
    head_info: HeadInfo,
    head_commit_short_id: Option<String>,
    changes_state: ChangesState,
    commit_state: CommitState,
    editor_tabs: EditorTabs,
    log_state: LogState,
    log_view: LogView,
    log_filter_text: Option<String>,
    log_filter_branch: Option<String>,
    log_filter_case_sensitive: bool,
    log_filter_draft: Option<String>,
    log_reached_end: bool,
    log_filter_status: Option<String>,
    branch_state: BranchState,
    branch_filter_dropdown: BranchFilterDropdownState,
    push_dialog: PushDialogState,
    pull_dialog: PullDialogState,
    fetch_dialog: FetchDialogState,
    discard_dialog: DiscardDialogState,
    abort_dialog: ConfirmDialogState,
    abort_dialog_operation: Option<RepoOperation>,
    quit_dialog: ConfirmDialogState,
    exit_requested: bool,
    delete_branch_dialog: DeleteBranchDialogState,
    new_branch_dialog: NewBranchDialogState,
    utc_offset_minutes: i32,
    diff_scroll: usize,
    diff_horizontal_offset: u16,
    word_highlights: bool,
    soft_wrap: bool,
    focus_before_diff_tab: Option<PaneId>,
    hovered_rail_pane: Option<PaneId>,
    hovered_changes_toolbar: Option<ChangesToolbarAction>,
    hovered_top_bar_divergence: bool,
    divergence_hint: Option<String>,
    help_overlay: HelpOverlayState,
    hunk_checks: BTreeMap<String, BTreeSet<usize>>,
    panel_error: Option<PanelError>,
    status_message: Option<StatusMessage>,
    open_error: Option<String>,
    pending_details_query: Option<PendingDetailsQuery>,
}

struct PendingDetailsQuery {
    commit_id: String,
    settled_ticks: u32,
}

struct PanelError {
    source: PanelErrorSource,
    message: String,
    scroll: usize,
}

impl PanelError {
    fn new(source: PanelErrorSource, message: String) -> Self {
        Self {
            source,
            message,
            scroll: 0,
        }
    }
}

struct StatusMessage {
    text: String,
    tone: StatusTone,
    remaining_ticks: u16,
}

struct BranchFilterItem {
    label: String,
    revision: Option<String>,
}

impl App {
    pub fn new(caps: CapsProfile, theme: ThemeId) -> Self {
        Self {
            caps,
            theme,
            screen: Size {
                width: 0,
                height: 0,
            },
            focus: PaneId::Changes,
            changes_width: None,
            details_width: None,
            dragged_divider: None,
            dragged_divider_offset: 0,
            repository: None,
            head_info: HeadInfo::default(),
            head_commit_short_id: None,
            changes_state: ChangesState::new(),
            commit_state: CommitState::new(),
            editor_tabs: EditorTabs::new(),
            log_state: LogState::new(),
            log_view: LogView::Log,
            log_filter_text: None,
            log_filter_branch: None,
            log_filter_case_sensitive: false,
            log_filter_draft: None,
            log_reached_end: false,
            log_filter_status: None,
            branch_state: BranchState::new(),
            branch_filter_dropdown: BranchFilterDropdownState::new(),
            push_dialog: PushDialogState::new(),
            pull_dialog: PullDialogState::new(),
            fetch_dialog: FetchDialogState::new(),
            discard_dialog: DiscardDialogState::new(),
            abort_dialog: ConfirmDialogState::new(),
            abort_dialog_operation: None,
            quit_dialog: ConfirmDialogState::new(),
            exit_requested: false,
            delete_branch_dialog: DeleteBranchDialogState::new(),
            new_branch_dialog: NewBranchDialogState::new(),
            utc_offset_minutes: 0,
            diff_scroll: 0,
            diff_horizontal_offset: 0,
            word_highlights: true,
            soft_wrap: true,
            focus_before_diff_tab: None,
            hovered_rail_pane: None,
            hovered_changes_toolbar: None,
            hovered_top_bar_divergence: false,
            divergence_hint: None,
            help_overlay: HelpOverlayState::new(),
            hunk_checks: BTreeMap::new(),
            panel_error: None,
            status_message: None,
            open_error: None,
            pending_details_query: None,
        }
    }

    pub fn with_repository(
        repository_path: impl AsRef<Path>,
        caps: CapsProfile,
        theme: ThemeId,
    ) -> Result<Self, GitError> {
        let repository = Box::new(CliGit::open(repository_path)?);
        Self::with_backend(repository, caps, theme)
    }

    pub fn with_backend(
        repository: Box<dyn GitBackend + Send>,
        caps: CapsProfile,
        theme: ThemeId,
    ) -> Result<Self, GitError> {
        let mut app = Self::new(caps, theme);
        app.repository = Some(repository);
        app.refresh()?;
        Ok(app)
    }

    pub fn open_repository(repository_path: &Path, caps: CapsProfile, theme: ThemeId) -> Self {
        match Self::with_repository(repository_path, caps, theme) {
            Ok(app) => app,
            Err(error) => {
                let mut app = Self::new(caps, theme);
                app.open_error = Some(single_line_error_text(&error));
                app
            }
        }
    }

    pub fn open_error(&self) -> Option<&str> {
        self.open_error.as_deref()
    }

    pub fn caps(&self) -> CapsProfile {
        self.caps
    }

    pub fn theme(&self) -> ThemeId {
        self.theme
    }

    pub fn repository_path(&self) -> Option<&Path> {
        self.repository.as_deref().map(GitBackend::repository_path)
    }

    pub fn project_name(&self) -> Option<String> {
        let directory_name = self.repository_path()?.file_name()?;
        Some(directory_name.to_string_lossy().into_owned())
    }

    pub fn head_info(&self) -> &HeadInfo {
        &self.head_info
    }

    pub fn changes(&self) -> &[ChangeEntry] {
        self.changes_state.entries()
    }

    pub fn selected_file_path(&self) -> Option<&str> {
        self.changes_state.selected_file_path()
    }

    pub fn checked_file_paths(&self) -> Vec<String> {
        self.changes_state.checked_file_paths()
    }

    pub fn commit_message(&self) -> &str {
        self.commit_state.message()
    }

    pub fn set_commit_message(&mut self, message: &str) {
        self.commit_state.set_message(message);
    }

    pub fn changes_focus(&self) -> ChangesFocus {
        self.commit_state.focus()
    }

    pub fn amend(&self) -> bool {
        self.commit_state.amend()
    }

    pub fn sign_off(&self) -> bool {
        self.commit_state.sign_off()
    }

    pub fn commit_options_visible(&self) -> bool {
        self.commit_state.options_visible()
    }

    pub fn commit_enabled(&self) -> bool {
        !self.commit_state.message().trim().is_empty()
            && (!self.committed_paths().is_empty()
                || self.commit_state.amend()
                || self.head_info.merging)
    }

    fn checked_untracked_file_count(&self) -> usize {
        self.changes_state.checked_untracked_paths().len()
    }

    fn committed_paths(&self) -> Vec<String> {
        let mut paths = self.changes_state.checked_file_paths();
        paths.extend(
            self.hunk_checks
                .iter()
                .filter(|(_, hunk_indices)| !hunk_indices.is_empty())
                .map(|(path, _)| path.clone()),
        );
        paths.sort();
        paths.dedup();
        paths
    }

    fn checked_hunk_selection(&self) -> Vec<(String, Vec<usize>)> {
        self.hunk_checks
            .iter()
            .filter(|(_, hunk_indices)| !hunk_indices.is_empty())
            .map(|(path, hunk_indices)| {
                (
                    path.clone(),
                    hunk_indices.iter().copied().collect::<Vec<usize>>(),
                )
            })
            .collect()
    }

    pub fn status_hint(&self) -> Option<&str> {
        if self.active_status_message().is_some() {
            return None;
        }
        if let Some(footer_label) = self.hovered_rail_footer_label() {
            return Some(footer_label);
        }
        if let Some(tooltip) = self
            .hovered_changes_toolbar
            .and_then(changes_toolbar_tooltip)
        {
            return Some(tooltip);
        }
        if let Some(hint) = self.divergence_hint.as_deref() {
            return Some(hint);
        }
        Some(
            self.conflict_resolution_hint()
                .or_else(|| self.diff_tab_close_hint())
                .unwrap_or_else(|| self.key_scope().key_hint()),
        )
    }

    fn diff_tab_close_hint(&self) -> Option<&'static str> {
        self.editor_tabs
            .file_diff()
            .is_some()
            .then_some(DIFF_PANE_KEY_HINT)
    }

    pub fn commit(&mut self) -> Result<CommitOutcome, GitError> {
        let result = self.run_commit();
        self.record_panel_error(PanelErrorSource::Commit, &result);
        result
    }

    pub fn commit_and_push(&mut self) -> Result<CommitOutcome, GitError> {
        let outcome = self.commit()?;
        let push_result = self.push_head_branch();
        self.record_panel_error(PanelErrorSource::Push, &push_result);
        push_result.map(|()| outcome)
    }

    fn run_commit(&mut self) -> Result<CommitOutcome, GitError> {
        let hunk_selection = self.checked_hunk_selection();
        let request = CommitRequest {
            message: self.commit_state.message().to_string(),
            amend: self.commit_state.amend(),
            sign_off: self.commit_state.sign_off(),
            paths: self.committed_paths(),
            hunks: hunk_selection.clone(),
        };
        if !hunk_selection.is_empty() {
            self.show_status_message(hunk_staging_receipt(&hunk_selection), StatusTone::Info);
        }
        let outcome = self.run_write_operation(|repository| repository.commit(request))?;
        self.commit_state.clear_after_commit();
        self.hunk_checks.clear();
        Ok(outcome)
    }

    fn push_head_branch(&mut self) -> Result<(), GitError> {
        let branch = self.head_info.branch.clone();
        self.run_write_operation(move |repository| match branch {
            Some(branch) => repository.push(&branch),
            None => Err(GitError::NoBranch),
        })
    }

    pub fn push_dialog_visible(&self) -> bool {
        self.push_dialog.is_open()
    }

    pub fn push_dialog_focus(&self) -> PushDialogFocus {
        self.push_dialog.focus()
    }

    pub fn push_dialog_button(&self) -> PushDialogButton {
        self.push_dialog.button()
    }

    pub fn push_dialog_set_upstream(&self) -> bool {
        self.push_dialog.set_upstream()
    }

    pub fn push_dialog_force_with_lease(&self) -> bool {
        self.push_dialog.force_with_lease()
    }

    pub fn push_dialog_branch_label(&self) -> &str {
        self.head_info
            .branch
            .as_deref()
            .unwrap_or(PUSH_DETACHED_HEAD_LABEL)
    }

    pub fn open_push_dialog(&mut self) {
        self.close_remote_dialogs();
        self.push_dialog.open();
    }

    pub fn close_push_dialog(&mut self) {
        self.push_dialog.close();
    }

    pub fn push_request(&mut self, request: PushRequest) -> Result<(), GitError> {
        let result = self.run_push_request(request);
        self.record_panel_error(PanelErrorSource::Push, &result);
        result
    }

    fn run_push_request(&mut self, request: PushRequest) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.push_request(request))
    }

    fn confirm_push_dialog(&mut self) {
        let branch = self
            .head_info
            .branch
            .clone()
            .unwrap_or_else(|| String::from(DETACHED_HEAD_BRANCH_LABEL));
        let request = PushRequest {
            branch: branch.clone(),
            set_upstream: self.push_dialog.set_upstream(),
            force_with_lease: self.push_dialog.force_with_lease(),
        };
        self.push_dialog.close();
        if self.push_request(request).is_err() {
            return;
        }
        self.show_status_message(
            format!("Pushed {branch} to {ORIGIN_REMOTE_NAME}"),
            StatusTone::Info,
        );
    }

    pub fn pull_dialog_visible(&self) -> bool {
        self.pull_dialog.is_open()
    }

    pub fn pull_dialog_focus(&self) -> PullDialogFocus {
        self.pull_dialog.focus()
    }

    pub fn pull_dialog_strategy(&self) -> PullStrategyOption {
        self.pull_dialog.strategy()
    }

    pub fn pull_dialog_button(&self) -> PullDialogButton {
        self.pull_dialog.button()
    }

    pub fn pull_dialog_branch_label(&self) -> &str {
        self.head_info
            .branch
            .as_deref()
            .unwrap_or(PUSH_DETACHED_HEAD_LABEL)
    }

    pub fn open_pull_dialog(&mut self) {
        self.close_remote_dialogs();
        self.pull_dialog.open();
    }

    pub fn close_pull_dialog(&mut self) {
        self.pull_dialog.close();
    }

    pub fn pull(&mut self, strategy: PullStrategy) -> Result<(), GitError> {
        let result = self.run_pull(strategy);
        self.record_panel_error(PanelErrorSource::Pull, &result);
        result
    }

    fn run_pull(&mut self, strategy: PullStrategy) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.pull(strategy))
    }

    fn confirm_pull_dialog(&mut self) {
        let branch = self
            .head_info
            .branch
            .clone()
            .unwrap_or_else(|| String::from(DETACHED_HEAD_BRANCH_LABEL));
        let strategy = pull_strategy(self.pull_dialog.strategy());
        self.pull_dialog.close();
        if self.pull(strategy).is_err() {
            return;
        }
        self.show_status_message(
            format!("Pulled {branch} from {ORIGIN_REMOTE_NAME}"),
            StatusTone::Info,
        );
    }

    pub fn fetch_dialog_visible(&self) -> bool {
        self.fetch_dialog.is_open()
    }

    pub fn fetch_dialog_button(&self) -> FetchDialogButton {
        self.fetch_dialog.button()
    }

    pub fn fetch_dialog_remote_available(&self) -> bool {
        self.head_info.upstream.is_some()
            || self
                .branch_state
                .branches()
                .iter()
                .any(|branch| branch.kind == RefKind::RemoteBranch)
    }

    pub fn open_fetch_dialog(&mut self) {
        self.close_remote_dialogs();
        self.fetch_dialog.open();
    }

    pub fn close_fetch_dialog(&mut self) {
        self.fetch_dialog.close();
    }

    pub fn fetch(&mut self, remote: Option<&str>) -> Result<(), GitError> {
        let result = self.run_fetch(remote);
        self.record_panel_error(PanelErrorSource::Fetch, &result);
        result
    }

    fn run_fetch(&mut self, remote: Option<&str>) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.fetch(remote))
    }

    fn confirm_fetch_dialog(&mut self) {
        self.fetch_dialog.close();
        if self.fetch(None).is_err() {
            return;
        }
        self.show_status_message(
            format!("Fetched from {ORIGIN_REMOTE_NAME}"),
            StatusTone::Info,
        );
    }

    fn close_remote_dialogs(&mut self) {
        self.push_dialog.close();
        self.pull_dialog.close();
        self.fetch_dialog.close();
    }

    pub fn checkout_branch(&mut self, name: &str) {
        let result = self.run_checkout_branch(name);
        self.record_panel_error(PanelErrorSource::Checkout, &result);
        if result.is_ok() {
            self.show_status_message(
                format!("{name}{CHECKED_OUT_STATUS_SUFFIX}"),
                StatusTone::Info,
            );
        }
    }

    fn run_checkout_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.checkout_branch(name))
    }

    pub fn resolve_conflict(&mut self, path: &str, resolution: ConflictResolution) {
        let result = self.run_conflict_resolution(path, resolution);
        self.record_panel_error(PanelErrorSource::Conflict, &result);
        if result.is_ok() {
            self.show_status_message(
                conflict_resolution_receipt(path, resolution),
                StatusTone::Info,
            );
        }
    }

    fn run_conflict_resolution(
        &mut self,
        path: &str,
        resolution: ConflictResolution,
    ) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.resolve_conflict(path, resolution))
    }

    fn resolve_selected_conflict(&mut self, resolution: ConflictResolution) {
        let Some(path) = self
            .changes_state
            .selected_conflicted_path()
            .map(str::to_string)
        else {
            if self.changes_state.selected_file_path().is_none() {
                self.warn_select_file_row();
            }
            return;
        };
        self.resolve_conflict(&path, resolution);
    }

    pub fn conflict_resolution_hint(&self) -> Option<&'static str> {
        self.changes_state
            .selected_conflicted_path()
            .map(|_| CONFLICT_RESOLUTION_HINT)
    }

    pub fn discard_dialog_visible(&self) -> bool {
        self.discard_dialog.is_open()
    }

    pub fn discard_dialog_button(&self) -> DiscardDialogButton {
        self.discard_dialog.button()
    }

    pub fn discard_dialog_path(&self) -> Option<&str> {
        self.discard_dialog.path()
    }

    pub fn open_discard_dialog(&mut self) {
        let Some(path) = self.changes_state.selected_file_path().map(str::to_string) else {
            self.warn_select_file_row();
            return;
        };
        self.close_remote_dialogs();
        self.end_commit_editing();
        self.discard_dialog.open(&path);
    }

    pub fn discard_changes(&mut self, path: &str) {
        let result = self.run_discard_changes(path);
        self.record_panel_error(PanelErrorSource::Discard, &result);
        if result.is_ok() {
            self.show_status_message(format!("{DISCARDED_STATUS_PREFIX}{path}"), StatusTone::Info);
        }
    }

    fn run_discard_changes(&mut self, path: &str) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.discard_changes(path))
    }

    fn close_discard_dialog(&mut self) {
        if let Some(path) = self.discard_dialog.path().map(str::to_string) {
            self.show_status_message(
                format!("{path}{DISCARD_CANCELLED_STATUS_SUFFIX}"),
                StatusTone::Info,
            );
        }
        self.discard_dialog.close();
    }

    fn warn_select_file_row(&mut self) {
        self.show_status_message(String::from(SELECT_FILE_ROW_HINT), StatusTone::Warning);
    }

    fn confirm_discard_dialog(&mut self) {
        let Some(path) = self.discard_dialog.path().map(str::to_string) else {
            return;
        };
        self.discard_dialog.close();
        self.discard_changes(&path);
    }

    fn activate_discard_dialog_button(&mut self) {
        match self.discard_dialog.button() {
            DiscardDialogButton::Discard => self.confirm_discard_dialog(),
            DiscardDialogButton::Cancel => self.close_discard_dialog(),
        }
    }

    pub fn delete_branch_dialog_visible(&self) -> bool {
        self.delete_branch_dialog.is_open()
    }

    pub fn delete_branch_dialog_button(&self) -> DeleteBranchDialogButton {
        self.delete_branch_dialog.button()
    }

    pub fn delete_branch_dialog_name(&self) -> Option<&str> {
        self.delete_branch_dialog.name()
    }

    pub fn open_delete_branch_dialog(&mut self) {
        if self.log_view != LogView::Branches || self.repository.is_none() {
            return;
        }
        let Some(branch) = self.branch_state.selected_branch() else {
            return;
        };
        let name = branch.name.clone();
        if branch.kind == RefKind::RemoteBranch {
            self.panel_error = Some(PanelError::new(
                PanelErrorSource::Branch,
                format!("remote branches cannot be deleted here: {name}"),
            ));
            return;
        }
        self.close_remote_dialogs();
        self.end_commit_editing();
        self.delete_branch_dialog.open(&name);
    }

    fn close_delete_branch_dialog(&mut self) {
        self.delete_branch_dialog.close();
    }

    fn confirm_delete_branch_dialog(&mut self) {
        let Some(name) = self.delete_branch_dialog.name().map(str::to_string) else {
            return;
        };
        self.delete_branch_dialog.close();
        self.delete_branch(&name);
    }

    fn activate_delete_branch_dialog_button(&mut self) {
        match self.delete_branch_dialog.button() {
            DeleteBranchDialogButton::Delete => self.confirm_delete_branch_dialog(),
            DeleteBranchDialogButton::Cancel => self.close_delete_branch_dialog(),
        }
    }

    pub fn delete_branch(&mut self, name: &str) {
        let result = self.run_delete_branch(name);
        self.record_panel_error(PanelErrorSource::Branch, &result);
        if result.is_ok() {
            self.show_status_message(
                format!("{DELETED_BRANCH_STATUS_PREFIX}{name}"),
                StatusTone::Info,
            );
        }
    }

    fn run_delete_branch(&mut self, name: &str) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.delete_branch(name))
    }

    pub fn ignore_path(&mut self, path: &str) {
        let result = self.run_ignore_path(path);
        self.record_panel_error(PanelErrorSource::Ignore, &result);
        if result.is_ok() {
            self.show_status_message(
                format!("{IGNORED_PATH_STATUS_PREFIX}{path}"),
                StatusTone::Info,
            );
        }
    }

    fn run_ignore_path(&mut self, path: &str) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.ignore_path(path))
    }

    fn ignore_selected_untracked_path(&mut self) {
        let Some(path) = self
            .changes_state
            .selected_untracked_path()
            .map(str::to_string)
        else {
            if self.changes_state.selected_file_path().is_none() {
                self.warn_select_file_row();
            }
            return;
        };
        self.ignore_path(&path);
    }

    pub fn merge_branch(&mut self, name: &str) {
        let outcome = self.run_merge_branch(name);
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                self.set_panel_error(PanelErrorSource::Merge, &error);
                return;
            }
        };
        self.show_merge_outcome(name, &outcome);
    }

    fn run_merge_branch(&mut self, name: &str) -> Result<MergeOutcome, GitError> {
        self.run_write_operation(|repository| repository.merge_branch(name))
    }

    fn show_merge_outcome(&mut self, name: &str, outcome: &MergeOutcome) {
        match outcome {
            MergeOutcome::AlreadyUpToDate => {
                self.show_status_message(String::from(ALREADY_UP_TO_DATE_STATUS), StatusTone::Info)
            }
            MergeOutcome::FastForward { .. } => self.show_status_message(
                format!("{FAST_FORWARD_STATUS_PREFIX}{name}"),
                StatusTone::Info,
            ),
            MergeOutcome::Merged { .. } => {
                let current = self
                    .head_info
                    .branch
                    .clone()
                    .unwrap_or_else(|| String::from(DETACHED_HEAD_BRANCH_LABEL));
                self.show_status_message(
                    format!("{MERGED_STATUS_PREFIX}{name}{MERGED_STATUS_INFIX}{current}"),
                    StatusTone::Info,
                )
            }
            MergeOutcome::Conflicts { paths } => {
                self.show_status_message(
                    format!("{MERGE_CONFLICTS_STATUS_PREFIX}{name}{MERGE_CONFLICTS_STATUS_SUFFIX}"),
                    StatusTone::Failure,
                );
                self.panel_error = Some(PanelError::new(
                    PanelErrorSource::Merge,
                    merge_conflict_count_text(paths.len()),
                ));
            }
        }
    }

    pub fn abort_operation(&mut self) {
        let result = self.run_operation_command(|repository| repository.abort_operation());
        match result {
            Ok(operation) => self.show_status_message(
                format!("Aborted the {}", operation.noun()),
                StatusTone::Info,
            ),
            Err(error) => self.set_panel_error(PanelErrorSource::Operation, &error),
        }
    }

    pub fn continue_operation(&mut self) {
        let result = self.run_operation_command(|repository| repository.continue_operation());
        match result {
            Ok(operation) => self
                .show_status_message(format!("{} continued", operation.title()), StatusTone::Info),
            Err(error) => self.set_panel_error(PanelErrorSource::Operation, &error),
        }
    }

    pub fn abort_dialog_visible(&self) -> bool {
        self.abort_dialog.is_open()
    }

    pub fn abort_dialog_button(&self) -> ConfirmDialogButton {
        self.abort_dialog.button()
    }

    pub fn abort_dialog_message(&self) -> Option<String> {
        self.abort_dialog_operation
            .map(|operation| ABORT_DIALOG_MESSAGE_TEMPLATE.replace("{noun}", operation.noun()))
    }

    pub fn quit_dialog_visible(&self) -> bool {
        self.quit_dialog.is_open()
    }

    pub fn quit_dialog_button(&self) -> ConfirmDialogButton {
        self.quit_dialog.button()
    }

    pub fn quit_dialog_message(&self) -> Option<&'static str> {
        match (
            !self.commit_state.message().is_empty(),
            self.commit_selection_differs_from_default(),
        ) {
            (false, false) => None,
            (true, false) => Some(UNSAVED_COMMIT_MESSAGE_QUIT_MESSAGE),
            (false, true) => Some(MODIFIED_SELECTION_QUIT_MESSAGE),
            (true, true) => Some(UNSAVED_DRAFT_QUIT_MESSAGE),
        }
    }

    pub const fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    pub fn apply_quit_key(&mut self, key_input: &KeyInput) {
        if key_input.code == KeyCode::Char('c') && key_input.modifiers.ctrl {
            self.exit_requested = true;
            return;
        }
        if self.quit_dialog.is_open() {
            self.apply_confirm_dialog_key(ConfirmDialogTarget::Quit, *key_input);
            return;
        }
        if self.quit_dialog_message().is_none() {
            self.exit_requested = true;
            return;
        }
        self.quit_dialog.open();
    }

    fn commit_selection_differs_from_default(&self) -> bool {
        !self.hunk_checks.is_empty()
            || self
                .changes_state
                .entries()
                .iter()
                .any(|entry| !self.changes_state.is_path_checked(&entry.path))
    }

    fn request_abort_operation(&mut self) {
        let Some(repository) = self.repository.as_ref() else {
            self.abort_operation();
            return;
        };
        match repository.current_operation() {
            Ok(Some(operation)) => {
                self.abort_dialog_operation = Some(operation);
                self.abort_dialog.open();
            }
            Ok(None) => self.abort_operation(),
            Err(error) => self.set_panel_error(PanelErrorSource::Operation, &error),
        }
    }

    fn apply_abort_dialog_key(&mut self, key_input: KeyInput) {
        self.apply_confirm_dialog_key(ConfirmDialogTarget::Abort, key_input);
    }

    fn apply_confirm_dialog_key(&mut self, target: ConfirmDialogTarget, key_input: KeyInput) {
        match key_input.code {
            KeyCode::Tab | KeyCode::Right => target.state_mut(self).select_button_by(1),
            KeyCode::BackTab | KeyCode::Left => target.state_mut(self).select_button_by(-1),
            KeyCode::Enter | KeyCode::Space => self.activate_confirm_dialog_button(target),
            KeyCode::Esc => self.close_confirm_dialog(target),
            _ => {}
        }
    }

    fn close_confirm_dialog(&mut self, target: ConfirmDialogTarget) {
        target.state_mut(self).close();
        if target == ConfirmDialogTarget::Abort {
            self.abort_dialog_operation = None;
        }
    }

    fn activate_confirm_dialog_button(&mut self, target: ConfirmDialogTarget) {
        let confirmed = target.state_mut(self).button() == ConfirmDialogButton::Confirm;
        self.close_confirm_dialog(target);
        match (target, confirmed) {
            (ConfirmDialogTarget::Abort, true) => self.abort_operation(),
            (ConfirmDialogTarget::Abort, false) => {
                self.show_status_message(String::from(ABORT_CANCELLED_STATUS), StatusTone::Info)
            }
            (ConfirmDialogTarget::Quit, true) => self.exit_requested = true,
            (ConfirmDialogTarget::Quit, false) => {
                self.show_status_message(String::from(QUIT_CANCELLED_STATUS), StatusTone::Info)
            }
        }
    }

    fn run_operation_command(
        &mut self,
        command: impl FnOnce(&dyn GitBackend) -> Result<(), GitError>,
    ) -> Result<RepoOperation, GitError> {
        self.run_write_operation(|repository| {
            let operation = repository
                .current_operation()?
                .ok_or(GitError::NoOperationInProgress)?;
            command(repository)?;
            Ok(operation)
        })
    }

    fn merge_selected_branch(&mut self) {
        if self.log_view != LogView::Branches {
            return;
        }
        let Some(branch) = self.branch_state.selected_branch() else {
            return;
        };
        let name = branch.name.clone();
        if branch.kind == RefKind::RemoteBranch {
            self.panel_error = Some(PanelError::new(
                PanelErrorSource::Merge,
                format!("{name}{REMOTE_MERGE_UNSUPPORTED_SUFFIX}"),
            ));
            return;
        }
        self.merge_branch(&name);
    }

    fn open_new_branch_dialog(&mut self) {
        if self.repository.is_none() {
            return;
        }
        self.new_branch_dialog.open();
    }

    pub fn new_branch_dialog_name(&self) -> Option<&str> {
        self.new_branch_dialog.name()
    }

    fn new_branch_start_point(&self) -> Option<String> {
        if self.log_view != LogView::Branches {
            return None;
        }
        self.branch_state
            .selected_branch()
            .map(|branch| branch.name.clone())
    }

    fn create_branch_from_selected_branch(&mut self) {
        let Some(name) = self.new_branch_dialog.name() else {
            return;
        };
        let name = name.to_string();
        let start_point = self.new_branch_start_point();
        let result = self.run_create_branch(&name, start_point.as_deref());
        self.new_branch_dialog.close();
        self.record_panel_error(PanelErrorSource::Checkout, &result);
        if result.is_ok() {
            self.branch_state.select_branch_named(&name);
            self.keep_selected_branch_visible();
            self.show_status_message(
                format!("{CREATED_BRANCH_STATUS_PREFIX}{name}"),
                StatusTone::Info,
            );
        }
    }

    fn run_create_branch(&mut self, name: &str, start_point: Option<&str>) -> Result<(), GitError> {
        self.run_write_operation(|repository| repository.create_branch(name, start_point))
    }

    fn checkout_selected_branch(&mut self) {
        let Some(branch) = self.branch_state.selected_branch() else {
            return;
        };
        let name = branch.name.clone();
        if branch.kind == RefKind::RemoteBranch {
            self.panel_error = Some(PanelError::new(
                PanelErrorSource::Checkout,
                remote_checkout_unsupported_text(&name),
            ));
            return;
        }
        self.checkout_branch(&name);
    }

    pub fn help_overlay_visible(&self) -> bool {
        self.help_overlay.is_open()
    }

    pub fn log_view(&self) -> LogView {
        self.log_view
    }

    pub fn branches(&self) -> &[BranchEntry] {
        self.branch_state.branches()
    }

    pub fn selected_branch_index(&self) -> Option<usize> {
        self.branch_state.selected_index()
    }

    fn toggle_log_view(&mut self) {
        if self.editor_tabs.file_diff().is_some() {
            return;
        }
        self.log_view = match self.log_view {
            LogView::Log => LogView::Branches,
            LogView::Branches => LogView::Log,
        };
        self.focus = PaneId::Log;
        self.keep_selected_branch_visible();
    }

    fn branch_list_visible_rows(&self) -> usize {
        let layout = self.layout();
        let list = branch_list_rows_area(layout.rows_above_bottom_rule(layout.commits));
        usize::from(list.height)
    }

    fn keep_selected_branch_visible(&mut self) {
        self.branch_state
            .keep_selected_branch_visible(self.branch_list_visible_rows());
    }

    fn select_branch(&mut self, delta: isize) {
        if !self.branch_state.select_by(delta) {
            return;
        }
        self.keep_selected_branch_visible();
    }

    fn record_panel_error<T>(&mut self, source: PanelErrorSource, result: &Result<T, GitError>) {
        let Err(error) = result else {
            return;
        };
        self.set_panel_error(source, error);
    }

    fn set_panel_error(&mut self, source: PanelErrorSource, error: &GitError) {
        let message = match source {
            PanelErrorSource::Push | PanelErrorSource::Pull | PanelErrorSource::Fetch => {
                remote_failure_message(error)
            }
            _ => single_line_error_text(error),
        };
        self.panel_error = Some(PanelError::new(source, message));
        if source == PanelErrorSource::Push {
            self.show_status_message(
                format!(
                    "{PUSH_FAILED_STATUS_PREFIX}{}",
                    first_line_error_text(error)
                ),
                StatusTone::Failure,
            );
        }
    }

    fn show_status_message(&mut self, text: String, tone: StatusTone) {
        self.status_message = Some(StatusMessage {
            text,
            tone,
            remaining_ticks: STATUS_MESSAGE_LIFETIME_TICKS,
        });
    }

    fn age_status_message(&mut self) {
        let Some(status_message) = self.status_message.as_mut() else {
            return;
        };
        status_message.remaining_ticks = status_message.remaining_ticks.saturating_sub(1);
        if status_message.remaining_ticks == 0 {
            self.status_message = None;
        }
    }

    pub fn status_message(&self) -> Option<(&str, StatusTone)> {
        self.active_status_message()
    }

    fn active_status_message(&self) -> Option<(&str, StatusTone)> {
        if let Some(status_message) = self.status_message.as_ref() {
            return Some((status_message.text.as_str(), status_message.tone));
        }
        self.log_filter_status
            .as_deref()
            .map(|text| (text, StatusTone::Info))
    }

    fn filter_bar_text(&self) -> &str {
        if let Some(draft) = self.log_filter_draft.as_deref() {
            return draft;
        }
        self.log_filter_text.as_deref().unwrap_or("")
    }

    pub fn log_filter_text(&self) -> Option<&str> {
        self.log_filter_text.as_deref()
    }

    pub fn log_filter_status(&self) -> Option<&str> {
        self.log_filter_status.as_deref()
    }

    pub fn log_reached_end(&self) -> bool {
        self.log_reached_end
    }

    pub fn log_filter_editing(&self) -> bool {
        self.log_filter_draft.is_some()
    }

    pub fn branch_filter_dropdown_visible(&self) -> bool {
        self.branch_filter_dropdown.is_open()
    }

    pub fn set_utc_offset_minutes(&mut self, minutes: i32) {
        self.utc_offset_minutes = minutes;
    }

    pub fn commits(&self) -> &[CommitSummary] {
        self.log_state.commits()
    }

    pub fn selected_commit(&self) -> Option<&CommitSummary> {
        self.log_state.selected_commit()
    }

    pub fn selected_commit_files(&self) -> &[ChangedFile] {
        self.log_state.files_at_selected_commit()
    }

    pub fn selected_commit_branches(&self) -> Option<ContainingBranches<'_>> {
        self.log_state
            .selected_commit_branches()
            .map(|cached| match cached {
                CachedCommitBranches::Listed(branches) => ContainingBranches::Listed(branches),
                CachedCommitBranches::Unavailable => ContainingBranches::Unavailable,
            })
    }

    pub fn select_commit(&mut self, delta: isize) {
        if !self.log_state.select_by(delta) {
            return;
        }
        self.keep_selected_commit_visible();
        self.load_next_log_page_when_selection_nears_the_end();
        self.defer_selected_commit_details_query();
    }

    fn scroll_log_list(&mut self, delta: isize) {
        if self.log_view == LogView::Branches {
            self.branch_state
                .scroll_viewport_by(delta, self.branch_list_visible_rows());
            return;
        }
        self.scroll_commit_viewport(delta);
    }

    pub fn scroll_commit_viewport(&mut self, delta: isize) {
        let visible_rows = self.commit_viewport_rows();
        if !self.log_state.scroll_viewport_by(delta, visible_rows) {
            return;
        }
        self.load_next_log_page_when_selection_nears_the_end();
        self.defer_selected_commit_details_query();
    }

    fn defer_selected_commit_details_query(&mut self) {
        let Some(commit_id) = self
            .log_state
            .selected_commit()
            .map(|commit| commit.id.clone())
        else {
            self.pending_details_query = None;
            return;
        };
        if self.log_state.selected_commit_details_are_cached() {
            self.pending_details_query = None;
            return;
        }
        match self.pending_details_query.as_mut() {
            Some(pending) if pending.commit_id == commit_id => pending.settled_ticks = 0,
            _ => {
                self.pending_details_query = Some(PendingDetailsQuery {
                    commit_id,
                    settled_ticks: 0,
                });
            }
        }
    }

    fn run_pending_details_query_once_the_selection_settles(&mut self) {
        let Some(pending) = self.pending_details_query.as_mut() else {
            return;
        };
        pending.settled_ticks = pending.settled_ticks.saturating_add(1);
        if pending.settled_ticks < DETAILS_QUERY_SETTLE_TICKS {
            return;
        }
        self.pending_details_query = None;
        self.load_selected_commit_files_from_ui();
    }

    fn load_selected_commit_files_from_ui(&mut self) {
        let result = self.load_selected_commit_files();
        self.record_panel_error(PanelErrorSource::Refresh, &result);
        self.load_selected_commit_branches_from_ui();
    }

    fn load_selected_commit_branches_from_ui(&mut self) {
        let result = self.load_selected_commit_branches();
        self.record_panel_error(PanelErrorSource::Refresh, &result);
    }

    pub fn panel_error(&self) -> Option<(PanelErrorSource, &str)> {
        self.panel_error
            .as_ref()
            .map(|panel_error| (panel_error.source, panel_error.message.as_str()))
    }

    fn dismiss_panel_error(&mut self) {
        self.panel_error = None;
    }

    pub fn active_tab(&self) -> EditorTab {
        self.editor_tabs.active_tab()
    }

    pub fn active_tab_index(&self) -> usize {
        self.editor_tabs.active_index()
    }

    pub fn tab_titles(&self) -> Vec<String> {
        self.editor_tabs.titles()
    }

    pub fn file_diff(&self) -> Option<&FileDiff> {
        self.editor_tabs.file_diff()
    }

    pub fn diff_scroll(&self) -> usize {
        self.diff_scroll
    }

    pub fn open_diff(&mut self, path: &str) -> Result<(), GitError> {
        if self.editor_tabs.is_open(path) {
            self.editor_tabs.activate(path);
            self.reset_diff_view_offsets();
            self.focus_active_diff_tab();
            return Ok(());
        }
        let Some(repository) = self.repository.as_ref() else {
            return Ok(());
        };
        let file_diff = repository.diff_working_tree(path)?;
        self.editor_tabs
            .open(path, DiffOrigin::WorkingTree, file_diff);
        self.reset_diff_view_offsets();
        self.focus_active_diff_tab();
        Ok(())
    }

    pub fn open_commit_diff(&mut self, commit_id: &str, path: &str) -> Result<(), GitError> {
        if self.editor_tabs.is_open(path) {
            self.editor_tabs.activate(path);
            self.reset_diff_view_offsets();
            self.focus_active_diff_tab();
            return Ok(());
        }
        let Some(repository) = self.repository.as_ref() else {
            return Ok(());
        };
        let file_diff = repository.diff_commit_file(commit_id, path)?;
        self.editor_tabs.open(path, DiffOrigin::Commit, file_diff);
        self.reset_diff_view_offsets();
        self.focus_active_diff_tab();
        Ok(())
    }

    pub fn close_active_tab(&mut self) {
        self.editor_tabs.close_active();
        self.reset_diff_view_offsets();
        self.restore_focus_after_diff_tab();
    }

    fn focus_active_diff_tab(&mut self) {
        if self.focus != PaneId::Log {
            self.focus_before_diff_tab = Some(self.focus);
        }
        self.focus = PaneId::Log;
    }

    fn restore_focus_after_diff_tab(&mut self) {
        if self.editor_tabs.file_diff().is_some() {
            return;
        }
        if let Some(pane) = self.focus_before_diff_tab.take() {
            self.focus = pane;
        }
    }

    fn reset_diff_view_offsets(&mut self) {
        self.diff_scroll = 0;
        self.diff_horizontal_offset = 0;
    }

    pub fn refresh(&mut self) -> Result<(), GitError> {
        self.panel_error = None;
        self.pending_details_query = None;
        let Some(repository) = self.repository.as_ref() else {
            return Ok(());
        };
        let head_info = repository.head_info()?;
        let changes = repository.changes()?;
        let branches = repository.branches()?;
        let commits = repository.log(
            LogQuery {
                max_commits: LOG_MAX_COMMITS,
                skip: 0,
            },
            &self.log_filter(),
        )?;
        let head_commit_short_id = if head_info.detached {
            commits.first().map(|commit| {
                commit
                    .short_id
                    .chars()
                    .take(DETACHED_SHORT_ID_LENGTH)
                    .collect()
            })
        } else {
            None
        };
        self.head_info = head_info;
        self.head_commit_short_id = head_commit_short_id;
        self.prefill_merge_message();
        self.changes_state.replace_entries(changes);
        self.branch_state.replace_branches(branches);
        self.replace_commits(commits);
        self.keep_selected_commit_visible();
        self.keep_selected_branch_visible();
        self.load_selected_commit_files()?;
        self.load_selected_commit_branches_from_ui();
        Ok(())
    }

    fn run_write_operation<T>(
        &mut self,
        operation: impl FnOnce(&dyn GitBackend) -> Result<T, GitError>,
    ) -> Result<T, GitError> {
        let result = match self.repository.as_ref() {
            Some(repository) => operation(repository.as_ref()),
            None => Err(GitError::NoRepository),
        };
        let refreshed = self.refresh();
        match result {
            Ok(value) => {
                if let Err(error) = refreshed {
                    self.show_status_message(
                        format!(
                            "{REFRESH_FAILED_STATUS_PREFIX}{}",
                            single_line_error_text(&error)
                        ),
                        StatusTone::Warning,
                    );
                }
                Ok(value)
            }
            Err(error) => {
                self.record_panel_error(PanelErrorSource::Refresh, &refreshed);
                Err(error)
            }
        }
    }

    fn prefill_merge_message(&mut self) {
        if !self.head_info.merging || !self.commit_state.message().is_empty() {
            return;
        }
        let Some(repository) = self.repository.as_ref() else {
            return;
        };
        let Some(message) = merge_message_prefill(repository.repository_path()) else {
            return;
        };
        self.commit_state.set_message(&message);
    }

    pub fn log_filter(&self) -> LogFilter {
        LogFilter {
            branch: self.log_filter_branch.clone(),
            text: self.log_filter_text.clone(),
            case_sensitive: self.log_filter_case_sensitive,
        }
    }

    pub fn log_filter_case_sensitive(&self) -> bool {
        self.log_filter_case_sensitive
    }

    pub fn log_filter_branch(&self) -> Option<&str> {
        self.log_filter_branch.as_deref()
    }

    fn log_page(&self, skip: usize) -> Result<Vec<CommitSummary>, GitError> {
        let Some(repository) = self.repository.as_ref() else {
            return Ok(Vec::new());
        };
        repository.log(
            LogQuery {
                max_commits: LOG_MAX_COMMITS,
                skip,
            },
            &self.log_filter(),
        )
    }

    fn replace_commits(&mut self, commits: Vec<CommitSummary>) {
        self.log_reached_end = commits.len() < LOG_MAX_COMMITS;
        self.log_state.replace_commits(commits);
        self.refresh_log_filter_status();
    }

    fn refresh_log_filter_status(&mut self) {
        self.log_filter_status = self
            .log_filter_text
            .is_some()
            .then(|| format!("Filtered {} commits", self.log_state.commits().len()));
    }

    fn reload_filtered_log(&mut self) {
        let result = self
            .log_page(0)
            .map(|commits| self.replace_commits(commits));
        self.record_panel_error(PanelErrorSource::Refresh, &result);
        self.keep_selected_commit_visible();
        self.load_selected_commit_files_from_ui();
    }

    fn load_next_log_page(&mut self) {
        if self.log_reached_end {
            return;
        }
        let skip = self.log_state.commits().len();
        let result = self.log_page(skip).map(|commits| {
            if commits.is_empty() {
                self.log_reached_end = true;
                return;
            }
            self.log_state.append_commits(commits);
            self.refresh_log_filter_status();
        });
        self.record_panel_error(PanelErrorSource::Refresh, &result);
    }

    fn load_next_log_page_when_selection_nears_the_end(&mut self) {
        let Some(selected_index) = self.log_state.selected_commit_index() else {
            return;
        };
        if selected_index.saturating_add(LOG_PAGE_LOAD_AHEAD_COMMITS)
            < self.log_state.commits().len()
        {
            return;
        }
        self.load_next_log_page();
    }

    fn load_selected_commit_branches(&mut self) -> Result<(), GitError> {
        let Some(commit_id) = self.log_state.commit_branches_query_target() else {
            return Ok(());
        };
        let Some(repository) = self.repository.as_ref() else {
            return Ok(());
        };
        match repository.branches_containing(&commit_id) {
            Ok(branches) => {
                self.log_state
                    .cache_commit_branches(commit_id, CachedCommitBranches::Listed(branches));
                Ok(())
            }
            Err(error) => {
                self.log_state
                    .cache_commit_branches(commit_id, CachedCommitBranches::Unavailable);
                Err(error)
            }
        }
    }

    fn load_selected_commit_files(&mut self) -> Result<(), GitError> {
        let Some(commit_id) = self
            .log_state
            .selected_commit()
            .map(|commit| commit.id.clone())
        else {
            return Ok(());
        };
        if self.log_state.cached_files(&commit_id).is_some() {
            return Ok(());
        }
        let Some(repository) = self.repository.as_ref() else {
            return Ok(());
        };
        let changed_files = repository.commit_changed_files(&commit_id)?;
        self.log_state.cache_files(commit_id, changed_files);
        Ok(())
    }

    fn keep_selected_commit_visible(&mut self) {
        self.log_state
            .keep_selected_commit_visible(self.commit_viewport_rows());
    }

    fn commit_viewport_rows(&self) -> usize {
        let layout = self.layout();
        usize::from(layout.rows_above_bottom_rule(layout.commits).height.max(1))
    }

    pub fn move_selection(&mut self, delta: isize) {
        self.changes_state.move_selection(delta);
    }

    pub fn toggle_checked_at_cursor(&mut self) {
        self.changes_state.toggle_checked_at_cursor();
    }

    pub fn layout(&self) -> PaneLayout {
        let changes_width = self
            .changes_width
            .unwrap_or_else(|| default_changes_width(self.screen.width));
        compute_pane_layout(
            self.screen,
            changes_width,
            self.details_column_width(),
            self.editor_tabs.file_diff().is_none(),
        )
    }

    fn details_column_width(&self) -> u16 {
        if self.editor_tabs.file_diff().is_some() {
            return DETAILS_WIDTH_HIDDEN;
        }
        self.details_width
            .unwrap_or_else(|| default_details_width(self.screen.width))
    }

    pub fn diff_horizontal_offset(&self) -> u16 {
        self.diff_horizontal_offset
    }

    pub fn checked_hunks_for_path(&self, path: &str) -> Vec<usize> {
        self.hunk_checks
            .get(path)
            .map(|hunk_indices| hunk_indices.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn hunk_staging_available(&self) -> bool {
        self.editor_tabs.active_origin() == Some(DiffOrigin::WorkingTree)
    }

    fn active_diff_checked_hunks(&self) -> Option<&BTreeSet<usize>> {
        let path = self.editor_tabs.active_path()?;
        if !self.hunk_staging_available() {
            return None;
        }
        Some(self.hunk_checks.get(path).unwrap_or(&NO_CHECKED_HUNKS))
    }

    fn toggle_hunk_check_at_the_first_visible_row(&mut self) {
        if !self.hunk_staging_available() {
            self.show_status_message(
                String::from(COMMIT_DIFF_HUNK_STAGING_STATUS),
                StatusTone::Failure,
            );
            return;
        }
        let Some(hunk_index) = self.hunk_index_at_diff_row(self.diff_scroll) else {
            return;
        };
        self.toggle_hunk_check(hunk_index);
    }

    fn toggle_hunk_check(&mut self, hunk_index: usize) {
        let Some(path) = self.editor_tabs.active_path().map(str::to_string) else {
            return;
        };
        let hunk_indices = self.hunk_checks.entry(path.clone()).or_default();
        if !hunk_indices.remove(&hunk_index) {
            hunk_indices.insert(hunk_index);
        }
        if hunk_indices.is_empty() {
            self.hunk_checks.remove(&path);
        }
    }

    fn hunk_index_at_diff_row(&self, display_row_index: usize) -> Option<usize> {
        let diff = self.editor_tabs.file_diff()?;
        let rows = diff_rows(diff);
        let display_rows = diff_display_rows(&rows, self.diff_wrap_width(), self.caps);
        let logical_row_index = display_rows.get(display_row_index)?.logical_row_index;
        rows[..=logical_row_index]
            .iter()
            .rev()
            .find_map(|diff_row| diff_row.hunk_index)
    }

    pub fn hit_divider(&self, column: u16, row: u16) -> Option<DividerId> {
        let layout = self.layout();
        [DividerId::ChangesLog, DividerId::CommitsDetails]
            .into_iter()
            .find(|divider| divider_hit_area_contains(layout.divider_rect(*divider), column, row))
    }

    fn dragged_divider_offset(&self, column: u16) -> i16 {
        let Some(divider) = self.dragged_divider else {
            return 0;
        };
        let divider_column = self.layout().divider_rect(divider).x;
        i16::try_from(i32::from(column) - i32::from(divider_column)).unwrap_or(0)
    }

    pub fn status_left_label(&self) -> String {
        if let Some(short_id) = self.head_commit_short_id.as_deref() {
            return format!("{DETACHED_AT_FORMAT}{short_id}");
        }
        match self.head_info.branch.as_deref() {
            Some(branch) => branch.to_string(),
            None => String::from(NO_REPOSITORY_LABEL),
        }
    }

    fn head_branch_label(&self) -> Option<&str> {
        match self.head_info.branch.as_deref() {
            Some(branch) => Some(branch),
            None => self
                .repository
                .is_some()
                .then_some(DETACHED_HEAD_BRANCH_LABEL),
        }
    }

    fn help_overlay_rows(&self) -> Vec<String> {
        let Some(layout) = help_overlay_layout(self.screen_area()) else {
            return Vec::new();
        };
        wrap_help_rows(&key_binding_rows(), layout.rows.width)
    }

    fn help_overlay_max_scroll(&self) -> usize {
        let visible_rows = help_overlay_visible_row_count(self.screen_area());
        self.help_overlay_rows().len().saturating_sub(visible_rows)
    }

    fn scroll_help_overlay(&mut self, delta: isize) {
        let max_scroll = self.help_overlay_max_scroll();
        self.help_overlay.scroll_by(delta, max_scroll);
    }

    fn apply_help_overlay_key(&mut self, key_input: KeyInput) -> bool {
        if !self.help_overlay.is_open() {
            return false;
        }
        let unmodified = key_input.modifiers == KeyModifiers::default();
        let page_step = isize::try_from(HELP_OVERLAY_PAGE_ROWS).unwrap_or(1);
        match key_input.code {
            KeyCode::Esc => self.help_overlay.close(),
            KeyCode::Char('?') if unmodified => self.help_overlay.close(),
            KeyCode::Char('j') | KeyCode::Down if unmodified => self.scroll_help_overlay(1),
            KeyCode::Char('k') | KeyCode::Up if unmodified => self.scroll_help_overlay(-1),
            KeyCode::PageDown => self.scroll_help_overlay(page_step),
            KeyCode::PageUp => self.scroll_help_overlay(-page_step),
            _ => {}
        }
        true
    }

    fn hovered_rail_footer_label(&self) -> Option<&'static str> {
        activity_rail_footer_label(self.hovered_rail_pane?)
    }

    fn apply_pointer_move(&mut self, mouse_input: MouseInput) {
        if self.help_overlay.is_open() {
            self.hovered_rail_pane = None;
            self.hovered_changes_toolbar = None;
            self.hovered_top_bar_divergence = false;
            return;
        }
        let layout = self.layout();
        self.hovered_rail_pane =
            activity_rail_entry_at(layout.activity_rail, mouse_input.column, mouse_input.row);
        self.hovered_changes_toolbar =
            changes_toolbar_entry_at(layout.changes, mouse_input.column, mouse_input.row);
        self.hovered_top_bar_divergence = self
            .divergence_rect()
            .is_some_and(|rect| rect_contains_cell(rect, mouse_input.column, mouse_input.row));
        self.divergence_hint = self
            .hovered_top_bar_divergence
            .then(|| ahead_behind_hint(self.head_info.ahead, self.head_info.behind));
    }

    fn divergence_rect(&self) -> Option<Rect> {
        top_bar_divergence_rect(
            self.layout().top_bar,
            self.project_name().as_deref(),
            self.head_branch_label(),
            self.head_info.ahead,
            self.head_info.behind,
            self.caps,
        )
    }

    fn apply_top_bar_divergence_press(&mut self, column: u16, row: u16) -> bool {
        let Some(rect) = self.divergence_rect() else {
            return false;
        };
        if !rect_contains_cell(rect, column, row) {
            return false;
        }
        self.open_pull_dialog();
        true
    }

    fn apply_changes_toolbar_press(&mut self, column: u16, row: u16) -> bool {
        let Some(action) = changes_toolbar_entry_at(self.layout().changes, column, row) else {
            return false;
        };
        match action {
            ChangesToolbarAction::Refresh => {
                self.reload();
                self.show_status_message(String::from(REFRESHED_STATUS), StatusTone::Info);
            }
            ChangesToolbarAction::Rollback => self.open_discard_dialog(),
            ChangesToolbarAction::ShowDiff => self.open_selected_diff(),
            ChangesToolbarAction::MoveUp => self.move_selection(-1),
            ChangesToolbarAction::MoveDown => self.move_selection(1),
        }
        true
    }

    fn apply_activity_rail_press(&mut self, column: u16, row: u16) -> bool {
        let Some(pane) = activity_rail_entry_at(self.layout().activity_rail, column, row) else {
            return false;
        };
        self.focus_pane(pane);
        true
    }

    fn apply_top_bar_help_press(&mut self, column: u16, row: u16) -> bool {
        let help_button = top_bar_help_button_rect(self.layout().top_bar);
        if !rect_contains_cell(help_button, column, row) {
            return false;
        }
        self.help_overlay.toggle();
        true
    }

    fn render_help_overlay(&self, area: Rect, buffer: &mut Buffer) {
        if !self.help_overlay.is_open() {
            return;
        }
        let rows = self.help_overlay_rows();
        render_help_overlay(
            area,
            HelpOverlayView {
                rows: &rows,
                scroll: self.help_overlay.scroll(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn focus_pane(&mut self, pane: PaneId) {
        self.end_commit_editing();
        if pane == PaneId::Details && !self.details_pane_is_focusable() {
            if self.editor_tabs.file_diff().is_some() {
                self.show_status_message(
                    String::from(DETAILS_HIDDEN_BY_DIFF_STATUS),
                    StatusTone::Info,
                );
            }
            return;
        }
        self.focus = pane;
    }

    fn focus_next_pane(&mut self) {
        self.end_commit_editing();
        self.focus = self.focus_after_step(1);
    }

    fn focus_previous_pane(&mut self) {
        self.end_commit_editing();
        self.focus = self.focus_after_step(-1);
    }

    fn end_commit_editing(&mut self) {
        self.commit_state.leave_message_editor();
        self.commit_state.set_options_visible(false);
    }

    fn focus_after_step(&self, step: isize) -> PaneId {
        let mut pane = self.step_focus(self.focus, step);
        if pane == PaneId::Details && !self.details_pane_is_focusable() {
            pane = self.step_focus(pane, step);
        }
        pane
    }

    fn step_focus(&self, pane: PaneId, step: isize) -> PaneId {
        let order = [PaneId::Changes, PaneId::Log, PaneId::Details];
        let index = order
            .iter()
            .position(|candidate| *candidate == pane)
            .unwrap_or(0);
        let next_index = (index as isize + step).rem_euclid(order.len() as isize);
        order[usize::try_from(next_index).unwrap_or(0)]
    }

    fn details_pane_is_focusable(&self) -> bool {
        let details = self.layout().details;
        details.width > 0 && details.height > 0
    }

    fn apply_key(&mut self, key_input: KeyInput) {
        if is_quit_key(&key_input) {
            self.apply_quit_key(&key_input);
            return;
        }
        if self.apply_help_overlay_key(key_input) {
            return;
        }
        if self.abort_dialog.is_open() {
            self.apply_abort_dialog_key(key_input);
            return;
        }
        if self.quit_dialog.is_open() {
            self.apply_confirm_dialog_key(ConfirmDialogTarget::Quit, key_input);
            return;
        }
        if self.delete_branch_dialog.is_open() {
            self.apply_delete_branch_dialog_key(key_input);
            return;
        }
        if self.discard_dialog.is_open() {
            self.apply_discard_dialog_key(key_input);
            return;
        }
        if self.new_branch_dialog.is_open() {
            self.apply_new_branch_dialog_key(key_input);
            return;
        }
        if self.branch_filter_dropdown.is_open() {
            self.apply_branch_filter_dropdown_key(key_input);
            return;
        }
        if self.pull_dialog.is_open() {
            self.apply_pull_dialog_key(key_input);
            return;
        }
        if self.fetch_dialog.is_open() {
            self.apply_fetch_dialog_key(key_input);
            return;
        }
        if self.push_dialog.is_open() {
            self.apply_push_dialog_key(key_input);
            return;
        }
        if self.apply_log_filter_key(key_input) {
            return;
        }
        if self.apply_commit_key(key_input) {
            return;
        }
        if self.apply_panel_error_scroll_key(key_input) {
            return;
        }
        let scope = self.key_scope();
        match key_input.code {
            KeyCode::Tab => self.focus_next_pane(),
            KeyCode::BackTab => self.focus_previous_pane(),
            KeyCode::Char('k') if key_input.modifiers.ctrl => self.focus = PaneId::Changes,
            KeyCode::Char('9') if key_input.modifiers.alt => self.focus = PaneId::Log,
            KeyCode::Char('1') if key_input.modifiers.alt => self.focus_pane(PaneId::Changes),
            KeyCode::Char('2') if key_input.modifiers.alt => self.focus_pane(PaneId::Log),
            KeyCode::Char('3') if key_input.modifiers.alt => self.focus_pane(PaneId::Details),
            KeyCode::Char('?') if key_input.modifiers == KeyModifiers::default() => {
                self.help_overlay.toggle()
            }
            KeyCode::Char('b')
                if key_input.modifiers == KeyModifiers::default()
                    && scope.accepts_single_letter_command(SingleLetterCommand::ToggleLogView) =>
            {
                self.toggle_log_view()
            }
            KeyCode::Char('b') if key_input.modifiers.ctrl => self.open_branch_filter_dropdown(),
            KeyCode::Char('f')
                if key_input.modifiers == KeyModifiers::default()
                    && scope
                        .accepts_single_letter_command(SingleLetterCommand::OpenFetchDialog) =>
            {
                self.open_fetch_dialog()
            }
            KeyCode::Char('p')
                if key_input.modifiers == KeyModifiers::default()
                    && scope.accepts_single_letter_command(SingleLetterCommand::OpenPullDialog) =>
            {
                self.open_pull_dialog()
            }
            KeyCode::Char('w') if key_input.modifiers.ctrl => self.close_active_tab(),
            KeyCode::Char('w') if key_input.modifiers.alt => self.toggle_word_highlights(),
            KeyCode::Char('z') if key_input.modifiers.alt => self.toggle_soft_wrap(),
            KeyCode::Char('c') if key_input.modifiers.alt => {
                self.toggle_log_filter_case_sensitive()
            }
            KeyCode::Char('o') if key_input.modifiers.alt => {
                self.resolve_selected_conflict(ConflictResolution::Ours)
            }
            KeyCode::Char('t') if key_input.modifiers.alt => {
                self.resolve_selected_conflict(ConflictResolution::Theirs)
            }
            KeyCode::Char('r') if key_input.modifiers.alt => {
                self.resolve_selected_conflict(ConflictResolution::MarkResolved)
            }
            KeyCode::Char('m') if key_input.modifiers.alt => self.merge_selected_branch(),
            KeyCode::Char('n') if key_input.modifiers.alt => self.open_new_branch_dialog(),
            KeyCode::Char('d') if key_input.modifiers.alt => self.open_discard_dialog(),
            KeyCode::Char('D') if key_input.modifiers.alt => self.open_delete_branch_dialog(),
            KeyCode::Char('i') if key_input.modifiers.alt => self.ignore_selected_untracked_path(),
            KeyCode::Char('x') if key_input.modifiers.alt => self.request_abort_operation(),
            KeyCode::Char('u') if key_input.modifiers.alt => self.continue_operation(),
            KeyCode::Esc => self.close_topmost_layer(),
            KeyCode::Enter
                if self.panel_error.is_some() && key_input.modifiers == KeyModifiers::default() =>
            {
                self.dismiss_panel_error()
            }
            KeyCode::F(5) => self.reload(),
            _ => self.apply_focused_key(key_input),
        }
    }

    fn key_scope(&self) -> KeyScope {
        match self.focus {
            PaneId::Changes => KeyScope::Changes,
            PaneId::Log if self.editor_tabs.file_diff().is_some() => KeyScope::Diff,
            PaneId::Log if self.log_view == LogView::Branches => KeyScope::Branches,
            PaneId::Log => KeyScope::LogList,
            PaneId::Details => KeyScope::Details,
        }
    }

    fn close_topmost_layer(&mut self) {
        if self.panel_error.is_some() {
            self.dismiss_panel_error();
            return;
        }
        self.close_active_tab();
    }

    fn apply_log_filter_key(&mut self, key_input: KeyInput) -> bool {
        if self.log_filter_draft.is_some() {
            return self.apply_log_filter_editor_key(key_input);
        }
        if key_input.modifiers != KeyModifiers::default()
            || key_input.code != KeyCode::Char('/')
            || self.focus != PaneId::Log
            || self.log_view != LogView::Log
            || self.editor_tabs.file_diff().is_some()
        {
            return false;
        }
        self.begin_log_filter_editing();
        true
    }

    fn begin_log_filter_editing(&mut self) {
        self.log_filter_draft = Some(self.log_filter_text.clone().unwrap_or_default());
    }

    fn apply_log_filter_editor_key(&mut self, key_input: KeyInput) -> bool {
        match key_input.code {
            KeyCode::Char(character) if !key_input.modifiers.ctrl && !key_input.modifiers.alt => {
                self.push_filter_draft_character(character);
                true
            }
            KeyCode::Space => {
                self.push_filter_draft_character(' ');
                true
            }
            KeyCode::Backspace => {
                self.remove_filter_draft_last_character();
                true
            }
            KeyCode::Enter => {
                self.apply_log_filter_draft();
                true
            }
            KeyCode::Esc => {
                self.clear_filter_draft_and_filter();
                true
            }
            _ => false,
        }
    }

    fn push_filter_draft_character(&mut self, character: char) {
        if let Some(draft) = self.log_filter_draft.as_mut() {
            draft.push(character);
        }
    }

    fn remove_filter_draft_last_character(&mut self) {
        if let Some(draft) = self.log_filter_draft.as_mut() {
            draft.pop();
        }
    }

    fn toggle_log_filter_case_sensitive(&mut self) {
        self.log_filter_case_sensitive = !self.log_filter_case_sensitive;
        let receipt = if self.log_filter_case_sensitive {
            CASE_SENSITIVE_ON_STATUS
        } else {
            CASE_SENSITIVE_OFF_STATUS
        };
        self.show_status_message(String::from(receipt), StatusTone::Info);
        self.reload_filtered_log();
    }

    fn apply_log_filter_draft(&mut self) {
        let Some(draft) = self.log_filter_draft.take() else {
            return;
        };
        self.log_filter_text = (!draft.trim().is_empty()).then_some(draft);
        self.reload_filtered_log();
    }

    fn clear_filter_draft_and_filter(&mut self) {
        self.log_filter_draft = None;
        if self.log_filter_text.take().is_none() {
            return;
        }
        self.reload_filtered_log();
    }

    fn open_branch_filter_dropdown(&mut self) {
        if self.repository.is_none() || self.branch_filter_dropdown_anchor().is_none() {
            return;
        }
        let selected_index = self.branch_filter_selection_index();
        self.branch_filter_dropdown.open(selected_index);
    }

    fn branch_filter_selection_index(&self) -> usize {
        let Some(branch) = self.log_filter_branch.as_deref() else {
            return 0;
        };
        self.branch_filter_items()
            .iter()
            .position(|item| item.revision.as_deref() == Some(branch))
            .unwrap_or(0)
    }

    fn branch_filter_items(&self) -> Vec<BranchFilterItem> {
        let mut items = vec![BranchFilterItem {
            label: String::from(BRANCH_FILTER_HEAD_LABEL),
            revision: None,
        }];
        let branches = self.branch_state.branches();
        for branch in branches
            .iter()
            .filter(|branch| branch.kind == RefKind::LocalBranch)
        {
            items.push(BranchFilterItem {
                label: format!(
                    "{}{}",
                    if branch.is_head {
                        BRANCH_FILTER_CURRENT_BRANCH_PREFIX
                    } else {
                        ""
                    },
                    branch.name
                ),
                revision: Some(branch.name.clone()),
            });
        }
        for branch in branches
            .iter()
            .filter(|branch| branch.kind == RefKind::RemoteBranch)
        {
            items.push(BranchFilterItem {
                label: branch.name.clone(),
                revision: Some(branch.name.clone()),
            });
        }
        items
    }

    fn branch_filter_item_labels(&self) -> Vec<String> {
        self.branch_filter_items()
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    fn branch_filter_dropdown_anchor(&self) -> Option<Rect> {
        branch_filter_block_rect(self.layout().log_filters, self.log_filter_branch.as_deref())
    }

    fn branch_filter_dropdown_layout(&self) -> Option<BranchFilterDropdownLayout> {
        let anchor = self.branch_filter_dropdown_anchor()?;
        let labels = self.branch_filter_item_labels();
        branch_filter_dropdown_layout(anchor, &labels, self.screen_area())
    }

    fn apply_branch_filter_dropdown_key(&mut self, key_input: KeyInput) {
        match key_input.code {
            KeyCode::Up => self.move_branch_filter_selection(-1),
            KeyCode::Down => self.move_branch_filter_selection(1),
            KeyCode::Enter => self.apply_selected_branch_filter(),
            KeyCode::Esc => self.branch_filter_dropdown.close(),
            _ => {}
        }
    }

    fn move_branch_filter_selection(&mut self, delta: isize) {
        let item_count = self.branch_filter_items().len();
        self.branch_filter_dropdown.select_by(delta, item_count);
    }

    fn apply_selected_branch_filter(&mut self) {
        let items = self.branch_filter_items();
        let Some(item) = items.get(self.branch_filter_dropdown.selected_index()) else {
            self.branch_filter_dropdown.close();
            return;
        };
        self.log_filter_branch = item.revision.clone();
        self.branch_filter_dropdown.close();
        self.reload_filtered_log();
    }

    fn apply_branch_filter_dropdown_mouse(&mut self, mouse_input: MouseInput) {
        match mouse_input.kind {
            MouseInputKind::ScrollUp => self.move_branch_filter_selection(-1),
            MouseInputKind::ScrollDown => self.move_branch_filter_selection(1),
            MouseInputKind::Down if mouse_input.button == MouseButton::Left => {
                self.apply_branch_filter_dropdown_press(mouse_input.column, mouse_input.row);
            }
            _ => {}
        }
    }

    fn apply_branch_filter_dropdown_press(&mut self, column: u16, row: u16) {
        let Some(layout) = self.branch_filter_dropdown_layout() else {
            self.branch_filter_dropdown.close();
            return;
        };
        if !rect_contains_cell(layout.frame, column, row) {
            self.branch_filter_dropdown.close();
            return;
        }
        let Some(visible_row) = layout.row_index_at(layout.rows.x, row) else {
            return;
        };
        let item_index = self
            .branch_filter_dropdown
            .scroll()
            .saturating_add(visible_row);
        self.branch_filter_dropdown.select_index(item_index);
        self.apply_selected_branch_filter();
    }

    fn apply_commit_key(&mut self, key_input: KeyInput) -> bool {
        if self.focus == PaneId::Changes {
            if self.commit_state.options_visible() && key_input.code == KeyCode::Esc {
                self.commit_state.set_options_visible(false);
                return true;
            }
            if self.commit_state.is_editing() {
                if key_input.code == KeyCode::Esc {
                    self.commit_state.leave_message_editor();
                    return true;
                }
                if self.apply_message_editor_key(key_input) {
                    return true;
                }
            }
        }
        if !self
            .key_scope()
            .accepts_single_letter_command(SingleLetterCommand::EditCommitMessage)
        {
            return false;
        }
        if key_input.modifiers.ctrl && key_input.code == KeyCode::Enter {
            self.commit_from_ui();
            return true;
        }
        if key_input.modifiers.alt {
            return match key_input.code {
                KeyCode::Char('a') => {
                    self.toggle_amend();
                    true
                }
                KeyCode::Char('s') => {
                    self.commit_state.toggle_sign_off();
                    true
                }
                _ => false,
            };
        }
        if key_input.modifiers == KeyModifiers::default() && key_input.code == KeyCode::Char('i') {
            self.begin_commit_message_editing();
            return true;
        }
        false
    }

    fn begin_commit_message_editing(&mut self) {
        self.focus = PaneId::Changes;
        self.commit_state.enter_message_editor();
    }

    fn apply_message_editor_key(&mut self, key_input: KeyInput) -> bool {
        if key_input.modifiers.ctrl {
            if key_input.code == KeyCode::Char('z') {
                self.commit_state.undo();
                return true;
            }
            return false;
        }
        if key_input.modifiers.alt {
            return false;
        }
        match key_input.code {
            KeyCode::Char(character) => {
                self.commit_state.insert_character(character);
                true
            }
            KeyCode::Space => {
                self.commit_state.insert_character(' ');
                true
            }
            KeyCode::Enter => {
                self.commit_state.insert_newline();
                true
            }
            KeyCode::Backspace => {
                self.commit_state.remove_last_grapheme();
                true
            }
            _ => false,
        }
    }

    fn apply_push_dialog_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match key_input.code {
            KeyCode::Tab => self.push_dialog.focus_by(1),
            KeyCode::BackTab => self.push_dialog.focus_by(-1),
            KeyCode::Left | KeyCode::Right
                if self.push_dialog.focus() == PushDialogFocus::Buttons =>
            {
                let step = if key_input.code == KeyCode::Right {
                    1
                } else {
                    -1
                };
                self.push_dialog.select_button_by(step);
            }
            KeyCode::Space => self.push_dialog.toggle_focused_option(),
            KeyCode::Enter => self.activate_push_dialog_button(),
            KeyCode::Esc => self.push_dialog.close(),
            _ => {}
        }
    }

    fn apply_pull_dialog_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match key_input.code {
            KeyCode::Up => self.pull_dialog.step_strategy_by(-1),
            KeyCode::Down => self.pull_dialog.step_strategy_by(1),
            KeyCode::Tab => self.pull_dialog.focus_by(1),
            KeyCode::BackTab => self.pull_dialog.focus_by(-1),
            KeyCode::Left | KeyCode::Right
                if self.pull_dialog.focus() == PullDialogFocus::Buttons =>
            {
                let step = if key_input.code == KeyCode::Right {
                    1
                } else {
                    -1
                };
                self.pull_dialog.select_button_by(step);
            }
            KeyCode::Space if self.pull_dialog.focus() == PullDialogFocus::Buttons => {
                self.activate_pull_dialog_button();
            }
            KeyCode::Enter => self.activate_pull_dialog_button(),
            KeyCode::Esc => self.pull_dialog.close(),
            _ => {}
        }
    }

    fn activate_pull_dialog_button(&mut self) {
        match self.pull_dialog.button() {
            PullDialogButton::Pull => self.confirm_pull_dialog(),
            PullDialogButton::Cancel => self.pull_dialog.close(),
        }
    }

    fn apply_new_branch_dialog_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match key_input.code {
            KeyCode::Char(character) if !character.is_control() => {
                self.new_branch_dialog.push_char(character)
            }
            KeyCode::Backspace => self.new_branch_dialog.pop_char(),
            KeyCode::Enter => self.create_branch_from_selected_branch(),
            KeyCode::Esc => self.new_branch_dialog.close(),
            _ => {}
        }
    }

    fn apply_delete_branch_dialog_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match key_input.code {
            KeyCode::Tab | KeyCode::Right => self.delete_branch_dialog.select_button_by(1),
            KeyCode::BackTab | KeyCode::Left => self.delete_branch_dialog.select_button_by(-1),
            KeyCode::Enter | KeyCode::Space => self.activate_delete_branch_dialog_button(),
            KeyCode::Esc => self.close_delete_branch_dialog(),
            _ => {}
        }
    }

    fn apply_discard_dialog_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match key_input.code {
            KeyCode::Tab | KeyCode::Right => self.discard_dialog.select_button_by(1),
            KeyCode::BackTab | KeyCode::Left => self.discard_dialog.select_button_by(-1),
            KeyCode::Enter | KeyCode::Space => self.activate_discard_dialog_button(),
            KeyCode::Esc => self.close_discard_dialog(),
            _ => {}
        }
    }

    fn apply_fetch_dialog_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match key_input.code {
            KeyCode::Tab | KeyCode::Right => self.fetch_dialog.select_button_by(1),
            KeyCode::BackTab | KeyCode::Left => self.fetch_dialog.select_button_by(-1),
            KeyCode::Enter | KeyCode::Space => self.activate_fetch_dialog_button(),
            KeyCode::Esc => self.fetch_dialog.close(),
            _ => {}
        }
    }

    fn activate_fetch_dialog_button(&mut self) {
        match self.fetch_dialog.button() {
            FetchDialogButton::Fetch => self.confirm_fetch_dialog(),
            FetchDialogButton::Cancel => self.fetch_dialog.close(),
        }
    }

    fn activate_push_dialog_button(&mut self) {
        match self.push_dialog.button() {
            PushDialogButton::Push => self.confirm_push_dialog(),
            PushDialogButton::Cancel => self.push_dialog.close(),
        }
    }

    fn commit_from_ui(&mut self) {
        self.apply_commit_request(false);
    }

    fn commit_and_push_from_ui(&mut self) {
        self.apply_commit_request(true);
    }

    fn apply_commit_request(&mut self, also_push: bool) {
        if let Some(operation) = self.head_info.operation {
            if !operation.is_merging() {
                self.show_status_message(
                    format!("{OPERATION_CONTINUATION_HINT_PREFIX}{}", operation.noun()),
                    StatusTone::Failure,
                );
                return;
            }
        }
        let merging = self.head_info.merging;
        if self.commit_state.message().trim().is_empty() {
            self.show_status_message(
                String::from(EMPTY_COMMIT_MESSAGE_STATUS),
                StatusTone::Failure,
            );
            return;
        }
        let committed_paths = self.committed_paths();
        if committed_paths.is_empty() && !self.commit_state.amend() && !self.head_info.merging {
            self.show_status_message(String::from(NOTHING_TO_COMMIT_STATUS), StatusTone::Failure);
            return;
        }
        let amended = self.commit_state.amend();
        let amended_untracked_file_count = if amended {
            self.checked_untracked_file_count()
        } else {
            0
        };
        let result = if also_push {
            self.commit_and_push()
        } else {
            self.commit()
        };
        let Ok(outcome) = result else {
            return;
        };
        let committed_file_count = if committed_paths.is_empty() && !amended {
            match self.resulting_commit_file_count(&outcome.commit_id) {
                Ok(file_count) => file_count,
                Err(error) => {
                    self.set_panel_error(PanelErrorSource::Refresh, &error);
                    return;
                }
            }
        } else {
            committed_paths.len()
        };
        if merging {
            let branch = self.head_branch_label().unwrap_or_default().to_string();
            self.show_status_message(format!("{branch} merged and committed"), StatusTone::Info);
            return;
        }
        if amended_untracked_file_count > 0 {
            self.show_status_message(
                untracked_amended_receipt(amended_untracked_file_count),
                StatusTone::Warning,
            );
            return;
        }
        self.show_commit_receipt(committed_file_count, amended);
    }

    fn resulting_commit_file_count(&self, commit_id: &str) -> Result<usize, GitError> {
        let Some(repository) = self.repository.as_ref() else {
            return Err(GitError::NoRepository);
        };
        Ok(repository.commit_changed_files(commit_id)?.len())
    }

    fn show_commit_receipt(&mut self, committed_file_count: usize, amended: bool) {
        if amended {
            self.show_status_message(
                amended_commit_receipt(committed_file_count),
                StatusTone::Info,
            );
            return;
        }
        let Some(branch) = self.head_branch_label().map(str::to_string) else {
            return;
        };
        self.show_status_message(
            format!(
                "Committed {} to {branch}",
                file_count_label(committed_file_count)
            ),
            StatusTone::Info,
        );
    }

    fn toggle_amend(&mut self) {
        let amend = !self.commit_state.amend();
        self.commit_state.set_amend(amend);
        if !amend {
            return;
        }
        let last_message = self
            .repository
            .as_ref()
            .map(|repository| repository.last_commit_message());
        match last_message {
            Some(Ok(message)) => self.commit_state.set_message(&message),
            Some(Err(error)) => {
                self.panel_error = Some(PanelError::new(
                    PanelErrorSource::Refresh,
                    single_line_error_text(&error),
                ))
            }
            None => {}
        }
    }

    fn apply_panel_error_scroll_key(&mut self, key_input: KeyInput) -> bool {
        if self.panel_error.is_none() || key_input.modifiers != KeyModifiers::default() {
            return false;
        }
        match key_input.code {
            KeyCode::Up => self.scroll_panel_error_body(-1),
            KeyCode::Down => self.scroll_panel_error_body(1),
            _ => return false,
        }
        true
    }

    fn scroll_panel_error_body(&mut self, delta: isize) {
        let (visible_rows, total_rows) = self.panel_error_body_metrics();
        let Some(panel_error) = self.panel_error.as_mut() else {
            return;
        };
        if delta < 0 {
            panel_error.scroll = panel_error.scroll.saturating_sub(1);
            return;
        }
        if panel_error.scroll + visible_rows < total_rows {
            panel_error.scroll += 1;
        }
    }

    fn panel_error_body_metrics(&self) -> (usize, usize) {
        let Some(panel_error) = self.panel_error.as_ref() else {
            return (0, 0);
        };
        let Some(panel) = changes_panel_layout(self.layout().changes, self.amend()) else {
            return (0, 0);
        };
        let Some(error_layout) = panel_error_layout(panel.list, self.screen_area()) else {
            return (0, 0);
        };
        let hint_rows = usize::from(next_step_hint(&panel_error.message).is_some());
        let visible_rows = usize::from(error_layout.body.height).saturating_sub(hint_rows);
        (
            visible_rows,
            error_body_line_count(&panel_error.message, error_layout.body.width),
        )
    }

    fn apply_focused_key(&mut self, key_input: KeyInput) {
        if key_input.modifiers != KeyModifiers::default() {
            return;
        }
        match self.focus {
            PaneId::Changes => {
                if key_input.code == KeyCode::Enter {
                    self.open_selected_diff();
                    return;
                }
                if self.editor_tabs.file_diff().is_some()
                    && matches!(key_input.code, KeyCode::Left | KeyCode::Right)
                {
                    self.apply_diff_key(key_input.code);
                    return;
                }
                self.apply_changes_key(key_input.code);
            }
            PaneId::Log => {
                if self.editor_tabs.file_diff().is_some() {
                    self.apply_diff_key(key_input.code);
                    return;
                }
                if self.log_view == LogView::Branches {
                    self.apply_branch_key(key_input.code);
                    return;
                }
                if key_input.code == KeyCode::Enter {
                    self.open_selected_commit_first_file_diff();
                    return;
                }
                self.apply_log_key(key_input.code);
            }
            PaneId::Details => {}
        }
    }

    fn apply_branch_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('j') | KeyCode::Down => self.select_branch(1),
            KeyCode::Char('k') | KeyCode::Up => self.select_branch(-1),
            KeyCode::PageDown => self.select_branch(LOG_PAGE_JUMP),
            KeyCode::PageUp => self.select_branch(-LOG_PAGE_JUMP),
            KeyCode::Home => self.select_branch_first(),
            KeyCode::End => self.select_branch_last(),
            KeyCode::Enter => self.checkout_selected_branch(),
            _ => {}
        }
    }

    fn select_branch_first(&mut self) {
        let Some(selected_index) = self.branch_state.selected_index() else {
            return;
        };
        self.select_branch(-isize::try_from(selected_index).unwrap_or(isize::MAX));
    }

    fn select_branch_last(&mut self) {
        let Some(selected_index) = self.branch_state.selected_index() else {
            return;
        };
        let last_index = self.branches().len().saturating_sub(1);
        let delta =
            isize::try_from(last_index).unwrap_or(0) - isize::try_from(selected_index).unwrap_or(0);
        self.select_branch(delta);
    }

    fn apply_log_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('j') | KeyCode::Down => self.select_commit(1),
            KeyCode::Char('k') | KeyCode::Up => self.select_commit(-1),
            KeyCode::PageDown => self.select_commit(LOG_PAGE_JUMP),
            KeyCode::PageUp => self.select_commit(-LOG_PAGE_JUMP),
            KeyCode::Home => self.select_newest_commit(),
            KeyCode::End => self.select_oldest_commit(),
            _ => {}
        }
    }

    fn select_newest_commit(&mut self) {
        let Some(selected_index) = self.log_state.selected_commit_index() else {
            return;
        };
        self.select_commit(-isize::try_from(selected_index).unwrap_or(isize::MAX));
    }

    fn select_oldest_commit(&mut self) {
        let Some(selected_index) = self.log_state.selected_commit_index() else {
            return;
        };
        let last_index = self.log_state.commits().len().saturating_sub(1);
        let delta =
            isize::try_from(last_index).unwrap_or(0) - isize::try_from(selected_index).unwrap_or(0);
        self.select_commit(delta);
    }

    fn apply_diff_key(&mut self, key_code: KeyCode) {
        let page_rows = isize::try_from(self.layout().commits.height.max(1)).unwrap_or(1);
        match key_code {
            KeyCode::Char('j') | KeyCode::Down => self.scroll_diff_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_diff_by(-1),
            KeyCode::PageDown => self.scroll_diff_by(page_rows),
            KeyCode::PageUp => self.scroll_diff_by(-page_rows),
            KeyCode::Right => {
                self.scroll_diff_horizontally(i32::from(DIFF_HORIZONTAL_STEP_COLUMNS))
            }
            KeyCode::Left => {
                self.scroll_diff_horizontally(-i32::from(DIFF_HORIZONTAL_STEP_COLUMNS))
            }
            KeyCode::Space => self.toggle_hunk_check_at_the_first_visible_row(),
            _ => {}
        }
    }

    fn toggle_word_highlights(&mut self) {
        self.word_highlights = !self.word_highlights;
        let status = if self.word_highlights {
            WORD_HIGHLIGHTS_ON_STATUS
        } else {
            WORD_HIGHLIGHTS_OFF_STATUS
        };
        self.show_status_message(String::from(status), StatusTone::Info);
    }

    fn toggle_soft_wrap(&mut self) {
        self.soft_wrap = !self.soft_wrap;
        if self.soft_wrap {
            self.diff_horizontal_offset = 0;
        }
        let status = if self.soft_wrap {
            SOFT_WRAP_ON_STATUS
        } else {
            SOFT_WRAP_OFF_STATUS
        };
        self.show_status_message(String::from(status), StatusTone::Info);
    }

    fn scroll_diff_horizontally(&mut self, delta: i32) {
        if self.soft_wrap {
            return;
        }
        let maximum_offset = self.diff_horizontal_scroll_limit();
        let target = i32::from(self.diff_horizontal_offset).saturating_add(delta);
        self.diff_horizontal_offset =
            u16::try_from(target.clamp(0, i32::from(maximum_offset))).unwrap_or(0);
    }

    pub fn soft_wrap(&self) -> bool {
        self.soft_wrap
    }

    fn diff_wrap_width(&self) -> Option<u16> {
        if !self.soft_wrap {
            return None;
        }
        let width = self.diff_text_visible_width();
        (width > 0).then_some(width)
    }

    fn diff_display_row_count(&self) -> Option<usize> {
        let diff = self.editor_tabs.file_diff()?;
        let rows = diff_rows(diff);
        Some(diff_display_rows(&rows, self.diff_wrap_width(), self.caps).len())
    }

    fn diff_horizontal_scroll_limit(&self) -> u16 {
        self.longest_scrollable_diff_row_width()
            .saturating_sub(self.diff_text_visible_width())
    }

    fn longest_scrollable_diff_row_width(&self) -> u16 {
        let Some(diff) = self.editor_tabs.file_diff() else {
            return 0;
        };
        diff_rows(diff)
            .iter()
            .filter(|diff_row| diff_row.kind != DiffRowKind::HunkHeader)
            .map(|diff_row| text_display_width(diff_row.text.as_ref()))
            .max()
            .unwrap_or(0)
    }

    fn diff_text_visible_width(&self) -> u16 {
        let layout = self.layout();
        layout
            .rows_above_bottom_rule(layout.log_content_rect())
            .width
            .saturating_sub(DIFF_TEXT_COLUMN)
    }

    fn scroll_diff_by(&mut self, delta: isize) {
        let Some(display_row_count) = self.diff_display_row_count() else {
            return;
        };
        let last_scroll_row = display_row_count.saturating_sub(1);
        let target_row = isize::try_from(self.diff_scroll)
            .unwrap_or(isize::MAX)
            .saturating_add(delta)
            .clamp(0, isize::try_from(last_scroll_row).unwrap_or(isize::MAX));
        self.diff_scroll = usize::try_from(target_row).unwrap_or(0);
    }

    fn open_selected_diff(&mut self) {
        let Some(path) = self.changes_state.selected_file_path().map(str::to_string) else {
            self.warn_select_file_row();
            return;
        };
        let result = self.open_diff(&path);
        self.record_panel_error(PanelErrorSource::Refresh, &result);
    }

    fn open_selected_commit_first_file_diff(&mut self) {
        let Some(commit_id) = self
            .log_state
            .selected_commit()
            .map(|commit| commit.id.clone())
        else {
            return;
        };
        let Some(path) = self
            .log_state
            .files_at_selected_commit()
            .first()
            .map(|file| file.path.clone())
        else {
            return;
        };
        let result = self.open_commit_diff(&commit_id, &path);
        self.record_panel_error(PanelErrorSource::Refresh, &result);
    }

    fn apply_changes_key(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Space => self.toggle_checked_at_cursor(),
            KeyCode::Left => self.changes_state.set_selected_group_collapsed(true),
            KeyCode::Right => self.changes_state.set_selected_group_collapsed(false),
            _ => {}
        }
    }

    fn reload(&mut self) {
        let result = self.refresh();
        self.record_panel_error(PanelErrorSource::Refresh, &result);
    }

    fn apply_mouse(&mut self, mouse_input: MouseInput) {
        if self.help_overlay.is_open() {
            self.apply_help_overlay_mouse(mouse_input);
            return;
        }
        if mouse_input.kind == MouseInputKind::Move {
            self.apply_pointer_move(mouse_input);
            return;
        }
        if self.abort_dialog.is_open() {
            self.apply_confirm_dialog_mouse(ConfirmDialogTarget::Abort, mouse_input);
            return;
        }
        if self.quit_dialog.is_open() {
            self.apply_confirm_dialog_mouse(ConfirmDialogTarget::Quit, mouse_input);
            return;
        }
        if self.discard_dialog.is_open() {
            self.apply_discard_dialog_mouse(mouse_input);
            return;
        }
        if self.branch_filter_dropdown.is_open() {
            self.apply_branch_filter_dropdown_mouse(mouse_input);
            return;
        }
        if self.pull_dialog.is_open() {
            self.apply_pull_dialog_mouse(mouse_input);
            return;
        }
        if self.fetch_dialog.is_open() {
            self.apply_fetch_dialog_mouse(mouse_input);
            return;
        }
        if self.push_dialog.is_open() {
            self.apply_push_dialog_mouse(mouse_input);
            return;
        }
        match mouse_input.kind {
            MouseInputKind::ScrollUp => self.scroll_hovered_pane(-WHEEL_STEP_ROWS, mouse_input),
            MouseInputKind::ScrollDown => self.scroll_hovered_pane(WHEEL_STEP_ROWS, mouse_input),
            MouseInputKind::Down if mouse_input.button == MouseButton::Left => {
                self.apply_left_press(mouse_input)
            }
            MouseInputKind::Drag if mouse_input.button == MouseButton::Left => {
                self.drag_divider_to(mouse_input.column)
            }
            MouseInputKind::Up if mouse_input.button == MouseButton::Left => {
                self.dragged_divider = None
            }
            _ => {}
        }
    }

    fn apply_help_overlay_mouse(&mut self, mouse_input: MouseInput) {
        match mouse_input.kind {
            MouseInputKind::ScrollUp => self.scroll_help_overlay(-WHEEL_STEP_ROWS),
            MouseInputKind::ScrollDown => self.scroll_help_overlay(WHEEL_STEP_ROWS),
            MouseInputKind::Down if mouse_input.button == MouseButton::Left => {
                self.apply_help_overlay_left_press(mouse_input.column, mouse_input.row)
            }
            _ => {}
        }
    }

    fn apply_help_overlay_left_press(&mut self, column: u16, row: u16) {
        if rect_contains_cell(top_bar_help_button_rect(self.layout().top_bar), column, row) {
            self.help_overlay.close();
            return;
        }
        let inside_frame = help_overlay_layout(self.screen_area())
            .is_some_and(|layout| rect_contains_cell(layout.frame, column, row));
        if !inside_frame {
            self.help_overlay.close();
        }
    }

    fn apply_push_dialog_mouse(&mut self, mouse_input: MouseInput) {
        if mouse_input.kind != MouseInputKind::Down || mouse_input.button != MouseButton::Left {
            return;
        }
        let Some(layout) = push_dialog_layout(self.screen_area()) else {
            return;
        };
        if rect_contains_cell(layout.set_upstream_row, mouse_input.column, mouse_input.row) {
            self.push_dialog.select_option(PushDialogFocus::SetUpstream);
            return;
        }
        if rect_contains_cell(
            layout.force_with_lease_row,
            mouse_input.column,
            mouse_input.row,
        ) {
            self.push_dialog
                .select_option(PushDialogFocus::ForceWithLease);
            return;
        }
        if rect_contains_cell(layout.push_button, mouse_input.column, mouse_input.row) {
            self.push_dialog.select_button(PushDialogButton::Push);
            self.activate_push_dialog_button();
            return;
        }
        if rect_contains_cell(layout.cancel_button, mouse_input.column, mouse_input.row) {
            self.push_dialog.select_button(PushDialogButton::Cancel);
            self.activate_push_dialog_button();
        }
    }

    fn apply_confirm_dialog_mouse(&mut self, target: ConfirmDialogTarget, mouse_input: MouseInput) {
        if mouse_input.kind != MouseInputKind::Down || mouse_input.button != MouseButton::Left {
            return;
        }
        let Some(layout) = confirm_dialog_layout(self.screen_area()) else {
            return;
        };
        for (rect, button) in [
            (layout.confirm_button, ConfirmDialogButton::Confirm),
            (layout.cancel_button, ConfirmDialogButton::Cancel),
        ] {
            if rect_contains_cell(rect, mouse_input.column, mouse_input.row) {
                target.state_mut(self).select_button(button);
                self.activate_confirm_dialog_button(target);
                return;
            }
        }
    }

    fn apply_discard_dialog_mouse(&mut self, mouse_input: MouseInput) {
        if mouse_input.kind != MouseInputKind::Down || mouse_input.button != MouseButton::Left {
            return;
        }
        let Some(layout) = discard_dialog_layout(self.screen_area()) else {
            return;
        };
        if rect_contains_cell(layout.discard_button, mouse_input.column, mouse_input.row) {
            self.discard_dialog
                .select_button(DiscardDialogButton::Discard);
            self.activate_discard_dialog_button();
            return;
        }
        if rect_contains_cell(layout.cancel_button, mouse_input.column, mouse_input.row) {
            self.discard_dialog
                .select_button(DiscardDialogButton::Cancel);
            self.activate_discard_dialog_button();
        }
    }

    fn apply_pull_dialog_mouse(&mut self, mouse_input: MouseInput) {
        if mouse_input.kind != MouseInputKind::Down || mouse_input.button != MouseButton::Left {
            return;
        }
        let Some(layout) = pull_dialog_layout(self.screen_area()) else {
            return;
        };
        for option in gitdeck_tui::PULL_STRATEGY_OPTIONS {
            if rect_contains_cell(
                layout.strategy_row(option),
                mouse_input.column,
                mouse_input.row,
            ) {
                self.pull_dialog.select_strategy(option);
                return;
            }
        }
        if rect_contains_cell(layout.pull_button, mouse_input.column, mouse_input.row) {
            self.pull_dialog.select_button(PullDialogButton::Pull);
            self.activate_pull_dialog_button();
            return;
        }
        if rect_contains_cell(layout.cancel_button, mouse_input.column, mouse_input.row) {
            self.pull_dialog.select_button(PullDialogButton::Cancel);
            self.activate_pull_dialog_button();
        }
    }

    fn apply_fetch_dialog_mouse(&mut self, mouse_input: MouseInput) {
        if mouse_input.kind != MouseInputKind::Down || mouse_input.button != MouseButton::Left {
            return;
        }
        let Some(layout) = fetch_dialog_layout(self.screen_area()) else {
            return;
        };
        if rect_contains_cell(layout.fetch_button, mouse_input.column, mouse_input.row) {
            self.fetch_dialog.select_button(FetchDialogButton::Fetch);
            self.activate_fetch_dialog_button();
            return;
        }
        if rect_contains_cell(layout.cancel_button, mouse_input.column, mouse_input.row) {
            self.fetch_dialog.select_button(FetchDialogButton::Cancel);
            self.activate_fetch_dialog_button();
        }
    }

    fn screen_area(&self) -> Rect {
        Rect::new(0, 0, self.screen.width, self.screen.height)
    }

    fn render_branch_filter_dropdown_overlay(&self, area: Rect, buffer: &mut Buffer) {
        if !self.branch_filter_dropdown.is_open() {
            return;
        }
        let Some(anchor) = self.branch_filter_dropdown_anchor() else {
            return;
        };
        let items = self.branch_filter_item_labels();
        render_branch_filter_dropdown(
            anchor,
            area,
            BranchFilterDropdownView {
                items: &items,
                selected_index: self.branch_filter_dropdown.selected_index(),
                scroll: self.branch_filter_dropdown.scroll(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_dialog_overlays(&self, area: Rect, buffer: &mut Buffer) {
        self.render_pull_dialog_overlay(area, buffer);
        self.render_fetch_dialog_overlay(area, buffer);
        self.render_push_dialog_overlay(area, buffer);
        self.render_discard_dialog_overlay(area, buffer);
        self.render_delete_branch_dialog_overlay(area, buffer);
        self.render_new_branch_dialog_overlay(area, buffer);
        self.render_abort_dialog_overlay(area, buffer);
        self.render_quit_dialog_overlay(area, buffer);
    }

    fn render_abort_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        let Some(operation) = self.abort_dialog_operation else {
            return;
        };
        if !self.abort_dialog.is_open() {
            return;
        }
        render_confirm_dialog(
            area,
            ConfirmDialogView {
                title: &format!("{ABORT_DIALOG_TITLE_PREFIX}{}", operation.title()),
                message: &ABORT_DIALOG_MESSAGE_TEMPLATE.replace("{noun}", operation.noun()),
                detail: Some(ABORT_DIALOG_DETAIL),
                confirm_label: ABORT_CONFIRM_BUTTON_LABEL,
                button: self.abort_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_quit_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        if !self.quit_dialog.is_open() {
            return;
        }
        let Some(message) = self.quit_dialog_message() else {
            return;
        };
        render_confirm_dialog(
            area,
            ConfirmDialogView {
                title: QUIT_DIALOG_TITLE,
                message,
                detail: None,
                confirm_label: QUIT_CONFIRM_BUTTON_LABEL,
                button: self.quit_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_new_branch_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        let Some(name) = self.new_branch_dialog.name() else {
            return;
        };
        render_new_branch_dialog(
            area,
            NewBranchDialogView {
                name,
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_discard_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        let Some(path) = self.discard_dialog.path() else {
            return;
        };
        render_discard_dialog(
            area,
            DiscardDialogView {
                path,
                button: self.discard_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_delete_branch_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        let Some(name) = self.delete_branch_dialog.name() else {
            return;
        };
        render_delete_branch_dialog(
            area,
            DeleteBranchDialogView {
                name,
                button: self.delete_branch_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_pull_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        if !self.pull_dialog.is_open() {
            return;
        }
        render_pull_dialog(
            area,
            PullDialogView {
                branch_label: self.pull_dialog_branch_label(),
                strategy: self.pull_dialog.strategy(),
                focus: self.pull_dialog.focus(),
                button: self.pull_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_fetch_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        if !self.fetch_dialog.is_open() {
            return;
        }
        render_fetch_dialog(
            area,
            FetchDialogView {
                remote_available: self.fetch_dialog_remote_available(),
                button: self.fetch_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn render_push_dialog_overlay(&self, area: Rect, buffer: &mut Buffer) {
        if !self.push_dialog.is_open() {
            return;
        }
        render_push_dialog(
            area,
            PushDialogView {
                branch_label: self.push_dialog_branch_label(),
                set_upstream: self.push_dialog.set_upstream(),
                force_with_lease: self.push_dialog.force_with_lease(),
                focus: self.push_dialog.focus(),
                button: self.push_dialog.button(),
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
    }

    fn apply_branch_filter_chip_press(&mut self, column: u16, row: u16) -> bool {
        if !self.filter_bar_shows_blocks() {
            return false;
        }
        let Some(anchor) = self.branch_filter_dropdown_anchor() else {
            return false;
        };
        if !rect_contains_cell(anchor, column, row) {
            return false;
        }
        self.open_branch_filter_dropdown();
        true
    }

    fn filter_bar_shows_blocks(&self) -> bool {
        self.log_filter_text.is_none() && self.log_filter_draft.is_none()
    }

    fn apply_remote_tool_press(&mut self, column: u16, row: u16) -> bool {
        let layout = self.layout();
        let rects = remote_tool_bar_rects(
            layout.top_bar,
            self.project_name().as_deref(),
            self.head_branch_label(),
            self.head_info.ahead,
            self.head_info.behind,
            self.caps,
        );
        for (rect, label) in rects.into_iter().zip(REMOTE_TOOL_BAR_LABELS) {
            if !rect_contains_cell(rect, column, row) {
                continue;
            }
            match label {
                REMOTE_FETCH_TOOL_LABEL => self.open_fetch_dialog(),
                REMOTE_PULL_TOOL_LABEL => self.open_pull_dialog(),
                REMOTE_PUSH_TOOL_LABEL => self.open_push_dialog(),
                _ => {}
            }
            return true;
        }
        false
    }

    fn scroll_hovered_pane(&mut self, delta: isize, mouse_input: MouseInput) {
        match self.pane_under_pointer(mouse_input.column, mouse_input.row) {
            Some(HoveredPane::Changes) => self.move_selection(delta),
            Some(HoveredPane::Commits) => self.scroll_log_list(delta),
            Some(HoveredPane::Diff) => self.scroll_diff_by(delta),
            Some(HoveredPane::Details) | None => {}
        }
    }

    fn pane_under_pointer(&self, column: u16, row: u16) -> Option<HoveredPane> {
        let layout = self.layout();
        if rect_contains_cell(layout.changes, column, row) {
            return Some(HoveredPane::Changes);
        }
        let shows_diff = self.editor_tabs.file_diff().is_some();
        if shows_diff && rect_contains_cell(layout.log_content_rect(), column, row) {
            return Some(HoveredPane::Diff);
        }
        if !shows_diff && rect_contains_cell(layout.commits, column, row) {
            return Some(HoveredPane::Commits);
        }
        if !shows_diff && rect_contains_cell(layout.details, column, row) {
            return Some(HoveredPane::Details);
        }
        self.pane_beside_column(column)
    }

    fn pane_beside_column(&self, column: u16) -> Option<HoveredPane> {
        let layout = self.layout();
        if layout.changes.width == 0 {
            return None;
        }
        if column < layout.log_content_rect().x {
            return Some(HoveredPane::Changes);
        }
        if self.editor_tabs.file_diff().is_some() {
            return Some(HoveredPane::Diff);
        }
        if column > layout.commits.right() {
            Some(HoveredPane::Details)
        } else {
            Some(HoveredPane::Commits)
        }
    }

    fn apply_left_press(&mut self, mouse_input: MouseInput) {
        if self.apply_activity_rail_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_top_bar_help_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_log_tabs_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_log_rail_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_remote_tool_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_changes_toolbar_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_top_bar_divergence_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_head_branch_chip_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_branch_filter_chip_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_log_filter_bar_press(mouse_input.column, mouse_input.row) {
            return;
        }
        if self.apply_hunk_checkbox_press(mouse_input.column, mouse_input.row) {
            self.dragged_divider = None;
            return;
        }
        let opens_diff_on_double_click = mouse_input.click_count == DOUBLE_CLICK_COUNT;
        self.dragged_divider = self.hit_divider(mouse_input.column, mouse_input.row);
        self.dragged_divider_offset = self.dragged_divider_offset(mouse_input.column);
        if self.dragged_divider.is_none() {
            self.apply_commit_press(mouse_input.column, mouse_input.row, mouse_input.click_count);
            self.apply_details_press(
                mouse_input.column,
                mouse_input.row,
                opens_diff_on_double_click,
            );
            self.apply_changes_press(
                mouse_input.column,
                mouse_input.row,
                opens_diff_on_double_click,
            );
        }
    }

    fn apply_log_tabs_press(&mut self, column: u16, row: u16) -> bool {
        let titles = self.editor_tabs.titles();
        let Some(hit) = editor_tab_hit_at(
            self.layout().log_tabs,
            &titles,
            self.editor_tabs.active_index(),
            self.caps,
            column,
            row,
        ) else {
            return false;
        };
        match hit {
            EditorTabHit::Log => self.return_to_the_log_view(),
            EditorTabHit::Diff(diff_index) => {
                self.editor_tabs.activate_diff_at(diff_index);
                self.reset_diff_view_offsets();
                self.focus_active_diff_tab();
            }
            EditorTabHit::CloseDiff(diff_index) => {
                self.editor_tabs.close_diff_at(diff_index);
                self.reset_diff_view_offsets();
                self.restore_focus_after_diff_tab();
            }
        }
        true
    }

    fn return_to_the_log_view(&mut self) {
        if self.editor_tabs.file_diff().is_none() {
            return;
        }
        self.editor_tabs.activate_log();
        self.reset_diff_view_offsets();
        self.restore_focus_after_diff_tab();
    }

    fn apply_details_press(&mut self, column: u16, row: u16, open_diff_on_double_click: bool) {
        if !open_diff_on_double_click || self.editor_tabs.file_diff().is_some() {
            return;
        }
        let Some((commit_id, path)) = self.details_file_at(column, row) else {
            return;
        };
        let result = self.open_commit_diff(&commit_id, &path);
        self.record_panel_error(PanelErrorSource::Refresh, &result);
    }

    fn details_file_at(&self, column: u16, row: u16) -> Option<(String, String)> {
        if self.log_view != LogView::Log {
            return None;
        }
        let details = self.layout().rows_above_bottom_rule(self.layout().details);
        let changed_files = commit_changed_file_entries(self.log_state.files_at_selected_commit());
        let file_index = changed_file_index_at(
            details,
            &changed_files,
            self.log_state.selected_commit(),
            self.head_info.branch.as_deref(),
            self.selected_commit_branches(),
            column,
            row,
        )?;
        let commit_id = self.log_state.selected_commit()?.id.clone();
        let path = changed_files.get(file_index)?.path.clone();
        Some((commit_id, path))
    }

    fn apply_log_rail_press(&mut self, column: u16, row: u16) -> bool {
        if self.editor_tabs.file_diff().is_some() {
            return false;
        }
        let rail = self.layout().rows_above_bottom_rule(self.layout().log_rail);
        if !rect_contains_cell(rail, column, row) {
            return false;
        }
        self.toggle_log_view();
        true
    }

    fn apply_head_branch_chip_press(&mut self, column: u16, row: u16) -> bool {
        let top_bar = self.layout().top_bar;
        let head_branch = self.head_branch_label();
        let project_name = self.project_name();
        let opens_dropdown = [
            top_bar_branch_chip_rect(
                top_bar,
                project_name.as_deref(),
                head_branch,
                self.head_info.ahead,
                self.head_info.behind,
                self.caps,
            ),
            top_bar_branch_chevron_rect(
                top_bar,
                project_name.as_deref(),
                head_branch,
                self.head_info.ahead,
                self.head_info.behind,
                self.caps,
            ),
        ]
        .into_iter()
        .flatten()
        .any(|rect| rect_contains_cell(rect, column, row));
        if !opens_dropdown {
            return false;
        }
        self.open_branch_filter_dropdown();
        true
    }

    fn apply_log_filter_bar_press(&mut self, column: u16, row: u16) -> bool {
        if self.log_view != LogView::Log || self.editor_tabs.file_diff().is_some() {
            return false;
        }
        let filters = self.layout().log_filters;
        if !rect_contains_cell(filters, column, row) {
            return false;
        }
        if self.filter_bar_shows_blocks() {
            if case_sensitive_filter_block_rect(filters, self.log_filter_branch.as_deref())
                .is_some_and(|block| rect_contains_cell(block, column, row))
            {
                self.toggle_log_filter_case_sensitive();
                return true;
            }
            if !text_filter_block_rect(filters, self.log_filter_branch.as_deref())
                .is_some_and(|block| rect_contains_cell(block, column, row))
            {
                return false;
            }
        }
        self.focus = PaneId::Log;
        self.begin_log_filter_editing();
        true
    }

    fn apply_hunk_checkbox_press(&mut self, column: u16, row: u16) -> bool {
        if self.editor_tabs.file_diff().is_none() || !self.hunk_staging_available() {
            return false;
        }
        let area = self
            .layout()
            .rows_above_bottom_rule(self.layout().log_content_rect());
        if !rect_contains_cell(area, column, row) {
            return false;
        }
        if !rect_contains_cell(hunk_checkbox_rect(area, row), column, row) {
            return false;
        }
        let Some(hunk_index) =
            self.hunk_index_at_diff_row(self.diff_scroll.saturating_add(usize::from(row - area.y)))
        else {
            return false;
        };
        self.toggle_hunk_check(hunk_index);
        true
    }

    fn apply_commit_press(&mut self, column: u16, row: u16, click_count: u8) {
        let commits = self.layout().commits;
        if !rect_contains_cell(commits, column, row) {
            return;
        }
        if self.log_view == LogView::Branches {
            self.apply_branch_press(commits, column, row, click_count);
            return;
        }
        let Some(row_offset) = row.checked_sub(commits.y).map(usize::from) else {
            return;
        };
        let Some(commit_index) = self.log_state.commit_index_at_visible_row(row_offset) else {
            return;
        };
        let Some(selected_index) = self.log_state.selected_commit_index() else {
            return;
        };
        let delta = isize::try_from(commit_index).unwrap_or(0)
            - isize::try_from(selected_index).unwrap_or(0);
        self.select_commit(delta);
    }

    fn apply_branch_press(&mut self, list_area: Rect, column: u16, row: u16, click_count: u8) {
        let list = branch_list_rows_area(list_area);
        let Some(row_offset) = row.checked_sub(list.y) else {
            return;
        };
        if !rect_contains_cell(branch_row_rect(list, row_offset), column, row) {
            return;
        }
        let Some(branch_index) = self
            .branch_state
            .branch_index_at_visible_row(usize::from(row_offset))
        else {
            return;
        };
        self.branch_state.select_index(branch_index);
        if click_count == SINGLE_CLICK_COUNT {
            self.checkout_selected_branch();
        }
    }

    fn apply_changes_press(&mut self, column: u16, row: u16, open_diff_on_file_row: bool) {
        let layout = self.layout();
        let Some(panel) = changes_panel_layout(layout.changes, self.amend()) else {
            return;
        };
        if self.apply_sign_off_popover_press(panel, column, row) {
            return;
        }
        if self.apply_panel_error_press(panel, column, row) {
            return;
        }
        if self.apply_commit_button_press(panel, column, row) {
            return;
        }
        if rect_contains_cell(panel.amend_row, column, row) {
            self.toggle_amend();
            return;
        }
        if rect_contains_cell(panel.message_box, column, row) {
            self.begin_commit_message_editing();
            return;
        }
        let content = panel.list;
        if !rect_contains_cell(content, column, row) {
            return;
        }
        let Some(row_index) = row.checked_sub(content.y).map(usize::from) else {
            return;
        };
        let Some(changes_row) = self.changes_state.row_at(row_index) else {
            return;
        };
        self.changes_state.select_row(row_index);
        let checkbox_column = content.x + CHECKBOX_COLUMN;
        if (checkbox_column..checkbox_column + CHECKBOX_WIDTH).contains(&column) {
            self.changes_state.toggle_checked_at_row(row_index);
            return;
        }
        if let (true, ChangesRow::Group { group, .. }) = (column == content.x, changes_row) {
            self.changes_state.toggle_group_collapsed(group);
            return;
        }
        if open_diff_on_file_row {
            self.open_selected_diff();
        }
    }

    fn apply_panel_error_press(
        &mut self,
        panel: ChangesPanelLayout,
        column: u16,
        row: u16,
    ) -> bool {
        if self.panel_error.is_none() {
            return false;
        }
        let Some(panel_error_layout) = panel_error_layout(panel.list, self.screen_area()) else {
            return false;
        };
        if !rect_contains_cell(panel_error_layout.ok_button, column, row) {
            return false;
        }
        self.dismiss_panel_error();
        true
    }

    fn apply_sign_off_popover_press(
        &mut self,
        panel: ChangesPanelLayout,
        column: u16,
        row: u16,
    ) -> bool {
        if !self.commit_state.options_visible() {
            return false;
        }
        let Some((frame, sign_off_row)) = sign_off_popover(panel.button_row) else {
            return false;
        };
        if rect_contains_cell(sign_off_row, column, row) {
            self.commit_state.toggle_sign_off();
            self.commit_state.set_options_visible(false);
            return true;
        }
        if rect_contains_cell(frame, column, row) {
            return true;
        }
        self.commit_state.set_options_visible(false);
        false
    }

    fn apply_commit_button_press(
        &mut self,
        panel: ChangesPanelLayout,
        column: u16,
        row: u16,
    ) -> bool {
        if rect_contains_cell(panel.options_button, column, row) {
            let visible = self.commit_state.options_visible();
            self.commit_state.set_options_visible(!visible);
            return true;
        }
        if rect_contains_cell(panel.commit_button, column, row) {
            self.commit_from_ui();
            return true;
        }
        if rect_contains_cell(panel.push_button, column, row) {
            self.commit_and_push_from_ui();
            return true;
        }
        false
    }

    fn drag_divider_to(&mut self, column: u16) {
        let divider_column = i32::from(column) - i32::from(self.dragged_divider_offset);
        let divider_column =
            u16::try_from(divider_column.clamp(0, i32::from(u16::MAX))).unwrap_or(0);
        match self.dragged_divider {
            Some(DividerId::ChangesLog) => {
                self.changes_width = Some(changes_width_for_divider_column(divider_column));
            }
            Some(DividerId::CommitsDetails) => {
                self.details_width = Some(details_width_for_divider_column(
                    self.screen.width,
                    self.layout().log_area.width,
                    divider_column,
                ));
            }
            None => {}
        }
    }

    fn selected_visible_row(&self, visible_row_count: usize) -> Option<usize> {
        (visible_row_count > 0).then(|| self.changes_state.selected_row())
    }
}

enum HoveredPane {
    Changes,
    Commits,
    Diff,
    Details,
}

fn ahead_behind_hint(ahead: u32, behind: u32) -> String {
    format!("Ahead {ahead}, behind {behind} (pulls with p)")
}

fn key_binding_rows() -> Vec<(&'static str, &'static str)> {
    KEY_BINDINGS
        .iter()
        .map(|binding| (binding.keys, binding.action))
        .collect()
}

fn commit_changed_file_entries(files: &[ChangedFile]) -> Vec<ChangeEntry> {
    files
        .iter()
        .map(|file| ChangeEntry::unstaged(file.path.clone(), file.previous_path.clone(), file.kind))
        .collect()
}

fn divider_hit_area_contains(divider: Rect, column: u16, row: u16) -> bool {
    let in_row_band = divider.height > 0 && row >= divider.y && row < divider.bottom();
    let in_column_band = divider.width > 0
        && (column == divider.x
            || column == divider.x.saturating_sub(1)
            || column == divider.x.saturating_add(1));
    in_row_band && in_column_band
}

pub fn is_quit_key(key_input: &KeyInput) -> bool {
    matches!(key_input.code, KeyCode::Char('q') | KeyCode::Char('c')) && key_input.modifiers.ctrl
}

fn single_line_error_text(error: &GitError) -> String {
    collapse_whitespace(&error.to_string())
}

fn remote_failure_message(error: &GitError) -> String {
    match error {
        GitError::CommandFailed { message, .. } => {
            collapse_whitespace(&git_message_without_remote_header(message))
        }
        _ => single_line_error_text(error),
    }
}

fn git_message_without_remote_header(message: &str) -> String {
    if !GIT_REMOTE_HEADER_PREFIXES
        .iter()
        .any(|prefix| message.starts_with(prefix))
    {
        return message.to_string();
    }
    match message.split_once(GIT_REPORTED_LINE_SEPARATOR) {
        Some((_remote_header, remainder)) => remainder.to_string(),
        None => message.to_string(),
    }
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn first_line_error_text(error: &GitError) -> String {
    error
        .to_string()
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn pull_strategy(option: PullStrategyOption) -> PullStrategy {
    match option {
        PullStrategyOption::FastForwardOnly => PullStrategy::FastForwardOnly,
        PullStrategyOption::Merge => PullStrategy::Merge,
        PullStrategyOption::Rebase => PullStrategy::Rebase,
    }
}

fn remote_checkout_unsupported_text(branch_name: &str) -> String {
    format!("{branch_name}{REMOTE_CHECKOUT_UNSUPPORTED_SUFFIX}")
}

fn merge_message_prefill(repository_path: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(
        repository_path
            .join(GIT_DIRECTORY_NAME)
            .join(MERGE_MESSAGE_FILE_NAME),
    )
    .ok()?;
    let first_line = contents.lines().next().unwrap_or_default().trim();
    (!first_line.is_empty()).then(|| first_line.to_string())
}

fn amended_commit_receipt(committed_file_count: usize) -> String {
    if committed_file_count == 0 {
        return String::from(AMENDED_COMMIT_STATUS);
    }
    format!(
        "Amended commit with {}",
        file_count_label(committed_file_count)
    )
}

fn untracked_amended_receipt(untracked_file_count: usize) -> String {
    format!(
        "{} amended into the last commit",
        untracked_file_label(untracked_file_count)
    )
}

fn untracked_file_label(untracked_file_count: usize) -> String {
    if untracked_file_count == 1 {
        return String::from("1 untracked file");
    }
    format!("{untracked_file_count} untracked files")
}

fn conflict_resolution_receipt(path: &str, resolution: ConflictResolution) -> String {
    match resolution {
        ConflictResolution::Ours => format!("{ACCEPTED_OURS_STATUS_PREFIX}{path}"),
        ConflictResolution::Theirs => format!("{ACCEPTED_THEIRS_STATUS_PREFIX}{path}"),
        ConflictResolution::MarkResolved => {
            format!("{MARKED_RESOLVED_STATUS_PREFIX}{path}{MARKED_RESOLVED_STATUS_SUFFIX}")
        }
    }
}

fn merge_conflict_count_text(path_count: usize) -> String {
    if path_count == 1 {
        return String::from("1 file has conflicts");
    }
    format!("{path_count} files have conflicts")
}

fn hunk_staging_receipt(hunk_selection: &[(String, Vec<usize>)]) -> String {
    let hunk_count: usize = hunk_selection
        .iter()
        .map(|(_, hunk_indices)| hunk_indices.len())
        .sum();
    let file_count = hunk_selection.len();
    format!("Staged {hunk_count} hunks in {file_count} files")
}

impl App {
    fn render_panel_error_overlay(&self, area: Rect, buffer: &mut Buffer) {
        let Some(panel_error) = self.panel_error.as_ref() else {
            return;
        };
        let Some(panel) = changes_panel_layout(self.layout().changes, self.amend()) else {
            return;
        };
        render_panel_error(
            panel.list,
            area,
            PanelErrorView {
                source: panel_error.source,
                message: panel_error.message.as_str(),
                scroll: panel_error.scroll,
            },
            self.caps,
            &theme_palette(self.theme),
            buffer,
        );
    }
}

impl ScreenDriver for App {
    fn handle_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key(key_input) => self.apply_key(key_input),
            InputEvent::Mouse(mouse_input) => self.apply_mouse(mouse_input),
            InputEvent::Resize(size) => {
                self.screen = size;
                self.keep_selected_commit_visible();
                self.keep_selected_branch_visible();
            }
            InputEvent::Tick => {
                self.age_status_message();
                self.run_pending_details_query_once_the_selection_settles();
            }
            InputEvent::Paste(_) => {}
        }
    }

    fn render_frame(&self, area: Rect, buffer: &mut Buffer) {
        let layout = self.layout();
        let tab_titles = self.editor_tabs.titles();
        render_shell(
            area,
            &layout,
            ShellView {
                caps: self.caps,
                theme: self.theme,
                status_left_label: &self.status_left_label(),
                operation: self.head_info.operation,
                status_message: self
                    .active_status_message()
                    .map(|(text, tone)| StatusMessageView { text, tone }),
                status_hint: self.status_hint(),
                project_name: self.project_name().as_deref(),
                head_branch: self.head_branch_label(),
                focused_pane: self.focus,
                editor_tabs: EditorTabStrip {
                    titles: &tab_titles,
                    active_index: self.editor_tabs.active_index(),
                },
                log_filter: LogFilterBarView {
                    text: self.filter_bar_text(),
                    editing: self.log_filter_draft.is_some(),
                    branch_filter: self.log_filter_branch.as_deref(),
                    case_sensitive: self.log_filter_case_sensitive,
                },
                hovered_changes_toolbar: self.hovered_changes_toolbar,
                ahead: self.head_info.ahead,
                behind: self.head_info.behind,
                divergence_hovered: self.hovered_top_bar_divergence,
            },
            buffer,
        );
        let rows = self.changes_state.rows();
        render_changes_panel(
            layout.changes,
            ChangesPanelView {
                entries: self.changes_state.entries(),
                rows: &rows,
                selected_row: self.selected_visible_row(rows.len()),
                focused: self.focus == PaneId::Changes && !self.commit_state.is_editing(),
                caps: self.caps,
                theme: self.theme,
                open_error: self.open_error.as_deref(),
                panel_error: self.panel_error.as_ref().map(|panel_error| PanelErrorView {
                    source: panel_error.source,
                    message: panel_error.message.as_str(),
                    scroll: panel_error.scroll,
                }),
                commit: CommitPanelView {
                    message: self.commit_state.message(),
                    editing: self.commit_state.is_editing(),
                    amend: self.commit_state.amend(),
                    sign_off: self.commit_state.sign_off(),
                    commit_enabled: self.commit_enabled(),
                    options_visible: self.commit_state.options_visible(),
                    checked_file_count: self.committed_paths().len(),
                    checked_untracked_file_count: self.checked_untracked_file_count(),
                },
            },
            buffer,
        );
        if let Some(diff) = self.editor_tabs.file_diff() {
            let mut diff_rows = diff_rows(diff);
            if self.word_highlights {
                assign_word_highlights(&mut diff_rows);
            }
            render_diff_pane(
                layout.rows_above_bottom_rule(layout.log_content_rect()),
                DiffPaneView {
                    rows: &diff_rows,
                    scroll: self.diff_scroll,
                    horizontal_offset: self.diff_horizontal_offset,
                    wrap_width: self.diff_wrap_width(),
                    checked_hunks: self.active_diff_checked_hunks(),
                    caps: self.caps,
                },
                &theme_palette(self.theme),
                buffer,
            );
            self.render_panel_error_overlay(area, buffer);
            self.render_dialog_overlays(area, buffer);
            self.render_branch_filter_dropdown_overlay(area, buffer);
            self.render_help_overlay(area, buffer);
            return;
        }
        let head_branch = self.head_info.branch.as_deref();
        let list_area = layout.rows_above_bottom_rule(layout.commits);
        render_branches_rail(
            layout.rows_above_bottom_rule(layout.log_rail),
            self.log_view == LogView::Branches,
            self.theme,
            buffer,
        );
        if self.log_view == LogView::Branches {
            render_branch_list(
                list_area,
                BranchListView {
                    branches: self.branch_state.branches(),
                    selected_index: self.branch_state.selected_index(),
                    scroll: self.branch_state.scroll(),
                    focused: self.focus == PaneId::Log,
                    caps: self.caps,
                    theme: self.theme,
                },
                buffer,
            );
        } else {
            render_commit_list(
                list_area,
                CommitListView {
                    commits: self.log_state.commits(),
                    lanes: self.log_state.lanes(),
                    selected_index: self.log_state.selected_index(),
                    scroll: self.log_state.scroll(),
                    focused: self.focus == PaneId::Log,
                    head_branch,
                    utc_offset_minutes: self.utc_offset_minutes,
                    screen_width: self.screen.width,
                    caps: self.caps,
                    theme: self.theme,
                },
                buffer,
            );
        }
        let changed_file_entries =
            commit_changed_file_entries(self.log_state.files_at_selected_commit());
        let containing_branches = self.selected_commit_branches();
        render_commit_details(
            layout.rows_above_bottom_rule(layout.details),
            CommitDetailsView {
                commit: self.log_state.selected_commit(),
                changed_files: &changed_file_entries,
                head_branch,
                containing_branches,
                utc_offset_minutes: self.utc_offset_minutes,
                caps: self.caps,
                theme: self.theme,
            },
            buffer,
        );
        self.render_panel_error_overlay(area, buffer);
        self.render_dialog_overlays(area, buffer);
        self.render_branch_filter_dropdown_overlay(area, buffer);
        self.render_help_overlay(area, buffer);
    }

    fn focused_pane(&self) -> PaneId {
        self.focus
    }

    fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
        let rect = self.layout().pane_rect(pane);
        (rect.width > 0 && rect.height > 0).then_some(rect)
    }
}

#[cfg(test)]
mod app_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use gitdeck_core::{
        InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInput, MouseInputKind,
        PaneId, Size,
    };
    use gitdeck_tui::{
        PanelErrorSource, ScreenDriver, StatusTone, CHECKOUT_FAILED_LABEL, COMMIT_FAILED_LABEL,
        REFRESH_FAILED_LABEL,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::{amended_commit_receipt, App, ChangesFocus, DividerId, GitError};

    fn app_with_screen(width: u16, height: u16) -> App {
        let mut app = App::new(CapsProfile::Rich, ThemeId::JbDark);
        app.handle_event(InputEvent::Resize(Size { width, height }));
        app
    }

    fn key(code: KeyCode, modifiers: KeyModifiers) -> InputEvent {
        InputEvent::Key(KeyInput { code, modifiers })
    }

    fn mouse(kind: MouseInputKind, column: u16, row: u16) -> InputEvent {
        InputEvent::Mouse(MouseInput {
            kind,
            button: MouseButton::Left,
            column,
            row,
            modifiers: KeyModifiers::default(),
            click_count: 1,
        })
    }

    const CTRL: KeyModifiers = KeyModifiers {
        ctrl: true,
        alt: false,
        shift: false,
    };
    const ALT: KeyModifiers = KeyModifiers {
        ctrl: false,
        alt: true,
        shift: false,
    };
    const SHIFT: KeyModifiers = KeyModifiers {
        ctrl: false,
        alt: false,
        shift: true,
    };

    #[test]
    fn shifted_characters_are_inserted_while_alt_stays_a_shortcut() {
        let mut app = app_with_screen(160, 48);
        app.handle_event(key(KeyCode::Char('i'), KeyModifiers::default()));
        for character in ['H', 'e', 'l', 'l', 'o'] {
            app.handle_event(key(KeyCode::Char(character), SHIFT));
        }
        app.handle_event(key(KeyCode::Space, SHIFT));
        for character in ['A', 'B', 'C'] {
            app.handle_event(key(KeyCode::Char(character), SHIFT));
        }
        assert_eq!(app.commit_message(), "Hello ABC");
        app.handle_event(key(KeyCode::Char('a'), ALT));
        assert_eq!(
            app.commit_message(),
            "Hello ABC",
            "alt keeps its shortcut meaning"
        );
        assert!(app.amend(), "alt+a toggled amend instead of typing");
    }

    #[test]
    fn the_push_failure_message_drops_the_remote_header_segment() {
        let error = GitError::CommandFailed {
            path: PathBuf::from("/repo"),
            arguments: String::from("push origin main"),
            message: String::from(
                "To /remote.git; ! [rejected] main -> main (non-fast-forward); error: failed to push some refs",
            ),
        };
        let message = super::remote_failure_message(&error);
        assert!(
            message.starts_with("! [rejected]"),
            "the rejection leads the message, message {message:?}"
        );
        assert!(
            !message.contains("To /remote.git"),
            "the remote url line stays out of the block, message {message:?}"
        );
        let missing_remote = super::remote_failure_message(&GitError::NoRemote);
        assert_eq!(missing_remote, "the repository has no remote to push to");
    }

    #[test]
    fn the_status_message_lifetime_is_three_seconds() {
        assert_eq!(
            super::TICK_INTERVAL * u32::from(super::STATUS_MESSAGE_LIFETIME_TICKS),
            std::time::Duration::from_secs(3)
        );
    }

    #[test]
    fn commit_message_api_tracks_the_editor_amend_and_sign_off() {
        let mut app = app_with_screen(160, 48);
        assert_eq!(app.commit_message(), "");
        assert_eq!(app.changes_focus(), ChangesFocus::List);
        assert!(!app.amend());
        assert!(!app.sign_off());
        assert!(!app.commit_options_visible());
        app.set_commit_message("typed");
        assert_eq!(app.commit_message(), "typed");
        assert!(
            !app.commit_enabled(),
            "the commit stays disabled while no file is checked"
        );
        app.handle_event(key(KeyCode::Char('i'), KeyModifiers::default()));
        assert_eq!(app.changes_focus(), ChangesFocus::Message);
        app.handle_event(key(KeyCode::Esc, KeyModifiers::default()));
        assert_eq!(app.changes_focus(), ChangesFocus::List);
        assert_eq!(
            app.commit_message(),
            "typed",
            "leaving the editor keeps the message"
        );
    }

    #[test]
    fn tab_and_back_tab_cycle_through_three_panes() {
        let mut app = app_with_screen(160, 48);
        assert_eq!(app.focused_pane(), PaneId::Changes);
        for expected_pane in [PaneId::Log, PaneId::Details, PaneId::Changes] {
            app.handle_event(key(KeyCode::Tab, KeyModifiers::default()));
            assert_eq!(app.focused_pane(), expected_pane);
        }
        for expected_pane in [PaneId::Details, PaneId::Log, PaneId::Changes] {
            app.handle_event(key(KeyCode::BackTab, KeyModifiers::default()));
            assert_eq!(app.focused_pane(), expected_pane);
        }
    }

    #[test]
    fn shortcuts_focus_changes_and_log() {
        let mut app = app_with_screen(160, 48);
        app.handle_event(key(KeyCode::Char('k'), CTRL));
        assert_eq!(app.focused_pane(), PaneId::Changes);
        app.handle_event(key(KeyCode::Char('9'), ALT));
        assert_eq!(app.focused_pane(), PaneId::Log);
    }

    #[test]
    fn log_pane_rect_is_the_commits_column() {
        let app = app_with_screen(160, 48);
        let layout = app.layout();
        assert_eq!(app.pane_rect(PaneId::Log), Some(layout.commits));
        assert_eq!(app.pane_rect(PaneId::Changes), Some(layout.changes));
        assert_eq!(app.pane_rect(PaneId::Details), Some(layout.details));
    }

    #[test]
    fn zero_sized_screen_reports_no_pane_rects() {
        let app = App::new(CapsProfile::Rich, ThemeId::JbDark);
        assert_eq!(app.pane_rect(PaneId::Changes), None);
        assert_eq!(app.pane_rect(PaneId::Log), None);
        assert_eq!(app.pane_rect(PaneId::Details), None);
        assert_eq!(app.hit_divider(0, 0), None);
    }

    #[test]
    fn divider_hits_report_the_matching_divider() {
        let app = app_with_screen(160, 48);
        let layout = app.layout();
        assert_eq!(
            app.hit_divider(layout.changes_log_divider.x, 5),
            Some(DividerId::ChangesLog)
        );
        assert_eq!(
            app.hit_divider(layout.commits_details_divider.x, 5),
            Some(DividerId::CommitsDetails)
        );
        assert_eq!(app.hit_divider(layout.changes.x, 5), None);
        assert_eq!(app.hit_divider(layout.changes_log_divider.x, 0), None);
    }

    #[test]
    fn dragging_the_changes_divider_moves_the_column_boundary() {
        let mut app = app_with_screen(160, 48);
        let divider_column = app.layout().changes_log_divider.x;
        app.handle_event(mouse(MouseInputKind::Down, divider_column, 5));
        app.handle_event(mouse(MouseInputKind::Drag, divider_column + 8, 5));
        app.handle_event(mouse(MouseInputKind::Up, divider_column + 8, 5));
        assert_eq!(app.layout().changes.width, 43);
        assert_eq!(app.layout().commits.width, 67 - 4);
        app.handle_event(mouse(MouseInputKind::Drag, divider_column + 40, 5));
        assert_eq!(
            app.layout().changes.width,
            43,
            "a drag after the mouse button was released is ignored"
        );
    }

    #[test]
    fn divider_hits_accept_the_adjacent_columns() {
        let app = app_with_screen(160, 48);
        let layout = app.layout();
        let divider_column = layout.changes_log_divider.x;
        for expected_column in [
            divider_column.saturating_sub(1),
            divider_column,
            divider_column + 1,
        ] {
            assert_eq!(
                app.hit_divider(expected_column, 5),
                Some(DividerId::ChangesLog),
                "column {expected_column} belongs to the divider hit area"
            );
        }
        assert_eq!(app.hit_divider(divider_column - 2, 5), None);
        assert_eq!(app.hit_divider(divider_column + 2, 5), None);
        assert_eq!(app.hit_divider(divider_column + 1, 0), None);
    }

    #[test]
    fn dragging_from_an_adjacent_divider_column_keeps_the_grab_offset() {
        let mut app = app_with_screen(160, 48);
        let divider_column = app.layout().changes_log_divider.x;
        let default_width = app.layout().changes.width;
        app.handle_event(mouse(MouseInputKind::Down, divider_column - 1, 5));
        app.handle_event(mouse(MouseInputKind::Drag, divider_column - 1, 5));
        assert_eq!(app.layout().changes.width, default_width);
        app.handle_event(mouse(MouseInputKind::Drag, divider_column + 7, 5));
        assert_eq!(app.layout().changes.width, default_width + 8);
    }

    #[test]
    fn opening_a_missing_repository_records_a_single_line_error() {
        let mut app = App::open_repository(
            std::path::Path::new("/gitdeck-no-such-repository"),
            CapsProfile::Rich,
            ThemeId::JbDark,
        );
        app.handle_event(InputEvent::Resize(Size {
            width: 80,
            height: 24,
        }));
        let open_error = app
            .open_error()
            .expect("a missing repository is reported")
            .to_string();
        assert!(!open_error.contains('\n'), "error text stays on one line");
        assert_eq!(app.repository_path(), None);
        assert_eq!(app.status_left_label(), "no repository");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
        app.render_frame(Rect::new(0, 0, 80, 24), &mut buffer);
        assert_eq!(pane_row_text(&buffer, 3), "Cannot open repository");
        assert!(pane_row_text(&buffer, 4).contains("git rev-parse"));
    }

    fn pane_row_text(buffer: &Buffer, row: u16) -> String {
        let changes = app_with_screen(80, 24).layout().changes;
        let mut text = String::new();
        for column in changes.x..changes.right() {
            if let Some(cell) = buffer.cell((column, row)) {
                text.push_str(cell.symbol());
            }
        }
        text.trim_end().to_string()
    }

    #[test]
    fn dragging_the_details_divider_respects_the_commits_floor() {
        let mut app = app_with_screen(160, 48);
        let divider_column = app.layout().commits_details_divider.x;
        app.handle_event(mouse(MouseInputKind::Down, divider_column, 5));
        app.handle_event(mouse(MouseInputKind::Drag, 0, 5));
        assert_eq!(
            app.layout().details.width,
            96,
            "the widened details column stops at sixty percent of the screen"
        );
        assert_eq!(app.layout().commits.width, 19);
    }

    #[test]
    fn rendering_a_zero_sized_area_leaves_the_buffer_untouched() {
        let app = App::new(CapsProfile::Rich, ThemeId::JbDark);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 2));
        app.render_frame(Rect::new(0, 0, 4, 2), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("cell").symbol(), " ");
    }

    #[test]
    fn app_without_a_repository_reports_the_placeholder_status() {
        let mut app = App::new(CapsProfile::Rich, ThemeId::JbDark);
        assert_eq!(app.repository_path(), None);
        assert_eq!(app.status_left_label(), "no repository");
        assert!(app.checked_file_paths().is_empty());
        assert_eq!(app.selected_file_path(), None);
        assert!(app.refresh().is_ok());
        assert_eq!(app.panel_error(), None);
    }

    #[test]
    fn opening_a_directory_outside_a_repository_returns_an_error() {
        let error = App::with_repository("/", CapsProfile::Rich, ThemeId::JbDark)
            .err()
            .expect("the filesystem root is not a git work tree");
        assert!(!error.to_string().is_empty());
    }

    fn pane_text(buffer: &Buffer, pane: Rect) -> String {
        let mut text = String::new();
        for row in pane.y..pane.bottom() {
            for column in pane.x..pane.right() {
                if let Some(cell) = buffer.cell((column, row)) {
                    text.push_str(cell.symbol());
                }
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn commit_failure_uses_the_commit_source() {
        let mut app = app_with_screen(160, 48);
        app.set_commit_message("doomed commit");
        let error = app
            .commit()
            .expect_err("a commit without a repository fails");
        assert!(!error.to_string().is_empty());
        let (source, message) = app.panel_error().expect("the failure is recorded");
        assert_eq!(source, PanelErrorSource::Commit);
        assert_eq!(source.title(), COMMIT_FAILED_LABEL);
        assert!(
            !message.is_empty(),
            "the panel error carries the git message"
        );
        let mut buffer = Buffer::empty(Rect::new(0, 0, 160, 48));
        app.render_frame(Rect::new(0, 0, 160, 48), &mut buffer);
        let screen_text = pane_text(&buffer, app.layout().changes);
        assert!(
            screen_text.contains(COMMIT_FAILED_LABEL),
            "the failed commit is titled Commit failed, screen text was:\n{screen_text}"
        );
        assert!(
            !screen_text.contains(REFRESH_FAILED_LABEL),
            "a failed commit is not titled Refresh failed, screen text was:\n{screen_text}"
        );
    }

    #[test]
    fn an_amended_receipt_without_files_names_no_branch() {
        assert_eq!(amended_commit_receipt(0), "Amended commit");
        assert_eq!(amended_commit_receipt(3), "Amended commit with 3 files");
    }

    #[test]
    fn selection_keys_are_ignored_while_the_log_pane_has_focus() {
        let mut app = app_with_screen(160, 48);
        app.handle_event(key(KeyCode::Char('9'), ALT));
        app.handle_event(key(KeyCode::Char('j'), KeyModifiers::default()));
        assert_eq!(app.focused_pane(), PaneId::Log);
        assert_eq!(app.changes().len(), 0);
    }

    const SCRATCH_REPOSITORY_DIRECTORY: &str = "artifacts/tmp/app-checkout";
    const CURRENT_BRANCH: &str = "main";
    const OTHER_BRANCH: &str = "feature/other";
    const UNKNOWN_BRANCH: &str = "no-such-branch";
    const OTHER_BRANCH_SUBJECT: &str = "call run on the other branch";
    const UNKNOWN_BRANCH_STDERR_KEY: &str = "did not match any file(s) known to git";

    fn scratch_repository(name: &str) -> PathBuf {
        let repository_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(SCRATCH_REPOSITORY_DIRECTORY)
            .join(name);
        if repository_path.exists() {
            fs::remove_dir_all(&repository_path).expect("the stale scratch repository is removed");
        }
        fs::create_dir_all(&repository_path).expect("the scratch repository directory is created");
        run_git(&repository_path, &["init", "-q", "-b", CURRENT_BRANCH]);
        run_git(
            &repository_path,
            &["config", "user.name", "gitdeck-fixture"],
        );
        run_git(
            &repository_path,
            &["config", "user.email", "fixture@gitdeck.invalid"],
        );
        run_git(&repository_path, &["config", "commit.gpgsign", "false"]);
        run_git(&repository_path, &["config", "core.hooksPath", "/dev/null"]);
        write_repository_file(
            &repository_path,
            "src/app.rs",
            "fn main() {\n    run();\n}\n",
        );
        run_git(&repository_path, &["add", "--all"]);
        run_git(&repository_path, &["commit", "-q", "-m", "initial import"]);
        run_git(&repository_path, &["checkout", "-q", "-b", OTHER_BRANCH]);
        write_repository_file(
            &repository_path,
            "src/app.rs",
            "fn main() {\n    run_on_the_other_branch();\n}\n",
        );
        run_git(
            &repository_path,
            &["commit", "-q", "-a", "-m", OTHER_BRANCH_SUBJECT],
        );
        run_git(&repository_path, &["checkout", "-q", CURRENT_BRANCH]);
        repository_path
    }

    fn write_repository_file(repository_path: &Path, relative_path: &str, contents: &str) {
        let path = repository_path.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("the file directory is created");
        }
        fs::write(&path, contents)
            .unwrap_or_else(|error| panic!("cannot write {}: {error}", path.display()));
    }

    fn run_git(repository_path: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository_path)
            .args(arguments)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LC_ALL", "C")
            .output()
            .unwrap_or_else(|error| panic!("cannot run git {arguments:?}: {error}"));
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn app_on_scratch_repository(name: &str) -> App {
        let repository_path = scratch_repository(name);
        let mut app = App::with_repository(&repository_path, CapsProfile::Rich, ThemeId::JbDark)
            .expect("repository opens");
        app.handle_event(InputEvent::Resize(Size {
            width: 160,
            height: 48,
        }));
        app
    }

    #[test]
    fn app_checkout_failure_sets_a_panel_error() {
        let mut app = app_on_scratch_repository("failure");
        app.checkout_branch(UNKNOWN_BRANCH);
        let (source, message) = app.panel_error().expect("the failure is recorded");
        assert_eq!(source, PanelErrorSource::Checkout);
        assert_eq!(source.title(), CHECKOUT_FAILED_LABEL);
        assert!(
            message.contains(UNKNOWN_BRANCH_STDERR_KEY),
            "the panel error keeps the git text, got {message:?}"
        );
        assert_eq!(app.head_info().branch.as_deref(), Some(CURRENT_BRANCH));
        assert_eq!(app.status_message(), None);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 160, 48));
        app.render_frame(Rect::new(0, 0, 160, 48), &mut buffer);
        let screen_text = pane_text(&buffer, app.layout().changes);
        assert!(
            screen_text.contains(CHECKOUT_FAILED_LABEL),
            "the failed checkout is titled Checkout failed, screen text was:\n{screen_text}"
        );
        assert!(
            !screen_text.contains(REFRESH_FAILED_LABEL),
            "a failed checkout is not titled Refresh failed, screen text was:\n{screen_text}"
        );
    }

    #[test]
    fn app_checkout_switches_the_head_and_reports_the_branch() {
        let mut app = app_on_scratch_repository("success");
        app.checkout_branch(OTHER_BRANCH);
        assert_eq!(app.panel_error(), None);
        assert_eq!(app.head_info().branch.as_deref(), Some(OTHER_BRANCH));
        assert_eq!(app.status_left_label(), OTHER_BRANCH);
        assert_eq!(
            app.status_message(),
            Some(("feature/other checked out", StatusTone::Info))
        );
        assert_eq!(
            app.commits().first().map(|commit| commit.subject.as_str()),
            Some(OTHER_BRANCH_SUBJECT)
        );
    }

    #[test]
    fn app_checkout_without_a_repository_reports_no_repository() {
        let mut app = app_with_screen(160, 48);
        app.checkout_branch(OTHER_BRANCH);
        let (source, message) = app.panel_error().expect("the failure is recorded");
        assert_eq!(source, PanelErrorSource::Checkout);
        assert_eq!(message, GitError::NoRepository.to_string());
    }
}
