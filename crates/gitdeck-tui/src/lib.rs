mod activity_rail;
mod branch_filter_dropdown;
mod branches;
mod changes;
mod confirm_dialog;
mod delete_branch_dialog;
mod dialog_frame;
mod diff;
mod discard_dialog;
mod display_text;
mod fetch_dialog;
mod glyphs;
mod help_overlay;
mod layout;
mod log;
mod new_branch_dialog;
mod next_step_hint;
mod pull_dialog;
mod push_dialog;
mod shell;
mod theme;
mod word_highlight;

pub use activity_rail::{
    activity_rail_entry_at, activity_rail_entry_index_of, activity_rail_entry_rects,
    activity_rail_footer_label, activity_rail_icon_rect, activity_rail_is_visible,
    render_activity_rail, ActivityRailEntry, ACTIVITY_RAIL_ENTRIES, ACTIVITY_RAIL_ENTRY_HEIGHT,
    ACTIVITY_RAIL_FOOTER_LABELS, ACTIVITY_RAIL_ICON_COLUMN, ACTIVITY_RAIL_ICON_ROW_OFFSET,
    ACTIVITY_RAIL_MIN_HEIGHT,
};
pub use branch_filter_dropdown::{
    branch_filter_dropdown_layout, render_branch_filter_dropdown, BranchFilterDropdownLayout,
    BranchFilterDropdownView, BRANCH_FILTER_CURRENT_BRANCH_PREFIX,
    BRANCH_FILTER_DROPDOWN_MAX_VISIBLE_ROWS, BRANCH_FILTER_HEAD_LABEL,
};
pub use branches::{
    branch_list_rows_area, branch_row_rect, render_branch_list, render_branches_rail,
    BranchListView, BRANCHES_RAIL_LABEL, BRANCHES_TITLE, BRANCH_NAME_COLUMN,
    BRANCH_TITLE_ROW_HEIGHT, NO_BRANCHES_LABEL,
};
pub use changes::{
    amend_file_notice, amend_summary_notice, amend_untracked_notice, box_glyphs, change_kind_color,
    change_kind_letter, changes_content_area, changes_group_of, changes_panel_layout, changes_rows,
    commit_and_push_compact_label, commit_and_push_label, commit_button_compact_label,
    commit_button_label, commit_label_color, commit_message_box_height, compact_amend_file_notice,
    compact_amend_summary_notice, conflict_status_letters, default_collapsed_groups,
    error_body_line_count, file_count_label, group_member_indices, message_character_count_label,
    panel_error_block_height, panel_error_layout, render_changes_panel, render_panel_error,
    sign_off_popover, visible_error_body_lines, wrap_error_body_lines, BoxGlyphs, ChangesGroup,
    ChangesPanelLayout, ChangesPanelView, ChangesRow, CheckState, CommitButtonRow, CommitPanelView,
    PanelErrorLayout, PanelErrorSource, PanelErrorView, AMEND_COMMIT_BUTTON_COMPACT_LABEL,
    AMEND_COMMIT_BUTTON_LABEL, AMEND_FILE_NOTICE_SUFFIX, AMEND_LABEL,
    AMEND_UNTRACKED_NOTICE_SUFFIX, BRANCH_FAILED_LABEL, CANNOT_OPEN_REPOSITORY_LABEL,
    CHANGES_GROUPS, CHANGES_PANEL_CONTENT_TOP, CHECKBOX_COLUMN, CHECKBOX_WIDTH,
    CHECKOUT_FAILED_LABEL, COMMIT_AND_PUSH_ASCII_LABEL, COMMIT_AND_PUSH_COMPACT_ASCII_LABEL,
    COMMIT_AND_PUSH_LABEL, COMMIT_BUTTON_LABEL, COMMIT_BUTTON_ROW_HEIGHT, COMMIT_FAILED_LABEL,
    COMMIT_MESSAGE_BOX_MIN_HEIGHT, COMMIT_MESSAGE_COUNT_SUFFIX, COMMIT_MESSAGE_HEIGHT_DIVISOR,
    COMMIT_MESSAGE_PLACEHOLDER, COMMIT_MESSAGE_TITLE, CONFLICT_FAILED_LABEL,
    CONFLICT_RESOLUTION_HINT, DISCARD_FAILED_LABEL, FETCH_FAILED_LABEL, FILE_DIRECTORY_GAP,
    FILE_NAME_COLUMN, FILE_ROW_INDENT, FILE_STATUS_COLUMN, FILE_WORKTREE_STATUS_COLUMN,
    GROUP_ARROW_COLUMN, GROUP_TITLE_COLUMN, IGNORE_FAILED_LABEL, MERGE_FAILED_LABEL,
    NO_CHANGES_LABEL, OPEN_ERROR_REASON_ROW_COUNT, OPERATION_FAILED_LABEL,
    PANEL_ERROR_BODY_ROW_COUNT, PANEL_ERROR_OK_LABEL, PANEL_ERROR_ROW_COUNT, PULL_FAILED_LABEL,
    PUSH_FAILED_LABEL, REFRESH_FAILED_LABEL, SIGN_OFF_LABEL,
};

