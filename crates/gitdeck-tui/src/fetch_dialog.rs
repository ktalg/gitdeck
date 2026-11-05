use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use crate::dialog_frame::{
    button_label_style, centered_dialog_frame, dialog_content_row, dialog_panel_style,
    draw_dialog_border, draw_text_row, fill_rect, label_rect, DIALOG_BUTTON_GAP,
    ORIGIN_REMOTE_NAME,
};
use crate::display_text::sanitize_display_string;
use crate::theme::theme_palette;

pub const FETCH_DIALOG_TITLE: &str = "Fetch";
pub const FETCH_DIALOG_MAX_WIDTH: u16 = 48;
pub const FETCH_DIALOG_HEIGHT: u16 = 6;
pub const FETCH_NO_REMOTE_LABEL: &str = "No remote configured";
pub const FETCH_CONFIRM_BUTTON_LABEL: &str = "[ Fetch ]";
pub const FETCH_CANCEL_BUTTON_LABEL: &str = "[ Cancel ]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FetchDialogButton {
    Fetch,
    Cancel,
}

pub const FETCH_DIALOG_BUTTON_ORDER: [FetchDialogButton; 2] =
    [FetchDialogButton::Fetch, FetchDialogButton::Cancel];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FetchDialogLayout {
    pub frame: Rect,
    pub message_row: Rect,
    pub button_row: Rect,
    pub fetch_button: Rect,
    pub cancel_button: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FetchDialogView {
    pub remote_available: bool,
    pub button: FetchDialogButton,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn fetch_message_text(remote_available: bool) -> String {
    if remote_available {
        return format!("Fetch from {ORIGIN_REMOTE_NAME}");
    }
    String::from(FETCH_NO_REMOTE_LABEL)
}

pub fn fetch_dialog_layout(area: Rect) -> Option<FetchDialogLayout> {
    let frame = centered_dialog_frame(area, FETCH_DIALOG_MAX_WIDTH, FETCH_DIALOG_HEIGHT)?;
    let button_row = dialog_content_row(frame, 4);
    let fetch_button = label_rect(
        button_row.x,
        button_row.y,
        FETCH_CONFIRM_BUTTON_LABEL,
        button_row.right(),
    );
    let cancel_button = label_rect(
        fetch_button.right().saturating_add(DIALOG_BUTTON_GAP),
        button_row.y,
        FETCH_CANCEL_BUTTON_LABEL,
        button_row.right(),
    );
    Some(FetchDialogLayout {
        frame,
        message_row: dialog_content_row(frame, 1),
        button_row,
        fetch_button,
        cancel_button,
    })
}

pub fn render_fetch_dialog(area: Rect, view: FetchDialogView, buffer: &mut Buffer) {
    let Some(layout) = fetch_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(layout.frame, FETCH_DIALOG_TITLE, view.caps, &theme, buffer);
    let message = sanitize_display_string(
        &fetch_message_text(view.remote_available),
        layout.message_row.x,
        view.caps,
    );
    draw_text_row(
        layout.message_row,
        &message,
        dialog_panel_style(&theme),
        buffer,
    );
    for (rect, label, kind, color) in [
        (
            layout.fetch_button,
            FETCH_CONFIRM_BUTTON_LABEL,
            FetchDialogButton::Fetch,
            theme.accent,
        ),
        (
            layout.cancel_button,
            FETCH_CANCEL_BUTTON_LABEL,
            FetchDialogButton::Cancel,
            theme.muted,
        ),
    ] {
        if rect.width == 0 {
            continue;
        }
        let disabled = !view.remote_available && kind == FetchDialogButton::Fetch;
        let selected = view.button == kind;
        let mut style = button_label_style(color, selected, &theme);
        if disabled {
            style = style.fg(theme.muted).add_modifier(Modifier::DIM);
        }
        draw_text_row(rect, label, style, buffer);
    }
}

#[cfg(test)]
mod fetch_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    use super::{
        fetch_dialog_layout, fetch_message_text, render_fetch_dialog, FetchDialogButton,
        FetchDialogView, FETCH_CANCEL_BUTTON_LABEL, FETCH_CONFIRM_BUTTON_LABEL,
        FETCH_DIALOG_HEIGHT, FETCH_DIALOG_MAX_WIDTH, FETCH_NO_REMOTE_LABEL,
    };

    fn dialog_view(remote_available: bool) -> FetchDialogView {
        FetchDialogView {
            remote_available,
            button: FetchDialogButton::Fetch,
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
        let wide = fetch_dialog_layout(Rect::new(0, 0, 220, 70))
            .expect("the wide screen holds the dialog");
        assert_eq!(wide.frame.width, FETCH_DIALOG_MAX_WIDTH);
        assert_eq!(wide.frame.height, FETCH_DIALOG_HEIGHT);
        assert_eq!(wide.frame.y, (70 - FETCH_DIALOG_HEIGHT) / 2);
        let narrow = fetch_dialog_layout(Rect::new(0, 0, 80, 24))
            .expect("the narrow screen holds the dialog");
        assert_eq!(narrow.frame.width, FETCH_DIALOG_MAX_WIDTH);
        assert!(narrow.frame.bottom() <= 24);
        assert!(
            fetch_dialog_layout(Rect::new(0, 0, 20, 24)).is_none(),
            "a screen narrower than the minimum drops the dialog instead of clipping it"
        );
    }

    #[test]
    fn the_dialog_names_the_remote_or_the_missing_one() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = fetch_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_fetch_dialog(area, dialog_view(true), &mut buffer);
        assert_eq!(
            row_text(&buffer, layout.message_row).trim_end(),
            fetch_message_text(true)
        );
        let button_text = row_text(&buffer, layout.button_row);
        assert!(button_text.contains(FETCH_CONFIRM_BUTTON_LABEL));
        assert!(button_text.contains(FETCH_CANCEL_BUTTON_LABEL));
    }

    #[test]
    fn the_missing_remote_dims_the_fetch_button() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = fetch_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_fetch_dialog(area, dialog_view(false), &mut buffer);
        assert_eq!(
            row_text(&buffer, layout.message_row).trim_end(),
            FETCH_NO_REMOTE_LABEL
        );
        let fetch_cell = buffer
            .cell((layout.fetch_button.x, layout.fetch_button.y))
            .expect("the fetch button starts inside the dialog");
        assert!(
            fetch_cell.modifier.contains(Modifier::DIM),
            "the disabled fetch button is dimmed"
        );
        let cancel_cell = buffer
            .cell((layout.cancel_button.x, layout.cancel_button.y))
            .expect("the cancel button starts inside the dialog");
        assert!(
            !cancel_cell.modifier.contains(Modifier::DIM),
            "the cancel button keeps its normal look"
        );
    }
}
