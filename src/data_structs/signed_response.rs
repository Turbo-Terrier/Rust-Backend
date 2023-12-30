use serde::Deserialize;
use sqlx::Decode;

#[derive(Debug)]
#[derive(Deserialize)]
pub struct SignedApplicationStartPermission {
    kerberos_username: String,
    membership_level: GrantLevel,
    session_id: i64,
    response_timestamp: u64,
    signature: String
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct SignedResponse {
    kerberos_username: String,
    status: ResponseStatus,
    reason: String,
    response_timestamp: u64,
    signature: String
}

#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Decode)]
pub enum GrantLevel {
    Full,
    Demo,
    None
}

#[derive(Debug)]
#[derive(Deserialize)]
pub enum ResponseStatus {
    Good,
    Warning,
    Error,
}

impl SignedApplicationStartPermission {
    pub fn new(kerberos_username: String, membership_level: GrantLevel, session_id: i64, response_timestamp: u64, signature: String) -> Self {
        Self { kerberos_username, membership_level, session_id, response_timestamp, signature }
    }
    pub fn is_valid() -> bool {
        true
    }
    pub fn kerberos_username(&self) -> &String {
        &self.kerberos_username
    }
    pub fn membership_level(&self) -> &GrantLevel {
        &self.membership_level
    }
    pub fn session_id(&self) -> u32 {
        self.session_id
    }
    pub fn response_timestamp(&self) -> u64 {
        self.response_timestamp
    }
    pub fn signature(&self) -> &String {
        &self.signature
    }

}

impl SignedResponse {
    pub fn new(kerberos_username: String, status: ResponseStatus, reason: String, response_timestamp: u64, signature: String) -> Self {
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
    pub fn response_timestamp(&self) -> u64 {
        self.response_timestamp
    }
    pub fn signature(&self) -> &String {
        &self.signature
    }

}