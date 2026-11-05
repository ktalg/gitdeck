mod ansi_palette;
mod fixture;
mod glyph_drawing;
mod hashing;
mod key_sequence;
mod manifest;
mod paths;
mod pixel_baseline;
mod pty;
mod raster;
mod scratch;
mod screen_text;
mod screenshot_report;
mod terminal;
mod theme_palette;

pub use fixture::{
    fixture_git_command, fixture_repo, FixtureRepo, FIXTURE_USER_EMAIL, FIXTURE_USER_NAME,
};
pub use gitdeck_caps::{CapsProfile, ThemeId};
pub use gitdeck_tui::ScreenDriver;
pub use manifest::ScreenshotManifest;
pub use paths::repository_root;
pub use pixel_baseline::{PixelDiff, PixelTolerance};
pub use pty::{key_token_bytes, PtyScreenCell, PtySession, PtySessionOptions};
pub use ratatui::layout::Rect;
pub use scratch::{copy_directory, scratch_directory};
pub use screenshot_report::ScreenshotReport;
pub use terminal::{TestCell, TestTerminal};

pub mod prelude {
    pub use crate::{
        copy_directory, fixture_git_command, fixture_repo, repository_root, scratch_directory,
        CapsProfile, FixtureRepo, PixelDiff, PixelTolerance, Rect, ScreenDriver,
        ScreenshotManifest, ScreenshotReport, TestCell, TestTerminal, ThemeId, FIXTURE_USER_EMAIL,
        FIXTURE_USER_NAME,
    };
    pub use gitdeck_core::{
        InputEvent, KeyCode, KeyInput, KeyModifiers, MouseButton, MouseInput, MouseInputKind,
        PaneId, Size,
    };
    pub use ratatui::style::Color;
}
