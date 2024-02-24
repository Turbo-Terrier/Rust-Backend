use std::fmt::Debug;
use std::time::Duration;

use rand::Rng;
use sqlx::{Error, Executor, MySql, Pool, Row};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult, MySqlRow};

use crate::data_structs::app_config::UserApplicationSettings;
use crate::data_structs::bu_course::{BUCourse, BUCourseSection};
use crate::data_structs::bu_course::CourseSection;
use crate::data_structs::device_meta::DeviceMeta;
use crate::data_structs::grant_level::GrantLevel;
use crate::data_structs::requests::application_start::ApplicationStart;
use crate::data_structs::requests::application_stopped::ApplicationStopped;
use crate::data_structs::requests::session_ping::SessionPing;
use crate::data_structs::semester::{Semester, SemesterSeason};
use crate::data_structs::user::User;
use crate::google_oauth::{GoogleAccessToken, GoogleUserInfo};
use crate::stripe_util::StripeHandler;

#[derive(Debug)]
#[derive(Clone)]
pub struct DatabasePool {
    pool: Pool<MySql>,
    connection_url: String
}
impl DatabasePool {

    pub async fn new(host: &str, port: i16, user: &str, pass: &str, database: &str) -> Self {
        let connection_url = format!("mysql://{user}:{pass}@{host}:{port}/{database}");

        let pool = match MySqlPoolOptions::new()
            .max_connections(5)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&connection_url).await {
                    Ok(res) => res,
                    Err(_) => panic!("Unable to connect to the database")
                };

