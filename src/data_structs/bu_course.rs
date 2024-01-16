use serde::{Deserialize, Serialize};
use sqlx::{Decode, Row};
use crate::data_structs::semester::Semester;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
#[derive(Clone)]
pub struct BUCourse {
    pub semester: Semester,
    pub college: String,
    pub department: String,
    pub course_code: u16,
    pub section: String,
}

impl BUCourse {
    pub fn decode(row: &sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BUCourse {
            semester: Semester::decode(row)?,
            college: row.try_get("college")?,
            department: row.try_get("department")?,
            course_code: row.try_get("course_code")?,
            section: row.try_get("section")?,
        })
    }
}