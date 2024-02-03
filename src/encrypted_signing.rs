use std::fs::File;
use std::io::Read;

use hmac::Hmac;
use jwt::{AlgorithmType, Header, SignWithKey, Token, Verified, VerifyWithKey};
use jwt::token::Signed;
use ring::signature::{Ed25519KeyPair, Signature};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::digest::KeyInit;
use sha2::Sha384;

use crate::data_structs::responses::signable_data::SignableData;

const ED25519_KEY_SIZE: usize = 48;

#[derive(Debug)]
pub struct Ed25519SecretKey {
    key_bytes: [u8; ED25519_KEY_SIZE],
    private_key: Ed25519KeyPair,
}

#[derive(Clone)]
pub struct JWTSecretKey {
    pub secret_key: String,
}

impl JWTSecretKey {

    pub fn new(secret_key: String) -> JWTSecretKey {
        return JWTSecretKey {
            secret_key
        };
    }

    pub fn encrypt_jwt_token<T: Serialize>(&self, data: T) -> Token<Header, T, Signed> {
        let key: Hmac<Sha384> = Hmac::new_from_slice(self.secret_key.as_bytes()).unwrap();
        let header = Header {
            algorithm: AlgorithmType::Hs384,
            ..Default::default()
        };
        let token: Token<Header, T, Signed> = Token::new(header, data).sign_with_key(&key).unwrap();

        return token;
    }

    pub fn decrypt_jwt_token<T>(&self, str_token: &str) -> Option<Token<Header, T, Verified>>
            where T: DeserializeOwned, T: Clone {
        let key: Hmac<Sha384> = Hmac::new_from_slice(self.secret_key.as_bytes()).unwrap();
        let token: Result<Token<Header, T, Verified>, jwt::Error> = str_token.verify_with_key(&key);
        return token.ok();
    }

}

impl Ed25519SecretKey {
    pub fn new(priv_key_path: &str) -> Ed25519SecretKey {
        let mut priv_key: Vec<u8> = Vec::new();
        File::open(priv_key_path)
            .expect(&*format!("Could not open private-key file at {}", priv_key_path))
            .read_to_end(&mut priv_key)
            .expect("Error reading private-key!");

        assert_eq!(priv_key.len(), ED25519_KEY_SIZE, "Private key size incorrect!");

        return Ed25519SecretKey {
            key_bytes: <[u8; ED25519_KEY_SIZE]>::try_from(priv_key.as_slice()).unwrap(),
            private_key: Ed25519KeyPair::from_pkcs8_maybe_unchecked(priv_key.as_slice())
            .expect("Error loading the ed25519 private key from bytes!")
        };
    }

    pub fn sign<T: SignableData>(&self, args: &T) -> String {
        let signature: Signature = self.private_key.sign(args.string_to_sign().as_bytes());
        let str_signature = base64::encode(signature.as_ref());
        return str_signature;
    }
}

impl Clone for Ed25519SecretKey {
    fn clone(&self) -> Self {
        return Ed25519SecretKey {
            key_bytes: self.key_bytes,
            private_key: Ed25519KeyPair::from_pkcs8_maybe_unchecked(&self.key_bytes)
                .expect("Error loading the ed25519 private key from bytes!")
        }
    }
}