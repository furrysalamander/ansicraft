#![allow(non_local_definitions)]

use crate::models::*;
use crate::soap::Header;
use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub struct MediaRequestEnvelope {
    #[yaserde(rename = "Header", prefix = "s")]
    pub header: Header,
    #[yaserde(rename = "Body", prefix = "s")]
    pub body: MediaRequest,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub struct MediaResponseEnvelope {
    #[yaserde(rename = "Header", prefix = "s")]
    pub header: Header,
    #[yaserde(rename = "Body", prefix = "s")]
    pub body: MediaResponse,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub enum MediaRequest {
    #[yaserde(rename = "GetProfiles", prefix = "trt")]
    GetProfiles(GetProfiles),
    #[yaserde(rename = "GetStreamUri", prefix = "trt")]
    GetStreamUri(GetStreamUri),
}

impl Default for MediaRequest {
    fn default() -> Self {
        MediaRequest::GetProfiles(GetProfiles::default())
    }
}

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub enum MediaResponse {
    #[yaserde(rename = "GetProfilesResponse", prefix = "trt")]
    GetProfilesResponse(GetProfilesResponse),
    #[yaserde(rename = "GetStreamUriResponse", prefix = "trt")]
    GetStreamUriResponse(GetStreamUriResponse),
}

impl Default for MediaResponse {
    fn default() -> Self {
        MediaResponse::GetProfilesResponse(GetProfilesResponse::default())
    }
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "trt",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl"
)]
pub struct GetProfiles {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "trt",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl"
)]
pub struct GetProfilesResponse {
    #[yaserde(rename = "Profiles", prefix = "trt")]
    pub profiles: Vec<Profile>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "trt",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl"
)]
pub struct GetStreamUri {
    #[yaserde(rename = "StreamSetup", prefix = "trt")]
    pub stream_setup: StreamSetup,
    #[yaserde(rename = "ProfileToken", prefix = "trt")]
    pub profile_token: String,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct StreamSetup {
    #[yaserde(rename = "Stream", prefix = "tt")]
    pub stream: String, // RTP-Unicast, RTP-Multicast
    #[yaserde(rename = "Transport", prefix = "tt")]
    pub transport: Transport,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Transport {
    #[yaserde(rename = "Protocol", prefix = "tt")]
    pub protocol: String, // UDP, TCP, RTSP, HTTP
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "trt",
    namespace = "trt: http://www.onvif.org/ver10/media/wsdl"
)]
pub struct GetStreamUriResponse {
    #[yaserde(rename = "MediaUri", prefix = "trt")]
    pub media_uri: MediaUri,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct MediaUri {
    #[yaserde(rename = "Uri", prefix = "tt")]
    pub uri: String,
    #[yaserde(rename = "InvalidAfterConnect", prefix = "tt")]
    pub invalid_after_connect: bool,
    #[yaserde(rename = "InvalidAfterReboot", prefix = "tt")]
    pub invalid_after_reboot: bool,
    #[yaserde(rename = "Timeout", prefix = "tt")]
    pub timeout: String,
}

// Handler function for media service requests
use std::convert::Infallible;
use yaserde::de::from_str;
use yaserde::ser::to_string;

pub async fn handle_media_service(
    body_bytes: bytes::Bytes,
) -> Result<impl warp::Reply, Infallible> {
    let body = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();

    let envelope: Result<MediaRequestEnvelope, _> = from_str(&body);

    match envelope {
        Ok(req) => {
            let response_body = match req.body {
                MediaRequest::GetProfiles(_) => {
                    MediaResponse::GetProfilesResponse(GetProfilesResponse {
                        profiles: vec![Profile {
                            token: "Profile1".to_string(),
                            fixed: true,
                            name: "MainStream".to_string(),
                            video_encoder_configuration: Some(VideoEncoderConfiguration {
                                token: "VideoEncoder1".to_string(),
                                name: "H264".to_string(),
                                encoding: "H264".to_string(),
                                resolution: VideoResolution {
                                    width: std::env::var("VIDEO_WIDTH")
                                        .unwrap_or_else(|_| "320".to_string())
                                        .parse()
                                        .unwrap_or(320),
                                    height: std::env::var("VIDEO_HEIGHT")
                                        .unwrap_or_else(|_| "200".to_string())
                                        .parse()
                                        .unwrap_or(200),
                                },
                            }),
                            ptz_configuration: Some(PTZConfiguration {
                                token: "PTZCfg1".to_string(),
                                name: "PTZ".to_string(),
                                node_token: "PTZNode1".to_string(),
                            }),
                        }],
                    })
                }
                MediaRequest::GetStreamUri(_) => {
                    let host_ip =
                        std::env::var("HOST_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
                    let rtsp_port =
                        std::env::var("RTSP_PORT").unwrap_or_else(|_| "554".to_string());
                    let rtsp_url =
                        format!("rtsp://{}:{}/stream", host_ip, rtsp_port);

                    MediaResponse::GetStreamUriResponse(GetStreamUriResponse {
                        media_uri: MediaUri {
                            uri: rtsp_url,
                            invalid_after_connect: false,
                            invalid_after_reboot: false,
                            timeout: "PT30S".to_string(),
                        },
                    })
                }
            };

            let response_envelope = MediaResponseEnvelope {
                header: Header {},
                body: response_body,
            };
            let xml = to_string(&response_envelope)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            Ok(warp::reply::with_status(xml, warp::http::StatusCode::OK))
        }
        Err(e) => {
            tracing::error!("Error deserializing media request: {}", e);
            Ok(warp::reply::with_status(
                "Invalid Request".to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
