pub mod graph;
pub mod repo;

pub use graph::{assign_lanes, LaneRow, LaneSpan};
pub use repo::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PaneId {
    Changes,
    Log,
    Details,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    F(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct KeyInput {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MouseInputKind {
    Down,
    Up,
    Drag,
    Move,
    ScrollUp,
    ScrollDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MouseInput {
    pub kind: MouseInputKind,
    pub button: MouseButton,
    pub column: u16,
    pub row: u16,
    pub modifiers: KeyModifiers,
    pub click_count: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputEvent {
    Key(KeyInput),
    Mouse(MouseInput),
    Resize(Size),
    Paste(String),
    Tick,
}

#[cfg(test)]
mod input_event_tests {
    use super::{
        InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInput, MouseInputKind, Size,
    };

    #[test]
    fn paste_event_compares_by_string_content() {
        assert_eq!(
            InputEvent::Paste(String::from("修复: 冲突")),
            InputEvent::Paste(String::from("修复: 冲突"))
        );
        assert_ne!(
            InputEvent::Paste(String::from("修复")),
            InputEvent::Paste(String::from("冲突"))
        );
    }

    #[test]
    fn key_modifiers_default_is_no_modifier_pressed() {
        assert_eq!(
            KeyModifiers::default(),
            KeyModifiers {
                ctrl: false,
                alt: false,
                shift: false
            }
        );
    }

    #[test]
    fn key_event_distinguishes_code_and_modifiers() {
        let with_ctrl = InputEvent::Key(KeyInput {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers {
                ctrl: true,
                alt: false,
                shift: false,
            },
        });
        let without_ctrl = InputEvent::Key(KeyInput {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::default(),
        });
        assert_ne!(with_ctrl, without_ctrl);
    }

    #[test]
    fn mouse_event_distinguishes_kind_and_position() {
        let base = MouseInput {
            kind: MouseInputKind::Down,
            button: MouseButton::Left,
            column: 3,
            row: 7,
            modifiers: KeyModifiers::default(),
            click_count: 1,
        };
        assert_eq!(InputEvent::Mouse(base), InputEvent::Mouse(base));
        assert_ne!(
            InputEvent::Mouse(base),
            InputEvent::Mouse(MouseInput {
                kind: MouseInputKind::Up,
                ..base
            })
        );
        assert_ne!(
            InputEvent::Mouse(base),
            InputEvent::Mouse(MouseInput { column: 4, ..base })
        );
        assert_ne!(
            InputEvent::Mouse(base),
            InputEvent::Mouse(MouseInput {
                click_count: 2,
                ..base
            })
        );
    }

    #[test]
    fn resize_event_compares_by_size() {
        assert_eq!(
            InputEvent::Resize(Size {
                width: 160,
                height: 48
            }),
            InputEvent::Resize(Size {
                width: 160,
                height: 48
            })
        );
        assert_ne!(
            InputEvent::Resize(Size {
                width: 160,
                height: 48
            }),
            InputEvent::Resize(Size {
                width: 80,
                height: 24
            })
        );
    }
}
