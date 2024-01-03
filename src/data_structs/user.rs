use jwt::{FromBase64, JoseHeader};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub kerberos_username: String,
    pub given_name: String,
    pub family_name: String,
    pub authentication_key: String,
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
            demo_expired_at: self.demo_expired_at.clone(),
            premium_since: self.premium_since.clone(),
            premium_expiry: self.premium_expiry.clone(),
            registration_timestamp: self.registration_timestamp.clone()
        }
    }
}
