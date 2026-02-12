use crate::models::{Date, Time, DateTime, PTZConfiguration, Profile, VideoEncoderConfiguration, VideoResolution};
// use chrono::{Datelike, Timelike, Utc};
use crate::soap::Header;
use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub struct DeviceRequestEnvelope {
    #[yaserde(rename = "Header", prefix = "s")]
    pub header: Header,
    #[yaserde(rename = "Body", prefix = "s")]
    pub body: DeviceRequest,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl",
    namespace = "tt: http://www.onvif.org/ver10/schema"
)]
pub struct DeviceResponseEnvelope {
    #[yaserde(rename = "Header", prefix = "s")]
    pub header: Header,
    #[yaserde(rename = "Body", prefix = "s")]
    pub body: DeviceResponse,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub enum DeviceRequest {
    #[yaserde(rename = "GetDeviceInformation", prefix = "tds")]
    GetDeviceInformation(GetDeviceInformation),
    #[yaserde(rename = "GetSystemDateAndTime", prefix = "tds")]
    GetSystemDateAndTime(GetSystemDateAndTime),
    #[yaserde(rename = "GetCapabilities", prefix = "tds")]
    GetCapabilities(GetCapabilities),
}

impl Default for DeviceRequest {
    fn default() -> Self {
        DeviceRequest::GetDeviceInformation(GetDeviceInformation::default())
    }
}

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "s",
    namespace = "s: http://www.w3.org/2003/05/soap-envelope",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub enum DeviceResponse {
    #[yaserde(rename = "GetDeviceInformationResponse", prefix = "tds")]
    GetDeviceInformationResponse(GetDeviceInformationResponse),
    #[yaserde(rename = "GetSystemDateAndTimeResponse", prefix = "tds")]
    GetSystemDateAndTimeResponse(GetSystemDateAndTimeResponse),
    #[yaserde(rename = "GetCapabilitiesResponse", prefix = "tds")]
    GetCapabilitiesResponse(GetCapabilitiesResponse),
}

impl Default for DeviceResponse {
    fn default() -> Self {
        DeviceResponse::GetDeviceInformationResponse(GetDeviceInformationResponse::default())
    }
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tds",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub struct GetDeviceInformation {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tds",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub struct GetDeviceInformationResponse {
    #[yaserde(rename = "Manufacturer", prefix = "tds")]
    pub manufacturer: String,
    #[yaserde(rename = "Model", prefix = "tds")]
    pub model: String,
    #[yaserde(rename = "FirmwareVersion", prefix = "tds")]
    pub firmware_version: String,
    #[yaserde(rename = "SerialNumber", prefix = "tds")]
    pub serial_number: String,
    #[yaserde(rename = "HardwareId", prefix = "tds")]
    pub hardware_id: String,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tds",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub struct GetSystemDateAndTime {}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tds",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub struct GetSystemDateAndTimeResponse {
    #[yaserde(rename = "SystemDateAndTime", prefix = "tds")]
    pub system_date_and_time: SystemDateAndTime,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct SystemDateAndTime {
    #[yaserde(rename = "DateTimeType", prefix = "tt")]
    pub date_time_type: String, // NTP or Manual
    #[yaserde(rename = "DaylightSavings", prefix = "tt")]
    pub daylight_savings: bool,
    #[yaserde(rename = "TimeZone", prefix = "tt")]
    pub time_zone: TimeZone,
    #[yaserde(rename = "UTCDateTime", prefix = "tt")]
    pub utc_date_time: DateTime,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct TimeZone {
    #[yaserde(rename = "TZ", prefix = "tt")]
    pub tz: String,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tds",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub struct GetCapabilities {
    #[yaserde(rename = "Category", prefix = "tds")]
    pub category: Vec<String>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(
    prefix = "tds",
    namespace = "tds: http://www.onvif.org/ver10/device/wsdl"
)]
pub struct GetCapabilitiesResponse {
    #[yaserde(rename = "Capabilities", prefix = "tds")]
    pub capabilities: Capabilities,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Capabilities {
    #[yaserde(rename = "Device", prefix = "tt")]
    pub device: Option<DeviceCapabilities>,
    #[yaserde(rename = "Media", prefix = "tt")]
    pub media: Option<MediaCapabilities>,
    #[yaserde(rename = "PTZ", prefix = "tt")]
    pub ptz: Option<PTZCapabilities>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct DeviceCapabilities {
    #[yaserde(rename = "XAddr", prefix = "tt")]
    pub x_addr: String,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct MediaCapabilities {
    #[yaserde(rename = "XAddr", prefix = "tt")]
    pub x_addr: String,
    #[yaserde(rename = "StreamingCapabilities", prefix = "tt")]
    pub streaming_capabilities: StreamingCapabilities,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct StreamingCapabilities {
    #[yaserde(rename = "RTPMulticast", prefix = "tt")]
    pub rtp_multicast: bool,
    #[yaserde(rename = "RTP_TCP", prefix = "tt")]
    pub rtp_tcp: bool,
    #[yaserde(rename = "RTP_RTSP_TCP", prefix = "tt")]
    pub rtp_rtsp_tcp: bool,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct PTZCapabilities {
    #[yaserde(rename = "XAddr", prefix = "tt")]
    pub x_addr: String,
}

// Handler function for device service requests
use std::convert::Infallible;
use yaserde::de::from_str;
use yaserde::ser::to_string;
use chrono::prelude::*;

pub async fn handle_device_service(
    body_bytes: bytes::Bytes,
) -> Result<impl warp::Reply, Infallible> {
    let body = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();

    let envelope: Result<DeviceRequestEnvelope, _> = from_str(&body);

    match envelope {
        Ok(req) => {
            let response_body = match req.body {
                DeviceRequest::GetDeviceInformation(_) => {
                    let device_name = std::env::var("DEVICE_NAME")
                        .unwrap_or_else(|_| "Minecraft Camera".to_string());
                    DeviceResponse::GetDeviceInformationResponse(GetDeviceInformationResponse {
                        manufacturer: "MinecraftCameraFactory".to_string(),
                        model: "MC-X1".to_string(),
                        firmware_version: "1.0.0".to_string(),
                        serial_number: std::env::var("DEVICE_UUID")
                            .unwrap_or_else(|_| "MC123456".to_string()),
                        hardware_id: device_name,
                    })
                }
                DeviceRequest::GetSystemDateAndTime(_) => {
                    let now = Utc::now();
                    DeviceResponse::GetSystemDateAndTimeResponse(GetSystemDateAndTimeResponse {
                        system_date_and_time: SystemDateAndTime {
                            date_time_type: "NTP".to_string(),
                            daylight_savings: false,
                            time_zone: TimeZone {
                                tz: "UTC".to_string(),
                            },
                            utc_date_time: DateTime {
                                time: Time {
                                    hour: now.hour() as i32,
                                    minute: now.minute() as i32,
                                    second: now.second() as i32,
                                },
                                date: Date {
                                    year: now.year(),
                                    month: now.month() as i32,
                                    day: now.day() as i32,
                                },
                            },
                        },
                    })
                }
                DeviceRequest::GetCapabilities(_) => {
                    let ip = std::env::var("HOST_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
                    let port = std::env::var("ONVIF_PORT").unwrap_or_else(|_| "8080".to_string());
                    DeviceResponse::GetCapabilitiesResponse(GetCapabilitiesResponse {
                        capabilities: Capabilities {
                            device: Some(DeviceCapabilities {
                                x_addr: format!("http://{}:{}/onvif/device_service", ip, port),
                            }),
                            media: Some(MediaCapabilities {
                                x_addr: format!("http://{}:{}/onvif/media_service", ip, port),
                                streaming_capabilities: StreamingCapabilities {
                                    rtp_multicast: false,
                                    rtp_tcp: true,
                                    rtp_rtsp_tcp: true,
                                },
                            }),
                            ptz: Some(PTZCapabilities {
                                x_addr: format!("http://{}:{}/onvif/ptz_service", ip, port),
                            }),
                        },
                    })
                }
            };

            let response_envelope = DeviceResponseEnvelope {
                header: Header {},
                body: response_body,
            };

            let xml = to_string(&response_envelope)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            Ok(warp::reply::with_status(xml, warp::http::StatusCode::OK))
        }
        Err(e) => {
            tracing::error!("Error deserializing device request: {}", e);
            Ok(warp::reply::with_status(
                "Invalid Request".to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        }
    }
}
