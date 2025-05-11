use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType},
};

const TARGET_WIDTH: usize = 80;
// The height must be a multiple of two
const TARGET_HEIGHT: usize = ((TARGET_WIDTH * 9 / 16) / 2) * 2;

// Platform-specific ffmpeg binary
#[cfg(target_os = "windows")]
const FFMPEG_BINARY: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
const FFMPEG_BINARY: &str = "ffmpeg";

// Main function with error handling
fn main() -> io::Result<()> {
    // Clear the terminal
    let mut stdout = io::stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        Clear(ClearType::All),
        cursor::Hide
    )?;
    
    terminal::enable_raw_mode()?;
    
    println!("Terminal Minecraft Viewer");
    println!("Loading Minecraft stream...");
    
    // Channels for communication between threads
    let (render_tx, render_rx) = mpsc::channel();
    let (keyboard_tx, keyboard_rx) = mpsc::channel();
    
    // Start the keyboard input capture thread
    let keyboard_handle = thread::spawn(move || {
        capture_keyboard_input(keyboard_tx).unwrap();
    });
    
    // Start the keyboard forwarding thread
    let keyboard_rx_handle = thread::spawn(move || {
        send_input_to_minecraft(keyboard_rx);
    });
    
    // Start the rendering thread
    let render_rx_handle = thread::spawn(move || {
        display_render_thread(render_rx).unwrap();
    });
    
    // Start the Minecraft rendering thread
    let render_handle = thread::spawn(move || {
        render_minecraft_directly(render_tx).unwrap();
    });
    
    // Wait for threads to finish
    keyboard_handle.join().unwrap();
    keyboard_rx_handle.join().unwrap();
    render_rx_handle.join().unwrap();
    render_handle.join().unwrap();
    
    // Restore terminal
    execute!(
        stdout,
        terminal::LeaveAlternateScreen,
        cursor::Show
    )?;
    terminal::disable_raw_mode()?;
    
    Ok(())
}

// Renders the Minecraft X11 screen directly to the terminal
fn render_minecraft_directly(render_tx: mpsc::Sender<String>) -> io::Result<()> {
    let x11_grab_args = [
        "-f", "x11grab",
        "-video_size", "1280x720",
        "-i", ":1",
        "-f", "rawvideo",
        "-vf", &format!("scale={}x{},setsar=1:1", TARGET_WIDTH, TARGET_HEIGHT),
        "-pix_fmt", "rgb24",
        "pipe:",
    ];
    
    let mut ffmpeg_process = Command::new(FFMPEG_BINARY)
        .args(&x11_grab_args)
        .stdout(Stdio::piped())
        .spawn()?;
    
    let stdout = ffmpeg_process.stdout.take().unwrap();
    
    render_byte_stream(stdout, TARGET_HEIGHT, TARGET_WIDTH, 0, 0, render_tx)?;
    
    ffmpeg_process.wait()?;
    Ok(())
}

// Renders an arbitrary bytes buffer to the terminal
fn render_byte_stream<R: Read>(
    mut buffer: R,
    height: usize,
    width: usize,
    offset_x: usize,
    offset_y: usize,
    render_tx: mpsc::Sender<String>,
) -> io::Result<()> {
    // The size of the static buffer for holding raw frame data
    let buffer_size = height * width * 3;
    
    // The buffer for holding the raw RGB values for the current frame
    let mut frame_data = vec![0u8; buffer_size];
    
    loop {
        // For holding the formatted escape sequence
        let mut output = String::new();
        
        // Start by moving the cursor to the appropriate coordinates
        output.push_str(&format!("\x1b[{};{}H", offset_y, offset_x));
        
        // Fill the frame_data buffer with a single frame's worth of pixel information
        buffer.read_exact(&mut frame_data)?;
        
        // Iterate through the frame two rows at a time
        for row_index in (0..height).step_by(2) {
            for column_index in 0..width {
                // Find the correct offset in the frame data for the current pixel
                let top_pixel_start = ((row_index * width) + column_index) * 3;
                let bottom_pixel_start = (((row_index + 1) * width) + column_index) * 3;
                
                // Populate the final buffer with a single formatted character
                output.push_str(&format!(
                    "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄",
                    frame_data[top_pixel_start],
                    frame_data[top_pixel_start + 1],
                    frame_data[top_pixel_start + 2],
                    frame_data[bottom_pixel_start],
                    frame_data[bottom_pixel_start + 1],
                    frame_data[bottom_pixel_start + 2],
                ));
            }
            
            // Move the cursor down a single row and back to the starting column
            output.push_str(&format!("\x1b[B\x1b[{}D", width));
        }
        
        // Reset the output back to standard colors
        output.push_str("\x1b[m");
        
        // Hand off the formatted string to the render thread
        render_tx.send(output).unwrap();
    }
}

