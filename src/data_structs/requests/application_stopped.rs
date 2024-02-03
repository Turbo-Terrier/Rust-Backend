use serde::{Deserialize, Serialize};

use crate::data_structs::app_credentials::AppCredentials;

#[derive(Debug, PartialEq)]
#[derive(Deserialize, Serialize)]
pub struct ApplicationStopped {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub did_finish: bool,
    pub unknown_crash_occurred: Option<bool>,
    pub reason: String,
    pub avg_cycle_time: Option<f64>,
    pub std_cycle_time: Option<f64>,
    pub avg_sleep_time: Option<f64>,
    pub std_sleep_time: Option<f64>,
    pub timestamp: i64,
}