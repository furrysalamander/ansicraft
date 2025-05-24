use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use termwiz::input::{InputEvent, InputParser, KeyCode, Modifiers, MouseButtons};

use crate::config::{GAME_HEIGHT, GAME_WIDTH, TerminalSize};

// Captures keyboard and mouse input using termwiz
pub fn capture_input<Reader: io::Read + Send + 'static>(
    input_channel: Arc<Mutex<Reader>>,
    input_tx: mpsc::Sender<InputEvent>,
    running: Arc<AtomicBool>,
) -> io::Result<()> {
    // let (bytes_tx, bytes_rx) = mpsc::channel::<Vec<u8>>();

    let mut reader = input_channel.lock().expect("Failed to lock mutex");
    let mut parser: InputParser = InputParser::new();
    while running.load(Ordering::SeqCst) {
        let mut buf = [0u8; 32];
        match reader.read(&mut buf) {
            Ok(0) => break,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
            Ok(n) => {
                parser.parse(
                    &buf[0..n],
                    |event| {
                        if let Err(e) = input_tx.send(event) {
                            eprintln!("Error sending event: {}", e);
                        }
                    },
                    false,
                );
            }
        }
    }

    Ok(())
}

// Forwards captured input to the Minecraft instance
pub fn forward_input_to_minecraft(
    input_rx: mpsc::Receiver<InputEvent>,
    term_size: Arc<Mutex<TerminalSize>>,
    running: Arc<AtomicBool>,
    display: String,
    absolute_mouse_mode_default: bool,
) -> io::Result<()> {
    let run_xdotool = |args: &[&str]| {
        Command::new("xdotool")
            .args(args)
            .env("DISPLAY", &display)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("Error running xdotool: {}", e);
                std::process::ExitStatus::from_raw(1)
            });
    };

    fn scale_mouse_coords(x: u16, y: u16, term_size: &TerminalSize) -> (u16, u16) {
        let scaled_x = (x as f32 / term_size.target_width as f32 * GAME_WIDTH as f32) as u16;
        let actual_height_in_pixels = term_size.target_height / 2;
        let scaled_y = (y as f32 / actual_height_in_pixels as f32 * GAME_HEIGHT as f32) as u16;
        (scaled_x, scaled_y)
    }

    fn calculate_relative_movement(
        current_x: u16,
        current_y: u16,
        last_x: u16,
        last_y: u16,
    ) -> (i32, i32) {
        let dx = current_x as i32 - last_x as i32;
        let dy = current_y as i32 - last_y as i32;
        (dx * 5, dy * 5)
    }

    #[derive(Clone)]
    struct KeyState {
        pressed: bool,
        release_time: std::time::Instant,
    }

    let mut wasd_release_time = std::collections::HashMap::new();
    wasd_release_time.insert(
        'w',
        KeyState {
            pressed: false,
            release_time: std::time::Instant::now(),
        },
    );
    wasd_release_time.insert(
        'a',
        KeyState {
            pressed: false,
            release_time: std::time::Instant::now(),
        },
    );
    wasd_release_time.insert(
        's',
        KeyState {
            pressed: false,
            release_time: std::time::Instant::now(),
        },
    );
    wasd_release_time.insert(
        'd',
        KeyState {
            pressed: false,
            release_time: std::time::Instant::now(),
        },
    );

    let mut inventory_open = absolute_mouse_mode_default;
    let mut last_mouse_x = 0u16;
    let mut last_mouse_y = 0u16;

    while running.load(Ordering::SeqCst) {
        match input_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => match event {
                InputEvent::Key(key_event) => match key_event.key {
                    KeyCode::Char(c) => match c {
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
                            inventory_open = !inventory_open;
                        }

                        'e' => {
                            inventory_open = !inventory_open;
                            run_xdotool(&["key", "e"]);
                        }

                        'c' => {
                            // Check for Ctrl+C
                            if key_event.key == KeyCode::Char('c')
                                && key_event.modifiers.contains(Modifiers::CTRL)
                            {
                                running.store(false, Ordering::SeqCst);
                                break;
                            }
                            run_xdotool(&["key", &c.to_string()])
                        }

                        'w' | 'a' | 's' | 'd' => {
                            if let Some(state) = wasd_release_time.get_mut(&c) {
                                if !state.pressed {
                                    run_xdotool(&["keydown", &c.to_string()]);
                                    state.pressed = true;
                                }
                                state.release_time =
                                    std::time::Instant::now() + Duration::from_millis(100);
                            }
                        }

                        _ => run_xdotool(&["key", &c.to_string()]),
                    },
                    KeyCode::Enter => run_xdotool(&["key", "Return"]),
                    KeyCode::UpArrow => run_xdotool(&["key", "Up"]),
                    KeyCode::DownArrow => run_xdotool(&["key", "Down"]),
                    KeyCode::RightArrow => run_xdotool(&["key", "Right"]),
                    KeyCode::LeftArrow => run_xdotool(&["key", "Left"]),
                    KeyCode::Backspace => run_xdotool(&["key", "BackSpace"]),
                    KeyCode::Escape => {
                        if inventory_open {
                            inventory_open = false;
                        }
                        run_xdotool(&["key", "Escape"]);
                    }
                    KeyCode::Tab => run_xdotool(&["key", "Tab"]),
                    KeyCode::Delete => run_xdotool(&["key", "Delete"]),
                    KeyCode::Home => run_xdotool(&["key", "Home"]),
                    KeyCode::End => run_xdotool(&["key", "End"]),
                    KeyCode::PageUp => run_xdotool(&["key", "Page_Up"]),
                    KeyCode::PageDown => run_xdotool(&["key", "Page_Down"]),
                    _ => {}
                },
                InputEvent::Mouse(mouse_event) => {
                    let term_size_value = term_size.lock().unwrap().clone();
                    let (game_x, game_y) =
                        scale_mouse_coords(mouse_event.x, mouse_event.y, &term_size_value);

                    if inventory_open {
                        run_xdotool(&["mousemove", &game_x.to_string(), &game_y.to_string()]);
                    } else if last_mouse_x > 0 && last_mouse_y > 0 {
                        let (dx, dy) =
                            calculate_relative_movement(game_x, game_y, last_mouse_x, last_mouse_y);
                        if dx != 0 || dy != 0 {
                            run_xdotool(&[
                                "mousemove_relative",
                                "--",
                                &dx.to_string(),
                                &dy.to_string(),
                            ]);
                        }
                    }

                    last_mouse_x = game_x;
                    last_mouse_y = game_y;

                    let buttons = mouse_event.mouse_buttons;
                    if buttons.contains(MouseButtons::LEFT) {
                        run_xdotool(&["mousedown", "1"]);
                    } else {
                        run_xdotool(&["mouseup", "1"]);
                    }
                    if buttons.contains(MouseButtons::RIGHT) {
                        run_xdotool(&["mousedown", "3"]);
                    } else {
                        run_xdotool(&["mouseup", "3"]);
                    }

                    // Handle wheel events
                    if buttons.contains(MouseButtons::VERT_WHEEL) {
                        if buttons.contains(MouseButtons::WHEEL_POSITIVE) {
                            run_xdotool(&["click", "4"]); // wheel up
                        } else {
                            run_xdotool(&["click", "5"]); // wheel down
                        }
                    }
                }
                _ => {}
            },
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        for (key, state) in wasd_release_time.iter_mut() {
            if state.pressed && std::time::Instant::now() >= state.release_time {
                run_xdotool(&["keyup", &key.to_string()]);
                state.pressed = false;
            }
        }
    }
    Ok(())
}
