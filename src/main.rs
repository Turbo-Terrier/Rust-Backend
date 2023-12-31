pub mod database;
pub mod api;
mod encrypted_signing;

pub mod data_structs {
    pub mod app_start_request;
    pub mod signed_response;
}


use std::fs::File;
use database::DatabasePool;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use yaml_rust::YamlLoader;
use actix_web::{App, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use sqlx::{Database, Executor};
use ring::signature::KeyPair;
use untrusted::{self};
use rand;
use serde::Deserialize;
use crate::encrypted_signing::Ed25519SecretKey;

#[derive(Debug)]
pub struct SharedResources {
    priv_key: Ed25519SecretKey,
    database: DatabasePool
}

impl Clone for SharedResources {
    fn clone(&self) -> Self {
        return SharedResources {
            priv_key: self.priv_key.clone(),
            database: self.database.clone()
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
    let private_key = Ed25519SecretKey::new(private_key_path);

    let shared_resources = SharedResources {
        priv_key: private_key, database
    };

    return Ok(shared_resources);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting HTTP server...");
    let shared_resources = match load().await {
        Ok(shared_resources) => shared_resources,
        Err(err) => panic!("ERR")
    };

    HttpServer::new( move || {
        App::new()
            .app_data(web::Data::new(shared_resources.clone()))
            .wrap(Logger::default())
            .service(web::scope("/api/v1",)
                .service(api::app_start)
                .service(api::app_stop)
                .service(api::send_mail)
                .service(api::course_registered)
                .service(api::ping)
                .service(api::debug_ping)
            )
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await

}