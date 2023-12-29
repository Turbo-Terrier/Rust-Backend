mod signed_reponse;
mod database;

use database::DatabasePool;
use std::fs::File;
use std::io::{Read, Write};
use yaml_rust::{YamlLoader, YamlEmitter, Yaml, ScanError};
use actix_web::{get, web, App, HttpServer, Responder};
use sqlx::{Database, Executor};
use ring::{signature::{self, KeyPair}, };
use ring::signature::{ED25519, Ed25519KeyPair, UnparsedPublicKey};
use untrusted::{self, Input};
use rand;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {}!", name)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Loading configurations...");
    let mut buf: String = String::new();
    let mut file = File::open("./config.yml")
        .expect("Error! A config.yml file was not found in the current directory.");
    file.read_to_string(&mut buf).expect("Error reading config.yml!");
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
    database.create_tables().await;

    println!("Loading encryption keys");

    let priv_key_path = &config["ed25519-private-key"].as_str()
        .expect("ed25519-private-key not found!");
    let mut priv_key: Vec<u8> = Vec::new();
    File::open(priv_key_path)
        .expect(&*format!("Could not open private-key file at {}", priv_key_path))
        .read_to_end(&mut priv_key)
        .expect("Error reading private-key!");

    let priv_key: Ed25519KeyPair = Ed25519KeyPair::from_pkcs8_maybe_unchecked(priv_key.as_slice())
        .expect("Error loading the ed25519 private key from bytes!");

    println!("Starting HTTP server...");
    HttpServer::new(|| {
        App::new().service(greet)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await

}