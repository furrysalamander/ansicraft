// filepath: /home/mike/source/docker-minecraft-rtsp/minecraft_terminal_viewer/src/main.rs
mod config;
mod minecraft;
mod queueing;
mod render;
mod ssh;
mod sshng;
mod xdo;

use config::TerminalSize;
use termwiz::terminal::Terminal;

use std::io;
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use std::thread;

use crossterm::{
    cursor,
    event::{self},
    execute,
    terminal::{self, Clear, ClearType},
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
async fn main() -> anyhow::Result<()> {
    let stdin = io::stdin();

    // Indicate that the user is prompted for input, if this is a terminal.
    if !stdin.is_terminal() {
        let mut server = ssh::MinecraftClientServer::new();
        server.run().await
    } else {
        // Clear the terminal
        let mut stdout = io::stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            Clear(ClearType::All),
            cursor::Hide
        )?;

        terminal::enable_raw_mode()?;

        let stdin_arc = std::sync::Arc::new(std::sync::Mutex::new(stdin));
        let stdout_arc = std::sync::Arc::new(std::sync::Mutex::new(stdout));
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let resize_running = running.clone();

        let target_width = 40 as usize;
        let target_height = render::get_height_from_width(target_width);

        let terminal_size = Arc::new(Mutex::new(TerminalSize {
            target_width,
            target_height,
        }));
        let resize_terminal_size = terminal_size.clone();

        // Spawn a thread to poll terminal size every 50ms
        thread::spawn(move || {
            if let Ok(termwiz_caps) = termwiz::caps::Capabilities::new_from_env() {
                if let Ok(mut tw_term) = termwiz::terminal::UnixTerminal::new(termwiz_caps) {
                    while resize_running.load(std::sync::atomic::Ordering::SeqCst) {
                        if let Ok(screen_size) = tw_term.get_screen_size() {
                            let mut size = resize_terminal_size.lock().unwrap();
                            size.target_width = screen_size.cols as usize;
                            size.target_height = render::get_height_from_width(screen_size.cols as usize);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
        });

        minecraft::run(
            minecraft::MinecraftConfig {
                xorg_display: ":1".to_owned(),
                username: "docker".to_owned(),
                server_address: "".to_owned(),
            },
            running,
            stdout_arc,
            stdin_arc,
            terminal_size,
        )?;

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

        cleanup_terminal()?;
        Ok(())
    }
}
