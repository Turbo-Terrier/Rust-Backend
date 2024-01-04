use serde::{Deserialize, Serialize};
use crate::data_structs::app_credentials::AppCredentials;
use crate::smtp_mailing_util::Email;

// when bot wants to send an email
#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct EmailSendRequest {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub email: Email,
}