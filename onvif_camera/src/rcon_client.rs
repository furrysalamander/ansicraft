use std::env;
use std::time::Duration;
use anyhow::{Context, Result};

pub struct RconClient {
    host: String,
    port: u16,
    password: String,
}

impl RconClient {
    pub fn from_env() -> Result<Self> {
        let host = env::var("RCON_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("RCON_PORT")
            .unwrap_or_else(|_| "25575".to_string())
            .parse()
            .context("Failed to parse RCON_PORT")?;
        let password = env::var("RCON_PASSWORD").unwrap_or_else(|_| "minecraft".to_string());

        Ok(Self {
            host,
            port,
            password,
        })
    }

    pub async fn connect(&self) -> Result<rcon::Connection> {
        let address = format!("{}:{}", self.host, self.port);
        tracing::info!("Connecting to Minecraft RCON at {}", address);

        <rcon::Connection>::builder()
            .enable_minecraft_quirks(true)
            .connect(&address, &self.password)
            .await
            .context("Failed to connect to RCON server")
    }

    pub async fn teleport_player(&self, username: &str, x: i32, y: i32, z: i32) -> Result<()> {
        let mut conn = self.connect().await?;

        let command = format!("/tp {} {} {} {}", username, x, y, z);
        tracing::info!("Sending RCON command: {}", command);

        let response = conn
            .cmd(&command)
            .await
            .context("Failed to execute teleport command")?;

        tracing::info!("RCON response: {}", response);
        Ok(())
    }

    pub async fn wait_and_teleport(
        &self,
        username: &str,
        x: i32,
        y: i32,
        z: i32,
        max_attempts: u32,
    ) -> Result<()> {
        // Wait for player to join, then teleport
        // This is a simple implementation that retries the teleport command
        // A more sophisticated approach would poll /list and wait for the player to appear

        tracing::info!(
            "Waiting for player '{}' to join before teleporting...",
            username
        );

        for attempt in 1..=max_attempts {
            tokio::time::sleep(Duration::from_secs(2)).await;

            match self.teleport_player(username, x, y, z).await {
                Ok(_) => {
                    tracing::info!("Successfully teleported player after {} attempts", attempt);
                    return Ok(());
                }
                Err(e) => {
                    if attempt < max_attempts {
                        tracing::debug!(
                            "Teleport attempt {} failed (player may not have joined yet): {}",
                            attempt,
                            e
                        );
                    } else {
                        tracing::warn!(
                            "Failed to teleport player after {} attempts: {}",
                            max_attempts,
                            e
                        );
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn send_command(&self, command: &str) -> Result<String> {
        let mut conn = self.connect().await?;

        tracing::debug!("Sending RCON command: {}", command);

        let response = conn
            .cmd(command)
            .await
            .context("Failed to execute command")?;

        tracing::debug!("RCON response: {}", response);
        Ok(response)
    }
}
