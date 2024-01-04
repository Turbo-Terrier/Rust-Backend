use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct AppCredentials {
    pub kerberos_username: String,
    pub authentication_key: String,
}