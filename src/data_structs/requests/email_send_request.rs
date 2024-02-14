use serde::{Deserialize, Serialize};

use crate::smtp_mailing_util::Email;

// when bot wants to send an email
#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct EmailSendRequest {
    pub license_key: String,
    pub session_id: i64,
    pub email: Email,
}