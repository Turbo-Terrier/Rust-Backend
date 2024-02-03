use serde::{Deserialize, Serialize};

use crate::data_structs::grant_level::GrantLevel;
use crate::data_structs::responses::signable_data::SignableData;

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct ApplicationStartPermission {
    kerberos_username: String,
    membership_level: GrantLevel,
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
    pub fn new(kerberos_username: String, membership_level: GrantLevel, session_id: i64, response_timestamp: i64) -> Self {
        Self { kerberos_username, membership_level, session_id, response_timestamp }
    }
}

impl SignableData for ApplicationStartPermission {
    fn string_to_sign(&self) -> String {
        format!("{},{},{},{}", self.kerberos_username, self.membership_level.to_string(), self.session_id, self.response_timestamp)
    }
}