use serde::{Deserialize, Serialize};
use sqlx::Decode;
use strum_macros::EnumString;

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct SignedApplicationStartPermission {
    kerberos_username: String,
    membership_level: GrantLevel,
    session_id: i64,
    response_timestamp: i64,
    signature: String
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct SignedResponse {
    kerberos_username: String,
    status: ResponseStatus,
    reason: String,
    response_timestamp: i64,
    signature: String
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Decode)]
#[derive(EnumString)]
#[strum(serialize_all = "snake_case")]
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
#[derive(EnumString)]
#[derive(Serialize)]
pub enum ResponseStatus {
    Good,
    Warning,
    Error,
}

impl SignedApplicationStartPermission {
    pub fn new(kerberos_username: String, membership_level: GrantLevel, session_id: i64, response_timestamp: i64, signature: String) -> Self {
        Self { kerberos_username, membership_level, session_id, response_timestamp, signature }
    }
}

impl SignedResponse {
    pub fn new(kerberos_username: String, status: ResponseStatus, reason: String, response_timestamp: i64, signature: String) -> Self {
        Self { kerberos_username, status, reason, response_timestamp, signature }
    }
    pub fn is_valid() -> bool {
        true
    }
    pub fn kerberos_username(&self) -> &String {
        &self.kerberos_username
    }
    pub fn status(&self) -> &ResponseStatus {
        &self.status
    }
    pub fn reason(&self) -> &String {
        &self.reason
    }
    pub fn response_timestamp(&self) -> i64 {
        self.response_timestamp
    }
    pub fn signature(&self) -> &String {
        &self.signature
    }

}