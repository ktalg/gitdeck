use gitdeck_caps::{CapsProfile, ThemeId};
use gitdeck_core::{PaneId, RepoOperation};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthChar;

use crate::activity_rail::render_activity_rail;
use crate::display_text::sanitize_display_string;
use crate::glyphs::{ellipsis_glyph, line_glyphs, IconGlyph, LineGlyphs};
use crate::layout::{rect_contains_cell, DividerId, PaneLayout};
use crate::theme::{theme_palette, Theme};

pub const TOP_BAR_HELP_LABEL: &str = "?";
pub const REMOTE_TOOL_BAR_LABELS: [&str; 3] = ["Fetch", "Pull", "Push"];
pub const REMOTE_FETCH_TOOL_LABEL: &str = REMOTE_TOOL_BAR_LABELS[0];
pub const REMOTE_PULL_TOOL_LABEL: &str = REMOTE_TOOL_BAR_LABELS[1];
pub const REMOTE_PUSH_TOOL_LABEL: &str = REMOTE_TOOL_BAR_LABELS[2];
const REMOTE_TOOL_BAR_GAP: u16 = 1;
const TOP_BAR_PROJECT_LABEL_WIDTH_DIVISOR: u16 = 4;
const TOP_BAR_PROJECT_LABEL_TRIM_STEP: u16 = 2;
const BRANCH_CHIP_PREFIX: &str = "* ";
pub const NO_REPOSITORY_LABEL: &str = "no repository";
const CHANGES_ACTIVE_TAB_LABEL: &str = "Commit";
pub const LOG_TAB_TITLE: &str = "Log: HEAD";
pub const WORKING_TREE_TAB_SUFFIX: &str = " (Working Tree)";
const LOG_TAB_INSET: u16 = 1;
const LOG_TAB_GAP: u16 = 2;
const FILTER_BAR_INSET: u16 = 1;
const FILTER_BAR_SEPARATOR: &str = "  ";
pub const FILTER_BAR_TEXT_BLOCK_INDEX: usize = 0;
pub const FILTER_BAR_CASE_SENSITIVE_BLOCK_INDEX: usize = 1;
pub const FILTER_BAR_BRANCH_BLOCK_INDEX: usize = 2;
pub const FILTER_BAR_BLOCK_LABELS: [&str; 3] = ["Text or hash", "Cc", "Branch: HEAD"];
pub const FILTER_BAR_BRANCH_BLOCK_PREFIX: &str = "Branch: ";
const ELLIPSIS: char = '…';
pub const MERGING_LABEL: &str = "MERGING";
pub const REBASING_LABEL: &str = "REBASING";
pub const CHERRY_PICKING_LABEL: &str = "CHERRY-PICKING";
pub const REVERTING_LABEL: &str = "REVERTING";
const STATUS_VERSION_LABEL: &str = concat!("gitdeck ", env!("CARGO_PKG_VERSION"));
const STATUS_MESSAGE_GAP: u16 = 2;
const STATUS_HINT_GAP: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditorTabStrip<'a> {
    pub titles: &'a [String],
    pub active_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusTone {
    Info,
    Warning,
    Failure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusMessageView<'a> {
    pub text: &'a str,
    pub tone: StatusTone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogFilterBarView<'a> {
    pub text: &'a str,
    pub editing: bool,
    pub branch_filter: Option<&'a str>,
    pub case_sensitive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShellView<'a> {
    pub caps: CapsProfile,
    pub theme: ThemeId,
    pub status_left_label: &'a str,
    pub operation: Option<RepoOperation>,
    pub status_message: Option<StatusMessageView<'a>>,
    pub status_hint: Option<&'a str>,
    pub project_name: Option<&'a str>,
    pub head_branch: Option<&'a str>,
    pub focused_pane: PaneId,
    pub editor_tabs: EditorTabStrip<'a>,
    pub log_filter: LogFilterBarView<'a>,
    pub hovered_changes_toolbar: Option<ChangesToolbarAction>,
    pub ahead: u32,
    pub behind: u32,
    pub divergence_hovered: bool,
}

pub fn render_shell(area: Rect, layout: &PaneLayout, view: ShellView<'_>, buffer: &mut Buffer) {
    let theme = theme_palette(view.theme);
    let glyphs = line_glyphs(view.caps);
    buffer.set_style(area, Style::default().fg(theme.text).bg(theme.background));
    render_top_bar(layout, view, &theme, buffer);
    paint_activity_rail_background(layout, &theme, buffer);
    render_activity_rail(
        layout.activity_rail,
        view.focused_pane,
        view.caps,
        &theme,
        buffer,
    );
    render_changes_pane(layout, view, &theme, buffer);
    render_log_pane(layout, view, &theme, buffer);
    render_dividers(layout, view.focused_pane, &theme, &glyphs, buffer);
    render_status_bar(layout, view, &theme, buffer);
}

fn render_top_bar(layout: &PaneLayout, view: ShellView<'_>, theme: &Theme, buffer: &mut Buffer) {
    let bar = layout.top_bar;
    if bar.width == 0 || bar.height == 0 {
        return;
    }
    buffer.set_style(bar, Style::default().fg(theme.text).bg(theme.panel));
    let left_labels = top_bar_left_labels(
        bar,
        view.project_name,
        view.head_branch,
        view.ahead,
        view.behind,
        view.caps,
    );
    let mut column = draw_text(
        buffer,
        bar.x,
        bar.y,
        bar.width,
        &left_labels,
        Style::default().fg(theme.text).bg(theme.panel),
    );
    let branch_chip = branch_chip_label(view.head_branch);
    let branch_style = match view.head_branch {
        Some(_) => Style::default().fg(theme.text).bg(theme.panel),
        None => Style::default().fg(theme.muted).bg(theme.panel),
    };
    let chip_column = super::top_bar_branch_chip_rect(
        bar,
        view.project_name,
        view.head_branch,
        view.ahead,
        view.behind,
        view.caps,
    )
    .map(|rect| rect.x)
    .unwrap_or(column);
    column = draw_sanitized_text(
        buffer,
        chip_column,
        bar.y,
        bar.right().saturating_sub(chip_column),
        &branch_chip,
        view.caps,
        branch_style,
    );
    column = draw_text(
        buffer,
        column,
        bar.y,
        bar.right().saturating_sub(column),
        &branch_suffix_label(view.caps),
        Style::default().fg(theme.text).bg(theme.panel),
    );
    let divergence_style = divergence_style(view.divergence_hovered, view.behind, theme);
    draw_text(
        buffer,
        column,
        bar.y,
        bar.right().saturating_sub(column),
        &divergence_label(view.caps, view.ahead, view.behind),
        divergence_style,
    );
    for (rect, label) in remote_tool_bar_rects(
        bar,
        view.project_name,
        view.head_branch,
        view.ahead,
        view.behind,
        view.caps,
    )
    .into_iter()
    .zip(REMOTE_TOOL_BAR_LABELS)
    {
        if rect.width == 0 {
            continue;
        }
        draw_text(
            buffer,
            rect.x,
            rect.y,
            rect.width,
            label,
            Style::default().fg(theme.muted).bg(theme.panel),
        );
    }
    let help_button = top_bar_help_button_rect(bar);
    draw_text(
        buffer,
        help_button.x,
        help_button.y,
        help_button.width,
        TOP_BAR_HELP_LABEL,
        Style::default().fg(theme.muted).bg(theme.panel),
    );
}

fn top_bar_project_label_name(project_name: Option<&str>, caps: CapsProfile) -> String {
    sanitize_display_string(project_name.unwrap_or(NO_REPOSITORY_LABEL), 0, caps)
}

