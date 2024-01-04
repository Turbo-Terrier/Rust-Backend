use serde::{Deserialize, Serialize};
use crate::data_structs::app_credentials::AppCredentials;
use crate::data_structs::bu_course::BUCourse;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct RegistrationNotification {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub course: BUCourse,
    pub timestamp: i64,
}