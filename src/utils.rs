use std::any::Any;
use std::fs::File;
use std::io::Read;
use ring::signature::Ed25519KeyPair;
use crate::data_structs::app_start_request::AppCredentials;
use crate::database::DatabasePool;

pub fn load_priv_key(priv_key_path: &str) -> Ed25519KeyPair {
    let mut priv_key: Vec<u8> = Vec::new();
    File::open(priv_key_path)
        .expect(&*format!("Could not open private-key file at {}", priv_key_path))
        .read_to_end(&mut priv_key)
        .expect("Error reading private-key!");

    return Ed25519KeyPair::from_pkcs8_maybe_unchecked(priv_key.as_slice())
        .expect("Error loading the ed25519 private key from bytes!");
}

pub fn read_file_as_str(file_path: &str) -> String {
    let mut buf: String = String::new();
    let mut file = File::open(file_path)
        .expect("Error! A config.yml file was not found in the current directory.");
    file.read_to_string(&mut buf).expect("Error reading config.yml!");
    return buf;
}

pub fn sign(the_pen: &Ed25519KeyPair, args: Vec<dyn Any>) -> String {
    let mut string_to_sign = String::new();
    for arg in args {
        string_to_sign.push_str(arg.to_string().as_str());
    }
    let signature = the_pen.sign(string_to_sign.as_bytes());
}