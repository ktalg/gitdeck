use gitdeck_app::App;
use gitdeck_testkit::prelude::*;

const NARROW_WIDTH: u16 = 80;
const NARROW_HEIGHT: u16 = 24;
const UNICODE_ELLIPSIS: char = '…';
const ASCII_ELLIPSIS: char = '~';

fn narrow_terminal(caps: CapsProfile) -> TestTerminal<App> {
    let mut terminal = TestTerminal::for_fixture(
        FixtureRepo::Dirty,
        NARROW_WIDTH,
        NARROW_HEIGHT,
        caps,
        ThemeId::JbDark,
        |repository_path| {
            App::with_repository(repository_path, caps, ThemeId::JbDark)
                .expect("the dirty fixture repository opens")
        },
    );
    terminal.resize(NARROW_WIDTH, NARROW_HEIGHT);
    terminal
}

#[test]
fn basic_caps_render_ascii_only_ellipsis() {
    let terminal = narrow_terminal(CapsProfile::Basic);
    let screen = terminal.screen_text();
    assert!(
        screen.contains(ASCII_ELLIPSIS),
        "the basic profile truncates with the ascii ellipsis, screen was:\n{screen}"
    );
    assert!(
        !screen.contains(UNICODE_ELLIPSIS),
        "the basic profile never draws the unicode ellipsis, screen was:\n{screen}"
    );
    terminal.screenshot("regressions/basic-ascii-ellipsis");
}

#[test]
fn rich_caps_keep_the_unicode_ellipsis() {
    let terminal = narrow_terminal(CapsProfile::Rich);
    let screen = terminal.screen_text();
    assert!(
        screen.contains(UNICODE_ELLIPSIS),
        "the rich profile truncates with the unicode ellipsis, screen was:\n{screen}"
    );
    assert!(
        !screen.contains(ASCII_ELLIPSIS),
        "the rich profile keeps its ascii text free of the ascii ellipsis, screen was:\n{screen}"
    );
}
