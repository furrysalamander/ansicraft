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

            // Build the output escape sequence
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
