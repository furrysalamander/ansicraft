use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{self, TerminalSize};
use crate::minecraft;
use crate::render::get_height_from_width;
use rand_core::OsRng;
use ratatui::layout::Rect;
use russh::keys::ssh_key::{self, PublicKey};
use russh::server::*;
use russh::{Channel, ChannelId, Pty};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

// Function to load or create SSH key
fn load_or_create_ssh_key() -> russh::keys::PrivateKey {
    // Honestly, maybe errors in this function should result in a panic.
    let key_path = Path::new("ssh_server_key");

    // Try to load existing key
    if key_path.exists() {
        match russh::keys::load_secret_key(key_path, None) {
            Ok(key) => {
                println!("Loaded existing SSH key");
                return key;
            }
            Err(e) => {
                eprintln!("Error loading SSH key: {:?}, generating new one", e);
            }
        }
    }
    // Generate and save new key if loading failed
    let key = russh::keys::PrivateKey::random(&mut OsRng, ssh_key::Algorithm::Ed25519).unwrap();

    match key.write_openssh_file(key_path, ssh_key::LineEnding::LF) {
        Ok(()) => {
            println!("Generated new SSH key");
        }
        Err(e) => {
            eprintln!("Error saving SSH key: {:?}", e);
        }
    }
    return key;
}

struct MinecraftInstance {
    terminal_size: Arc<std::sync::Mutex<config::TerminalSize>>,
    running: Arc<AtomicBool>,
    stdin_writer: pipe::PipeWriter,
    display: String, // Store the display string for cleanup
}

impl MinecraftInstance {
    pub fn new<W: std::io::Write + Send + 'static>(
        writer: W,
        display: String,
    ) -> MinecraftInstance {
        let (stdin_reader, stdin_writer) = pipe::pipe();

        let potato = Self {
            terminal_size: Arc::new(std::sync::Mutex::new(TerminalSize {
                target_width: 20,
                target_height: get_height_from_width(20),
            })),
            running: Arc::new(AtomicBool::new(true)),
            stdin_writer: stdin_writer,
            display: display.clone(),
        };

        let config = minecraft::MinecraftConfig {
            xorg_display: display,
            username: "docker".to_owned(),
            server_address: "".to_owned(),
        };

        let output_channel = Arc::new(std::sync::Mutex::new(writer));
        let input_channel = Arc::new(std::sync::Mutex::new(stdin_reader));

        let running_clone = Arc::clone(&potato.running);
        let terminal_size_clone = Arc::clone(&potato.terminal_size);

        tokio::spawn(async move {
            let _ = minecraft::run(
                config,
                running_clone,
                output_channel,
                input_channel,
                terminal_size_clone,
            );
        });

        potato
    }
}

struct TerminalHandle {
    sender: UnboundedSender<Vec<u8>>,
    // The sink collects the data which is finally sent to sender.
    sink: Vec<u8>,
}

impl TerminalHandle {
    async fn start(handle: Handle, channel_id: ChannelId) -> Self {
        let (sender, mut receiver) = unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                let result = handle.data(channel_id, data.into()).await;
                if result.is_err() {
                    eprintln!("Failed to send data: {:?}", result);
                }
            }
        });
        Self {
            sender,
            sink: Vec::new(),
        }
    }
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.sender.send(self.sink.clone());
        if result.is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                result.unwrap_err(),
            ));
        }

        self.sink.clear();
        Ok(())
    }
}

#[derive(Clone)]
pub struct MinecraftClientServer {
    clients: Arc<Mutex<HashMap<usize, MinecraftInstance>>>,
    id: usize,
    displays_in_use: Arc<Mutex<HashSet<String>>>, // Track displays in use
}

impl MinecraftClientServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
            displays_in_use: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    async fn get_next_available_display(&self) -> String {
        let mut displays = self.displays_in_use.lock().await;
        // Use separate X server numbers instead of screen numbers (:1, :2, :3, etc.)
        for i in 1..=10 {
            let display = format!(":{}", i);
            if !displays.contains(&display) {
                displays.insert(display.clone());
                return display;
            }
        }
        // Fallback: if all are in use, just use :1 (could also error)
        ":1".to_string()
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        // let clients = self.clients.clone();
        // tokio::spawn(async move {
        // loop {
        //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        //     for (_, (terminal, app)) in clients.lock().await.iter_mut() {
        //         app.counter += 1;

        //         terminal
        //             .draw(|f| {
        //                 let area = f.area();
        //                 f.render_widget(Clear, area);
        //                 let style = match app.counter % 3 {
        //                     0 => Style::default().fg(Color::Red),
        //                     1 => Style::default().fg(Color::Green),
        //                     _ => Style::default().fg(Color::Blue),
        //                 };
        //                 let paragraph = Paragraph::new(format!("Counter: {}", app.counter))
        //                     .alignment(ratatui::layout::Alignment::Center)
        //                     .style(style);
        //                 let block = Block::default()
        //                     .title("Press 'c' to reset the counter!")
        //                     .borders(Borders::ALL);
        //                 f.render_widget(paragraph.block(block), area);
        //             })
        //             .unwrap();
        //     }
        // }
        // });

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![load_or_create_ssh_key()],
            nodelay: true,
            ..Default::default()
        };

        self.run_on_address(Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }
}

impl russh::server::Server for MinecraftClientServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

impl russh::server::Handler for MinecraftClientServer {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let terminal_handle = TerminalHandle::start(session.handle(), channel.id()).await;

        // let backend = CrosstermBackend::new(terminal_handle);

        // // the correct viewport area will be set when the client request a pty
        // let options = TerminalOptions {
        //     viewport: ratatui::Viewport::Fixed(Rect::default()),
        // };

        // let terminal = ratatui::Terminal::with_options(backend, options)?;
        let display = self.get_next_available_display().await;

        let app = MinecraftInstance::new(terminal_handle, display.clone());

        let mut clients = self.clients.lock().await;
        clients.insert(self.id, app);

        Ok(true)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let mut clients = self.clients.lock().await;
        if let Some(instance) = clients.get_mut(&self.id) {
            instance.stdin_writer.write(data)?;
        }
        Ok(())
    }

    /// The client's window size has changed.
    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        // let rect = Rect {
        //     x: 0,
        //     y: 0,
        //     width: col_width as u16,
        //     height: row_height as u16,
        // };

        let mut clients = self.clients.lock().await;
        let instance = clients.get_mut(&self.id).unwrap();

        let mut size = instance.terminal_size.lock().unwrap();
        size.target_width = col_width as usize;
        size.target_height = get_height_from_width(col_width as usize);

        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // let rect = Rect {
        //     x: 0,
        //     y: 0,
        //     width: col_width as u16,
        //     height: row_height as u16,
        // };

        let mut clients = self.clients.lock().await;
        let instance = clients.get_mut(&self.id).unwrap();

        let mut size = instance.terminal_size.lock().unwrap();

        size.target_width = col_width as usize;
        size.target_height = get_height_from_width(col_width as usize);

        session.channel_success(channel)?;

        Ok(())
    }
}

impl Drop for MinecraftClientServer {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.clients.clone();
        let displays_in_use = self.displays_in_use.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            if let Some(instance) = clients.get_mut(&id) {
                instance.running.store(false, Ordering::SeqCst);
            }
            if let Some(instance) = clients.remove(&id) {
                // Release the display when the client disconnects
                let display = instance.display;
                displays_in_use.lock().await.remove(&display);
            }
        });
    }
}
