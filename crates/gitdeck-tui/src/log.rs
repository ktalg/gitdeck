use gitdeck_caps::{CapsProfile, ThemeId};
use gitdeck_core::graph::LANE_COLOR_SLOT_COUNT;
use gitdeck_core::{BranchEntry, ChangeEntry, CommitSummary, LaneRow, LaneSpan, RefKind, RefName};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

use crate::changes::{
    change_kind_color, change_kind_letter, file_count_label, repaint_blank_cell_backgrounds,
    selection_background, wrap_error_body_lines, wrap_error_text, write_row_segments, RowSegment,
};
use crate::display_text::sanitize_display_string;
use crate::glyphs::{line_glyphs, IconGlyph, LineGlyphs};
use crate::shell::{text_display_width, truncate_with_caps_ellipsis};
use crate::theme::{theme_palette, Theme};

pub const GRAPH_COLUMN_WIDTH: u16 = 6;
pub const SUBJECT_COLUMN: u16 = GRAPH_COLUMN_WIDTH;
pub const LANE_COLUMN_WIDTH: u16 = 2;
pub const VISIBLE_LANE_COUNT: u8 = 3;
pub const REFS_SUBJECT_GAP: u16 = 2;
pub const REFS_SEPARATOR: &str = ", ";
pub const DATE_COLUMN_WIDTH: u16 = 18;
pub const COMPACT_COLUMNS_SCREEN_WIDTH: u16 = 120;
pub const DATE_AUTHOR_GAP: u16 = 2;
pub const MINIMUM_SUBJECT_WIDTH: u16 = 8;
pub const SUBJECT_COLUMN_GAP: u16 = 1;
pub const LANE_PALETTE: [Color; LANE_COLOR_SLOT_COUNT as usize] = [
    Color::Rgb(0x57, 0x96, 0x5C),
    Color::Rgb(0x35, 0x74, 0xF0),
    Color::Rgb(0xC9, 0xA6, 0x3C),
    Color::Rgb(0xDB, 0x5C, 0x5C),
    Color::Rgb(0x7A, 0x7E, 0x85),
    Color::Rgb(0x2E, 0x9B, 0x8F),
];
pub const NO_COMMITS_LABEL: &str = "No commits";
pub const NO_FILES_LABEL: &str = "No files";
pub const FILES_HEADER_HEIGHT: u16 = 1;
pub const COMMIT_DETAIL_SEPARATOR_HEIGHT: u16 = 1;
pub const COMMIT_DETAIL_SUBJECT_MAX_ROWS: u16 = 3;
pub const COMMIT_DETAIL_META_HEIGHT: u16 = 3;
pub const CHANGED_FILE_INDENT: u16 = 2;
pub const CHANGED_FILE_STATUS_GAP: u16 = 2;
pub const CHANGED_FILE_DIRECTORY_GAP: u16 = 2;
pub const CHANGED_FILE_COUNT_GAP: u16 = 2;
pub const COMMIT_DETAIL_ID_GAP: &str = " ";
pub const TAG_REF_LABEL_PREFIX: &str = "tag: ";
pub const HEAD_REF_LABEL_PREFIX: &str = "HEAD -> ";
pub const HEAD_REF_LABEL: &str = "HEAD";
pub const COMMITTED_BY_LABEL_PREFIX: &str = "committed by ";
pub const CONTAINING_BRANCHES_SINGULAR_LABEL: &str = "In 1 branch";
pub const CONTAINING_BRANCHES_UNAVAILABLE_LABEL: &str = "In ? branches";
pub const MAX_LISTED_CONTAINING_BRANCHES: usize = 3;
pub const REMOTE_REF_PREFIX: &str = "refs/remotes/";
const COMMIT_DETAIL_SHORT_ID_LENGTH: usize = 7;
const COMMIT_DETAIL_KEEP_RANK: u8 = u8::MAX;
const CONTAINING_BRANCHES_ROW_RANK: u8 = 0;
const COMMITTED_BY_ROW_RANK: u8 = 1;
const COMMIT_DATE_ROW_RANK: u8 = 2;
const AUTHOR_EMAIL_ROW_RANK: u8 = 3;
const REFERENCE_ROW_RANK: u8 = 4;

pub struct CommitListView<'a> {
    pub commits: &'a [CommitSummary],
    pub lanes: &'a [LaneRow],
    pub selected_index: Option<usize>,
    pub scroll: usize,
    pub focused: bool,
    pub head_branch: Option<&'a str>,
    pub utc_offset_minutes: i32,
    pub screen_width: u16,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContainingBranches<'a> {
    Listed(&'a [BranchEntry]),
    Unavailable,
}

pub struct CommitDetailsView<'a> {
    pub commit: Option<&'a CommitSummary>,
    pub changed_files: &'a [ChangeEntry],
    pub head_branch: Option<&'a str>,
    pub containing_branches: Option<ContainingBranches<'a>>,
    pub utc_offset_minutes: i32,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangedFileRow {
    Directory {
        directory: String,
        file_count: usize,
    },
    File {
        index: usize,
        indented: bool,
    },
}

pub fn render_commit_list(area: Rect, view: CommitListView<'_>, buffer: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let theme = theme_palette(view.theme);
    if view.commits.is_empty() {
        buffer.set_stringn(
            area.x,
            area.y,
            NO_COMMITS_LABEL,
            usize::from(area.width),
            Style::default().fg(theme.muted).bg(theme.background),
        );
        return;
    }
    let visible_commit_indices = (view.scroll..view.commits.len()).take(usize::from(area.height));
    for (row_offset, commit_index) in visible_commit_indices.enumerate() {
        let Ok(row_offset) = u16::try_from(row_offset) else {
            return;
        };
        let background = selection_background(
            &theme,
            view.focused,
            view.selected_index == Some(commit_index),
            theme.background,
        );
        render_commit_row(
            &CommitRow {
                area,
                row_y: area.y + row_offset,
                commit: &view.commits[commit_index],
                lanes: view.lanes.get(commit_index),
                head_branch: view.head_branch,
                utc_offset_minutes: view.utc_offset_minutes,
                compact_columns: view.screen_width <= COMPACT_COLUMNS_SCREEN_WIDTH,
                caps: view.caps,
                background,
            },
            &theme,
            buffer,
        );
    }
}

struct CommitRow<'a> {
    area: Rect,
    row_y: u16,
    commit: &'a CommitSummary,
    lanes: Option<&'a LaneRow>,
    head_branch: Option<&'a str>,
    utc_offset_minutes: i32,
    compact_columns: bool,
    caps: CapsProfile,
    background: Color,
}

