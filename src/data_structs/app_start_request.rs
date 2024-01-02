use serde::Deserialize;
use sqlx::Decode;
use crate::smtp_mailing_util::Email;

//todo: this whole thing is a mess. I need to clean it up
// by reorganizing modules
#[derive(Debug)]
#[derive(Deserialize)]
pub struct AppCredentials {
    pub kerberos_username: String,
    pub authentication_key: String,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct DeviceMeta {
    pub core_count: i16,
    pub cpu_speed: f32,
    pub system_arch: String,
    pub os: String,
    pub ip: Option<String>, // client doesn't send this field, server adds it
}

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize)]
#[derive(Decode)]
pub enum SemesterSeason {
    Summer1,
    Summer2,
    Fall,
    Spring
}

impl SemesterSeason {
    pub fn to_string(&self) -> String {
        match self {
            SemesterSeason::Summer1 => "Summer1".to_string(),
            SemesterSeason::Summer2 => "Summer2".to_string(),
            SemesterSeason::Fall => "Fall".to_string(),
            SemesterSeason::Spring => "Spring".to_string(),
        }
    }

    pub fn from_string(season: &str) -> SemesterSeason {
        let lower_season = season.clone().to_lowercase().as_str();
        match season {
            "summer1" => SemesterSeason::Summer1,
            "summer2" => SemesterSeason::Summer2,
            "fall" => SemesterSeason::Fall,
            "spring" => SemesterSeason::Spring,
            _ => panic!("Invalid season string")
        }
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct Semester {
    pub semester_season: SemesterSeason,
    pub semester_year: u16,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct BUCourse {
    pub semester: Semester,
    pub college: String,
    pub department: String,
    pub course_code: u16,
    pub section: String,
}

// when bot wants to send an email
#[derive(Debug)]
#[derive(Deserialize)]
pub struct EmailSendRequest {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub email: Email,
}

// sent when bot successfully registers for a course
#[derive(Debug)]
#[derive(Deserialize)]
pub struct RegistrationNotification {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub course: BUCourse,
    pub timestamp: i64,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct SessionPing {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub timestamp: i64,
}

// sent when application starts
#[derive(Debug)]
#[derive(Deserialize)]
pub struct ApplicationStart {
    pub credentials: AppCredentials,
    pub target_courses: Vec<BUCourse>,
    pub device_meta: DeviceMeta,
    pub timestamp: i64
}

// sent when application stops
#[derive(Debug)]
#[derive(Deserialize)]
pub struct ApplicationStopped {
    pub credentials: AppCredentials,
    pub session_id: i64,
    pub did_finish: bool,
    pub unknown_crash_occurred: Option<bool>,
    pub reason: String,
    pub avg_cycle_time: Option<f64>,
    pub std_cycle_time: Option<f64>,
    pub avg_sleep_time: Option<f64>,
    pub std_sleep_time: Option<f64>,
    pub timestamp: i64,
}