fn top_bar_left_labels(
    top_bar: Rect,
    project_name: Option<&str>,
    head_branch: Option<&str>,
    ahead: u32,
    behind: u32,
    caps: CapsProfile,
) -> String {
    let name = top_bar_project_label_name(project_name, caps);
    let mut slot =
        (top_bar.width / TOP_BAR_PROJECT_LABEL_WIDTH_DIVISOR).min(text_display_width(&name));
    loop {
        let labels = top_bar_left_labels_for_slot(&name, slot, caps);
        let left_width = text_display_width(&labels);
        let fetch_fits = remote_tool_bar_rects_after_left_labels(
            top_bar,
            left_width,
            head_branch,
            ahead,
            behind,
            caps,
        )[0]
        .width
            > 0;
        if slot == 0 || fetch_fits {
            return labels;
        }
        slot = slot.saturating_sub(TOP_BAR_PROJECT_LABEL_TRIM_STEP);
    }
}

fn top_bar_left_labels_for_slot(name: &str, slot_width: u16, caps: CapsProfile) -> String {
    let truncated = truncate_with_caps_ellipsis(name, slot_width, caps);
    format!(" {truncated} {} ", line_glyphs(caps).vertical)
}

fn branch_chip_label(head_branch: Option<&str>) -> String {
    match head_branch {
        Some(branch) => format!("{BRANCH_CHIP_PREFIX}{branch}"),
        None => String::from(NO_REPOSITORY_LABEL),
    }
}

fn branch_suffix_label(caps: CapsProfile) -> String {
    let chevron = IconGlyph::Chevron.character(caps);
    format!(" {chevron} ")
}

pub fn divergence_label(caps: CapsProfile, ahead: u32, behind: u32) -> String {
    let arrow_up = IconGlyph::ArrowUp.character(caps);
    let arrow_down = IconGlyph::ArrowDown.character(caps);
    format!("{arrow_up}{ahead} {arrow_down}{behind}")
}

fn divergence_style(hovered: bool, behind: u32, theme: &Theme) -> Style {
    if hovered {
        return Style::default().fg(theme.accent).bg(theme.panel);
    }
    if behind > 0 {
        return Style::default().fg(theme.warning).bg(theme.panel);
    }
    Style::default().fg(theme.muted).bg(theme.panel)
}

pub fn top_bar_divergence_rect(
    top_bar: Rect,
    project_name: Option<&str>,
    head_branch: Option<&str>,
    ahead: u32,
    behind: u32,
    caps: CapsProfile,
) -> Option<Rect> {
    let chip = top_bar_branch_chip_rect(
        top_bar,
        project_name,
        Some(head_branch?),
        ahead,
        behind,
        caps,
    )?;
    let start = chip
        .right()
        .saturating_add(text_display_width(&branch_suffix_label(caps)));
    let width = text_display_width(&divergence_label(caps, ahead, behind))
        .min(top_bar.right().saturating_sub(start));
    (width > 0).then_some(Rect::new(start, top_bar.y, width, 1))
}

pub fn top_bar_branch_chip_rect(
    top_bar: Rect,
    project_name: Option<&str>,
    head_branch: Option<&str>,
    ahead: u32,
    behind: u32,
    caps: CapsProfile,
) -> Option<Rect> {
    let chip_label = branch_chip_label(Some(head_branch?));
    let start = top_bar
        .x
        .saturating_add(text_display_width(&top_bar_left_labels(
            top_bar,
            project_name,
            head_branch,
            ahead,
            behind,
            caps,
        )));
    let width = text_display_width(&chip_label).min(top_bar.right().saturating_sub(start));
    (width > 0).then_some(Rect::new(start, top_bar.y, width, 1))
}

pub fn top_bar_branch_chevron_rect(
    top_bar: Rect,
    project_name: Option<&str>,
    head_branch: Option<&str>,
    ahead: u32,
    behind: u32,
    caps: CapsProfile,
) -> Option<Rect> {
    let chip = top_bar_branch_chip_rect(top_bar, project_name, head_branch, ahead, behind, caps)?;
    let width = text_display_width(&branch_suffix_label(caps))
        .min(top_bar.right().saturating_sub(chip.right()));
    (width > 0).then_some(Rect::new(chip.right(), top_bar.y, width, 1))
}

pub fn top_bar_help_button_rect(top_bar: Rect) -> Rect {
    let width = text_display_width(TOP_BAR_HELP_LABEL).min(top_bar.width);
    Rect::new(
        top_bar.right().saturating_sub(width),
        top_bar.y,
        width,
        top_bar.height,
    )
}

pub fn remote_tool_bar_rects(
    top_bar: Rect,
    project_name: Option<&str>,
    head_branch: Option<&str>,
    ahead: u32,
    behind: u32,
    caps: CapsProfile,
) -> [Rect; REMOTE_TOOL_BAR_LABELS.len()] {
    let left_width = text_display_width(&top_bar_left_labels(
        top_bar,
        project_name,
        head_branch,
        ahead,
        behind,
        caps,
    ));
    remote_tool_bar_rects_after_left_labels(top_bar, left_width, head_branch, ahead, behind, caps)
}

fn remote_tool_bar_rects_after_left_labels(
    top_bar: Rect,
    left_width: u16,
    head_branch: Option<&str>,
    ahead: u32,
    behind: u32,
    caps: CapsProfile,
) -> [Rect; REMOTE_TOOL_BAR_LABELS.len()] {
    let mut column = top_bar
        .x
        .saturating_add(left_width)
        .saturating_add(text_display_width(&branch_chip_label(head_branch)))
        .saturating_add(text_display_width(&branch_suffix_label(caps)))
        .saturating_add(text_display_width(&divergence_label(caps, ahead, behind)))
        .saturating_add(REMOTE_TOOL_BAR_GAP);
    let right_limit = top_bar
        .right()
        .saturating_sub(text_display_width(TOP_BAR_HELP_LABEL) + REMOTE_TOOL_BAR_GAP);
    let mut rects = [Rect::new(top_bar.x, top_bar.y, 0, 0); REMOTE_TOOL_BAR_LABELS.len()];
    for (index, label) in REMOTE_TOOL_BAR_LABELS.iter().enumerate() {
        let width = text_display_width(label);
        if column.saturating_add(width) > right_limit {
            break;
        }
        rects[index] = Rect::new(column, top_bar.y, width, 1);
        column = column.saturating_add(width + REMOTE_TOOL_BAR_GAP);
    }
    rects
}

fn paint_activity_rail_background(layout: &PaneLayout, theme: &Theme, buffer: &mut Buffer) {
    let rail = layout.activity_rail;
    if rail.width == 0 || rail.height == 0 {
        return;
    }
    buffer.set_style(rail, Style::default().fg(theme.muted).bg(theme.background));
}

fn render_changes_pane(
    layout: &PaneLayout,
    view: ShellView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let pane = layout.changes;
    if pane.width == 0 || pane.height == 0 {
        return;
    }
    buffer.set_style(pane, Style::default().fg(theme.text).bg(theme.panel));
    render_changes_tab_strip(pane, theme, buffer);
    render_changes_toolbar(pane, view, theme, buffer);
}

