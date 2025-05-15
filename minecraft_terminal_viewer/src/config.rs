
use termwiz::input::{KeyEvent, MouseEvent};

// Game's native resolution
pub const GAME_WIDTH: u16 = 640;
pub const GAME_HEIGHT: u16 = 480;

// Platform-specific ffmpeg binary
#[cfg(target_os = "windows")]
pub const FFMPEG_BINARY: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
pub const FFMPEG_BINARY: &str = "ffmpeg";

// Terminal size information
#[derive(Clone)]
pub struct TerminalSize {
    pub width: u16,
    pub height: u16,
    pub target_width: usize,
    pub target_height: usize,
}

// Input events enum to handle both keyboard and mouse
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}