// Display the rendered frames
fn display_render_thread(render_rx: mpsc::Receiver<String>) -> io::Result<()> {
    let mut stdout = io::stdout();
    
    loop {
        match render_rx.recv() {
            Ok(frame) => {
                print!("{}", frame);
                stdout.flush()?;
            }
            Err(_) => break,
        }
    }
    
    Ok(())
}

// Captures keyboard input using crossterm
fn capture_keyboard_input(keyboard_tx: mpsc::Sender<String>) -> io::Result<()> {
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match code {
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+C to exit
                        break;
                    }
                    KeyCode::Char(c) => {
                        keyboard_tx.send(c.to_string()).unwrap();
                    }
                    KeyCode::Esc => {
                        keyboard_tx.send("ESC".to_string()).unwrap();
                    }
                    KeyCode::Up => {
                        keyboard_tx.send("SPECIAL_A".to_string()).unwrap();
                    }
                    KeyCode::Down => {
                        keyboard_tx.send("SPECIAL_B".to_string()).unwrap();
                    }
                    KeyCode::Right => {
                        keyboard_tx.send("SPECIAL_C".to_string()).unwrap();
                    }
                    KeyCode::Left => {
                        keyboard_tx.send("SPECIAL_D".to_string()).unwrap();
                    }
                    KeyCode::Enter => {
                        keyboard_tx.send("\r".to_string()).unwrap();
                    }
                    // Fixed: Replace KeyCode::Space with KeyCode::Char(' ')
                    _ => {}
                }
            }
        }
    }
    
    Ok(())
}

// Forwards captured keyboard input to the Minecraft instance
fn send_input_to_minecraft(keyboard_rx: mpsc::Receiver<String>) {
    while let Ok(key) = keyboard_rx.recv() {
        // Helper function to run xdotool commands
        fn run_xdotool(args: &[&str]) {
            Command::new("xdotool")
                .args(args)
                .env("DISPLAY", ":1")
                .status()
                .unwrap();
        }
        
        match key.as_str() {
            "w" => run_xdotool(&["key", "w"]),
            "a" => run_xdotool(&["key", "a"]),
            "s" => run_xdotool(&["key", "s"]),
            "d" => run_xdotool(&["key", "d"]),
            " " => run_xdotool(&["key", "space"]),
            "SPECIAL_A" => run_xdotool(&["key", "Up"]),
            "SPECIAL_B" => run_xdotool(&["key", "Down"]),
            "SPECIAL_C" => run_xdotool(&["key", "Right"]),
            "SPECIAL_D" => run_xdotool(&["key", "Left"]),
            "ESC" => run_xdotool(&["key", "Escape"]),
            "\r" => run_xdotool(&["key", "Return"]),
            "e" => run_xdotool(&["mousemove_relative", "200", "0"]),
            "r" => run_xdotool(&["key", "e"]),
            "q" => run_xdotool(&["mousemove_relative", "--", "-200", "0"]),
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => run_xdotool(&["key", &key]),
            "b" => std::process::exit(0),
            "t" => run_xdotool(&["mouseup", "1"]),
            "g" => run_xdotool(&["mousedown", "1"]),
            _ => run_xdotool(&["click", "1"]), // Default action is to click
        }
    }
}
