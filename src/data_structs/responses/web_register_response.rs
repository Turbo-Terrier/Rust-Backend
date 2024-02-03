use serde::{Deserialize, Serialize};

use crate::data_structs::user::User;

#[derive(Debug, PartialEq)]
#[derive(Deserialize, Serialize)]
pub struct WebRegisterResponse {
    pub(crate) jwt_cookie: String,
    pub(crate) user: User
}