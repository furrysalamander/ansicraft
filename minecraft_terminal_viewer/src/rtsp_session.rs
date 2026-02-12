use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::net::TcpStream;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tokio::sync::Mutex;

use crate::minecraft::MinecraftConfig;
use crate::session_manager::{ProcessState, SessionManager};

/// Start an RTSP session for a given session ID.
/// This registers the FFmpeg stream with go2rtc so viewers can connect.
/// Minecraft will be started lazily when viewers connect.
pub async fn start_rtsp_session(
    session_id: String,
    _username: String,
    x_display: u32,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<(), String> {
    let display_str = format!(":{}", x_display + 1);

    println!(
        "Registering RTSP session {} on display {} (lazy Minecraft start)",
        session_id, display_str
    );

    // Register the FFmpeg stream with go2rtc immediately
    // This allows viewers to connect, which triggers the lazy Minecraft start
    // The viewer will see a black screen until Minecraft loads
    if let Err(e) = register_go2rtc_stream(&session_id, &display_str).await {
        eprintln!("Failed to register go2rtc stream for {}: {}", session_id, e);
        // Continue anyway - we can try again later
    }

    // Update session process state to Stopped (ready for lazy start)
    {
        let mut mgr = manager.lock().await;
        if let Some(session) = mgr.get_session_mut(&session_id) {
            session.process_state = ProcessState::Stopped;
        }
    }

    Ok(())
}

/// Spawn Minecraft for a session (called when viewer detected)
pub async fn spawn_minecraft_for_session(
    session_id: &str,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<(), String> {
    // Get session info and check process state
    let (username, display_str, running, server_address) = {
        let mut mgr = manager.lock().await;
        let session = mgr
            .get_session_mut(session_id)
            .ok_or_else(|| format!("Session {} not found", session_id))?;

        // Don't start if already running or starting
        if session.process_state != ProcessState::Stopped {
            println!(
                "Session {} process state is {:?}, skipping spawn",
                session_id, session.process_state
            );
            return Ok(());
        }

        // Mark as starting
        session.process_state = ProcessState::Starting;
        // Reset the running flag for the new process
        session.running.store(true, Ordering::SeqCst);

        let display = session
            .get_display_string()
            .ok_or_else(|| format!("Session {} has no display assigned", session_id))?;

        (
            session.username.clone(),
            display,
            Arc::clone(&session.running),
            std::env::var("MINECRAFT_SERVER_ADDRESS").unwrap_or_default(),
        )
    };

    println!(
        "Spawning Minecraft for session {} on display {}",
        session_id, display_str
    );

    let config = MinecraftConfig {
        xorg_display: display_str.clone(),
        username,
        server_address,
    };

    // Spawn Minecraft process
    let manager_clone = Arc::clone(&manager);
    let sid = session_id.to_string();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = run_minecraft_process(config, running) {
            eprintln!("Minecraft process error for session {}: {}", sid, e);
        }

        // On process exit, update state back to Stopped
        futures::executor::block_on(async {
            let mut mgr = manager_clone.lock().await;
            if let Some(session) = mgr.get_session_mut(&sid) {
                session.process_state = ProcessState::Stopped;
                println!("Session {} Minecraft process stopped", sid);
            }
        });
    });

    // Mark as running (process is now spawned)
    {
        let mut mgr = manager.lock().await;
        if let Some(session) = mgr.get_session_mut(session_id) {
            session.process_state = ProcessState::Running;
        }
    }

    // Wait for Minecraft to fully start before registering the stream
    // This ensures FFmpeg captures content, not a black screen
    let startup_delay = std::env::var("MINECRAFT_STARTUP_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8); // Default 8 seconds for Minecraft to load

    println!(
        "Waiting {}s for Minecraft to start on session {}...",
        startup_delay, session_id
    );
    tokio::time::sleep(Duration::from_secs(startup_delay)).await;

    // Stream was already registered in start_rtsp_session - no need to re-register
    println!("Minecraft startup complete for session {}", session_id);
    Ok(())
}

/// Stop Minecraft for a session (called on idle timeout)
/// Note: We keep the stream registered so viewers can reconnect and trigger a restart
pub async fn stop_minecraft_for_session(
    session_id: &str,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<(), String> {
    let mut mgr = manager.lock().await;
    if let Some(session) = mgr.get_session_mut(session_id) {
        if session.process_state == ProcessState::Running {
            println!(
                "Stopping Minecraft for session {} due to idle timeout",
                session_id
            );
            session.running.store(false, Ordering::SeqCst);
            session.process_state = ProcessState::Stopping;
        }
    }
    Ok(())
}

/// Register an FFmpeg exec stream with go2rtc via its API
async fn register_go2rtc_stream(session_id: &str, display: &str) -> Result<(), String> {
    // Build the FFmpeg command that go2rtc will execute
    // FFmpeg outputs to stdout in MPEG-TS format which go2rtc can read
    // Use 320x200 to match the X display resolution
    // x11grab requires DISPLAY env var set via go2rtc exec options
    
    // NOTE: FFmpeg's -i syntax for x11grab is :DISPLAY_NUMBER.SCREEN_NUMBER
    // The 'display' variable passed here is like ":1".
    // So if display is ":1", we want "-i :1.0".
    
    let ffmpeg_cmd = format!(
        "ffmpeg -f x11grab -framerate 30 -video_size 320x200 -i {}.0 -c:v libx264 -preset ultrafast -tune zerolatency -pix_fmt yuv420p -g 30 -f mpegts pipe:1",
        display  // e.g., ":1"
    );

    // Full source with go2rtc exec options (DISPLAY env var)
    let source = format!("exec:{}#env=DISPLAY={}", ffmpeg_cmd, display);

    // Use go2rtc API to add the stream via config patching
    // This is more reliable than PUT /api/streams in some versions
    let go2rtc_api = "http://localhost:1984";

    // go2rtc API: PATCH /api/config
    let url = format!("{}/api/config", go2rtc_api);

    println!("Registering go2rtc stream: {} using config patch...", session_id);

    // Retry logic for transient failures
    let client = reqwest::Client::new();
    let max_retries = 3;
    let mut last_error = String::new();

    // Build JSON body for the request - streams object mapping name to source
    // We need to construct the map manually to use the session_id variable as the key
    let mut streams = serde_json::Map::new();
    streams.insert(session_id.to_string(), serde_json::Value::String(source));
    
    let body = serde_json::json!({
        "streams": streams
    });

    for attempt in 1..=max_retries {
        match client.patch(&url).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    println!("Successfully registered go2rtc stream: {}", session_id);
                    return Ok(());
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    last_error = format!("go2rtc API error: {} - {}", status, body);
                    println!("{}", last_error);
                }
            }
            Err(e) => {
                last_error = format!("Failed to call go2rtc API: {}", e);
            }
        }

        if attempt < max_retries {
            println!(
                "Stream registration attempt {} failed for {}, retrying in 1s...",
                attempt, session_id
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    Err(last_error)
}

/// Unregister a stream from go2rtc via its API
/// Note: Currently unused as we keep streams registered for lazy restart support.
/// Could be used when fully deleting a session.
#[allow(dead_code)]
async fn unregister_go2rtc_stream(session_id: &str) -> Result<(), String> {
    let go2rtc_api = "http://localhost:1984";

    // go2rtc API: DELETE /api/streams?name=xxx
    let url = format!("{}/api/streams?name={}", go2rtc_api, session_id);

    println!("Unregistering go2rtc stream: {}", session_id);

    let client = reqwest::Client::new();
    match client.delete(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Successfully unregistered go2rtc stream: {}", session_id);
                Ok(())
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("go2rtc API error: {} - {}", status, body))
            }
        }
        Err(e) => Err(format!("Failed to call go2rtc API: {}", e)),
    }
}

