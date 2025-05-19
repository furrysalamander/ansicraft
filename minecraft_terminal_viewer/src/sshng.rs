use std::{any, io::Write};

use crate::ssh;
use anyhow::Ok;
use russh::{self, server::Server};

pub struct MinecraftSshServer {}

impl MinecraftSshServer {
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
pub struct MinecraftClientSession {}

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

    async fn auth_none(&mut self, user: &str) -> Result<russh::server::Auth, Self::Error> {
        Ok(russh::server::Auth::reject())
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &russh::keys::ssh_key::PublicKey) -> Result<russh::server::Auth, Self::Error> {
        // Save the username and public key to a database.
        Ok(russh::server::Auth::Accept)
    }

    // async fn auth_keyboard_interactive<'a>(
    //     &'a mut self,
    //     user: &str,
    //     submethods: &str,
    //     response: Option<russh::server::Response<'a>>,
    // ) -> Result<russh::server::Auth, Self::Error> {
    //     Ok(russh::server::Auth::reject())
    // }

    async fn auth_succeeded(
        &mut self,
        session: &mut russh::server::Session,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

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

    async fn channel_open_session(
        &mut self,
        channel: russh::Channel<russh::server::Msg>,
        session: &mut russh::server::Session,
    ) -> Result<bool, Self::Error> {
        session.handle().data(channel.id(), russh::CryptoVec::from("goodbye\r\n".to_owned())).await;
        channel.close().await?;
        Ok(true)
    }

    fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        session: &mut russh::server::Session,
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
    }
}
