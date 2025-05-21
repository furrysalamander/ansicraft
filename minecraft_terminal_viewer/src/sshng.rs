use std::{collections::VecDeque, sync::{atomic::{AtomicUsize, Ordering}, Arc}, vec};

use crate::ssh;
// use anyhow::Ok;
use russh::{self, keys::PublicKeyBase64, server::Server};
use tokio::sync::{mpsc, oneshot, watch};

const MAX_SIMULTANEOUS_SESSIONS: u32 = 2;

#[derive(Clone)]
pub struct MinecraftSshServerHandle {
    request_tx: mpsc::Sender<XSessionRequest>,
    release_tx: mpsc::Sender<u32>,
    queue_state_rx: watch::Receiver<Vec<usize>>,
    next_id: Arc<AtomicUsize>,
}

impl MinecraftSshServerHandle {
    pub fn from(server: &MinecraftSshServer) -> Self {
        Self {
            request_tx: server.request_tx.clone(),
            release_tx: server.release_tx.clone(),
            queue_state_rx: server.queue_state_tx.subscribe(),
            next_id: Arc::clone(&server.next_id),
        }
    }

    pub async fn request(&self) -> (usize, oneshot::Receiver<u32>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        let req = XSessionRequest { id, response: tx };
        let _ = self.request_tx.send(req).await;
        (id, rx)
    }

    pub async fn release(&self, x: u32) {
        let _ = self.release_tx.send(x).await;
    }

    pub fn subscribe_queue(&self) -> watch::Receiver<Vec<usize>> {
        self.queue_state_rx.clone()
    }
}


pub struct MinecraftSshServer {
    request_tx: mpsc::Sender<XSessionRequest>,
    release_tx: mpsc::Sender<u32>,
    queue_state_tx: watch::Sender<Vec<usize>>, // Broadcast queue IDs

    next_id: Arc<AtomicUsize>,
}

struct XSessionRequest {
    id: usize,
    response: oneshot::Sender<u32>,
}

impl MinecraftSshServer {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<XSessionRequest>(100);
        let (release_tx, release_rx) = mpsc::channel(100);
        let (queue_state_tx, _) = watch::channel(Vec::new());
        let available_resources = VecDeque::from((0..MAX_SIMULTANEOUS_SESSIONS).collect::<Vec<_>>());
        let pending_requests: VecDeque<XSessionRequest> = VecDeque::new();
        let next_id = Arc::new(AtomicUsize::new(0));
        let tx_for_watch = queue_state_tx.clone();

        // Move the queue manager to its own function
        tokio::spawn(Self::resource_queue_manager(
            available_resources,
            pending_requests,
            request_rx,
            release_rx,
            tx_for_watch,
        ));

