use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};

use crate::config::TerminalSize;
use crate::{render, xdo};
use crossterm::terminal::{self, BeginSynchronizedUpdate, Clear, EndSynchronizedUpdate};
use crossterm::{self, cursor, event, queue};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

#[derive(Clone)]
pub struct MinecraftConfig {
    pub xorg_display: String,
    pub username: String,
    pub server_address: String,
}

// TODO: Maybe I should put this in the render crate...?
fn display_render_thread<Writer: std::io::Write + Send + 'static>(
    completed_frames: mpsc::Receiver<String>,
    output_channel: Arc<Mutex<Writer>>,
) -> io::Result<()> {
    let mut writer = output_channel.lock().expect("Failed to lock mutex");

    // To be clear, I really don't think these (or the cleanup commands) belong here...
    // but I'm not quite proficient enough with rust's borrow checker to understand
    // how to put them higher up.  Maybe later.
    crossterm::execute!(
        writer,
        event::EnableMouseCapture,
        terminal::EnterAlternateScreen,
        cursor::Hide
    )?;

    loop {
        match completed_frames.recv_timeout(Duration::from_millis(1)) {
            Ok(frame) => {
                queue!(writer, BeginSynchronizedUpdate)?;
                // I wonder if we want to add a clear here.
                writer.write(frame.as_bytes())?;
                queue!(
                    writer,
                    Clear(crossterm::terminal::ClearType::FromCursorDown)
                )?;
                queue!(writer, EndSynchronizedUpdate)?;
                writer.flush()?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    crossterm::execute!(
        writer,
        event::DisableMouseCapture,
        terminal::LeaveAlternateScreen,
        cursor::Show,
    )?;

    Ok(())
}

fn run_minecraft(config: MinecraftConfig, running: Arc<AtomicBool>) -> io::Result<()> {
    use std::process::Command;

    // Set the DISPLAY environment variable based on config.xorg_display
    let display_env = config.xorg_display.clone();

    // Find the Python script location relative to the current executable
    let launch_script = "/root/launch_minecraft.py";

    // Build command with proper arguments
    let mut cmd = Command::new("python3");
    cmd.arg(launch_script)
        .arg("--username")
        .arg(&config.username)
        .env("DISPLAY", &display_env);

    // Add server address if specified and not empty
    if !config.server_address.is_empty() {
        cmd.arg("--server").arg(&config.server_address);
    }

    // Redirect standard output and error
    // cmd.stdout(Stdio::piped())
    //    .stderr(Stdio::piped());

    // Execute the command
    println!(
        "Launching Minecraft with username: {} on display: {}",
        config.username, display_env
    );
    if !config.server_address.is_empty() {
        println!("Connecting to server: {}", config.server_address);
    }

    // Start the command but don't wait for it to complete
    let child = cmd.spawn()?;
    let pid = child.id();

    println!("Minecraft launched (PID: {})", pid);

    // Create a separate thread to manage the minecraft process
    let minecraft_process_running = running.clone();
    thread::spawn(move || {
        let mut process = child;

        // Check if we should terminate the process
        while minecraft_process_running.load(Ordering::SeqCst) {
            // Check if process has exited on its own
            match process.try_wait() {
                Ok(Some(status)) => {
                    println!("Minecraft process exited with status: {}", status);
                    break;
                }
                Ok(None) => {
                    // Process still running, sleep and check again
                    thread::sleep(Duration::from_millis(500));
                }
                Err(e) => {
                    eprintln!("Error checking Minecraft process status: {}", e);
                    break;
                }
            }
        }

        // Ensure the running flag is set to false when shutting down
        if minecraft_process_running.load(Ordering::SeqCst) {
            minecraft_process_running.store(false, Ordering::SeqCst);
        }

        println!("Shutting down minecraft.");

        // Check if process is still running before sending signals
        match process.try_wait() {
            Ok(Some(status)) => {
                println!("Minecraft process already exited with status: {}", status);
            }
            Ok(None) => {
                // Process is still running, try SIGTERM first
                println!("Sending SIGTERM to Minecraft process (PID: {})...", pid);
                if let Err(e) = signal::kill(Pid::from_raw(process.id() as i32), Signal::SIGTERM) {
                    println!("Could not send SIGTERM to process: {}", e);
                } else {
                    // Wait for up to 5 seconds for the process to exit gracefully
                    let mut terminated = false;
                    for _ in 0..10 {
                        thread::sleep(Duration::from_millis(500));
                        match process.try_wait() {
                            Ok(Some(status)) => {
                                println!(
                                    "Minecraft process exited gracefully with status: {}",
                                    status
                                );
                                terminated = true;
                                break;
                            }
                            Ok(None) => continue, // Still running
                            Err(e) => {
                                eprintln!("Error checking process status: {}", e);
                                break;
                            }
                        }
                    }

                    // If process is still alive, force kill it
                    if !terminated {
                        println!("Process didn't exit after SIGTERM, attempting to kill...");
                        match process.kill() {
                            Ok(_) => println!("Successfully terminated Minecraft process."),
                            Err(e) => eprintln!("Failed to terminate Minecraft process: {}", e),
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error checking Minecraft process status: {}", e);
            }
        }
    });

    Ok(())
}

pub fn run<Writer: std::io::Write + Send + 'static, Reader: std::io::Read + Send + 'static>(
    config: MinecraftConfig,
    running: Arc<AtomicBool>,
    output_channel: Arc<Mutex<Writer>>,
    input_channel: Arc<Mutex<Reader>>,
    terminal_size: Arc<Mutex<TerminalSize>>,
) -> io::Result<()> {
    // First, launch Minecraft in the background
    run_minecraft(config.clone(), running.clone())?;

    let (completed_frames_tx, completed_frames_rx) = mpsc::sync_channel(1);
    let (input_event_tx, input_event_rx) = mpsc::channel();

    let mut children = vec![];

    // Clone Arc for each thread
    let running_render = Arc::clone(&running);
    let running_input = Arc::clone(&running);
    let running_forward = Arc::clone(&running);
    let terminal_size_render = Arc::clone(&terminal_size);
    let terminal_size_forward = Arc::clone(&terminal_size);
    let display_for_forward = config.xorg_display.clone();
    let display_for_ffmpeg = config.xorg_display.clone();

    children.push(thread::spawn(move || {
        render::render_x11_window(
            completed_frames_tx,
            terminal_size_render,
            display_for_ffmpeg,
            running_render,
        )
    }));
    children.push(thread::spawn(move || {
        display_render_thread(completed_frames_rx, output_channel)
    }));
    children.push(thread::spawn(move || {
        xdo::capture_input(input_channel, input_event_tx, running_input)
    }));
    children.push(thread::spawn(move || {
        xdo::forward_input_to_minecraft(
            input_event_rx,
            terminal_size_forward,
            running_forward,
            display_for_forward,
            config.server_address == "",
        )
    }));

    for child in children {
        // Wait for the thread to finish. Returns a result.
        let _ = child.join();
    }

    Ok(())
}
