use std::fs::File;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use actix_cors::Cors;
use actix_web::{App, Handler, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use actix_web::rt::time;
use env_logger::Env;
use futures::FutureExt;
use lettre::SmtpTransport;
use ring::signature::KeyPair;
use sqlx::{Database, Executor};
use untrusted::{self};
use yaml_rust::{Yaml, YamlLoader};
use yaml_rust::yaml::Array;

use api::app_api;
use api::web_api;
use database::DatabasePool;
use encrypted_signing::Ed25519SecretKey;
use google_oauth::GoogleClientSecretWrapper;

use crate::api::stripe_hook;
use crate::encrypted_signing::JWTSecretKey;
use crate::google_oauth::GoogleClientSecret;
use crate::stripe_util::{StripeHandler, TieredPrice};

pub mod database;
mod encrypted_signing;
mod smtp_mailing_util;
mod google_oauth;
mod stripe_util;
mod course_list_scraper;

pub mod data_structs {
    pub mod user;
    pub mod semester;
    pub mod device_meta;
    pub mod bu_course;
    pub mod grant_level;
    pub mod app_config;
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

pub mod api;


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
    let product_id: &str = stripe_config["product-id"].as_str().expect("stripe.product-id not found!");
    let tiered_pricing: &Array = stripe_config["pricing"].as_vec().expect("stripe.pricing not found!");
    let tiered_pricing: Vec<TieredPrice> = tiered_pricing.into_iter().map(|price| {
        let required_quantity = price["required-quantity"].as_i64().expect("stripe.pricing.required-quantity not found!");
        let price = price["unit-price"].as_f64().expect("stripe.pricing.unit-price not found!");
        TieredPrice::new(required_quantity as u64, price)
    }).collect();
    assert_ne!(tiered_pricing.len(), 0, "stripe.pricing must have at least 1 pricing!");
    let stripe_handler = StripeHandler::new(stripe_secret.to_string(), stripe_webhook_secret.to_string(), product_id.parse().unwrap(), tiered_pricing);

    let shared_resources = SharedResources {
        private_key,
        database,
        smtp_transport,
        google_client_secret,
        jwt_secret,
        base_url,
        stripe_handler
    };
    // todo: add a referral program
    return Ok(shared_resources);
}

// todo: Vonage API for voice alerts.

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let shared_resources = load().await.unwrap();
    let copied_resource_1 = shared_resources.clone();
    let copied_resource_2 = shared_resources.clone();

    println!("Starting cleanup task");
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(5000));
        loop {
            let cleanup_start_time = Instant::now();
            copied_resource_1.database.cleanup_dead_sessions().await;
            let task_time = cleanup_start_time.elapsed().as_millis();
            // as the database grows, this task will take longer to complete
            // if it takes longer than 9 seconds, we should warn ourselves
            if task_time > 9000 {
                println!("Warning: cleanup task took {}ms to complete", task_time);
            }
            interval.tick().await;
        }
    });

    println!("Starting course scraping task");
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60 * 60 * 3)); //3 hrs
        loop {
            let course_find_task = Instant::now();
            course_list_scraper::discover_regular_semesters(&copied_resource_2.database).await;
            // todo: order matters here since summer courses search based on departments already in db
            course_list_scraper::discover_summer_courses(&copied_resource_2.database).await;
            let _ = course_find_task.elapsed().as_millis();
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
                .service(web_api::update_user_app_settings)
                .service(web_api::get_user_app_settings)
                .service(web_api::add_course)
                .service(web_api::del_course)
                .service(web_api::get_available_courses)
                .service(web_api::get_active_semesters)
                .service(web_api::payment_status)
                .service(web_api::pricing)
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