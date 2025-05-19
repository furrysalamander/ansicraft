use std::any;

use russh::{self, server::Server};
use crate::ssh;

pub struct MinecraftSshServer {}

impl MinecraftSshServer {
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let config = russh::server::Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![ssh::load_or_create_ssh_key()],
            nodelay: true,
            methods: vec![russh::MethodKind::PublicKey].into(),
            ..Default::default()
        };
        
        self.run_on_address(std::sync::Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }
}
pub struct MinecraftClientSession {
}

impl russh::server::Server for MinecraftSshServer {
    type Handler = MinecraftClientSession;
    
    fn new_client(&mut self, peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        MinecraftClientSession {}
    }
    
    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {

    }
}

impl russh::server::Handler for MinecraftClientSession {
    type Error = anyhow::Error;

    fn auth_none(&mut self, user: &str) -> impl Future<Output = Result<russh::server::Auth, Self::Error>> + Send {
        async { Ok(russh::server::Auth::reject()) }
    }

    fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &russh::keys::ssh_key::PublicKey,
    ) -> impl Future<Output = Result<russh::server::Auth, Self::Error>> + Send {
        async { Ok(russh::server::Auth::reject()) }
    }

    fn auth_keyboard_interactive<'a>(
        &'a mut self,
        user: &str,
        submethods: &str,
        response: Option<russh::server::Response<'a>>,
    ) -> impl Future<Output = Result<russh::server::Auth, Self::Error>> + Send {
        async { Ok(russh::server::Auth::reject()) }
    }
    
    fn auth_succeeded(
        &mut self,
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
    
    fn authentication_banner(
        &mut self,
    ) -> impl Future<Output = Result<Option<String>, Self::Error>> + Send {
        async { Ok(Some("If you are unable to log in, please be sure to generate a public key first.".to_owned())) }
        // async { Ok(None) }
    }
    
    fn channel_close(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
    
    fn channel_eof(
        &mut self,
        channel: russh::ChannelId,
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
    
    fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        async { Ok(false) }
    }
    
    fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
    
    fn pty_request(
        &mut self,
        channel: russh::ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        modes: &[(russh::Pty, u32)],
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
    
    fn window_change_request(
        &mut self,
        channel: russh::ChannelId,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
    
    fn signal(
        &mut self,
        channel: russh::ChannelId,
        signal: russh::Sig,
        session: &mut russh::server::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}
