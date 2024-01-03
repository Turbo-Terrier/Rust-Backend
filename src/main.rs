pub mod database;
mod encrypted_signing;
mod smtp_mailing_util;
mod google_oauth;

pub mod data_structs {
    pub mod app_start_request;
    pub mod signed_response;
}

pub mod api {
    pub mod app_api;
    pub mod web_api;
}


use std::fs::File;
use database::DatabasePool;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use yaml_rust::{Yaml, YamlLoader};
use actix_web::{App, HttpServer, middleware, Responder, web};
use actix_web::middleware::Logger;
use actix_web::rt::time;
use lettre::SmtpTransport;
use sqlx::{Database, Executor};
use ring::signature::KeyPair;
use untrusted::{self};
use api::app_api;
use encrypted_signing::Ed25519SecretKey;
use google_oauth::GoogleClientSecretWrapper;
use api::web_api;
use crate::google_oauth::GoogleClientSecret;

pub struct SharedResources {
    private_key: Ed25519SecretKey,
    database: DatabasePool,
    smtp_transport: SmtpTransport,
    google_client_secret: GoogleClientSecret
}

impl Clone for SharedResources {
    fn clone(&self) -> Self {
        return SharedResources {
            private_key: self.private_key.clone(),
            database: self.database.clone(),
            smtp_transport: self.smtp_transport.clone(),
            google_client_secret: self.google_client_secret.clone()
        }
    }
}

pub fn read_file_as_str(file_path: &str) -> String {
    let mut buf: String = String::new();
    let mut file = File::open(file_path)
        .expect("Error! A config.yml file was not found in the current directory.");
    file.read_to_string(&mut buf).expect("Error reading config.yml!");
    return buf;
}

async fn load() -> Result<SharedResources, std::io::Error> {
    println!("Loading configurations...");

    let mut buf: String = read_file_as_str("config.yml");
    let config: Vec<Yaml> = YamlLoader::load_from_str(&mut buf).expect("Error loading yml file");
    let config: &Yaml = &config[0];

    println!("Connecting to the database...");

    let creds: &Yaml = &config["mysql"];
    let host: &str = creds["host"].as_str().expect("mysql.host not found!");
    let port: i16 = creds["port"].as_i64().expect("mysql.port not found!") as i16;
    let user: &str = creds["username"].as_str().expect("mysql.user not found!");
    let pass: &str = creds["password"].as_str().expect("mysql.password not found!");
    let database: &str = creds["database"].as_str().expect("mysql.database not found!");
    let database: DatabasePool = DatabasePool::new(host, port, user, pass, database).await;
    database.init().await;

    println!("Loading Google OAuth2 Secrets");
    let oauth_config_location = &config["google-client-secret"].as_str()
        .expect("google-client-secret not found!");
    let mut buf: String = read_file_as_str(oauth_config_location);
    let oauth_creds: GoogleClientSecretWrapper = serde_json::from_str::<GoogleClientSecretWrapper>(&mut buf)
        .expect("Error parsing google-client-secret file!");
    let google_client_secret: GoogleClientSecret = oauth_creds.web;

    println!("Loading encryption keys");

    let private_key_path = config["ed25519-private-key"].as_str()
        .expect("ed25519-private-key not found!");
    let private_key = Ed25519SecretKey::new(private_key_path);

    println!("Loading SMTP configuration");
    let smtp_config = &config["smtp"];
    let smtp_host = smtp_config["host"].as_str().expect("smtp.host not found!");
    let smtp_port = smtp_config["port"].as_i64().expect("smtp.port not found!") as u16;
    let smtp_username = smtp_config["username"].as_str().expect("smtp.username not found!");
    let smtp_password = smtp_config["password"].as_str().expect("smtp.password not found!");
    let smtp_transport = smtp_mailing_util::create_smtp_transport(smtp_host, smtp_port, smtp_username, smtp_password);

    let shared_resources = SharedResources {
        private_key,
        database,
        smtp_transport,
        google_client_secret
    };

    return Ok(shared_resources);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let shared_resources = load().await.unwrap();
    let copied_resource = shared_resources.clone();

    println!("Starting cleanup task");
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(10000));
        loop {
            let cleanup_start_time = Instant::now();
            copied_resource.database.cleanup_dead_sessions().await;
            let task_time = cleanup_start_time.elapsed().as_millis();
            // as the database grows, this task will take longer to complete
            // if it takes longer than 9 seconds, we should warn ourselves
            if task_time > 9000 {
                println!("Warning: cleanup task took {}ms to complete", task_time);
            }
            interval.tick().await;
        }
    });

    println!("Starting HTTP server...");
    HttpServer::new( move || {
        App::new()
            .app_data(web::Data::new(shared_resources.clone()))
            .wrap(Logger::new("%a \"%r\" %s %b \"%{User-Agent}i\" %T")) //todo: doesnt work as expected
            .service(web::scope("/api/app/v1",)
                .service(app_api::app_start)
                .service(app_api::app_stop)
                .service(app_api::send_mail)
                .service(app_api::course_registered)
                .service(app_api::ping)
                .service(app_api::debug_ping)
            )
            .service(web::scope("/api/web/v1")
                .service(web_api::debug_ping)
                .service(web_api::oauth_register)
                .service(web_api::oauth_url)
            )
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await

}