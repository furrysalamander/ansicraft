use onvif_camera::run;
use onvif_camera::rcon_client::RconClient;
use std::env;
use minecraft_terminal_viewer::{MinecraftConfig, launch_minecraft};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut verbose = false;
    for arg in args {
        if arg == "-v" || arg == "--verbose" {
            verbose = true;
        }
    }
    // Also support VERBOSE env var
    if env::var("VERBOSE").is_ok() {
        verbose = true;
    }

    // Parse Minecraft configuration from environment variables
    let username = env::var("USERNAME").unwrap_or_else(|_| "camera_player".to_string());
    let server_address = env::var("MINECRAFT_SERVER").unwrap_or_else(|_| "".to_string());
    let xorg_display = env::var("DISPLAY").unwrap_or_else(|_| ":1".to_string());

    // Parse spawn coordinates
    let spawn_x: i32 = env::var("SPAWN_X")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .unwrap_or(0);
    let spawn_y: i32 = env::var("SPAWN_Y")
        .unwrap_or_else(|_| "70".to_string())
        .parse()
        .unwrap_or(70);
    let spawn_z: i32 = env::var("SPAWN_Z")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .unwrap_or(0);

    // Launch Minecraft
    tracing::info!("Launching Minecraft as user '{}' on display '{}'", username, xorg_display);
    if !server_address.is_empty() {
        tracing::info!("Connecting to Minecraft server: {}", server_address);
    }

    let minecraft_config = MinecraftConfig {
        xorg_display,
        username: username.clone(),
        server_address: server_address.clone(),
    };

    let _minecraft_running = launch_minecraft(minecraft_config)
        .expect("Failed to launch Minecraft");

    // If connecting to a server, wait and then teleport player to spawn position
    if !server_address.is_empty() {
        tracing::info!("Will teleport player to ({}, {}, {})", spawn_x, spawn_y, spawn_z);

        // Spawn RCON teleport task
        let username_clone = username.clone();
        tokio::spawn(async move {
            // Wait a bit longer for Minecraft to fully launch and connect
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

            match RconClient::from_env() {
                Ok(rcon) => {
                    // Try to teleport with retries (player may not have joined yet)
                    if let Err(e) = rcon
                        .wait_and_teleport(&username_clone, spawn_x, spawn_y, spawn_z, 10)
                        .await
                    {
                        tracing::error!("Failed to teleport player: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create RCON client: {}", e);
                }
            }
        });
    } else {
        // Give Minecraft a moment to start (no server connection)
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }

    // Start ONVIF camera server (this will block)
    run(8080, verbose).await;
}
