use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CapsProfile {
    Rich,
    Standard,
    Basic,
}

impl CapsProfile {
    pub const EVERY: [Self; 3] = [Self::Rich, Self::Standard, Self::Basic];

    pub fn parse_cli(cli_value: &str) -> Option<Self> {
        match cli_value {
            "rich" => Some(Self::Rich),
            "standard" => Some(Self::Standard),
            "basic" => Some(Self::Basic),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rich => "rich",
            Self::Standard => "standard",
            Self::Basic => "basic",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CapsSource {
    Cli,
    Environment,
    Detected,
}

impl CapsSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Environment => "env",
            Self::Detected => "detected",
        }
    }
}

const RICH_TERM_PROGRAMS: [&str; 5] = [
    "iterm.app",
    "wezterm",
    "ghostty",
    "vscode",
    "apple_terminal",
];

const RICH_COLORTERM_VALUES: [&str; 2] = ["truecolor", "24bit"];

const LOW_COLOR_TERMS: [&str; 4] = ["dumb", "vt100", "ansi", "linux"];

pub fn detect(environment: &BTreeMap<String, String>) -> CapsProfile {
    if environment
        .get("NO_COLOR")
        .is_some_and(|value| !value.is_empty())
    {
        return CapsProfile::Basic;
    }
    if environment
        .get("TERM")
        .is_some_and(|value| selects_low_color_terminal(value))
    {
        return CapsProfile::Basic;
    }
    if environment
        .get("COLORTERM")
        .is_some_and(|value| value_matches_any_ignore_case(value, &RICH_COLORTERM_VALUES))
    {
        return CapsProfile::Rich;
    }
    if environment
        .get("TERM_PROGRAM")
        .is_some_and(|value| value_matches_any_ignore_case(value, &RICH_TERM_PROGRAMS))
    {
        return CapsProfile::Rich;
    }
    if environment
        .get("TERM")
        .is_some_and(|value| value_contains_ignore_case(value, "screen"))
    {
        return CapsProfile::Standard;
    }
    if environment
        .get("TERM")
        .is_some_and(|value| value_contains_ignore_case(value, "256color"))
    {
        return CapsProfile::Standard;
    }
    CapsProfile::Standard
}

fn selects_low_color_terminal(value: &str) -> bool {
    value_contains_ignore_case(value, "dumb")
        || value_matches_any_ignore_case(value, &LOW_COLOR_TERMS)
}

