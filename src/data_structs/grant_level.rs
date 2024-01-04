use serde::{Deserialize, Serialize};
use sqlx::Decode;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize, Decode)]
pub enum GrantLevel {
    Full,
    Demo,
    Expired,
    Error
}

impl GrantLevel {
    pub fn to_string(&self) -> String {
        Self::as_str(self).to_string()
    }
    pub fn as_str(&self) -> &str {
        match self {
            GrantLevel::Full => "Full",
            GrantLevel::Demo => "Demo",
            GrantLevel::Expired => "Expired",
            GrantLevel::Error => "Error"
        }
    }

}