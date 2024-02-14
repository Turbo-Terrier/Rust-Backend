use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct SessionPing {
    pub license_key: String,
    pub session_id: i64,
    pub timestamp: i64,
}