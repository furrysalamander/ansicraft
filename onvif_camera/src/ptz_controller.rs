use std::sync::{Arc, Mutex};
use std::time::Duration;
use minecraft_terminal_viewer::xdo;
use tokio::time;

#[derive(Clone, Copy, Debug, Default)]
struct Velocity {
    pan: f32,
    tilt: f32,
    zoom: f32,
}

#[derive(Clone)]
pub struct PtzController {
    current_velocity: Arc<Mutex<Velocity>>,
    display: String,
}

impl PtzController {
    pub fn new() -> Self {
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":1".to_string());
        let current_velocity = Arc::new(Mutex::new(Velocity::default()));
        
        // Spawn background task for continuous movement
        let velocity_clone = current_velocity.clone();
        let display_clone = display.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(50)); // 20Hz
            loop {
                interval.tick().await;

                let (vx, vy, vz) = {
                    let guard = velocity_clone.lock().unwrap();
                    (guard.pan, guard.tilt, guard.zoom)
                };

                // Move mouse if velocity is non-zero
                if vx != 0.0 || vy != 0.0 {
                    // Scaling: 1.0 = max speed. 
                    // Let's assume 1.0 = 20 pixels per 50ms (400px/s)
                    let dx = (vx * 20.0) as i32;
                    let dy = (-vy * 20.0) as i32; // Invert tilt (up is -y)
                    
                    if dx != 0 || dy != 0 {
                        // Use xdo helper
                        xdo::send_relative_mouse(&display_clone, dx, dy);
                    }
                }
                
                // For zoom (scrolling)
                if vz.abs() > 0.1 {
                     let scroll_delta = if vz > 0.0 { 1 } else { -1 };
                     // Only scroll occasionally to avoid spamming
                     xdo::send_scroll(&display_clone, scroll_delta);
                }
            }
        });

        Self {
            current_velocity,
            display,
        }
    }

    pub async fn continuous_move(&self, pan: f32, tilt: f32, zoom: f32) {
        tracing::debug!("PTZ ContinuousMove: pan={}, tilt={}, zoom={}", pan, tilt, zoom);
        let mut v = self.current_velocity.lock().unwrap();
        v.pan = pan;
        v.tilt = tilt;
        v.zoom = zoom;
    }

    pub async fn stop(&self) {
        tracing::debug!("PTZ Stop");
        let mut v = self.current_velocity.lock().unwrap();
        *v = Velocity::default();
    }

    pub async fn absolute_move(&self, pan: f32, tilt: f32, zoom: f32) {
        tracing::debug!("PTZ AbsoluteMove: pan={}, tilt={}, zoom={}", pan, tilt, zoom);
        // Absolute move is hard with relative mouse input. 
        // We could maybe implement it if we tracked position, but for now log it.
        // Or treat it as a "Look At" vector if we knew current rotation.
        // For now, doing nothing is safest to avoid erratic behavior.
    }

    pub async fn relative_move(&self, pan: f32, tilt: f32, zoom: f32) {
        tracing::debug!("PTZ RelativeMove: pan={}, tilt={}, zoom={}", pan, tilt, zoom);
        // Interpret input as normalized displacement
        // 1.0 = ~500 pixels ?
        let dx = (pan * 500.0) as i32;
        let dy = (-tilt * 500.0) as i32; // Invert tilt
        
        if dx != 0 || dy != 0 {
            xdo::send_relative_mouse(&self.display, dx, dy);
        }

        if zoom != 0.0 {
             // Scroll wheel simulation for zoom/hotbar
             xdo::send_scroll(&self.display, (zoom * 5.0) as i32);
        }
    }
}
