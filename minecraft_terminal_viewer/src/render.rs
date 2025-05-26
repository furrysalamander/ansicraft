// filepath: /home/mike/source/docker-minecraft-rtsp/minecraft_terminal_viewer/src/render.rs
use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read};
use std::os::unix::io::{AsRawFd, RawFd};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;


use crate::config::{FFMPEG_BINARY, GAME_HEIGHT, GAME_WIDTH, TerminalSize};

// Helper function to set or unset nonblocking mode on a file descriptor
fn set_nonblocking(fd: RawFd, nonblocking: bool) -> io::Result<()> {
    use libc::{F_GETFL, F_SETFL, O_NONBLOCK, fcntl};

    unsafe {
        let mut flags = fcntl(fd, F_GETFL, 0);
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }

        if nonblocking {
            flags |= O_NONBLOCK;
        } else {
            flags &= !O_NONBLOCK;
        }

        if fcntl(fd, F_SETFL, flags) < 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

pub fn get_height_from_width(width: usize) -> usize {
    // TODO: dynamically get aspect ratio from config GAME_WIDTH and GAME_HEIGHT
    let target_height = ((width * 10 / 16 + 1) / 2) * 2;
    return target_height;
}

// Renders the Minecraft X11 screen directly to the terminal with resize support
pub fn render_x11_window(
    render_tx: mpsc::SyncSender<String>,
    term_size: Arc<Mutex<TerminalSize>>,
    display: String,
    running: Arc<AtomicBool>,
) -> io::Result<()> {
    let mut current_process: Option<std::process::Child> = None;
    let mut last_width = 0;
    let mut last_height = 0;

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

            // Start a new ffmpeg process with updated dimensions
            let x11_grab_args = [
                "-f",
                "x11grab",
                "-framerate",
                "30",
                "-video_size",
                &format!("{}x{}", GAME_WIDTH, GAME_HEIGHT),
                "-i",
                &display,
                "-f",
                "rawvideo",
                "-vf",
                &format!("scale={}x{},setsar=1:1", target_width, target_height),
                "-pix_fmt",
                "rgb24",
                "pipe:",
            ];

            let mut ffmpeg_process = Command::new(FFMPEG_BINARY)
                .args(&x11_grab_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::null()) // Redirect stderr to /dev/null
                .spawn()?;

            let ffmpeg_stdout = ffmpeg_process.stdout.take().unwrap();
            current_process = Some(ffmpeg_process);

            // Clone necessary channels and values for the render thread
            let render_tx_clone = render_tx.clone();
            let running_clone = Arc::clone(&running);

            // Spawn a thread to handle the rendering for this process
            let _render_thread = thread::spawn(move || {
                if let Err(e) = render_byte_stream(
                    ffmpeg_stdout,
                    target_height,
                    target_width,
                    0,
                    0,
                    render_tx_clone,
                    running_clone,
                ) {
                    eprintln!("Render error: {}", e);
                }
            });

            // Update last dimensions
            last_width = target_width;
            last_height = target_height;
        }
    }

    // Ensure the current process is killed
    if let Some(mut process) = current_process {
        let _ = process.kill();
        let _ = process.wait();
    }

    Ok(())
}

// Function to convert RGB to ANSI 256-color palette index
fn rgb_to_ansi_256(r: u8, g: u8, b: u8) -> u8 {
    // Check if this is a grayscale color
    if r == g && g == b {
        if r < 8 {
            return 16; // Near black
        }
        if r > 248 {
            return 231; // Near white
        }
        // Use grayscale ramp (232-255)
        return 232 + ((r - 8) / 10);
    }
    
    // Convert to 6×6×6 color cube (colors 16-231)
    let r_index = (r as f32 / 255.0 * 5.0).round() as u8;
    let g_index = (g as f32 / 255.0 * 5.0).round() as u8;
    let b_index = (b as f32 / 255.0 * 5.0).round() as u8;
    
    16 + 36 * r_index + 6 * g_index + b_index
}

