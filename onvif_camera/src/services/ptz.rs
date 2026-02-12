#![allow(non_local_definitions)]

use crate::models::*;
use crate::soap::Header;
use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub struct PTZRequestEnvelope {
    #[yaserde(rename = "Header", prefix = "s")]
    pub header: Header,
    #[yaserde(rename = "Body", prefix = "s")]
    pub body: PTZRequest,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub struct PTZResponseEnvelope {
    #[yaserde(rename = "Header", prefix = "s")]
    pub header: Header,
    #[yaserde(rename = "Body", prefix = "s")]
    pub body: PTZResponse,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub enum PTZRequest {
    #[yaserde(rename = "ContinuousMove", prefix = "tptz")]
    ContinuousMove(ContinuousMove),
    #[yaserde(rename = "AbsoluteMove", prefix = "tptz")]
    AbsoluteMove(AbsoluteMove),
    #[yaserde(rename = "RelativeMove", prefix = "tptz")]
    RelativeMove(RelativeMove),
    #[yaserde(rename = "Stop", prefix = "tptz")]
    Stop(Stop),
    #[yaserde(rename = "GetConfigurations", prefix = "tptz")]
    GetConfigurations(GetConfigurations),
}

impl Default for PTZRequest {
    fn default() -> Self {
        PTZRequest::GetConfigurations(GetConfigurations::default())
    }
}

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub enum PTZResponse {
    #[yaserde(rename = "ContinuousMoveResponse", prefix = "tptz")]
    ContinuousMoveResponse(ContinuousMoveResponse),
    #[yaserde(rename = "AbsoluteMoveResponse", prefix = "tptz")]
    AbsoluteMoveResponse(AbsoluteMoveResponse),
    #[yaserde(rename = "RelativeMoveResponse", prefix = "tptz")]
    RelativeMoveResponse(RelativeMoveResponse),
    #[yaserde(rename = "StopResponse", prefix = "tptz")]
    StopResponse(StopResponse),
    #[yaserde(rename = "GetConfigurationsResponse", prefix = "tptz")]
    GetConfigurationsResponse(GetConfigurationsResponse),
}

impl Default for PTZResponse {
    fn default() -> Self {
        PTZResponse::GetConfigurationsResponse(GetConfigurationsResponse::default())
    }
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct ContinuousMove {
    #[yaserde(rename = "ProfileToken", prefix = "tptz")]
    pub profile_token: String,
    #[yaserde(rename = "Velocity", prefix = "tptz")]
    pub velocity: PTZSpeed,
    #[yaserde(rename = "Timeout", prefix = "tptz")]
    pub timeout: Option<String>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct PTZSpeed {
    #[yaserde(rename = "PanTilt", prefix = "tt")]
    pub pan_tilt: Option<Vector2D>,
    #[yaserde(rename = "Zoom", prefix = "tt")]
    pub zoom: Option<Vector1D>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Vector2D {
    #[yaserde(attribute, rename = "x")]
    pub x: f32,
    #[yaserde(attribute, rename = "y")]
    pub y: f32,
    #[yaserde(attribute, rename = "space")]
    pub space: Option<String>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Vector1D {
    #[yaserde(attribute, rename = "x")]
    pub x: f32,
    #[yaserde(attribute, rename = "space")]
    pub space: Option<String>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct ContinuousMoveResponse {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct Stop {
    #[yaserde(rename = "ProfileToken", prefix = "tptz")]
    pub profile_token: String,
    #[yaserde(rename = "PanTilt", prefix = "tptz")]
    pub pan_tilt: bool,
    #[yaserde(rename = "Zoom", prefix = "tptz")]
    pub zoom: bool,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct StopResponse {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct GetConfigurations {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct GetConfigurationsResponse {
    #[yaserde(rename = "PTZConfiguration", prefix = "tptz")]
    pub ptz_configuration: Vec<PTZConfiguration>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct AbsoluteMove {
    #[yaserde(rename = "ProfileToken", prefix = "tptz")]
    pub profile_token: String,
    #[yaserde(rename = "Position", prefix = "tptz")]
    pub position: PTZVector,
    #[yaserde(rename = "Speed", prefix = "tptz")]
    pub speed: Option<PTZSpeed>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct AbsoluteMoveResponse {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct RelativeMove {
    #[yaserde(rename = "ProfileToken", prefix = "tptz")]
    pub profile_token: String,
    #[yaserde(rename = "Translation", prefix = "tptz")]
    pub translation: PTZVector,
    #[yaserde(rename = "Speed", prefix = "tptz")]
    pub speed: Option<PTZSpeed>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tptz",
    namespace = "tptz: http://www.onvif.org/ver20/ptz/wsdl"
)]
pub struct RelativeMoveResponse {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct PTZVector {
    #[yaserde(rename = "PanTilt", prefix = "tt")]
    pub pan_tilt: Option<Vector2D>,
    #[yaserde(rename = "Zoom", prefix = "tt")]
    pub zoom: Option<Vector1D>,
}

// Handler function for PTZ service requests
use std::convert::Infallible;
use yaserde::de::from_str;
use yaserde::ser::to_string;
use crate::ptz_controller::PtzController;

pub async fn handle_ptz_service(
    body_bytes: bytes::Bytes,
    controller: PtzController
) -> Result<impl warp::Reply, Infallible> {
    let body = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();

    let envelope: Result<PTZRequestEnvelope, _> = from_str(&body);

    match envelope {
        Ok(req) => {
            let response_body = match req.body {
                PTZRequest::ContinuousMove(move_req) => {
                    let pan = move_req.velocity.pan_tilt.as_ref().map(|v| v.x).unwrap_or(0.0);
                    let tilt = move_req.velocity.pan_tilt.as_ref().map(|v| v.y).unwrap_or(0.0);
                    let zoom = move_req.velocity.zoom.as_ref().map(|v| v.x).unwrap_or(0.0);
                    controller.continuous_move(pan, tilt, zoom).await;
                    PTZResponse::ContinuousMoveResponse(ContinuousMoveResponse {})
                }
                PTZRequest::Stop(_) => {
                    controller.stop().await;
                    PTZResponse::StopResponse(StopResponse {})
                }
                PTZRequest::GetConfigurations(_) => {
                    PTZResponse::GetConfigurationsResponse(GetConfigurationsResponse {
                        ptz_configuration: vec![crate::models::PTZConfiguration {
                            token: "PTZCfg1".to_string(),
                            name: "PTZ".to_string(),
                            node_token: "PTZNode1".to_string(),
                        }],
                    })
                }
                PTZRequest::AbsoluteMove(move_req) => {
                    let pan = move_req.position.pan_tilt.as_ref().map(|v| v.x).unwrap_or(0.0);
                    let tilt = move_req.position.pan_tilt.as_ref().map(|v| v.y).unwrap_or(0.0);
                    let zoom = move_req.position.zoom.as_ref().map(|v| v.x).unwrap_or(0.0);
                    controller.absolute_move(pan, tilt, zoom).await;
                    PTZResponse::AbsoluteMoveResponse(AbsoluteMoveResponse {})
                }
                PTZRequest::RelativeMove(move_req) => {
                    let pan = move_req.translation.pan_tilt.as_ref().map(|v| v.x).unwrap_or(0.0);
                    let tilt = move_req.translation.pan_tilt.as_ref().map(|v| v.y).unwrap_or(0.0);
                    let zoom = move_req.translation.zoom.as_ref().map(|v| v.x).unwrap_or(0.0);
                    controller.relative_move(pan, tilt, zoom).await;
                    PTZResponse::RelativeMoveResponse(RelativeMoveResponse {})
                }
            };

            let response_envelope = PTZResponseEnvelope {
                header: crate::soap::Header {},
                body: response_body,
            };
            let xml = to_string(&response_envelope)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            Ok(warp::reply::with_status(xml, warp::http::StatusCode::OK))
        }
        Err(e) => {
            tracing::error!("Error deserializing PTZ request: {}", e);
            Ok(warp::reply::with_status(
                "Invalid Request".to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