impl CommitRow<'_> {
    fn subject_start(&self) -> u16 {
        self.area.x + SUBJECT_COLUMN
    }

    fn subject_floor_offset(&self) -> u16 {
        SUBJECT_COLUMN + MINIMUM_SUBJECT_WIDTH
    }

    fn subject_limit(&self) -> u16 {
        self.area.x + self.subject_limit_offset()
    }

    fn subject_limit_offset(&self) -> u16 {
        if let Some(date_offset) = self.date_offset() {
            return date_offset.saturating_sub(SUBJECT_COLUMN_GAP);
        }
        if let Some(author_offset) = self.author_offset() {
            return author_offset.saturating_sub(SUBJECT_COLUMN_GAP);
        }
        self.area.width
    }

    fn author_offset(&self) -> Option<u16> {
        if self.compact_columns {
            return None;
        }
        let author_offset = self.area.width.saturating_sub(self.author_width());
        (author_offset >= self.subject_floor_offset()).then_some(author_offset)
    }

    fn author_width(&self) -> u16 {
        u16::try_from(self.commit.author_name.as_str().width()).unwrap_or(u16::MAX)
    }

    fn author_column(&self) -> Option<u16> {
        self.author_offset().map(|offset| self.area.x + offset)
    }

    fn date_offset(&self) -> Option<u16> {
        if self.compact_columns {
            return None;
        }
        let date_end = match self.author_offset() {
            Some(author_offset) => author_offset.checked_sub(DATE_AUTHOR_GAP)?,
            None => self.area.width,
        };
        let date_offset = date_end.checked_sub(DATE_COLUMN_WIDTH)?;
        (date_offset >= self.subject_floor_offset()).then_some(date_offset)
    }

    fn date_column(&self) -> Option<u16> {
        self.date_offset().map(|offset| self.area.x + offset)
    }

    fn lane_marker_column(&self, lane: u8) -> Option<u16> {
        if lane >= VISIBLE_LANE_COUNT {
            return None;
        }
        let column = self.area.x + u16::from(lane) * LANE_COLUMN_WIDTH;
        (column < self.area.right()).then_some(column)
    }

    fn lane_connection_columns(&self, from_lane: u8, to_lane: u8) -> Vec<u16> {
        let (first_lane, last_lane) = if from_lane < to_lane {
            (from_lane, to_lane)
        } else {
            (to_lane, from_lane)
        };
        (first_lane..last_lane)
            .filter_map(|lane| self.lane_connection_column(lane))
            .collect()
    }

    fn lane_connection_column(&self, lane: u8) -> Option<u16> {
        if lane >= VISIBLE_LANE_COUNT {
            return None;
        }
        let column = self.area.x + u16::from(lane) * LANE_COLUMN_WIDTH + 1;
        (column < self.area.right()).then_some(column)
    }
}

fn render_commit_row(row: &CommitRow<'_>, theme: &Theme, buffer: &mut Buffer) {
    buffer.set_style(
        Rect::new(row.area.x, row.row_y, row.area.width, 1),
        Style::default().fg(theme.text).bg(row.background),
    );
    render_commit_graph(row, buffer);
    write_commit_refs_and_subject(row, theme, buffer);
    if let Some(author_column) = row.author_column() {
        write_commit_text(
            buffer,
            author_column,
            row.row_y,
            row.area.right(),
            &row.commit.author_name,
            row.caps,
            Style::default().fg(theme.muted).bg(row.background),
        );
    }
    if let Some(date_column) = row.date_column() {
        write_right_aligned_text(
            buffer,
            date_column,
            row.row_y,
            DATE_COLUMN_WIDTH,
            &format_commit_date(row.commit.author_timestamp, row.utc_offset_minutes),
            Style::default().fg(theme.muted).bg(row.background),
        );
    }
    repaint_blank_cell_backgrounds(row.area, row.row_y, theme, row.background, buffer);
}

fn render_commit_graph(row: &CommitRow<'_>, buffer: &mut Buffer) {
    let Some(lane_row) = row.lanes else {
        return;
    };
    let glyphs = line_glyphs(row.caps);
    for span in &lane_row.lanes {
        let Some(marker_column) = row.lane_marker_column(span.lane) else {
            continue;
        };
        let Some(glyph) = lane_glyph(span, lane_row, row.caps, &glyphs) else {
            continue;
        };
        write_graph_cell(buffer, marker_column, row, glyph, lane_span_color(span));
    }
    for span in &lane_row.lanes {
        if span.lane == lane_row.commit_lane || span.continues_above == span.continues_below {
            continue;
        }
        let color = lane_span_color(span);
        for column in row.lane_connection_columns(lane_row.commit_lane, span.lane) {
            write_graph_cell(buffer, column, row, glyphs.horizontal, color);
        }
    }
}

fn lane_glyph(
    span: &LaneSpan,
    lane_row: &LaneRow,
    caps: CapsProfile,
    glyphs: &LineGlyphs,
) -> Option<char> {
    if span.lane == lane_row.commit_lane {
        return Some(commit_node_glyph(span.lane, lane_row, caps, glyphs));
    }
    match (span.continues_above, span.continues_below) {
        (true, true) => Some(glyphs.vertical),
        (false, true) => Some(glyphs.branch_corner),
        (true, false) => Some(glyphs.merge_corner),
        (false, false) => None,
    }
}

fn commit_node_glyph(lane: u8, lane_row: &LaneRow, caps: CapsProfile, glyphs: &LineGlyphs) -> char {
    let branches_below = lane_row
        .lanes
        .iter()
        .any(|span| span.lane > lane && span.continues_below && !span.continues_above);
    let merges_above = lane_row
        .lanes
        .iter()
        .any(|span| span.lane > lane && span.continues_above && !span.continues_below);
    if branches_below {
        glyphs.tee_down
    } else if merges_above {
        glyphs.tee_up
    } else {
        IconGlyph::DotFilled.character(caps)
    }
}

fn lane_span_color(span: &LaneSpan) -> Color {
    LANE_PALETTE[usize::from(span.color_slot % LANE_COLOR_SLOT_COUNT)]
}

fn write_graph_cell(
    buffer: &mut Buffer,
    column: u16,
    row: &CommitRow<'_>,
    glyph: char,
    foreground: Color,
) {
    let mut encoded = [0u8; 4];
    let symbol = glyph.encode_utf8(&mut encoded);
    if let Some(cell) = buffer.cell_mut((column, row.row_y)) {
        cell.set_symbol(symbol);
        cell.set_style(Style::default().fg(foreground).bg(row.background));
    }
}

fn write_commit_refs_and_subject(row: &CommitRow<'_>, theme: &Theme, buffer: &mut Buffer) -> u16 {
    let labels = if row.compact_columns {
        compact_ref_labels(row.commit, row.head_branch)
    } else {
        commit_ref_labels(row.commit, row.head_branch)
    };
    let limit = row.subject_limit();
    let subject = row.commit.subject.as_str();
    let reserved_subject_width = if row.compact_columns {
        MINIMUM_SUBJECT_WIDTH.min(text_display_width(subject))
    } else {
        0
    };
    let refs_limit = limit.saturating_sub(reserved_subject_width);
    let mut column = row.subject_start();
    for label in &labels {
        let style = Style::default()
            .fg(ref_label_color(label.kind, theme))
            .bg(row.background);
        column = write_commit_text(
            buffer,
            column,
            row.row_y,
            refs_limit,
            &label.text,
            row.caps,
            style,
        );
        if label.has_following_label {
            let separator_style = Style::default().fg(theme.muted).bg(row.background);
            column = write_commit_text(
                buffer,
                column,
                row.row_y,
                refs_limit,
                REFS_SEPARATOR,
                row.caps,
                separator_style,
            );
        }
    }
    if !labels.is_empty() && !subject.is_empty() {
        let gap_width = REFS_SUBJECT_GAP.min(refs_limit.saturating_sub(column));
        let gap_style = Style::default().bg(row.background);
        column = write_commit_text(
            buffer,
            column,
            row.row_y,
            limit,
            &" ".repeat(usize::from(gap_width)),
            row.caps,
            gap_style,
        );
    }
    let text = truncate_with_caps_ellipsis(subject, limit.saturating_sub(column), row.caps);
    write_commit_text(
        buffer,
        column,
        row.row_y,
        limit,
        &text,
        row.caps,
        Style::default().fg(theme.text).bg(row.background),
    )
}

fn write_commit_text(
    buffer: &mut Buffer,
    column: u16,
    row_y: u16,
    limit: u16,
    text: &str,
    caps: CapsProfile,
    style: Style,
) -> u16 {
    if column >= limit {
        return column;
    }
    let sanitized_text = sanitize_display_string(text, column, caps);
    let (next_column, _) = buffer.set_stringn(
        column,
        row_y,
        &sanitized_text,
        usize::from(limit.saturating_sub(column)),
        style,
    );
    next_column
}

