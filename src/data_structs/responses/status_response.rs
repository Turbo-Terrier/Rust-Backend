use serde::{Deserialize, Serialize};

use crate::data_structs::responses::signable_data::SignableData;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct StatusResponse {
    kerberos_username: String,
    status: ResponseStatus,
    reason: String,
    response_timestamp: i64
}

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct SignedStatusResponse {
    pub data: StatusResponse,
    pub signature: String
}

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub enum ResponseStatus {
    Success,
    Warning,
    Error,
}

impl StatusResponse {
    pub fn new(kerberos_username: String, reason: String, response_timestamp: i64) -> Self {
        Self { kerberos_username, status: ResponseStatus::Success, reason, response_timestamp }
    }
}

impl SignableData for StatusResponse {
    fn string_to_sign(&self) -> String {
        format!("{},{},{},{}", self.kerberos_username, self.status.to_string(), self.reason, self.response_timestamp)
    }
}

impl ResponseStatus {
    pub fn to_string(&self) -> String {
        Self::as_str(self).to_string()
    }
    pub fn as_str(&self) -> &str {  //todo: this is kinda redundant, is there a better way?
        match self {
            ResponseStatus::Success => "Success",
            ResponseStatus::Warning => "Warning",
            ResponseStatus::Error => "Error"
        }
    }

}