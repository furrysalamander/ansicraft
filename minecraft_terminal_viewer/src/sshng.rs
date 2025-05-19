use russh;

struct MinecraftSshServer {}

struct MinecraftClientSession {}

impl russh::server::Server for MinecraftSshServer {
    type Handler = MinecraftClientSession;
    
    fn new_client(&mut self, peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        todo!()
    }
    
    fn handle_session_error(&mut self, _error: <Self::Handler as russh::server::Handler>::Error) {}
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