fn write_right_aligned_text(
    buffer: &mut Buffer,
    column: u16,
    row_y: u16,
    field_width: u16,
    text: &str,
    style: Style,
) {
    let text_width = u16::try_from(text.chars().count()).unwrap_or(field_width);
    let start = column + field_width.saturating_sub(text_width);
    let _ = buffer.set_stringn(start, row_y, text, usize::from(text_width), style);
}

pub struct RefLabel {
    pub text: String,
    pub kind: RefKind,
    pub has_following_label: bool,
}

pub fn commit_ref_labels(commit: &CommitSummary, head_branch: Option<&str>) -> Vec<RefLabel> {
    let names = renderable_ref_names(commit, head_branch);
    let last_index = names.len().saturating_sub(1);
    names
        .iter()
        .enumerate()
        .map(|(index, name)| RefLabel {
            text: ref_label(name, head_branch),
            kind: name.kind,
            has_following_label: index < last_index,
        })
        .collect()
}

fn compact_ref_labels(commit: &CommitSummary, head_branch: Option<&str>) -> Vec<RefLabel> {
    commit_ref_labels(commit, head_branch)
        .into_iter()
        .map(
            |label| match label.text.strip_prefix(HEAD_REF_LABEL_PREFIX) {
                Some(branch) => RefLabel {
                    text: branch.to_string(),
                    ..label
                },
                None => label,
            },
        )
        .collect()
}

fn renderable_ref_names<'a>(
    commit: &'a CommitSummary,
    head_branch: Option<&str>,
) -> Vec<&'a RefName> {
    let head_points_at_branch =
        head_branch.is_some() && commit.refs.iter().any(|name| name.kind == RefKind::Head);
    commit
        .refs
        .iter()
        .filter(|name| {
            let repeats_head_branch = head_points_at_branch
                && name.kind == RefKind::LocalBranch
                && Some(name.short.as_str()) == head_branch;
            !repeats_head_branch
        })
        .collect()
}

pub fn ref_label(name: &RefName, head_branch: Option<&str>) -> String {
    match name.kind {
        RefKind::Head => match head_branch {
            Some(branch) => format!("{HEAD_REF_LABEL_PREFIX}{branch}"),
            None => String::from(HEAD_REF_LABEL),
        },
        RefKind::LocalBranch | RefKind::RemoteBranch => name.short.clone(),
        RefKind::Tag => format!("{TAG_REF_LABEL_PREFIX}{}", name.short),
    }
}

pub fn ref_label_color(kind: RefKind, theme: &Theme) -> Color {
    match kind {
        RefKind::Head => theme.text,
        RefKind::LocalBranch => theme.success,
        RefKind::RemoteBranch => theme.accent,
        RefKind::Tag => theme.warning,
    }
}

pub fn format_commit_date(timestamp: i64, utc_offset_minutes: i32) -> String {
    let shifted_seconds = timestamp + i64::from(utc_offset_minutes) * 60;
    let days = shifted_seconds.div_euclid(86_400);
    let seconds_of_day = shifted_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_date_from_days(days);
    let hour_of_day = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let (hour, meridiem) = twelve_hour_clock(hour_of_day);
    format!(
        "{day}/{month}/{year:02}, {hour}:{minute:02} {meridiem}",
        year = year.rem_euclid(100)
    )
}

fn twelve_hour_clock(hour_of_day: i64) -> (i64, &'static str) {
    match hour_of_day {
        0 => (12, "am"),
        1..=11 => (hour_of_day, "am"),
        12 => (12, "pm"),
        _ => (hour_of_day - 12, "pm"),
    }
}

fn civil_date_from_days(days: i64) -> (i64, i64, i64) {
    let shifted_days = days + 719_468;
    let era = shifted_days.div_euclid(146_097);
    let day_of_era = shifted_days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

pub fn commit_details_row_count(
    commit: &CommitSummary,
    head_branch: Option<&str>,
    containing_branches: Option<ContainingBranches<'_>>,
    details_width: u16,
) -> u16 {
    let refs_count =
        u16::try_from(renderable_ref_names(commit, head_branch).len()).unwrap_or(u16::MAX);
    let committed_by_count = u16::from(committer_differs_from_author(commit));
    let containing_branches_count = u16::from(containing_branches.is_some());
    COMMIT_DETAIL_SEPARATOR_HEIGHT
        .saturating_add(commit_subject_row_count(&commit.subject, details_width))
        .saturating_add(COMMIT_DETAIL_META_HEIGHT)
        .saturating_add(committed_by_count)
        .saturating_add(containing_branches_count)
        .saturating_add(refs_count)
}

pub fn commit_subject_row_count(subject: &str, width: u16) -> u16 {
    let wrapped = wrap_error_body_lines(subject, width).len();
    u16::try_from(wrapped.max(1))
        .unwrap_or(u16::MAX)
        .min(COMMIT_DETAIL_SUBJECT_MAX_ROWS)
}

pub fn commit_details_area(
    details: Rect,
    commit: Option<&CommitSummary>,
    head_branch: Option<&str>,
    containing_branches: Option<ContainingBranches<'_>>,
) -> Rect {
    let block_height = commit
        .map(|commit| {
            commit_details_row_count(commit, head_branch, containing_branches, details.width)
                .min(details.height)
        })
        .unwrap_or(0);
    Rect::new(
        details.x,
        details.y + details.height.saturating_sub(block_height),
        details.width,
        block_height,
    )
}

pub fn changed_files_area(
    details: Rect,
    commit: Option<&CommitSummary>,
    head_branch: Option<&str>,
    containing_branches: Option<ContainingBranches<'_>>,
) -> Rect {
    let commit_details = commit_details_area(details, commit, head_branch, containing_branches);
    Rect::new(
        details.x,
        details.y,
        details.width,
        commit_details.y.saturating_sub(details.y),
    )
}

pub fn changed_file_index_at(
    details: Rect,
    changed_files: &[ChangeEntry],
    commit: Option<&CommitSummary>,
    head_branch: Option<&str>,
    containing_branches: Option<ContainingBranches<'_>>,
    column: u16,
    row: u16,
) -> Option<usize> {
    let area = changed_files_area(details, commit, head_branch, containing_branches);
    if !crate::layout::rect_contains_cell(area, column, row) {
        return None;
    }
    let rows_y = area.y.saturating_add(FILES_HEADER_HEIGHT);
    let row_offset = usize::from(row.checked_sub(rows_y)?);
    match changed_file_rows(changed_files).get(row_offset) {
        Some(ChangedFileRow::File { index, .. }) => Some(*index),
        _ => None,
    }
}

pub fn render_commit_details(area: Rect, view: CommitDetailsView<'_>, buffer: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let theme = theme_palette(view.theme);
    let commit_details = commit_details_area(
        area,
        view.commit,
        view.head_branch,
        view.containing_branches,
    );
    render_changed_file_tree(
        changed_files_area(
            area,
            view.commit,
            view.head_branch,
            view.containing_branches,
        ),
        &view,
        &theme,
        buffer,
    );
    if let Some(commit) = view.commit {
        render_commit_detail_block(commit_details, commit, &view, &theme, buffer);
    }
}

fn render_changed_file_tree(
    area: Rect,
    view: &CommitDetailsView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    if view.changed_files.is_empty() {
        buffer.set_stringn(
            area.x,
            area.y,
            NO_FILES_LABEL,
            usize::from(area.width),
            Style::default().fg(theme.muted).bg(theme.background),
        );
        return;
    }
    render_files_header(area, view.changed_files.len(), theme, buffer);
    let rows_area = Rect::new(
        area.x,
        area.y + FILES_HEADER_HEIGHT,
        area.width,
        area.height.saturating_sub(FILES_HEADER_HEIGHT),
    );
    let rows = changed_file_rows(view.changed_files);
    let visible_rows = rows.len().min(usize::from(rows_area.height));
    for (row_offset, row) in rows.iter().take(visible_rows).enumerate() {
        let Ok(row_offset) = u16::try_from(row_offset) else {
            return;
        };
        let segments = changed_file_row_segments(row, view, theme);
        let background = theme.background;
        let row_y = rows_area.y + row_offset;
        buffer.set_style(
            Rect::new(rows_area.x, row_y, rows_area.width, 1),
            Style::default().fg(theme.text).bg(background),
        );
        write_row_segments(rows_area, row_y, &segments, background, view.caps, buffer);
        repaint_blank_cell_backgrounds(rows_area, row_y, theme, background, buffer);
    }
}

fn render_files_header(area: Rect, file_count: usize, theme: &Theme, buffer: &mut Buffer) {
    buffer.set_style(
        Rect::new(area.x, area.y, area.width, FILES_HEADER_HEIGHT),
        Style::default().fg(theme.text).bg(theme.background),
    );
    let _ = buffer.set_stringn(
        area.x,
        area.y,
        file_count_label(file_count),
        usize::from(area.width),
        Style::default().fg(theme.muted).bg(theme.background),
    );
    repaint_blank_cell_backgrounds(area, area.y, theme, theme.background, buffer);
}

fn changed_file_row_segments(
    row: &ChangedFileRow,
    view: &CommitDetailsView<'_>,
    theme: &Theme,
) -> Vec<RowSegment> {
    match row {
        ChangedFileRow::Directory {
            directory,
            file_count,
        } => vec![
            RowSegment::Text {
                text: IconGlyph::Chevron.character(view.caps).to_string(),
                color: theme.muted,
            },
            RowSegment::Pad(1),
            RowSegment::Text {
                text: directory.clone(),
                color: theme.text,
            },
            RowSegment::Pad(CHANGED_FILE_COUNT_GAP),
            RowSegment::Text {
                text: file_count_label(*file_count),
                color: theme.muted,
            },
        ],
        ChangedFileRow::File { index, indented } => {
            let entry = &view.changed_files[*index];
            let (file_name, directory) = split_changed_file_path(&entry.path);
            let mut segments = vec![RowSegment::Pad(if *indented {
                CHANGED_FILE_INDENT
            } else {
                0
            })];
            segments.push(RowSegment::Text {
                text: change_kind_letter(entry.kind).to_string(),
                color: change_kind_color(entry.kind, theme),
            });
            segments.push(RowSegment::Pad(CHANGED_FILE_STATUS_GAP));
            segments.push(RowSegment::Text {
                text: file_name,
                color: theme.text,
            });
            if !directory.is_empty() {
                segments.push(RowSegment::Pad(CHANGED_FILE_DIRECTORY_GAP));
                segments.push(RowSegment::Text {
                    text: directory,
                    color: theme.muted,
                });
            }
            segments
        }
    }
}

pub fn changed_file_rows(entries: &[ChangeEntry]) -> Vec<ChangedFileRow> {
    let mut directories: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let directory = parent_directory(&entry.path);
        match directories.iter_mut().find(|(name, _)| *name == directory) {
            Some((_, indices)) => indices.push(index),
            None => directories.push((directory, vec![index])),
        }
    }
    let mut rows = Vec::new();
    for (directory, indices) in directories {
        if indices.len() == 1 {
            rows.push(ChangedFileRow::File {
                index: indices[0],
                indented: false,
            });
            continue;
        }
        rows.push(ChangedFileRow::Directory {
            directory,
            file_count: indices.len(),
        });
        rows.extend(indices.into_iter().map(|index| ChangedFileRow::File {
            index,
            indented: true,
        }));
    }
    rows
}

