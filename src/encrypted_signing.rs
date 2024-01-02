use std::fs::File;
use std::io::Read;
use ring::signature::{Ed25519KeyPair, Signature};
use crate::data_structs::signed_response::SignableData;

const KEY_SIZE: usize = 48;

#[derive(Debug)]
pub struct Ed25519SecretKey {
    key_bytes: [u8; KEY_SIZE],
    private_key: Ed25519KeyPair,
}

impl Ed25519SecretKey {
    pub fn new(priv_key_path: &str) -> Ed25519SecretKey {
        let mut priv_key: Vec<u8> = Vec::new();
        File::open(priv_key_path)
            .expect(&*format!("Could not open private-key file at {}", priv_key_path))
            .read_to_end(&mut priv_key)
            .expect("Error reading private-key!");

        assert_eq!(priv_key.len(), KEY_SIZE, "Private key size incorrect!");

        return Ed25519SecretKey {
            key_bytes: <[u8; KEY_SIZE]>::try_from(priv_key.as_slice()).unwrap(),
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