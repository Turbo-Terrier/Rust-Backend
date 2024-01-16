use actix_web::App;
use serde::{Deserialize, Serialize};
use sqlx::{Decode, MySql, Pool, Row, Type};
use sqlx::mysql::MySqlRow;
use crate::data_structs::bu_course::BUCourse;
use crate::database::DatabasePool;

#[derive(Debug, Serialize, Deserialize)]
#[derive(Eq, PartialEq)]
#[derive(Default)]
#[derive(Clone)]
pub struct CustomDriver {
    pub enabled: bool,
    pub driver_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(Eq, PartialEq)]
#[derive(Default)]
#[derive(Clone)]
pub struct PushNotification {
    pub enabled: bool,
    pub email_alerts: bool,
    pub text_alerts: bool,
    pub call_alerts: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[derive(Eq, PartialEq)]
#[derive(Clone)]
pub(crate) struct UserApplicationSettings {
    pub real_registrations: bool,
    pub keep_trying: bool,
    pub save_password: bool,
    pub save_duo_cookies: bool,
    pub registration_notifications: PushNotification,
    pub watchdog_notifications: PushNotification,
    pub console_colors: bool,
    pub custom_driver: CustomDriver,
    pub debug_mode: bool,
    pub target_courses: Vec<BUCourse>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

impl Default for UserApplicationSettings {
    fn default() -> Self {
        return UserApplicationSettings {
            real_registrations: false,
            keep_trying: false,
            save_password: false,
            save_duo_cookies: false,
            registration_notifications: PushNotification {
                enabled: true,
                email_alerts: true,
                ..PushNotification::default()
            },
            watchdog_notifications: PushNotification {
                ..PushNotification::default()
            },
            console_colors: true,
            custom_driver: CustomDriver {
                ..CustomDriver::default()
            },
            debug_mode: false,
            email: None,
            phone: None,
            target_courses: Vec::new(),
        }
    }
}

impl UserApplicationSettings {
    pub(crate) fn decode(row: &MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(UserApplicationSettings {
            real_registrations: row.try_get("real_registrations")?,
            keep_trying: row.try_get("keep_trying")?,
            save_password: row.try_get("save_password")?,
            save_duo_cookies: row.try_get("save_duo_cookies")?,
            registration_notifications: PushNotification {
                enabled: row.try_get("registration_notifications")?,
                email_alerts: row.try_get("register_email_alert")?,
                text_alerts: row.try_get("register_text_alert")?,
                call_alerts: row.try_get("register_phone_alert")?,
            },
            watchdog_notifications: PushNotification {
                enabled: row.try_get("watchdog_notifications")?,
                email_alerts: row.try_get("watchdog_email_alert")?,
                text_alerts: row.try_get("watchdog_text_alert")?,
                call_alerts: row.try_get("watchdog_phone_alert")?,
            },
            console_colors: row.try_get("console_colors")?,
            custom_driver: CustomDriver {
                enabled: row.try_get("custom_chrome_driver")?,
                driver_path: row.try_get("custom_chrome_driver_path")?,
            },
            email: row.try_get("alert_email")?,
            phone: row.try_get("alert_phone")?,
            debug_mode: row.try_get("debug_mode")?,
            target_courses: Vec::new(),
        })
    }
}