fn parent_directory(path: &str) -> String {
    match path.rfind('/') {
        Some(separator) => path[..separator].to_string(),
        None => String::new(),
    }
}

fn split_changed_file_path(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(separator) => (
            path[separator + 1..].to_string(),
            path[..separator].to_string(),
        ),
        None => (path.to_string(), String::new()),
    }
}

struct CommitDetailRow {
    drop_rank: u8,
    text: String,
    color: Color,
}

fn render_commit_detail_block(
    area: Rect,
    commit: &CommitSummary,
    view: &CommitDetailsView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    render_commit_detail_separator(area, view.caps, theme, buffer);
    let mut rows = commit_detail_rows(commit, view, theme, area.width);
    trim_commit_detail_rows(
        &mut rows,
        usize::from(area.height.saturating_sub(COMMIT_DETAIL_SEPARATOR_HEIGHT)),
    );
    for (row_offset, row) in rows.into_iter().enumerate() {
        let Ok(row_offset) = u16::try_from(row_offset) else {
            return;
        };
        let row_y = area.y + COMMIT_DETAIL_SEPARATOR_HEIGHT + row_offset;
        if row_y >= area.bottom() {
            return;
        }
        buffer.set_style(
            Rect::new(area.x, row_y, area.width, 1),
            Style::default().fg(theme.text).bg(theme.background),
        );
        let _ = buffer.set_stringn(
            area.x,
            row_y,
            sanitize_display_string(&row.text, area.x, view.caps),
            usize::from(area.width),
            Style::default().fg(row.color).bg(theme.background),
        );
        repaint_blank_cell_backgrounds(area, row_y, theme, theme.background, buffer);
    }
}

fn trim_commit_detail_rows(rows: &mut Vec<CommitDetailRow>, budget: usize) {
    while rows.len() > budget {
        let Some(droppable) = last_droppable_commit_detail_row(rows) else {
            return;
        };
        if rows[droppable].drop_rank == COMMIT_DETAIL_KEEP_RANK {
            return;
        }
        rows.remove(droppable);
    }
}

fn last_droppable_commit_detail_row(rows: &[CommitDetailRow]) -> Option<usize> {
    let mut droppable: Option<usize> = None;
    for (index, row) in rows.iter().enumerate() {
        let rank_of_best = droppable.map(|best| rows[best].drop_rank);
        if rank_of_best.is_none_or(|rank| row.drop_rank <= rank) {
            droppable = Some(index);
        }
    }
    droppable
}

fn render_commit_detail_separator(
    area: Rect,
    caps: CapsProfile,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    let separator = line_glyphs(caps)
        .horizontal
        .to_string()
        .repeat(usize::from(area.width));
    let style = Style::default().fg(theme.border).bg(theme.background);
    buffer.set_style(Rect::new(area.x, area.y, area.width, 1), style);
    let _ = buffer.set_stringn(area.x, area.y, &separator, usize::from(area.width), style);
}

fn commit_detail_rows(
    commit: &CommitSummary,
    view: &CommitDetailsView<'_>,
    theme: &Theme,
    width: u16,
) -> Vec<CommitDetailRow> {
    let mut rows: Vec<CommitDetailRow> = wrap_error_text(
        &commit.subject,
        width,
        usize::from(COMMIT_DETAIL_SUBJECT_MAX_ROWS),
        view.caps,
    )
    .into_iter()
    .map(|line| CommitDetailRow {
        drop_rank: COMMIT_DETAIL_KEEP_RANK,
        text: line,
        color: theme.accent,
    })
    .collect();
    rows.push(CommitDetailRow {
        drop_rank: COMMIT_DETAIL_KEEP_RANK,
        text: format!(
            "{}{}{}",
            commit_detail_short_id(commit),
            COMMIT_DETAIL_ID_GAP,
            commit.author_name
        ),
        color: theme.text,
    });
    rows.extend([
        CommitDetailRow {
            drop_rank: AUTHOR_EMAIL_ROW_RANK,
            text: commit.author_email.clone(),
            color: theme.muted,
        },
        CommitDetailRow {
            drop_rank: COMMIT_DATE_ROW_RANK,
            text: format_commit_date(commit.author_timestamp, view.utc_offset_minutes),
            color: theme.muted,
        },
    ]);
    if committer_differs_from_author(commit) {
        rows.push(CommitDetailRow {
            drop_rank: COMMITTED_BY_ROW_RANK,
            text: format!("{COMMITTED_BY_LABEL_PREFIX}{}", commit.committer_name),
            color: theme.muted,
        });
    }
    if let Some(containing_branches) = view.containing_branches {
        rows.push(CommitDetailRow {
            drop_rank: CONTAINING_BRANCHES_ROW_RANK,
            text: truncate_with_caps_ellipsis(
                &containing_branches_label(containing_branches),
                width,
                view.caps,
            ),
            color: theme.muted,
        });
    }
    rows.extend(
        renderable_ref_names(commit, view.head_branch)
            .into_iter()
            .map(|name| CommitDetailRow {
                drop_rank: REFERENCE_ROW_RANK,
                text: ref_label(name, view.head_branch),
                color: ref_label_color(name.kind, theme),
            }),
    );
    rows
}

