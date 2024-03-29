use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[derive(Serialize)]
#[derive(Debug)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
    pub locale: String,
    pub hd: String, // pretty sure this only exists for workspace email addresses (like @bu.edu)
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct GoogleAccessToken {
    pub access_token: String,
    pub id_token: String,
    pub expires_in: i32,
    pub token_type: String,
    pub scope: String,
    pub refresh_token: String,
}

#[derive(Debug)]
#[derive(serde::Deserialize)]
pub struct GoogleAuthCode {
    pub code: String,
    pub scope: String,
    pub authuser: String,
    pub prompt: String,
}

#[derive(Deserialize)]
#[derive(Debug)]
#[derive(Clone)]
pub struct GoogleClientSecret {
    pub client_id: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
}

impl GoogleClientSecret {

    pub fn create_oauth_uri(&self) -> String {
        let uri = format!("{}?redirect_uri={}&prompt=consent&response_type=code&client_id={}&scope={}&access_type=offline",
                              self.auth_uri,
                              self.redirect_uris[1],
                              self.client_id,
                              "openid+https://www.googleapis.com/auth/userinfo.email+https://www.googleapis.com/auth/userinfo.profile");
        return uri;
    }

    pub async fn get_access_token(&self, code: &str) -> GoogleAccessToken {
        let client = reqwest::Client::new();
        let response = client.post(&self.token_uri)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("code", code),
                    ("redirect_uri", self.redirect_uris[1].as_str()),
                    ("client_id", self.client_id.as_str()),
                    ("client_secret", self.client_secret.as_str()),
                    ("scope", ""),
                    ("grant_type", "authorization_code")]
            )
            .send().await.unwrap().text().await.unwrap();
        let response = serde_json::from_str::<GoogleAccessToken>(response.as_str()).unwrap(); //todo error handling
        return response;
    }

    pub async fn refresh_access_token(&self, refresh_token: &str) -> GoogleAccessToken {
        let client = reqwest::Client::new();
        let response = client.post(&self.token_uri)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("refresh_token", refresh_token),
                    ("client_id", self.client_id.as_str()),
                    ("client_secret", self.client_secret.as_str()),
                    ("grant_type", "refresh_token")])
            .send().await.unwrap().json::<GoogleAccessToken>().await.unwrap();
        return response;
    }

    pub async fn get_user_info(&self, access_token: &str) -> GoogleUserInfo {
        let client = reqwest::Client::new();
        let response = client.get("https://www.googleapis.com/oauth2/v1/userinfo")
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", access_token))
            .send().await.unwrap().json::<GoogleUserInfo>().await.unwrap();
        return response;
    }

}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct GoogleClientSecretWrapper {
    pub web: GoogleClientSecret,
}