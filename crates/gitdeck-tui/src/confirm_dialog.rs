use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::dialog_frame::{
    button_label_style, centered_dialog_frame, dialog_content_row, dialog_panel_style,
    draw_dialog_border, draw_text_row, fill_rect, label_rect, DIALOG_BUTTON_GAP,
};
use crate::display_text::sanitize_display_string;
use crate::theme::theme_palette;

pub const CONFIRM_DIALOG_MAX_WIDTH: u16 = 66;
pub const CONFIRM_DIALOG_HEIGHT: u16 = 7;
pub const ABORT_CONFIRM_BUTTON_LABEL: &str = "[ Abort ]";
pub const QUIT_CONFIRM_BUTTON_LABEL: &str = "[ Quit ]";
pub const CONFIRM_CANCEL_BUTTON_LABEL: &str = "[ Cancel ]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmDialogButton {
    Confirm,
    Cancel,
}

pub const CONFIRM_DIALOG_BUTTON_ORDER: [ConfirmDialogButton; 2] =
    [ConfirmDialogButton::Confirm, ConfirmDialogButton::Cancel];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfirmDialogLayout {
    pub frame: Rect,
    pub message_row: Rect,
    pub detail_row: Rect,
    pub button_row: Rect,
    pub confirm_button: Rect,
    pub cancel_button: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfirmDialogView<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub detail: Option<&'a str>,
    pub confirm_label: &'a str,
    pub button: ConfirmDialogButton,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn confirm_dialog_layout(area: Rect) -> Option<ConfirmDialogLayout> {
    let frame = centered_dialog_frame(area, CONFIRM_DIALOG_MAX_WIDTH, CONFIRM_DIALOG_HEIGHT)?;
    let button_row = dialog_content_row(frame, 4);
    let confirm_button = label_rect(
        button_row.x,
        button_row.y,
        ABORT_CONFIRM_BUTTON_LABEL,
        button_row.right(),
    );
    let cancel_button = label_rect(
        confirm_button.right().saturating_add(DIALOG_BUTTON_GAP),
        button_row.y,
        CONFIRM_CANCEL_BUTTON_LABEL,
        button_row.right(),
    );
    Some(ConfirmDialogLayout {
        frame,
        message_row: dialog_content_row(frame, 1),
        detail_row: dialog_content_row(frame, 2),
        button_row,
        confirm_button,
        cancel_button,
    })
}

pub fn render_confirm_dialog(area: Rect, view: ConfirmDialogView<'_>, buffer: &mut Buffer) {
    let Some(layout) = confirm_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(layout.frame, view.title, view.caps, &theme, buffer);
    let message = sanitize_display_string(view.message, layout.message_row.x, view.caps);
    draw_text_row(
        layout.message_row,
        &message,
        dialog_panel_style(&theme),
        buffer,
    );
    if let Some(detail) = view.detail {
        let detail = sanitize_display_string(detail, layout.detail_row.x, view.caps);
        draw_text_row(
            layout.detail_row,
            &detail,
            dialog_panel_style(&theme),
            buffer,
        );
    }
    for (rect, label, kind, color) in [
        (
            layout.confirm_button,
            view.confirm_label,
            ConfirmDialogButton::Confirm,
            theme.error,
        ),
        (
            layout.cancel_button,
            CONFIRM_CANCEL_BUTTON_LABEL,
            ConfirmDialogButton::Cancel,
            theme.muted,
        ),
    ] {
        draw_text_row(
            rect,
            label,
            button_label_style(color, view.button == kind, &theme),
            buffer,
        );
    }
}

#[cfg(test)]
mod confirm_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    use super::{
        confirm_dialog_layout, render_confirm_dialog, ConfirmDialogButton, ConfirmDialogView,
        CONFIRM_CANCEL_BUTTON_LABEL, CONFIRM_DIALOG_HEIGHT, QUIT_CONFIRM_BUTTON_LABEL,
    };
    use crate::theme::theme_palette;

    fn dialog_view(button: ConfirmDialogButton) -> ConfirmDialogView<'static> {
        ConfirmDialogView {
            title: "Quit",
            message: "You have an unsaved commit message.",
            detail: None,
            confirm_label: QUIT_CONFIRM_BUTTON_LABEL,
            button,
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
    fn the_dialog_rows_carry_the_message_and_both_buttons() {
        for (width, height) in [(120, 32), (80, 24)] {
            let area = Rect::new(0, 0, width, height);
            let layout = confirm_dialog_layout(area).expect("the dialog fits the screen");
            assert_eq!(layout.frame.height, CONFIRM_DIALOG_HEIGHT);
            assert!(layout.frame.bottom() <= height);
            let mut buffer = Buffer::empty(area);
            render_confirm_dialog(area, dialog_view(ConfirmDialogButton::Cancel), &mut buffer);
            assert_eq!(
                row_text(&buffer, layout.message_row).trim_end(),
                "You have an unsaved commit message."
            );
            let button_text = row_text(&buffer, layout.button_row);
            assert!(button_text.contains(QUIT_CONFIRM_BUTTON_LABEL));
            assert!(button_text.contains(CONFIRM_CANCEL_BUTTON_LABEL));
        }
    }

    #[test]
    fn the_dialog_marks_the_focused_button_and_underlines_only_it() {
        let area = Rect::new(0, 0, 120, 32);
        let theme = theme_palette(ThemeId::JbDark);
        let layout = confirm_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_confirm_dialog(area, dialog_view(ConfirmDialogButton::Cancel), &mut buffer);
        let cancel_cell = buffer
            .cell((layout.cancel_button.x, layout.cancel_button.y))
            .expect("the cancel button starts inside the dialog");
        let confirm_cell = buffer
            .cell((layout.confirm_button.x, layout.confirm_button.y))
            .expect("the confirm button starts inside the dialog");
        assert_eq!(cancel_cell.bg, theme.accent);
        assert_eq!(confirm_cell.bg, theme.panel);
        assert!(cancel_cell.modifier.contains(Modifier::UNDERLINED));
        assert!(!confirm_cell.modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn the_detail_line_is_drawn_only_when_the_view_carries_one() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = confirm_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_confirm_dialog(
            area,
            ConfirmDialogView {
                detail: Some("This discards the conflict resolutions you already made."),
                ..dialog_view(ConfirmDialogButton::Cancel)
            },
            &mut buffer,
        );
        assert_eq!(
            row_text(&buffer, layout.detail_row).trim_end(),
            "This discards the conflict resolutions you already made."
        );
    }
}
