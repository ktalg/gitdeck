use gitdeck_caps::{CapsProfile, ThemeId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::dialog_frame::{
    centered_dialog_frame, dialog_content_row, dialog_panel_style, draw_dialog_border,
    draw_text_row, fill_rect,
};
use crate::display_text::sanitize_display_string;
use crate::theme::theme_palette;

pub const NEW_BRANCH_DIALOG_TITLE: &str = "New Branch";
pub const NEW_BRANCH_DIALOG_MAX_WIDTH: u16 = 60;
pub const NEW_BRANCH_DIALOG_HEIGHT: u16 = 3;
pub const NEW_BRANCH_INPUT_LABEL: &str = "Branch name: ";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NewBranchDialogLayout {
    pub frame: Rect,
    pub input_row: Rect,
    pub name_column: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NewBranchDialogView<'a> {
    pub name: &'a str,
    pub caps: CapsProfile,
    pub theme: ThemeId,
}

pub fn new_branch_dialog_layout(area: Rect) -> Option<NewBranchDialogLayout> {
    let frame = centered_dialog_frame(area, NEW_BRANCH_DIALOG_MAX_WIDTH, NEW_BRANCH_DIALOG_HEIGHT)?;
    let input_row = dialog_content_row(frame, 1);
    let name_column = input_row
        .x
        .saturating_add(u16::try_from(NEW_BRANCH_INPUT_LABEL.chars().count()).unwrap_or(u16::MAX))
        .min(input_row.right());
    Some(NewBranchDialogLayout {
        frame,
        input_row,
        name_column,
    })
}

pub fn render_new_branch_dialog(area: Rect, view: NewBranchDialogView<'_>, buffer: &mut Buffer) {
    let Some(layout) = new_branch_dialog_layout(area) else {
        return;
    };
    let theme = theme_palette(view.theme);
    fill_rect(layout.frame, dialog_panel_style(&theme), buffer);
    draw_dialog_border(
        layout.frame,
        NEW_BRANCH_DIALOG_TITLE,
        view.caps,
        &theme,
        buffer,
    );
    let label = sanitize_display_string(NEW_BRANCH_INPUT_LABEL, layout.input_row.x, view.caps);
    draw_text_row(layout.input_row, &label, dialog_panel_style(&theme), buffer);
    let name_area = Rect::new(
        layout.name_column,
        layout.input_row.y,
        layout.input_row.right().saturating_sub(layout.name_column),
        1,
    );
    let name = sanitize_display_string(view.name, name_area.x, view.caps);
    draw_text_row(
        name_area,
        &name,
        Style::default().fg(theme.text).bg(theme.panel),
        buffer,
    );
}

#[cfg(test)]
mod new_branch_dialog_tests {
    use gitdeck_caps::{CapsProfile, ThemeId};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    use super::{
        new_branch_dialog_layout, render_new_branch_dialog, NewBranchDialogView,
        NEW_BRANCH_DIALOG_HEIGHT, NEW_BRANCH_DIALOG_TITLE, NEW_BRANCH_INPUT_LABEL,
    };

    fn row_text(buffer: &Buffer, row: Rect) -> String {
        (row.x..row.right())
            .filter_map(|column| buffer.cell((column, row.y)).map(|cell| cell.symbol()))
            .collect()
    }

    #[test]
    fn the_dialog_titles_itself_and_shows_the_typed_name() {
        for (width, height) in [(120, 32), (80, 24)] {
            let area = Rect::new(0, 0, width, height);
            let layout = new_branch_dialog_layout(area).expect("the dialog fits the screen");
            assert_eq!(layout.frame.height, NEW_BRANCH_DIALOG_HEIGHT);
            assert!(layout.frame.bottom() <= height);
            let mut buffer = Buffer::empty(area);
            render_new_branch_dialog(
                area,
                NewBranchDialogView {
                    name: "feature/new",
                    caps: CapsProfile::Rich,
                    theme: ThemeId::JbDark,
                },
                &mut buffer,
            );
            let title_row = row_text(
                &buffer,
                Rect::new(layout.frame.x, layout.frame.y, layout.frame.width, 1),
            );
            assert!(
                title_row.contains(NEW_BRANCH_DIALOG_TITLE),
                "title row was {title_row:?}"
            );
            let input = row_text(&buffer, layout.input_row);
            assert!(
                input.starts_with(&format!("{NEW_BRANCH_INPUT_LABEL}feature/new")),
                "input row was {input:?}"
            );
        }
    }

    #[test]
    fn an_empty_name_leaves_the_input_row_blank_after_the_label() {
        let area = Rect::new(0, 0, 120, 32);
        let layout = new_branch_dialog_layout(area).expect("the dialog fits");
        let mut buffer = Buffer::empty(area);
        render_new_branch_dialog(
            area,
            NewBranchDialogView {
                name: "",
                caps: CapsProfile::Basic,
                theme: ThemeId::JbDark,
            },
            &mut buffer,
        );
        assert_eq!(
            row_text(&buffer, layout.input_row).trim_end(),
            NEW_BRANCH_INPUT_LABEL.trim_end()
        );
    }
}
