use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::changes::CheckState;
use crate::dialog_frame::{
    centered_dialog_frame, dialog_content_row, dialog_panel_style, draw_dialog_border,
    draw_dialog_button, draw_option_row, draw_text_row, fill_rect, label_rect, DIALOG_BUTTON_GAP,
    ORIGIN_REMOTE_NAME,
};
use crate::display_text::sanitize_display_string;
use crate::theme::{theme_palette, Theme};

pub const PUSH_DIALOG_TITLE: &str = "Push";
pub const PUSH_DIALOG_MAX_WIDTH: u16 = 60;
pub const PUSH_DIALOG_HEIGHT: u16 = 7;
pub const PUSH_SET_UPSTREAM_LABEL: &str = "Set upstream (-u)";
pub const PUSH_FORCE_WITH_LEASE_LABEL: &str = "Force with lease";
pub const PUSH_CONFIRM_BUTTON_LABEL: &str = "[ Push ]";
pub const PUSH_CANCEL_BUTTON_LABEL: &str = "[ Cancel ]";
pub const PUSH_DETACHED_HEAD_LABEL: &str = "HEAD (detached)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PushDialogFocus {
    SetUpstream,
    ForceWithLease,
    Buttons,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PushDialogButton {
    Push,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PushDialogLayout {
    pub frame: Rect,
    pub message_row: Rect,
    pub set_upstream_row: Rect,
    pub force_with_lease_row: Rect,
    pub button_row: Rect,
    pub push_button: Rect,
    pub cancel_button: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PushDialogView<'a> {
    pub branch_label: &'a str,
    pub set_upstream: bool,
    pub force_with_lease: bool,
    pub focus: PushDialogFocus,
    pub button: PushDialogButton,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn push_message_text(branch_label: &str) -> String {
    format!("Push {branch_label} to {ORIGIN_REMOTE_NAME}")
}

pub fn push_dialog_layout(area: Rect) -> Option<PushDialogLayout> {
    let frame = centered_dialog_frame(area, PUSH_DIALOG_MAX_WIDTH, PUSH_DIALOG_HEIGHT)?;
    let button_row = dialog_content_row(frame, 5);
    let push_button = label_rect(
        button_row.x,
        button_row.y,
        PUSH_CONFIRM_BUTTON_LABEL,
        button_row.right(),
    );
    let cancel_button = label_rect(
        push_button.right().saturating_add(DIALOG_BUTTON_GAP),
        button_row.y,
        PUSH_CANCEL_BUTTON_LABEL,
        button_row.right(),
    );
    Some(PushDialogLayout {
        frame,
        message_row: dialog_content_row(frame, 1),
        set_upstream_row: dialog_content_row(frame, 2),
        force_with_lease_row: dialog_content_row(frame, 3),
        button_row,
        push_button,
        cancel_button,
    })
}

pub fn render_push_dialog(area: Rect, view: PushDialogView<'_>, buffer: &mut Buffer) {
    let Some(layout) = push_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(layout.frame, PUSH_DIALOG_TITLE, view.caps, &theme, buffer);
    let message = sanitize_display_string(
        &push_message_text(view.branch_label),
        layout.message_row.x,
        view.caps,
    );
    draw_text_row(
        layout.message_row,
        &message,
        dialog_panel_style(&theme),
        buffer,
    );
    for (row, checked, label, focus) in [
        (
            layout.set_upstream_row,
            view.set_upstream,
            PUSH_SET_UPSTREAM_LABEL,
            PushDialogFocus::SetUpstream,
        ),
        (
            layout.force_with_lease_row,
            view.force_with_lease,
            PUSH_FORCE_WITH_LEASE_LABEL,
            PushDialogFocus::ForceWithLease,
        ),
    ] {
        draw_option_row(
            row,
            &check_box_marker(checked),
            checked,
            label,
            view.focus == focus,
            &theme,
            buffer,
        );
    }
    draw_push_dialog_buttons(layout, view, &theme, buffer);
}

fn check_box_marker(checked: bool) -> String {
    let check = if checked {
        CheckState::Checked
    } else {
        CheckState::Unchecked
    };
    format!("[{}]", check.glyph())
}

fn draw_push_dialog_buttons(
    layout: PushDialogLayout,
    view: PushDialogView<'_>,
    theme: &Theme,
    buffer: &mut Buffer,
) {
    for (rect, label, kind, color) in [
        (
            layout.push_button,
            PUSH_CONFIRM_BUTTON_LABEL,
            PushDialogButton::Push,
            theme.accent,
        ),
        (
            layout.cancel_button,
            PUSH_CANCEL_BUTTON_LABEL,
            PushDialogButton::Cancel,
            theme.muted,
        ),
    ] {
        let selected = view.focus == PushDialogFocus::Buttons && view.button == kind;
        draw_dialog_button(rect, label, color, selected, theme, buffer);
    }
}

#[cfg(test)]
mod push_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{
        push_dialog_layout, push_message_text, render_push_dialog, PushDialogButton,
        PushDialogFocus, PushDialogView, PUSH_CANCEL_BUTTON_LABEL, PUSH_CONFIRM_BUTTON_LABEL,
        PUSH_DIALOG_HEIGHT, PUSH_DIALOG_MAX_WIDTH, PUSH_FORCE_WITH_LEASE_LABEL,
        PUSH_SET_UPSTREAM_LABEL,
    };

    fn dialog_view(focus: PushDialogFocus) -> PushDialogView<'static> {
        PushDialogView {
            branch_label: "main",
            set_upstream: true,
            force_with_lease: false,
            focus,
            button: PushDialogButton::Push,
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
            push_dialog_layout(Rect::new(0, 0, 220, 70)).expect("the wide screen holds the dialog");
        assert_eq!(wide.frame.width, PUSH_DIALOG_MAX_WIDTH);
        assert_eq!(wide.frame.height, PUSH_DIALOG_HEIGHT);
        assert_eq!(wide.frame.x, (220 - PUSH_DIALOG_MAX_WIDTH) / 2);
        assert_eq!(wide.frame.y, (70 - PUSH_DIALOG_HEIGHT) / 2);
        let narrow = push_dialog_layout(Rect::new(0, 0, 80, 24))
            .expect("the narrow screen holds the dialog");
        assert_eq!(narrow.frame.width, PUSH_DIALOG_MAX_WIDTH);
        assert_eq!(narrow.frame.x, (80 - PUSH_DIALOG_MAX_WIDTH) / 2);
        assert!(narrow.frame.right() <= 80);
        assert!(narrow.frame.bottom() <= 24);
        assert!(
            push_dialog_layout(Rect::new(0, 0, 20, 24)).is_none(),
            "a screen narrower than the minimum drops the dialog instead of clipping it"
        );
    }

    #[test]
    fn the_dialog_rows_carry_the_labels_and_the_check_state() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = push_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_push_dialog(area, dialog_view(PushDialogFocus::SetUpstream), &mut buffer);
        assert_eq!(
            row_text(&buffer, layout.message_row).trim_end(),
            push_message_text("main")
        );
        assert!(row_text(&buffer, layout.set_upstream_row).contains(PUSH_SET_UPSTREAM_LABEL));
        assert!(row_text(&buffer, layout.set_upstream_row).starts_with("[x]"));
        assert!(
            row_text(&buffer, layout.force_with_lease_row).contains(PUSH_FORCE_WITH_LEASE_LABEL)
        );
        assert!(row_text(&buffer, layout.force_with_lease_row).starts_with("[ ]"));
        let button_text = row_text(&buffer, layout.button_row);
        assert!(button_text.contains(PUSH_CONFIRM_BUTTON_LABEL));
        assert!(button_text.contains(PUSH_CANCEL_BUTTON_LABEL));
    }

    #[test]
    fn only_the_selected_button_is_underlined() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = push_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_push_dialog(
            area,
            PushDialogView {
                button: PushDialogButton::Cancel,
                ..dialog_view(PushDialogFocus::Buttons)
            },
            &mut buffer,
        );
        let cancel_cell = buffer
            .cell((layout.cancel_button.x, layout.cancel_button.y))
            .expect("the cancel button starts inside the dialog");
        let push_cell = buffer
            .cell((layout.push_button.x, layout.push_button.y))
            .expect("the push button starts inside the dialog");
        assert!(cancel_cell
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED));
        assert!(!push_cell
            .modifier
            .contains(ratatui::style::Modifier::UNDERLINED));
    }
}
