use std::collections::BTreeSet;

use gitdeck_caps::{CapsProfile, ThemeId};
use gitdeck_core::{ChangeEntry, ChangeKind, ConflictKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::display_text::sanitize_display_string;
use crate::glyphs::{ellipsis_glyph, IconGlyph};
use crate::next_step_hint::{next_step_hint, next_step_hint_prefix};
use crate::shell::{
    character_display_width, take_display_prefix, take_display_suffix, text_display_width,
};
use crate::theme::{theme_palette, Theme};

pub const NO_CHANGES_LABEL: &str = "No changes";
pub const CANNOT_OPEN_REPOSITORY_LABEL: &str = "Cannot open repository";
pub const REFRESH_FAILED_LABEL: &str = "Refresh failed";
pub const PUSH_FAILED_LABEL: &str = "Push failed";
pub const PULL_FAILED_LABEL: &str = "Pull failed";
pub const FETCH_FAILED_LABEL: &str = "Fetch failed";
pub const COMMIT_FAILED_LABEL: &str = "Commit failed";
pub const CHECKOUT_FAILED_LABEL: &str = "Checkout failed";
pub const CONFLICT_FAILED_LABEL: &str = "Conflict resolution failed";
pub const DISCARD_FAILED_LABEL: &str = "Discard failed";
pub const MERGE_FAILED_LABEL: &str = "Merge failed";
pub const OPERATION_FAILED_LABEL: &str = "Operation failed";
pub const BRANCH_FAILED_LABEL: &str = "Branch operation failed";
pub const IGNORE_FAILED_LABEL: &str = "Ignore failed";
pub const CONFLICT_RESOLUTION_HINT: &str = "alt+o ours  alt+t theirs  alt+r resolve";
pub const PANEL_ERROR_BODY_ROW_COUNT: u16 = 4;
pub const PANEL_ERROR_MAX_BLOCK_WIDTH: u16 = 96;
pub const PANEL_ERROR_SCREEN_MARGIN: u16 = 8;
const PANEL_ERROR_BORDER_ROW_COUNT: u16 = 2;
const PANEL_ERROR_OK_ROW_COUNT: u16 = 1;
pub const PANEL_ERROR_ROW_COUNT: u16 =
    PANEL_ERROR_BODY_ROW_COUNT + PANEL_ERROR_BORDER_ROW_COUNT + PANEL_ERROR_OK_ROW_COUNT;
pub const PANEL_ERROR_MIN_HEIGHT: u16 = 4;
pub const PANEL_ERROR_MIN_WIDTH: u16 = 10;
pub const PANEL_ERROR_OK_LABEL: &str = "[ OK ]";
const PANEL_ERROR_INSET: u16 = 2;
pub const OPEN_ERROR_REASON_ROW_COUNT: u16 = 3;
pub const CHANGES_PANEL_CONTENT_TOP: u16 = 2;
pub const GROUP_ARROW_COLUMN: u16 = 0;
pub const CHECKBOX_COLUMN: u16 = 2;
pub const CHECKBOX_WIDTH: u16 = 3;
pub const GROUP_TITLE_COLUMN: u16 = 6;
pub const FILE_ROW_INDENT: u16 = 2;
pub const FILE_STATUS_COLUMN: u16 = 6;
pub const FILE_WORKTREE_STATUS_COLUMN: u16 = 7;
pub const FILE_NAME_COLUMN: u16 = 9;
pub const STATUS_LETTER_WIDTH: u16 = 1;
const MIN_ELLIPSIZED_PATH_WIDTH: u16 = 2;
pub const FILE_DIRECTORY_GAP: u16 = 2;
const COUNT_LABEL_GAP: u16 = 1;
const COLUMN_GAP: u16 = 1;

pub const AMEND_ROW_HEIGHT: u16 = 1;
pub const AMEND_ROW_INDENT: u16 = 2;
pub const AMEND_LABEL: &str = "Amend last commit";
pub const AMEND_COMPACT_LABEL: &str = "Amend";
pub const AMEND_EXPAND_GAP: u16 = 1;
pub const COMMIT_MESSAGE_TITLE: &str = "Commit Message";
pub const COMMIT_MESSAGE_PLACEHOLDER: &str = "Commit Message";
pub const COMMIT_MESSAGE_COUNT_SUFFIX: &str = " chars";
pub const AMEND_FILE_NOTICE_SUFFIX: &str = " will be amended in)";
pub const AMEND_UNTRACKED_NOTICE_SUFFIX: &str = " untracked";
pub const COMMIT_MESSAGE_BOX_MIN_HEIGHT: u16 = 3;
pub const COMMIT_MESSAGE_HEIGHT_DIVISOR: u16 = 4;
const COMMIT_MESSAGE_RESERVED_ROW_COUNT: u16 =
    CHANGES_PANEL_CONTENT_TOP + AMEND_ROW_HEIGHT + COMMIT_BUTTON_ROW_HEIGHT;
pub const COMMIT_BUTTON_ROW_HEIGHT: u16 = 1;
pub const COMMIT_BUTTON_ROW_BOTTOM_INSET: u16 = 1;
pub const COMMIT_BUTTON_LABEL: &str = "[ Commit ]";
pub const AMEND_COMMIT_BUTTON_LABEL: &str = "[ Amend Commit ]";
pub const AMEND_COMMIT_BUTTON_COMPACT_LABEL: &str = "[ Amend ]";
pub const COMMIT_AND_PUSH_LABEL: &str = "[ Commit and Push… ]";
pub const COMMIT_AND_PUSH_ASCII_LABEL: &str = "[ Commit and Push... ]";
pub const COMMIT_AND_PUSH_COMPACT_ASCII_LABEL: &str = "[ Push... ]";

pub fn commit_button_label(amend: bool) -> &'static str {
    if amend {
        AMEND_COMMIT_BUTTON_LABEL
    } else {
        COMMIT_BUTTON_LABEL
    }
}

pub fn commit_button_compact_label(amend: bool) -> &'static str {
    if amend {
        AMEND_COMMIT_BUTTON_COMPACT_LABEL
    } else {
        COMMIT_BUTTON_LABEL
    }
}

pub fn commit_and_push_label(caps: CapsProfile) -> String {
    format!("[ Commit and Push{} ]", ellipsis_glyph(caps))
}

pub fn commit_and_push_compact_label(caps: CapsProfile) -> String {
    format!("[ Push{} ]", ellipsis_glyph(caps))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitButtonRow {
    FullLabels,
    CompactPush,
    CompactLabels,
    CommitOnly,
}

impl CommitButtonRow {
    pub fn commit_label(self, amend: bool) -> &'static str {
        match self {
            Self::FullLabels | Self::CompactPush => commit_button_label(amend),
            Self::CompactLabels | Self::CommitOnly => commit_button_compact_label(amend),
        }
    }

    pub fn push_label(self, caps: CapsProfile) -> Option<String> {
        match self {
            Self::FullLabels => Some(commit_and_push_label(caps)),
            Self::CompactPush | Self::CompactLabels => Some(commit_and_push_compact_label(caps)),
            Self::CommitOnly => None,
        }
    }

    fn push_label_width(self) -> u16 {
        let push_label = match self {
            Self::FullLabels => Some(COMMIT_AND_PUSH_ASCII_LABEL),
            Self::CompactPush | Self::CompactLabels => Some(COMMIT_AND_PUSH_COMPACT_ASCII_LABEL),
            Self::CommitOnly => None,
        };
        push_label.map_or(0, text_display_width)
    }

    fn measured_width(self, amend: bool) -> u16 {
        let push_label = match self {
            Self::FullLabels => Some(COMMIT_AND_PUSH_ASCII_LABEL),
            Self::CompactPush | Self::CompactLabels => Some(COMMIT_AND_PUSH_COMPACT_ASCII_LABEL),
            Self::CommitOnly => None,
        };
        let commit_width = match self {
            Self::FullLabels | Self::CompactPush => commit_button_label(amend),
            Self::CompactLabels | Self::CommitOnly => commit_button_compact_label(amend),
        };
        commit_button_row_width(commit_width, push_label)
    }
}

const COMMIT_BUTTON_ROWS: [CommitButtonRow; 4] = [
    CommitButtonRow::FullLabels,
    CommitButtonRow::CompactPush,
    CommitButtonRow::CompactLabels,
    CommitButtonRow::CommitOnly,
];

fn commit_button_row_width(commit_label: &str, push_label: Option<&str>) -> u16 {
    let push_width = push_label.map_or(0, |label| {
        COMMIT_BUTTON_GAP.saturating_add(text_display_width(label))
    });
    text_display_width(commit_label).saturating_add(push_width)
}

fn commit_button_row_for(available_width: u16, amend: bool) -> CommitButtonRow {
    COMMIT_BUTTON_ROWS
        .into_iter()
        .find(|row| row.measured_width(amend) <= available_width)
        .unwrap_or(CommitButtonRow::CommitOnly)
}
pub const COMMIT_BUTTON_GAP: u16 = 2;
pub const COMMIT_OPTIONS_INDENT: u16 = 2;
pub const COMMIT_OPTIONS_GAP: u16 = 1;
pub const SIGN_OFF_POPOVER_TITLE: &str = "Git";
pub const SIGN_OFF_LABEL: &str = "Sign-off commit";
pub const SIGN_OFF_POPOVER_ROW_COUNT: u16 = 3;
const SIGN_OFF_POPOVER_LEFT_PAD: u16 = 1;
const SIGN_OFF_POPOVER_BORDER_WIDTH: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ChangesGroup {
    MergeConflicts,
    Changes,
    Unversioned,
}

pub const CHANGES_GROUPS: [ChangesGroup; 3] = [
    ChangesGroup::MergeConflicts,
    ChangesGroup::Changes,
    ChangesGroup::Unversioned,
];

impl ChangesGroup {
    pub const fn title(self) -> &'static str {
        match self {
            Self::MergeConflicts => "Merge Conflicts",
            Self::Changes => "Changes",
            Self::Unversioned => "Unversioned Files",
        }
    }
}

pub fn changes_group_of(entry: &ChangeEntry) -> ChangesGroup {
    if entry.conflict.is_some() {
        return ChangesGroup::MergeConflicts;
    }
    match entry.kind {
        ChangeKind::Untracked => ChangesGroup::Unversioned,
        ChangeKind::Conflicted => ChangesGroup::MergeConflicts,
        ChangeKind::Modified
        | ChangeKind::Added
        | ChangeKind::Deleted
        | ChangeKind::Renamed
        | ChangeKind::TypeChanged => ChangesGroup::Changes,
    }
}

pub fn default_collapsed_groups() -> BTreeSet<ChangesGroup> {
    BTreeSet::new()
}

pub fn group_member_indices(entries: &[ChangeEntry], group: ChangesGroup) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| changes_group_of(entry) == group)
        .map(|(index, _)| index)
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CheckState {
    Checked,
    Unchecked,
    Partial,
}

impl CheckState {
    pub const fn from_counts(checked: usize, total: usize) -> Self {
        if checked == 0 || total == 0 {
            Self::Unchecked
        } else if checked == total {
            Self::Checked
        } else {
            Self::Partial
        }
    }

