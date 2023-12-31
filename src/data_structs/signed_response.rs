use serde::{Deserialize, Serialize};
use sqlx::Decode;

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct ApplicationStartPermission {
    kerberos_username: String,
    membership_level: GrantLevel,
    session_id: i64,
    response_timestamp: i64,
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct SignedApplicationStartPermission {
    pub data: ApplicationStartPermission,
    pub signature: String
}

impl SignableData for ApplicationStartPermission {
    fn string_to_sign(&self) -> String {
        format!("{},{},{},{}", self.kerberos_username, self.membership_level.to_string(), self.session_id, self.response_timestamp)
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct StatusResponse {
    kerberos_username: String,
    status: ResponseStatus,
    reason: String,
    response_timestamp: i64
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct SignedStatusResponse {
    pub data: StatusResponse,
    pub signature: String
}

impl SignableData for StatusResponse {
    fn string_to_sign(&self) -> String {
        format!("{},{},{},{}", self.kerberos_username, self.status.to_string(), self.reason, self.response_timestamp)
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Decode)]
pub enum GrantLevel {
    Full,
    Demo,
    None
}

impl GrantLevel {
    pub fn to_string(&self) -> String {
        Self::as_str(self).to_string()
    }
    pub fn as_str(&self) -> &str {
        match self {
            GrantLevel::Full => "Full",
            GrantLevel::Demo => "Demo",
            GrantLevel::None => "None"
        }
    }

}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub enum ResponseStatus {
    Good,
    Warning,
    Error,
}

impl ResponseStatus {
    pub fn to_string(&self) -> String {
        Self::as_str(self).to_string()
    }
    pub fn as_str(&self) -> &str {  //todo: this is kinda redundant, is there a better way?
        match self {
            ResponseStatus::Good => "Good",
            ResponseStatus::Warning => "Warning",
            ResponseStatus::Error => "Error"
        }
    }

}

impl ApplicationStartPermission {
    pub fn new(kerberos_username: String, membership_level: GrantLevel, session_id: i64, response_timestamp: i64) -> Self {
        Self { kerberos_username, membership_level, session_id, response_timestamp }
    }
}

impl StatusResponse {
    pub fn new(kerberos_username: String, reason: String, response_timestamp: i64) -> Self {
        Self { kerberos_username, status: ResponseStatus::Good, reason, response_timestamp }
    }
}


pub trait SignableData {
    /** This is the string the client side will check to make sure was signed by the server */
    fn string_to_sign(&self) -> String;
}
