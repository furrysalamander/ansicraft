use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use warp::Filter;

use crate::rtsp_session::{spawn_minecraft_for_session, stop_minecraft_for_session};
use crate::session_manager::{SessionManager, SessionState};
use crate::xdo;

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub session_id: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum InputRequest {
    #[serde(rename = "mouse_relative")]
    MouseRelative { dx: i32, dy: i32 },

    #[serde(rename = "mouse_absolute")]
    MouseAbsolute { x: i32, y: i32 },

    #[serde(rename = "key")]
    Key { code: String, pressed: bool },

    #[serde(rename = "scroll")]
    Scroll { delta: i32 },

    #[serde(rename = "click")]
    Click { button: u8 },
}

#[derive(Debug, Serialize)]
pub struct SessionStatus {
    pub session_id: String,
    pub state: String,
    pub x_display: Option<u32>,
    pub stream_url: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionStatus>,
}

impl ApiResponse {
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            session: None,
        }
    }

    pub fn success_with_message(message: &str) -> Self {
        Self {
            success: true,
            message: Some(message.to_string()),
            session: None,
        }
    }

    pub fn success_with_session(session: SessionStatus) -> Self {
        Self {
            success: true,
            message: None,
            session: Some(session),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: Some(message.to_string()),
            session: None,
        }
    }
}

fn state_to_string(state: &SessionState) -> String {
    match state {
        SessionState::Pending => "pending".to_string(),
        SessionState::Active => "active".to_string(),
        SessionState::Terminated => "terminated".to_string(),
    }
}

pub async fn run(manager: Arc<Mutex<SessionManager>>, port: u16) {
    let manager_filter = warp::any().map(move || Arc::clone(&manager));

    // POST /session - Create a new session
    // Use path! macro or path::end() to ensure we strictly match "/session" and not "/session/..."
    let create_session = warp::path!("session")
        .and(warp::post())
        .and(warp::body::json())
        .and(manager_filter.clone())
        .and_then(handle_create_session);

    // GET /session/{id} - Get session status
    let get_session = warp::path!("session" / String)
        .and(warp::get())
        .and(manager_filter.clone())
        .and_then(handle_get_session);

    // POST /session/{id}/input - Send input to session
    let send_input = warp::path!("session" / String / "input")
        .and(warp::post())
        .and(warp::body::json())
        .and(manager_filter.clone())
        .and_then(handle_send_input);

    // DELETE /session/{id} - Terminate session
    let delete_session = warp::path!("session" / String)
        .and(warp::delete())
        .and(manager_filter.clone())
        .and_then(handle_delete_session);

    // GET /sessions - List all sessions
    let list_sessions = warp::path!("sessions")
        .and(warp::get())
        .and(manager_filter.clone())
        .and_then(handle_list_sessions);

    // POST /session/{id}/start - Start Minecraft for session (lazy start trigger)
    let start_minecraft = warp::path!("session" / String / "start")
        .and(warp::post())
        .and(manager_filter.clone())
        .and_then(handle_start_minecraft);

    // POST /session/{id}/stop - Stop Minecraft for session (idle timeout trigger)
    let stop_minecraft = warp::path!("session" / String / "stop")
        .and(warp::post())
        .and(manager_filter.clone())
        .and_then(handle_stop_minecraft);

    let routes = create_session
        .or(get_session)
        .or(send_input)
        .or(delete_session)
        .or(list_sessions)
        .or(start_minecraft)
        .or(stop_minecraft);

    println!("HTTP API server starting on port {}", port);
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}

async fn handle_create_session(
    req: CreateSessionRequest,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut mgr = manager.lock().await;

    match mgr.create_session(req.session_id.clone(), req.username.clone()) {
        Ok(()) => {
            println!(
                "Created session {} for user {}",
                req.session_id, req.username
            );

            // Start the RTSP session in the background
            let session_id = req.session_id.clone();
            let username = req.username.clone();

            // Get the allocator and request a resource
            if let Some(allocator) = mgr.get_allocator(&session_id) {
                let mut status_rx = allocator.request_resource();
                let manager_clone = Arc::clone(&manager);

                // Spawn background task to handle resource allocation
                tokio::spawn(async move {
                    while let Some(status) = status_rx.recv().await {
                        match status {
                            crate::queueing::ResourceStatus::Success(resource_id) => {
                                println!(
                                    "Session {} assigned to X display :{}",
                                    session_id,
                                    resource_id + 1
                                );

                                // Update session state
                                {
                                    let mut mgr = manager_clone.lock().await;
                                    mgr.set_session_active(&session_id, resource_id);
                                }

                                // Start the RTSP session
                                if let Err(e) = crate::rtsp_session::start_rtsp_session(
                                    session_id.clone(),
                                    username.clone(),
                                    resource_id,
                                    manager_clone.clone(),
                                )
                                .await
                                {
                                    eprintln!("Failed to start RTSP session: {}", e);
                                }
                                break;
                            }
                            crate::queueing::ResourceStatus::QueuePosition(pos) => {
                                println!("Session {} is at queue position {}", session_id, pos + 1);
                            }
                            crate::queueing::ResourceStatus::Failed(reason) => {
                                eprintln!("Session {} failed: {}", session_id, reason);
                                break;
                            }
                            crate::queueing::ResourceStatus::Cancelled => {
                                println!("Session {} was cancelled", session_id);
                                break;
                            }
                        }
                    }
                });
            }

            let response = ApiResponse::success_with_message(&format!(
                "Session {} created, waiting for resource allocation",
                req.session_id
            ));
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::CREATED,
            ))
        }
        Err(e) => {
            let response = ApiResponse::error(&e);
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::CONFLICT,
            ))
        }
    }
}

