use std::fmt::Debug;
use std::time::Duration;
use sqlx::{Error, Executor, MySql, Pool, Row};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult};
use crate::data_structs::app_start_request::{ApplicationStart, ApplicationStopped, BUCourse, DeviceMeta};
use crate::data_structs::signed_response::GrantLevel;

#[derive(Debug)]
pub struct DatabasePool {
    pool: Pool<MySql>,
    connection_url: String
}
impl DatabasePool {

    pub fn clone(&self) -> DatabasePool {
        DatabasePool {
            pool: self.pool.clone(),
            connection_url: self.connection_url.clone()
        }
    }

    pub async fn new(host: &str, port: i16, user: &str, pass: &str, database: &str) -> Self {
        let connection_url = format!("mysql://{user}:{pass}@{host}:{port}/{database}");

        let pool = match MySqlPoolOptions::new()
            .max_connections(3)
            .min_connections(3)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&connection_url).await {
                    Ok(res) => res,
                    Err(err) => panic!("Unable to connect to the database")
                };

        DatabasePool { pool: pool, connection_url}
    }

    pub async fn init(&self) {
        Self::create_tables(&self).await;
    }

    pub async fn is_authenticated(&self, kerberos_username: &String, auth_key: &String) -> bool {
        let result = sqlx::query("SELECT * from users WHERE kerberos_username=? AND authentication_key=?")
            .bind(&kerberos_username)
            .bind(&auth_key)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the is_authenticated query");
        return !result.is_empty();
    }

    pub async fn get_user_grant(&self, kerberos_username: &String) -> GrantLevel {
        let result = sqlx::query("SELECT * from users WHERE kerberos_username=?")
            .bind(&kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the get_user_grant query");

        // then this is the first time this user has used
        // this app so we create an acc for them
        if result.is_empty() {
            Self::create_user(&self, kerberos_username).await;
            return GrantLevel::Demo;  // all new users will have the demo grant
        } else {
            let row_res = result.get(0).unwrap();
            // check premium status
            let premium_since = row_res.get_unchecked::<Option<i64>, &str>("premium_since");
            if premium_since.is_some() {
                let premium_expiry = row_res.get_unchecked::<Option<i64>, &str>("premium_expiry").unwrap();
                let current_time = chrono::Local::now().timestamp();
                if current_time < premium_expiry {
                    return GrantLevel::Full;
                }
            }
            // check demo status
            let has_demo_credit = row_res.get_unchecked::<bool, &str>("has_demo_credit");
            return if has_demo_credit {
                GrantLevel::Demo
            } else {
                GrantLevel::None
            }
        }

    }

    pub async fn session_ping(&self, session_id: i64) -> Result<&str, &str> {
        // first some sanity checks to ensure the session is still active
        if Self::is_session_alive(&self, session_id).await {
            return Err("Session not found or is no longer alive")
        }

        // now update ping
        sqlx::query("UPDATE application_launch_session SET last_ping=current_timestamp WHERE session_id=?")
            .bind(&session_id)
            .execute(&self.pool).await
            .expect("Error executing the session_ping query");

        return Ok("Pong!")
    }

    // todo make this clearner bc the strings arent actually ever used
    pub async fn mark_course_registered(&self, session_id: i64, registration_timestamp: i64, course: BUCourse) -> Result<&str, &str> {
        // sanity check to ensure session is alive
        if !Self::is_session_alive(&self, session_id).await {
            return Err("Session not found or is no longer alive")
        }

        sqlx::query("UPDATE session_courses SET register_timestamp=? WHERE session_id=? AND semester_key=? AND college=? AND dept=? AND course=? AND section=?")
            .bind(&registration_timestamp)
            .bind(&session_id)
            .bind(&course.semester_key)
            .bind(&course.college)
            .bind(&course.department)
            .bind(&course.course)
            .bind(&course.section)
            .execute(&self.pool).await
            .expect("Error executing the mark_course_registered query");

        return Ok("OK")
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
                sleep_time_std, num_registered, terminate_timestamp)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&session_data.session_id)
            .bind(&session_data.did_finish)
            .bind(&session_data.unknown_crash_occured)
            .bind(&session_data.reason)
            .bind(&session_data.avg_cycle_time)
            .bind(&session_data.std_cycle_time)
            .bind(&session_data.avg_sleep_time)
            .bind(&session_data.std_sleep_time)
            .bind(&session_data.num_registered)
            .bind(&session_data.timestamp)
            .execute(&self.pool).await
            .expect("Error executing the end_session query");

        return Ok("OK")
    }

    pub async fn create_session(&self, session_data: &ApplicationStart, grant_level: &GrantLevel) -> i64 {
        // write the session data to the database and return the session_id key
        let result = sqlx::query(
            r#"INSERT INTO application_launch_session
                (kerberos_username, device_ip, device_os, system_arch,
                device_cores, device_clock_speed, grant_type, launch_time)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#)
            .bind(&session_data.credentials.kerberos_username)
            .bind(&session_data.device_meta.ip)
            .bind(&session_data.device_meta.os)
            .bind(&session_data.device_meta.system_arch)
            .bind(&session_data.device_meta.core_count)
            .bind(&session_data.device_meta.cpu_speed)
            .bind(&grant_level.to_string())
            .bind(chrono::Local::now().timestamp())
            .execute(&self.pool).await
            .expect("Error executing the create_session query");

        let session_id = result.last_insert_id() as i64;

        // write the courses to the database as well
        for course in &session_data.target_courses {
            sqlx::query("INSERT INTO session_courses (session_id, semester_key, college, dept, course, section) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&session_id)
                .bind(&course.semester_key)
                .bind(&course.college)
                .bind(&course.department)
                .bind(&course.course)
                .bind(&course.section)
                .execute(&self.pool).await
                .expect("Error executing the create_session query");
        }

        return session_id;
    }

    async fn create_user(&self, kerberos_username: &String) {
        sqlx::query("INSERT INTO users (kerberos_username) VALUES (?)")
            .bind(&kerberos_username)
            .execute(&self.pool).await
            .expect("Error executing the create_user query");
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

    pub async fn has_active_session(&self, kerberos_username: &str) -> Option<DeviceMeta> {
        let result = sqlx::query("SELECT * from application_launch_session WHERE kerberos_username=? AND is_active=1")
            .bind(kerberos_username)
            .fetch_all(&self.pool).await
            .expect("Error fetching rows for the session_ping query");

        return if result.is_empty() {
            None
        } else {
            let row = result.get(0).unwrap();
            Option::from(DeviceMeta {
                ip: row.get_unchecked::<String, &str>("device_ip"),
                os: row.get_unchecked::<String, &str>("device_os"),
                system_arch: row.get_unchecked::<String, &str>("system_arch"),
                core_count: row.get_unchecked::<i16, &str>("device_cores"),
                cpu_speed: row.get_unchecked::<f32, &str>("device_clock_speed")
            })
        }
    }

    async fn create_tables(&self) {
        Self::create_user_table(&self).await
            .expect("An error occurred create the 'users' table");
        Self::create_launch_tracker_table(&self).await
            .expect("An error occurred create the 'application_launch_session' table");
        Self::create_session_courses_table(&self).await
            .expect("An error occurred create the 'session_courses' table");
        Self::create_session_end_table(&self).await
            .expect("An error occurred create the 'application_terminate_session' table");
    }

    async fn create_user_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists users (
            kerberos_username      varchar(64)                                   not null,
            authentication_key     varchar(64)                                   not null,
            has_demo_credit        tinyint(1)  default 1                         not null,
            premium_since          bigint                                        null,
            premium_expiry         bigint                                        null,
            registration_timestamp bigint      default CURRENT_TIMESTAMP         not null,
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
            kerberos_username  varchar(64)                       not null,
            device_ip          varchar(16)                               not null,
            device_os          varchar(32)                       null,
            system_arch        varchar(12)                       null,
            device_cores       smallint                          null,
            device_clock_speed float                             null,
            grant_type         enum('Full', 'Demo', 'None')      not null,
            launch_time        bigint                            not null,
            is_active          tinyint(1)                        default 1 not null,
            last_ping          bigint default current_timestamp  not null,
            primary key (session_id),
            foreign key (kerberos_username) references users (kerberos_username)
        );
        "#).await
    }

    async fn create_session_courses_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists session_courses
        (
            session_id          int          not null,
            semester_key        varchar(12)  not null,
            college             varchar(6)   not null,
            dept                varchar(6)   not null,
            course              smallint     not null,
            section             varchar(6)   not null,
            register_timestamp  bigint       null,
            primary key (session_id, semester_key, college, dept, course, section),
            foreign key (session_id) references application_launch_session (session_id)
        );
        "#).await
    }

    async fn create_session_end_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists application_terminate_session
        (
            session_id           int auto_increment,
            did_finish           tinyint(1)   not null,
            unknown_crash        tinyint(1)   not null,
            reason               varchar(512) not null,
            avg_cycle_time       float        not null,
            cycle_time_std       float        not null,
            avg_sleep_time       float        not null,
            sleep_time_std       float        not null,
            num_registered       tinyint      not null,
            terminate_timestamp  bigint       not null,
            primary key (session_id),
            foreign key (session_id) references application_launch_session (session_id)
        );
        "#).await
    }

}