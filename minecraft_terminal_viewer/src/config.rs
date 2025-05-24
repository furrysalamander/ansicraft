// Game's native resolution
pub const GAME_WIDTH: u16 = 320;
pub const GAME_HEIGHT: u16 = 200;

// Platform-specific ffmpeg binary
#[cfg(target_os = "windows")]
pub const FFMPEG_BINARY: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
pub const FFMPEG_BINARY: &str = "ffmpeg";

// Terminal size information
#[derive(Clone, Default)]
pub struct TerminalSize {
    pub target_width: usize,
    pub target_height: usize,
}
