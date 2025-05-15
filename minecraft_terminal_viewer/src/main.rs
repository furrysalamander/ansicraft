// filepath: /home/mike/source/docker-minecraft-rtsp/minecraft_terminal_viewer/src/main.rs
mod config;
mod render;
mod xdo;
mod minecraft;
mod ssh;

use config::TerminalSize;

use std::io;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::panic;

use crossterm::{
    cursor,
    event::{self},
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
#[tokio::main]
async fn main() -> io::Result<()> {
    let mut server = ssh::MinecraftClientServer::new();
    server.run().await.expect("Failed running server");


    // // Clear the terminal
    // let mut stdout = io::stdout();
    // execute!(
    //     stdout,
    //     terminal::EnterAlternateScreen,
    //     Clear(ClearType::All),
    //     cursor::Hide
    // )?;
    
    // terminal::enable_raw_mode()?;
    
    // // Enable mouse capture
    // execute!(stdout, event::EnableMouseCapture)?;
    
    // // Setup panic handler to clean up terminal even on panic
    // let original_hook = panic::take_hook();
    // panic::set_hook(Box::new(move |panic_info| {
    //     // Clean up terminal
    //     let _ = cleanup_terminal();
    //     // Then call the original panic handler
    //     original_hook(panic_info);
    // }));
    
    // // Get initial terminal size
    // let (term_width, _term_height) = size()?;
    
    // // Calculate target dimensions (must be even height for the block character approach)
    // let target_width = term_width as usize;
    // // For proper aspect ratio and block character rendering
    // let target_height = ((target_width * 9 / 16 + 1) / 2) * 2;
    
    // // Create a shared terminal size that can be updated on resize
    // let term_size = Arc::new(Mutex::new(TerminalSize {
    //     // width: term_width,
    //     // height: term_height,
    //     target_width,
    //     target_height,
    // }));
    
    // // Shared running flag to signal threads to stop
    // let running = Arc::new(AtomicBool::new(true));
    
    // // Clone Arc for each thread
    // let run_minecraft = Arc::clone(&running);
    // let terminal_size = Arc::clone(&term_size);

    // let stdout = Arc::new(Mutex::new(std::io::stdout()));
    // let stdin = Arc::new(Mutex::new(std::io::stdin()));
    
    // let render_handle = thread::spawn(move || {
    //     if let Err(e) = minecraft::run(minecraft::MinecraftConfig { xorg_display: 1, username: "docker".to_string(), server_address: "".to_string() }, run_minecraft, stdout, stdin, terminal_size) {
    //         eprintln!("Error in Minecraft thread: {:?}", e);
    //     }
    // });
    
    // // crossterm::execute!(
    // //     output_channel,
    // //     event::EnableMouseCapture,
    // //     event::EnableFocusChange,
    // //     terminal::EnterAlternateScreen,
    // //     cursor::Hide
    // // );

    // // crossterm::execute!(
    // //     output_channel,
    // //     event::DisableMouseCapture,
    // //     event::DisableFocusChange,
    // //     terminal::LeaveAlternateScreen,
    // //     cursor::Show,
    // // );

    
    // // Wait for a thread to finish (this indicates we should stop)
    // let _ = render_handle.join();
    // cleanup_terminal()?;
    
    // Ok(())
}