pub use help_overlay::{
    help_overlay_layout, help_overlay_visible_row_count, render_help_overlay, wrap_help_rows,
    HelpOverlayLayout, HelpOverlayView, HELP_OVERLAY_KEYS_GAP, HELP_OVERLAY_MAX_HEIGHT,
    HELP_OVERLAY_MAX_WIDTH, HELP_OVERLAY_MORE_ASCII_LABEL, HELP_OVERLAY_MORE_LABEL,
    HELP_OVERLAY_PAGE_ROWS, HELP_OVERLAY_TITLE,
};
pub use next_step_hint::{
    next_step_hint, next_step_hint_prefix, next_step_hints, DIRTY_CHECKOUT_HINT, LOCKED_INDEX_HINT,
    MERGE_CONFLICT_HINT, MISSING_REMOTE_HINT, NEXT_STEP_HINT_ASCII_PREFIX, NEXT_STEP_HINT_PREFIX,
    NEXT_STEP_HINT_TEXTS, NOT_FAST_FORWARD_PULL_HINT, REJECTED_PUSH_HINT, UNKNOWN_BRANCH_HINT,
};

pub use diff::{
    diff_display_rows, hunk_checkbox_rect, render_diff_pane, DiffDisplayRow, DiffPaneView, DiffRow,
    DiffRowKind, BINARY_FILE_LABEL, CONTINUATION_MARKER_ASCII, CONTINUATION_MARKER_UNICODE,
    DIFF_HUNK_CHECKBOX_COLUMN, DIFF_HUNK_CHECKBOX_WIDTH, DIFF_HUNK_TEXT_COLUMN,
    DIFF_LINE_NUMBER_WIDTH, DIFF_MARKER_COLUMN, DIFF_NEW_NUMBER_COLUMN, DIFF_OLD_NUMBER_COLUMN,
    DIFF_TEXT_COLUMN, IDENTICAL_CONTENTS_LABEL,
};

