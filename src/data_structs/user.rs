use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub kerberos_username: &'static str,
    pub given_name: &'static str,
    pub family_name: &'static str,
    pub authentication_key: &'static str,
    pub google_access_token: &'static str,
    pub google_refresh_token: &'static str,
    pub demo_expired_at: Option<i64>,
    pub premium_since: Option<i64>,
    pub premium_expiry: Option<i64>,
    pub registration_timestamp: i64
}