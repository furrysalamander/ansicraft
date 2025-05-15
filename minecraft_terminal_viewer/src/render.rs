// filepath: /home/mike/source/docker-minecraft-rtsp/minecraft_terminal_viewer/src/render.rs
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crossterm::{
    cursor,
    event,
    execute,
    terminal::{self, Clear, ClearType},
};

use crate::config::{FFMPEG_BINARY, TerminalSize, GAME_HEIGHT, GAME_WIDTH};

// Renders the Minecraft X11 screen directly to the terminal with resize support
pub fn render_minecraft_directly(
    render_tx: mpsc::Sender<String>, 
    resize_rx: mpsc::Receiver<()>,
    term_size: Arc<Mutex<TerminalSize>>,
    running: Arc<AtomicBool>
) -> io::Result<()> {
    let mut current_process: Option<std::process::Child> = None;
    let mut last_width = 0;
    let mut last_height = 0;
    
    // Clear the terminal on startup
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All))?;
    
    while running.load(Ordering::SeqCst) {
        // Get current terminal dimensions
        let (target_width, target_height) = {
            let size = term_size.lock().unwrap();
            (size.target_width, size.target_height)
        };
        
        // Only restart ffmpeg if the dimensions actually changed
        if target_width != last_width || target_height != last_height {
            // Kill previous ffmpeg process if it exists
            if let Some(mut process) = current_process.take() {
                let _ = process.kill();
                let _ = process.wait();
            }
            
            // Clear the terminal when dimensions change
            let mut stdout = io::stdout();
            execute!(stdout, Clear(ClearType::All))?;
            
            // Start a new ffmpeg process with updated dimensions
            let x11_grab_args = [
                "-f", "x11grab",
                "-video_size", &format!("{}x{}", GAME_WIDTH, GAME_HEIGHT),
                "-i", ":1",
                "-f", "rawvideo",
                "-vf", &format!("scale={}x{},setsar=1:1", target_width, target_height),
                "-pix_fmt", "rgb24",
                "pipe:",
            ];
            
            let mut ffmpeg_process = Command::new(FFMPEG_BINARY)
                .args(&x11_grab_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::null()) // Redirect stderr to /dev/null
                .spawn()?;
            
            let stdout = ffmpeg_process.stdout.take().unwrap();
            current_process = Some(ffmpeg_process);
            
            // Clone necessary channels and values for the render thread
            let render_tx_clone = render_tx.clone();
            let running_clone = Arc::clone(&running);
            
            // Spawn a thread to handle the rendering for this process
            let _render_thread = thread::spawn(move || {
                if let Err(e) = render_byte_stream(stdout, target_height, target_width, 0, 0, render_tx_clone, running_clone) {
                    eprintln!("Render error: {}", e);
                }
            });
            
            // Update last dimensions
            last_width = target_width;
            last_height = target_height;
        }
        
        // Wait for a resize event or exit
        match resize_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(_) => continue, // Resize event received, restart ffmpeg on next loop
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                running.store(false, Ordering::SeqCst);
                break;
            }
        }
    }
    
    // Ensure the current process is killed
    if let Some(mut process) = current_process {
        let _ = process.kill();
        let _ = process.wait();
    }
    
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
    running: Arc<AtomicBool>,
) -> io::Result<()> {
    // The size of the static buffer for holding raw frame data
    let buffer_size = height * width * 3;
    
    // The buffer for holding the raw RGB values for the current frame
    let mut frame_data = vec![0u8; buffer_size];
    
    while running.load(Ordering::SeqCst) {
        // For holding the formatted escape sequence
        let mut output = String::with_capacity(13 + (height/2) * (width * 41 + 8));
        
        // Start by moving the cursor to the appropriate coordinates
        output.push_str(&format!("\x1b[{};{}H", offset_y, offset_x));
        
        // Fill the frame_data buffer with a single frame's worth of pixel information
        match buffer.read_exact(&mut frame_data) {
            Ok(_) => {
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
                if let Err(_) = render_tx.send(output) {
                    // Receiver has been dropped, we should exit
                    break;
                }
            },
            Err(_) => {
                // Error reading from buffer, we should exit
                break;
            }
        }
    }
    
    Ok(())
}

// Display the rendered frames
pub fn old_display_render_thread(
    render_rx: mpsc::Receiver<String>, 
    _term_size: Arc<Mutex<TerminalSize>>,
    running: Arc<AtomicBool>
) -> io::Result<()> {
    let mut stdout = io::stdout();
    
    while running.load(Ordering::SeqCst) {
        // Try to get the latest frame, draining any previous ones
        let frame = match render_rx.try_recv() {
            Ok(frame) => {
                // Got a frame, now drain any newer ones that might be waiting
                let mut latest = frame;
                while let Ok(newer) = render_rx.try_recv() {
                    latest = newer; // Keep only the newest frame
                }
                Some(latest)
            },
            Err(mpsc::TryRecvError::Empty) => {
                // No frames available, wait for one
                match render_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(frame) => Some(frame),
                    Err(_) => None, // Timeout or disconnected
                }
            },
            Err(mpsc::TryRecvError::Disconnected) => {
                // Channel closed, exit the loop
                break;
            }
        };
        
        // Display the frame if we got one
        if let Some(frame) = frame {
            print!("{}", frame);
            stdout.flush()?;
            stdout.write("Press ` to toggle mouse mode.  Press Ctrl+C to exit.\r\n".as_bytes())?;
        }
    }
    
    Ok(())
}

// Function to clean up terminal state
pub fn cleanup_terminal() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        event::DisableMouseCapture,
        terminal::LeaveAlternateScreen,
        cursor::Show
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}
