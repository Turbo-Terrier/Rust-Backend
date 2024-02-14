use serde::Serialize;

pub trait SignableData: Serialize {
    /** This is the string the client side will check to make sure was signed by the server */
    fn string_to_sign(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}