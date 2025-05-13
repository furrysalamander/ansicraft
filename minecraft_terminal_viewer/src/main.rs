use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::panic;
use std::time::Duration;
use std::os::unix::process::ExitStatusExt;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton},
    execute,
    terminal::{self, Clear, ClearType, size},
};

// Game's native resolution
const GAME_WIDTH: u16 = 1280;
const GAME_HEIGHT: u16 = 720;

// Platform-specific ffmpeg binary
#[cfg(target_os = "windows")]
const FFMPEG_BINARY: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
const FFMPEG_BINARY: &str = "ffmpeg";

// Terminal size information
#[derive(Clone)]
struct TerminalSize {
    width: u16,
    height: u16,
    target_width: usize,
    target_height: usize,
}

// Input events enum to handle both keyboard and mouse
enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

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
    
    // Enable mouse capture
    execute!(stdout, event::EnableMouseCapture)?;
    
    // Setup panic handler to clean up terminal even on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Clean up terminal
        let _ = cleanup_terminal();
        // Then call the original panic handler
        original_hook(panic_info);
    }));
    
    // Get initial terminal size
    let (term_width, term_height) = size()?;
    
    // Calculate target dimensions (must be even height for the block character approach)
    let target_width = term_width as usize;
    // For proper aspect ratio and block character rendering
    let target_height = ((target_width * 9 / 16 + 1) / 2) * 2;
    
    // Create a shared terminal size that can be updated on resize
    let term_size = Arc::new(Mutex::new(TerminalSize {
        width: term_width,
        height: term_height,
        target_width,
        target_height,
    }));
    
    // Shared running flag to signal threads to stop
    let running = Arc::new(AtomicBool::new(true));
    
    // Channels for communication between threads
    let (render_tx, render_rx) = mpsc::channel();
    let (input_tx, input_rx) = mpsc::channel();
    let (resize_tx, resize_rx) = mpsc::channel();
    
    // Clone Arc for each thread
    let running_input = Arc::clone(&running);
    let running_render = Arc::clone(&running);
    let running_display = Arc::clone(&running);
    let running_forward = Arc::clone(&running);
    let term_size_render = Arc::clone(&term_size);
    let term_size_input = Arc::clone(&term_size);
    let term_size_display = Arc::clone(&term_size);
    let term_size_forward = Arc::clone(&term_size);
    
    // Start the input capture thread (now also handles resize events)
    let input_handle = thread::spawn(move || {
        if let Err(e) = capture_input(input_tx, resize_tx, term_size_input, running_input) {
            eprintln!("Input capture error: {}", e);
        }
    });
    
    // Start the input forwarding thread
    let input_rx_handle = thread::spawn(move || {
        forward_input_to_minecraft(input_rx, term_size_forward, running_forward);
    });
    
    // Start the rendering thread
    let render_rx_handle = thread::spawn(move || {
        if let Err(e) = display_render_thread(render_rx, term_size_display, running_display) {
            eprintln!("Render display error: {}", e);
        }
    });
    
    // Start the Minecraft rendering thread
    let render_handle = thread::spawn(move || {
        if let Err(e) = render_minecraft_directly(render_tx, resize_rx, term_size_render, running_render) {
            eprintln!("Render error: {}", e);
        }
    });
    
    // Wait for a thread to finish (this indicates we should stop)
    let _ = input_handle.join();
    
    // Signal all threads to stop
    running.store(false, Ordering::SeqCst);
    
    // Clean up terminal
    cleanup_terminal()?;
    
    // Give threads a chance to exit gracefully
    thread::sleep(Duration::from_millis(100));
    
    // Wait for threads to finish with a timeout
    let _ = input_rx_handle.join();
    let _ = render_rx_handle.join();
    let _ = render_handle.join();
    
    Ok(())
}

