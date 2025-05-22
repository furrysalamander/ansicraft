use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    future::Future,
    pin::Pin,
};

use crate::{queueing::{self, ResourceAllocator, ResourcePool}, ssh};
use russh::{self, keys::PublicKeyBase64, server::Server};
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

pub struct MinecraftClientSession {
    username: String,
    allocator: ResourceAllocator,
    my_request_id: Option<usize>,
    my_x_session: Option<u32>,
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
        mut status_rx: mpsc::UnboundedReceiver<queueing::ResourceStatus>,
        username: String,
        session_handle: russh::server::Handle,
        channel_id: russh::ChannelId,
    ) {
        let mut position_interval = tokio::time::interval(std::time::Duration::from_secs(3));

        loop {
            tokio::select! {
                Some(status) = status_rx.recv() => {
                    match status {
                        queueing::ResourceStatus::Success(resource_id) => {
                            let _ = session_handle
                                .data(channel_id, format!("✅ Assigned session {}\r\n", resource_id).into())
                                .await;

                            // Simulate session duration (replace with actual logic)
                            tokio::time::sleep(std::time::Duration::from_secs(15)).await;

                            let _ = session_handle.data(
                                channel_id,
                                russh::CryptoVec::from(format!("goodbye {}\r\n", username))
                            ).await;
                            let _ = session_handle.close(channel_id).await;

                            // allocator.release(resource_id).await;
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
        tokio::spawn(Self::handle_session_background(
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
        self.username = public_key.public_key_base64();
        self.username.truncate(12);

        Ok(russh::server::Auth::Accept)
    }

    async fn authentication_banner(&mut self) -> Result<Option<String>, Self::Error> {
        Ok(Some(
            "If you are unable to log in, please be sure to generate a public key first.\n".to_owned(),
        ))
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
