use serde::{Deserialize, Serialize};
use crate::data_structs::app_config::UserApplicationSettings;

use crate::data_structs::grant_level::GrantLevel;
use crate::data_structs::responses::signable_data::SignableData;

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct ApplicationStartPermission {
    kerberos_username: String,
    membership_level: GrantLevel,
    user_app_settings: UserApplicationSettings,
    session_id: i64,
    response_timestamp: i64,
}

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct SignedApplicationStartPermission {
    pub data: ApplicationStartPermission,
    pub signature: String
}

impl ApplicationStartPermission {
    pub fn new(kerberos_username: String, membership_level: GrantLevel, user_app_settings: UserApplicationSettings, session_id: i64, response_timestamp: i64) -> Self {
        Self { kerberos_username, membership_level, user_app_settings, session_id, response_timestamp }
    }
}

impl SignableData for ApplicationStartPermission {

}