// Function to clean up terminal state
fn cleanup_terminal() -> io::Result<()> {
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

// Renders the Minecraft X11 screen directly to the terminal with resize support
fn render_minecraft_directly(
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
                "-video_size", "1280x720",
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
        let mut output = String::new();
        
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
fn display_render_thread(
    render_rx: mpsc::Receiver<String>, 
    _term_size: Arc<Mutex<TerminalSize>>,
    running: Arc<AtomicBool>
) -> io::Result<()> {
    let mut stdout = io::stdout();
    
    while running.load(Ordering::SeqCst) {
        match render_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(frame) => {
                print!("{}", frame);
                stdout.flush()?;
                stdout.write("Press ` to toggle mouse mode.  Press Ctrl+C to exit.\r\n".as_bytes())?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is expected, just check if we should keep running
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed, we should exit
                break;
            }
        }
    }
    
    Ok(())
}

// Captures keyboard and mouse input using crossterm
fn capture_input(
    input_tx: mpsc::Sender<InputEvent>, 
    resize_tx: mpsc::Sender<()>,
    term_size: Arc<Mutex<TerminalSize>>,
    running: Arc<AtomicBool>
) -> io::Result<()> {
    while running.load(Ordering::SeqCst) {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key_event) => {
                    // Check for exit command (Ctrl+C)
                    if key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        running.store(false, Ordering::SeqCst);
                        break;
                    }
                    
                    // Forward all other key events directly
                    let _ = input_tx.send(InputEvent::Key(key_event));
                }
                Event::Mouse(mouse_event) => {
                    // Forward all mouse events directly
                    let _ = input_tx.send(InputEvent::Mouse(mouse_event));
                }
                Event::Resize(width, height) => {
                    // Update terminal size structure when resize occurs
                    let target_width = width as usize;
                    // Ensure height is a multiple of 2 for the block character rendering
                    let target_height = ((target_width * 9 / 16 + 1) / 2) * 2;
                    
                    // Update shared terminal size
                    {
                        let mut size = term_size.lock().unwrap();
                        size.width = width;
                        size.height = height;
                        size.target_width = target_width;
                        size.target_height = target_height;
                    }
                    
                    // Send resize event to trigger ffmpeg restart
                    let _ = resize_tx.send(());
                }
                _ => {}
            }
        }
    }
    
    Ok(())
}

