//! ONVIF Mock Camera Library
//!
//! This library provides an ONVIF-compliant mock camera that streams Minecraft gameplay.

pub mod discovery;
pub mod models;
pub mod ptz_controller;
pub mod services;
pub mod soap;

use std::net::SocketAddr;
use warp::Filter;

/// Run the ONVIF camera server
pub async fn run(port: u16, verbose: bool) {
    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    tracing::info!("Starting ONVIF camera server on port {}", port);

    // Create ONVIF service routes
    let device_service = warp::path!("onvif" / "device_service")
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(services::device::handle_device_service);

    let media_service = warp::path!("onvif" / "media_service")
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(services::media::handle_media_service);

    let ptz_service = warp::path!("onvif" / "ptz_service")
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(services::ptz::handle_ptz_service);

    let routes = device_service
        .or(media_service)
        .or(ptz_service);

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    // TODO: Also start WS-Discovery service

    warp::serve(routes).run(addr).await;
}
