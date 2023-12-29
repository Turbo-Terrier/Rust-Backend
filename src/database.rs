use std::time::Duration;
use sqlx::{Error, Executor, MySql, Pool};
use sqlx::mysql::{MySqlPoolOptions, MySqlQueryResult};

pub(crate) struct DatabasePool {
    pool: Pool<MySql>,
    connection_url: String
}
impl DatabasePool {

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

        DatabasePool {pool, connection_url}
    }

    pub async fn create_tables(&self) {
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
            kerberos_username      varchar(64)                            NOT NULL,
            license_key            varchar(255)                           NULL,
            premium_since          bigint                                 NULL,
            premium_expiry         bigint                                 NULL,
            registration_timestamp bigint     DEFAULT CURRENT_TIMESTAMP   NOT NULL,
            PRIMARY KEY (kerberos_username)
        );
        "#).await
    }

    async fn create_launch_tracker_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists application_launch_session
        (
            session_id         int auto_increment,
            kerberos_username  varchar(64) not null,
            device_ip          int         not null,
            device_os          varchar(32) null,
            system_arch        varchar(12) null,
            device_cores       smallint    null,
            device_clock_speed float       null,
            launch_time        bigint      not null,
            primary key (session_id),
            foreign key (kerberos_username) references users (kerberos_username)
        );
        "#).await
    }

    async fn create_session_courses_table(&self) -> Result<MySqlQueryResult, Error> {
        self.pool.execute(r#"
        create table if not exists session_courses
        (
            session_id          int          auto_increment,
            semester_key        varchar(12)  not null,
            college             varchar(8)   not null,
            dept                varchar(8)   null,
            course              varchar(8)   null,
            section             smallint(8)  null,
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
            cause                varchar(512) not null,
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