        Self {
            request_tx,
            release_tx,
            queue_state_tx,
            next_id,
        }
    }

    async fn resource_queue_manager(
        mut available_resources: VecDeque<u32>,
        mut pending_requests: VecDeque<XSessionRequest>,
        mut request_rx: mpsc::Receiver<XSessionRequest>,
        mut release_rx: mpsc::Receiver<u32>,
        tx_for_watch: watch::Sender<Vec<usize>>,
    ) {
        loop {
            tokio::select! {
                Some(id) = release_rx.recv() => {
                    if let Some(req) = pending_requests.pop_front() {
                        let _ = req.response.send(id);
                    } else {
                        available_resources.push_back(id);
                    }
                }
                Some(req) = request_rx.recv() => {
                    if let Some(id) = available_resources.pop_front() {
                        let _ = req.response.send(id);
                    } else {
                        pending_requests.push_back(req);
                    }
                }
                else => break,
            }
            // Update queue state
            let ids: Vec<usize> = pending_requests.iter().map(|r| r.id).collect();
            let _ = tx_for_watch.send(ids);
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

        self.run_on_address(std::sync::Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }
}
pub struct MinecraftClientSession {
    username: String,

    allocator: MinecraftSshServerHandle,
    my_request_id: Option<usize>,
    my_x_session: Option<u32>,
}

impl russh::server::Server for MinecraftSshServer {
    type Handler = MinecraftClientSession;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        MinecraftClientSession {
            username: "".to_owned(),
            allocator: MinecraftSshServerHandle::from(self),
            my_request_id: None,
            my_x_session: None,
        }
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {

    }    
}    

impl MinecraftClientSession {
    // Add a method to cleanup resources if client disconnects
    fn cleanup_resources(&mut self) {
        // If we have a pending request but no assigned session,
        // we need to remove the request from the queue
        if self.my_request_id.is_some() && self.my_x_session.is_none() {
            // We can't actually remove from queue directly, but we'll handle
            // this differently in a more comprehensive solution
        }
        
        // If we have an assigned resource, release it
        if let Some(resource) = self.my_x_session.take() {
            // Use a non-async version for Drop context
            let release_tx = self.allocator.release_tx.clone();
            if let Ok(()) = release_tx.try_send(resource) {
                // Resource released successfully
            }
        }
    }

    async fn handle_session_background(
        allocator: MinecraftSshServerHandle,
        queue_rx: tokio::sync::watch::Receiver<Vec<usize>>,
        username: String,
        session_handle: russh::server::Handle,
        channel_id: russh::ChannelId,
    ) {
        let (request_id, mut rx) = allocator.request().await;
        let mut position_interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            tokio::select! {
                resource = &mut rx => {
                    match resource {
                        Ok(resource_id) => {
                            let _ = session_handle
                                .data(channel_id, format!("✅ Assigned session {}\r\n", resource_id).into())
                                .await;
                            // Simulate session work
                            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                            let _ = session_handle.data(
                                channel_id, 
                                russh::CryptoVec::from(format!("goodbye {}\r\n", username))
                            ).await;
                            let _ = session_handle.close(channel_id).await;
                            allocator.release(resource_id).await;
                            break;
                        },
                        Err(_) => {
                            let _ = session_handle.data(
                                channel_id, 
                                "❌ Server error: Failed to get a resource assignment\r\n".into()
                            ).await;
                            break;
                        }
                    }
                }
                _ = position_interval.tick() => {
                    if queue_rx.has_changed().unwrap_or(false) {
                        let queue_snapshot = queue_rx.borrow().clone();
                        if let Some(position) = queue_snapshot.iter().position(|&i| i == request_id) {
                            let _ = session_handle
                                .data(channel_id, 
                                    format!("⏳ You are position {} in queue\r\n", position + 1).into())
                                .await;
                        } else {
                            let _ = session_handle
                                .data(channel_id, "⏳ Waiting for a session...\r\n".into())
                                .await;
                        }
                    }
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
        let allocator = self.allocator.clone();
        let queue_rx = allocator.subscribe_queue();
        let username = self.username.clone();
        let session_handle = session.handle().clone();
        let channel_id = channel.id();

        tokio::spawn(Self::handle_session_background(
            allocator,
            queue_rx,
            username,
            session_handle,
            channel_id,
        ));
        Ok(true)
    }
    
    async fn auth_publickey(
        &mut self,
        _user: &str,
        public_key: &russh::keys::ssh_key::PublicKey) -> Result<russh::server::Auth, Self::Error> {
            // Maybe eventually we would want to use the client's username along with the hash, but not yet.    
            self.username = public_key.public_key_base64();
            // Truncating usernames to the first 12 digits of a public key.
            self.username.truncate(12);

        Ok(russh::server::Auth::Accept)
    }    
    
    // async fn auth_none(&mut self, _user: &str) -> Result<russh::server::Auth, Self::Error> {
    //     Ok(russh::server::Auth::reject())    
    // }

    // async fn auth_keyboard_interactive<'a>(
    //     &'a mut self,    
    //     user: &str,
    //     submethods: &str,
    //     response: Option<russh::server::Response<'a>>,
    // ) -> Result<russh::server::Auth, Self::Error> {
    //     Ok(russh::server::Auth::reject())    
    // }

    // async fn auth_succeeded(
    //     &mut self,    
    //     _session: &mut russh::server::Session,
    // ) -> Result<(), Self::Error> {
    //     Ok(())    
    // }

    async fn authentication_banner(
        &mut self,
    ) -> Result<Option<String>, Self::Error> {
        Ok(Some(
            "If you are unable to log in, please be sure to generate a public key first.\n"
                .to_owned(),
        ))        
        // async { Ok(None) }
    }    

    // fn channel_close(
    //     &mut self,    
    //     channel: russh::ChannelId,
    //     session: &mut russh::server::Session,
    // ) -> impl Future<Output = Result<(), Self::Error>> + Send {
    //     async { Ok(()) }    
    // }

    // fn channel_eof(
    //     &mut self,    
    //     channel: russh::ChannelId,
    //     session: &mut russh::server::Session,
    // ) -> impl Future<Output = Result<(), Self::Error>> + Send {
    //     async { Ok(()) }    
    // }
    fn data(
        &mut self,
        _channel: russh::ChannelId,
        _data: &[u8],
        _session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }

    // fn pty_request(
    //     &mut self,
    //     channel: russh::ChannelId,
    //     term: &str,
    //     col_width: u32,
    //     row_height: u32,
    //     pix_width: u32,
    //     pix_height: u32,
    //     modes: &[(russh::Pty, u32)],
    //     session: &mut russh::server::Session,
    // ) -> impl Future<Output = Result<(), Self::Error>> + Send {
    //     async { Ok(()) }
    // }

    // fn window_change_request(
    //     &mut self,
    //     channel: russh::ChannelId,
    //     col_width: u32,
    //     row_height: u32,
    //     pix_width: u32,
    //     pix_height: u32,
    //     session: &mut russh::server::Session,
    // ) -> impl Future<Output = Result<(), Self::Error>> + Send {
    //     async { Ok(()) }
    // }

    // fn signal(
    //     &mut self,
    //     channel: russh::ChannelId,
    //     signal: russh::Sig,
    //     session: &mut russh::server::Session,
    // ) -> impl Future<Output = Result<(), Self::Error>> + Send {
    //     async { Ok(()) }
    // }
}

// impl std::io::Write for MinecraftClientSession {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         Ok(0)
//     }

//     fn flush(&mut self) -> std::io::Result<()> {
//         Ok(())
//     }
// }

impl Drop for MinecraftClientSession {
    fn drop(&mut self) {
        self.cleanup_resources();
    }
}
