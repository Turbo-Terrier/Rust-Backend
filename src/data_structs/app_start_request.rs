use serde::Deserialize;

#[derive(Debug)]
#[derive(Deserialize)]
pub struct AppCredentials {
    pub kerberos_username: String,
    pub authentication_key: String,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct Email {
    pub email_heading: String,
    pub sender_name: String,
    pub email_body: String,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct DeviceMeta {
    pub core_count: i16,
    pub cpu_speed: f32,
    pub system_arch: String,
    pub os: String,
    pub ip: String,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct BUCourse {
    pub semester_key: String,
    pub college: String,
    pub department: String,
    pub course: u16,
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
    pub unknown_crash_occured: bool,
    pub reason: String,
    pub avg_cycle_time: f64,
    pub std_cycle_time: f64,
    pub avg_sleep_time: f64,
    pub std_sleep_time: f64,
    pub num_registered: u8,
    pub timestamp: i64,
}