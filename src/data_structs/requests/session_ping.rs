use serde::{Deserialize, Serialize};
use crate::data_structs::app_credentials::AppCredentials;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct SessionPing {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub timestamp: i64,
}