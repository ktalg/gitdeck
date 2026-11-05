use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::dialog_frame::{
    centered_dialog_frame, dialog_content_row, dialog_panel_style, draw_dialog_border,
    draw_dialog_button, draw_option_row, draw_text_row, fill_rect, label_rect, DIALOG_BUTTON_GAP,
    ORIGIN_REMOTE_NAME,
};
use crate::display_text::sanitize_display_string;
use crate::theme::{theme_palette, Theme};

pub const PULL_DIALOG_TITLE: &str = "Pull";
pub const PULL_DIALOG_MAX_WIDTH: u16 = 60;
pub const PULL_DIALOG_HEIGHT: u16 = 8;
pub const PULL_FAST_FORWARD_ONLY_LABEL: &str = "Fast-forward only";
pub const PULL_MERGE_LABEL: &str = "Merge";
pub const PULL_REBASE_LABEL: &str = "Rebase";
pub const PULL_CONFIRM_BUTTON_LABEL: &str = "[ Pull ]";
pub const PULL_CANCEL_BUTTON_LABEL: &str = "[ Cancel ]";
pub const PULL_CHECKED_MARKER: &str = "(*)";
pub const PULL_UNCHECKED_MARKER: &str = "( )";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PullStrategyOption {
    FastForwardOnly,
    Merge,
    Rebase,
}

pub const PULL_STRATEGY_OPTIONS: [PullStrategyOption; 3] = [
    PullStrategyOption::FastForwardOnly,
    PullStrategyOption::Merge,
    PullStrategyOption::Rebase,
];

