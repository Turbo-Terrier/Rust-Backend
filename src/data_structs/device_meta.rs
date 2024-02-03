use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[derive(Deserialize, Serialize)]
pub struct DeviceMeta {
    pub core_count: i16,
    pub cpu_speed: f32,
    pub system_arch: String,
    pub name: Option<String>,
    pub os: String,
    pub ip: Option<String>, // client doesn't send this field, server adds it
}