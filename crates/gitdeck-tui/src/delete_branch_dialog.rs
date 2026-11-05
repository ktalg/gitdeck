use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::dialog_frame::{
    button_label_style, centered_dialog_frame, dialog_content_row, dialog_panel_style,
    draw_dialog_border, draw_text_row, fill_rect, label_rect, DIALOG_BUTTON_GAP,
};
use crate::discard_dialog::{
    DISCARD_DIALOG_HEIGHT, DISCARD_DIALOG_MAX_WIDTH, DISCARD_IRREVERSIBLE_LABEL,
};
use crate::display_text::sanitize_display_string;
use crate::theme::theme_palette;

pub const DELETE_BRANCH_DIALOG_TITLE: &str = "Delete Branch";
pub const DELETE_BRANCH_CONFIRM_BUTTON_LABEL: &str = "[ Delete ]";
pub const DELETE_BRANCH_CANCEL_BUTTON_LABEL: &str = "[ Cancel ]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeleteBranchDialogButton {
    Delete,
    Cancel,
}

pub const DELETE_BRANCH_DIALOG_BUTTON_ORDER: [DeleteBranchDialogButton; 2] = [
    DeleteBranchDialogButton::Delete,
    DeleteBranchDialogButton::Cancel,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeleteBranchDialogLayout {
    pub frame: Rect,
    pub message_row: Rect,
    pub irreversible_row: Rect,
    pub button_row: Rect,
    pub delete_button: Rect,
    pub cancel_button: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeleteBranchDialogView<'a> {
    pub name: &'a str,
    pub button: DeleteBranchDialogButton,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn delete_branch_message_text(name: &str) -> String {
    format!("Delete branch {name}?")
}

pub fn delete_branch_dialog_layout(area: Rect) -> Option<DeleteBranchDialogLayout> {
    let frame = centered_dialog_frame(area, DISCARD_DIALOG_MAX_WIDTH, DISCARD_DIALOG_HEIGHT)?;
    let button_row = dialog_content_row(frame, 4);
    let delete_button = label_rect(
        button_row.x,
        button_row.y,
        DELETE_BRANCH_CONFIRM_BUTTON_LABEL,
        button_row.right(),
    );
    let cancel_button = label_rect(
        delete_button.right().saturating_add(DIALOG_BUTTON_GAP),
        button_row.y,
        DELETE_BRANCH_CANCEL_BUTTON_LABEL,
        button_row.right(),
    );
    Some(DeleteBranchDialogLayout {
        frame,
        message_row: dialog_content_row(frame, 1),
        irreversible_row: dialog_content_row(frame, 2),
        button_row,
        delete_button,
        cancel_button,
    })
}

pub fn render_delete_branch_dialog(
    area: Rect,
    view: DeleteBranchDialogView<'_>,
    buffer: &mut Buffer,
) {
    let Some(layout) = delete_branch_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(
        layout.frame,
        DELETE_BRANCH_DIALOG_TITLE,
        view.caps,
        &theme,
        buffer,
    );
    let message = sanitize_display_string(
        &delete_branch_message_text(view.name),
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
            layout.delete_button,
            DELETE_BRANCH_CONFIRM_BUTTON_LABEL,
            DeleteBranchDialogButton::Delete,
            theme.error,
        ),
        (
            layout.cancel_button,
            DELETE_BRANCH_CANCEL_BUTTON_LABEL,
            DeleteBranchDialogButton::Cancel,
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
mod delete_branch_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    use super::{
        delete_branch_dialog_layout, delete_branch_message_text, render_delete_branch_dialog,
        DeleteBranchDialogButton, DeleteBranchDialogView, DELETE_BRANCH_CANCEL_BUTTON_LABEL,
        DELETE_BRANCH_CONFIRM_BUTTON_LABEL, DELETE_BRANCH_DIALOG_TITLE,
    };

    fn row_text(buffer: &Buffer, row: Rect) -> String {
        (row.x..row.right())
            .filter_map(|column| buffer.cell((column, row.y)).map(|cell| cell.symbol()))
            .collect()
    }

    #[test]
    fn the_dialog_rows_carry_the_title_the_message_and_both_buttons() {
        for (width, height) in [(120, 32), (80, 24)] {
            let area = Rect::new(0, 0, width, height);
            let layout = delete_branch_dialog_layout(area).expect("the dialog fits the screen");
            let mut buffer = Buffer::empty(area);
            render_delete_branch_dialog(
                area,
                DeleteBranchDialogView {
                    name: "feature/gone",
                    button: DeleteBranchDialogButton::Cancel,
                    caps: CapsProfile::Rich,
                    theme: ThemeId::JbDark,
                },
                &mut buffer,
            );
            let title_row = row_text(
                &buffer,
                Rect::new(layout.frame.x, layout.frame.y, layout.frame.width, 1),
            );
            assert!(title_row.contains(DELETE_BRANCH_DIALOG_TITLE));
            assert_eq!(
                row_text(&buffer, layout.message_row).trim_end(),
                delete_branch_message_text("feature/gone")
            );
            let button_text = row_text(&buffer, layout.button_row);
            assert!(button_text.contains(DELETE_BRANCH_CONFIRM_BUTTON_LABEL));
            assert!(button_text.contains(DELETE_BRANCH_CANCEL_BUTTON_LABEL));
        }
    }

    #[test]
    fn only_the_selected_button_is_underlined() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = delete_branch_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_delete_branch_dialog(
            area,
            DeleteBranchDialogView {
                name: "feature/gone",
                button: DeleteBranchDialogButton::Cancel,
                caps: CapsProfile::Rich,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        let cancel_cell = buffer
            .cell((layout.cancel_button.x, layout.cancel_button.y))
            .expect("the cancel button starts inside the dialog");
        let delete_cell = buffer
            .cell((layout.delete_button.x, layout.delete_button.y))
            .expect("the delete button starts inside the dialog");
        assert!(cancel_cell.modifier.contains(Modifier::UNDERLINED));
        assert!(!delete_cell.modifier.contains(Modifier::UNDERLINED));
    }
}