impl PullStrategyOption {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FastForwardOnly => PULL_FAST_FORWARD_ONLY_LABEL,
            Self::Merge => PULL_MERGE_LABEL,
            Self::Rebase => PULL_REBASE_LABEL,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PullDialogFocus {
    Strategy,
    Buttons,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PullDialogButton {
    Pull,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PullDialogLayout {
    pub frame: Rect,
    pub message_row: Rect,
    pub fast_forward_row: Rect,
    pub merge_row: Rect,
    pub rebase_row: Rect,
    pub button_row: Rect,
    pub pull_button: Rect,
    pub cancel_button: Rect,
}

impl PullDialogLayout {
    pub const fn strategy_row(self, option: PullStrategyOption) -> Rect {
        match option {
            PullStrategyOption::FastForwardOnly => self.fast_forward_row,
            PullStrategyOption::Merge => self.merge_row,
            PullStrategyOption::Rebase => self.rebase_row,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PullDialogView<'a> {
    pub branch_label: &'a str,
    pub strategy: PullStrategyOption,
    pub focus: PullDialogFocus,
    pub button: PullDialogButton,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn pull_message_text(branch_label: &str) -> String {
    format!("Pull {branch_label} from {ORIGIN_REMOTE_NAME}")
}

pub fn pull_strategy_marker(
    strategy: PullStrategyOption,
    selected: PullStrategyOption,
) -> &'static str {
    if strategy == selected {
        return PULL_CHECKED_MARKER;
    }
    PULL_UNCHECKED_MARKER
}

pub fn pull_dialog_layout(area: Rect) -> Option<PullDialogLayout> {
    let frame = centered_dialog_frame(area, PULL_DIALOG_MAX_WIDTH, PULL_DIALOG_HEIGHT)?;
    let button_row = dialog_content_row(frame, 6);
    let pull_button = label_rect(
        button_row.x,
        button_row.y,
        PULL_CONFIRM_BUTTON_LABEL,
        button_row.right(),
    );
    let cancel_button = label_rect(
        pull_button.right().saturating_add(DIALOG_BUTTON_GAP),
        button_row.y,
        PULL_CANCEL_BUTTON_LABEL,
        button_row.right(),
    );
    Some(PullDialogLayout {
        frame,
        message_row: dialog_content_row(frame, 1),
        fast_forward_row: dialog_content_row(frame, 2),
        merge_row: dialog_content_row(frame, 3),
        rebase_row: dialog_content_row(frame, 4),
        button_row,
        pull_button,
        cancel_button,
    })
}

pub fn render_pull_dialog(area: Rect, view: PullDialogView<'_>, buffer: &mut Buffer) {
    let Some(layout) = pull_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(layout.frame, PULL_DIALOG_TITLE, view.caps, &theme, buffer);
    let message = sanitize_display_string(
        &pull_message_text(view.branch_label),
        layout.message_row.x,
        view.caps,
    );
    draw_text_row(
        layout.message_row,
        &message,
        dialog_panel_style(&theme),
        buffer,
    );
    for option in PULL_STRATEGY_OPTIONS {
        draw_option_row(
            layout.strategy_row(option),
            pull_strategy_marker(option, view.strategy),
            option == view.strategy,
            option.label(),
            view.focus == PullDialogFocus::Strategy && view.strategy == option,
            &theme,
            buffer,
        );
    }
    draw_pull_dialog_buttons(layout, view, &theme, buffer);
}

fn draw_pull_dialog_buttons(
    layout: PullDialogLayout,
    view: PullDialogView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    for (rect, label, kind, color) in [
        (
            layout.pull_button,
            PULL_CONFIRM_BUTTON_LABEL,
            PullDialogButton::Pull,
            theme.accent,
        ),
        (
            layout.cancel_button,
            PULL_CANCEL_BUTTON_LABEL,
            PullDialogButton::Cancel,
            theme.muted,
        ),
    ] {
        let selected = view.focus == PullDialogFocus::Buttons && view.button == kind;
        draw_dialog_button(rect, label, color, selected, theme, buffer);
    }
}

#[cfg(test)]
mod pull_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{
        pull_dialog_layout, pull_message_text, render_pull_dialog, PullDialogButton,
        PullDialogFocus, PullDialogView, PullStrategyOption, PULL_CANCEL_BUTTON_LABEL,
        PULL_CHECKED_MARKER, PULL_CONFIRM_BUTTON_LABEL, PULL_DIALOG_HEIGHT, PULL_DIALOG_MAX_WIDTH,
        PULL_UNCHECKED_MARKER,
    };

    fn dialog_view(strategy: PullStrategyOption) -> PullDialogView<'static> {
        PullDialogView {
            branch_label: "main",
            strategy,
            focus: PullDialogFocus::Strategy,
            button: PullDialogButton::Pull,
            caps: CapsProfile::Rich,
            theme: ThemeId::JbDark,
        }
    }

    fn row_text(buffer: &Buffer, row: Rect) -> String {
        (row.x..row.right())
            .filter_map(|column| buffer.cell((column, row.y)).map(|cell| cell.symbol()))
            .collect()
    }

    #[test]
    fn the_dialog_is_centred_and_kept_inside_a_narrow_screen() {
        let wide =
            pull_dialog_layout(Rect::new(0, 0, 220, 70)).expect("the wide screen holds the dialog");
        assert_eq!(wide.frame.width, PULL_DIALOG_MAX_WIDTH);
        assert_eq!(wide.frame.height, PULL_DIALOG_HEIGHT);
        assert_eq!(wide.frame.y, (70 - PULL_DIALOG_HEIGHT) / 2);
        let narrow = pull_dialog_layout(Rect::new(0, 0, 80, 24))
            .expect("the narrow screen holds the dialog");
        assert_eq!(narrow.frame.width, PULL_DIALOG_MAX_WIDTH);
        assert_eq!(narrow.frame.x, (80 - PULL_DIALOG_MAX_WIDTH) / 2);
        assert!(narrow.frame.bottom() <= 24);
        assert!(
            pull_dialog_layout(Rect::new(0, 0, 20, 24)).is_none(),
            "a screen narrower than the minimum drops the dialog instead of clipping it"
        );
    }

    #[test]
    fn the_dialog_rows_carry_the_strategy_markers() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = pull_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_pull_dialog(area, dialog_view(PullStrategyOption::Merge), &mut buffer);
        assert_eq!(
            row_text(&buffer, layout.message_row).trim_end(),
            pull_message_text("main")
        );
        assert!(
            row_text(&buffer, layout.fast_forward_row).starts_with(PULL_UNCHECKED_MARKER),
            "the unselected strategy shows an empty radio"
        );
        assert!(
            row_text(&buffer, layout.merge_row).starts_with(PULL_CHECKED_MARKER),
            "the selected strategy shows a filled radio"
        );
        assert!(
            row_text(&buffer, layout.rebase_row).starts_with(PULL_UNCHECKED_MARKER),
            "the other unselected strategy shows an empty radio"
        );
        let button_text = row_text(&buffer, layout.button_row);
        assert!(button_text.contains(PULL_CONFIRM_BUTTON_LABEL));
        assert!(button_text.contains(PULL_CANCEL_BUTTON_LABEL));
    }
}
