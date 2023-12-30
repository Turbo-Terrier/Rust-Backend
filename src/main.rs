pub mod database;
pub mod api;
pub mod utils;

pub mod data_structs {
    pub mod app_start_request;
    pub mod signed_response;
}


use database::DatabasePool;
use std::fs::File;
use std::io::{Read, Write};
use yaml_rust::YamlLoader;
use actix_web::{App, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use sqlx::{Database, Executor};
use ring::signature::KeyPair;
use ring::signature::Ed25519KeyPair;
use untrusted::{self};
use rand;
use serde::Deserialize;

pub struct SharedResources {
    priv_key: Ed25519KeyPair,
    database: DatabasePool
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Loading configurations...");

    let mut buf: String = utils::read_file_as_str("config.yml");
    let config = match YamlLoader::load_from_str(&mut buf) {
        Ok(config) => config,
        Err(err) => panic!("Error loading yml file")
    };
    let config = &config[0];

    println!("Connecting to the database...");

    let creds = &config["mysql"];
    let host = creds["host"].as_str().expect("mysql.host not found!");
    let port = creds["port"].as_i64().expect("mysql.port not found!") as i16;
    let user = creds["username"].as_str().expect("mysql.user not found!");
    let pass = creds["password"].as_str().expect("mysql.password not found!");
    let database = creds["database"].as_str().expect("mysql.database not found!");
    let database: DatabasePool = DatabasePool::new(host, port, user, pass, database).await;
    database.init().await;

    println!("Loading encryption keys");

    let private_key_path = config["ed25519-private-key"].as_str()
        .expect("ed25519-private-key not found!");
    let private_key = utils::load_priv_key(private_key_path);

    let shared_resources = SharedResources {
        priv_key: private_key, database
    };

    println!("Starting HTTP server...");
    HttpServer::new(|| {
        App::new()
            .app_data(shared_resources)
            .wrap(Logger::default())
            .service(api::app_start)
            .service(api::app_stop)
            .service(api::send_mail)
            .service(api::course_registered)
            .service(api::ping)
            .service(api::debug_ping)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await

}