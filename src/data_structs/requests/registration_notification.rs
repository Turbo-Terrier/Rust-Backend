use serde::{Deserialize, Serialize};

use crate::data_structs::app_credentials::AppCredentials;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct RegistrationNotification {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub course_id: u32,
    pub section_id: String,
    pub timestamp: i64,
}