fn committer_differs_from_author(commit: &CommitSummary) -> bool {
    commit.committer_name != commit.author_name || commit.committer_email != commit.author_email
}

pub fn containing_branch_label(branch: &BranchEntry) -> &str {
    branch
        .name
        .strip_prefix(REMOTE_REF_PREFIX)
        .unwrap_or(branch.name.as_str())
}

fn containing_branches_label(containing_branches: ContainingBranches<'_>) -> String {
    let ContainingBranches::Listed(branches) = containing_branches else {
        return String::from(CONTAINING_BRANCHES_UNAVAILABLE_LABEL);
    };
    let count = branches.len();
    if count == 0 {
        return containing_branches_count_label(0);
    }
    let listed: Vec<&str> = branches
        .iter()
        .take(MAX_LISTED_CONTAINING_BRANCHES)
        .map(containing_branch_label)
        .collect();
    let head = if count == 1 {
        String::from(CONTAINING_BRANCHES_SINGULAR_LABEL)
    } else {
        containing_branches_count_label(count)
    };
    let mut label = format!("{head}: {}", listed.join(REFS_SEPARATOR));
    if let Some(remaining) = count.checked_sub(MAX_LISTED_CONTAINING_BRANCHES) {
        if remaining > 0 {
            label.push_str(&format!(" +{remaining}"));
        }
    }
    label
}

fn containing_branches_count_label(count: usize) -> String {
    format!("In {count} branches")
}

pub fn commit_detail_short_id(commit: &CommitSummary) -> String {
    commit
        .short_id
        .chars()
        .take(COMMIT_DETAIL_SHORT_ID_LENGTH)
        .collect()
}

