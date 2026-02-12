//! Minecraft Terminal Viewer Library
//!
//! This library provides functionality for managing Minecraft client sessions,
//! RTSP streaming, and input handling via xdotool.

pub mod config;
pub mod minecraft;
pub mod render;
pub mod xdo;

// Re-export key types for convenience
pub use minecraft::{MinecraftConfig, run};
