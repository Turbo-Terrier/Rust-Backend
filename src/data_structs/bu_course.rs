use serde::{Deserialize, Serialize};
use crate::data_structs::semester::Semester;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct BUCourse {
    pub semester: Semester,
    pub college: String,
    pub department: String,
    pub course_code: u16,
    pub section: String,
}