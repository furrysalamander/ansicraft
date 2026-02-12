mod config;
mod http_api;
mod minecraft;
mod queueing;
mod render;
mod rtsp_session;
mod session_manager;
mod sshng;
mod xdo;

use config::TerminalSize;
use session_manager::SessionManager;
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

const MAX_RTSP_SESSIONS: u32 = 10;

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

    // Start the HTTP API server for RTSP session management
    let session_manager = Arc::new(tokio::sync::Mutex::new(SessionManager::new(MAX_RTSP_SESSIONS)));

    // Try to start go2rtc (if available)
    match rtsp_session::spawn_go2rtc() {
        Ok(child) => {
            println!("go2rtc started successfully (PID: {})", child.id());
            // Wait for go2rtc to be ready before accepting sessions
            println!("Waiting for go2rtc to be ready...");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Verify go2rtc is responding
            let client = reqwest::Client::new();
            for i in 1..=5 {
                match client.get("http://localhost:1984/api/streams").send().await {
                    Ok(resp) if resp.status().is_success() => {
                        println!("go2rtc is ready (attempt {})", i);
                        break;
                    }
                    _ => {
                        if i < 5 {
                            println!("go2rtc not ready yet, retrying... (attempt {})", i);
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        } else {
                            println!("go2rtc may not be fully ready, continuing anyway");
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("go2rtc not available or failed to start: {}", e);
            println!("RTSP streaming will not be available without go2rtc");
        }
    }

    // Start HTTP API in background
    let http_manager = Arc::clone(&session_manager);
    let http_port = std::env::var("HTTP_API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    tokio::spawn(async move {
        http_api::run(http_manager, http_port).await;
    });

    // Indicate that the user is prompted for input, if this is a terminal.
    if !stdin.is_terminal() {
        // SSH server mode
        let mut server = sshng::MinecraftSshServer::new();
        server.run().await
    } else {
        // Interactive mode (local terminal)
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

        cleanup_terminal()?;
        Ok(())
    }
}
