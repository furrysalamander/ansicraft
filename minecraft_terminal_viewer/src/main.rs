// filepath: /home/mike/source/docker-minecraft-rtsp/minecraft_terminal_viewer/src/main.rs
mod config;
mod render;
mod xdo;
mod minecraft;

use config::TerminalSize;
use config::InputEvent;

use std::io;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::panic;
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType, size},
};

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
    // let (render_tx, render_rx) = mpsc::channel();
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
        xdo::forward_input_to_minecraft(input_rx, term_size_forward, running_forward);
    });

    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let stdin = Arc::new(Mutex::new(std::io::stdin()));
    
    let render_handle = thread::spawn(move || {
        minecraft::run(minecraft::MinecraftConfig { xorg_display: 1, username: "docker".to_string(), server_address: "".to_string() }, running_render, stdout, stdin, term_size_render);
    });
    

    // // need channels for:
    // //  resize events
    // //  cancellation
    // //  shutting down


    // crossterm::execute!(
    //     output_channel,
    //     event::EnableMouseCapture,
    //     event::EnableFocusChange,
    //     terminal::EnterAlternateScreen,
    //     cursor::Hide
    // );

    // crossterm::execute!(
    //     output_channel,
    //     event::DisableMouseCapture,
    //     event::DisableFocusChange,
    //     terminal::LeaveAlternateScreen,
    //     cursor::Show,
    // );

    
    // Wait for a thread to finish (this indicates we should stop)
    let _ = input_handle.join();
    
    // Signal all threads to stop
    // running.store(false, Ordering::SeqCst);
    
    // Clean up terminal
    
    // Give threads a chance to exit gracefully
    // thread::sleep(Duration::from_millis(100));
    
    // Wait for threads to finish with a timeout
    let _ = input_rx_handle.join();
    // let _ = render_rx_handle.join();  // Commented out as this thread is not being started
    let _ = render_handle.join();
    cleanup_terminal()?;
    
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