fn value_matches_any_ignore_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn value_contains_ignore_case(value: &str, needle: &str) -> bool {
    value.to_ascii_lowercase().contains(needle)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ThemeId {
    JbDark,
    JbLight,
    Solarized,
    Nord,
    HighContrast,
    Mono,
}

impl ThemeId {
    pub fn parse_cli(cli_value: &str) -> Option<Self> {
        match cli_value {
            "jb-dark" => Some(Self::JbDark),
            "jb-light" => Some(Self::JbLight),
            "solarized" => Some(Self::Solarized),
            "nord" => Some(Self::Nord),
            "high-contrast" => Some(Self::HighContrast),
            "mono" => Some(Self::Mono),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::JbDark => "jb-dark",
            Self::JbLight => "jb-light",
            Self::Solarized => "solarized",
            Self::Nord => "nord",
            Self::HighContrast => "high-contrast",
            Self::Mono => "mono",
        }
    }
}

#[cfg(test)]
mod caps_profile_cli_tests {
    use super::CapsProfile;

    const EVERY_CAPS_PROFILE: [CapsProfile; 3] =
        [CapsProfile::Rich, CapsProfile::Standard, CapsProfile::Basic];

    #[test]
    fn caps_profile_parse_cli_accepts_documented_names() {
        let accepted_names = [
            ("rich", Some(CapsProfile::Rich)),
            ("standard", Some(CapsProfile::Standard)),
            ("basic", Some(CapsProfile::Basic)),
        ];
        for (cli_value, expected_profile) in accepted_names {
            assert_eq!(
                CapsProfile::parse_cli(cli_value),
                expected_profile,
                "cli value {cli_value}"
            );
        }
    }

    #[test]
    fn caps_profile_parse_cli_rejects_unknown_names() {
        let rejected_names = ["", "Rich", "RICH", "rich ", " basic", "auto", "dumb"];
        for cli_value in rejected_names {
            assert_eq!(
                CapsProfile::parse_cli(cli_value),
                None,
                "cli value {cli_value}"
            );
        }
    }

    #[test]
    fn caps_profile_as_str_round_trips_through_parse_cli() {
        for caps_profile in EVERY_CAPS_PROFILE {
            assert_eq!(
                CapsProfile::parse_cli(caps_profile.as_str()),
                Some(caps_profile)
            );
        }
    }
}

#[cfg(test)]
mod caps_detection_tests {
    use std::collections::BTreeMap;

    use super::{detect, CapsProfile};

    fn environment(variables: &[(&str, &str)]) -> BTreeMap<String, String> {
        variables
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn empty_environment_selects_standard() {
        assert_eq!(detect(&environment(&[])), CapsProfile::Standard);
        assert_eq!(
            detect(&environment(&[("NO_COLOR", ""), ("COLORTERM", "")])),
            CapsProfile::Standard,
            "empty values do not select a profile"
        );
    }

    #[test]
    fn no_color_forces_basic() {
        let cases = [
            vec![("NO_COLOR", "1")],
            vec![("NO_COLOR", "0")],
            vec![("NO_COLOR", "1"), ("COLORTERM", "truecolor")],
            vec![
                ("NO_COLOR", "1"),
                ("TERM", "xterm-256color"),
                ("TERM_PROGRAM", "WezTerm"),
            ],
        ];
        for variables in cases {
            assert_eq!(
                detect(&environment(&variables)),
                CapsProfile::Basic,
                "environment {variables:?}"
            );
        }
    }

    #[test]
    fn term_dumb_forces_basic() {
        let cases = [
            vec![("TERM", "dumb")],
            vec![("TERM", "xterm-dumb")],
            vec![("TERM", "dumb"), ("COLORTERM", "truecolor")],
        ];
        for variables in cases {
            assert_eq!(
                detect(&environment(&variables)),
                CapsProfile::Basic,
                "environment {variables:?}"
            );
        }
    }

    #[test]
    fn colorterm_truecolor_selects_rich() {
        for colorterm in ["truecolor", "24bit"] {
            assert_eq!(
                detect(&environment(&[("COLORTERM", colorterm)])),
                CapsProfile::Rich,
                "COLORTERM {colorterm}"
            );
            assert_eq!(
                detect(&environment(&[
                    ("COLORTERM", colorterm),
                    ("TERM", "xterm-256color"),
                ])),
                CapsProfile::Rich,
                "COLORTERM {colorterm} outranks the 256 color term"
            );
        }
    }

    #[test]
    fn known_terminal_programs_select_rich() {
        for term_program in [
            "iTerm.app",
            "WezTerm",
            "ghostty",
            "vscode",
            "Apple_Terminal",
        ] {
            assert_eq!(
                detect(&environment(&[("TERM_PROGRAM", term_program)])),
                CapsProfile::Rich,
                "TERM_PROGRAM {term_program}"
            );
        }
        assert_eq!(
            detect(&environment(&[
                ("TERM_PROGRAM", "iTerm.app"),
                ("TERM", "xterm-256color"),
            ])),
            CapsProfile::Rich,
            "a known terminal program outranks the 256 color term"
        );
    }

    #[test]
    fn term_256color_selects_standard() {
        for term in ["xterm-256color", "screen-256color", "tmux-256color"] {
            assert_eq!(
                detect(&environment(&[("TERM", term)])),
                CapsProfile::Standard,
                "TERM {term}"
            );
        }
        assert_eq!(
            detect(&environment(&[
                ("TERM", "xterm-256color"),
                ("NO_COLOR", "")
            ])),
            CapsProfile::Standard,
            "an empty NO_COLOR leaves the 256 color term alone"
        );
    }

    #[test]
    fn vt100_and_ansi_select_basic() {
        for term in ["vt100", "VT100", "ansi", "ANSI"] {
            assert_eq!(
                detect(&environment(&[("TERM", term)])),
                CapsProfile::Basic,
                "TERM {term}"
            );
        }
        assert_eq!(
            detect(&environment(&[
                ("TERM", "vt100"),
                ("COLORTERM", "truecolor")
            ])),
            CapsProfile::Basic,
            "a terminal without color outranks COLORTERM"
        );
        assert_eq!(
            detect(&environment(&[
                ("TERM", "ansi"),
                ("TERM_PROGRAM", "iTerm.app")
            ])),
            CapsProfile::Basic,
            "the term family outranks a rich terminal program"
        );
    }

    #[test]
    fn linux_console_selects_basic() {
        for term in ["linux", "Linux"] {
            assert_eq!(
                detect(&environment(&[("TERM", term)])),
                CapsProfile::Basic,
                "TERM {term}"
            );
        }
    }

    #[test]
    fn plain_screen_selects_standard() {
        assert_eq!(
            detect(&environment(&[("TERM", "screen")])),
            CapsProfile::Standard
        );
        assert_eq!(
            detect(&environment(&[("TERM", "screen.xterm-new")])),
            CapsProfile::Standard
        );
        assert_eq!(
            detect(&environment(&[
                ("TERM", "screen"),
                ("COLORTERM", "truecolor")
            ])),
            CapsProfile::Rich,
            "COLORTERM still outranks the screen term"
        );
    }

    #[test]
    fn screen_256color_stays_standard() {
        assert_eq!(
            detect(&environment(&[("TERM", "screen-256color")])),
            CapsProfile::Standard
        );
        assert_eq!(
            detect(&environment(&[
                ("TERM", "screen-256color"),
                ("COLORTERM", "")
            ])),
            CapsProfile::Standard,
            "an empty COLORTERM leaves the screen term alone"
        );
    }

    #[test]
    fn unknown_terminal_selects_standard() {
        let cases = [
            vec![("TERM", "xterm")],
            vec![("TERM", "rxvt")],
            vec![("TERM_PROGRAM", "OtherTerminal")],
            vec![("COLORTERM", "16color")],
            vec![("TERM", ""), ("COLORTERM", ""), ("TERM_PROGRAM", "")],
        ];
        for variables in cases {
            assert_eq!(
                detect(&environment(&variables)),
                CapsProfile::Standard,
                "environment {variables:?}"
            );
        }
    }

    #[test]
    fn value_matching_ignores_case() {
        assert_eq!(
            detect(&environment(&[("COLORTERM", "TrueColor")])),
            CapsProfile::Rich
        );
        assert_eq!(
            detect(&environment(&[("TERM_PROGRAM", "ITERM.APP")])),
            CapsProfile::Rich
        );
        assert_eq!(
            detect(&environment(&[("TERM", "XTERM-DUMB")])),
            CapsProfile::Basic
        );
        assert_eq!(
            detect(&environment(&[("TERM", "XTerm-256Color")])),
            CapsProfile::Standard
        );
        assert_eq!(
            detect(&environment(&[("colorterm", "truecolor")])),
            CapsProfile::Standard,
            "variable names keep their posix case"
        );
    }
}

#[cfg(test)]
mod theme_id_cli_tests {
    use super::ThemeId;

    const EVERY_THEME_ID: [ThemeId; 6] = [
        ThemeId::JbDark,
        ThemeId::JbLight,
        ThemeId::Solarized,
        ThemeId::Nord,
        ThemeId::HighContrast,
        ThemeId::Mono,
    ];

    #[test]
    fn theme_id_parse_cli_accepts_documented_names() {
        let accepted_names = [
            ("jb-dark", Some(ThemeId::JbDark)),
            ("jb-light", Some(ThemeId::JbLight)),
            ("solarized", Some(ThemeId::Solarized)),
            ("nord", Some(ThemeId::Nord)),
            ("high-contrast", Some(ThemeId::HighContrast)),
            ("mono", Some(ThemeId::Mono)),
        ];
        for (cli_value, expected_theme) in accepted_names {
            assert_eq!(
                ThemeId::parse_cli(cli_value),
                expected_theme,
                "cli value {cli_value}"
            );
        }
    }

    #[test]
    fn theme_id_parse_cli_rejects_unknown_names() {
        let rejected_names = [
            "", "dark", "JbDark", "jb_dark", "jb-dark ", "gruvbox", "none",
        ];
        for cli_value in rejected_names {
            assert_eq!(ThemeId::parse_cli(cli_value), None, "cli value {cli_value}");
        }
    }

    #[test]
    fn theme_id_as_str_round_trips_through_parse_cli() {
        for theme_id in EVERY_THEME_ID {
            assert_eq!(ThemeId::parse_cli(theme_id.as_str()), Some(theme_id));
        }
    }
}
