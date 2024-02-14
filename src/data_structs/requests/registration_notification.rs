use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct RegistrationNotification {
    pub license_key: String,
    pub session_id: i64,
    pub course_id: u32,
    pub course_section: String,
    pub timestamp: i64,
}