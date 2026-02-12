use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Date {
    #[yaserde(rename = "Year", prefix = "tt")]
    pub year: i32,
    #[yaserde(rename = "Month", prefix = "tt")]
    pub month: i32,
    #[yaserde(rename = "Day", prefix = "tt")]
    pub day: i32,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Time {
    #[yaserde(rename = "Hour", prefix = "tt")]
    pub hour: i32,
    #[yaserde(rename = "Minute", prefix = "tt")]
    pub minute: i32,
    #[yaserde(rename = "Second", prefix = "tt")]
    pub second: i32,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct DateTime {
    #[yaserde(rename = "Time", prefix = "tt")]
    pub time: Time,
    #[yaserde(rename = "Date", prefix = "tt")]
    pub date: Date,
}

// Media Profile Stub
#[derive(Debug, Default, YaSerialize, YaDeserialize, Clone)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct Profile {
    #[yaserde(attribute, rename = "token")]
    pub token: String,
    #[yaserde(attribute, rename = "fixed")]
    pub fixed: bool,
    #[yaserde(rename = "Name", prefix = "tt")]
    pub name: String,
    // Add VideoSourceConfiguration, VideoEncoderConfiguration, PTZConfiguration, etc. as needed
    #[yaserde(rename = "VideoEncoderConfiguration", prefix = "tt")]
    pub video_encoder_configuration: Option<VideoEncoderConfiguration>,
    #[yaserde(rename = "PTZConfiguration", prefix = "tt")]
    pub ptz_configuration: Option<PTZConfiguration>,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize, Clone)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct VideoEncoderConfiguration {
    #[yaserde(attribute, rename = "token")]
    pub token: String,
    #[yaserde(rename = "Name", prefix = "tt")]
    pub name: String,
    #[yaserde(rename = "Encoding", prefix = "tt")]
    pub encoding: String, // JPEG, MPEG4, H264
    #[yaserde(rename = "Resolution", prefix = "tt")]
    pub resolution: VideoResolution,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize, Clone)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct VideoResolution {
    #[yaserde(rename = "Width", prefix = "tt")]
    pub width: i32,
    #[yaserde(rename = "Height", prefix = "tt")]
    pub height: i32,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize, Clone)]
#[yaserde(prefix = "tt", namespace = "tt: http://www.onvif.org/ver10/schema")]
pub struct PTZConfiguration {
    #[yaserde(attribute, rename = "token")]
    pub token: String,
    #[yaserde(rename = "Name", prefix = "tt")]
    pub name: String,
    #[yaserde(rename = "NodeToken", prefix = "tt")]
    pub node_token: String,
}
