use std::{
    io::{Read, Write}, path::Path, sync::{
        Arc, Mutex,
    }
};

use crate::{
    minecraft,
    queueing::{self, ResourceAllocator, ResourcePool},
};

use anyhow;
use rand_core::OsRng;
use russh::{self, keys::{ssh_key::{self, public}, PublicKeyBase64}, server::Server};
use tokio::sync::mpsc;

const MAX_SIMULTANEOUS_SESSIONS: u32 = 10;

// Function to load or create SSH key
pub fn load_or_create_ssh_key() -> russh::keys::PrivateKey {
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
            keys: vec![load_or_create_ssh_key()],
            nodelay: true,
            channel_buffer_size: 1,
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
    terminal_size: Arc<Mutex<crate::config::TerminalSize>>, // Store terminal size for resize events
    input_channel_tx: mpsc::UnboundedSender<Vec<u8>>,
    input_channel_rx: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl Server for MinecraftSshServer {
    type Handler = MinecraftClientSession;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        // Create allocator from pool for each new client
        let allocator = ResourceAllocator::new(&self.x_server_pool);

        let (input_channel_tx, input_channel_rx) = mpsc::unbounded_channel();

        MinecraftClientSession {
            username: "".to_owned(),
            allocator,
            my_request_id: None,
            my_x_session: None, // Sooo, due to the clone semantics, I'm pretty sure that this causes the session to not get cleaned up by drop because it only gets added after the clone happens.  Some arc/mutex action can fix this.  I'll deal with it later.
            terminal_size: Arc::new(Mutex::new(crate::config::TerminalSize {
                target_width: 10,
                target_height: 10,
            })),
            input_channel_tx,
            input_channel_rx: Arc::new(Mutex::new(input_channel_rx)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {
        // Log or handle session errors as needed
    }
}

impl MinecraftClientSession {
    fn cleanup_resources(&mut self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);

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

    fn set_terminal_size(&mut self, width: u32) -> anyhow::Result<()> {
        let mut size = self.terminal_size.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock terminal size mutex: {}", e))?;
        size.target_width = width as usize;
        size.target_height = crate::render::get_height_from_width(width as usize);
        Ok(())
    }

    pub async fn handle_session_background(
        self,
        mut status_rx: mpsc::UnboundedReceiver<queueing::ResourceStatus>,
        username: String,
        session_handle: russh::server::Handle,
        channel_id: russh::ChannelId,
    ) {
        let mut queue_position_interval = tokio::time::interval(std::time::Duration::from_secs(3));

        loop {
            tokio::select! {
                Some(status) = status_rx.recv() => {
                    match status {
                        queueing::ResourceStatus::Success(resource_id) => {
                            let _ = session_handle
                                .data(channel_id, format!("✅ Assigned session {}\r\n", resource_id).into())
                                .await;

                            // Get Minecraft server address from environment variable if set
                            let server_address = std::env::var("MINECRAFT_SERVER_ADDRESS").unwrap_or_else(|_| "".to_string());
                            let minecraft_config = minecraft::MinecraftConfig { xorg_display: format!(":{}", resource_id+1), username: username.clone(), server_address };

                            // Output: send Minecraft output to SSH client
                            let output_channel = Arc::new(Mutex::new(SessionWriter::new(session_handle.clone(), channel_id)));
                            // Input: receive input from SSH client
                            let input_channel = Arc::new(Mutex::new(SessionReader::new(self.input_channel_rx.clone())));

                            // Run the Minecraft session (blocking call)
                            tokio::spawn(async move {
                                minecraft::run(
                                    minecraft_config,
                                    self.running.clone(),
                                    output_channel,
                                    input_channel,
                                    self.terminal_size.clone(),
                                ).unwrap();

                                let _ = session_handle.close(channel_id).await;
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
                                .data(channel_id, format!("⏳ You are position {} in queue\r\n", pos + 1).into())
                                .await;
                        }
                        queueing::ResourceStatus::Cancelled => {
                            let _ = session_handle
                                .data(channel_id, "❌ Request was cancelled\r\n".into())
                                .await;
                            break;
                        }
                        queueing::ResourceStatus::Failed(reason) => {
                            let _ = session_handle
                                .data(channel_id, format!("❌ Server error: {}\r\n", reason).into())
                                .await;
                            break;
                        }
                    }
                },
                _ = queue_position_interval.tick() => {
                    // No-op: status updates come from ResourceAllocator now
                }
            }
        }
    }
}

impl russh::server::Handler for MinecraftClientSession {
    type Error = anyhow::Error;

    async fn channel_close(
            &mut self,
            _channel: russh::ChannelId,
            _session: &mut russh::server::Session,
        ) -> Result<(), Self::Error> {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    async fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        session: &mut russh::server::Session,
    ) -> Result<bool, Self::Error> {
        let username = self.username.clone();
        let session_handle = session.handle().clone();
        let channel_id = channel.id();

        // We have to run this as a background task because the channel won't work until this function returns.
        tokio::spawn(self.clone().handle_session_background(
            self.allocator.request_resource(),
            username,
            session_handle,
            channel_id,
        ));

        Ok(true)
    }

    async fn auth_publickey(
        &mut self,
        _user: &str,
        public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<russh::server::Auth, Self::Error> {
        // Use first 12 characters of base64 public key as username for now
        let public_key = public_key
            .public_key_base64();
        self.username = sha256::digest(public_key);
        self.username.truncate(12);

        Ok(russh::server::Auth::Accept)
    }

    async fn authentication_banner(&mut self) -> Result<Option<String>, Self::Error> {
        Ok(Some(
            "If you are unable to log in, please be sure to generate a public key first.\n"
                .to_owned(),
        ))
    }

    async fn pty_request(
        &mut self,
        _channel: russh::ChannelId,
        _term: &str,
        col_width: u32,
        _row_height: u32,
        _pix_width: u32, // TODO MAKE THIS SUPPORT PIXEL MOUSE COORDS!!!!!
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        _session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        self.set_terminal_size(col_width)
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
        self.set_terminal_size(col_width)
    }

    async fn data(
        &mut self,
        _channel: russh::ChannelId,
        data: &[u8],
        _session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        if let Err(e) = self.input_channel_tx.send(data.to_owned()) {
            eprintln!("Failed to send data: {}", e);
        }
        Ok(())
    }
}

impl Drop for MinecraftClientSession {
    fn drop(&mut self) {
        self.cleanup_resources();
    }
}

// Stub for SessionWriter - to be implemented
struct SessionWriter {
    session_handle: russh::server::Handle,
    channel_id: russh::ChannelId,
    buffer: Vec<u8>,
}

impl SessionWriter {
    fn new(session_handle: russh::server::Handle, channel_id: russh::ChannelId) -> Self {
        Self {
            session_handle,
            channel_id,
            buffer: vec![],
        }
    }
}

impl Write for SessionWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Send data to SSH client (blocking)
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        // Handle any errors from the SSH session
        if let Err(e) = futures::executor::block_on(
            self.session_handle
                .data(self.channel_id, self.buffer.clone().into()), // This is the actual write
        ) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("SSH session error: {:?}", e),
            ));
        }
        self.buffer.clear();
        Ok(())
    }
}

// There's probably a cleaner way to do this without the SessionReader struct, but this works for now.
struct SessionReader {
    buffer: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
}

impl SessionReader {
    fn new(buffer: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>) -> Self {
        Self { buffer }
    }
}

impl Read for SessionReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Lock the receiver for exclusive access
        let mut receiver = self.buffer.lock().unwrap();
        match receiver.try_recv() {
            Ok(data) => {
                let to_copy = std::cmp::min(buf.len(), data.len());
                buf[..to_copy].copy_from_slice(&data[..to_copy]);
                Ok(to_copy)
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, ""))
            }
            Err(mpsc::error::TryRecvError::Disconnected) => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Channel closed",
            )),
        }
    }
}