    pub const fn glyph(self) -> char {
        match self {
            Self::Checked => 'x',
            Self::Unchecked => ' ',
            Self::Partial => '-',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangesRow {
    Group {
        group: ChangesGroup,
        collapsed: bool,
        check: CheckState,
        count: usize,
    },
    File {
        group: ChangesGroup,
        index: usize,
        checked: bool,
    },
}

pub fn changes_rows(
    entries: &[ChangeEntry],
    collapsed: &BTreeSet<ChangesGroup>,
    checked: &BTreeSet<String>,
) -> Vec<ChangesRow> {
    let mut rows = Vec::new();
    for group in CHANGES_GROUPS {
        let member_indices = group_member_indices(entries, group);
        if member_indices.is_empty() {
            continue;
        }
        let checked_count = member_indices
            .iter()
            .filter(|index| checked.contains(&entries[**index].path))
            .count();
        let is_collapsed = collapsed.contains(&group);
        rows.push(ChangesRow::Group {
            group,
            collapsed: is_collapsed,
            check: CheckState::from_counts(checked_count, member_indices.len()),
            count: member_indices.len(),
        });
        if is_collapsed {
            continue;
        }
        rows.extend(member_indices.into_iter().map(|index| ChangesRow::File {
            group,
            index,
            checked: checked.contains(&entries[index].path),
        }));
    }
    rows
}

pub const fn change_kind_letter(kind: ChangeKind) -> char {
    match kind {
        ChangeKind::Modified => 'M',
        ChangeKind::TypeChanged => 'T',
        ChangeKind::Added => 'A',
        ChangeKind::Deleted => 'D',
        ChangeKind::Renamed => 'R',
        ChangeKind::Untracked => '?',
        ChangeKind::Conflicted => '!',
    }
}

pub fn change_kind_color(kind: ChangeKind, theme: &Theme) -> Color {
    match kind {
        ChangeKind::Modified | ChangeKind::TypeChanged => theme.warning,
        ChangeKind::Added | ChangeKind::Renamed => theme.success,
        ChangeKind::Deleted | ChangeKind::Conflicted => theme.error,
        ChangeKind::Untracked => theme.muted,
    }
}

pub fn file_count_label(count: usize) -> String {
    if count == 1 {
        format!("{count} file")
    } else {
        format!("{count} files")
    }
}

pub fn changes_content_area(area: Rect) -> Option<Rect> {
    let height = area.height.checked_sub(CHANGES_PANEL_CONTENT_TOP)?;
    (area.width > 0 && height > 0).then(|| {
        Rect::new(
            area.x,
            area.y + CHANGES_PANEL_CONTENT_TOP,
            area.width,
            height,
        )
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangesPanelLayout {
    pub list: Rect,
    pub amend_row: Rect,
    pub message_box: Rect,
    pub message_lines: Rect,
    pub button_row: Rect,
    pub commit_button: Rect,
    pub push_button: Rect,
    pub options_button: Rect,
    pub buttons: CommitButtonRow,
}

pub fn changes_panel_layout(area: Rect, amend: bool) -> Option<ChangesPanelLayout> {
    let content = changes_content_area(area)?;
    let message_height = commit_message_box_height(area.height);
    let button_row = Rect::new(
        content.x,
        area.bottom()
            .saturating_sub(COMMIT_BUTTON_ROW_BOTTOM_INSET)
            .saturating_sub(COMMIT_BUTTON_ROW_HEIGHT),
        content.width,
        COMMIT_BUTTON_ROW_HEIGHT,
    );
    let message_box = Rect::new(
        content.x,
        button_row.y.saturating_sub(message_height),
        content.width,
        message_height,
    );
    let amend_row = Rect::new(
        content.x,
        message_box.y.saturating_sub(AMEND_ROW_HEIGHT),
        content.width,
        AMEND_ROW_HEIGHT,
    );
    let list_height = amend_row.y.saturating_sub(content.y);
    let options_button = Rect::new(
        content
            .right()
            .saturating_sub(COMMIT_OPTIONS_INDENT)
            .min(content.right()),
        button_row.y,
        1.min(content.width),
        COMMIT_BUTTON_ROW_HEIGHT,
    );
    let buttons = commit_button_row_for(
        options_button
            .x
            .saturating_sub(content.x)
            .saturating_sub(COMMIT_OPTIONS_GAP),
        amend,
    );
    let commit_button = label_rect(
        content.x,
        button_row.y,
        buttons.commit_label(amend),
        options_button.x,
    );
    let push_button = if buttons.push_label_width() == 0 {
        Rect::new(commit_button.x, button_row.y, 0, COMMIT_BUTTON_ROW_HEIGHT)
    } else {
        Rect::new(
            commit_button
                .right()
                .saturating_add(COMMIT_BUTTON_GAP)
                .min(options_button.x),
            button_row.y,
            buttons.push_label_width().min(
                options_button
                    .x
                    .saturating_sub(commit_button.right().saturating_add(COMMIT_BUTTON_GAP)),
            ),
            COMMIT_BUTTON_ROW_HEIGHT,
        )
    };
    Some(ChangesPanelLayout {
        list: Rect::new(content.x, content.y, content.width, list_height),
        amend_row,
        message_box,
        message_lines: message_interior(message_box),
        button_row,
        commit_button,
        push_button,
        options_button,
        buttons,
    })
}

pub fn commit_message_box_height(panel_height: u16) -> u16 {
    (panel_height / COMMIT_MESSAGE_HEIGHT_DIVISOR)
        .max(COMMIT_MESSAGE_BOX_MIN_HEIGHT)
        .min(panel_height.saturating_sub(COMMIT_MESSAGE_RESERVED_ROW_COUNT))
        .max(1)
}

pub fn sign_off_popover(button_row: Rect) -> Option<(Rect, Rect)> {
    let width = text_display_width(SIGN_OFF_LABEL)
        .saturating_add(CHECKBOX_WIDTH)
        .saturating_add(COLUMN_GAP)
        .saturating_add(SIGN_OFF_POPOVER_LEFT_PAD)
        .saturating_add(SIGN_OFF_POPOVER_BORDER_WIDTH);
    let width = width.min(button_row.width);
    if width < 4 || button_row.y < SIGN_OFF_POPOVER_ROW_COUNT {
        return None;
    }
    let frame = Rect::new(
        button_row.right().saturating_sub(width),
        button_row.y.saturating_sub(SIGN_OFF_POPOVER_ROW_COUNT),
        width,
        SIGN_OFF_POPOVER_ROW_COUNT,
    );
    let sign_off_row = Rect::new(frame.x, frame.y + 1, frame.width, 1);
    Some((frame, sign_off_row))
}

fn label_rect(x: u16, y: u16, label: &str, right: u16) -> Rect {
    let width = text_display_width(label).min(right.saturating_sub(x));
    Rect::new(x, y, width, 1)
}

fn message_interior(message_box: Rect) -> Rect {
    if message_box.height <= 2 || message_box.width <= 4 {
        return message_box;
    }
    Rect::new(
        message_box.x + 2,
        message_box.y + 1,
        message_box.width - 4,
        message_box.height - 2,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelErrorSource {
    Commit,
    Push,
    Pull,
    Fetch,
    Refresh,
    Checkout,
    Conflict,
    Discard,
    Merge,
    Operation,
    Branch,
    Ignore,
}

impl PanelErrorSource {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Commit => COMMIT_FAILED_LABEL,
            Self::Push => PUSH_FAILED_LABEL,
            Self::Pull => PULL_FAILED_LABEL,
            Self::Fetch => FETCH_FAILED_LABEL,
            Self::Refresh => REFRESH_FAILED_LABEL,
            Self::Checkout => CHECKOUT_FAILED_LABEL,
            Self::Conflict => CONFLICT_FAILED_LABEL,
            Self::Discard => DISCARD_FAILED_LABEL,
            Self::Merge => MERGE_FAILED_LABEL,
            Self::Operation => OPERATION_FAILED_LABEL,
            Self::Branch => BRANCH_FAILED_LABEL,
            Self::Ignore => IGNORE_FAILED_LABEL,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelErrorView<'a> {
    pub source: PanelErrorSource,
    pub message: &'a str,
    pub scroll: usize,
}

pub struct ChangesPanelView<'a> {
    pub entries: &'a [ChangeEntry],
    pub rows: &'a [ChangesRow],
    pub selected_row: Option<usize>,
    pub focused: bool,
    pub caps: CapsProfile,
    pub theme: ThemeId,
    pub open_error: Option<&'a str>,
    pub panel_error: Option<PanelErrorView<'a>>,
    pub commit: CommitPanelView<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitPanelView<'a> {
    pub message: &'a str,
    pub editing: bool,
    pub amend: bool,
    pub sign_off: bool,
    pub commit_enabled: bool,
    pub options_visible: bool,
    pub checked_file_count: usize,
    pub checked_untracked_file_count: usize,
}

pub fn render_changes_panel(area: Rect, view: ChangesPanelView<'_>, buffer: &mut Buffer) {
    let theme = theme_palette(view.theme);
    let Some(layout) = changes_panel_layout(area, view.commit.amend) else {
        return;
    };
    render_changes_body(
        list_area(layout.list, view.panel_error.is_some()),
        &view,
        &theme,
        buffer,
    );
    render_amend_row(layout.amend_row, view.commit, view.caps, &theme, buffer);
    render_commit_message(layout, view.commit, &view, &theme, buffer);
    render_commit_button_row(layout, view.commit, view.caps, &theme, buffer);
    if view.commit.options_visible {
        render_sign_off_popover(layout.button_row, view.commit, view.caps, &theme, buffer);
    }
}

fn list_area(content: Rect, shows_panel_error: bool) -> Rect {
    if !shows_panel_error {
        return content;
    }
    let panel_error_height = panel_error_block_height(content);
    Rect::new(
        content.x,
        content.y,
        content.width,
        content.height.saturating_sub(panel_error_height),
    )
}

fn render_changes_body(
    content: Rect,
    view: &ChangesPanelView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if content.width == 0 || content.height == 0 {
        return;
    }
    if let Some(open_error) = view.open_error {
        render_open_error(content, open_error, view.caps, theme, buffer);
        return;
    }
    if view.rows.is_empty() {
        buffer.set_stringn(
            content.x,
            content.y,
            NO_CHANGES_LABEL,
            usize::from(content.width),
            Style::default().fg(theme.muted).bg(theme.panel),
        );
        return;
    }
    let visible_row_count = usize::from(content.height);
    for (visible_index, row) in view.rows.iter().take(visible_row_count).enumerate() {
        render_changes_row(content, visible_index, *row, view, theme, buffer);
    }
}

fn render_amend_row(
    amend_row: Rect,
    view: CommitPanelView<'_>,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if amend_row.width == 0 || amend_row.height == 0 {
        return;
    }
    buffer.set_style(amend_row, Style::default().fg(theme.text).bg(theme.panel));
    let segments = amend_row_segments(view, amend_row.width, caps, theme);
    write_row_segments(amend_row, amend_row.y, &segments, theme.panel, caps, buffer);
}

fn amend_row_segments(
    view: CommitPanelView<'_>,
    row_width: u16,
    caps: CapsProfile,
    theme: &Theme,
) -> Vec<RowSegment> {
    let check = if view.amend {
        CheckState::Checked
    } else {
        CheckState::Unchecked
    };
    let (label, notices) = amend_row_notices(view, row_width, caps);
    let mut segments = vec![
        RowSegment::Pad(AMEND_ROW_INDENT),
        checkbox_segment(check, theme),
        RowSegment::Pad(COLUMN_GAP),
        RowSegment::Text {
            text: String::from(label),
            color: theme.text,
        },
        RowSegment::Pad(AMEND_EXPAND_GAP),
        RowSegment::Text {
            text: IconGlyph::Chevron.character(caps).to_string(),
            color: theme.muted,
        },
    ];
    for (index, notice) in notices.into_iter().enumerate() {
        segments.push(RowSegment::Pad(COLUMN_GAP));
        segments.push(RowSegment::Text {
            text: notice,
            color: if index == 0 {
                theme.muted
            } else {
                theme.warning
            },
        });
    }
    segments
}

fn amend_row_notices(
    view: CommitPanelView<'_>,
    row_width: u16,
    caps: CapsProfile,
) -> (&'static str, Vec<String>) {
    if !view.amend || view.checked_file_count == 0 {
        return (AMEND_LABEL, Vec::new());
    }
    let untracked = (view.checked_untracked_file_count > 0)
        .then(|| amend_untracked_notice(view.checked_untracked_file_count));
    let mut candidates = vec![
        (
            AMEND_LABEL,
            vec![
                Some(amend_file_notice(view.checked_file_count)),
                untracked.clone(),
            ],
        ),
        (
            AMEND_LABEL,
            vec![
                Some(compact_amend_file_notice(view.checked_file_count)),
                untracked.clone(),
            ],
        ),
        (
            AMEND_COMPACT_LABEL,
            vec![Some(amend_summary_notice(
                view.checked_file_count,
                view.checked_untracked_file_count,
            ))],
        ),
        (
            AMEND_COMPACT_LABEL,
            vec![Some(compact_amend_summary_notice(
                view.checked_file_count,
                view.checked_untracked_file_count,
            ))],
        ),
    ];
    if untracked.is_none() {
        candidates.push((
            AMEND_COMPACT_LABEL,
            vec![Some(compact_amend_file_notice(view.checked_file_count))],
        ));
    }
    for (label, notices) in candidates {
        let notices: Vec<String> = notices.into_iter().flatten().collect();
        if amend_row_needed_width(label, &notices, caps) <= row_width {
            return (label, notices);
        }
    }
    (AMEND_COMPACT_LABEL, Vec::new())
}

fn amend_row_needed_width(label: &str, notices: &[String], caps: CapsProfile) -> u16 {
    let prefix_width = AMEND_ROW_INDENT
        .saturating_add(CHECKBOX_WIDTH)
        .saturating_add(COLUMN_GAP)
        .saturating_add(text_display_width(label))
        .saturating_add(AMEND_EXPAND_GAP)
        .saturating_add(text_display_width(
            &IconGlyph::Chevron.character(caps).to_string(),
        ));
    notices.iter().fold(prefix_width, |width, notice| {
        width
            .saturating_add(COLUMN_GAP)
            .saturating_add(text_display_width(notice))
    })
}

pub fn amend_file_notice(checked_file_count: usize) -> String {
    format!(
        "({}{AMEND_FILE_NOTICE_SUFFIX}",
        file_count_label(checked_file_count)
    )
}

pub fn compact_amend_file_notice(checked_file_count: usize) -> String {
    format!("({})", file_count_label(checked_file_count))
}

pub fn compact_amend_summary_notice(
    checked_file_count: usize,
    checked_untracked_file_count: usize,
) -> String {
    format!(
        "{}, {}{AMEND_UNTRACKED_NOTICE_SUFFIX}",
        file_count_label(checked_file_count),
        checked_untracked_file_count
    )
}

pub fn amend_summary_notice(
    checked_file_count: usize,
    checked_untracked_file_count: usize,
) -> String {
    format!(
        "{} ({}{AMEND_UNTRACKED_NOTICE_SUFFIX})",
        file_count_label(checked_file_count),
        checked_untracked_file_count
    )
}

pub fn amend_untracked_notice(checked_untracked_file_count: usize) -> String {
    format!(", {checked_untracked_file_count}{AMEND_UNTRACKED_NOTICE_SUFFIX}")
}

fn render_commit_message(
    layout: ChangesPanelLayout,
    view: CommitPanelView<'_>,
    panel: &ChangesPanelView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let message_box = layout.message_box;
    if message_box.width == 0 || message_box.height == 0 {
        return;
    }
    buffer.set_style(message_box, Style::default().fg(theme.text).bg(theme.panel));
    if message_box.height < COMMIT_MESSAGE_BOX_MIN_HEIGHT {
        render_message_text(layout.message_lines, view, panel.caps, theme, buffer);
        return;
    }
    render_message_box_border(
        message_box,
        COMMIT_MESSAGE_TITLE,
        box_glyphs(panel.caps),
        theme,
        buffer,
    );
    render_message_character_count(message_box, view.message, panel.caps, theme, buffer);
    render_message_text(layout.message_lines, view, panel.caps, theme, buffer);
}

fn render_message_character_count(
    frame: Rect,
    message: &str,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let label = message_character_count_label(message);
    let label_width = text_display_width(&label);
    let title_width = text_display_width(COMMIT_MESSAGE_TITLE).saturating_add(2);
    let inner_width = frame.width.saturating_sub(2);
    if inner_width < label_width.saturating_add(title_width).saturating_add(2) {
        return;
    }
    let column = frame.right().saturating_sub(label_width).saturating_sub(2);
    let sanitized_label = sanitize_display_string(&label, column, caps);
    buffer.set_stringn(
        column,
        frame.y,
        &sanitized_label,
        usize::from(label_width),
        Style::default().fg(theme.muted).bg(theme.panel),
    );
}

pub fn message_character_count_label(message: &str) -> String {
    format!("{}{COMMIT_MESSAGE_COUNT_SUFFIX}", message.chars().count())
}

fn render_message_box_border(
    frame: Rect,
    title: &str,
    glyphs: BoxGlyphs,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let border_style = Style::default().fg(theme.border).bg(theme.panel);
    let top = frame.y;
    let bottom = frame.bottom().saturating_sub(1);
    let right = frame.right().saturating_sub(1);
    put_char(buffer, frame.x, top, glyphs.top_left, border_style);
    put_char(buffer, right, top, glyphs.top_right, border_style);
    put_char(buffer, frame.x, bottom, glyphs.bottom_left, border_style);
    put_char(buffer, right, bottom, glyphs.bottom_right, border_style);
    for row in frame.y + 1..bottom {
        put_char(buffer, frame.x, row, glyphs.vertical, border_style);
        put_char(buffer, right, row, glyphs.vertical, border_style);
    }
    for column in frame.x + 1..right {
        put_char(buffer, column, top, glyphs.horizontal, border_style);
        put_char(buffer, column, bottom, glyphs.horizontal, border_style);
    }
    buffer.set_stringn(
        frame.x.saturating_add(1),
        top,
        format!(" {title} "),
        usize::from(right.saturating_sub(frame.x).saturating_sub(1)),
        Style::default().fg(theme.muted).bg(theme.panel),
    );
}

fn render_message_text(
    area: Rect,
    view: CommitPanelView<'_>,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text_style = Style::default().fg(theme.text).bg(theme.panel);
    if !view.editing && view.message.is_empty() {
        buffer.set_stringn(
            area.x,
            area.y,
            COMMIT_MESSAGE_PLACEHOLDER,
            usize::from(area.width),
            Style::default().fg(theme.muted).bg(theme.panel),
        );
        return;
    }
    let visible_lines = visible_message_lines(view.message, area.width, usize::from(area.height));
    let caret_line_index = visible_lines.len().saturating_sub(1);
    for (offset, line) in visible_lines.iter().enumerate() {
        let Ok(offset) = u16::try_from(offset) else {
            return;
        };
        let sanitized_line = sanitize_display_string(line, area.x, caps);
        buffer.set_stringn(
            area.x,
            area.y + offset,
            &sanitized_line,
            usize::from(area.width),
            text_style,
        );
        if view.editing && offset as usize == caret_line_index {
            render_message_caret(area, area.y + offset, &sanitized_line, theme, buffer);
        }
    }
}

fn visible_message_lines(message: &str, width: u16, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    let mut lines = wrapped_message_lines(message, width);
    if width > 0
        && lines
            .last()
            .is_some_and(|line| text_display_width(line) >= width)
    {
        lines.push(String::new());
    }
    let first_visible = lines.len().saturating_sub(max_lines);
    lines.split_off(first_visible)
}

fn wrapped_message_lines(message: &str, width: u16) -> Vec<String> {
    message
        .split('\n')
        .flat_map(|line| wrap_message_line(line, width))
        .collect()
}

fn wrap_message_line(line: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return vec![line.to_string()];
    }
    let mut wrapped: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0u16;
    let mut break_index: Option<usize> = None;
    for character in line.chars() {
        let character_width = character_display_width(character);
        if current_width.saturating_add(character_width) > width && !current.is_empty() {
            match break_index.take() {
                Some(index) => {
                    let rest = current.split_off(index);
                    wrapped.push(std::mem::take(&mut current));
                    current_width = text_display_width(&rest);
                    current = rest;
                }
                None => {
                    wrapped.push(std::mem::take(&mut current));
                    current_width = 0;
                }
            }
        }
        current.push(character);
        current_width = current_width.saturating_add(character_width);
        if character == ' ' {
            break_index = Some(current.len());
        }
        if character_width > width {
            wrapped.push(std::mem::take(&mut current));
            current_width = 0;
            break_index = None;
        }
    }
    if !current.is_empty() || wrapped.is_empty() {
        wrapped.push(current);
    }
    wrapped
}

fn render_message_caret(
    area: Rect,
    row: u16,
    caret_line: &str,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if row >= area.bottom() {
        return;
    }
    let caret_column = area
        .x
        .saturating_add(text_display_width(caret_line))
        .min(area.right().saturating_sub(1));
    let Some(cell) = buffer.cell_mut((caret_column, row)) else {
        return;
    };
    cell.set_symbol(" ");
    cell.set_style(Style::default().fg(theme.panel).bg(theme.text));
}

fn render_commit_button_row(
    layout: ChangesPanelLayout,
    view: CommitPanelView<'_>,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let button_row = layout.button_row;
    if button_row.width == 0 || button_row.height == 0 {
        return;
    }
    buffer.set_style(button_row, Style::default().fg(theme.text).bg(theme.panel));
    let label_color = commit_label_color(view.commit_enabled, theme);
    let mut labels = vec![(
        layout.commit_button,
        String::from(layout.buttons.commit_label(view.amend)),
    )];
    if let Some(push_label) = layout.buttons.push_label(caps) {
        labels.push((layout.push_button, push_label));
    }
    for (button, label) in labels {
        if button.width == 0 {
            continue;
        }
        buffer.set_stringn(
            button.x,
            button.y,
            &label,
            usize::from(button.width),
            Style::default().fg(label_color).bg(theme.panel),
        );
    }
    let options = layout.options_button;
    if options.width == 0 {
        return;
    }
    put_char(
        buffer,
        options.x,
        options.y,
        IconGlyph::Gear.character(caps),
        Style::default().fg(theme.muted).bg(theme.panel),
    );
}

pub fn commit_label_color(commit_enabled: bool, theme: &Theme) -> Color {
    if commit_enabled {
        theme.accent
    } else {
        theme.muted
    }
}

fn render_sign_off_popover(
    button_row: Rect,
    view: CommitPanelView<'_>,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let Some((frame, sign_off_row)) = sign_off_popover(button_row) else {
        return;
    };
    buffer.set_style(frame, Style::default().fg(theme.text).bg(theme.panel));
    render_message_box_border(
        frame,
        SIGN_OFF_POPOVER_TITLE,
        box_glyphs(caps),
        theme,
        buffer,
    );
    let check = if view.sign_off {
        CheckState::Checked
    } else {
        CheckState::Unchecked
    };
    let segments = vec![
        RowSegment::Pad(SIGN_OFF_POPOVER_LEFT_PAD),
        checkbox_segment(check, theme),
        RowSegment::Pad(COLUMN_GAP),
        RowSegment::Text {
            text: String::from(SIGN_OFF_LABEL),
            color: theme.text,
        },
    ];
    let row = Rect::new(
        sign_off_row.x.saturating_add(1),
        sign_off_row.y,
        sign_off_row.width.saturating_sub(2),
        1,
    );
    write_row_segments(row, row.y, &segments, theme.panel, caps, buffer);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoxGlyphs {
    pub horizontal: char,
    pub vertical: char,
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
}

pub const fn box_glyphs(caps: CapsProfile) -> BoxGlyphs {
    match caps {
        CapsProfile::Rich | CapsProfile::Standard => BoxGlyphs {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        },
        CapsProfile::Basic => BoxGlyphs {
            horizontal: '-',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        },
    }
}

fn put_char(buffer: &mut Buffer, column: u16, row: u16, character: char, style: Style) {
    if let Some(cell) = buffer.cell_mut((column, row)) {
        cell.set_symbol(&character.to_string());
        cell.set_style(style);
    }
}

fn paint_blank_area(area: Rect, style: Style, buffer: &mut Buffer) {
    let clipped = buffer.area.intersection(area);
    for row in clipped.top()..clipped.bottom() {
        for column in clipped.left()..clipped.right() {
            if let Some(cell) = buffer.cell_mut((column, row)) {
                cell.set_symbol(" ");
                cell.set_style(style);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelErrorLayout {
    pub block: Rect,
    pub body: Rect,
    pub ok_button: Rect,
}

pub fn panel_error_block_height(content: Rect) -> u16 {
    if content.height < PANEL_ERROR_MIN_HEIGHT || content.width < PANEL_ERROR_MIN_WIDTH {
        return 0;
    }
    PANEL_ERROR_ROW_COUNT.min(content.height)
}

pub fn panel_error_block_width(content: Rect, screen: Rect) -> u16 {
    let screen_limit = screen
        .width
        .saturating_sub(PANEL_ERROR_SCREEN_MARGIN)
        .min(PANEL_ERROR_MAX_BLOCK_WIDTH);
    let space_right_of_content = screen.right().saturating_sub(content.x);
    screen_limit.max(content.width).min(space_right_of_content)
}

pub fn panel_error_layout(content: Rect, screen: Rect) -> Option<PanelErrorLayout> {
    let height = panel_error_block_height(content);
    if height == 0 {
        return None;
    }
    let block = Rect::new(
        content.x,
        content.bottom().saturating_sub(height),
        panel_error_block_width(content, screen),
        height,
    );
    let ok_button_y = block.bottom().saturating_sub(PANEL_ERROR_OK_ROW_COUNT + 1);
    let interior_width = block.width.saturating_sub(PANEL_ERROR_INSET * 2);
    let ok_button = Rect::new(
        block.x.saturating_add(PANEL_ERROR_INSET),
        ok_button_y,
        text_display_width(PANEL_ERROR_OK_LABEL).min(interior_width),
        PANEL_ERROR_OK_ROW_COUNT,
    );
    let body_top = block.y.saturating_add(1);
    let body = Rect::new(
        block.x.saturating_add(PANEL_ERROR_INSET),
        body_top,
        interior_width,
        ok_button_y.saturating_sub(body_top),
    );
    Some(PanelErrorLayout {
        block,
        body,
        ok_button,
    })
}

pub fn render_panel_error(
    content: Rect,
    screen: Rect,
    panel_error: PanelErrorView<'_>,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let Some(panel_error_layout) = panel_error_layout(content, screen) else {
        return;
    };
    let block = panel_error_layout.block;
    paint_blank_area(
        block,
        Style::default().fg(theme.muted).bg(theme.panel),
        buffer,
    );
    draw_panel_error_border(block, panel_error.source.title(), caps, theme, buffer);
    let body = panel_error_layout.body;
    let hint = next_step_hint(panel_error.message);
    let body_rows = usize::from(body.height).saturating_sub(usize::from(hint.is_some()));
    let wrapped_lines = wrap_error_body_lines(panel_error.message, body.width);
    let visible_lines =
        visible_error_body_lines(&wrapped_lines, panel_error.scroll, body_rows, body.width);
    render_error_body_lines(body, &visible_lines, theme, buffer);
    if let Some(hint) = hint {
        render_next_step_hint(body, body_rows, hint, caps, theme, buffer);
    }
    if wrapped_lines.len() > body_rows {
        render_error_scroll_indicator(block, panel_error_layout.ok_button.y, caps, theme, buffer);
    }
    let ok_button = panel_error_layout.ok_button;
    buffer.set_stringn(
        ok_button.x,
        ok_button.y,
        PANEL_ERROR_OK_LABEL,
        usize::from(ok_button.width),
        Style::default().fg(theme.accent).bg(theme.panel),
    );
    repaint_blank_cells_in_block(block, theme.panel, buffer);
}

fn repaint_blank_cells_in_block(block: Rect, background: Color, buffer: &mut Buffer) {
    let style = Style::default().bg(background);
    let clipped = buffer.area.intersection(block);
    for row in clipped.top()..clipped.bottom() {
        for column in clipped.left()..clipped.right() {
            if let Some(cell) = buffer.cell_mut((column, row)) {
                if cell.symbol() == " " {
                    cell.set_style(style);
                }
            }
        }
    }
}

fn draw_panel_error_border(
    block: Rect,
    title: &str,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let glyphs = box_glyphs(caps);
    let border_style = Style::default().fg(theme.error).bg(theme.panel);
    let bottom_row = block.bottom().saturating_sub(1);
    let right_column = block.right().saturating_sub(1);
    for column in block.x..block.right() {
        put_char(buffer, column, block.y, glyphs.horizontal, border_style);
        put_char(buffer, column, bottom_row, glyphs.horizontal, border_style);
    }
    for row in block.y..block.bottom() {
        put_char(buffer, block.x, row, glyphs.vertical, border_style);
        put_char(buffer, right_column, row, glyphs.vertical, border_style);
    }
    put_char(buffer, block.x, block.y, glyphs.top_left, border_style);
    put_char(
        buffer,
        right_column,
        block.y,
        glyphs.top_right,
        border_style,
    );
    put_char(
        buffer,
        block.x,
        bottom_row,
        glyphs.bottom_left,
        border_style,
    );
    put_char(
        buffer,
        right_column,
        bottom_row,
        glyphs.bottom_right,
        border_style,
    );
    put_char(
        buffer,
        block.x.saturating_add(1),
        block.y,
        ' ',
        border_style,
    );
    let title_column = block.x.saturating_add(PANEL_ERROR_INSET);
    buffer.set_stringn(
        title_column,
        block.y,
        title,
        usize::from(right_column.saturating_sub(title_column)),
        Style::default().fg(theme.error).bg(theme.panel),
    );
}

fn render_open_error(
    content: Rect,
    open_error: &str,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    buffer.set_style(content, Style::default().fg(theme.text).bg(theme.panel));
    buffer.set_stringn(
        content.x,
        content.y,
        CANNOT_OPEN_REPOSITORY_LABEL,
        usize::from(content.width),
        Style::default().fg(theme.error).bg(theme.panel),
    );
    let reason_area = Rect::new(
        content.x,
        content.y.saturating_add(1),
        content.width,
        OPEN_ERROR_REASON_ROW_COUNT.min(content.height.saturating_sub(1)),
    );
    render_wrapped_error_text(reason_area, open_error, caps, theme, buffer);
}

fn render_wrapped_error_text(
    area: Rect,
    text: &str,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let lines = wrap_error_text(text, area.width, usize::from(area.height), caps);
    let style = Style::default().fg(theme.muted).bg(theme.panel);
    for (row_offset, line) in lines.iter().enumerate() {
        let Ok(row_offset) = u16::try_from(row_offset) else {
            return;
        };
        if row_offset >= area.height {
            return;
        }
        buffer.set_stringn(
            area.x,
            area.y + row_offset,
            sanitize_display_string(line, area.x, caps),
            usize::from(area.width),
            style,
        );
    }
}

fn render_error_body_lines(area: Rect, lines: &[String], theme: &Theme, buffer: &mut Buffer) {
    let style = Style::default().fg(theme.muted).bg(theme.panel);
    for (row_offset, line) in lines.iter().enumerate() {
        let Ok(row_offset) = u16::try_from(row_offset) else {
            return;
        };
        if row_offset >= area.height {
            return;
        }
        buffer.set_stringn(
            area.x,
            area.y + row_offset,
            line,
            usize::from(area.width),
            style,
        );
    }
}

fn render_next_step_hint(
    area: Rect,
    row_offset: usize,
    hint: &str,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let Ok(row_offset) = u16::try_from(row_offset) else {
        return;
    };
    if row_offset >= area.height {
        return;
    }
    let line = format!("{}{hint}", next_step_hint_prefix(caps));
    buffer.set_stringn(
        area.x,
        area.y + row_offset,
        &line,
        usize::from(area.width),
        Style::default().fg(theme.muted).bg(theme.panel),
    );
}

fn render_error_scroll_indicator(
    block: Rect,
    row: u16,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let text: String = [IconGlyph::ArrowUp, IconGlyph::ArrowDown]
        .iter()
        .map(|glyph| glyph.character(caps))
        .collect();
    let width = text_display_width(&text);
    let column = block.right().saturating_sub(width).saturating_sub(2);
    buffer.set_stringn(
        column,
        row,
        &text,
        usize::from(width),
        Style::default().fg(theme.muted).bg(theme.panel),
    );
}

pub(crate) fn wrap_error_text(
    text: &str,
    width: u16,
    max_lines: usize,
    caps: CapsProfile,
) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines = greedy_error_lines(text, width);
    if lines.len() <= max_lines {
        return lines;
    }
    lines.truncate(max_lines);
    let last_line = lines.pop().unwrap_or_default();
    lines.push(ellipsized_line(&last_line, width, caps));
    lines
}

pub fn wrap_error_body_lines(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    greedy_error_lines(text, width)
}

pub fn error_body_line_count(text: &str, width: u16) -> usize {
    wrap_error_body_lines(text, width).len()
}

pub fn visible_error_body_lines(
    lines: &[String],
    scroll: usize,
    row_count: usize,
    width: u16,
) -> Vec<String> {
    if row_count == 0 || width == 0 || lines.is_empty() {
        return Vec::new();
    }
    let start = scroll.min(lines.len().saturating_sub(row_count));
    let end = (start + row_count).min(lines.len());
    let mut visible = lines[start..end].to_vec();
    if end < lines.len() {
        let remaining = lines.len() - end + 1;
        let marker_row = visible.pop().unwrap_or_default();
        visible.push(overflow_marker_line(&marker_row, remaining, width));
    }
    visible
}

fn overflow_marker_line(line: &str, remaining: usize, width: u16) -> String {
    let marker = format!(" (+{remaining} more lines)");
    let marker_width = text_display_width(&marker);
    if marker_width >= width {
        return take_display_prefix(&marker, width).0.to_string();
    }
    let (prefix, _) = take_display_prefix(line, width - marker_width);
    format!("{}{marker}", prefix.trim_end())
}

fn greedy_error_lines(text: &str, width: u16) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0u16;
    for word in text.split_whitespace() {
        let mut remaining_word = word;
        while !remaining_word.is_empty() {
            let word_width = text_display_width(remaining_word);
            let separator_width = u16::from(!current.is_empty());
            if current_width + separator_width + word_width <= width {
                if separator_width == 1 {
                    current.push(' ');
                }
                current.push_str(remaining_word);
                current_width += separator_width + word_width;
                break;
            }
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
                continue;
            }
            let (prefix, rest) = take_display_prefix(remaining_word, width);
            if prefix.is_empty() {
                let first_character_length = remaining_word
                    .char_indices()
                    .nth(1)
                    .map(|(index, _)| index)
                    .unwrap_or(remaining_word.len());
                let (head, tail) = remaining_word.split_at(first_character_length);
                lines.push(head.to_string());
                remaining_word = tail;
                continue;
            }
            lines.push(prefix.to_string());
            remaining_word = rest;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn ellipsized_line(line: &str, width: u16, caps: CapsProfile) -> String {
    let ellipsis = ellipsis_glyph(caps);
    let with_ellipsis = format!("{line}{ellipsis}");
    if text_display_width(&with_ellipsis) <= width {
        return with_ellipsis;
    }
    let (prefix, _) = take_display_prefix(line, width.saturating_sub(1));
    format!("{prefix}{ellipsis}")
}

fn render_changes_row(
    content: Rect,
    visible_index: usize,
    row: ChangesRow,
    view: &ChangesPanelView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let Ok(row_offset) = u16::try_from(visible_index) else {
        return;
    };
    let row_y = content.y + row_offset;
    if row_y >= content.bottom() {
        return;
    }
    let background = selection_background(
        theme,
        view.focused,
        view.selected_row == Some(visible_index),
        theme.panel,
    );
    buffer.set_style(
        Rect::new(content.x, row_y, content.width, 1),
        Style::default().fg(theme.text).bg(background),
    );
    let segments = row_segments(row, view.entries, content.width, view.caps, theme);
    write_row_segments(content, row_y, &segments, background, view.caps, buffer);
    repaint_blank_cell_backgrounds(content, row_y, theme, background, buffer);
}

pub(crate) fn selection_background(
    theme: &Theme,
    focused: bool,
    selected: bool,
    row_background: Color,
) -> Color {
    if !selected {
        return row_background;
    }
    if focused {
        theme.selection
    } else {
        theme.border
    }
}

pub(crate) enum RowSegment {
    Pad(u16),
    Text { text: String, color: Color },
}

fn row_segments(
    row: ChangesRow,
    entries: &[ChangeEntry],
    row_width: u16,
    caps: CapsProfile,
    theme: &Theme,
) -> Vec<RowSegment> {
    match row {
        ChangesRow::Group {
            group,
            collapsed,
            check,
            count,
        } => group_row_segments(group, collapsed, check, count, caps, theme),
        ChangesRow::File { index, checked, .. } => {
            file_row_segments(&entries[index], checked, row_width, caps, theme)
        }
    }
}

fn group_row_segments(
    group: ChangesGroup,
    collapsed: bool,
    check: CheckState,
    count: usize,
    caps: CapsProfile,
    theme: &Theme,
) -> Vec<RowSegment> {
    let arrow = if collapsed {
        IconGlyph::Pointer
    } else {
        IconGlyph::Chevron
    };
    vec![
        RowSegment::Text {
            text: arrow.character(caps).to_string(),
            color: theme.muted,
        },
        RowSegment::Pad(COLUMN_GAP),
        checkbox_segment(check, theme),
        RowSegment::Pad(COLUMN_GAP),
        RowSegment::Text {
            text: group.title().to_string(),
            color: theme.text,
        },
        RowSegment::Pad(COUNT_LABEL_GAP),
        RowSegment::Text {
            text: file_count_label(count),
            color: theme.muted,
        },
    ]
}

fn file_row_segments(
    entry: &ChangeEntry,
    checked: bool,
    row_width: u16,
    caps: CapsProfile,
    theme: &Theme,
) -> Vec<RowSegment> {
    let (file_name, directory) = split_file_path(&entry.path);
    let check = if checked {
        CheckState::Checked
    } else {
        CheckState::Unchecked
    };
    let mut segments = vec![
        RowSegment::Pad(FILE_ROW_INDENT),
        checkbox_segment(check, theme),
        RowSegment::Pad(COLUMN_GAP),
    ];
    segments.extend(status_column_segments(entry, theme));
    segments.push(RowSegment::Pad(COLUMN_GAP));
    let name_budget = row_width.saturating_sub(FILE_NAME_COLUMN);
    let name_width = text_display_width(&file_name);
    if name_width > name_budget {
        push_row_text(
            &mut segments,
            ellipsized_path_tail(&file_name, name_budget, caps),
            theme.text,
        );
        return segments;
    }
    push_row_text(&mut segments, file_name, theme.text);
    if directory.is_empty() {
        return segments;
    }
    let directory_budget = name_budget
        .saturating_sub(name_width)
        .saturating_sub(FILE_DIRECTORY_GAP);
    if directory_budget < MIN_ELLIPSIZED_PATH_WIDTH {
        return segments;
    }
    let directory_tail = ellipsized_path_tail(&directory, directory_budget, caps);
    if directory_tail.is_empty() {
        return segments;
    }
    segments.push(RowSegment::Pad(FILE_DIRECTORY_GAP));
    push_row_text(&mut segments, directory_tail, theme.muted);
    segments
}

fn push_row_text(segments: &mut Vec<RowSegment>, text: String, color: Color) {
    if text.is_empty() {
        return;
    }
    segments.push(RowSegment::Text { text, color });
}

fn ellipsized_path_tail(path: &str, available_width: u16, caps: CapsProfile) -> String {
    if available_width == 0 {
        return String::new();
    }
    if text_display_width(path) <= available_width {
        return path.to_string();
    }
    let ellipsis = ellipsis_glyph(caps);
    let lead = format!("{ellipsis}/");
    let lead_width = text_display_width(&lead);
    let mut parent_segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let Some(last_segment) = parent_segments.pop() else {
        return ellipsized_text_tail(path, available_width, caps);
    };
    if lead_width.saturating_add(text_display_width(last_segment)) > available_width {
        return ellipsized_text_tail(path, available_width, caps);
    }
    let mut tail = last_segment.to_string();
    while let Some(parent) = parent_segments.pop() {
        let candidate = format!("{parent}/{tail}");
        if lead_width.saturating_add(text_display_width(&candidate)) > available_width {
            break;
        }
        tail = candidate;
    }
    format!("{lead}{tail}")
}

fn ellipsized_text_tail(text: &str, available_width: u16, caps: CapsProfile) -> String {
    let ellipsis = ellipsis_glyph(caps);
    if available_width <= 1 {
        return ellipsis.to_string();
    }
    format!(
        "{ellipsis}{}",
        take_display_suffix(text, available_width.saturating_sub(1))
    )
}

pub const fn conflict_status_letters(conflict: ConflictKind) -> [char; 2] {
    match conflict {
        ConflictKind::BothDeleted => ['D', 'D'],
        ConflictKind::AddedByUs => ['A', 'U'],
        ConflictKind::DeletedByThem => ['U', 'D'],
        ConflictKind::AddedByThem => ['U', 'A'],
        ConflictKind::DeletedByUs => ['D', 'U'],
        ConflictKind::BothAdded => ['A', 'A'],
        ConflictKind::BothModified => ['U', 'U'],
    }
}

fn status_column_segments(entry: &ChangeEntry, theme: &Theme) -> [RowSegment; 2] {
    match entry.conflict {
        Some(conflict) => conflict_status_letters(conflict).map(|letter| RowSegment::Text {
            text: letter.to_string(),
            color: theme.error,
        }),
        None => [
            status_letter_segment(entry.index_kind, theme),
            status_letter_segment(entry.worktree_kind, theme),
        ],
    }
}

fn status_letter_segment(kind: Option<ChangeKind>, theme: &Theme) -> RowSegment {
    match kind {
        Some(kind) => RowSegment::Text {
            text: change_kind_letter(kind).to_string(),
            color: change_kind_color(kind, theme),
        },
        None => RowSegment::Pad(STATUS_LETTER_WIDTH),
    }
}

fn checkbox_segment(check: CheckState, theme: &Theme) -> RowSegment {
    let color = match check {
        CheckState::Checked | CheckState::Partial => theme.text,
        CheckState::Unchecked => theme.muted,
    };
    RowSegment::Text {
        text: format!("[{}]", check.glyph()),
        color,
    }
}

fn split_file_path(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(separator) => (
            path[separator + 1..].to_string(),
            path[..separator].to_string(),
        ),
        None => (path.to_string(), String::new()),
    }
}

pub(crate) fn write_row_segments(
    content: Rect,
    row_y: u16,
    segments: &[RowSegment],
    background: Color,
    caps: CapsProfile,
    buffer: &mut Buffer,
) {
    let mut column = content.x;
    for segment in segments {
        if column >= content.right() {
            return;
        }
        let available_width = content.right().saturating_sub(column);
        match segment {
            RowSegment::Pad(width) => {
                let pad_width = (*width).min(available_width);
                buffer.set_stringn(
                    column,
                    row_y,
                    " ".repeat(usize::from(pad_width)),
                    usize::from(pad_width),
                    Style::default().bg(background),
                );
                column += pad_width;
            }
            RowSegment::Text { text, color } => {
                let sanitized_text = sanitize_display_string(text, column, caps);
                let (next_column, _) = buffer.set_stringn(
                    column,
                    row_y,
                    &sanitized_text,
                    usize::from(available_width),
                    Style::default().fg(*color).bg(background),
                );
                column = next_column;
            }
        }
    }
}

pub(crate) fn repaint_blank_cell_backgrounds(
    content: Rect,
    row_y: u16,
    theme: &Theme,
    background: Color,
    buffer: &mut Buffer,
) {
    for column in content.x..content.right() {
        let Some(cell) = buffer.cell_mut((column, row_y)) else {
            continue;
        };
        if cell.symbol() == " " {
            cell.set_style(Style::default().fg(theme.text).bg(background));
        }
    }
}

#[cfg(test)]
mod changes_render_tests {
    use std::collections::BTreeSet;

    use gitdeck_caps::{CapsProfile, ThemeId};
    use gitdeck_core::{ChangeEntry, ChangeKind, ConflictKind};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;
    use ratatui::style::Style;

    use super::{
        box_glyphs, change_kind_color, change_kind_letter, changes_content_area, changes_group_of,
        changes_panel_layout, changes_rows, commit_label_color, commit_message_box_height,
        conflict_status_letters, default_collapsed_groups, error_body_line_count, file_count_label,
        panel_error_layout, render_changes_panel, render_panel_error, selection_background,
        sign_off_popover, visible_error_body_lines, visible_message_lines, wrap_error_body_lines,
        wrapped_message_lines, write_row_segments, BoxGlyphs, ChangesGroup, ChangesPanelView,
        ChangesRow, CheckState, CommitPanelView, PanelErrorSource, PanelErrorView, RowSegment,
        AMEND_LABEL, AMEND_ROW_INDENT, BRANCH_FAILED_LABEL, CHANGES_GROUPS,
        CHANGES_PANEL_CONTENT_TOP, CHECKOUT_FAILED_LABEL, COMMIT_AND_PUSH_ASCII_LABEL,
        COMMIT_AND_PUSH_LABEL, COMMIT_BUTTON_LABEL, COMMIT_BUTTON_ROW_BOTTOM_INSET,
        COMMIT_FAILED_LABEL, COMMIT_MESSAGE_BOX_MIN_HEIGHT, COMMIT_MESSAGE_HEIGHT_DIVISOR,
        COMMIT_MESSAGE_PLACEHOLDER, COMMIT_MESSAGE_TITLE, CONFLICT_FAILED_LABEL,
        DISCARD_FAILED_LABEL, FETCH_FAILED_LABEL, FILE_DIRECTORY_GAP, FILE_NAME_COLUMN,
        FILE_ROW_INDENT, FILE_STATUS_COLUMN, FILE_WORKTREE_STATUS_COLUMN, GROUP_TITLE_COLUMN,
        IGNORE_FAILED_LABEL, NO_CHANGES_LABEL, OPERATION_FAILED_LABEL, PANEL_ERROR_BODY_ROW_COUNT,
        PANEL_ERROR_OK_LABEL, PULL_FAILED_LABEL, PUSH_FAILED_LABEL, REFRESH_FAILED_LABEL,
        SIGN_OFF_LABEL, SIGN_OFF_POPOVER_TITLE,
    };
    use crate::glyphs::IconGlyph;
    use crate::next_step_hint::{
        LOCKED_INDEX_HINT, NEXT_STEP_HINT_ASCII_PREFIX, NEXT_STEP_HINT_PREFIX,
    };
    use crate::shell::text_display_width;
    use crate::theme::theme_palette;

    const PANEL_HEIGHT: u16 = 24;
    const TEST_ROW_WIDTH: u16 = 80;
    const NARROW_ROW_WIDTH: u16 = 34;
    const NARROWER_ROW_WIDTH: u16 = 27;
    const FILENAME_ONLY_ROW_WIDTH: u16 = 20;

    fn entry(path: &str, kind: ChangeKind) -> ChangeEntry {
        ChangeEntry::unstaged(path.to_string(), None, kind)
    }

    fn dirty_entries() -> Vec<ChangeEntry> {
        vec![
            entry("docs/readme.md", ChangeKind::Deleted),
            entry("src/app.rs", ChangeKind::Modified),
            entry("notes.txt", ChangeKind::Untracked),
        ]
    }

    fn checked_paths(entries: &[ChangeEntry]) -> BTreeSet<String> {
        entries.iter().map(|entry| entry.path.clone()).collect()
    }

    fn row_text(segments: &[RowSegment]) -> String {
        segments
            .iter()
            .map(|segment| match segment {
                RowSegment::Pad(width) => " ".repeat(usize::from(*width)),
                RowSegment::Text { text, .. } => text.clone(),
            })
            .collect()
    }

    fn render_rows(
        entries: &[ChangeEntry],
        rows: &[ChangesRow],
        selected: Option<usize>,
    ) -> Buffer {
        render_rows_with_commit(entries, rows, selected, idle_commit_view(""))
    }

    fn idle_commit_view(message: &str) -> CommitPanelView<'_> {
        CommitPanelView {
            message,
            editing: false,
            amend: false,
            sign_off: false,
            commit_enabled: false,
            options_visible: false,
            checked_file_count: 0,
            checked_untracked_file_count: 0,
        }
    }

    fn render_rows_with_commit(
        entries: &[ChangeEntry],
        rows: &[ChangesRow],
        selected: Option<usize>,
        commit: CommitPanelView<'_>,
    ) -> Buffer {
        let area = Rect::new(0, 0, 40, PANEL_HEIGHT);
        let mut buffer = Buffer::empty(area);
        render_changes_panel(
            area,
            ChangesPanelView {
                entries,
                rows,
                selected_row: selected,
                focused: true,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
                open_error: None,
                panel_error: None,
                commit,
            },
            &mut buffer,
        );
        buffer
    }

    const LONG_LOCKED_INDEX_MESSAGE: &str = "git add -A -- docs/readme.md notes.txt src/app.rs src/module.rs failed in /repo: fatal: Unable to create '/repo/.git/index.lock': File exists. Another git process seems to be running in this repository";

    const UNKNOWN_FAILURE_MESSAGE: &str = "git commit failed because the pre-commit hook refused the change and the reason stays unknown to the client for now";

    fn render_panel_error_block(caps: CapsProfile, message: &str) -> Buffer {
        let area = Rect::new(0, 0, 40, PANEL_HEIGHT);
        let mut buffer = Buffer::empty(area);
        let panel_error = PanelErrorView {
            scroll: 0,
            source: PanelErrorSource::Commit,
            message,
        };
        render_changes_panel(
            area,
            ChangesPanelView {
                entries: &[],
                rows: &[],
                selected_row: None,
                focused: true,
                caps,
                theme: ThemeId::JbDark,
                open_error: None,
                panel_error: Some(panel_error),
                commit: idle_commit_view(""),
            },
            &mut buffer,
        );
        let layout = changes_panel_layout(area, false).expect("layout");
        render_panel_error(
            layout.list,
            area,
            panel_error,
            caps,
            &theme_palette(ThemeId::JbDark),
            &mut buffer,
        );
        buffer
    }

    fn line_text(buffer: &Buffer, row: u16) -> String {
        let mut line = String::new();
        for column in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((column, row)) {
                line.push_str(cell.symbol());
            }
        }
        line.trim_end().to_string()
    }

    #[test]
    fn the_error_overlay_paints_an_opaque_background() {
        let area = Rect::new(0, 0, 40, PANEL_HEIGHT);
        let layout = changes_panel_layout(area, false).expect("layout");
        let theme = theme_palette(ThemeId::JbDark);
        let panel_error = PanelErrorView {
            scroll: 0,
            source: PanelErrorSource::Push,
            message: LONG_LOCKED_INDEX_MESSAGE,
        };
        let mut buffer = Buffer::empty(area);
        for row in area.y..area.bottom() {
            buffer.set_style(
                Rect::new(area.x, row, area.width, 1),
                Style::default().fg(theme.text).bg(theme.selection),
            );
            buffer.set_stringn(
                area.x,
                row,
                "27310226 JoeWu-tech 4:48 pm",
                usize::from(area.width),
                Style::default().fg(theme.text).bg(theme.selection),
            );
        }
        render_panel_error(
            layout.list,
            area,
            panel_error,
            CapsProfile::Rich,
            &theme,
            &mut buffer,
        );
        let block = panel_error_layout(layout.list, area)
            .expect("the error block is laid out")
            .block;
        for row in block.y..block.bottom() {
            for column in block.x..block.right() {
                let cell = buffer.cell((column, row)).expect("the block cell exists");
                assert_eq!(
                    cell.bg, theme.panel,
                    "row {row} column {column} keeps the overlay background"
                );
            }
            let block_row = line_text(&buffer, row);
            assert!(
                !block_row.contains("JoeWu-tech"),
                "row {row} hides the content below the overlay, row {block_row:?}"
            );
        }
    }

    #[test]
    fn tracked_and_untracked_entries_land_in_their_groups() {
        assert_eq!(
            changes_group_of(&entry("src/app.rs", ChangeKind::Modified)),
            ChangesGroup::Changes
        );
        assert_eq!(
            changes_group_of(&ChangeEntry::from_status_codes(
                String::from("src/conflict.rs"),
                None,
                None,
                None,
                Some(ConflictKind::BothModified),
            )),
            ChangesGroup::MergeConflicts
        );
        assert_eq!(
            changes_group_of(&entry("notes.txt", ChangeKind::Untracked)),
            ChangesGroup::Unversioned
        );
        let rows = changes_rows(
            &dirty_entries(),
            &default_collapsed_groups(),
            &checked_paths(&dirty_entries()),
        );
        assert_eq!(
            rows,
            vec![
                ChangesRow::Group {
                    group: ChangesGroup::Changes,
                    collapsed: false,
                    check: CheckState::Checked,
                    count: 2,
                },
                ChangesRow::File {
                    group: ChangesGroup::Changes,
                    index: 0,
                    checked: true,
                },
                ChangesRow::File {
                    group: ChangesGroup::Changes,
                    index: 1,
                    checked: true,
                },
                ChangesRow::Group {
                    group: ChangesGroup::Unversioned,
                    collapsed: false,
                    check: CheckState::Checked,
                    count: 1,
                },
                ChangesRow::File {
                    group: ChangesGroup::Unversioned,
                    index: 2,
                    checked: true,
                },
            ]
        );
    }

    #[test]
    fn collapsed_groups_hide_their_children() {
        let entries = dirty_entries();
        let collapsed = BTreeSet::from([ChangesGroup::Changes]);
        let rows = changes_rows(&entries, &collapsed, &checked_paths(&entries));
        assert_eq!(
            rows,
            vec![
                ChangesRow::Group {
                    group: ChangesGroup::Changes,
                    collapsed: true,
                    check: CheckState::Checked,
                    count: 2,
                },
                ChangesRow::Group {
                    group: ChangesGroup::Unversioned,
                    collapsed: false,
                    check: CheckState::Checked,
                    count: 1,
                },
                ChangesRow::File {
                    group: ChangesGroup::Unversioned,
                    index: 2,
                    checked: true,
                },
            ]
        );
    }

    #[test]
    fn check_state_reports_checked_unchecked_and_partial() {
        assert_eq!(CheckState::from_counts(0, 0), CheckState::Unchecked);
        assert_eq!(CheckState::from_counts(0, 3), CheckState::Unchecked);
        assert_eq!(CheckState::from_counts(3, 3), CheckState::Checked);
        assert_eq!(CheckState::from_counts(1, 3), CheckState::Partial);
        assert_eq!(CheckState::Checked.glyph(), 'x');
        assert_eq!(CheckState::Unchecked.glyph(), ' ');
        assert_eq!(CheckState::Partial.glyph(), '-');
        assert_eq!(file_count_label(1), "1 file");
        assert_eq!(file_count_label(2), "2 files");
    }

    #[test]
    fn status_letter_and_color_follow_change_kind() {
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(change_kind_letter(ChangeKind::Modified), 'M');
        assert_eq!(change_kind_letter(ChangeKind::TypeChanged), 'T');
        assert_eq!(change_kind_letter(ChangeKind::Added), 'A');
        assert_eq!(change_kind_letter(ChangeKind::Deleted), 'D');
        assert_eq!(change_kind_letter(ChangeKind::Renamed), 'R');
        assert_eq!(change_kind_letter(ChangeKind::Untracked), '?');
        assert_eq!(change_kind_letter(ChangeKind::Conflicted), '!');
        assert_eq!(
            change_kind_color(ChangeKind::Modified, &theme),
            theme.warning
        );
        assert_eq!(
            change_kind_color(ChangeKind::TypeChanged, &theme),
            theme.warning
        );
        assert_eq!(change_kind_color(ChangeKind::Added, &theme), theme.success);
        assert_eq!(
            change_kind_color(ChangeKind::Renamed, &theme),
            theme.success
        );
        assert_eq!(change_kind_color(ChangeKind::Deleted, &theme), theme.error);
        assert_eq!(
            change_kind_color(ChangeKind::Conflicted, &theme),
            theme.error
        );
        assert_eq!(
            change_kind_color(ChangeKind::Untracked, &theme),
            theme.muted
        );
    }

    #[test]
    fn selection_background_follows_focus_and_selection() {
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(
            selection_background(&theme, true, true, theme.panel),
            theme.selection
        );
        assert_eq!(
            selection_background(&theme, false, true, theme.panel),
            theme.border
        );
        assert_eq!(
            selection_background(&theme, true, false, theme.panel),
            theme.panel
        );
        assert_eq!(
            selection_background(&theme, true, false, theme.background),
            theme.background
        );
    }

    #[test]
    fn group_rows_keep_the_documented_column_offsets() {
        let theme = theme_palette(ThemeId::JbDark);
        let expanded = super::group_row_segments(
            ChangesGroup::Changes,
            false,
            CheckState::Partial,
            3,
            CapsProfile::Rich,
            &theme,
        );
        assert_eq!(row_text(&expanded), "▾ [-] Changes 3 files");
        let collapsed = super::group_row_segments(
            ChangesGroup::Unversioned,
            true,
            CheckState::Unchecked,
            1,
            CapsProfile::Rich,
            &theme,
        );
        assert_eq!(row_text(&collapsed), "▸ [ ] Unversioned Files 1 file");
        assert_eq!(GROUP_TITLE_COLUMN, 6);
    }

    #[test]
    fn file_rows_indent_two_columns_and_split_the_directory() {
        let theme = theme_palette(ThemeId::JbDark);
        let segments = super::file_row_segments(
            &entry("docs/readme.md", ChangeKind::Deleted),
            true,
            TEST_ROW_WIDTH,
            CapsProfile::Rich,
            &theme,
        );
        assert_eq!(row_text(&segments), "  [x]  D readme.md  docs");
        let without_directory = super::file_row_segments(
            &entry("notes.txt", ChangeKind::Untracked),
            false,
            TEST_ROW_WIDTH,
            CapsProfile::Rich,
            &theme,
        );
        assert_eq!(row_text(&without_directory), "  [ ]  ? notes.txt");
        assert_eq!(FILE_ROW_INDENT, 2);
        assert_eq!(FILE_STATUS_COLUMN, 6);
        assert_eq!(FILE_WORKTREE_STATUS_COLUMN, 7);
        assert_eq!(FILE_NAME_COLUMN, 9);
        assert_eq!(FILE_DIRECTORY_GAP, 2);
    }

    fn status_pair(
        index_kind: Option<ChangeKind>,
        worktree_kind: Option<ChangeKind>,
    ) -> ChangeEntry {
        ChangeEntry::from_status_codes(
            String::from("src/app.rs"),
            None,
            index_kind,
            worktree_kind,
            None,
        )
    }

    #[test]
    fn long_paths_are_ellipsized_in_the_middle() {
        let theme = theme_palette(ThemeId::JbDark);
        let long_entry = entry("internal/library/note_extra.go", ChangeKind::Modified);
        let segments = super::file_row_segments(
            &long_entry,
            true,
            NARROW_ROW_WIDTH,
            CapsProfile::Rich,
            &theme,
        );
        let rendered = row_text(&segments);
        assert_eq!(rendered, "  [x]  M note_extra.go  …/library");
        assert!(
            text_display_width(&rendered) <= NARROW_ROW_WIDTH,
            "{rendered:?} stays inside a {NARROW_ROW_WIDTH} column row"
        );
        let narrower = super::file_row_segments(
            &long_entry,
            true,
            NARROWER_ROW_WIDTH,
            CapsProfile::Rich,
            &theme,
        );
        assert_eq!(row_text(&narrower), "  [x]  M note_extra.go  …ry");
        let filename_only = super::file_row_segments(
            &long_entry,
            true,
            FILENAME_ONLY_ROW_WIDTH,
            CapsProfile::Rich,
            &theme,
        );
        assert_eq!(row_text(&filename_only), "  [x]  M …e_extra.go");
    }

    #[test]
    fn basic_caps_uses_ascii_ellipsis_for_paths() {
        let theme = theme_palette(ThemeId::JbDark);
        let long_entry = entry("internal/library/note_extra.go", ChangeKind::Modified);
        let segments = super::file_row_segments(
            &long_entry,
            true,
            NARROW_ROW_WIDTH,
            CapsProfile::Basic,
            &theme,
        );
        let rendered = row_text(&segments);
        assert_eq!(rendered, "  [x]  M note_extra.go  ~/library");
        assert!(rendered.is_ascii(), "{rendered:?} must stay ascii");
    }

    #[test]
    fn the_two_status_letters_keep_a_fixed_alignment() {
        let theme = theme_palette(ThemeId::JbDark);
        let cases = [
            (
                status_pair(None, Some(ChangeKind::Modified)),
                [' ', 'M', 'a'],
                "src/app.rs",
            ),
            (
                status_pair(Some(ChangeKind::Added), None),
                ['A', ' ', 'a'],
                "src/app.rs",
            ),
            (
                entry("notes.txt", ChangeKind::Untracked),
                [' ', '?', 'n'],
                "notes.txt",
            ),
            (
                status_pair(Some(ChangeKind::Modified), Some(ChangeKind::Modified)),
                ['M', 'M', 'a'],
                "src/app.rs",
            ),
        ];
        for (file_entry, letters, file_name) in cases {
            let segments = super::file_row_segments(
                &file_entry,
                false,
                TEST_ROW_WIDTH,
                CapsProfile::Rich,
                &theme,
            );
            let rendered = row_text(&segments);
            let cells: Vec<char> = rendered.chars().collect();
            assert_eq!(
                cells[usize::from(FILE_STATUS_COLUMN)],
                letters[0],
                "index letter of {rendered:?}"
            );
            assert_eq!(
                cells[usize::from(FILE_WORKTREE_STATUS_COLUMN)],
                letters[1],
                "worktree letter of {rendered:?}"
            );
            assert_eq!(
                cells[usize::from(FILE_NAME_COLUMN) - 1],
                ' ',
                "one gap column before the file name in {rendered:?}"
            );
            assert_eq!(
                cells[usize::from(FILE_NAME_COLUMN)],
                letters[2],
                "the file name starts at the file name column in {rendered:?}"
            );
            let file_base_name = file_name.rsplit('/').next().unwrap_or(file_name);
            assert!(
                rendered.contains(file_base_name),
                "{rendered:?} keeps the file name {file_base_name:?}"
            );
        }
    }

    fn conflicted(path: &str, conflict: ConflictKind) -> ChangeEntry {
        ChangeEntry::from_status_codes(path.to_string(), None, None, None, Some(conflict))
    }

    fn group_rows_of(entries: &[ChangeEntry]) -> Vec<ChangesRow> {
        changes_rows(
            entries,
            &default_collapsed_groups(),
            &checked_paths(entries),
        )
    }

    #[test]
    fn status_column_shows_both_index_and_worktree_letters() {
        let theme = theme_palette(ThemeId::JbDark);
        let cases = [
            (
                status_pair(Some(ChangeKind::Modified), Some(ChangeKind::Modified)),
                "  [x] MM app.rs  src",
            ),
            (
                status_pair(Some(ChangeKind::Modified), None),
                "  [x] M  app.rs  src",
            ),
            (
                status_pair(None, Some(ChangeKind::Modified)),
                "  [x]  M app.rs  src",
            ),
            (
                status_pair(None, Some(ChangeKind::Deleted)),
                "  [x]  D app.rs  src",
            ),
        ];
        for (entry, expected) in cases {
            let segments =
                super::file_row_segments(&entry, true, TEST_ROW_WIDTH, CapsProfile::Rich, &theme);
            assert_eq!(row_text(&segments), expected);
        }
    }

    #[test]
    fn type_changed_renders_its_own_letter() {
        let theme = theme_palette(ThemeId::JbDark);
        let entries = vec![status_pair(None, Some(ChangeKind::TypeChanged))];
        let rows = group_rows_of(&entries);
        let buffer = render_rows(&entries, &rows, Some(1));
        let status_cell = buffer
            .cell((FILE_WORKTREE_STATUS_COLUMN, CHANGES_PANEL_CONTENT_TOP + 1))
            .expect("status cell");
        assert_eq!(status_cell.symbol(), "T");
        assert_eq!(status_cell.fg, theme.warning);
        assert_eq!(
            row_text(&super::file_row_segments(
                &entries[0],
                true,
                TEST_ROW_WIDTH,
                CapsProfile::Rich,
                &theme
            )),
            "  [x]  T app.rs  src"
        );
    }

    #[test]
    fn conflicts_get_their_own_group_at_the_top() {
        let entries = vec![
            conflicted("src/merged.rs", ConflictKind::BothModified),
            conflicted("src/added.rs", ConflictKind::BothAdded),
            entry("src/app.rs", ChangeKind::Modified),
        ];
        let rows = group_rows_of(&entries);
        assert_eq!(
            rows,
            vec![
                ChangesRow::Group {
                    group: ChangesGroup::MergeConflicts,
                    collapsed: false,
                    check: CheckState::Checked,
                    count: 2,
                },
                ChangesRow::File {
                    group: ChangesGroup::MergeConflicts,
                    index: 0,
                    checked: true,
                },
                ChangesRow::File {
                    group: ChangesGroup::MergeConflicts,
                    index: 1,
                    checked: true,
                },
                ChangesRow::Group {
                    group: ChangesGroup::Changes,
                    collapsed: false,
                    check: CheckState::Checked,
                    count: 1,
                },
                ChangesRow::File {
                    group: ChangesGroup::Changes,
                    index: 2,
                    checked: true,
                },
            ]
        );
        assert_eq!(ChangesGroup::MergeConflicts.title(), "Merge Conflicts");
        assert_eq!(
            CHANGES_GROUPS,
            [
                ChangesGroup::MergeConflicts,
                ChangesGroup::Changes,
                ChangesGroup::Unversioned,
            ]
        );
    }

    #[test]
    fn conflict_rows_show_the_two_character_code() {
        let theme = theme_palette(ThemeId::JbDark);
        let cases = [
            (ConflictKind::BothDeleted, "DD"),
            (ConflictKind::AddedByUs, "AU"),
            (ConflictKind::DeletedByThem, "UD"),
            (ConflictKind::AddedByThem, "UA"),
            (ConflictKind::DeletedByUs, "DU"),
            (ConflictKind::BothAdded, "AA"),
            (ConflictKind::BothModified, "UU"),
        ];
        let entries: Vec<ChangeEntry> = cases
            .iter()
            .map(|(conflict, _)| conflicted("src/conflict.rs", *conflict))
            .collect();
        let rows = group_rows_of(&entries);
        let buffer = render_rows(&entries, &rows, Some(1));
        for (row, (conflict, code)) in cases.iter().enumerate() {
            let segments = super::file_row_segments(
                &entries[row],
                true,
                TEST_ROW_WIDTH,
                CapsProfile::Rich,
                &theme,
            );
            assert_eq!(
                row_text(&segments),
                format!("  [x] {code} conflict.rs  src")
            );
            let row_y = CHANGES_PANEL_CONTENT_TOP + 1 + row as u16;
            let index_cell = buffer
                .cell((FILE_STATUS_COLUMN, row_y))
                .expect("index status cell");
            let worktree_cell = buffer
                .cell((FILE_WORKTREE_STATUS_COLUMN, row_y))
                .expect("worktree status cell");
            assert_eq!(index_cell.symbol(), &code[..1]);
            assert_eq!(worktree_cell.symbol(), &code[1..]);
            assert_eq!(index_cell.fg, theme.error);
            assert_eq!(worktree_cell.fg, theme.error);
            assert_eq!(
                conflict_status_letters(*conflict),
                [code.as_bytes()[0] as char, code.as_bytes()[1] as char]
            );
        }
    }

    #[test]
    fn unversioned_group_still_renders_when_no_conflicts() {
        let entries = dirty_entries();
        let rows = group_rows_of(&entries);
        let group_titles: Vec<&str> = rows
            .iter()
            .filter_map(|row| match row {
                ChangesRow::Group { group, .. } => Some(group.title()),
                ChangesRow::File { .. } => None,
            })
            .collect();
        assert_eq!(group_titles, vec!["Changes", "Unversioned Files"]);
        let buffer = render_rows(&entries, &rows, None);
        assert_eq!(
            line_text(&buffer, CHANGES_PANEL_CONTENT_TOP),
            "▾ [x] Changes 2 files"
        );
    }

    #[test]
    fn rendered_rows_start_below_the_tab_and_toolbar_rows() {
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let buffer = render_rows(&entries, &rows, Some(0));
        assert_eq!(
            line_text(&buffer, CHANGES_PANEL_CONTENT_TOP),
            "▾ [x] Changes 2 files"
        );
        assert_eq!(
            line_text(&buffer, CHANGES_PANEL_CONTENT_TOP + 1),
            "  [x]  D readme.md  docs"
        );
        assert_eq!(
            line_text(&buffer, CHANGES_PANEL_CONTENT_TOP + 2),
            "  [x]  M app.rs  src"
        );
        assert_eq!(
            line_text(&buffer, CHANGES_PANEL_CONTENT_TOP + 3),
            "▾ [x] Unversioned Files 1 file"
        );
    }

    #[test]
    fn empty_changes_render_the_placeholder_on_the_first_content_row() {
        let buffer = render_rows(&[], &[], None);
        assert_eq!(
            line_text(&buffer, CHANGES_PANEL_CONTENT_TOP),
            NO_CHANGES_LABEL
        );
        assert_eq!(line_text(&buffer, CHANGES_PANEL_CONTENT_TOP + 1), "");
        assert_eq!(changes_content_area(Rect::new(0, 0, 10, 2)), None);
    }

    #[test]
    fn selected_row_paints_the_selection_background_across_the_pane() {
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let buffer = render_rows(&entries, &rows, Some(1));
        let theme = theme_palette(ThemeId::JbDark);
        for column in 0..40 {
            let cell = buffer
                .cell((column, CHANGES_PANEL_CONTENT_TOP + 1))
                .expect("cell");
            assert_eq!(cell.bg, theme.selection, "column {column}");
        }
        let unselected = buffer.cell((0, CHANGES_PANEL_CONTENT_TOP)).expect("cell");
        assert_eq!(unselected.bg, theme.panel);
    }

    #[test]
    fn wide_glyphs_keep_the_selection_background_on_their_continuation_cell() {
        let entries = vec![entry("文档/说明.md", ChangeKind::Modified)];
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let buffer = render_rows(&entries, &rows, Some(1));
        let theme = theme_palette(ThemeId::JbDark);
        let wide_glyph = buffer
            .cell((FILE_NAME_COLUMN, CHANGES_PANEL_CONTENT_TOP + 1))
            .expect("cell");
        assert_eq!(wide_glyph.symbol(), "说");
        let following = buffer
            .cell((FILE_NAME_COLUMN + 1, CHANGES_PANEL_CONTENT_TOP + 1))
            .expect("cell");
        assert_eq!(following.bg, theme.selection);
        assert_eq!(following.symbol(), " ");
    }

    #[test]
    fn segments_are_truncated_at_the_pane_right_edge() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 1));
        let segments = vec![
            RowSegment::Pad(2),
            RowSegment::Text {
                text: String::from("abcdefgh"),
                color: Color::Reset,
            },
        ];
        write_row_segments(
            Rect::new(0, 0, 6, 1),
            0,
            &segments,
            Color::Reset,
            CapsProfile::Rich,
            &mut buffer,
        );
        assert_eq!(line_text(&buffer, 0), "  abcd");
    }

    #[test]
    fn panel_layout_stacks_the_commit_block_below_the_list() {
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        assert_eq!(layout.list, Rect::new(0, 2, 40, 13));
        assert_eq!(layout.amend_row, Rect::new(0, 15, 40, 1));
        assert_eq!(layout.message_box, Rect::new(0, 16, 40, 6));
        assert_eq!(layout.message_lines, Rect::new(2, 17, 36, 4));
        assert_eq!(layout.button_row, Rect::new(0, 22, 40, 1));
        assert_eq!(layout.commit_button, Rect::new(0, 22, 10, 1));
        assert_eq!(
            layout.push_button,
            Rect::new(12, 22, text_display_width(COMMIT_AND_PUSH_ASCII_LABEL), 1)
        );
        assert_eq!(layout.options_button, Rect::new(38, 22, 1, 1));
        assert_eq!(
            layout.button_row.bottom() + COMMIT_BUTTON_ROW_BOTTOM_INSET,
            PANEL_HEIGHT,
            "the shell rule keeps the last pane row"
        );
        assert_eq!(changes_panel_layout(Rect::new(0, 0, 0, 0), false), None);
    }

    #[test]
    fn message_box_height_follows_the_panel_quarter_without_a_cap() {
        assert_eq!(
            commit_message_box_height(PANEL_HEIGHT),
            PANEL_HEIGHT / COMMIT_MESSAGE_HEIGHT_DIVISOR
        );
        assert_eq!(commit_message_box_height(64), 16);
        assert_eq!(commit_message_box_height(70), 17);
        assert_eq!(commit_message_box_height(12), COMMIT_MESSAGE_BOX_MIN_HEIGHT);
        assert_eq!(commit_message_box_height(8), COMMIT_MESSAGE_BOX_MIN_HEIGHT);
        assert_eq!(commit_message_box_height(6), 2);
        assert_eq!(commit_message_box_height(4), 1);
    }

    #[test]
    fn short_panel_keeps_three_message_rows_and_the_button_row() {
        let layout = changes_panel_layout(Rect::new(0, 0, 40, 15), false).expect("layout");
        assert_eq!(
            layout.message_box.height, COMMIT_MESSAGE_BOX_MIN_HEIGHT,
            "a short panel never drops the message box below three rows"
        );
        assert_eq!(layout.message_lines, Rect::new(2, 11, 36, 1));
        assert_eq!(layout.message_box, Rect::new(0, 10, 40, 3));
        assert_eq!(layout.amend_row, Rect::new(0, 9, 40, 1));
        assert_eq!(layout.list, Rect::new(0, 2, 40, 7));
        assert_eq!(layout.button_row, Rect::new(0, 13, 40, 1));
    }

    #[test]
    fn amend_row_renders_the_checkbox_label_and_the_expand_chevron() {
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let buffer = render_rows(&entries, &rows, Some(0));
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let chevron = IconGlyph::Chevron.character(CapsProfile::Rich);
        assert_eq!(
            line_text(&buffer, layout.amend_row.y),
            format!("  [ ] {AMEND_LABEL} {chevron}")
        );
        assert_eq!(AMEND_ROW_INDENT, 2);
        let checked = render_rows_with_commit(
            &entries,
            &rows,
            Some(0),
            CommitPanelView {
                amend: true,
                ..idle_commit_view("")
            },
        );
        assert_eq!(
            line_text(&checked, layout.amend_row.y),
            format!("  [x] {AMEND_LABEL} {chevron}")
        );
    }

    #[test]
    fn commit_message_box_renders_its_title_and_the_placeholder() {
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let buffer = render_rows(&entries, &rows, Some(0));
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        assert_eq!(
            line_text(&buffer, layout.message_box.y),
            format!("┌ {COMMIT_MESSAGE_TITLE} {}0 chars─┐", "─".repeat(14))
        );
        assert_eq!(
            line_text(&buffer, layout.message_lines.y),
            format!("│ {COMMIT_MESSAGE_PLACEHOLDER}{}│", " ".repeat(23))
        );
        assert_eq!(
            line_text(&buffer, layout.message_box.bottom() - 1),
            format!("└{}┘", "─".repeat(38))
        );
    }

    #[test]
    fn editing_message_renders_the_lines_and_a_reversed_caret() {
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let text = "fix conflict\nsecond line";
        let buffer = render_rows_with_commit(
            &entries,
            &rows,
            Some(0),
            CommitPanelView {
                editing: true,
                commit_enabled: true,
                ..idle_commit_view(text)
            },
        );
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(
            line_text(&buffer, layout.message_lines.y),
            format!("│ {}{}│", "fix conflict", " ".repeat(25))
        );
        let caret = buffer
            .cell((2 + 11, layout.message_lines.y + 1))
            .expect("caret cell");
        assert_eq!(caret.symbol(), " ");
        assert_eq!(caret.bg, theme.text);
        assert_eq!(caret.fg, theme.panel);
        assert_eq!(
            line_text(&buffer, layout.message_lines.y + 1),
            format!("│ second line{}│", " ".repeat(26))
        );
    }

    #[test]
    fn visible_message_lines_keep_the_caret_line_inside_the_window() {
        assert_eq!(visible_message_lines("", 36, 3), vec![String::new()]);
        assert_eq!(
            visible_message_lines("one\ntwo", 36, 3),
            vec![String::from("one"), String::from("two")]
        );
        assert_eq!(
            visible_message_lines("one\ntwo\nthree\nfour", 36, 3),
            vec![
                String::from("two"),
                String::from("three"),
                String::from("four")
            ]
        );
        assert!(visible_message_lines("one", 36, 0).is_empty());
    }

    #[test]
    fn wrapped_message_lines_keep_every_character_and_respect_the_width() {
        let message = "the quick brown fox jumps over the lazy dog again and again";
        let lines = wrapped_message_lines(message, 12);
        assert!(lines.len() > 1, "the message wraps into {lines:?}");
        assert_eq!(lines.concat(), message);
        for line in &lines {
            assert!(
                text_display_width(line) <= 12,
                "line {line:?} stays inside the width"
            );
        }
    }

    #[test]
    fn wrapped_message_lines_never_split_a_wide_character_and_terminate() {
        let message = "修复提交消息编辑器里的折行显示问题";
        let lines = wrapped_message_lines(message, 5);
        assert_eq!(lines.concat(), message);
        for line in &lines {
            assert!(
                text_display_width(line) <= 5,
                "line {line:?} keeps whole wide characters"
            );
        }
        assert_eq!(
            wrapped_message_lines(message, 0),
            vec![message.to_string()],
            "a zero width never loops"
        );
        assert_eq!(
            wrapped_message_lines("修", 1),
            vec![String::from("修")],
            "a single wide character wider than the row still terminates"
        );
        let long_word = "x".repeat(40);
        let long_word_lines = wrapped_message_lines(&long_word, 7);
        assert_eq!(long_word_lines.concat(), long_word);
        assert_eq!(
            long_word_lines,
            vec![String::from("xxxxxxx"); 5]
                .into_iter()
                .chain(vec![String::from("xxxxx")])
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_full_last_line_pushes_the_caret_onto_the_next_line() {
        assert_eq!(
            visible_message_lines("abcdef", 6, 3),
            vec![String::from("abcdef"), String::new()]
        );
        assert_eq!(
            visible_message_lines("abcde", 6, 3),
            vec![String::from("abcde")]
        );
    }

    #[test]
    fn commit_button_row_colors_the_labels_from_the_enabled_state() {
        let theme = theme_palette(ThemeId::JbDark);
        assert_eq!(commit_label_color(true, &theme), theme.accent);
        assert_eq!(commit_label_color(false, &theme), theme.muted);
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let buffer = render_rows_with_commit(
            &entries,
            &rows,
            Some(0),
            CommitPanelView {
                commit_enabled: true,
                ..idle_commit_view("a message")
            },
        );
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let gear = IconGlyph::Gear.character(CapsProfile::Rich);
        assert_eq!(
            line_text(&buffer, layout.button_row.y),
            format!(
                "{COMMIT_BUTTON_LABEL}  {COMMIT_AND_PUSH_LABEL}{}{gear}",
                " ".repeat(40 - 34)
            )
        );
        assert_eq!(
            buffer.cell((0, layout.button_row.y)).expect("cell").fg,
            theme.accent
        );
        let disabled = render_rows(&entries, &rows, Some(0));
        assert_eq!(
            disabled.cell((0, layout.button_row.y)).expect("cell").fg,
            theme.muted
        );
    }

    #[test]
    fn sign_off_popover_sits_above_the_button_row() {
        let button_row = Rect::new(0, 20, 40, 1);
        let (frame, sign_off_row) = sign_off_popover(button_row).expect("popover");
        assert_eq!(frame, Rect::new(18, 17, 22, 3));
        assert_eq!(sign_off_row, Rect::new(18, 18, 22, 1));
        assert_eq!(sign_off_popover(Rect::new(0, 2, 40, 1)), None);
    }

    #[test]
    fn sign_off_popover_renders_its_title_and_the_checkbox_state() {
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let (frame, sign_off_row) = sign_off_popover(layout.button_row).expect("popover");
        let hidden = render_rows_with_commit(
            &entries,
            &rows,
            Some(0),
            CommitPanelView {
                sign_off: true,
                ..idle_commit_view("")
            },
        );
        assert_ne!(
            line_text(&hidden, frame.y),
            format!("┌ {} ─{}┐", SIGN_OFF_POPOVER_TITLE, "─".repeat(16))
        );
        let shown = render_rows_with_commit(
            &entries,
            &rows,
            Some(0),
            CommitPanelView {
                sign_off: true,
                options_visible: true,
                ..idle_commit_view("")
            },
        );
        let title_line = line_text(&shown, frame.y);
        assert!(
            title_line.contains(SIGN_OFF_POPOVER_TITLE),
            "the popover title row is {title_line:?}"
        );
        assert!(
            line_text(&shown, sign_off_row.y).contains(format!("│ [x] {SIGN_OFF_LABEL}│").as_str()),
            "the sign-off row is {:?}",
            line_text(&shown, sign_off_row.y)
        );
    }

    #[test]
    fn a_panel_error_renders_a_bordered_block_with_its_title_and_ok_row() {
        let area = Rect::new(0, 0, 40, PANEL_HEIGHT);
        let mut buffer = Buffer::empty(area);
        let panel_error = PanelErrorView {
            scroll: 0,
            source: PanelErrorSource::Push,
            message: "git push failed because the configured remote cannot be reached from this network right now please retry later",
        };
        render_changes_panel(
            area,
            ChangesPanelView {
                entries: &[],
                rows: &[],
                selected_row: None,
                focused: true,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
                open_error: None,
                panel_error: Some(panel_error),
                commit: idle_commit_view(""),
            },
            &mut buffer,
        );
        let layout = changes_panel_layout(area, false).expect("layout");
        render_panel_error(
            layout.list,
            area,
            panel_error,
            CapsProfile::Rich,
            &theme_palette(ThemeId::JbDark),
            &mut buffer,
        );
        let panel_error_layout =
            panel_error_layout(layout.list, area).expect("the error block is laid out");
        let block = panel_error_layout.block;
        let theme = theme_palette(ThemeId::JbDark);
        let title_row = line_text(&buffer, block.y);
        assert!(
            title_row.starts_with('┌') && title_row.ends_with('┐'),
            "the title row is bordered, row {title_row:?}"
        );
        assert!(
            title_row.contains(PUSH_FAILED_LABEL),
            "the title row names the source, row {title_row:?}"
        );
        assert_eq!(buffer.cell((1, block.y)).expect("cell").fg, theme.error);
        let bottom_row = line_text(&buffer, block.bottom() - 1);
        assert!(
            bottom_row.starts_with('└') && bottom_row.ends_with('┘'),
            "the block is closed at the bottom, row {bottom_row:?}"
        );
        for row in 0..PANEL_ERROR_BODY_ROW_COUNT {
            assert!(
                !line_text(&buffer, block.y + 1 + row).is_empty(),
                "body row {row} carries a wrapped line"
            );
        }
        let ok_button = panel_error_layout.ok_button;
        assert!(
            line_text(&buffer, ok_button.y).contains(format!("│ {PANEL_ERROR_OK_LABEL}").as_str()),
            "the ok row is {:?}",
            line_text(&buffer, ok_button.y)
        );
        assert_eq!(PanelErrorSource::Refresh.title(), REFRESH_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Push.title(), PUSH_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Pull.title(), PULL_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Fetch.title(), FETCH_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Commit.title(), COMMIT_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Checkout.title(), CHECKOUT_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Conflict.title(), CONFLICT_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Discard.title(), DISCARD_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Operation.title(), OPERATION_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Branch.title(), BRANCH_FAILED_LABEL);
        assert_eq!(PanelErrorSource::Ignore.title(), IGNORE_FAILED_LABEL);
    }

    #[test]
    fn a_basic_panel_error_block_uses_ascii_borders_and_overflow_marker() {
        let buffer = render_panel_error_block(CapsProfile::Basic, LONG_LOCKED_INDEX_MESSAGE);
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let block = panel_error_layout(layout.list, Rect::new(0, 0, 40, PANEL_HEIGHT))
            .expect("the error block is laid out")
            .block;
        let block_text: String = (block.y..block.bottom())
            .map(|row| line_text(&buffer, row))
            .collect();
        assert!(block_text.contains('+'), "the block uses ascii corners");
        assert!(
            block_text.contains("(+"),
            "the wrapped reason reports the hidden line count, block text {block_text:?}"
        );
    }

    #[test]
    fn basic_caps_error_block_is_ascii_only() {
        let buffer = render_panel_error_block(CapsProfile::Basic, UNKNOWN_FAILURE_MESSAGE);
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let block = panel_error_layout(layout.list, Rect::new(0, 0, 40, PANEL_HEIGHT))
            .expect("the error block is laid out")
            .block;
        let block_text: String = (block.y..block.bottom())
            .map(|row| line_text(&buffer, row))
            .collect();
        assert!(
            block_text.is_ascii(),
            "the basic block stays ascii without a hint, block text {block_text:?}"
        );
        let hinted = render_panel_error_block(CapsProfile::Basic, LONG_LOCKED_INDEX_MESSAGE);
        let hinted_layout = panel_error_layout(layout.list, Rect::new(0, 0, 40, PANEL_HEIGHT))
            .expect("the error block is laid out");
        let hint_row = line_text(&hinted, hinted_layout.body.bottom().saturating_sub(1));
        assert!(
            hint_row.contains(NEXT_STEP_HINT_ASCII_PREFIX),
            "the basic hint uses the ascii prefix, hint row {hint_row:?}"
        );
        assert!(
            !hint_row.contains(NEXT_STEP_HINT_PREFIX),
            "the basic hint never draws the unicode arrow, hint row {hint_row:?}"
        );
        let chrome_rows: String = (block.y..block.bottom())
            .filter(|row| *row != hinted_layout.body.bottom().saturating_sub(1))
            .map(|row| line_text(&hinted, row))
            .collect();
        assert!(
            chrome_rows.is_ascii(),
            "the basic block chrome stays ascii, rows {chrome_rows:?}"
        );
    }

    #[test]
    fn long_messages_wrap_on_word_boundaries() {
        let message = "git commit failed in /repo because the pre-commit hook refused the change and printed a long explanation";
        let lines = wrap_error_body_lines(message, 20);
        assert!(lines.len() > 1, "the message wraps, lines {lines:?}");
        for line in &lines {
            assert!(
                text_display_width(line) <= 20,
                "line {line:?} overflows the 20 column body"
            );
        }
        let rejoined = lines.join(" ");
        assert_eq!(
            rejoined.split_whitespace().collect::<Vec<_>>(),
            message.split_whitespace().collect::<Vec<_>>(),
            "wrapping keeps every word in order"
        );
        for line in &lines {
            for word in line.split_whitespace() {
                assert!(
                    message.split_whitespace().any(|source| source == word),
                    "line {line:?} splits the word {word:?}"
                );
            }
        }
    }

    #[test]
    fn cjk_messages_wrap_without_splitting_cells() {
        let message = "提交失败：另一个 git 进程正在运行，稍后重试另一个 git 进程正在运行";
        let lines = wrap_error_body_lines(message, 12);
        assert!(lines.len() > 1, "the cjk message wraps, lines {lines:?}");
        for line in &lines {
            assert!(
                text_display_width(line) <= 12,
                "line {line:?} overflows the 12 column body"
            );
        }
        let rejoined: String = lines.concat().replace(' ', "");
        assert_eq!(
            rejoined,
            message.replace(' ', ""),
            "wrapping keeps every wide character"
        );
        let single_wide_columns = wrap_error_body_lines("中文", 1);
        assert_eq!(
            single_wide_columns.len(),
            2,
            "a one column body holds one wide character per row, lines {single_wide_columns:?}"
        );
    }

    #[test]
    fn wrapping_terminates_when_a_character_is_wider_than_the_line() {
        let lines = wrap_error_body_lines("中文", 1);
        assert_eq!(lines, vec![String::from("中"), String::from("文")]);
        let empty = wrap_error_body_lines("中文", 0);
        assert!(empty.is_empty(), "a zero width body holds no rows");
    }

    #[test]
    fn truncated_bodies_report_the_remaining_line_count() {
        let words: Vec<String> = (0..12).map(|index| format!("line{index:0>14}")).collect();
        let message = words.join(" ");
        let lines = wrap_error_body_lines(&message, 20);
        assert_eq!(lines.len(), 12, "each word keeps its own row, {lines:?}");
        assert_eq!(error_body_line_count(&message, 20), 12);
        let visible = visible_error_body_lines(&lines, 0, 3, 20);
        assert_eq!(visible.len(), 3, "the body shows three rows");
        assert_eq!(visible[0], words[0]);
        assert_eq!(visible[1], words[1]);
        assert!(
            visible[2].contains("(+10 more lines)"),
            "the last row counts the hidden lines, row {:?}",
            visible[2]
        );
        assert!(
            text_display_width(&visible[2]) <= 20,
            "the marker fits the body width, row {:?}",
            visible[2]
        );
        let scrolled = visible_error_body_lines(&lines, 9, 3, 20);
        assert_eq!(
            scrolled,
            vec![words[9].clone(), words[10].clone(), words[11].clone()],
            "the last window needs no marker"
        );
    }

    #[test]
    fn error_block_renders_the_hint_below_the_body() {
        let buffer = render_panel_error_block(CapsProfile::Rich, LONG_LOCKED_INDEX_MESSAGE);
        let layout =
            changes_panel_layout(Rect::new(0, 0, 40, PANEL_HEIGHT), false).expect("layout");
        let body = panel_error_layout(layout.list, Rect::new(0, 0, 40, PANEL_HEIGHT))
            .expect("the error block is laid out")
            .body;
        let hint_row = line_text(&buffer, body.bottom().saturating_sub(1));
        assert!(
            hint_row.contains(NEXT_STEP_HINT_PREFIX),
            "the hint row starts with the arrow, row {hint_row:?}"
        );
        let hint_fragment = LOCKED_INDEX_HINT
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            hint_row
                .replace(' ', "")
                .contains(&hint_fragment.replace(' ', "")),
            "the hint row names the next step, row {hint_row:?}"
        );
        let body_row = line_text(&buffer, body.y);
        assert!(
            !body_row.contains(&hint_fragment),
            "the hint never replaces the first body row, row {body_row:?}"
        );
        assert_eq!(
            buffer
                .cell((body.x + 2, body.bottom().saturating_sub(1)))
                .expect("cell")
                .fg,
            theme_palette(ThemeId::JbDark).muted,
            "the hint keeps the muted color"
        );
        let unknown = render_panel_error_block(CapsProfile::Rich, UNKNOWN_FAILURE_MESSAGE);
        assert!(
            !(body.y..body.bottom())
                .any(|row| line_text(&unknown, row).contains(NEXT_STEP_HINT_PREFIX)),
            "an unknown failure renders no hint row"
        );
    }

    #[test]
    fn a_long_message_wraps_inside_the_message_box_and_keeps_the_caret_visible() {
        let area = Rect::new(0, 0, 40, PANEL_HEIGHT);
        let layout = changes_panel_layout(area, false).expect("layout");
        let entries = dirty_entries();
        let rows = changes_rows(
            &entries,
            &default_collapsed_groups(),
            &checked_paths(&entries),
        );
        let long_message = "0123456789".repeat(6);
        let buffer = render_rows_with_commit(
            &entries,
            &rows,
            Some(0),
            CommitPanelView {
                editing: true,
                commit_enabled: true,
                ..idle_commit_view(&long_message)
            },
        );
        let first_line = interior_text(&buffer, layout.message_lines.y, layout.message_lines);
        let second_line = interior_text(&buffer, layout.message_lines.y + 1, layout.message_lines);
        assert_eq!(layout.message_lines.width, 36);
        assert_eq!(
            format!("{first_line}{second_line}"),
            long_message,
            "wrapping keeps every typed character"
        );
        assert_eq!(
            first_line.chars().next(),
            Some('0'),
            "the first line starts at the message start instead of scrolling"
        );
        let theme = theme_palette(ThemeId::JbDark);
        let caret_column = layout
            .message_lines
            .x
            .saturating_add(text_display_width(&second_line));
        assert_eq!(
            buffer
                .cell((caret_column, layout.message_lines.y + 1))
                .expect("caret cell")
                .bg,
            theme.text,
            "the caret follows the wrapped line"
        );
    }

    fn interior_text(buffer: &Buffer, row: u16, lines: Rect) -> String {
        let mut text = String::new();
        for column in lines.x..lines.right() {
            text.push_str(
                buffer
                    .cell((column, row))
                    .map(ratatui::buffer::Cell::symbol)
                    .unwrap_or(""),
            );
        }
        text.trim_end().to_string()
    }

    #[test]
    fn box_glyphs_degrade_to_ascii_for_basic_caps() {
        let rich = box_glyphs(CapsProfile::Rich);
        assert_eq!(rich.top_left, '┌');
        assert_eq!(rich.bottom_right, '┘');
        let basic = box_glyphs(CapsProfile::Basic);
        assert_eq!(
            basic,
            BoxGlyphs {
                horizontal: '-',
                vertical: '|',
                top_left: '+',
                top_right: '+',
                bottom_left: '+',
                bottom_right: '+',
            }
        );
        assert_eq!(box_glyphs(CapsProfile::Standard), rich);
    }
}
