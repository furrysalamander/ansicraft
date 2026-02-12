use std::collections::HashMap;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::queueing::{ResourceAllocator, ResourcePool, ResourceStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Pending,
    Active,
    Terminated,
}

/// Tracks the state of the Minecraft process for lazy startup
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Stopped,   // No Minecraft running
    Starting,  // Minecraft is starting up
    Running,   // Minecraft is running
    Stopping,  // Minecraft is shutting down
}

pub struct Session {
    pub session_id: String,
    pub username: String,
    pub x_display: Option<u32>,
    pub state: SessionState,
    pub process_state: ProcessState,
    pub created_at: Instant,
    pub last_viewer_at: Option<Instant>,
    pub running: Arc<AtomicBool>,
}

impl Session {
    pub fn new(session_id: String, username: String) -> Self {
        Self {
            session_id,
            username,
            x_display: None,
            state: SessionState::Pending,
            process_state: ProcessState::Stopped,
            created_at: Instant::now(),
            last_viewer_at: None,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn get_display_string(&self) -> Option<String> {
        self.x_display.map(|d| format!(":{}", d + 1))
    }

    pub fn get_stream_url(&self) -> String {
        format!("rtsp://localhost:8554/{}", self.session_id)
    }

    pub fn terminate(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.state = SessionState::Terminated;
    }
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    pool: ResourcePool,
    allocators: HashMap<String, ResourceAllocator>,
}

impl SessionManager {
    pub fn new(max_sessions: u32) -> Self {
        Self {
            sessions: HashMap::new(),
            pool: ResourcePool::new(max_sessions),
            allocators: HashMap::new(),
        }
    }

    pub fn create_session(&mut self, session_id: String, username: String) -> Result<(), String> {
        if self.sessions.contains_key(&session_id) {
            return Err(format!("Session {} already exists", session_id));
        }

        let session = Session::new(session_id.clone(), username);
        self.sessions.insert(session_id.clone(), session);

        // Create an allocator for this session
        let allocator = ResourceAllocator::new(&self.pool);
        self.allocators.insert(session_id, allocator);

        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    pub fn get_allocator(&self, session_id: &str) -> Option<&ResourceAllocator> {
        self.allocators.get(session_id)
    }

    pub fn set_session_active(&mut self, session_id: &str, x_display: u32) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.x_display = Some(x_display);
            session.state = SessionState::Active;
        }
    }

    pub fn terminate_session(&mut self, session_id: &str) -> Result<(), String> {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.terminate();

            // Release the resource if allocated
            if let Some(x_display) = session.x_display {
                if let Some(allocator) = self.allocators.get(session_id) {
                    allocator.release(x_display);
                }
            }

            Ok(())
        } else {
            Err(format!("Session {} not found", session_id))
        }
    }

    pub fn remove_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
        self.allocators.remove(session_id);
    }

    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }
}
