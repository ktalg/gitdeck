use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScreenshotReport {
    pub png_path: PathBuf,
    pub text_path: PathBuf,
    pub ansi_path: PathBuf,
    pub bytes_sha256: String,
    pub missing_glyphs: Vec<char>,
}