pub use confirm_dialog::{
    confirm_dialog_layout, render_confirm_dialog, ConfirmDialogButton, ConfirmDialogLayout,
    ConfirmDialogView, ABORT_CONFIRM_BUTTON_LABEL, CONFIRM_CANCEL_BUTTON_LABEL,
    CONFIRM_DIALOG_BUTTON_ORDER, CONFIRM_DIALOG_HEIGHT, CONFIRM_DIALOG_MAX_WIDTH,
    QUIT_CONFIRM_BUTTON_LABEL,
};
pub use delete_branch_dialog::{
    delete_branch_dialog_layout, delete_branch_message_text, render_delete_branch_dialog,
    DeleteBranchDialogButton, DeleteBranchDialogLayout, DeleteBranchDialogView,
    DELETE_BRANCH_CANCEL_BUTTON_LABEL, DELETE_BRANCH_CONFIRM_BUTTON_LABEL,
    DELETE_BRANCH_DIALOG_BUTTON_ORDER, DELETE_BRANCH_DIALOG_TITLE,
};
pub use dialog_frame::ORIGIN_REMOTE_NAME;
pub use discard_dialog::{
    discard_dialog_layout, discard_message_text, render_discard_dialog, DiscardDialogButton,
    DiscardDialogLayout, DiscardDialogView, DISCARD_CANCEL_BUTTON_LABEL,
    DISCARD_CONFIRM_BUTTON_LABEL, DISCARD_DIALOG_BUTTON_ORDER, DISCARD_DIALOG_HEIGHT,
    DISCARD_DIALOG_MAX_WIDTH, DISCARD_DIALOG_TITLE, DISCARD_IRREVERSIBLE_LABEL,
};
pub use display_text::{
    control_placeholder, glyph_display_width, sanitize_display_string, sanitize_display_text,
    DisplayGlyph, SanitizedText, TAB_STOP_WIDTH,
};
pub use fetch_dialog::{
    fetch_dialog_layout, fetch_message_text, render_fetch_dialog, FetchDialogButton,
    FetchDialogLayout, FetchDialogView, FETCH_CANCEL_BUTTON_LABEL, FETCH_CONFIRM_BUTTON_LABEL,
    FETCH_DIALOG_BUTTON_ORDER, FETCH_DIALOG_HEIGHT, FETCH_DIALOG_MAX_WIDTH, FETCH_DIALOG_TITLE,
    FETCH_NO_REMOTE_LABEL,
};
pub use glyphs::{ellipsis_glyph, line_glyphs, IconGlyph, LineGlyphs, EVERY_ICON_GLYPH};
pub use layout::{
    changes_width_for_divider_column, compute_pane_layout, default_changes_width,
    default_details_width, details_max_width, details_width_for_divider_column, rect_contains_cell,
    resolve_changes_width, resolve_details_and_commits_widths, DividerId, PaneLayout,
    ACTIVITY_RAIL_WIDTH, BOTTOM_RULE_HEIGHT, CHANGES_DEFAULT_WIDTH, CHANGES_MAX_WIDTH,
    CHANGES_MIN_WIDTH, CHANGES_WIDTH_PERCENT, COMMITS_FLOOR_WIDTH, DETAILS_DEFAULT_MAX_WIDTH,
    DETAILS_DEFAULT_WIDTH, DETAILS_MAX_WIDTH_PERCENT, DETAILS_MIN_WIDTH, DETAILS_WIDTH_HIDDEN,
    DETAILS_WIDTH_PERCENT, DIVIDER_WIDTH, LOG_RAIL_GAP, LOG_RAIL_WIDTH, STATUS_BAR_HEIGHT,
    TOP_BAR_HEIGHT, WIDE_LAYOUT_MIN_SCREEN_WIDTH,
};
pub use log::{
    changed_file_index_at, changed_file_rows, changed_files_area, commit_detail_short_id,
    commit_details_area, commit_details_row_count, commit_ref_labels, containing_branch_label,
    format_commit_date, ref_label, ref_label_color, render_commit_details, render_commit_list,
    ChangedFileRow, CommitDetailsView, CommitListView, ContainingBranches, RefLabel,
    CHANGED_FILE_COUNT_GAP, CHANGED_FILE_DIRECTORY_GAP, CHANGED_FILE_INDENT,
    CHANGED_FILE_STATUS_GAP, COMMITTED_BY_LABEL_PREFIX, COMMIT_DETAIL_ID_GAP,
    COMMIT_DETAIL_META_HEIGHT, COMMIT_DETAIL_SEPARATOR_HEIGHT, COMMIT_DETAIL_SUBJECT_MAX_ROWS,
    COMPACT_COLUMNS_SCREEN_WIDTH, CONTAINING_BRANCHES_SINGULAR_LABEL,
    CONTAINING_BRANCHES_UNAVAILABLE_LABEL, DATE_AUTHOR_GAP, DATE_COLUMN_WIDTH, FILES_HEADER_HEIGHT,
    GRAPH_COLUMN_WIDTH, HEAD_REF_LABEL, HEAD_REF_LABEL_PREFIX, LANE_COLUMN_WIDTH, LANE_PALETTE,
    MAX_LISTED_CONTAINING_BRANCHES, MINIMUM_SUBJECT_WIDTH, NO_COMMITS_LABEL, NO_FILES_LABEL,
    REFS_SEPARATOR, REFS_SUBJECT_GAP, REMOTE_REF_PREFIX, SUBJECT_COLUMN, TAG_REF_LABEL_PREFIX,
    VISIBLE_LANE_COUNT,
};
pub use new_branch_dialog::{
    new_branch_dialog_layout, render_new_branch_dialog, NewBranchDialogLayout, NewBranchDialogView,
    NEW_BRANCH_DIALOG_HEIGHT, NEW_BRANCH_DIALOG_MAX_WIDTH, NEW_BRANCH_DIALOG_TITLE,
    NEW_BRANCH_INPUT_LABEL,
};
pub use pull_dialog::{
    pull_dialog_layout, pull_message_text, pull_strategy_marker, render_pull_dialog,
    PullDialogButton, PullDialogFocus, PullDialogLayout, PullDialogView, PullStrategyOption,
    PULL_CANCEL_BUTTON_LABEL, PULL_CHECKED_MARKER, PULL_CONFIRM_BUTTON_LABEL, PULL_DIALOG_HEIGHT,
    PULL_DIALOG_MAX_WIDTH, PULL_DIALOG_TITLE, PULL_FAST_FORWARD_ONLY_LABEL, PULL_MERGE_LABEL,
    PULL_REBASE_LABEL, PULL_STRATEGY_OPTIONS, PULL_UNCHECKED_MARKER,
};
pub use push_dialog::{
    push_dialog_layout, push_message_text, render_push_dialog, PushDialogButton, PushDialogFocus,
    PushDialogLayout, PushDialogView, PUSH_CANCEL_BUTTON_LABEL, PUSH_CONFIRM_BUTTON_LABEL,
    PUSH_DETACHED_HEAD_LABEL, PUSH_DIALOG_HEIGHT, PUSH_DIALOG_MAX_WIDTH, PUSH_DIALOG_TITLE,
    PUSH_FORCE_WITH_LEASE_LABEL, PUSH_SET_UPSTREAM_LABEL,
};

