use std::{
    collections::VecDeque,
    io::{Read, Write},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use crate::{minecraft, queueing::{self, ResourceAllocator, ResourcePool}, ssh};
use russh::{self, keys::PublicKeyBase64, server::{Msg, Server}};
use tokio::sync::{mpsc, oneshot};

const MAX_SIMULTANEOUS_SESSIONS: u32 = 2;

pub struct MinecraftSshServer {
    x_server_pool: ResourcePool,
}

impl MinecraftSshServer {
    pub fn new() -> Self {
        Self {
            x_server_pool: ResourcePool::new(MAX_SIMULTANEOUS_SESSIONS),
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let mut authentication_methods = russh::MethodSet::empty();
        authentication_methods.push(russh::MethodKind::PublicKey);

        let config = russh::server::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(0),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![ssh::load_or_create_ssh_key()],
            nodelay: true,
            methods: authentication_methods,
            ..Default::default()
        };

        self.run_on_address(Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MinecraftClientSession {
    allocator: ResourceAllocator,
    username: String,
    my_request_id: Option<usize>, // I think this can be eliminated
    my_x_session: Option<u32>,
    terminal_size: Option<Arc<Mutex<crate::config::TerminalSize>>>, // Store terminal size for resize events
}

impl Server for MinecraftSshServer {
    type Handler = MinecraftClientSession;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        // Create allocator from pool for each new client
        let allocator = ResourceAllocator::new(&self.x_server_pool);
        MinecraftClientSession {
            username: "".to_owned(),
            allocator,
            my_request_id: None,
            my_x_session: None,
            terminal_size: None,
        }
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {
        // Log or handle session errors as needed
    }
}

impl MinecraftClientSession {
    fn cleanup_resources(&mut self) {
        if self.my_request_id.is_some() && self.my_x_session.is_none() {
            // Currently no direct way to remove a pending request; can implement cancellation logic here later
        }

        if let Some(resource) = self.my_x_session.take() {
            // Use try_send so it works in sync Drop context
            // Gotta check for errors here
            println!("Releasing resource {}", resource);
            let _release_response = self.allocator.release(resource);
        }
    }

    pub async fn handle_session_background(
        mut self,
        mut status_rx: mpsc::UnboundedReceiver<queueing::ResourceStatus>,
        username: String,
        session_handle: russh::Channel<Msg>,
        // channel_id: russh::ChannelId,
    ) {
        let mut position_interval = tokio::time::interval(std::time::Duration::from_secs(3));

        // Prepare terminal size Arc for resize events
        let terminal_size = Arc::new(Mutex::new(crate::config::TerminalSize::default()));
        self.terminal_size = Some(terminal_size.clone());

        loop {
            tokio::select! {
                Some(status) = status_rx.recv() => {
                    match status {
                        queueing::ResourceStatus::Success(resource_id) => {
                            let _ = session_handle
                                .data(format!("✅ Assigned session {}\r\n", resource_id).as_bytes())
                                .await;

                            // Get Minecraft server address from environment variable if set
                            let server_address = std::env::var("MINECRAFT_SERVER_ADDRESS").unwrap_or_else(|_| "".to_string());
                            let minecraft_config = minecraft::MinecraftConfig { xorg_display: format!(":{}", resource_id+1), username: username.clone(), server_address };

                            // Shared running flag
                            let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
                            // Output: send Minecraft output to SSH client
                            let output_channel = Arc::new(Mutex::new(SessionWriter::new(session_handle)));
                            // Input: receive input from SSH client
                            let input_channel = Arc::new(Mutex::new(SessionReader::new(session_handle)));

                            // Run the Minecraft session (blocking call)
                            tokio::spawn(async move {
                                minecraft::run(
                                    minecraft_config,
                                    running.clone(),
                                    output_channel,
                                    input_channel,
                                    terminal_size.clone(),
                                ).unwrap();
                                
                                let _ = session_handle.close().await;
                                self.allocator.release(resource_id);

                        });
                            // let _ = session_handle.data(
                            //     channel_id,
                            //     russh::CryptoVec::from(format!("goodbye {}\r\n", username))
                            // ).await;
                            // let _ = session_handle.close(channel_id).await;
                            // self.allocator.release(resource_id);
                            break;
                        }
                        queueing::ResourceStatus::QueuePosition(pos) => {
                            let _ = session_handle
                                .data(format!("⏳ You are position {} in queue\r\n", pos + 1).as_bytes())
                                .await;
                        }
                        queueing::ResourceStatus::Cancelled => {
                            let _ = session_handle
                                .data("❌ Request was cancelled\r\n".as_bytes())
                                .await;
                            break;
                        }
                        queueing::ResourceStatus::Failed(reason) => {
                            let _ = session_handle
                                .data(format!("❌ Server error: {}\r\n", reason).as_bytes())
                                .await;
                            break;
                        }
                    }
                },
                _ = position_interval.tick() => {
                    // No-op: status updates come from ResourceAllocator now
                }
            }
        }
    }
}

impl russh::server::Handler for MinecraftClientSession {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        session: &mut russh::server::Session,
    ) -> Result<bool, Self::Error> {
        let username = self.username.clone();
        let session_handle = session.handle().clone();
        let channel_id = channel.id();
        
        // Spawn background task that handles resource allocation, queueing, and session lifecycle
        tokio::spawn(self.clone().handle_session_background(
            self.allocator.request_resource(),
            username,
            channel,
            // channel_id,
        ));

        Ok(true)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<russh::server::Auth, Self::Error> {
        // Use first 12 characters of base64 public key as username for now
        self.username = public_key.public_key_base64();
        self.username.truncate(12);

        Ok(russh::server::Auth::Accept)
    }

    async fn authentication_banner(&mut self) -> Result<Option<String>, Self::Error> {
        Ok(Some(
            "If you are unable to log in, please be sure to generate a public key first.\n".to_owned(),
        ))
    }

    async fn pty_request(
            &mut self,
            _channel: russh::ChannelId,
            _term: &str,
            col_width: u32,
            _row_height: u32,
            _pix_width: u32,
            _pix_height: u32,
            _modes: &[(russh::Pty, u32)],
            _session: &mut russh::server::Session,
        ) -> Result<(), Self::Error> {
        // Update terminal size on PTY request
        if let Some(ref term_size) = self.terminal_size {
            let mut size = term_size.lock().unwrap();
            size.target_width = col_width as usize;
            size.target_height = crate::render::get_height_from_width(col_width as usize);
        }
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _channel: russh::ChannelId,
        col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        // Update terminal size on window change
        if let Some(ref term_size) = self.terminal_size {
            let mut size = term_size.lock().unwrap();
            size.target_width = col_width as usize;
            size.target_height = crate::render::get_height_from_width(col_width as usize);
        }
        Ok(())
    }

    fn data(
        &mut self,
        _channel: russh::ChannelId,
        _data: &[u8],
        _session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}

impl Drop for MinecraftClientSession {
    fn drop(&mut self) {
        self.cleanup_resources();
    }
}

// Stub for SessionWriter - to be implemented
struct SessionWriter {
    session_handle: russh::Channel<Msg>,
}

impl SessionWriter {
    fn new(session_handle: russh::Channel<Msg>) -> Self {
        Self { session_handle }
    }
}

impl Write for SessionWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Send data to SSH client (blocking)
        // Note: This is a stub; in production, you may want to buffer or spawn a task
        // let data = russh::CryptoVec::from_slice(buf);
        // Ignore errors for now
        let _ = futures::executor::block_on(self.session_handle.data(buf));
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Stub for SessionReader - to be implemented
struct SessionReader {
    // For a real implementation, you would buffer incoming SSH data here
    // For now, this is a stub
    session_handle: russh::Channel<Msg>,
}

impl SessionReader {
    fn new(session_handle: russh::Channel<Msg>) -> Self {
        Self { session_handle }
    }
}

impl Read for SessionReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        // TODO: Implement reading from SSH client (requires buffering input from SSH data events)
        Ok(0)
    }
}
