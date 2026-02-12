#![allow(non_local_definitions)]

use yaserde_derive::{YaDeserialize, YaSerialize};

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
pub struct Header {
    // Add header fields if necessary, e.g., Security
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
pub struct Fault {
    #[yaserde(rename = "Code", prefix = "s")]
    pub code: FaultCode,
    #[yaserde(rename = "Reason", prefix = "s")]
    pub reason: FaultReason,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
pub struct FaultCode {
    #[yaserde(rename = "Value", prefix = "s")]
    pub value: String,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
pub struct FaultReason {
    #[yaserde(rename = "Text", prefix = "s")]
    pub text: FaultText,
}

#[derive(Debug, Default, YaSerialize, YaDeserialize)]
#[yaserde(prefix = "s", namespace = "s: http://www.w3.org/2003/05/soap-envelope")]
pub struct FaultText {
    #[yaserde(attribute, rename = "lang", prefix = "xml")]
    pub lang: String,
    #[yaserde(text)]
    pub value: String,
}
