pub trait SignableData {
    /** This is the string the client side will check to make sure was signed by the server */
    fn string_to_sign(&self) -> String;
}