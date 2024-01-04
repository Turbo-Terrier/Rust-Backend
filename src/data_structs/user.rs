use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct User {
    pub kerberos_username: String,
    pub given_name: String,
    pub family_name: String,
    pub authentication_key: String,
    pub profile_image_url: String,
    pub demo_expired_at: Option<i64>,
    pub premium_since: Option<i64>,
    pub premium_expiry: Option<i64>,
    pub registration_timestamp: i64
}

impl Clone for User {
    fn clone(&self) -> Self {
        return User {
            kerberos_username: self.kerberos_username.clone(),
            given_name: self.given_name.clone(),
            family_name: self.family_name.clone(),
            authentication_key: self.authentication_key.clone(),
            profile_image_url: self.profile_image_url.clone(),
            demo_expired_at: self.demo_expired_at.clone(),
            premium_since: self.premium_since.clone(),
            premium_expiry: self.premium_expiry.clone(),
            registration_timestamp: self.registration_timestamp.clone()
        }
    }
}
