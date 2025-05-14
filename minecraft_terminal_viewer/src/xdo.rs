use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::os::unix::process::ExitStatusExt;

use crossterm::event::{KeyCode, MouseButton, MouseEventKind};

use crate::config::{GAME_WIDTH, GAME_HEIGHT, TerminalSize, InputEvent};

// Forwards captured input to the Minecraft instance
pub fn forward_input_to_minecraft(
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
        (dx * 5, dy * 5) // Scale the movement for better sensitivity
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