async fn handle_get_session(
    session_id: String,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = manager.lock().await;

    match mgr.get_session(&session_id) {
        Some(session) => {
            let status = SessionStatus {
                session_id: session.session_id.clone(),
                state: state_to_string(&session.state),
                x_display: session.x_display,
                stream_url: session.get_stream_url(),
            };
            let response = ApiResponse::success_with_session(status);
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        None => {
            let response = ApiResponse::error(&format!("Session {} not found", session_id));
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
    }
}

async fn handle_send_input(
    session_id: String,
    input: InputRequest,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = manager.lock().await;

    match mgr.get_session(&session_id) {
        Some(session) => {
            if session.state != SessionState::Active {
                let response = ApiResponse::error("Session is not active");
                return Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }

            let display = match session.get_display_string() {
                Some(d) => d,
                None => {
                    let response = ApiResponse::error("Session has no display assigned");
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&response),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
            };

            // Execute the input command
            match input {
                InputRequest::MouseRelative { dx, dy } => {
                    xdo::send_relative_mouse(&display, dx, dy);
                }
                InputRequest::MouseAbsolute { x, y } => {
                    xdo::send_absolute_mouse(&display, x, y);
                }
                InputRequest::Key { code, pressed } => {
                    xdo::send_key(&display, &code, pressed);
                }
                InputRequest::Scroll { delta } => {
                    xdo::send_scroll(&display, delta);
                }
                InputRequest::Click { button } => {
                    xdo::send_click(&display, button);
                }
            }

            let response = ApiResponse::success();
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        None => {
            let response = ApiResponse::error(&format!("Session {} not found", session_id));
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
    }
}

async fn handle_delete_session(
    session_id: String,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut mgr = manager.lock().await;

    match mgr.terminate_session(&session_id) {
        Ok(()) => {
            println!("Terminated session {}", session_id);
            let response = ApiResponse::success_with_message(&format!(
                "Session {} terminated",
                session_id
            ));
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            let response = ApiResponse::error(&e);
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
    }
}

async fn handle_list_sessions(
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mgr = manager.lock().await;

    let sessions: Vec<SessionStatus> = mgr
        .list_sessions()
        .iter()
        .map(|s| SessionStatus {
            session_id: s.session_id.clone(),
            state: state_to_string(&s.state),
            x_display: s.x_display,
            stream_url: s.get_stream_url(),
        })
        .collect();

    Ok(warp::reply::with_status(
        warp::reply::json(&sessions),
        warp::http::StatusCode::OK,
    ))
}

async fn handle_start_minecraft(
    session_id: String,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Check if session exists and is active
    {
        let mgr = manager.lock().await;
        match mgr.get_session(&session_id) {
            Some(session) => {
                if session.state != SessionState::Active {
                    let response = ApiResponse::error("Session is not active");
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&response),
                        warp::http::StatusCode::BAD_REQUEST,
                    ));
                }
            }
            None => {
                let response = ApiResponse::error(&format!("Session {} not found", session_id));
                return Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
        }
    }

    // Start Minecraft for this session
    match spawn_minecraft_for_session(&session_id, manager).await {
        Ok(()) => {
            println!("Started Minecraft for session {} (viewer connected)", session_id);
            let response = ApiResponse::success_with_message("Minecraft started");
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            let response = ApiResponse::error(&e);
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn handle_stop_minecraft(
    session_id: String,
    manager: Arc<Mutex<SessionManager>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Check if session exists
    {
        let mgr = manager.lock().await;
        if mgr.get_session(&session_id).is_none() {
            let response = ApiResponse::error(&format!("Session {} not found", session_id));
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
    }

    // Stop Minecraft for this session
    match stop_minecraft_for_session(&session_id, manager).await {
        Ok(()) => {
            println!("Stopped Minecraft for session {} (idle timeout)", session_id);
            let response = ApiResponse::success_with_message("Minecraft stopped");
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            let response = ApiResponse::error(&e);
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}