/// Run the Minecraft process
fn run_minecraft_process(
    config: MinecraftConfig,
    running: Arc<AtomicBool>,
) -> Result<(), String> {
    let display_env = config.xorg_display.clone();
    let launch_script = "/root/launch_minecraft.py";

    // Wait for Minecraft server if address is specified
    if !config.server_address.is_empty() {
        println!("Waiting for Minecraft server at {} to be ready...", config.server_address);
        
        let server_addr = if config.server_address.contains(':') {
            config.server_address.clone()
        } else {
            format!("{}:25565", config.server_address)
        };

        // Retry loop for server availability - wait up to 5 minutes
        let max_retries = 150; 
        let mut ready = false;
        
        for i in 0..max_retries {
            // Check if session was cancelled while waiting
            if !running.load(Ordering::SeqCst) {
                return Ok(()); 
            }
            
            match TcpStream::connect(&server_addr) {
                Ok(_) => {
                    println!("Minecraft server {} is reachable!", server_addr);
                    ready = true;
                    break;
                }
                Err(_) => {
                    if i % 10 == 0 {
                        println!("Waiting for server... (attempt {}/{})", i + 1, max_retries);
                    }
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        
        if !ready {
            return Err(format!("Timed out waiting for Minecraft server at {}", server_addr));
        }
    }

    let mut cmd = Command::new("python3");
    cmd.arg(launch_script)
        .arg("--username")
        .arg(&config.username)
        .env("DISPLAY", &display_env);

    if !config.server_address.is_empty() {
        cmd.arg("--server").arg(&config.server_address);
    }

    println!(
        "Launching Minecraft for RTSP session with username: {} on display: {}",
        config.username, display_env
    );

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Minecraft: {}", e))?;

    let pid = child.id();
    println!("Minecraft launched (PID: {})", pid);

    // Monitor the process
    let mut process = child;
    while running.load(Ordering::SeqCst) {
        match process.try_wait() {
            Ok(Some(status)) => {
                println!("Minecraft process exited with status: {}", status);
                running.store(false, Ordering::SeqCst);
                break;
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(500));
            }
            Err(e) => {
                eprintln!("Error checking Minecraft process status: {}", e);
                break;
            }
        }
    }

    // Cleanup: terminate Minecraft if still running
    if !running.load(Ordering::SeqCst) {
        terminate_process(&mut process, "Minecraft");
    }

    Ok(())
}

/// Gracefully terminate a process with SIGTERM, falling back to SIGKILL
fn terminate_process(process: &mut Child, name: &str) {
    match process.try_wait() {
        Ok(Some(status)) => {
            println!("{} process already exited with status: {}", name, status);
        }
        Ok(None) => {
            println!("Sending SIGTERM to {} process (PID: {})...", name, process.id());
            if let Err(e) = signal::kill(Pid::from_raw(process.id() as i32), Signal::SIGTERM) {
                println!("Could not send SIGTERM to {} process: {}", name, e);
            } else {
                // Wait for graceful termination
                let mut terminated = false;
                for _ in 0..10 {
                    thread::sleep(Duration::from_millis(500));
                    match process.try_wait() {
                        Ok(Some(status)) => {
                            println!("{} process exited gracefully with status: {}", name, status);
                            terminated = true;
                            break;
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            eprintln!("Error checking {} process status: {}", name, e);
                            break;
                        }
                    }
                }

                if !terminated {
                    println!("{} didn't exit after SIGTERM, force killing...", name);
                    match process.kill() {
                        Ok(_) => println!("Successfully terminated {} process.", name),
                        Err(e) => eprintln!("Failed to terminate {} process: {}", name, e),
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error checking {} process status: {}", name, e);
        }
    }
}

/// Spawn go2rtc process for RTSP distribution
pub fn spawn_go2rtc() -> Result<Child, String> {
    let config_path = std::env::var("GO2RTC_CONFIG")
        .unwrap_or_else(|_| "/config/go2rtc.yaml".to_string());

    println!("Starting go2rtc with config: {}", config_path);

    let child = Command::new("go2rtc")
        .args(["-config", &config_path])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn go2rtc: {}", e))?;

    println!("go2rtc launched (PID: {})", child.id());
    Ok(child)
}