        DatabasePool { pool: pool, connection_url}
    }

    pub async fn init(&self) {
        Self::create_tables(&self).await;
    }

    pub async fn get_all_course_departments(&self) -> Vec<String> {
        let results = sqlx::query("SELECT DISTINCT department FROM course_catalog;")
            .fetch_all(&self.pool).await.expect("Error fetching rows for the get_all_course_departments query");
        let mut departments: Vec<String> = Vec::new();
        for result in &results {
            let department = result.get_unchecked::<String, &str>("department");
            departments.push(department);
        }
        return departments;
    }

    pub async fn is_authenticated(&self, auth_key: &String) -> Option<String> {
        let result = sqlx::query("SELECT kerberos_username from users WHERE authentication_key=?")
            .bind(&auth_key)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the is_authenticated query");
        if !result.is_empty() {
            let row = result.get(0).unwrap();
            let kerberos_username = row.get_unchecked::<String, &str>("kerberos_username");
            return Option::from(kerberos_username);
        } else {
            return None;
        }
    }

    pub async fn mark_demo_over(&self, kerberos_username: &String) {
        sqlx::query("UPDATE users SET demo_expired_at=? WHERE kerberos_username=?")
            .bind(&chrono::Local::now().timestamp())
            .bind(&kerberos_username)
            .execute(&self.pool).await
            .expect("Error executing the mark_demo_over query");
    }

    pub async fn session_ping(&self, session_ping: &SessionPing) -> Result<&str, &str> {

        // first some sanity checks to ensure the session is still active
        if !Self::is_session_alive(&self, session_ping.session_id).await {
            return Err("Session not found or is no longer alive")
        }

        // now update ping
        sqlx::query("UPDATE application_launch_session SET last_ping=? WHERE session_id=?")
            .bind(&session_ping.timestamp)
            .bind(&session_ping.session_id)
            .execute(&self.pool).await
            .expect("Error executing the session_ping query");

        return Ok("Pong!")
    }


    pub async fn mark_course_registered(&self, kerberos_username: &String, session_id: i64, registration_timestamp: i64, course_id: u32, course_section: &str) -> bool {
        // sanity check to ensure session is alive
        if !Self::is_session_alive(&self, session_id).await {
            return false;
        }

        sqlx::query(r#"
            UPDATE app_session_courses
            SET register_timestamp=?
            WHERE session_id=?
            AND course_id=?
            AND course_section=?
        "#)
            .bind(&registration_timestamp)
            .bind(&session_id)
            .bind(&course_id)
            .bind(course_section)
            .execute(&self.pool).await
            .expect("Error executing the mark_course_registered query 1");

        // check if this was a planner session and only subtract credits if this was a real registration
        let result: Vec<MySqlRow> = sqlx::query(r#"
            SELECT planner_session from application_launch_session WHERE session_id=?
        "#).bind(&session_id)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the mark_course_registered query 2");

        if result.is_empty() {
            eprintln!("Error, session not found but this should never happen!");
            return false;
        }

        let planner_session: bool = result.get(0).unwrap().get_unchecked::<bool, &str>("planner_session");
        if planner_session {
            return true;
        }

        sqlx::query(r#"
            UPDATE users SET current_credits=GREATEST(0,current_credits-1) WHERE kerberos_username=?;
        "#)
            .bind(kerberos_username)
            .execute(&self.pool).await
            .expect("Error executing the mark_course_registered query 3");

        return true;
    }

    pub async fn end_session(&self, session_data: &ApplicationStopped) -> Result<&str, &str> {
        // sanity check to ensure session is alive
        if !Self::is_session_alive(&self, session_data.session_id).await {
            return Err("Session not found or is no longer alive")
        }
        // update the session to inactive
        sqlx::query("UPDATE application_launch_session SET is_active=0 WHERE session_id=?")
            .bind(&session_data.session_id)
            .execute(&self.pool).await
            .expect("Error executing the end_session query");

        // write the session terminate data to the database
        sqlx::query(r#"
                INSERT INTO application_terminate_session
                (session_id, did_finish, unknown_crash, reason,
                avg_cycle_time, cycle_time_std, avg_sleep_time,
                sleep_time_std, terminate_timestamp)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&session_data.session_id)
            .bind(&session_data.did_finish)
            .bind(&session_data.unknown_crash_occurred)
            .bind(&session_data.reason)
            .bind(&session_data.avg_cycle_time)
            .bind(&session_data.std_cycle_time)
            .bind(&session_data.avg_sleep_time)
            .bind(&session_data.std_sleep_time)
            .bind(&session_data.timestamp)
            .execute(&self.pool).await
            .expect("Error executing the end_session query");

        return Ok("OK")
    }

    pub async fn create_session(&self, session_data: &ApplicationStart, kerberos_username: &String, grant_level: &GrantLevel, planner_session: bool) -> i64 {
        // write the session data to the database and return the session_id key
        let result = sqlx::query(
            r#"INSERT INTO application_launch_session
                (kerberos_username, device_ip, device_name, device_os, system_arch,
                device_cores, device_clock_speed, grant_type, planner_session, launch_time)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#)
            .bind(kerberos_username)
            .bind(&session_data.device_meta.ip)
            .bind(&session_data.device_meta.name)
            .bind(&session_data.device_meta.os)
            .bind(&session_data.device_meta.system_arch)
            .bind(&session_data.device_meta.core_count)
            .bind(&session_data.device_meta.cpu_speed)
            .bind(&grant_level.to_string())
            .bind(&planner_session)
            .bind(chrono::Local::now().timestamp())
            .execute(&self.pool).await
            .expect("Error executing the create_session query");

        let session_id = result.last_insert_id() as i64;

        let courses = self.get_user_application_courses(kerberos_username).await;

        // write the courses to the database as well
        for bu_course_section in &courses {
            sqlx::query(r#"
                INSERT INTO app_session_courses
                (session_id, course_id, course_section)
                VALUES (?, ?, ?)
            "#)
                .bind(&session_id)
                .bind(&bu_course_section.course.course_id)
                .bind(&bu_course_section.section.section)
                .execute(&self.pool).await
                .expect("Error executing the create_session query");
        }

        return session_id;
    }

    // todo figure out demo credit management...
    pub async fn get_user(&self, kerberos_username: &String) -> Option<User> {
        let result: Vec<MySqlRow> = sqlx::query("SELECT * from users WHERE kerberos_username=?")
            .bind(kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the get_user query");

        if result.is_empty() {
            return None;
        } else {
            let row = result.get(0).unwrap();
            let stripe_id = row.get_unchecked::<String, &str>("stripe_id");
            let user = User {
                kerberos_username: kerberos_username.to_string(),
                stripe_id: stripe_id.as_str().parse().unwrap(),
                given_name: row.get_unchecked::<String, &str>("given_name"),
                family_name: row.get_unchecked::<String, &str>("family_name"),
                authentication_key: row.get_unchecked::<String, &str>("authentication_key"),
                profile_image_url: row.get_unchecked::<String, &str>("profile_image_url"),
                current_credits: row.get_unchecked::<i64, &str>("current_credits"),
                demo_expired_at: row.get_unchecked::<Option<i64>, &str>("demo_expired_at"),
                registration_timestamp: row.get_unchecked::<i64, &str>("registration_timestamp")
            };
            return Option::from(user);
        }
    }

    pub async fn create_purchase_session(&self, kerberos_username: &String, quantity: u64, subtotal: f64, session_id: &str) {
        sqlx::query(r#"
            INSERT INTO user_purchase_sessions
            (kerberos_username, session_id, quantity, subtotal, total,
            coupon, succeeded, processed, begin_timestamp, finish_timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
            .bind(&kerberos_username)
            .bind(&session_id)
            .bind(&quantity)
            .bind(&subtotal)
            .bind(None::<f64>)
            .bind(None::<f64>)
            .bind(0)
            .bind(0)
            .bind(&chrono::Local::now().timestamp())
            .bind(None::<i64>)
            .execute(&self.pool).await
            .expect("Error executing the create_purchase_session query");
    }

    pub async fn close_purchase_session(&self, session_id: &str, success: bool, total: Option<f64>, coupon: Option<String>) -> bool {
        // get quantity
        let result: Vec<MySqlRow> = sqlx::query("SELECT kerberos_username, quantity from user_purchase_sessions WHERE session_id=?")
            .bind(&session_id)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the close_purchase_session query");

        if !result.is_empty() {
            let row = result.get(0).unwrap();
            let quantity = row.get_unchecked::<i64, &str>("quantity");
            let kerberos_username = row.get_unchecked::<String, &str>("kerberos_username");

            // update
            sqlx::query(r#"
                UPDATE user_purchase_sessions
                SET succeeded=?, processed=1, total=?, coupon=?, finish_timestamp=?
                WHERE session_id=?
            "#)
                .bind(&success)
                .bind(&total)
                .bind(&chrono::Local::now().timestamp())
                .bind(&kerberos_username)
                .bind(&coupon)
                .bind(&session_id)
                .execute(&self.pool).await
                .expect("Error executing the close_purchase_session query");

            if success {
                // add credits
                sqlx::query("UPDATE users SET current_credits=current_credits+? WHERE kerberos_username=?")
                    .bind(&quantity)
                    .bind(&kerberos_username)
                    .execute(&self.pool).await
                    .expect("Error executing the close_purchase_session query");

                // mark demo over
                self.mark_demo_over(&kerberos_username).await;
            }

            return true;
        }

        return false;
    }

    /// Creates a new user in the database and on stripe if they don't already exist, or updates their info if they do
    /// Returns the user object and a bool indicating whether or not a new user was created
    pub async fn create_or_update_user(&self, user_info: &GoogleUserInfo, google_access_token: &GoogleAccessToken, stripe_handler: &StripeHandler) -> User {

        let registration_timestamp = chrono::Local::now().timestamp();
        let kerberos_username: &str = user_info.email.split("@").collect::<Vec<&str>>()[0];

        // first check if this user already exists
        let result: Vec<MySqlRow> = sqlx::query("SELECT * from users WHERE kerberos_username=?")
            .bind(kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the create_or_update_user query");

        // if this user already exists, update the user in db and on stripe and return
        if !result.is_empty() {

            // first load the user as is directly from the database
            let row: &MySqlRow = result.get(0).unwrap();
            let stripe_id = row.get_unchecked::<String, &str>("stripe_id");
            let mut user = User {
                kerberos_username: kerberos_username.to_string(),
                stripe_id: stripe_id.as_str().parse().unwrap(),
                given_name: row.get_unchecked::<String, &str>("given_name"),
                family_name: row.get_unchecked::<String, &str>("family_name"),
                authentication_key: row.get_unchecked::<String, &str>("authentication_key"),
                profile_image_url: row.get_unchecked::<String, &str>("profile_image_url"),
                current_credits: row.get_unchecked::<i64, &str>("current_credits"),
                demo_expired_at: row.get_unchecked::<Option<i64>, &str>("demo_expired_at"),
                registration_timestamp: row.get_unchecked::<i64, &str>("registration_timestamp")
            };

            // if any of the fields that can change have changed, update the user and the db and stripe
            if user.given_name != user_info.given_name || user.family_name != user_info.family_name || user.profile_image_url != user_info.picture {

                let name_changed = user.given_name != user_info.given_name || user.family_name != user_info.family_name;

                // update the user object
                user.given_name = user_info.given_name.clone();
                user.family_name = user_info.family_name.clone();
                user.profile_image_url = user_info.picture.clone();

                // now update the db
                sqlx::query(r#"
                    UPDATE users
                    SET given_name=?, family_name=?, profile_image_url=?
                    WHERE kerberos_username=?
                "#)
                    .bind(&user.given_name)
                    .bind(&user.family_name)
                    .bind(&user.profile_image_url)
                    .bind(&user.kerberos_username)
                    .execute(&self.pool).await
                    .expect("Error executing the create_or_update_user query");

                // update the updated user on stripe if their name changes
                // since thats the only thing on stripe that can change
                if name_changed {
                    stripe_handler.update_stripe_customer(&user).await;
                }
            }

            return user; //finally return
        }
        else {
            // else this user doesn't exist, so we create them

            // first we create the user on stripe
            let customer_id = stripe_handler.create_new_stripe_customer(
                user_info.name.as_str(),
                user_info.email.as_str()
            ).await;

            // generate a random authentication key using only alphabetical cased characters
            let auth_key: String = self.generate_new_key();

            // insert user
            sqlx::query(r#"
                INSERT INTO users
                    (kerberos_username, stripe_id, given_name, family_name, profile_image_url,
                    current_credits, authentication_key, registration_timestamp)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#)
                .bind(kerberos_username)
                .bind(customer_id.as_str())
                .bind(&user_info.given_name)
                .bind(&user_info.family_name)
                .bind(&user_info.picture)
                .bind(1)
                .bind(&auth_key)
                .bind(registration_timestamp)
                .execute(&self.pool).await
                .expect("Error executing the create_user query");

            // create default settings and insert settings
            let mut default_settings = UserApplicationSettings::default();
            default_settings.email = Some(user_info.email.clone());
            self.create_or_update_user_application_settings(kerberos_username, &default_settings).await;

            // now create the user object
            let user = User {
                kerberos_username: kerberos_username.to_string(),
                stripe_id: customer_id,
                given_name: user_info.given_name.clone(),
                family_name: user_info.family_name.clone(),
                authentication_key: auth_key,
                profile_image_url: user_info.picture.clone(),
                current_credits: 0,
                demo_expired_at: None,  // all new users get a demo,
                registration_timestamp: registration_timestamp
            };

            return user
        }

    }

    pub async fn reset_authentication_key(&self, kerberos_username: &String) -> String {
        let auth_key: String = self.generate_new_key();
        sqlx::query("UPDATE users SET authentication_key=? WHERE kerberos_username=?")
            .bind(&auth_key)
            .bind(&kerberos_username)
            .execute(&self.pool).await
            .expect("Error executing the reset_authentication_key query");

        auth_key
    }

    fn generate_new_key(&self) -> String {
        let mut rng = rand::thread_rng();
        let mut auth_key = String::new();
        for _ in 0..64 {
            let random_char = if rng.gen::<bool>() {
                rng.gen_range(65..=90) as u8 as char
            } else {
                rng.gen_range(97..=122) as u8 as char
            };
            auth_key.push(random_char);
        }

        return auth_key;
    }

    pub async fn cleanup_dead_sessions(&self) {

        let to_update = sqlx::query("SELECT session_id FROM application_launch_session WHERE last_ping < ? AND is_active=1")
            .bind(chrono::Local::now().timestamp() - 45) // close all sessions where no ping was received for 45sec
            .fetch_all(&self.pool).await
            .expect("Error executing the selection cleanup_dead_sessions query");

        for row in &to_update {
            let session_id = row.get_unchecked::<i64, &str>("session_id");

            // first, insert a session terminate entry
            Self::end_session(&self, &ApplicationStopped {
                license_key: String::new(), // this field isn't used soo I can just makeup data
                session_id: session_id,
                did_finish: false,
                unknown_crash_occurred: Option::Some(true),
                reason: "Session timed out".to_string(),
                avg_cycle_time: None,
                std_cycle_time: None,
                avg_sleep_time: None,
                std_sleep_time: None,
                timestamp: chrono::Local::now().timestamp()
            }).await.expect("Error executing the cleanup_dead_sessions query");

            // now update the session to inactive
            sqlx::query("UPDATE application_launch_session SET is_active=0 WHERE session_id=?")
                .bind(&session_id)
                .execute(&self.pool).await
                .expect("Error executing the cleanup_dead_sessions query");
        }

        if to_update.len() != 0 {
            println!("Pruned {} dead sessions", to_update.len());
        }

    }

    pub async fn is_session_alive(&self, session_id: i64) -> bool {
        let result = sqlx::query("SELECT * from application_launch_session WHERE session_id=? AND is_active=1")
            .bind(&session_id)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the session_ping query");

        return if result.is_empty() {
            false
        } else {
            true
        }
    }

    pub async fn has_active_session(&self, kerberos_username: &String) -> Option<DeviceMeta> {
        let result = sqlx::query("SELECT * from application_launch_session WHERE kerberos_username=? AND is_active=1")
            .bind(kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the session_ping query");

        return if result.is_empty() {
            None
        } else {
            let row = result.get(0).unwrap();
            Option::from(DeviceMeta {
                ip: row.get_unchecked::<Option<String>, &str>("device_ip"),
                os: row.get_unchecked::<String, &str>("device_os"),
                name: row.get_unchecked::<Option<String>, &str>("device_name"),
                system_arch: row.get_unchecked::<String, &str>("system_arch"),
                core_count: row.get_unchecked::<i16, &str>("device_cores"),
                cpu_speed: row.get_unchecked::<f32, &str>("device_clock_speed")
            })
        }
    }

    pub async fn get_user_application_settings(&self, kerberos_username: &str) -> Option<UserApplicationSettings> {
        let result = sqlx::query("SELECT * from user_application_settings WHERE kerberos_username=?")
            .bind(kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the get_user_application_settings query");
        if result.is_empty() {
            let mut application_config = UserApplicationSettings::default();
            application_config.target_courses = self.get_user_application_courses(kerberos_username).await;
            return Some(application_config);
        } else {
            let row = result.get(0).unwrap();
            match UserApplicationSettings::decode(row) {
                Ok(mut application_config) => {
                    let courses = self.get_user_application_courses(kerberos_username).await;
                    application_config.target_courses = courses;
                    return Some(application_config);
                },
                Err(err) => {
                    eprintln!("Error decoding application config: {}", err);
                    return None;
                }
            }
        }
    }

    pub async fn create_or_update_user_application_settings(&self, kerberos_username: &str, course_settings: &UserApplicationSettings) {
        // create
        sqlx::query(r#"
                INSERT INTO user_application_settings
                (kerberos_username, real_registrations, keep_trying, save_password,
                save_duo_cookies, registration_notifications,
                register_email_alert, register_text_alert, register_phone_alert,
                watchdog_notifications, watchdog_email_alert, watchdog_text_alert,
                watchdog_phone_alert, allow_update_emails, allow_marketing_emails, alert_phone, alert_email,
                console_colors, custom_chrome_driver, custom_chrome_driver_path, debug_mode)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE real_registrations=VALUES(real_registrations),
                keep_trying=VALUES(keep_trying), save_password=VALUES(save_password),
                save_duo_cookies=VALUES(save_duo_cookies), registration_notifications=VALUES(registration_notifications),
                register_email_alert=VALUES(register_email_alert),
                register_text_alert=VALUES(register_text_alert), register_phone_alert=VALUES(register_phone_alert),
                watchdog_notifications=VALUES(watchdog_notifications), watchdog_email_alert=VALUES(watchdog_email_alert),
                watchdog_text_alert=VALUES(watchdog_text_alert), watchdog_phone_alert=VALUES(watchdog_phone_alert),
                allow_update_emails=VALUES(allow_update_emails), allow_marketing_emails=VALUES(allow_marketing_emails),
                alert_phone=VALUES(alert_phone), alert_email=VALUES(alert_email), console_colors=VALUES(console_colors),
                custom_chrome_driver=VALUES(custom_chrome_driver), custom_chrome_driver_path=VALUES(custom_chrome_driver_path),
                debug_mode=VALUES(debug_mode)
            "#)
            .bind(&kerberos_username)
            .bind(&course_settings.real_registrations)
            .bind(&course_settings.keep_trying)
            .bind(&course_settings.save_password)
            .bind(&course_settings.save_duo_cookies)
            .bind(&course_settings.registration_notifications.enabled)
            .bind(&course_settings.registration_notifications.email_alerts)
            .bind(&course_settings.registration_notifications.text_alerts)
            .bind(&course_settings.registration_notifications.call_alerts)
            .bind(&course_settings.watchdog_notifications.enabled)
            .bind(&course_settings.watchdog_notifications.email_alerts)
            .bind(&course_settings.watchdog_notifications.text_alerts)
            .bind(&course_settings.watchdog_notifications.call_alerts)
            .bind(&course_settings.allow_update_emails)
            .bind(&course_settings.allow_marketing_emails)
            .bind(&course_settings.email)
            .bind(&course_settings.phone)
            .bind(&course_settings.console_colors)
            .bind(&course_settings.custom_driver.enabled)
            .bind(&course_settings.custom_driver.driver_path)
            .bind(&course_settings.debug_mode)
            .execute(&self.pool).await
            .expect("Error executing the create_or_update_user_application_settings query");
    }

    pub async fn add_custom_course_and_section(&self, semester: Semester, course_code: String, section: &str) -> BUCourseSection {
        let course_section = CourseSection {
            section: section.to_string(),
            ..CourseSection::default()
        };

        let bu_course_section = self.add_course(semester, course_code, None,
                                      None, false, vec![course_section]).await;

        return bu_course_section[0].clone();
    }

    pub async fn user_course_settings_add_course(&self, kerberos_username: &String, course_id: u32, course_section: &String) {
        sqlx::query(r#"
                INSERT IGNORE INTO user_application_course_settings
                (kerberos_username, course_id, course_section)
                VALUES (?, ?, ?);
            "#)
            .bind(&kerberos_username)
            .bind(course_id)
            .bind(course_section)
            .execute(&self.pool).await
            .expect("Error executing the user_course_settings_add_course query");
    }

    pub async fn user_course_settings_delete_course(&self, kerberos_username: &String, course_id: u32, course_section: &str) {
        sqlx::query(r#"
                DELETE FROM user_application_course_settings
                WHERE kerberos_username=? AND course_id=? AND course_section=?;
            "#)
            .bind(&kerberos_username)
            .bind(course_id)
            .bind(course_section)
            .execute(&self.pool).await
            .expect("Error executing the user_course_settings_delete_course query");
    }

    async fn get_user_application_courses(&self, kerberos_username: &str) -> Vec<BUCourseSection> {
        let result = sqlx::query(r#"
                    SELECT * from user_application_course_settings
                    INNER JOIN course_catalog cc on user_application_course_settings.course_id = cc.course_id
                    INNER JOIN course_sections_catalog csc on user_application_course_settings.course_id = csc.course_id
                                                     AND user_application_course_settings.course_section = csc.course_section
                    WHERE kerberos_username=?
                "#)
            .bind(kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the get_user_application_courses query");
        if result.is_empty() {
            return Vec::new();
        } else {
            let mut courses = Vec::new();
            for row in result {
                match BUCourseSection::decode(&row) {
                    Ok(course) => {
                        courses.push(course);
                    },
                    Err(err) => {
                        eprintln!("Error decoding user application setting courses: {}", err);
                    }
                }
            }
            return courses;
        }
    }


    pub async fn get_courses(&self, semester: &Semester) -> Vec<BUCourseSection> {
        let mut output: Vec<BUCourseSection> = Vec::new();

        let results = sqlx::query(r#"
                    SELECT * from course_catalog cc
                    INNER JOIN course_sections_catalog csc on cc.course_id = csc.course_id
                        WHERE semester_season=? AND
                            semester_year=?;
                "#)
            .bind(&semester.semester_season.to_string())
            .bind(&semester.semester_year)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the search_course query");

        for result in &results {
            output.push(BUCourseSection::decode(result).unwrap());
        }

        return output;
    }

    // course added by the scrapper are "confirmed to exist"
    pub async fn add_course(&self, semester: Semester, course_code: String, course_title: Option<String>, credits: Option<u8>, existence_confirmed: bool, sections: Vec<CourseSection>) -> Vec<BUCourseSection> {
        println!("ADDING {} - {:?} | {:?}", course_code, course_title, &sections);
        let (college, department, code) = BUCourse::from_course_code_str(&course_code);

        let result = sqlx::query(r#"
            INSERT INTO course_catalog
            (semester_season, semester_year, college, department, course_code, title, credits, course_existence, added_timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE title=VALUES(title), credits=VALUES(credits), course_existence=VALUES(course_existence)
        "#)
            .bind(&semester.semester_season.to_string())
            .bind(&semester.semester_year)
            .bind(college)
            .bind(department)
            .bind(code)
            .bind(&course_title)
            .bind(&credits)
            .bind(&existence_confirmed)
            .bind(&chrono::Local::now().timestamp())
            .execute(&self.pool).await
            .expect("Error executing the add_course query");

        // retrieve insert (or updated) id todo make unsigned
        let course_id: u32 = sqlx::query_scalar(r#"
                SELECT course_id FROM course_catalog WHERE semester_season=? AND semester_year=? AND college=? AND department=? AND course_code=?;
            "#)
            .bind(&semester.semester_season.to_string())
            .bind(&semester.semester_year)
            .bind(college)
            .bind(department)
            .bind(code)
            .fetch_one(&self.pool).await
            .expect("Error retrieving last insert id");

        for section in &sections {
            sqlx::query(r#"
                INSERT INTO course_sections_catalog
                (course_id, course_section, open_seats, instructor, section_type, location, schedule, dates, notes, section_existence, added_timestamp)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE
                open_seats=VALUES(open_seats), instructor=VALUES(instructor), section_type=VALUES(section_type),
                location=VALUES(location), schedule=VALUES(schedule), dates=VALUES(dates), notes=VALUES(notes)
            "#)
                .bind(&course_id)
                .bind(&section.section)
                .bind(&section.open_seats)
                .bind(&section.instructor)
                .bind(&section.section_type)
                .bind(&section.location)
                .bind(&section.schedule)
                .bind(&section.dates)
                .bind(&section.notes)
                .bind(&existence_confirmed)
                .bind(&chrono::Local::now().timestamp())
                .execute(&self.pool).await
                .expect("Error executing the add_course query");
        }

        let mut bu_course_sections: Vec<BUCourseSection> = Vec::new();

        for course_section in sections {
            let bu_course_section = BUCourseSection {
                course: BUCourse {
                    course_id: course_id,
                    semester: Semester {
                        semester_season: semester.semester_season.clone(),
                        semester_year: semester.semester_year
                    },
                    college: college.to_string(),
                    department: department.to_string(),
                    course_code: code.to_string(),
                    title: course_title.clone(),
                    credits,
                },
                section: course_section,
                existence_confirmed
            };
            bu_course_sections.push(bu_course_section);
        }

        bu_course_sections
    }

    async fn create_tables(&self) {
        Self::create_user_table(&self).await
            .expect("An error occurred create the 'users' table");
        Self::create_course_catalog_table(&self).await
            .expect("An error occurred create the 'course_catalog' table");
        Self::create_course_section_catalog_tables(&self).await
            .expect("An error occurred create the 'course_sections_catalog' table");
        Self::create_launch_tracker_table(&self).await
            .expect("An error occurred create the 'application_launch_session' table");
        Self::create_session_courses_table(&self).await
            .expect("An error occurred create the 'app_session_courses' table");
        Self::create_session_end_table(&self).await
            .expect("An error occurred create the 'application_terminate_session' table");
        Self::create_user_purchase_sessions_table(&self).await
            .expect("An error occurred create the 'user_purchase_sessions' table");
        Self::create_user_application_settings_table(&self).await
            .expect("An error occurred create the 'user_application_settings' table");
        Self::create_user_application_course_settings(&self).await
            .expect("An error occurred create the 'user_application_course_settings' table");
    }

    async fn create_course_section_catalog_tables(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
            create table if not exists course_sections_catalog
            (
                course_id             int unsigned,
                course_section        varchar(4)   not null,
                open_seats            smallint     null,
                instructor            varchar(64)  null,
                section_type          varchar(6)   null,
                location              varchar(64)  null,
                schedule              varchar(64)  null,
                dates                 varchar(64)  null,
                notes                 varchar(256) null,
                section_existence     tinyint(1)   not null,
                added_timestamp       bigint       not null,
                foreign key (course_id) references course_catalog (course_id),
                primary key (course_id, course_section)
            );
        "#).await
    }

    async fn create_course_catalog_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
            create table if not exists course_catalog
                (
                    course_id             int unsigned auto_increment                    primary key,
                    semester_season       enum ('Spring', 'Summer 1', 'Summer 2', 'Fall')  not null,
                    semester_year         smallint unsigned                              not null,
                    college               char(3)                                     not null,
                    department            char(2)                                     not null,
                    course_code           char(3)                                     not null,
                    title                 varchar(256)                                   null,
                    credits               tinyint unsigned                               null,
                    course_existence      tinyint(1)                                     not null,
                    added_timestamp       bigint                                         not null,
                    unique key (semester_season, semester_year, college, department, course_code)
                );
        "#).await
    }

    async fn create_user_application_course_settings(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
            create table if not exists user_application_course_settings
            (
                kerberos_username varchar(64)                                   not null,
                course_id         int unsigned                                  not null,
                course_section    varchar(6)                                    not null,
                foreign key (course_id, course_section)
                    references course_sections_catalog (course_id, course_section),
                foreign key (kerberos_username)
                    references users (kerberos_username),
                primary key  (kerberos_username, course_id, course_section)
            );
        "#).await
    }

    async fn create_user_application_settings_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
            create table if not exists user_application_settings
            (
                kerberos_username          varchar(64)  not null
                    primary key
                    references users (kerberos_username),
                real_registrations         tinyint(1)   not null,
                keep_trying                tinyint(1)   not null,
                save_password              tinyint(1)   not null,
                save_duo_cookies           tinyint(1)   not null,
                registration_notifications tinyint(1)   not null,
                register_email_alert       tinyint(1)   not null,
                register_text_alert        tinyint(1)   not null,
                register_phone_alert       tinyint(1)   not null,
                watchdog_notifications     tinyint(1)   not null,
                watchdog_email_alert       tinyint(1)   not null,
                watchdog_text_alert        tinyint(1)   not null,
                watchdog_phone_alert       tinyint(1)   not null,
                allow_update_emails        tinyint(1)   not null,
                allow_marketing_emails     tinyint(1)   not null,
                alert_phone                varchar(16)  null,
                alert_email                varchar(320) null,
                console_colors             tinyint(1)   not null,
                custom_chrome_driver       tinyint(1)   not null,
                custom_chrome_driver_path  varchar(512) null,
                debug_mode                 tinyint(1)   not null
            );
        "#).await
    }

    async fn create_user_purchase_sessions_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
            create table if not exists user_purchase_sessions
            (
                kerberos_username varchar(64)                                   not null
                    references users (kerberos_username),
                session_id        varchar(256)                                  null,
                quantity          int                                           not null,
                subtotal          float                                         null,
                total             float                                         null,
                coupon            varchar(32)                                   null,
                succeeded         tinyint(1)                                    not null,
                processed         tinyint(1)                                    not null,
                begin_timestamp   bigint                                        not null,
                finish_timestamp  bigint                                        null,
                primary key (kerberos_username, begin_timestamp),
                unique key (session_id)
            );
        "#).await
    }

    async fn create_user_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists users (
            kerberos_username      varchar(64)                                   not null,
            stripe_id              varchar(32)                                   not null,
            given_name             varchar(128)                                  not null,
            family_name            varchar(128)                                  not null,
            profile_image_url      varchar(256)                                  null,
            authentication_key     varchar(64)                                   not null,
            current_credits        int                                           not null,
            demo_expired_at        bigint                                        null,
            registration_timestamp bigint      default unix_timestamp()          not null,
            PRIMARY KEY (kerberos_username),
            UNIQUE KEY (authentication_key)
        );
        "#).await
    }

    async fn create_launch_tracker_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists application_launch_session
        (
            session_id         int auto_increment,
            kerberos_username  varchar(64)                               not null,
            device_ip          varchar(16)                               null,
            device_name        varchar(64)                               null,
            device_os          varchar(32)                               null,
            system_arch        varchar(32)                               null,
            device_cores       smallint                                  null,
            device_clock_speed float                                     null,
            grant_type         enum('Full', 'Partial', 'Demo', 'Expired', 'Error')  not null,
            planner_session    tinyint(1)                                not null,
            launch_time        bigint                                    not null,
            is_active          tinyint(1)                                default 1 not null,
            last_ping          bigint default unix_timestamp()           not null,
            primary key (session_id),
            foreign key (kerberos_username) references users (kerberos_username)
        );
        "#).await
    }

    async fn create_session_courses_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists app_session_courses
        (
            session_id          int                                           not null,
            course_id           int unsigned                                  not null,
            course_section      varchar(6)                                    not null,
            register_timestamp  bigint                                        null,
            primary key (session_id, course_id, course_section),
            foreign key (session_id)
                references application_launch_session        (session_id),
            foreign key (course_id, course_section) 
                references course_sections_catalog           (course_id, course_section)
        );
        "#).await
    }

    async fn create_session_end_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists application_terminate_session
        (
            session_id           int auto_increment,
            did_finish           tinyint(1)   not null,
            unknown_crash        tinyint(1)   null,
            reason               varchar(512) not null,
            avg_cycle_time       float        null,
            cycle_time_std       float        null,
            avg_sleep_time       float        null,
            sleep_time_std       float        null,
            terminate_timestamp  bigint       not null,
            primary key (session_id),
            foreign key (session_id) references application_launch_session (session_id)
        );
        "#).await
    }

}