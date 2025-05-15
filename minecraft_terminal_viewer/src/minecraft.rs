use std::{io, thread};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{render, xdo};
use crate::config::TerminalSize;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{self, queue};

pub struct MinecraftConfig {
    pub xorg_display: u8,
    pub username: String,

    pub server_address: String,
}

// TODO: Maybe I should put this in the render crate...?
fn display_render_thread<Writer: std::io::Write + Send + 'static>(
    completed_frames: mpsc::Receiver<String>, 
    output_channel: Arc<Mutex<Writer>>
) -> io::Result<()> {
    loop {
        match completed_frames.recv_timeout(Duration::from_millis(1)) {
            Ok(frame) => {
                let mut writer = output_channel.lock().expect("Failed to lock mutex");
                
                queue!(writer, BeginSynchronizedUpdate)?;
                // I wonder if we want to add a clear here.
                writer.write(frame.as_bytes())?;
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
    Ok(())
}

fn run_minecraft(config: MinecraftConfig, running: Arc<AtomicBool>) -> io::Result<()> {
    use std::process::{Command, Stdio};
    
    // Set the DISPLAY environment variable based on xorg_display config
    let display_env = format!(":{}", config.xorg_display);
    
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
        cmd.arg("--server")
           .arg(&config.server_address);
    }
    
    // Redirect standard output and error
    cmd.stdout(Stdio::piped())
       .stderr(Stdio::piped());
    
    // Execute the command
    println!("Launching Minecraft with username: {} on display: {}", 
             config.username, display_env);
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
        
        // If the loop ended because running became false, kill the process
        if !minecraft_process_running.load(Ordering::SeqCst) {
            println!("Terminating Minecraft process (PID: {})...", pid);
            // Try to cleanly terminate
            let _ = process.kill();
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
    run_minecraft(config, running.clone())?;
    
    let (completed_frames_tx, completed_frames_rx) = mpsc::channel();
    let (input_event_tx, input_event_rx) = mpsc::channel();

    let mut children = vec![];

    // Clone Arc for each thread
    let running_render = Arc::clone(&running);
    let running_input = Arc::clone(&running);
    let running_forward = Arc::clone(&running);
    let terminal_size_render = Arc::clone(&terminal_size);
    let terminal_size_input = Arc::clone(&terminal_size);
    let terminal_size_forward = Arc::clone(&terminal_size);

    children.push(thread::spawn(move || {
        render::render_x11_window(completed_frames_tx, terminal_size_render, running_render)
    }));
    children.push(thread::spawn(move || {
        display_render_thread(completed_frames_rx, output_channel)
    }));
    children.push(thread::spawn(move || {
        xdo::capture_input(input_channel, input_event_tx, terminal_size_input, running_input)
    }));
    children.push(thread::spawn(move || {
        xdo::forward_input_to_minecraft(input_event_rx, terminal_size_forward, running_forward)
    }));

    for child in children {
        // Wait for the thread to finish. Returns a result.
        let _ = child.join();
    }


    Ok(())
}
