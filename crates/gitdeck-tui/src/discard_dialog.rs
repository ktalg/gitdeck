use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::dialog_frame::{
    button_label_style, centered_dialog_frame, dialog_content_row, dialog_panel_style,
    draw_dialog_border, draw_text_row, fill_rect, label_rect, DIALOG_BUTTON_GAP,
};
use crate::display_text::sanitize_display_string;
use crate::theme::theme_palette;

pub const DISCARD_DIALOG_TITLE: &str = "Discard Changes";
pub const DISCARD_DIALOG_MAX_WIDTH: u16 = 60;
pub const DISCARD_DIALOG_HEIGHT: u16 = 7;
pub const DISCARD_IRREVERSIBLE_LABEL: &str = "This cannot be undone.";
pub const DISCARD_CONFIRM_BUTTON_LABEL: &str = "[ Discard ]";
pub const DISCARD_CANCEL_BUTTON_LABEL: &str = "[ Cancel ]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscardDialogButton {
    Discard,
    Cancel,
}

pub const DISCARD_DIALOG_BUTTON_ORDER: [DiscardDialogButton; 2] =
    [DiscardDialogButton::Discard, DiscardDialogButton::Cancel];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscardDialogLayout {
    pub frame: Rect,
    pub message_row: Rect,
    pub irreversible_row: Rect,
    pub button_row: Rect,
    pub discard_button: Rect,
    pub cancel_button: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscardDialogView<'a> {
    pub path: &'a str,
    pub button: DiscardDialogButton,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn discard_message_text(path: &str) -> String {
    format!("Discard all local changes to {path}?")
}

pub fn discard_dialog_layout(area: Rect) -> Option<DiscardDialogLayout> {
    let frame = centered_dialog_frame(area, DISCARD_DIALOG_MAX_WIDTH, DISCARD_DIALOG_HEIGHT)?;
    let button_row = dialog_content_row(frame, 4);
    let discard_button = label_rect(
        button_row.x,
        button_row.y,
        DISCARD_CONFIRM_BUTTON_LABEL,
        button_row.right(),
    );
    let cancel_button = label_rect(
        discard_button.right().saturating_add(DIALOG_BUTTON_GAP),
        button_row.y,
        DISCARD_CANCEL_BUTTON_LABEL,
        button_row.right(),
    );
    Some(DiscardDialogLayout {
        frame,
        message_row: dialog_content_row(frame, 1),
        irreversible_row: dialog_content_row(frame, 2),
        button_row,
        discard_button,
        cancel_button,
    })
}

pub fn render_discard_dialog(area: Rect, view: DiscardDialogView<'_>, buffer: &mut Buffer) {
    let Some(layout) = discard_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(
        layout.frame,
        DISCARD_DIALOG_TITLE,
        view.caps,
        &theme,
        buffer,
    );
    let message = sanitize_display_string(
        &discard_message_text(view.path),
        layout.message_row.x,
        view.caps,
    );
    draw_text_row(
        layout.message_row,
        &message,
        dialog_panel_style(&theme),
        buffer,
    );
    draw_text_row(
        layout.irreversible_row,
        DISCARD_IRREVERSIBLE_LABEL,
        dialog_panel_style(&theme),
        buffer,
    );
    for (rect, label, kind, color) in [
        (
            layout.discard_button,
            DISCARD_CONFIRM_BUTTON_LABEL,
            DiscardDialogButton::Discard,
            theme.error,
        ),
        (
            layout.cancel_button,
            DISCARD_CANCEL_BUTTON_LABEL,
            DiscardDialogButton::Cancel,
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
mod discard_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    use super::{
        discard_dialog_layout, discard_message_text, render_discard_dialog, DiscardDialogButton,
        DiscardDialogView, DISCARD_CANCEL_BUTTON_LABEL, DISCARD_CONFIRM_BUTTON_LABEL,
        DISCARD_DIALOG_HEIGHT, DISCARD_IRREVERSIBLE_LABEL,
    };
    use crate::theme::theme_palette;

    fn dialog_view(button: DiscardDialogButton) -> DiscardDialogView<'static> {
        DiscardDialogView {
            path: "src/app.rs",
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
    fn the_dialog_rows_carry_the_message_the_warning_and_both_buttons() {
        for (width, height) in [(120, 32), (80, 24)] {
            let area = Rect::new(0, 0, width, height);
            let layout = discard_dialog_layout(area).expect("the dialog fits the screen");
            assert_eq!(layout.frame.height, DISCARD_DIALOG_HEIGHT);
            assert!(layout.frame.bottom() <= height);
            let mut buffer = Buffer::empty(area);
            render_discard_dialog(area, dialog_view(DiscardDialogButton::Cancel), &mut buffer);
            assert_eq!(
                row_text(&buffer, layout.message_row).trim_end(),
                discard_message_text("src/app.rs")
            );
            assert_eq!(
                row_text(&buffer, layout.irreversible_row).trim_end(),
                DISCARD_IRREVERSIBLE_LABEL
            );
            let button_text = row_text(&buffer, layout.button_row);
            assert!(button_text.contains(DISCARD_CONFIRM_BUTTON_LABEL));
            assert!(button_text.contains(DISCARD_CANCEL_BUTTON_LABEL));
        }
    }

    #[test]
    fn the_discard_dialog_marks_the_focused_button() {
        let area = Rect::new(0, 0, 120, 32);
        let theme = theme_palette(ThemeId::JbDark);
        let layout = discard_dialog_layout(area).expect("the dialog fits");
        for (focused, focused_button, other_button) in [
            (
                DiscardDialogButton::Cancel,
                layout.cancel_button,
                layout.discard_button,
            ),
            (
                DiscardDialogButton::Discard,
                layout.discard_button,
                layout.cancel_button,
            ),
        ] {
            let mut buffer = Buffer::empty(area);
            render_discard_dialog(area, dialog_view(focused), &mut buffer);
            let focused_cell = buffer
                .cell((focused_button.x, focused_button.y))
                .expect("the focused button starts inside the dialog");
            let other_cell = buffer
                .cell((other_button.x, other_button.y))
                .expect("the other button starts inside the dialog");
            assert_eq!(focused_cell.bg, theme.accent, "{focused:?} is highlighted");
            assert_eq!(other_cell.bg, theme.panel, "the other button stays plain");
        }
    }

    #[test]
    fn only_the_selected_button_is_underlined() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = discard_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_discard_dialog(area, dialog_view(DiscardDialogButton::Cancel), &mut buffer);
        let cancel_cell = buffer
            .cell((layout.cancel_button.x, layout.cancel_button.y))
            .expect("the cancel button starts inside the dialog");
        let discard_cell = buffer
            .cell((layout.discard_button.x, layout.discard_button.y))
            .expect("the discard button starts inside the dialog");
        assert!(cancel_cell.modifier.contains(Modifier::UNDERLINED));
        assert!(!discard_cell.modifier.contains(Modifier::UNDERLINED));
    }
}
