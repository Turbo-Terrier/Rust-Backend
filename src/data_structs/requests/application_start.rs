use serde::{Deserialize, Serialize};

use crate::data_structs::device_meta::DeviceMeta;

#[derive(Debug, PartialEq)]
#[derive(Deserialize, Serialize)]
pub struct ApplicationStart {
    pub license_key: String,
    pub device_meta: DeviceMeta,
    pub timestamp: i64
}