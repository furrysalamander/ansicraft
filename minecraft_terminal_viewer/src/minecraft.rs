use std::thread::JoinHandle;
use std::{io, thread};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::TerminalSize;
use crate::render::{cleanup_terminal, old_display_render_thread};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{self, cursor, event, execute, terminal};

struct MinecraftConfig {
    pub xorg_display: u8,
    pub username: String,

    pub server_address: String,
}

struct MinecraftInstance {
}

fn display_render_thread<Writer: std::io::Write>(
    completed_frames: mpsc::Receiver<String>, 
    output_channel: Arc<Mutex<Writer>>
) -> io::Result<()> {
    loop {
        match completed_frames.recv_timeout(Duration::from_millis(1)) {
            Ok(frame) => {
                let mut writer = output_channel.lock().expect("Failed to lock mutex");
                
                execute!(writer, BeginSynchronizedUpdate)?;
                // I wonder if we want to add a clear here.
                writer.write(frame.as_bytes())?;
                execute!(writer, EndSynchronizedUpdate)?;
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

fn process_inputs() {}

pub fn resize(x: u8, y: u8) {}

pub fn new<Writer: std::io::Write, Reader: std::io::Read>(
    config: MinecraftConfig,
    running: Arc<AtomicBool>,
    output_channel: Arc<Mutex<Writer>>,
    input_channel: Reader,
    terminal_size: Arc<Mutex<TerminalSize>>,
) {
    let (completed_frames_tx, completed_frames_rx) = mpsc::channel();

    let mut children = vec![];

    children.push(thread::spawn(move || {
        display_render_thread(completed_frames_rx, output_channel)
    }));

    crossterm::execute!(
        output_channel,
        event::EnableMouseCapture,
        event::EnableFocusChange,
        terminal::EnterAlternateScreen,
        cursor::Hide
    );



    // need channels for:
    //  resize events
    //  cancellation
    //  shutting down

    crossterm::execute!(
        output_channel,
        event::DisableMouseCapture,
        event::DisableFocusChange,
        terminal::LeaveAlternateScreen,
        cursor::Show,
    );
}
