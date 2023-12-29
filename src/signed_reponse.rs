
#[derive(Debug)]
struct SignedResponse {
    kerberos_username: String,
    status: ResponseStatus,
    reason: String,
    response_timestamp: u64,
    signature: String
}

#[derive(Debug)]
enum  ResponseStatus {
    GOOD,
    WARNING,
    ERROR,
}

 impl SignedResponse {
    pub fn new(kerberos_username: String, status: ResponseStatus, reason: String, response_timestamp: u64, signature: String) -> Self {
        Self { kerberos_username, status, reason, response_timestamp, signature }
    }
    pub fn is_valid() -> bool {
        true
    }
    pub fn kerberos_username(&self) -> &String {
        &self.kerberos_username
    }
    pub fn status(&self) -> &ResponseStatus {
        &self.status
    }
    pub fn reason(&self) -> &String {
        &self.reason
    }
    pub fn response_timestamp(&self) -> u64 {
        self.response_timestamp
    }
    pub fn signature(&self) -> &String {
        &self.signature
    }

}