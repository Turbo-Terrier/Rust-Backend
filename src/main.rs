pub mod database;
mod encrypted_signing;
mod smtp_mailing_util;
mod google_oauth;
mod stripe_util;

pub mod data_structs {
    pub mod user;
    pub mod semester;
    pub mod device_meta;
    pub mod app_credentials;
    pub mod bu_course;
    pub mod grant_level;
    pub mod requests {
        pub mod application_start;
        pub mod application_stopped;
        pub mod email_send_request;
        pub mod registration_notification;
        pub mod session_ping;
    }
    pub mod responses {
        pub mod app_start_permission;
        pub mod signable_data;
        pub mod status_response;
        pub mod web_register_response;
    }
}

pub mod api {
    pub mod app_api;
    pub mod web_api;
    pub mod stripe_hook;
}


use std::fs::File;
use database::DatabasePool;
use std::io::{Read, Write};
use std::time::{Duration, Instant};
use actix_cors::Cors;
use yaml_rust::{Yaml, YamlLoader};
use actix_web::{App, Handler, http, HttpServer, middleware, Responder, web};
use actix_web::middleware::Logger;
use actix_web::rt::time;
use env_logger::Env;
use futures::FutureExt;
use lettre::SmtpTransport;
use sqlx::{Database, Executor};
use ring::signature::KeyPair;
use stripe::Client;
use untrusted::{self};
use api::app_api;
use encrypted_signing::Ed25519SecretKey;
use google_oauth::GoogleClientSecretWrapper;
use api::web_api;
use crate::api::stripe_hook;
use crate::data_structs::semester::Semester;
use crate::encrypted_signing::JWTSecretKey;
use crate::google_oauth::GoogleClientSecret;
use crate::stripe_util::StripeHandler;

#[derive(Clone)]
pub struct SharedResources {
    private_key: Ed25519SecretKey,
    database: DatabasePool,
    smtp_transport: SmtpTransport,
    google_client_secret: GoogleClientSecret,
    jwt_secret: JWTSecretKey,
    base_url: String,
    stripe_handler: StripeHandler,
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

    let base_url: &str = config["base-url"].as_str().expect("base-url not found!");
    let base_url: String = base_url.to_string();

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

    let jwt_secret: &str = config["jwt-secret-key"].as_str()
        .expect("jwt-secret-key not found!");
    let jwt_secret: JWTSecretKey = JWTSecretKey::new(jwt_secret.to_string());

    println!("Loading SMTP configuration");
    let smtp_config: &Yaml = &config["smtp"];
    let smtp_host: &str = smtp_config["host"].as_str().expect("smtp.host not found!");
    let smtp_port: u16 = smtp_config["port"].as_i64().expect("smtp.port not found!") as u16;
    let smtp_username: &str = smtp_config["username"].as_str().expect("smtp.username not found!");
    let smtp_password: &str = smtp_config["password"].as_str().expect("smtp.password not found!");
    let smtp_transport = smtp_mailing_util::create_smtp_transport(smtp_host, smtp_port, smtp_username, smtp_password);

    println!("Loading Stripe configurations");
    let stripe_config: &Yaml = &config["stripe"];
    let stripe_secret: &str = stripe_config["secret-key"].as_str().expect("stripe.secret-key not found!");
    let stripe_webhook_secret: &str = stripe_config["webhook-signing-secret"].as_str().expect("stripe.webhook-signing-secret not found!");
    let base_price_regular: i64  = stripe_config["normal-session-base-price"].as_i64().expect("stripe.summer-session-base-price not found!");
    let base_price_summer: i64 = stripe_config["summer-session-base-price"].as_i64().expect("stripe.summer-session-base-price not found!");
    let stripe_handler = StripeHandler::new(stripe_secret.to_string(), stripe_webhook_secret.to_string(), base_price_regular, base_price_summer);

    let shared_resources = SharedResources {
        private_key,
        database,
        smtp_transport,
        google_client_secret,
        jwt_secret,
        base_url,
        stripe_handler
    };

    shared_resources.stripe_handler.create_or_get_products(&shared_resources).await;

    return Ok(shared_resources);
}

// todo: Vonage API for voice alerts.

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
    env_logger::init_from_env(Env::default().default_filter_or("info")); // enables built in actix logger
    HttpServer::new( move || {
        App::new()
            .app_data(web::Data::new(shared_resources.clone()))
            .wrap(Logger::new("%a \"%r\" %s %b \"%{User-Agent}i\" %T"))
            // Enable CORS
            .wrap(
                Cors::permissive()
            )
            .service(web::scope("/api/app/v1")
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
                .service(web_api::profile_info)
                .service(web_api::reset_app_token)
                .service(web_api::test_web_auth)
                .service(web_api::create_checkout_session)
                .service(web_api::payment_status)
            )
            .service(web::scope("/api/stripe/v1")
                .service(stripe_hook::webhook_handler)
            )
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await

}