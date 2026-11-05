use gitdeck_core::{KeyCode, KeyInput, KeyModifiers};

pub(crate) fn parse_key_sequence(keys: &str) -> Vec<KeyInput> {
    keys.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|token| !token.is_empty())
        .map(parse_key_token)
        .collect()
}

fn parse_key_token(token: &str) -> KeyInput {
    let segments: Vec<&str> = token.split('+').collect();
    let (modifier_segments, key_segments) = segments.split_at(segments.len() - 1);
    let key_segment = key_segments[0];
    if key_segment.is_empty() {
        panic!("unrecognized key token {token:?}");
    }
    let mut modifiers = KeyModifiers::default();
    for modifier_segment in modifier_segments {
        match modifier_segment.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" | "meta" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            _ => panic!("unrecognized key modifier {modifier_segment:?} in key token {token:?}"),
        }
    }
    KeyInput {
        code: shift_key_code(parse_key_code(token, key_segment), modifiers.shift),
        modifiers,
    }
}

fn shift_key_code(code: KeyCode, shift: bool) -> KeyCode {
    if !shift {
        return code;
    }
    match code {
        KeyCode::Char(character) if character.is_ascii_alphabetic() => {
            KeyCode::Char(character.to_ascii_uppercase())
        }
        other => other,
    }
}

fn parse_key_code(token: &str, key_segment: &str) -> KeyCode {
    let mut characters = key_segment.chars();
    if let (Some(single_character), None) = (characters.next(), characters.next()) {
        return KeyCode::Char(single_character);
    }
    let named_key = key_segment.to_ascii_lowercase();
    match named_key.as_str() {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "space" => KeyCode::Space,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "page-up" => KeyCode::PageUp,
        "pagedown" | "page-down" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        other => match parse_function_key_number(other) {
            Some(number) => KeyCode::F(number),
            None => panic!("unrecognized key token {token:?}"),
        },
    }
}

fn parse_function_key_number(named_key: &str) -> Option<u8> {
    let digits = named_key.strip_prefix('f')?;
    let number = digits.parse::<u8>().ok()?;
    (1..=12).contains(&number).then_some(number)
}

#[cfg(test)]
mod key_sequence_tests {
    use gitdeck_core::{KeyCode, KeyInput, KeyModifiers};

    use super::parse_key_sequence;

    fn key_input(code: KeyCode, modifiers: KeyModifiers) -> KeyInput {
        KeyInput { code, modifiers }
    }

    fn ctrl() -> KeyModifiers {
        KeyModifiers {
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    #[test]
    fn ctrl_letter_token_becomes_ctrl_char_event() {
        assert_eq!(
            parse_key_sequence("ctrl+k"),
            vec![key_input(KeyCode::Char('k'), ctrl())]
        );
    }

    #[test]
    fn alt_letter_token_becomes_alt_char_event() {
        assert_eq!(
            parse_key_sequence("alt+g"),
            vec![key_input(
                KeyCode::Char('g'),
                KeyModifiers {
                    ctrl: false,
                    alt: true,
                    shift: false,
                }
            )]
        );
    }

    #[test]
    fn shift_tab_token_becomes_shifted_tab_event() {
        assert_eq!(
            parse_key_sequence("shift+tab"),
            vec![key_input(
                KeyCode::Tab,
                KeyModifiers {
                    ctrl: false,
                    alt: false,
                    shift: true,
                }
            )]
        );
    }

    #[test]
    fn function_keys_f1_to_f12_are_accepted() {
        for number in 1..=12u8 {
            assert_eq!(
                parse_key_sequence(&format!("f{number}")),
                vec![key_input(KeyCode::F(number), KeyModifiers::default())],
                "function key f{number}"
            );
        }
    }

    #[test]
    fn named_keys_map_to_their_key_codes() {
        let named_keys = [
            ("enter", KeyCode::Enter),
            ("esc", KeyCode::Esc),
            ("tab", KeyCode::Tab),
            ("backtab", KeyCode::BackTab),
            ("space", KeyCode::Space),
            ("backspace", KeyCode::Backspace),
            ("delete", KeyCode::Delete),
            ("insert", KeyCode::Insert),
            ("home", KeyCode::Home),
            ("end", KeyCode::End),
            ("pageup", KeyCode::PageUp),
            ("pagedown", KeyCode::PageDown),
            ("up", KeyCode::Up),
            ("down", KeyCode::Down),
            ("left", KeyCode::Left),
            ("right", KeyCode::Right),
        ];
        for (token, expected_code) in named_keys {
            assert_eq!(
                parse_key_sequence(token),
                vec![key_input(expected_code, KeyModifiers::default())],
                "token {token}"
            );
        }
    }

    #[test]
    fn shifted_letter_tokens_arrive_uppercase_with_the_shift_modifier() {
        let shifted = KeyModifiers {
            ctrl: false,
            alt: false,
            shift: true,
        };
        assert_eq!(
            parse_key_sequence("shift+h"),
            vec![key_input(KeyCode::Char('H'), shifted)]
        );
        assert_eq!(
            parse_key_sequence("shift+space"),
            vec![key_input(KeyCode::Space, shifted)]
        );
    }

    #[test]
    fn single_character_tokens_are_sent_verbatim() {
        let single_character_tokens = [("a", 'a'), ("1", '1'), (":", ':'), ("A", 'A')];
        for (token, expected_character) in single_character_tokens {
            assert_eq!(
                parse_key_sequence(token),
                vec![key_input(
                    KeyCode::Char(expected_character),
                    KeyModifiers::default()
                )],
                "token {token}"
            );
        }
    }

    #[test]
    fn comma_and_space_separate_key_tokens() {
        assert_eq!(
            parse_key_sequence("ctrl+k,ctrl+c"),
            vec![
                key_input(KeyCode::Char('k'), ctrl()),
                key_input(KeyCode::Char('c'), ctrl())
            ]
        );
        assert_eq!(
            parse_key_sequence("g i"),
            vec![
                key_input(KeyCode::Char('g'), KeyModifiers::default()),
                key_input(KeyCode::Char('i'), KeyModifiers::default())
            ]
        );
    }

    #[test]
    fn empty_sequence_produces_no_events() {
        assert!(parse_key_sequence("").is_empty());
        assert!(parse_key_sequence("  ").is_empty());
    }

    #[test]
    #[should_panic(expected = "\"nope\"")]
    fn unknown_named_key_panics_with_the_original_token() {
        parse_key_sequence("nope");
    }

    #[test]
    #[should_panic(expected = "\"f13\"")]
    fn function_key_above_f12_panics_with_the_original_token() {
        parse_key_sequence("f13");
    }

    #[test]
    #[should_panic(expected = "\"hyper+k\"")]
    fn unknown_modifier_panics_with_the_original_token() {
        parse_key_sequence("hyper+k");
    }

    #[test]
    #[should_panic(expected = "\"ctrl+\"")]
    fn trailing_plus_panics_with_the_original_token() {
        parse_key_sequence("ctrl+");
    }
}
