use gitdeck_core::{PaneId, Size};
use ratatui::layout::Rect;

pub const TOP_BAR_HEIGHT: u16 = 1;
pub const STATUS_BAR_HEIGHT: u16 = 1;
pub const ACTIVITY_RAIL_WIDTH: u16 = 4;
pub const DIVIDER_WIDTH: u16 = 1;
pub const BOTTOM_RULE_HEIGHT: u16 = 1;
pub const CHANGES_DEFAULT_WIDTH: u16 = 34;
pub const CHANGES_MIN_WIDTH: u16 = 24;
pub const CHANGES_MAX_WIDTH: u16 = 56;
pub const CHANGES_WIDTH_PERCENT: u16 = 22;
pub const DETAILS_DEFAULT_WIDTH: u16 = 40;
pub const DETAILS_MIN_WIDTH: u16 = 24;
pub const DETAILS_DEFAULT_MAX_WIDTH: u16 = 72;
pub const DETAILS_WIDTH_PERCENT: u16 = 28;
pub const DETAILS_MAX_WIDTH_PERCENT: u16 = 60;
pub const WIDE_LAYOUT_MIN_SCREEN_WIDTH: u16 = 160;
pub const COMMITS_FLOOR_WIDTH: u16 = 10;
pub const DETAILS_HIDDEN_BELOW_SCREEN_WIDTH: u16 = 100;
pub const DETAILS_WIDTH_HIDDEN: u16 = 0;
pub const LOG_RAIL_WIDTH: u16 = 3;
pub const LOG_RAIL_GAP: u16 = 1;
const LOG_TABS_HEIGHT: u16 = 1;
const LOG_FILTERS_HEIGHT: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DividerId {
    ChangesLog,
    CommitsDetails,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaneLayout {
    pub top_bar: Rect,
    pub activity_rail: Rect,
    pub changes: Rect,
    pub changes_log_divider: Rect,
    pub log_area: Rect,
    pub log_tabs: Rect,
    pub log_filters: Rect,
    pub log_rail: Rect,
    pub commits: Rect,
    pub commits_details_divider: Rect,
    pub details: Rect,
    pub status_bar: Rect,
}

impl PaneLayout {
    pub fn pane_rect(self, pane: PaneId) -> Rect {
        match pane {
            PaneId::Changes => self.changes,
            PaneId::Log => self.commits,
            PaneId::Details => self.details,
        }
    }

    pub fn divider_rect(self, divider: DividerId) -> Rect {
        match divider {
            DividerId::ChangesLog => self.changes_log_divider,
            DividerId::CommitsDetails => self.commits_details_divider,
        }
    }

    pub fn log_content_rect(self) -> Rect {
        Rect::new(
            self.log_area.x,
            self.commits.y,
            self.log_area.width,
            self.commits.height,
        )
    }

    pub fn rows_above_bottom_rule(self, pane: Rect) -> Rect {
        Rect::new(
            pane.x,
            pane.y,
            pane.width,
            pane.height.saturating_sub(BOTTOM_RULE_HEIGHT),
        )
    }
}

pub fn rect_contains_cell(rect: Rect, column: u16, row: u16) -> bool {
    rect.width > 0
        && rect.height > 0
        && column >= rect.x
        && column < rect.right()
        && row >= rect.y
        && row < rect.bottom()
}

struct ShellFrame {
    top_bar: Rect,
    status_bar: Rect,
    activity_rail: Rect,
    body_height: u16,
}

struct ChangesColumns {
    changes: Rect,
    changes_log_divider: Rect,
    log_area: Rect,
}

struct LogColumns {
    log_tabs: Rect,
    log_filters: Rect,
    log_rail: Rect,
    commits: Rect,
    commits_details_divider: Rect,
    details: Rect,
}

pub fn compute_pane_layout(
    screen: Size,
    changes_width: u16,
    details_width: u16,
    shows_log_filters: bool,
) -> PaneLayout {
    let frame = compute_shell_frame(screen);
    let columns = compute_changes_columns(screen.width, frame.body_height, changes_width);
    let log_columns = compute_log_columns(
        screen.width,
        columns.log_area,
        details_width,
        shows_log_filters,
    );
    PaneLayout {
        top_bar: frame.top_bar,
        activity_rail: frame.activity_rail,
        changes: columns.changes,
        changes_log_divider: columns.changes_log_divider,
        log_area: columns.log_area,
        log_tabs: log_columns.log_tabs,
        log_filters: log_columns.log_filters,
        log_rail: log_columns.log_rail,
        commits: log_columns.commits,
        commits_details_divider: log_columns.commits_details_divider,
        details: log_columns.details,
        status_bar: frame.status_bar,
    }
}

fn compute_shell_frame(screen: Size) -> ShellFrame {
    let top_bar = Rect::new(0, 0, screen.width, TOP_BAR_HEIGHT.min(screen.height));
    let status_bar = Rect::new(
        0,
        screen.height.saturating_sub(STATUS_BAR_HEIGHT),
        screen.width,
        STATUS_BAR_HEIGHT.min(screen.height),
    );
    let body_top = top_bar.bottom();
    let body_height = status_bar.y.saturating_sub(body_top);
    let activity_rail = Rect::new(
        0,
        body_top,
        ACTIVITY_RAIL_WIDTH.min(screen.width),
        body_height,
    );
    ShellFrame {
        top_bar,
        status_bar,
        activity_rail,
        body_height,
    }
}

fn compute_changes_columns(
    screen_width: u16,
    body_height: u16,
    changes_width: u16,
) -> ChangesColumns {
    let changes_x = ACTIVITY_RAIL_WIDTH.min(screen_width);
    let body_top = TOP_BAR_HEIGHT;
    let changes = Rect::new(
        changes_x,
        body_top,
        resolve_changes_width(screen_width, changes_width),
        body_height,
    );
    let changes_log_divider = Rect::new(changes.right(), body_top, DIVIDER_WIDTH, body_height);
    let log_area_x = changes_log_divider.right();
    let log_area = Rect::new(
        log_area_x,
        body_top,
        screen_width.saturating_sub(log_area_x),
        body_height,
    );
    ChangesColumns {
        changes,
        changes_log_divider,
        log_area,
    }
}

fn compute_log_columns(
    screen_width: u16,
    log_area: Rect,
    details_width: u16,
    shows_log_filters: bool,
) -> LogColumns {
    let log_tabs = Rect::new(
        log_area.x,
        log_area.y,
        log_area.width,
        LOG_TABS_HEIGHT.min(log_area.height),
    );
    let filters_height = if shows_log_filters {
        LOG_FILTERS_HEIGHT.min(log_area.height.saturating_sub(log_tabs.height))
    } else {
        0
    };
    let log_filters = Rect::new(
        log_area.x,
        log_tabs.bottom(),
        log_area.width,
        filters_height,
    );
    let commits_top = log_filters.bottom();
    let commits_height = log_area.bottom().saturating_sub(commits_top);
    let (resolved_details_width, resolved_commits_width) =
        resolve_details_and_commits_widths(screen_width, log_area.width, details_width);
    let log_rail_width = LOG_RAIL_WIDTH.min(resolved_commits_width);
    let log_rail = Rect::new(log_area.x, commits_top, log_rail_width, commits_height);
    let rail_gap = LOG_RAIL_GAP.min(resolved_commits_width.saturating_sub(log_rail_width));
    let commits_x = log_rail.right().saturating_add(rail_gap);
    let commits_width = log_area
        .x
        .saturating_add(resolved_commits_width)
        .saturating_sub(commits_x);
    let commits = Rect::new(commits_x, commits_top, commits_width, commits_height);
    let divider_width = if resolved_details_width == 0 {
        0
    } else {
        DIVIDER_WIDTH
    };
    let commits_details_divider =
        Rect::new(commits.right(), commits_top, divider_width, commits_height);
    let details = Rect::new(
        commits_details_divider.right(),
        commits_top,
        resolved_details_width,
        commits_height,
    );
    LogColumns {
        log_tabs,
        log_filters,
        log_rail,
        commits,
        commits_details_divider,
        details,
    }
}

pub fn default_changes_width(screen_width: u16) -> u16 {
    if screen_width < WIDE_LAYOUT_MIN_SCREEN_WIDTH {
        return CHANGES_DEFAULT_WIDTH;
    }
    proportional_pane_width(screen_width, CHANGES_WIDTH_PERCENT)
        .clamp(CHANGES_DEFAULT_WIDTH, CHANGES_MAX_WIDTH)
}

pub fn default_details_width(screen_width: u16) -> u16 {
    if screen_width < WIDE_LAYOUT_MIN_SCREEN_WIDTH {
        return DETAILS_DEFAULT_WIDTH;
    }
    proportional_pane_width(screen_width, DETAILS_WIDTH_PERCENT)
        .clamp(DETAILS_DEFAULT_WIDTH, DETAILS_DEFAULT_MAX_WIDTH)
}

pub fn details_max_width(screen_width: u16) -> u16 {
    proportional_pane_width(screen_width, DETAILS_MAX_WIDTH_PERCENT).max(DETAILS_MIN_WIDTH)
}

fn proportional_pane_width(screen_width: u16, percent: u16) -> u16 {
    u16::try_from(u32::from(screen_width) * u32::from(percent) / 100).unwrap_or(u16::MAX)
}

pub fn resolve_changes_width(screen_width: u16, requested_width: u16) -> u16 {
    let room_for_log_area = screen_width.saturating_sub(ACTIVITY_RAIL_WIDTH + DIVIDER_WIDTH + 1);
    requested_width
        .clamp(CHANGES_MIN_WIDTH, CHANGES_MAX_WIDTH)
        .min(room_for_log_area)
}

pub fn resolve_details_and_commits_widths(
    screen_width: u16,
    log_area_width: u16,
    requested_details_width: u16,
) -> (u16, u16) {
    if screen_width < DETAILS_HIDDEN_BELOW_SCREEN_WIDTH
        || requested_details_width == DETAILS_WIDTH_HIDDEN
    {
        return (DETAILS_WIDTH_HIDDEN, log_area_width);
    }
    let details_ceiling = log_area_width.saturating_sub(DIVIDER_WIDTH + COMMITS_FLOOR_WIDTH);
    let details_width = requested_details_width
        .min(details_max_width(screen_width))
        .max(DETAILS_MIN_WIDTH)
        .min(details_ceiling);
    let commits_width = log_area_width.saturating_sub(details_width + DIVIDER_WIDTH);
    (details_width, commits_width)
}

pub fn changes_width_for_divider_column(column: u16) -> u16 {
    column
        .saturating_sub(ACTIVITY_RAIL_WIDTH)
        .clamp(CHANGES_MIN_WIDTH, CHANGES_MAX_WIDTH)
}

pub fn details_width_for_divider_column(
    screen_width: u16,
    log_area_width: u16,
    column: u16,
) -> u16 {
    let requested_width = screen_width.saturating_sub(column + DIVIDER_WIDTH);
    requested_width
        .clamp(DETAILS_MIN_WIDTH, details_max_width(screen_width))
        .min(log_area_width.saturating_sub(DIVIDER_WIDTH + COMMITS_FLOOR_WIDTH))
}

#[cfg(test)]
mod pane_layout_tests {
    use gitdeck_core::{PaneId, Size};

    use super::{
        changes_width_for_divider_column, compute_pane_layout, default_changes_width,
        default_details_width, details_max_width, details_width_for_divider_column,
        resolve_details_and_commits_widths, DividerId, ACTIVITY_RAIL_WIDTH, CHANGES_DEFAULT_WIDTH,
        CHANGES_MAX_WIDTH, DETAILS_DEFAULT_WIDTH, DETAILS_HIDDEN_BELOW_SCREEN_WIDTH,
        DETAILS_MAX_WIDTH_PERCENT, DETAILS_WIDTH_HIDDEN, DIVIDER_WIDTH, LOG_RAIL_GAP,
        LOG_RAIL_WIDTH, STATUS_BAR_HEIGHT, TOP_BAR_HEIGHT, WIDE_LAYOUT_MIN_SCREEN_WIDTH,
    };

    const SCREEN_SIZES: [(u16, u16, u16, u16); 4] = [
        (80, 24, 0, 41),
        (120, 32, 40, 40),
        (160, 48, 40, 80),
        (200, 60, 40, 120),
    ];

    #[test]
    fn pane_columns_follow_the_fixed_geometry() {
        for (width, height, expected_details, expected_commits) in SCREEN_SIZES {
            let layout = compute_pane_layout(
                Size { width, height },
                CHANGES_DEFAULT_WIDTH,
                DETAILS_DEFAULT_WIDTH,
                true,
            );
            assert_eq!(
                layout.top_bar,
                ratatui::layout::Rect::new(0, 0, width, TOP_BAR_HEIGHT)
            );
            assert_eq!(
                layout.status_bar,
                ratatui::layout::Rect::new(0, height - STATUS_BAR_HEIGHT, width, STATUS_BAR_HEIGHT)
            );
            assert_eq!(layout.activity_rail.width, ACTIVITY_RAIL_WIDTH);
            assert_eq!(layout.changes.width, CHANGES_DEFAULT_WIDTH);
            assert_eq!(layout.details.width, expected_details);
            assert_eq!(layout.log_rail.x, layout.log_area.x);
            assert_eq!(
                layout.log_rail.width, LOG_RAIL_WIDTH,
                "{width}x{height} rail"
            );
            assert_eq!(layout.commits.x, layout.log_rail.right() + LOG_RAIL_GAP);
            assert_eq!(
                layout.commits.width + LOG_RAIL_WIDTH + LOG_RAIL_GAP,
                expected_commits,
                "{width}x{height} commits share the log column with the rail"
            );
            assert_eq!(layout.changes.right(), layout.changes_log_divider.x);
            assert_eq!(layout.commits.right(), layout.commits_details_divider.x);
            if expected_details == 0 {
                assert_eq!(layout.commits_details_divider.width, 0, "{width}x{height}");
                assert_eq!(
                    layout.commits.right(),
                    layout.log_area.right(),
                    "{width}x{height} commits absorb the hidden details column"
                );
                continue;
            }
            assert_eq!(
                layout.details.x,
                layout.commits_details_divider.right(),
                "{width}x{height} details follows the divider"
            );
            assert_eq!(
                layout.details.right(),
                width,
                "{width}x{height} details ends at the screen edge"
            );
            assert_eq!(layout.log_tabs.y, layout.top_bar.bottom());
            assert_eq!(layout.log_filters.y, layout.log_tabs.bottom());
            assert_eq!(layout.commits.y, layout.log_filters.bottom());
            assert_eq!(
                layout.commits.height,
                height - TOP_BAR_HEIGHT - STATUS_BAR_HEIGHT - 2
            );
        }
    }

    #[test]
    fn zero_sized_screen_produces_empty_rects() {
        let layout = compute_pane_layout(
            Size {
                width: 0,
                height: 0,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        assert_eq!(layout.top_bar.width, 0);
        assert_eq!(layout.activity_rail.height, 0);
        assert_eq!(layout.commits.width, 0);
        assert_eq!(layout.details.height, 0);
    }

    #[test]
    fn screens_below_the_details_threshold_hide_the_details_column() {
        for width in [40, 80, 99] {
            let layout = compute_pane_layout(
                Size { width, height: 24 },
                CHANGES_DEFAULT_WIDTH,
                DETAILS_DEFAULT_WIDTH,
                true,
            );
            assert_eq!(layout.details.width, 0, "{width} columns hide details");
            assert_eq!(layout.commits_details_divider.width, 0);
            assert_eq!(layout.commits.right(), layout.log_area.right());
        }
        let layout = compute_pane_layout(
            Size {
                width: DETAILS_HIDDEN_BELOW_SCREEN_WIDTH,
                height: 24,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        assert_eq!(
            layout.details.width, DETAILS_DEFAULT_WIDTH,
            "the threshold width keeps the details column at its default width"
        );
    }

    #[test]
    fn hidden_log_filters_hand_their_row_to_the_commits_column() {
        let size = Size {
            width: 160,
            height: 48,
        };
        let with_filters =
            compute_pane_layout(size, CHANGES_DEFAULT_WIDTH, DETAILS_DEFAULT_WIDTH, true);
        let without_filters =
            compute_pane_layout(size, CHANGES_DEFAULT_WIDTH, DETAILS_DEFAULT_WIDTH, false);
        assert_eq!(with_filters.log_filters.height, 1);
        assert_eq!(without_filters.log_filters.height, 0);
        assert_eq!(without_filters.commits.y, with_filters.commits.y - 1);
        assert_eq!(
            without_filters.commits.height,
            with_filters.commits.height + 1
        );
        assert_eq!(without_filters.log_tabs, with_filters.log_tabs);
    }

    #[test]
    fn details_width_keeps_commits_minimum_when_room_allows() {
        assert_eq!(resolve_details_and_commits_widths(160, 121, 40), (40, 80));
        assert_eq!(resolve_details_and_commits_widths(160, 81, 40), (40, 40));
        assert_eq!(resolve_details_and_commits_widths(200, 161, 40), (40, 120));
        assert_eq!(
            resolve_details_and_commits_widths(160, 41, 40),
            (30, 10),
            "a narrow log area keeps the commits floor of ten columns"
        );
        assert_eq!(
            resolve_details_and_commits_widths(160, 41, 60),
            (30, 10),
            "a wide stored details width cannot squeeze commits below its floor"
        );
    }

    #[test]
    fn a_hidden_details_width_hands_the_whole_log_area_to_the_commits() {
        assert_eq!(
            resolve_details_and_commits_widths(160, 121, DETAILS_WIDTH_HIDDEN),
            (0, 121)
        );
        let layout = compute_pane_layout(
            Size {
                width: 160,
                height: 48,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_WIDTH_HIDDEN,
            true,
        );
        assert_eq!(layout.details.width, 0);
        assert_eq!(layout.commits_details_divider.width, 0);
        assert_eq!(layout.commits.right(), layout.log_area.right());
        assert_eq!(layout.changes_log_divider.width, DIVIDER_WIDTH);
    }

    #[test]
    fn changes_divider_column_maps_to_clamped_width() {
        assert_eq!(changes_width_for_divider_column(0), 24);
        assert_eq!(changes_width_for_divider_column(4 + 34), 34);
        assert_eq!(changes_width_for_divider_column(159), CHANGES_MAX_WIDTH);
    }

    #[test]
    fn details_divider_column_maps_to_clamped_width() {
        assert_eq!(
            details_width_for_divider_column(160, 121, 0),
            details_max_width(160)
        );
        assert_eq!(details_width_for_divider_column(160, 121, 119), 40);
        assert_eq!(details_width_for_divider_column(160, 121, 159), 24);
        assert_eq!(
            details_width_for_divider_column(160, 60, 0),
            49,
            "commits keeps its floor of ten columns"
        );
        assert_eq!(
            details_width_for_divider_column(120, 81, 55),
            64,
            "a drag to the left grows the details column past forty columns"
        );
        assert_eq!(
            details_width_for_divider_column(120, 81, 95),
            24,
            "a drag to the right shrinks the details column to its floor"
        );
    }

    #[test]
    fn the_details_maximum_is_sixty_percent_of_the_screen() {
        for screen_width in [100, 120, 160, 200, 220, 300] {
            assert_eq!(
                details_max_width(screen_width),
                screen_width * DETAILS_MAX_WIDTH_PERCENT / 100,
                "{screen_width} columns"
            );
        }
        assert_eq!(
            details_max_width(20),
            super::DETAILS_MIN_WIDTH,
            "a screen narrower than the floor keeps the details minimum"
        );
    }

    #[test]
    fn wide_screens_take_proportional_pane_widths() {
        assert_eq!(default_changes_width(220), 48);
        assert_eq!(default_details_width(220), 61);
        assert_eq!(default_changes_width(160), 35);
        assert_eq!(default_details_width(160), 44);
        assert_eq!(
            default_changes_width(400),
            CHANGES_MAX_WIDTH,
            "the proportional width stops at the wide ceiling"
        );
        assert_eq!(default_details_width(400), super::DETAILS_DEFAULT_MAX_WIDTH);
    }

    #[test]
    fn narrow_screens_keep_the_fixed_pane_widths() {
        for width in [40, 80, 120, WIDE_LAYOUT_MIN_SCREEN_WIDTH - 1] {
            assert_eq!(default_changes_width(width), CHANGES_DEFAULT_WIDTH);
            assert_eq!(default_details_width(width), DETAILS_DEFAULT_WIDTH);
        }
    }

    #[test]
    fn divider_rects_cover_only_their_own_column() {
        let layout = compute_pane_layout(
            Size {
                width: 160,
                height: 48,
            },
            CHANGES_DEFAULT_WIDTH,
            DETAILS_DEFAULT_WIDTH,
            true,
        );
        assert_eq!(layout.changes_log_divider.width, DIVIDER_WIDTH);
        assert_eq!(layout.changes_log_divider.height, 46);
        assert_eq!(layout.commits_details_divider.height, 44);
        assert_eq!(layout.divider_rect(DividerId::ChangesLog).x, 4 + 34);
        assert_eq!(layout.log_rail.x, layout.log_area.x);
        assert_eq!(
            layout.pane_rect(PaneId::Log).x,
            layout.log_area.x + LOG_RAIL_WIDTH + LOG_RAIL_GAP
        );
    }

    #[test]
    fn log_content_rect_spans_commits_details_and_the_divider_between_them() {
        for (width, height, _, _) in SCREEN_SIZES {
            let layout = compute_pane_layout(
                Size { width, height },
                CHANGES_DEFAULT_WIDTH,
                DETAILS_DEFAULT_WIDTH,
                true,
            );
            assert_eq!(
                layout.log_content_rect(),
                ratatui::layout::Rect::new(
                    layout.log_area.x,
                    layout.commits.y,
                    layout.log_area.width,
                    layout.commits.height
                ),
                "{width}x{height} log content rect"
            );
        }
    }
}
