use serde::{Deserialize, Serialize};
use crate::data_structs::app_credentials::AppCredentials;
use crate::data_structs::bu_course::BUCourse;
use crate::data_structs::device_meta::DeviceMeta;

#[derive(Debug, PartialEq)]
#[derive(Deserialize, Serialize)]
pub struct ApplicationStart {
    pub credentials: AppCredentials,
    pub target_courses: Vec<BUCourse>,
    pub device_meta: DeviceMeta,
    pub timestamp: i64
}