#[cfg(test)]
mod log_render_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use gitdeck_core::{BranchEntry, ChangeEntry, ChangeKind, CommitSummary, RefKind, RefName};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    use super::{
        changed_file_rows, changed_files_area, commit_detail_short_id, commit_details_area,
        commit_details_row_count, commit_ref_labels, commit_subject_row_count, format_commit_date,
        ref_label, render_commit_details, render_commit_list, ChangedFileRow, CommitDetailsView,
        CommitListView, ContainingBranches, COMMITTED_BY_LABEL_PREFIX,
        CONTAINING_BRANCHES_UNAVAILABLE_LABEL, DATE_AUTHOR_GAP, DATE_COLUMN_WIDTH,
        FILES_HEADER_HEIGHT, GRAPH_COLUMN_WIDTH, NO_COMMITS_LABEL, NO_FILES_LABEL, SUBJECT_COLUMN,
    };
    use crate::theme::theme_palette;
    use gitdeck_core::assign_lanes;

    const BACKGROUND: Color = Color::Rgb(0x1E, 0x1F, 0x22);
    const TEST_DETAILS_WIDTH: u16 = 40;
    const LIST_RENDER_WIDTH: u16 = 64;
    const SELECTION: Color = Color::Rgb(0x2E, 0x43, 0x6E);
    const INACTIVE_SELECTION: Color = Color::Rgb(0x39, 0x3B, 0x40);

    fn reference(kind: RefKind, short: &str) -> RefName {
        RefName {
            full: format!("refs/heads/{short}"),
            short: short.to_string(),
            kind,
        }
    }

    fn commit(index: usize, subject: &str) -> CommitSummary {
        CommitSummary {
            id: format!("{index:040}"),
            short_id: format!("{index:07}"),
            subject: subject.to_string(),
            author_name: String::from("gitdeck-fixture"),
            author_email: String::from("fixture@gitdeck.invalid"),
            author_timestamp: 1_577_836_800 + i64::try_from(index).unwrap_or(0) * 60,
            committer_name: String::from("gitdeck-fixture"),
            committer_email: String::from("fixture@gitdeck.invalid"),
            committer_timestamp: 1_577_836_800 + i64::try_from(index).unwrap_or(0) * 60,
            parent_ids: Vec::new(),
            refs: Vec::new(),
        }
    }

    fn changed_entry(path: &str, kind: ChangeKind) -> ChangeEntry {
        ChangeEntry::unstaged(path.to_string(), None, kind)
    }

    fn row_text(buffer: &Buffer, row: u16) -> String {
        let mut line = String::new();
        for column in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((column, row)) {
                line.push_str(cell.symbol());
            }
        }
        line.trim_end().to_string()
    }

    fn row_from_column(text: &str, column: u16) -> String {
        text.chars().skip(usize::from(column)).collect()
    }

    fn render_list(commits: &[CommitSummary], selected: Option<usize>, focused: bool) -> Buffer {
        let lanes = assign_lanes(commits);
        let mut buffer = Buffer::empty(Rect::new(0, 0, LIST_RENDER_WIDTH, 4));
        render_commit_list(
            buffer.area,
            CommitListView {
                commits,
                lanes: &lanes,
                selected_index: selected,
                scroll: 0,
                focused,
                head_branch: Some("main"),
                utc_offset_minutes: 0,
                screen_width: 160,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        buffer
    }

    fn render_list_at_width(commits: &[CommitSummary], width: u16, author_name: &str) -> Buffer {
        let commits: Vec<CommitSummary> = commits
            .iter()
            .map(|commit| CommitSummary {
                author_name: author_name.to_string(),
                ..commit.clone()
            })
            .collect();
        let lanes = assign_lanes(&commits);
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, 1));
        render_commit_list(
            buffer.area,
            CommitListView {
                commits: &commits,
                lanes: &lanes,
                selected_index: None,
                scroll: 0,
                focused: false,
                head_branch: None,
                utc_offset_minutes: 0,
                screen_width: 160,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        buffer
    }

    fn linear_history(subjects: &[&str]) -> Vec<CommitSummary> {
        let mut commits: Vec<CommitSummary> = subjects
            .iter()
            .enumerate()
            .map(|(index, subject)| commit(index, subject))
            .collect();
        for index in 0..commits.len().saturating_sub(1) {
            let parent_id = commits[index + 1].id.clone();
            commits[index].parent_ids = vec![parent_id];
        }
        commits
    }

    #[test]
    fn documented_example_renders_without_zero_padding() {
        assert_eq!(
            format_commit_date(1_781_722_560, 0),
            String::from("17/6/26, 6:56 pm")
        );
    }

    #[test]
    fn midnight_noon_and_late_evening_use_the_twelve_hour_clock() {
        assert_eq!(
            format_commit_date(1_577_836_800, 0),
            String::from("1/1/20, 12:00 am")
        );
        assert_eq!(
            format_commit_date(1_577_837_100, 0),
            String::from("1/1/20, 12:05 am")
        );
        assert_eq!(
            format_commit_date(1_577_840_820, 0),
            String::from("1/1/20, 1:07 am")
        );
        assert_eq!(
            format_commit_date(1_577_880_000, 0),
            String::from("1/1/20, 12:00 pm")
        );
        assert_eq!(
            format_commit_date(1_577_923_140, 0),
            String::from("1/1/20, 11:59 pm")
        );
    }

    #[test]
    fn utc_offset_moves_the_rendered_date_across_the_day_boundary() {
        assert_eq!(
            format_commit_date(1_577_838_600, 0),
            String::from("1/1/20, 12:30 am")
        );
        assert_eq!(
            format_commit_date(1_577_838_600, -60),
            String::from("31/12/19, 11:30 pm")
        );
        assert_eq!(
            format_commit_date(1_577_835_000, 60),
            String::from("1/1/20, 12:30 am")
        );
    }

    #[test]
    fn leap_day_keeps_its_calendar_date() {
        assert_eq!(
            format_commit_date(1_709_197_500, 0),
            String::from("29/2/24, 9:05 am")
        );
    }

    #[test]
    fn ref_labels_follow_the_documented_formats() {
        let mut head = commit(0, "subject");
        head.refs = vec![
            reference(RefKind::Head, "HEAD"),
            reference(RefKind::LocalBranch, "main"),
            reference(RefKind::RemoteBranch, "origin/main"),
            reference(RefKind::Tag, "v1"),
        ];
        assert_eq!(
            ref_label(&head.refs[0], Some("main")),
            String::from("HEAD -> main")
        );
        assert_eq!(ref_label(&head.refs[0], None), String::from("HEAD"));
        assert_eq!(ref_label(&head.refs[1], Some("main")), String::from("main"));
        assert_eq!(
            ref_label(&head.refs[2], Some("main")),
            String::from("origin/main")
        );
        assert_eq!(
            ref_label(&head.refs[3], Some("main")),
            String::from("tag: v1")
        );
        let labels = commit_ref_labels(&head, Some("main"));
        assert_eq!(
            labels
                .iter()
                .map(|label| label.text.as_str())
                .collect::<Vec<&str>>(),
            vec!["HEAD -> main", "origin/main", "tag: v1"],
            "the local branch HEAD points at is not repeated"
        );
        assert_eq!(
            labels
                .iter()
                .map(|label| label.has_following_label)
                .collect::<Vec<bool>>(),
            vec![true, true, false]
        );
        let detached_labels = commit_ref_labels(&head, None);
        assert_eq!(
            detached_labels
                .iter()
                .map(|label| label.text.as_str())
                .collect::<Vec<&str>>(),
            vec!["HEAD", "main", "origin/main", "tag: v1"],
            "a detached head keeps every local branch label"
        );
    }

    #[test]
    fn changed_file_rows_group_by_parent_directory_and_indent_members() {
        let entries = vec![
            changed_entry("src/app.rs", ChangeKind::Modified),
            changed_entry("src/main.rs", ChangeKind::Added),
            changed_entry("docs/readme.md", ChangeKind::Deleted),
            changed_entry("README.md", ChangeKind::Modified),
        ];
        assert_eq!(
            changed_file_rows(&entries),
            vec![
                ChangedFileRow::Directory {
                    directory: String::from("src"),
                    file_count: 2,
                },
                ChangedFileRow::File {
                    index: 0,
                    indented: true,
                },
                ChangedFileRow::File {
                    index: 1,
                    indented: true,
                },
                ChangedFileRow::File {
                    index: 2,
                    indented: false,
                },
                ChangedFileRow::File {
                    index: 3,
                    indented: false,
                },
            ]
        );
    }

    #[test]
    fn single_file_history_renders_without_a_group_header() {
        let entries = vec![changed_entry("docs/readme.md", ChangeKind::Deleted)];
        assert_eq!(
            changed_file_rows(&entries),
            vec![ChangedFileRow::File {
                index: 0,
                indented: false,
            }]
        );
    }

    #[test]
    fn details_block_hangs_from_the_panel_bottom_and_leaves_the_rest_to_the_file_tree() {
        for height in [1u16, 5, 20, 44, 45] {
            let details = Rect::new(0, 2, 40, height);
            let commit = commit(0, "subject");
            let block_rows = commit_details_row_count(&commit, None, None, TEST_DETAILS_WIDTH);
            let changed_files = changed_files_area(details, Some(&commit), None, None);
            let commit_details = commit_details_area(details, Some(&commit), None, None);
            assert_eq!(
                commit_details.height,
                block_rows.min(height),
                "height {height} clamps the block to the panel"
            );
            assert_eq!(
                commit_details.bottom(),
                details.bottom(),
                "height {height} pins the block to the panel bottom"
            );
            assert_eq!(changed_files.y, details.y);
            assert_eq!(
                u32::from(changed_files.height) + u32::from(commit_details.height),
                u32::from(height),
                "height {height} keeps every row"
            );
        }
        let details = Rect::new(0, 2, 40, 20);
        assert_eq!(
            changed_files_area(details, None, None, None).height,
            20,
            "a panel without a selected commit gives every row to the file tree"
        );
        assert_eq!(commit_details_area(details, None, None, None).height, 0);
    }

    #[test]
    fn details_row_count_follows_the_subject_meta_and_reference_rows() {
        let mut head = commit(0, "subject");
        assert_eq!(
            commit_details_row_count(&head, None, None, TEST_DETAILS_WIDTH),
            5
        );
        head.refs = vec![
            reference(RefKind::Head, "HEAD"),
            reference(RefKind::Tag, "v1"),
        ];
        assert_eq!(
            commit_details_row_count(&head, None, None, TEST_DETAILS_WIDTH),
            7
        );
    }

    #[test]
    fn subject_starts_at_column_six_and_the_author_ends_at_the_last_column() {
        let commits = vec![commit(0, "fix: render the log")];
        let buffer = render_list(&commits, Some(0), false);
        let first_row = row_text(&buffer, 0);
        assert_eq!(
            buffer.cell((0, 0)).expect("lane cell").symbol(),
            "●",
            "the graph node sits in column 0"
        );
        assert!(
            row_from_column(&first_row, SUBJECT_COLUMN).starts_with("fix: render the log"),
            "row {first_row:?} starts the subject at column {SUBJECT_COLUMN}"
        );
        assert!(
            first_row.ends_with("gitdeck-fixture"),
            "row {first_row:?} must end with the author"
        );
        assert!(first_row.contains("1/1/20, 12:00 am"));
    }

    #[test]
    fn date_sits_left_of_the_author_with_two_column_gaps() {
        let commits = vec![commit(0, "subject")];
        let buffer = render_list(&commits, Some(0), false);
        let width = buffer.area.width;
        let author_start = width - u16::try_from("gitdeck-fixture".len()).expect("author width");
        let date_field_start = author_start - DATE_AUTHOR_GAP - DATE_COLUMN_WIDTH;
        let date_text = format_commit_date(commits[0].author_timestamp, 0);
        let date_width = u16::try_from(date_text.len()).expect("date width");
        let date_start = date_field_start + DATE_COLUMN_WIDTH - date_width;
        let date_cell = buffer
            .cell((date_start, 0))
            .expect("the date field start cell")
            .symbol()
            .to_string();
        assert_eq!(date_cell, date_text.chars().next().unwrap().to_string());
        for column in date_field_start + DATE_COLUMN_WIDTH..author_start {
            let symbol = buffer.cell((column, 0)).expect("gap cell").symbol();
            assert_eq!(symbol, " ", "column {column} is the author gap");
        }
    }

    #[test]
    fn narrow_rows_drop_the_author_before_the_date() {
        let commits = vec![commit(0, "subject")];
        let author_dropped = render_list_at_width(&commits, 40, "an-extremely-long-author-name");
        let row = row_text(&author_dropped, 0);
        assert!(
            !row.contains("an-extremely-long-author-name"),
            "row {row:?} keeps no room for the author"
        );
        assert!(
            row.contains("1/1/20, 12:00 am"),
            "row {row:?} keeps the date"
        );
        let date_dropped = render_list_at_width(&commits, 20, "ab");
        let row = row_text(&date_dropped, 0);
        assert!(row.contains("ab"), "row {row:?} keeps the author");
        assert!(
            !row.contains("1/1/20"),
            "row {row:?} drops the date before the subject"
        );
    }

    #[test]
    fn linear_history_draws_a_single_lane_of_nodes() {
        let commits = linear_history(&["newest", "middle", "oldest"]);
        let buffer = render_list(&commits, None, false);
        for row in 0..3u16 {
            assert_eq!(
                buffer.cell((0, row)).expect("lane cell").symbol(),
                "●",
                "row {row} lane 0"
            );
            assert_eq!(
                buffer.cell((2, row)).expect("lane cell").symbol(),
                " ",
                "row {row} keeps the second lane empty"
            );
        }
    }

    #[test]
    fn diverged_history_draws_corners_and_tees() {
        let mut merge = commit(0, "merge");
        let mut left = commit(1, "left");
        let mut right = commit(2, "right");
        let base = commit(3, "base");
        merge.parent_ids = vec![left.id.clone(), right.id.clone()];
        left.parent_ids = vec![base.id.clone()];
        right.parent_ids = vec![base.id.clone()];
        let commits = vec![merge, left, right, base];
        let buffer = render_list(&commits, None, false);
        assert_eq!(buffer.cell((0, 0)).expect("branch tee").symbol(), "┬");
        assert_eq!(buffer.cell((1, 0)).expect("branch link").symbol(), "─");
        assert_eq!(buffer.cell((2, 0)).expect("branch corner").symbol(), "╮");
        assert_eq!(buffer.cell((0, 3)).expect("merge tee").symbol(), "┴");
        assert_eq!(buffer.cell((2, 3)).expect("merge corner").symbol(), "╯");
    }

    #[test]
    fn lanes_beyond_the_third_column_are_not_drawn() {
        let mut octopus = commit(0, "octopus");
        let roots: Vec<CommitSummary> = (1..5).map(|index| commit(index, "root")).collect();
        octopus.parent_ids = roots.iter().map(|commit| commit.id.clone()).collect();
        let mut commits = vec![octopus];
        commits.extend(roots);
        let buffer = render_list(&commits, None, false);
        for column in GRAPH_COLUMN_WIDTH..GRAPH_COLUMN_WIDTH + 4 {
            let symbol = buffer.cell((column, 0)).expect("subject cell").symbol();
            assert!(
                !"●│╮╯┬┴─".contains(symbol),
                "column {column} holds the graph glyph {symbol:?}"
            );
        }
        assert_eq!(buffer.cell((4, 0)).expect("third lane").symbol(), "╮");
    }

    #[test]
    fn refs_prefix_precedes_the_subject_without_extra_spaces() {
        let mut head = commit(0, "subject");
        head.refs = vec![reference(RefKind::Head, "HEAD")];
        let buffer = render_list(&[head], Some(0), false);
        let first_row = row_text(&buffer, 0);
        assert!(
            row_from_column(&first_row, SUBJECT_COLUMN).starts_with("HEAD -> main  subje"),
            "row {first_row:?}"
        );
    }

    #[test]
    fn selected_commit_row_switches_background_with_the_focus() {
        let commits = vec![commit(0, "subject"), commit(1, "older")];
        let focused = render_list(&commits, Some(0), true);
        assert_eq!(focused.cell((0, 0)).expect("cell").bg, SELECTION);
        let unfocused = render_list(&commits, Some(0), false);
        assert_eq!(unfocused.cell((0, 0)).expect("cell").bg, INACTIVE_SELECTION);
        assert_eq!(focused.cell((0, 1)).expect("cell").bg, BACKGROUND);
        let second_selected = render_list(&commits, Some(1), true);
        assert_eq!(second_selected.cell((0, 0)).expect("cell").bg, BACKGROUND);
        assert_eq!(second_selected.cell((0, 1)).expect("cell").bg, SELECTION);
    }

    #[test]
    fn empty_history_renders_the_no_commits_placeholder() {
        let buffer = render_list(&[], None, false);
        assert_eq!(row_text(&buffer, 0), NO_COMMITS_LABEL);
        assert_eq!(
            buffer.cell((0, 0)).expect("cell").fg,
            theme_palette(ThemeId::JbDark).muted
        );
    }

    #[test]
    fn only_the_rows_inside_the_scroll_window_are_rendered() {
        let commits: Vec<CommitSummary> = (0..10).map(|index| commit(index, "subject")).collect();
        let lanes = assign_lanes(&commits);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 3));
        render_commit_list(
            buffer.area,
            CommitListView {
                commits: &commits,
                lanes: &lanes,
                selected_index: Some(5),
                scroll: 5,
                focused: true,
                head_branch: None,
                utc_offset_minutes: 0,
                screen_width: 160,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        for (row, expected_index) in [(0u16, 5usize), (1, 6), (2, 7)] {
            let line = row_text(&buffer, row);
            let expected_date = format_commit_date(commits[expected_index].author_timestamp, 0);
            assert!(
                line.contains(&expected_date),
                "row {row} of {line:?} does not hold {expected_date:?}"
            );
        }
    }

    #[test]
    fn commit_details_render_author_email_date_and_references_in_order() {
        let mut head = commit(0, "subject");
        head.refs = vec![
            reference(RefKind::Head, "HEAD"),
            reference(RefKind::LocalBranch, "main"),
            reference(RefKind::RemoteBranch, "origin/main"),
            reference(RefKind::Tag, "v1"),
        ];
        let changed_files = vec![changed_entry("src/app.rs", ChangeKind::Modified)];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 20));
        render_commit_details(
            Rect::new(0, 0, 40, 20),
            CommitDetailsView {
                commit: Some(&head),
                changed_files: &changed_files,
                head_branch: Some("main"),
                containing_branches: None,
                utc_offset_minutes: 0,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        let detail_top =
            commit_details_area(Rect::new(0, 0, 40, 20), Some(&head), Some("main"), None).y;
        assert_eq!(
            row_text(&buffer, detail_top),
            "─".repeat(40),
            "the separator opens the block"
        );
        assert_eq!(row_text(&buffer, detail_top + 1), "subject");
        assert_eq!(
            row_text(&buffer, detail_top + 2),
            format!("{} gitdeck-fixture", commit_detail_short_id(&head))
        );
        assert_eq!(row_text(&buffer, detail_top + 3), "fixture@gitdeck.invalid");
        assert_eq!(
            row_text(&buffer, detail_top + 4),
            format_commit_date(head.author_timestamp, 0)
        );
        assert_eq!(row_text(&buffer, detail_top + 5), "HEAD -> main");
        assert_eq!(row_text(&buffer, detail_top + 6), "origin/main");
        assert_eq!(row_text(&buffer, detail_top + 7), "tag: v1");
        assert_eq!(row_text(&buffer, 0), "1 file");
        assert_eq!(row_text(&buffer, FILES_HEADER_HEIGHT), "M  app.rs  src");
    }

    #[test]
    fn committed_by_row_appears_only_when_committer_differs() {
        let same_identity = commit(0, "subject");
        let mut other_committer = commit(0, "subject");
        other_committer.committer_name = String::from("release-bot");
        other_committer.committer_email = String::from("bot@gitdeck.invalid");
        let (same_buffer, same_top) = render_details(&same_identity, None, 40, 20);
        assert!(
            !buffer_holds_text(&same_buffer, COMMITTED_BY_LABEL_PREFIX),
            "an identical committer leaves no committed by row"
        );
        assert_eq!(
            commit_details_row_count(&same_identity, None, None, TEST_DETAILS_WIDTH),
            5
        );
        let (other_buffer, other_top) = render_details(&other_committer, None, 40, 20);
        assert_eq!(
            row_text(&other_buffer, other_top + 5),
            "committed by release-bot",
            "a different committer adds the row under the date"
        );
        assert_eq!(row_text(&same_buffer, same_top + 1), "subject");
        assert_eq!(
            commit_details_row_count(&other_committer, None, None, TEST_DETAILS_WIDTH),
            6
        );
    }

    #[test]
    fn branches_row_lists_up_to_three_names_then_a_plus_count() {
        let head = commit(0, "subject");
        let branches = vec![
            branch_entry("main", RefKind::LocalBranch),
            branch_entry("alpha", RefKind::LocalBranch),
            branch_entry("feature", RefKind::LocalBranch),
            branch_entry("origin/main", RefKind::RemoteBranch),
            branch_entry("refs/remotes/origin/release", RefKind::RemoteBranch),
        ];
        let containing_branches = Some(ContainingBranches::Listed(&branches));
        let (buffer, top) = render_details(&head, containing_branches, 60, 20);
        assert_eq!(
            row_text(&buffer, top + 5),
            "In 5 branches: main, alpha, feature +2",
            "three names are listed and the rest is counted"
        );
        assert_eq!(
            commit_details_row_count(&head, None, containing_branches, TEST_DETAILS_WIDTH),
            6,
            "the branches row counts toward the block height"
        );
    }

    #[test]
    fn branches_row_uses_singular_for_one_branch() {
        let head = commit(0, "subject");
        let branches = vec![branch_entry("main", RefKind::LocalBranch)];
        let (buffer, top) =
            render_details(&head, Some(ContainingBranches::Listed(&branches)), 40, 20);
        assert_eq!(row_text(&buffer, top + 5), "In 1 branch: main");
    }

    #[test]
    fn branches_row_reports_an_empty_result() {
        let head = commit(0, "subject");
        let (buffer, top) = render_details(&head, Some(ContainingBranches::Listed(&[])), 40, 20);
        assert_eq!(row_text(&buffer, top + 5), "In 0 branches");
        let (unavailable_buffer, unavailable_top) =
            render_details(&head, Some(ContainingBranches::Unavailable), 40, 20);
        assert_eq!(
            row_text(&unavailable_buffer, unavailable_top + 5),
            CONTAINING_BRANCHES_UNAVAILABLE_LABEL
        );
    }

    #[test]
    fn short_panel_drops_branches_before_committed_by() {
        let mut head = commit(0, "subject");
        head.committer_name = String::from("release-bot");
        head.committer_email = String::from("bot@gitdeck.invalid");
        let branches = vec![branch_entry("main", RefKind::LocalBranch)];
        let containing_branches = Some(ContainingBranches::Listed(&branches));
        let (branches_row, branches_top) = render_details(&head, containing_branches, 40, 7);
        assert_eq!(
            row_text(&branches_row, branches_top + 6),
            "In 1 branch: main",
            "the full block lists the branches under the committed by row"
        );
        let (five_rows, five_top) = render_details(&head, containing_branches, 40, 6);
        assert_eq!(
            row_text(&five_rows, five_top + 5),
            "committed by release-bot",
            "the committed by row survives while the branches row goes"
        );
        assert!(!buffer_holds_text(&five_rows, "In 1 branch"));
        let (four_rows, four_top) = render_details(&head, containing_branches, 40, 5);
        assert_eq!(
            row_text(&four_rows, four_top + 4),
            format_commit_date(head.author_timestamp, 0),
            "the date row stays while the committed by row goes"
        );
        assert!(!buffer_holds_text(&four_rows, COMMITTED_BY_LABEL_PREFIX));
        let (three_rows, three_top) = render_details(&head, containing_branches, 40, 4);
        assert_eq!(row_text(&three_rows, three_top + 1), "subject");
        assert_eq!(
            row_text(&three_rows, three_top + 2),
            format!("{} gitdeck-fixture", commit_detail_short_id(&head)),
            "the subject and the short id survive the squeeze"
        );
        assert_eq!(
            row_text(&three_rows, three_top + 3),
            "fixture@gitdeck.invalid",
            "the email goes after the date"
        );
        assert!(!buffer_holds_text(
            &three_rows,
            &format_commit_date(head.author_timestamp, 0)
        ));
    }

    fn buffer_holds_text(buffer: &Buffer, text: &str) -> bool {
        (0..buffer.area.height).any(|row| row_text(buffer, row).contains(text))
    }

    fn branch_entry(name: &str, kind: RefKind) -> BranchEntry {
        BranchEntry {
            name: name.to_string(),
            kind,
            is_head: false,
            upstream: None,
        }
    }

    fn render_details(
        commit: &CommitSummary,
        containing_branches: Option<ContainingBranches<'_>>,
        width: u16,
        height: u16,
    ) -> (Buffer, u16) {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        render_commit_details(
            buffer.area,
            CommitDetailsView {
                commit: Some(commit),
                changed_files: &[],
                head_branch: None,
                containing_branches,
                utc_offset_minutes: 0,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        let top = commit_details_area(buffer.area, Some(commit), None, containing_branches).y;
        (buffer, top)
    }

    #[test]
    fn the_details_subject_wraps_across_lines() {
        let head = commit(0, "a subject that is far too long for the pane");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 20));
        render_commit_details(
            Rect::new(0, 0, 20, 20),
            CommitDetailsView {
                commit: Some(&head),
                changed_files: &[],
                head_branch: None,
                containing_branches: None,
                utc_offset_minutes: 0,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        let block = commit_details_area(Rect::new(0, 0, 20, 20), Some(&head), None, None);
        let first = row_text(&buffer, block.y + 1);
        let second = row_text(&buffer, block.y + 2);
        assert_eq!(first, "a subject that is", "first subject row {first:?}");
        assert_eq!(
            second, "far too long for the",
            "second subject row {second:?}"
        );
        assert!(!first.contains('…'), "first subject row {first:?}");
        assert!(!second.contains('…'), "second subject row {second:?}");
    }

    #[test]
    fn details_wrapping_terminates_on_pathological_input() {
        let head = commit(0, &"x".repeat(400));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 20));
        render_commit_details(
            Rect::new(0, 0, 12, 20),
            CommitDetailsView {
                commit: Some(&head),
                changed_files: &[],
                head_branch: None,
                containing_branches: None,
                utc_offset_minutes: 0,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        let subject_rows = commit_subject_row_count(&head.subject, 1);
        assert_eq!(subject_rows, 3, "a one-column pane keeps the row cap");
        let block = commit_details_area(Rect::new(0, 0, 12, 20), Some(&head), None, None);
        assert!(
            block.y < block.bottom(),
            "the block keeps room for the subject"
        );
        let widest = (block.y..block.bottom())
            .map(|row| row_text(&buffer, row).chars().count())
            .max()
            .unwrap_or(0);
        assert!(widest <= 12, "no detail row overflows the pane width");
    }

    #[test]
    fn commit_details_without_changed_files_render_the_placeholder() {
        let head = commit(0, "merge");
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 12));
        render_commit_details(
            Rect::new(0, 0, 40, 12),
            CommitDetailsView {
                commit: Some(&head),
                changed_files: &[],
                head_branch: Some("main"),
                containing_branches: None,
                utc_offset_minutes: 0,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(row_text(&buffer, 0), NO_FILES_LABEL);
        let author_row =
            commit_details_area(Rect::new(0, 0, 40, 12), Some(&head), Some("main"), None).y + 2;
        assert!(
            row_text(&buffer, author_row).starts_with("0000000"),
            "the author row sits under the subject"
        );
    }

    #[test]
    fn detail_short_id_is_seven_characters() {
        let head = commit(0, "subject");
        assert_eq!(commit_detail_short_id(&head), "0000000");
    }
}