fn frame_to_rgb_ansi(frame_data: &Vec<u8>, height: usize, width: usize, offset_x: usize, offset_y: usize) -> String {
    let mut output = String::with_capacity(13 + (height / 2) * (width * 41 + 8));
    output.push_str(&format!("\x1b[{};{}H", offset_y + 1, offset_x + 1));

    // Render the frame (iterate two rows per character)
    for row_index in (0..height).step_by(2) {
        for column_index in 0..width {
            let top_pixel_start = ((row_index * width) + column_index) * 3;
            let bottom_pixel_start = (((row_index + 1) * width) + column_index) * 3;

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
        output.push_str(&format!("\x1b[B\x1b[{}D", width));
    }
    return output;
}

fn frame_to_256_ansi(frame_data: &Vec<u8>, height: usize, width: usize, offset_x: usize, offset_y: usize) -> String {
    let mut output = String::with_capacity(13 + (height / 2) * (width * 18 + 8));
    output.push_str(&format!("\x1b[{};{}H", offset_y + 1, offset_x + 1));

    // Render the frame in ANSI art style (use half-blocks to maintain density)
    for row_index in (0..height).step_by(2) {
        for column_index in 0..width {
            let top_pixel_start = ((row_index * width) + column_index) * 3;
            let bottom_pixel_start = (((row_index + 1).min(height - 1) * width) + column_index) * 3;

            // Get RGB values for top and bottom pixels
            let r1 = frame_data[top_pixel_start];
            let g1 = frame_data[top_pixel_start + 1];
            let b1 = frame_data[top_pixel_start + 2];
            
            let r2 = frame_data[bottom_pixel_start];
            let g2 = frame_data[bottom_pixel_start + 1];
            let b2 = frame_data[bottom_pixel_start + 2];
            
            // Convert RGB to 256-color palette indices
            let bg_color = rgb_to_ansi_256(r1, g1, b1);
            let fg_color = rgb_to_ansi_256(r2, g2, b2);
            
            // Use 256-color ANSI escape sequences
            output.push_str(&format!(
                "\x1b[48;5;{}m\x1b[38;5;{}m▄",
                bg_color,
                fg_color,
            ));
        }
        output.push_str(&format!("\x1b[B\x1b[{}D", width));
    }
    return output;
}

// Renders an arbitrary bytes buffer to the terminal using non-blocking I/O
fn render_byte_stream<R: Read + AsRawFd>(
    mut buffer: R,
    height: usize,
    width: usize,
    offset_x: usize,
    offset_y: usize,
    render_tx: mpsc::SyncSender<String>,
    running: Arc<AtomicBool>,
) -> io::Result<()> {
    // One frame is (height * width * 3) bytes (RGB for each pixel)
    let frame_size = height * width * 3;

    // Set non-blocking mode on the raw file descriptor
    set_nonblocking(buffer.as_raw_fd(), true)?;

    // Use a VecDeque to store incoming frames
    let mut frame_queue = VecDeque::new();

    // Storage for the frame we'll actually process
    let mut frame_data = vec![0u8; frame_size];

    // Temporary buffer for reading data
    let mut read_buffer = vec![0u8; frame_size];
    let mut partial_buffer = Vec::with_capacity(frame_size);

    while running.load(Ordering::SeqCst) {
        // Read as much data as possible without blocking
        let mut read_something = false;

        loop {
            match buffer.read(&mut read_buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    read_something = true;

                    // Add the new data to our partial buffer
                    partial_buffer.extend_from_slice(&read_buffer[0..n]);

                    // Process complete frames from the partial buffer
                    while partial_buffer.len() >= frame_size {
                        // Extract a complete frame
                        let frame = partial_buffer.drain(0..frame_size).collect::<Vec<u8>>();

                        // Add to the queue, limiting queue size to avoid memory issues
                        frame_queue.push_back(frame);
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // No more data available right now
                    break;
                }
                Err(e) => return Err(e), // Actual error
            }
        }

        // Render the most recent frame if available
        if let Some(latest_frame) = frame_queue.pop_back() {
            // Put any remaining frames back at the end of the queue
            // This effectively drops all but the latest frame
            if !frame_queue.is_empty() {
                let dropped_count = frame_queue.len();
                frame_queue.clear();
                eprintln!("Dropping {} frames for real-time display", dropped_count);
            }

            // Copy the latest frame to our frame data buffer
            frame_data.copy_from_slice(&latest_frame);

            let mut output = frame_to_rgb_ansi(&frame_data, height, width, offset_x, offset_y);

            // Reset colors
            output.push_str("\x1b[m");

            // Send the rendered output
            if render_tx.send(output).is_err() {
                break; // Receiver dropped
            }
        } else if !read_something && partial_buffer.len() < frame_size {
            // If we didn't read anything and don't have a full frame, sleep briefly
            // to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    // Restore blocking mode before returning
    let _ = set_nonblocking(buffer.as_raw_fd(), false);

    Ok(())
}
