use lettre::{Message, SmtpTransport, Transport};
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::PoolConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
#[derive(Deserialize, Serialize)]
pub struct Email {
    pub subject: String,
    pub sender_name: String,
    pub mail_body: String,
}

impl Email {

    pub fn new(subject: String, sender_name: String, mail_body: String) -> Email {
        return Email {
            subject,
            sender_name,
            mail_body
        }
    }
	
	// todo: add the List-unsubcribe header
    pub fn send(&self, smtp_transport: SmtpTransport, recipient: &str) {
        let email_message = Message::builder()
            .from("no-reply@aseef.dev".parse().unwrap())
            .to(recipient.parse().unwrap())
            .subject(&self.subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_PLAIN)
                            .body(self.mail_body.clone())
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(lettre::message::header::ContentType::TEXT_HTML)
                            .body(self.mail_body.clone())
                    )
            ).unwrap();
        smtp_transport.send(&email_message).unwrap();
    }

}

pub fn create_smtp_transport(host: &str, port: u16, username: &str, password: &str) -> SmtpTransport {
    let creds = Credentials::new(username.to_owned(), password.to_owned());

    let smtp_transport = SmtpTransport::starttls_relay(host)
        .unwrap()
        .port(port)
        .credentials(creds)
        .authentication(vec![Mechanism::Plain])
        .pool_config(PoolConfig::new().max_size(5))
        .build();
    smtp_transport.test_connection().expect("Failed to connect to SMTP server");
    return smtp_transport;
}