pub use shell::{
    branch_filter_block_label, branch_filter_block_rect, case_sensitive_filter_block_rect,
    changes_toolbar_entry_at, changes_toolbar_entry_rects, changes_toolbar_tooltip,
    divergence_label, editor_tab_hit_at, remote_tool_bar_rects, render_shell, take_display_prefix,
    take_display_suffix, text_display_width, text_filter_block_rect, top_bar_branch_chevron_rect,
    top_bar_branch_chip_rect, top_bar_divergence_rect, top_bar_help_button_rect,
    truncate_with_caps_ellipsis, truncate_with_ellipsis, visible_filter_bar_labels,
    ChangesToolbarAction, ChangesToolbarEntry, EditorTabHit, EditorTabStrip, LogFilterBarView,
    ShellView, StatusMessageView, StatusTone, CHANGES_TOOLBAR_ENTRIES, CHERRY_PICKING_LABEL,
    FILTER_BAR_BLOCK_LABELS, FILTER_BAR_BRANCH_BLOCK_INDEX, FILTER_BAR_CASE_SENSITIVE_BLOCK_INDEX,
    FILTER_BAR_TEXT_BLOCK_INDEX, LOG_TAB_TITLE, MERGING_LABEL, NO_REPOSITORY_LABEL, REBASING_LABEL,
    REMOTE_FETCH_TOOL_LABEL, REMOTE_PULL_TOOL_LABEL, REMOTE_PUSH_TOOL_LABEL,
    REMOTE_TOOL_BAR_LABELS, REVERTING_LABEL, TOP_BAR_HELP_LABEL, WORKING_TREE_TAB_SUFFIX,
};
pub use theme::{theme_palette, Theme};
pub use word_highlight::{assign_word_highlights, changed_char_ranges};

pub trait ScreenDriver {
    fn handle_event(&mut self, event: gitdeck_core::InputEvent);
    fn render_frame(&self, area: ratatui::layout::Rect, buffer: &mut ratatui::buffer::Buffer);
    fn focused_pane(&self) -> gitdeck_core::PaneId;
    fn pane_rect(&self, pane: gitdeck_core::PaneId) -> Option<ratatui::layout::Rect>;
}
