use serde::{Deserialize, Serialize};
use stripe::CustomerId;
use crate::data_structs::semester::Semester;

#[derive(Debug, PartialEq)]
#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct User {
    pub kerberos_username: String,
    pub stripe_id: CustomerId,
    pub given_name: String,
    pub family_name: String,
    pub authentication_key: String,
    pub profile_image_url: String,
    pub demo_expired_at: Option<i64>,
    pub grants: Vec<Semester>,
    pub registration_timestamp: i64
}