// Forwards captured input to the Minecraft instance
fn forward_input_to_minecraft(
    input_rx: mpsc::Receiver<InputEvent>, 
    term_size: Arc<Mutex<TerminalSize>>,
    running: Arc<AtomicBool>
) {
    // Helper function to run xdotool commands
    fn run_xdotool(args: &[&str]) {
        Command::new("xdotool")
            .args(args)
            .env("DISPLAY", ":1")
            .status()
            .unwrap_or_else(|e| {
                eprintln!("Error running xdotool: {}", e);
                std::process::ExitStatus::from_raw(1)
            });
    }
    
    // Helper function to scale mouse coordinates from terminal to game resolution
    fn scale_mouse_coords(column: u16, row: u16, term_size: &TerminalSize) -> (u16, u16) {
        // Scale the coordinates from terminal size to game resolution
        // For width, use target_width since that's the actual number of characters
        let scaled_x = (column as f32 / term_size.target_width as f32 * GAME_WIDTH as f32) as u16;
        
        // For height, account for the fact that each character is 2 pixels tall
        // Each row in the terminal represents 2 pixels in height
        let actual_height_in_pixels = term_size.target_height / 2; // Since each character is 2 pixels tall
        let scaled_y = (row as f32 / actual_height_in_pixels as f32 * GAME_HEIGHT as f32) as u16;
        
        (scaled_x, scaled_y)
    }
    
    // Calculate relative mouse movement between current and last position
    fn calculate_relative_movement(current_x: u16, current_y: u16, last_x: u16, last_y: u16) -> (i32, i32) {
        let dx = current_x as i32 - last_x as i32;
        let dy = current_y as i32 - last_y as i32;
        (dx * 3, dy * 3) // Scale the movement for better sensitivity
    }
    
    // A struct to hold the state of the wasd key presses, and the timers for releasing them
    #[derive(Clone)]
    struct KeyState {
        pressed: bool,
        release_time: std::time::Instant,
    }
    // A map holding the time until the wasd key releases should be sent
    let mut wasd_release_time: std::collections::HashMap<char, KeyState> = std::collections::HashMap::new();
    wasd_release_time.insert('w', KeyState { pressed: false, release_time: std::time::Instant::now() });
    wasd_release_time.insert('a', KeyState { pressed: false, release_time: std::time::Instant::now() });
    wasd_release_time.insert('s', KeyState { pressed: false, release_time: std::time::Instant::now() });
    wasd_release_time.insert('d', KeyState { pressed: false, release_time: std::time::Instant::now() });
    
    // Variables for mouse mode detection and handling
    let mut inventory_open = true;
    let mut last_mouse_x = 0u16;
    let mut last_mouse_y = 0u16;
    
    // We'll only use two modes now:
    // 1. Free mouse movement (default) - using mousemove_relative for camera control
    // 2. Inventory mode - using direct mousemove for UI interaction
    //
    // We don't need the mouse mode detection anymore since we'll always use relative movement
    // when not in inventory mode.
    
    while running.load(Ordering::SeqCst) {
        match input_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => {
                match event {
                    InputEvent::Key(key_event) => {
                        match key_event.code {
                            KeyCode::Char(c) => {
                                // Special handling for space and semicolon
                                match c {
                                    ' ' => run_xdotool(&["key", "space"]),
                                    ';' => run_xdotool(&["key", "semicolon"]),
                                    '?' => run_xdotool(&["key", "question"]),
                                    '!' => run_xdotool(&["key", "exclam"]),
                                    ':' => run_xdotool(&["key", "colon"]),
                                    '"' => run_xdotool(&["key", "quotedbl"]),
                                    '\'' => run_xdotool(&["key", "apostrophe"]),
                                    '>' => run_xdotool(&["key", "greater"]),
                                    '<' => run_xdotool(&["key", "less"]),
                                    '|' => run_xdotool(&["key", "bar"]),
                                    '\\' => run_xdotool(&["key", "backslash"]),
                                    '/' => run_xdotool(&["key", "slash"]),
                                    '[' => run_xdotool(&["key", "bracketleft"]),
                                    ']' => run_xdotool(&["key", "bracketright"]),
                                    '{' => run_xdotool(&["key", "braceleft"]),
                                    '}' => run_xdotool(&["key", "braceright"]),
                                    '(' => run_xdotool(&["key", "parenleft"]),
                                    ')' => run_xdotool(&["key", "parenright"]),
                                    '+' => run_xdotool(&["key", "plus"]),
                                    '-' => run_xdotool(&["key", "minus"]),
                                    '=' => run_xdotool(&["key", "equal"]),
                                    '_' => run_xdotool(&["key", "underscore"]),
                                    ',' => run_xdotool(&["key", "comma"]),
                                    '.' => run_xdotool(&["key", "period"]),
                                    '^' => run_xdotool(&["key", "asciicircum"]),
                                    '~' => run_xdotool(&["key", "asciitilde"]),
                                    '@' => run_xdotool(&["key", "at"]),
                                    '#' => run_xdotool(&["key", "numbersign"]),
                                    '$' => run_xdotool(&["key", "dollar"]),
                                    '%' => run_xdotool(&["key", "percent"]),
                                    '&' => run_xdotool(&["key", "ampersand"]),
                                    '*' => run_xdotool(&["key", "asterisk"]),

                                    '`' => {
                                        // Remove backtick toggle functionality since we're 
                                        // always using free movement except in inventory
                                        inventory_open = !inventory_open;
                                    },
                                    
                                    'e' => {
                                        // Toggle inventory state
                                        inventory_open = !inventory_open;
                                        
                                        // Send the 'e' keypress to the game
                                        run_xdotool(&["key", "e"]);
                                    },

                                    'w' | 'a' | 's' | 'd' => {
                                        if let Some(state) = wasd_release_time.get_mut(&c) {
                                            if !state.pressed {
                                                run_xdotool(&["keydown", &c.to_string()]);
                                                state.pressed = true;
                                            }
                                            state.release_time = std::time::Instant::now() + Duration::from_millis(100);
                                        }
                                    },

                                    _ => run_xdotool(&["key", &c.to_string()]),
                                }
                            }
                            KeyCode::Backspace => {
                                run_xdotool(&["key", "BackSpace"]);
                            }
                            KeyCode::Esc => {
                                // If inventory is open, closing with Escape should restore the previous mouse mode
                                if inventory_open {
                                    inventory_open = false;
                                }
                                run_xdotool(&["key", "Escape"]);
                            }
                            KeyCode::Up => {
                                run_xdotool(&["key", "Up"]);
                            }
                            KeyCode::Down => {
                                run_xdotool(&["key", "Down"]);
                            }
                            KeyCode::Right => {
                                run_xdotool(&["key", "Right"]);
                            }
                            KeyCode::Left => {
                                run_xdotool(&["key", "Left"]);
                            }
                            KeyCode::Enter => {
                                run_xdotool(&["key", "Return"]);
                            }
                            KeyCode::Tab => {
                                run_xdotool(&["key", "Tab"]);
                            }
                            KeyCode::Delete => {
                                run_xdotool(&["key", "Delete"]);
                            }
                            KeyCode::Home => {
                                run_xdotool(&["key", "Home"]);
                            }
                            KeyCode::End => {
                                run_xdotool(&["key", "End"]);
                            }
                            KeyCode::PageUp => {
                                run_xdotool(&["key", "Page_Up"]);
                            }
                            KeyCode::PageDown => {
                                run_xdotool(&["key", "Page_Down"]);
                            }
                            // Handle other keys if needed
                            _ => {}
                        }
                    }
                    InputEvent::Mouse(mouse_event) => {
                        // Get current terminal size from the shared state
                        let term_size_value = term_size.lock().unwrap().clone();
                        
                        let (game_x, game_y) = scale_mouse_coords(mouse_event.column, mouse_event.row, &term_size_value);
                        
                        // Only two modes now:
                        // 1. Inventory open: direct mouse movement
                        // 2. Free movement: relative mouse movement for camera control
                        
                        if inventory_open {
                            // When inventory is open, always use direct mouse movement
                            run_xdotool(&["mousemove", &game_x.to_string(), &game_y.to_string()]);
                        } else {
                            // In normal gameplay, use relative mouse movement for camera control
                            if last_mouse_x > 0 && last_mouse_y > 0 {
                                let (dx, dy) = calculate_relative_movement(game_x, game_y, last_mouse_x, last_mouse_y);
                                if dx != 0 || dy != 0 {
                                    run_xdotool(&["mousemove_relative", "--", &dx.to_string(), &dy.to_string()]);
                                }
                            }
                        }
                        
                        // Update last mouse position
                        last_mouse_x = game_x;
                        last_mouse_y = game_y;
                        
                        // Handle different mouse event types
                        match mouse_event.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                // In inventory mode, we've already positioned the mouse directly
                                run_xdotool(&["mousedown", "1"]);
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                run_xdotool(&["mouseup", "1"]);
                            }
                            MouseEventKind::Down(MouseButton::Right) => {
                                // In inventory mode, we've already positioned the mouse directly
                                run_xdotool(&["mousedown", "3"]);
                            }
                            MouseEventKind::Up(MouseButton::Right) => {
                                run_xdotool(&["mouseup", "3"]);
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                // Mouse movement is already handled above based on mode
                                // No additional clicks needed for drag
                            }
                            MouseEventKind::Drag(MouseButton::Right) => {
                                // Mouse movement is already handled above based on mode
                                // No additional clicks needed for drag
                            }
                            MouseEventKind::ScrollDown => {
                                run_xdotool(&["click", "5"]);
                            }
                            MouseEventKind::ScrollUp => {
                                run_xdotool(&["click", "4"]);
                            }
                            _ => {}
                        }
                    }
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is expected, just check if we should keep running
                continue;
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed, we should exit
                break;
            }
        }
        
        // Check for key releases - this should run on every loop iteration
        for (key, state) in wasd_release_time.iter_mut() {
            if state.pressed && std::time::Instant::now() >= state.release_time {
                run_xdotool(&["keyup", &key.to_string()]);
                state.pressed = false;
            }
        }
    }
}