fn render_changes_tab_strip(pane: Rect, theme: &Theme, buffer: &mut Buffer) {
    draw_text(
        buffer,
        pane.x,
        pane.y,
        pane.width,
        CHANGES_ACTIVE_TAB_LABEL,
        Style::default()
            .fg(theme.text)
            .bg(theme.panel)
            .add_modifier(Modifier::UNDERLINED),
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangesToolbarAction {
    Refresh,
    Rollback,
    ShowDiff,
    MoveUp,
    MoveDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangesToolbarEntry {
    pub action: ChangesToolbarAction,
    pub icon: IconGlyph,
    pub tooltip: &'static str,
}

pub const CHANGES_TOOLBAR_ENTRIES: [ChangesToolbarEntry; 5] = [
    ChangesToolbarEntry {
        action: ChangesToolbarAction::Refresh,
        icon: IconGlyph::DotOutline,
        tooltip: "Refresh the repository (f5)",
    },
    ChangesToolbarEntry {
        action: ChangesToolbarAction::Rollback,
        icon: IconGlyph::ArrowLeft,
        tooltip: "Roll back the selected changes (alt+d)",
    },
    ChangesToolbarEntry {
        action: ChangesToolbarAction::ShowDiff,
        icon: IconGlyph::Pointer,
        tooltip: "Show the diff of the selected file (enter)",
    },
    ChangesToolbarEntry {
        action: ChangesToolbarAction::MoveUp,
        icon: IconGlyph::ArrowUp,
        tooltip: "Move the selection up (k)",
    },
    ChangesToolbarEntry {
        action: ChangesToolbarAction::MoveDown,
        icon: IconGlyph::Chevron,
        tooltip: "Move the selection down (j)",
    },
];

pub const CHANGES_TOOLBAR_ROW_OFFSET: u16 = 1;
pub const CHANGES_TOOLBAR_ENTRY_GAP: u16 = 1;

pub fn changes_toolbar_entry_rects(changes: Rect) -> [Rect; CHANGES_TOOLBAR_ENTRIES.len()] {
    let mut rects = [Rect::new(changes.x, changes.y, 0, 0); CHANGES_TOOLBAR_ENTRIES.len()];
    if changes.width == 0 || changes.height <= CHANGES_TOOLBAR_ROW_OFFSET {
        return rects;
    }
    let row = changes.y.saturating_add(CHANGES_TOOLBAR_ROW_OFFSET);
    for (index, rect) in rects.iter_mut().enumerate() {
        let offset =
            (1 + CHANGES_TOOLBAR_ENTRY_GAP).saturating_mul(u16::try_from(index).unwrap_or(0));
        let column = changes.x.saturating_add(offset);
        if column >= changes.right() {
            break;
        }
        *rect = Rect::new(column, row, 1, 1);
    }
    rects
}

pub fn changes_toolbar_entry_at(
    changes: Rect,
    column: u16,
    row: u16,
) -> Option<ChangesToolbarAction> {
    changes_toolbar_entry_rects(changes)
        .into_iter()
        .zip(CHANGES_TOOLBAR_ENTRIES)
        .find(|(rect, _)| rect_contains_cell(*rect, column, row))
        .map(|(_, entry)| entry.action)
}

pub fn changes_toolbar_tooltip(action: ChangesToolbarAction) -> Option<&'static str> {
    CHANGES_TOOLBAR_ENTRIES
        .iter()
        .find(|entry| entry.action == action)
        .map(|entry| entry.tooltip)
}

fn render_changes_toolbar(pane: Rect, view: ShellView<'_>, theme: &Theme, buffer: &mut Buffer) {
    for (rect, entry) in changes_toolbar_entry_rects(pane)
        .into_iter()
        .zip(CHANGES_TOOLBAR_ENTRIES)
    {
        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        let style = if view.hovered_changes_toolbar == Some(entry.action) {
            Style::default().fg(theme.accent).bg(theme.panel)
        } else {
            Style::default().fg(theme.muted).bg(theme.panel)
        };
        put_char(
            buffer,
            rect.x,
            rect.y,
            entry.icon.character(view.caps),
            style,
        );
    }
}

fn render_case_sensitive_filter_block(
    filters: Rect,
    branch_filter: Option<&str>,
    case_sensitive: bool,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if !case_sensitive {
        return;
    }
    let Some(block) = case_sensitive_filter_block_rect(filters, branch_filter) else {
        return;
    };
    draw_text(
        buffer,
        block.x,
        block.y,
        block.width,
        FILTER_BAR_BLOCK_LABELS[FILTER_BAR_CASE_SENSITIVE_BLOCK_INDEX],
        Style::default().fg(theme.accent).bg(theme.background),
    );
}

fn render_log_pane(layout: &PaneLayout, view: ShellView<'_>, theme: &Theme, buffer: &mut Buffer) {
    render_log_tabs(layout, view, theme, buffer);
    render_filter_bar(layout, view, theme, buffer);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorTabHit {
    Log,
    Diff(usize),
    CloseDiff(usize),
}

pub fn editor_tab_hit_at(
    tabs: Rect,
    titles: &[String],
    active_index: usize,
    caps: CapsProfile,
    column: u16,
    row: u16,
) -> Option<EditorTabHit> {
    if tabs.width == 0 || tabs.height == 0 || row != tabs.y {
        return None;
    }
    let close_glyph_width = text_display_width(&IconGlyph::Cross.character(caps).to_string());
    let mut start = tabs.x.saturating_add(LOG_TAB_INSET);
    for (index, title) in titles.iter().enumerate() {
        if start >= tabs.right() {
            return None;
        }
        let available = tabs.right().saturating_sub(start);
        let title_width =
            text_display_width(&sanitize_display_string(title, start, caps)).min(available);
        let title_end = start.saturating_add(title_width);
        let trailing_end = if index == active_index && index > 0 {
            title_end
                .saturating_add(1)
                .saturating_add(close_glyph_width)
                .min(tabs.right())
        } else {
            title_end
        };
        if (start..title_end).contains(&column) {
            return Some(tab_hit(index));
        }
        if (title_end..trailing_end).contains(&column) {
            return Some(close_hit(index));
        }
        start = trailing_end.saturating_add(LOG_TAB_GAP);
    }
    None
}

fn tab_hit(index: usize) -> EditorTabHit {
    match index.checked_sub(1) {
        Some(diff_index) => EditorTabHit::Diff(diff_index),
        None => EditorTabHit::Log,
    }
}

fn close_hit(index: usize) -> EditorTabHit {
    EditorTabHit::CloseDiff(index.saturating_sub(1))
}

fn render_log_tabs(layout: &PaneLayout, view: ShellView<'_>, theme: &Theme, buffer: &mut Buffer) {
    let tabs = layout.log_tabs;
    if tabs.width == 0 || tabs.height == 0 {
        return;
    }
    buffer.set_style(tabs, Style::default().fg(theme.text).bg(theme.background));
    let mut column = tabs.x.saturating_add(LOG_TAB_INSET);
    for (index, title) in view.editor_tabs.titles.iter().enumerate() {
        if column >= tabs.right() {
            break;
        }
        let active = index == view.editor_tabs.active_index;
        column = draw_ellipsized_text(
            buffer,
            column,
            tabs.y,
            tabs.right().saturating_sub(column),
            title,
            view.caps,
            tab_title_style(active, theme),
        );
        if active && index > 0 {
            column = draw_close_glyph(buffer, tabs, column, view, theme);
        }
        column = column.saturating_add(LOG_TAB_GAP);
    }
}

fn tab_title_style(active: bool, theme: &Theme) -> Style {
    if active {
        return Style::default()
            .fg(theme.text)
            .bg(theme.background)
            .add_modifier(Modifier::UNDERLINED);
    }
    Style::default().fg(theme.muted).bg(theme.background)
}

fn draw_close_glyph(
    buffer: &mut Buffer,
    tabs: Rect,
    column: u16,
    view: ShellView<'_>,
    theme: &Theme,
) -> u16 {
    let close_style = Style::default().fg(theme.muted).bg(theme.background);
    let column = draw_text(
        buffer,
        column,
        tabs.y,
        tabs.right().saturating_sub(column),
        " ",
        close_style,
    );
    draw_text(
        buffer,
        column,
        tabs.y,
        tabs.right().saturating_sub(column),
        &IconGlyph::Cross.character(view.caps).to_string(),
        close_style,
    )
}

fn render_filter_bar(layout: &PaneLayout, view: ShellView<'_>, theme: &Theme, buffer: &mut Buffer) {
    let filters = layout.log_filters;
    if filters.width == 0 || filters.height == 0 {
        return;
    }
    buffer.set_style(
        filters,
        Style::default().fg(theme.text).bg(theme.background),
    );
    let available_width = filters.width.saturating_sub(FILTER_BAR_INSET);
    if view.log_filter.text.is_empty() && !view.log_filter.editing {
        let labels = filter_bar_labels_within_width(
            available_width,
            view.log_filter.branch_filter,
            view.caps,
        );
        draw_text(
            buffer,
            filters.x + FILTER_BAR_INSET,
            filters.y,
            available_width,
            &labels,
            Style::default().fg(theme.muted).bg(theme.background),
        );
        render_case_sensitive_filter_block(
            filters,
            view.log_filter.branch_filter,
            view.log_filter.case_sensitive,
            theme,
            buffer,
        );
        return;
    }
    render_filter_query(filters, available_width, view, theme, buffer);
}

fn render_filter_query(
    filters: Rect,
    available_width: u16,
    view: ShellView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let text = view.log_filter.text;
    let text_style = filter_query_style(view.log_filter.editing, theme);
    let column = filters.x + FILTER_BAR_INSET;
    if !view.log_filter.editing {
        draw_ellipsized_text(
            buffer,
            column,
            filters.y,
            available_width,
            text,
            view.caps,
            text_style,
        );
        return;
    }
    let shown_text = take_display_suffix(text, available_width.saturating_sub(1));
    let caret_column = draw_sanitized_text(
        buffer,
        column,
        filters.y,
        available_width,
        shown_text,
        view.caps,
        text_style,
    );
    put_caret_at(buffer, caret_column, filters.right(), filters.y, theme);
}

fn filter_query_style(editing: bool, theme: &Theme) -> Style {
    if editing {
        return Style::default().fg(theme.text).bg(theme.background);
    }
    Style::default().fg(theme.accent).bg(theme.background)
}

fn put_caret_at(buffer: &mut Buffer, column: u16, right: u16, row: u16, theme: &Theme) {
    if column >= right {
        return;
    }
    put_char(
        buffer,
        column,
        row,
        ' ',
        Style::default().fg(theme.panel).bg(theme.text),
    );
}

pub fn visible_filter_bar_labels(branch_filter: Option<&str>) -> String {
    filter_bar_block_labels(branch_filter).join(FILTER_BAR_SEPARATOR)
}

pub fn filter_bar_labels_within_width(
    available_width: u16,
    branch_filter: Option<&str>,
    caps: CapsProfile,
) -> String {
    let labels = visible_filter_bar_labels(branch_filter);
    if text_display_width(&labels) <= available_width {
        return labels;
    }
    truncate_with_caps_ellipsis(&labels, available_width, caps)
}

pub fn branch_filter_block_label(branch_filter: Option<&str>) -> String {
    match branch_filter {
        Some(branch) => format!("{FILTER_BAR_BRANCH_BLOCK_PREFIX}{branch}"),
        None => String::from(FILTER_BAR_BLOCK_LABELS[FILTER_BAR_BRANCH_BLOCK_INDEX]),
    }
}

pub fn branch_filter_block_rect(filters: Rect, branch_filter: Option<&str>) -> Option<Rect> {
    filter_bar_block_rect(filters, branch_filter, FILTER_BAR_BRANCH_BLOCK_INDEX)
}

pub fn text_filter_block_rect(filters: Rect, branch_filter: Option<&str>) -> Option<Rect> {
    filter_bar_block_rect(filters, branch_filter, FILTER_BAR_TEXT_BLOCK_INDEX)
}

pub fn case_sensitive_filter_block_rect(
    filters: Rect,
    branch_filter: Option<&str>,
) -> Option<Rect> {
    filter_bar_block_rect(
        filters,
        branch_filter,
        FILTER_BAR_CASE_SENSITIVE_BLOCK_INDEX,
    )
}

fn filter_bar_block_rect(
    filters: Rect,
    branch_filter: Option<&str>,
    block_index: usize,
) -> Option<Rect> {
    if filters.height == 0 {
        return None;
    }
    let blocks = filter_bar_block_labels(branch_filter);
    let mut column = filters.x.saturating_add(FILTER_BAR_INSET);
    for block in blocks.iter().take(block_index) {
        column = column
            .saturating_add(text_display_width(block))
            .saturating_add(filter_bar_separator_width());
    }
    let label = blocks.get(block_index)?;
    let width = text_display_width(label).min(filters.right().saturating_sub(column));
    (width > 0).then_some(Rect::new(column, filters.y, width, 1))
}

fn filter_bar_block_labels(branch_filter: Option<&str>) -> Vec<String> {
    let mut blocks: Vec<String> = FILTER_BAR_BLOCK_LABELS
        .iter()
        .map(|label| String::from(*label))
        .collect();
    blocks[FILTER_BAR_BRANCH_BLOCK_INDEX] = branch_filter_block_label(branch_filter);
    blocks
}

fn filter_bar_separator_width() -> u16 {
    text_display_width(FILTER_BAR_SEPARATOR)
}

fn render_dividers(
    layout: &PaneLayout,
    focused_pane: PaneId,
    theme: &Theme,
    glyphs: &LineGlyphs,
    buffer: &mut Buffer,
) {
    if layout.activity_rail.height == 0 || layout.top_bar.width == 0 {
        return;
    }
    let rule_style = Style::default().fg(theme.border).bg(theme.background);
    let rule_row = layout.activity_rail.bottom() - 1;
    for column in 0..layout.top_bar.width {
        put_char(buffer, column, rule_row, glyphs.horizontal, rule_style);
    }
    for divider in [DividerId::ChangesLog, DividerId::CommitsDetails] {
        let rect = layout.divider_rect(divider);
        if rect.width == 0 {
            continue;
        }
        let style = divider_style(divider, focused_pane, theme);
        for row in rect.y..rect.bottom() {
            put_char(buffer, rect.x, row, glyphs.vertical, style);
        }
        put_char(buffer, rect.x, rule_row, glyphs.tee_up, style);
    }
}

fn divider_style(divider: DividerId, focused_pane: PaneId, theme: &Theme) -> Style {
    let borders_the_focused_pane = match divider {
        DividerId::ChangesLog => matches!(focused_pane, PaneId::Changes | PaneId::Log),
        DividerId::CommitsDetails => matches!(focused_pane, PaneId::Log | PaneId::Details),
    };
    let color = if borders_the_focused_pane {
        theme.accent
    } else {
        theme.border
    };
    Style::default().fg(color).bg(theme.background)
}

fn render_status_bar(layout: &PaneLayout, view: ShellView<'_>, theme: &Theme, buffer: &mut Buffer) {
    let status_bar = layout.status_bar;
    if status_bar.width == 0 || status_bar.height == 0 {
        return;
    }
    buffer.set_style(
        status_bar,
        Style::default().fg(theme.muted).bg(theme.background),
    );
    let mut label_end = draw_text(
        buffer,
        status_bar.x + 1,
        status_bar.y,
        status_bar.width,
        view.status_left_label,
        Style::default().fg(theme.accent).bg(theme.background),
    );
    if let Some(operation) = view.operation {
        let chip_column = label_end.saturating_add(STATUS_MESSAGE_GAP);
        label_end = draw_text(
            buffer,
            chip_column,
            status_bar.y,
            status_bar.right().saturating_sub(chip_column),
            operation.label(),
            Style::default().fg(theme.warning).bg(theme.background),
        );
    }
    let version_column = status_bar
        .right()
        .saturating_sub(u16::try_from(STATUS_VERSION_LABEL.chars().count()).unwrap_or(0) + 1);
    let hint_start = view.status_hint.and_then(|hint| {
        let start = version_column.saturating_sub(text_display_width(hint) + STATUS_HINT_GAP);
        (start > label_end).then_some(start)
    });
    let message_limit = hint_start.unwrap_or(version_column);
    if let Some(message) = view.status_message {
        let message_column = label_end.saturating_add(STATUS_MESSAGE_GAP);
        draw_ellipsized_text(
            buffer,
            message_column,
            status_bar.y,
            message_limit.saturating_sub(message_column + 1),
            message.text,
            view.caps,
            Style::default()
                .fg(status_message_color(message.tone, theme))
                .bg(theme.background),
        );
    }
    if let (Some(hint), Some(hint_start)) = (view.status_hint, hint_start) {
        draw_text(
            buffer,
            hint_start,
            status_bar.y,
            version_column.saturating_sub(hint_start),
            hint,
            Style::default().fg(theme.muted).bg(theme.background),
        );
    }
    draw_text(
        buffer,
        version_column,
        status_bar.y,
        status_bar.right().saturating_sub(version_column),
        STATUS_VERSION_LABEL,
        Style::default().fg(theme.muted).bg(theme.background),
    );
}

fn status_message_color(tone: StatusTone, theme: &Theme) -> Color {
    match tone {
        StatusTone::Info => theme.text,
        StatusTone::Warning => theme.warning,
        StatusTone::Failure => theme.error,
    }
}

fn draw_text(buffer: &mut Buffer, x: u16, y: u16, max_width: u16, text: &str, style: Style) -> u16 {
    let limit = x.saturating_add(max_width);
    let mut column = x;
    for character in text.chars() {
        let width = character_display_width(character);
        if column.saturating_add(width) > limit {
            break;
        }
        put_cell(buffer, column, y, character, width, style);
        column = column.saturating_add(width);
    }
    column
}

fn draw_sanitized_text(
    buffer: &mut Buffer,
    x: u16,
    y: u16,
    max_width: u16,
    text: &str,
    caps: CapsProfile,
    style: Style,
) -> u16 {
    let sanitized = sanitize_display_string(text, x, caps);
    draw_text(buffer, x, y, max_width, &sanitized, style)
}

fn draw_ellipsized_text(
    buffer: &mut Buffer,
    x: u16,
    y: u16,
    max_width: u16,
    text: &str,
    caps: CapsProfile,
    style: Style,
) -> u16 {
    let sanitized = sanitize_display_string(text, x, caps);
    if text_display_width(&sanitized) <= max_width {
        return draw_text(buffer, x, y, max_width, &sanitized, style);
    }
    let truncated = truncate_with_caps_ellipsis(&sanitized, max_width, caps);
    draw_text(buffer, x, y, max_width, &truncated, style)
}

pub fn text_display_width(text: &str) -> u16 {
    text.chars()
        .map(character_display_width)
        .fold(0u16, |width, character_width| {
            width.saturating_add(character_width)
        })
}

pub fn take_display_suffix(text: &str, max_width: u16) -> &str {
    let mut consumed_width = 0u16;
    for (index, character) in text.char_indices().rev() {
        let character_width = character_display_width(character);
        if consumed_width.saturating_add(character_width) > max_width {
            return &text[index + character.len_utf8()..];
        }
        consumed_width = consumed_width.saturating_add(character_width);
    }
    text
}

pub fn take_display_prefix(text: &str, max_width: u16) -> (&str, &str) {
    let mut consumed_width = 0u16;
    for (index, character) in text.char_indices() {
        let character_width = character_display_width(character);
        if consumed_width.saturating_add(character_width) > max_width {
            return (&text[..index], &text[index..]);
        }
        consumed_width = consumed_width.saturating_add(character_width);
    }
    (text, "")
}

pub fn truncate_with_ellipsis(text: &str, max_width: u16) -> String {
    truncate_with_glyph(text, max_width, ELLIPSIS)
}

pub fn truncate_with_caps_ellipsis(text: &str, max_width: u16, caps: CapsProfile) -> String {
    truncate_with_glyph(text, max_width, ellipsis_glyph(caps))
}

fn truncate_with_glyph(text: &str, max_width: u16, ellipsis: char) -> String {
    if text_display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let budget = max_width.saturating_sub(character_display_width(ellipsis));
    let mut truncated = String::new();
    let mut width = 0u16;
    for character in text.chars() {
        let character_width = character_display_width(character);
        if width.saturating_add(character_width) > budget {
            break;
        }
        truncated.push(character);
        width = width.saturating_add(character_width);
    }
    truncated.push(ellipsis);
    truncated
}

pub(crate) fn character_display_width(character: char) -> u16 {
    u16::try_from(UnicodeWidthChar::width(character).unwrap_or(1).max(1)).unwrap_or(1)
}

fn put_cell(buffer: &mut Buffer, x: u16, y: u16, symbol: char, width: u16, style: Style) {
    put_char(buffer, x, y, symbol, style);
    for offset in 1..width {
        put_char(buffer, x.saturating_add(offset), y, ' ', style);
    }
}

fn put_char(buffer: &mut Buffer, x: u16, y: u16, symbol: char, style: Style) {
    let mut encoded = [0u8; 4];
    let encoded_symbol = symbol.encode_utf8(&mut encoded);
    if let Some(cell) = buffer.cell_mut((x, y)) {
        cell.set_symbol(encoded_symbol);
        cell.set_style(style);
    }
}

#[cfg(test)]
mod shell_tests {
    #[test]
    fn the_compact_filter_bar_ends_with_an_ellipsis() {
        let branch_filter = Some("feature/a-very-long-branch-name");
        let mut truncated_widths = 0;
        for (caps, ellipsis) in [
            (CapsProfile::Rich, '…'),
            (CapsProfile::Standard, '…'),
            (CapsProfile::Basic, '~'),
        ] {
            for available_width in 12u16..=80 {
                let labels =
                    super::filter_bar_labels_within_width(available_width, branch_filter, caps);
                assert!(
                    super::text_display_width(&labels) <= available_width,
                    "the {caps:?} row stays inside {available_width} columns, row was {labels:?}"
                );
                let full_row = super::visible_filter_bar_labels(branch_filter);
                if super::text_display_width(&full_row) <= available_width {
                    assert_eq!(
                        labels, full_row,
                        "a row that fits keeps every block at {available_width} columns"
                    );
                    continue;
                }
                truncated_widths += 1;
                assert!(
                    labels.ends_with(ellipsis),
                    "the {caps:?} row ends with the ellipsis at {available_width} columns, row was {labels:?}"
                );
            }
        }
        assert!(
            truncated_widths > 0,
            "the compact widths cut the filter row at least once"
        );
    }

    use gitdeck_caps::{CapsProfile, ThemeId};
    use gitdeck_core::{PaneId, Size};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier};

    use super::{
        branch_filter_block_rect, render_shell, take_display_suffix, text_display_width,
        truncate_with_caps_ellipsis, truncate_with_ellipsis, visible_filter_bar_labels,
        EditorTabStrip, LogFilterBarView, ShellView, StatusMessageView, StatusTone, LOG_TAB_TITLE,
    };
    use crate::activity_rail::{
        activity_rail_icon_rect, ACTIVITY_RAIL_ENTRIES, ACTIVITY_RAIL_ICON_COLUMN,
    };
    use crate::layout::{
        compute_pane_layout, ACTIVITY_RAIL_WIDTH, BOTTOM_RULE_HEIGHT, CHANGES_DEFAULT_WIDTH,
        DETAILS_DEFAULT_WIDTH, STATUS_BAR_HEIGHT, TOP_BAR_HEIGHT,
    };

    const BACKGROUND_COLOR: Color = Color::Rgb(0x1E, 0x1F, 0x22);
    const MUTED_COLOR: Color = Color::Rgb(0x86, 0x8A, 0x91);
    const TEXT_COLOR: Color = Color::Rgb(0xDF, 0xE1, 0xE5);
    const LOG_TAB_TITLE_COLUMN: u16 = 40;

    fn render(width: u16, height: u16, caps: CapsProfile) -> Buffer {
        let log_only_titles = [String::from(LOG_TAB_TITLE)];
        render_with_titles(width, height, caps, &log_only_titles, 0)
    }

    fn render_with_titles(
        width: u16,
        height: u16,
        caps: CapsProfile,
        titles: &[String],
        active_index: usize,
    ) -> Buffer {
        render_with_project_name(width, height, caps, titles, active_index, Some("gitdeck"))
    }

    fn render_with_project_name(
        width: u16,
        height: u16,
        caps: CapsProfile,
        titles: &[String],
        active_index: usize,
        project_name: Option<&str>,
    ) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        let layout = compute_pane_layout(
            Size { width, height },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        render_shell(
            buffer.area,
            &layout,
            ShellView {
                caps,
                theme: ThemeId::JbDark,
                status_left_label: "master",
                operation: None,
                status_message: None,
                status_hint: None,
                project_name,
                head_branch: Some("master"),
                focused_pane: PaneId::Changes,
                editor_tabs: EditorTabStrip {
                    titles,
                    active_index,
                },
                log_filter: LogFilterBarView {
                    text: "",
                    editing: false,
                    branch_filter: None,
                    case_sensitive: false,
                },
                hovered_changes_toolbar: None,
                ahead: 0,
                behind: 0,
                divergence_hovered: false,
            },
            &mut buffer,
        );
        buffer
    }

    #[test]
    fn the_activity_rail_paints_only_the_entry_icons() {
        for caps in [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic] {
            for (width, height) in [(80, 24), (120, 32), (220, 70)] {
                let buffer = render(width, height, caps);
                let area = buffer.area;
                let last_rail_row = area
                    .bottom()
                    .saturating_sub(STATUS_BAR_HEIGHT)
                    .saturating_sub(BOTTOM_RULE_HEIGHT);
                let icon_rows: Vec<u16> = ACTIVITY_RAIL_ENTRIES
                    .iter()
                    .filter_map(|entry| {
                        activity_rail_icon_rect(
                            Rect::new(0, TOP_BAR_HEIGHT, ACTIVITY_RAIL_WIDTH, area.height),
                            *entry,
                        )
                        .map(|icon| icon.y)
                    })
                    .collect();
                assert_eq!(icon_rows.len(), 3);
                for row in TOP_BAR_HEIGHT..last_rail_row {
                    for column in 0..ACTIVITY_RAIL_WIDTH.min(area.width) {
                        let symbol = buffer.cell((column, row)).expect("rail cell").symbol();
                        if column == ACTIVITY_RAIL_ICON_COLUMN && icon_rows.contains(&row) {
                            assert_ne!(
                                symbol, " ",
                                "{caps:?} at {width}x{height} drops the icon at ({column}, {row})"
                            );
                            continue;
                        }
                        assert_eq!(
                            symbol, " ",
                            "{caps:?} at {width}x{height} paints {symbol:?} into the rail at ({column}, {row})"
                        );
                    }
                }
            }
        }
    }

    fn line_text_without_wide_fillers(buffer: &Buffer, row: u16) -> String {
        let mut line = String::new();
        let mut column = 0;
        while column < buffer.area.width {
            let Some(cell) = buffer.cell((column, row)) else {
                break;
            };
            line.push_str(cell.symbol());
            let symbol_width =
                u16::try_from(unicode_width::UnicodeWidthStr::width(cell.symbol()).max(1))
                    .unwrap_or(1);
            column += symbol_width;
        }
        line
    }

    fn line_text(buffer: &Buffer, row: u16) -> String {
        let mut line = String::new();
        for column in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((column, row)) {
                line.push_str(cell.symbol());
            }
        }
        line
    }

    #[test]
    fn top_bar_project_label_follows_the_project_name() {
        let titles = [String::from(LOG_TAB_TITLE)];
        let first = line_text(
            &render_with_project_name(
                160,
                48,
                CapsProfile::Rich,
                &titles,
                0,
                Some("gitdeck-sandbox"),
            ),
            0,
        );
        let second = line_text(
            &render_with_project_name(
                160,
                48,
                CapsProfile::Rich,
                &titles,
                0,
                Some("other-checkout"),
            ),
            0,
        );
        assert!(
            first.starts_with(" gitdeck-sandbox │ * master ▾ ↑0 ↓0"),
            "line {first:?}"
        );
        assert!(
            second.starts_with(" other-checkout │ * master ▾ ↑0 ↓0"),
            "line {second:?}"
        );
        assert_ne!(first, second, "two projects name themselves differently");
    }

    #[test]
    fn top_bar_truncates_a_long_project_name_to_a_quarter_of_the_bar() {
        let titles = [String::from(LOG_TAB_TITLE)];
        let project_name = "a-very-long-project-directory-name";
        let top_bar = line_text(
            &render_with_project_name(120, 32, CapsProfile::Rich, &titles, 0, Some(project_name)),
            0,
        );
        let separator = top_bar
            .chars()
            .position(|cell| cell == '│')
            .unwrap_or_else(|| panic!("the project label keeps its divider, line {top_bar:?}"));
        assert_eq!(
            separator,
            1 + 30 + 1,
            "the name stops at a quarter of the 120 columns, line {top_bar:?}"
        );
        assert!(
            top_bar.contains('…'),
            "the capped name ends with an ellipsis, line {top_bar:?}"
        );
        for label in super::REMOTE_TOOL_BAR_LABELS {
            assert!(
                top_bar.contains(label),
                "the capped name keeps {label} reachable, line {top_bar:?}"
            );
        }
    }

    #[test]
    fn top_bar_lists_project_and_branch_chips() {
        let buffer = render(160, 48, CapsProfile::Rich);
        let top_bar = line_text(&buffer, 0);
        assert!(
            top_bar.starts_with(" gitdeck │ * master ▾ ↑0 ↓0"),
            "line {top_bar:?}"
        );
        assert!(top_bar.trim_end().ends_with('?'), "line {top_bar:?}");
    }

    #[test]
    fn changes_tab_strip_draws_only_the_commit_tab() {
        let buffer = render(160, 48, CapsProfile::Rich);
        let tabs_line = line_text(&buffer, 1);
        assert!(
            tabs_line.trim_start().starts_with("Commit"),
            "line {tabs_line:?}"
        );
        assert!(!tabs_line.contains("Shelf"), "line {tabs_line:?}");
        for column in 4..10 {
            let cell = buffer.cell((column, 1)).expect("commit tab cell");
            assert!(
                cell.modifier.contains(Modifier::UNDERLINED),
                "column {column} is not underlined"
            );
        }
    }

    #[test]
    fn log_area_draws_tab_and_filter_bar() {
        let buffer = render(160, 48, CapsProfile::Rich);
        let tabs_line = line_text(&buffer, 1);
        assert!(tabs_line.contains("Log: HEAD"), "line {tabs_line:?}");
        assert!(!tabs_line.contains('+'), "line {tabs_line:?}");
        let filter_line = line_text(&buffer, 2);
        for label in ["Text or hash", "Cc", "Branch: HEAD"] {
            assert!(filter_line.contains(label), "filter {label:?} is missing");
        }
        for removed in [".*", "User", "Date", "Paths"] {
            assert!(!filter_line.contains(removed), "filter {removed:?} is back");
        }
    }

    #[test]
    fn log_tabs_list_every_open_tab_with_the_active_one_underlined() {
        let titles = [
            String::from(LOG_TAB_TITLE),
            String::from("app.rs (Working Tree)"),
        ];
        let buffer = render_with_titles(160, 48, CapsProfile::Rich, &titles, 1);
        let tabs_line = line_text(&buffer, 1);
        assert!(
            tabs_line.contains("Log: HEAD  app.rs (Working Tree)"),
            "line {tabs_line:?}"
        );
        let log_tab_cell = buffer
            .cell((LOG_TAB_TITLE_COLUMN, 1))
            .expect("log tab cell");
        assert!(!log_tab_cell.modifier.contains(Modifier::UNDERLINED));
        assert_eq!(log_tab_cell.fg, MUTED_COLOR);
        let diff_tab_column =
            LOG_TAB_TITLE_COLUMN + u16::try_from(LOG_TAB_TITLE.chars().count()).unwrap_or(0) + 2;
        let diff_tab_cell = buffer.cell((diff_tab_column, 1)).expect("diff tab cell");
        assert!(diff_tab_cell.modifier.contains(Modifier::UNDERLINED));
        assert_eq!(diff_tab_cell.fg, TEXT_COLOR);
    }

    #[test]
    fn the_permanent_log_tab_carries_no_close_glyph() {
        let buffer = render(160, 48, CapsProfile::Rich);
        assert!(
            !line_text(&buffer, 1).contains('✗'),
            "line {:?}",
            line_text(&buffer, 1)
        );
    }

    #[test]
    fn editor_and_status_surfaces_use_the_editor_background() {
        let buffer = render(160, 48, CapsProfile::Rich);
        for (column, row) in [(0, 5), (38, 5), (1, 8), (0, 47)] {
            assert_eq!(
                buffer.cell((column, row)).expect("cell").bg,
                BACKGROUND_COLOR,
                "cell ({column}, {row})"
            );
        }
        assert_eq!(
            buffer.cell((10, 3)).expect("changes cell").bg,
            Color::Rgb(0x2B, 0x2D, 0x30),
            "the changes pane keeps the panel background"
        );
    }

    #[test]
    fn status_bar_draws_branch_and_version() {
        let buffer = render(160, 48, CapsProfile::Rich);
        let status_line = line_text(&buffer, 47);
        assert!(status_line.contains("master"), "line {status_line:?}");
        assert!(
            status_line.contains(concat!("gitdeck ", env!("CARGO_PKG_VERSION"))),
            "line {status_line:?}"
        );
    }

    #[test]
    fn basic_profile_draws_ascii_lines_and_icons() {
        let buffer = render(160, 48, CapsProfile::Basic);
        let screen: String = (0..48).map(|row| line_text(&buffer, row)).collect();
        assert!(screen.contains('+'));
        assert!(screen.contains('|'));
        assert!(!screen.contains('┌'));
        assert!(!screen.contains('│'));
        assert!(!screen.contains('▾'));
        assert!(!screen.contains('●'));
    }

    #[test]
    fn details_pane_carries_no_labels() {
        let buffer = render(160, 48, CapsProfile::Rich);
        for row in 3..46 {
            let mut details_line = String::new();
            for column in 120..160 {
                details_line.push_str(buffer.cell((column, row)).expect("details cell").symbol());
            }
            assert!(
                details_line.trim().is_empty(),
                "details row {row} carries {details_line:?}"
            );
        }
    }

    #[test]
    fn zero_sized_screen_renders_without_panicking() {
        let buffer = render(0, 0, CapsProfile::Rich);
        assert_eq!(buffer.area.width, 0);
        let buffer = render(3, 2, CapsProfile::Rich);
        assert_eq!(buffer.area.width, 3);
    }

    #[test]
    fn top_bar_without_a_branch_reports_no_repository_instead_of_a_branch() {
        let titles = [String::from(LOG_TAB_TITLE)];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 120, 32));
        let layout = compute_pane_layout(
            Size {
                width: 120,
                height: 32,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        render_shell(
            buffer.area,
            &layout,
            ShellView {
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
                status_left_label: "no repository",
                operation: None,
                status_message: None,
                status_hint: None,
                project_name: None,
                head_branch: None,
                focused_pane: PaneId::Changes,
                editor_tabs: EditorTabStrip {
                    titles: &titles,
                    active_index: 0,
                },
                log_filter: LogFilterBarView {
                    text: "",
                    editing: false,
                    branch_filter: None,
                    case_sensitive: false,
                },
                hovered_changes_toolbar: None,
                ahead: 0,
                behind: 0,
                divergence_hovered: false,
            },
            &mut buffer,
        );
        let top_bar = line_text(&buffer, 0);
        let chip_column = top_bar
            .match_indices(super::NO_REPOSITORY_LABEL)
            .nth(1)
            .and_then(|(column, _)| u16::try_from(column).ok())
            .unwrap_or_else(|| {
                panic!("the branch chip falls back to the placeholder, line {top_bar:?}")
            });
        assert!(
            !top_bar.contains("master"),
            "the placeholder never claims a branch, line {top_bar:?}"
        );
        assert_eq!(
            top_bar.matches(super::NO_REPOSITORY_LABEL).count(),
            2,
            "the project label and the branch chip both fall back, line {top_bar:?}"
        );
        assert_eq!(
            buffer.cell((chip_column, 0)).expect("branch chip cell").fg,
            MUTED_COLOR,
            "the placeholder chip is muted"
        );
    }

    #[test]
    fn cjk_tab_title_keeps_every_wide_character() {
        let titles = [
            String::from(LOG_TAB_TITLE),
            String::from("说明.md (Working Tree)"),
        ];
        let buffer = render_with_titles(160, 48, CapsProfile::Rich, &titles, 1);
        let tabs_line = line_text_without_wide_fillers(&buffer, 1);
        assert!(
            tabs_line.contains("说明.md (Working Tree)"),
            "the wide tab title stays whole, line {tabs_line:?}"
        );
        assert!(
            !tabs_line.contains("说.md"),
            "the second wide character is not overwritten, line {tabs_line:?}"
        );
    }

    #[test]
    fn tab_titles_end_with_an_ellipsis_when_the_width_runs_out() {
        let titles = [
            String::from(LOG_TAB_TITLE),
            String::from("a-rather-long-file-name.txt (Working Tree)"),
        ];
        let buffer = render_with_titles(60, 24, CapsProfile::Rich, &titles, 1);
        let tabs_line = line_text(&buffer, 1);
        assert!(
            tabs_line.contains('…'),
            "the clipped tab title ends with an ellipsis, line {tabs_line:?}"
        );
        assert!(
            !tabs_line.contains("(Working Tree)"),
            "the tail of the clipped title is dropped, line {tabs_line:?}"
        );
    }

    #[test]
    fn truncation_keeps_wide_characters_whole() {
        assert_eq!(truncate_with_ellipsis("说明.md", 4), String::from("说…"));
        assert_eq!(truncate_with_ellipsis("说明.md", 6), String::from("说明.…"));
        assert_eq!(truncate_with_ellipsis("abc", 6), String::from("abc"));
        assert_eq!(truncate_with_ellipsis("abc", 2), String::from("a…"));
    }

    #[test]
    fn the_basic_profile_truncates_with_a_one_column_ascii_marker() {
        assert_eq!(
            truncate_with_caps_ellipsis("abcdef", 4, CapsProfile::Basic),
            String::from("abc~")
        );
        assert_eq!(
            truncate_with_caps_ellipsis("abcdef", 4, CapsProfile::Rich),
            String::from("abc…")
        );
        assert_eq!(
            text_display_width(&truncate_with_caps_ellipsis(
                "abcdef",
                4,
                CapsProfile::Basic
            )),
            4
        );
    }

    #[test]
    fn the_filter_bar_lists_only_the_blocks_that_act() {
        assert_eq!(
            visible_filter_bar_labels(None),
            String::from("Text or hash  Cc  Branch: HEAD")
        );
        assert_eq!(
            visible_filter_bar_labels(Some("feature/other")),
            String::from("Text or hash  Cc  Branch: feature/other")
        );
    }

    #[test]
    fn an_active_branch_filter_widens_its_chip_in_place() {
        let filters = Rect::new(35, 2, 45, 1);
        let branch_offset = 1 + text_display_width("Text or hash  Cc  ");
        assert_eq!(
            branch_filter_block_rect(filters, Some("feature/other")),
            Some(Rect::new(35 + branch_offset, 2, 21, 1))
        );
        assert_eq!(
            branch_filter_block_rect(Rect::new(35, 2, branch_offset, 1), Some("feature/other")),
            None,
            "a chip that starts past the right edge has no hit area"
        );
    }

    #[test]
    fn the_divider_beside_the_focused_pane_uses_the_accent_color() {
        let hidden_titles = [String::from(LOG_TAB_TITLE)];
        let render_with_focus = |focused_pane: PaneId| {
            let mut buffer = Buffer::empty(Rect::new(0, 0, 160, 48));
            let layout = compute_pane_layout(
                Size {
                    width: 160,
                    height: 48,
                },
                CHANGES_DEFAULT_WIDTH,
                DETAILS_DEFAULT_WIDTH,
                true,
            );
            render_shell(
                buffer.area,
                &layout,
                ShellView {
                    caps: CapsProfile::Rich,
                    theme: ThemeId::JbDark,
                    status_left_label: "main",
                    operation: None,
                    status_message: None,
                    status_hint: None,
                    project_name: Some("gitdeck"),
                    head_branch: Some("main"),
                    focused_pane,
                    editor_tabs: EditorTabStrip {
                        titles: &hidden_titles,
                        active_index: 0,
                    },
                    log_filter: LogFilterBarView {
                        text: "",
                        editing: false,
                        branch_filter: None,
                        case_sensitive: false,
                    },
                    hovered_changes_toolbar: None,
                    ahead: 0,
                    behind: 0,
                    divergence_hovered: false,
                },
                &mut buffer,
            );
            (buffer, layout)
        };
        let accent = Color::Rgb(0x35, 0x74, 0xF0);
        let border = Color::Rgb(0x39, 0x3B, 0x40);
        let (changes_buffer, changes_layout) = render_with_focus(PaneId::Changes);
        assert_eq!(
            changes_buffer
                .cell((changes_layout.changes_log_divider.x, 5))
                .expect("divider cell")
                .fg,
            accent
        );
        assert_eq!(
            changes_buffer
                .cell((changes_layout.commits_details_divider.x, 5))
                .expect("divider cell")
                .fg,
            border
        );
        let (details_buffer, details_layout) = render_with_focus(PaneId::Details);
        assert_eq!(
            details_buffer
                .cell((details_layout.commits_details_divider.x, 5))
                .expect("divider cell")
                .fg,
            accent
        );
        assert_eq!(
            details_buffer
                .cell((details_layout.changes_log_divider.x, 5))
                .expect("divider cell")
                .fg,
            border
        );
    }

    fn status_bar_row(buffer: &Buffer) -> String {
        line_text(buffer, buffer.area.height - 1)
    }

    #[test]
    fn a_status_message_sits_beside_the_branch_label() {
        let titles = [String::from(LOG_TAB_TITLE)];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 120, 32));
        let layout = compute_pane_layout(
            Size {
                width: 120,
                height: 32,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        render_shell(
            buffer.area,
            &layout,
            ShellView {
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
                status_left_label: "main",
                operation: None,
                status_message: Some(StatusMessageView {
                    text: "Committed 2 files to main",
                    tone: StatusTone::Info,
                }),
                status_hint: None,
                project_name: Some("gitdeck"),
                head_branch: Some("main"),
                focused_pane: PaneId::Changes,
                editor_tabs: EditorTabStrip {
                    titles: &titles,
                    active_index: 0,
                },
                log_filter: LogFilterBarView {
                    text: "",
                    editing: false,
                    branch_filter: None,
                    case_sensitive: false,
                },
                hovered_changes_toolbar: None,
                ahead: 0,
                behind: 0,
                divergence_hovered: false,
            },
            &mut buffer,
        );
        let row = status_bar_row(&buffer);
        assert!(
            row.starts_with(" main  Committed 2 files to main"),
            "status row is {row:?}"
        );
        assert_eq!(buffer.cell((7, 31)).expect("status cell").fg, TEXT_COLOR);
    }

    #[test]
    fn a_failure_status_message_uses_the_error_color() {
        let message = StatusMessageView {
            text: "Push failed: no remote",
            tone: StatusTone::Failure,
        };
        let mut buffer = Buffer::empty(Rect::new(0, 0, 120, 32));
        let theme = crate::theme::theme_palette(ThemeId::JbDark);
        render_status_message_for_test(&mut buffer, message);
        assert_eq!(buffer.cell((7, 31)).expect("status cell").fg, theme.error);
        assert!(line_text(&buffer, 31).contains("Push failed: no remote"));
    }

    fn render_status_message_for_test(buffer: &mut Buffer, message: StatusMessageView<'_>) {
        let layout = compute_pane_layout(
            Size {
                width: 120,
                height: 32,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        let titles = [String::from(LOG_TAB_TITLE)];
        render_shell(
            buffer.area,
            &layout,
            ShellView {
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
                status_left_label: "main",
                operation: None,
                status_message: Some(message),
                status_hint: None,
                project_name: Some("gitdeck"),
                head_branch: Some("main"),
                focused_pane: PaneId::Changes,
                editor_tabs: EditorTabStrip {
                    titles: &titles,
                    active_index: 0,
                },
                log_filter: LogFilterBarView {
                    text: "",
                    editing: false,
                    branch_filter: None,
                    case_sensitive: false,
                },
                hovered_changes_toolbar: None,
                ahead: 0,
                behind: 0,
                divergence_hovered: false,
            },
            buffer,
        );
    }

    #[test]
    fn take_display_suffix_keeps_the_tail_inside_the_column_budget() {
        assert_eq!(take_display_suffix("abcdef", 3), "def");
        assert_eq!(take_display_suffix("abcdef", 0), "");
        assert_eq!(take_display_suffix("abc", 9), "abc");
        assert_eq!(take_display_suffix("修复：冲突", 4), "冲突");
        assert_eq!(take_display_suffix("修复：冲突", 3), "突");
    }
}
