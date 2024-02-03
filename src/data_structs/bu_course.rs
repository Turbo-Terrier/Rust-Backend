use serde::{Deserialize, Serialize};
use sqlx::{Decode, Row};

use crate::data_structs::semester::Semester;

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
#[derive(Clone)]
pub struct BUCourse {
    pub course_id: u32,
    pub semester: Semester,
    pub college: String,
    pub department: String,
    pub course_code: String,
    pub title: Option<String>,
    pub credits: Option<u8>,
}

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
#[derive(Clone)]
pub struct BUCourseSection {
    pub course: BUCourse,
    pub section: CourseSection
}

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
#[derive(Default)]
#[derive(Clone)]
pub struct CourseSection {
    pub section: String,
    pub open_seats: Option<u8>,
    pub instructor: Option<String>,
    pub section_type: Option<String>,
    pub location: Option<String>,
    pub schedule: Option<String>,
    pub dates: Option<String>,
    pub notes: Option<String>
}

impl BUCourseSection {
    pub fn decode(row: &sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BUCourseSection {
            course: BUCourse::decode(row)?,
            section: CourseSection::decode(row)?,
        })
    }
}

impl CourseSection {
    pub fn decode(row: &sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(CourseSection {
            section: row.try_get("course_section")?,
            open_seats: row.try_get("open_seats")?,
            instructor: row.try_get("instructor")?,
            section_type: row.try_get("section_type")?,
            location: row.try_get("location")?,
            schedule: row.try_get("schedule")?,
            dates: row.try_get("dates")?,
            notes: row.try_get("notes")?,
        })
    }
}

impl BUCourse {
    pub fn decode(row: &sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(BUCourse {
            course_id: row.try_get("course_id")?,
            semester: Semester::decode(row)?,
            college: row.try_get("college")?,
            department: row.try_get("department")?,
            course_code: row.try_get("course_code")?,
            title: None,
            credits: None,
        })
    }

    pub fn split_course_code_str_into_parts(course_code_str: &str) -> Vec<&str> {
        course_code_str.split_ascii_whitespace().collect()
    }

    pub fn from_course_code_str(course_code_str: &str) -> (&str, &str, &str) {
        let parts: Vec<&str> = course_code_str.split_ascii_whitespace().collect();
        let college = parts[0];
        let department = parts[1];
        let code = parts[2];
        return (college, department, code)
    }

    pub fn to_full_course_code_str(&self) -> String{
        let mut course_code_str = self.college.clone();
        course_code_str.push_str(" ");
        course_code_str.push_str(self.department.as_str());
        course_code_str.push_str(" ");
        course_code_str.push_str(self.course_code.to_string().as_str());
        return course_code_str;
    }
}