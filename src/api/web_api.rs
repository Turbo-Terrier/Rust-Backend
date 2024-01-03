use std::collections::BTreeMap;
use actix_web::{get, HttpRequest, HttpResponse, Responder, web};
use serde_json::value::Serializer;
use crate::data_structs::user::User;
use crate::google_oauth::GoogleAuthCode;
use crate::SharedResources;

#[get("/ping")]
async fn debug_ping() -> impl Responder {
    // just to test that the server is running
    "pong!"
}

#[get("/register")]
async fn oauth_register(data: web::Data<SharedResources>, info: web::Query<GoogleAuthCode>) -> impl Responder {
    let client_secrets = &data.get_ref().google_client_secret;
    let database = &data.get_ref().database;
    let jwt_secret = &data.get_ref().jwt_secret;

    let code = info.0.code.as_str();
    // just to test that the server is running
    let access_token = client_secrets.get_access_token(code).await;
    let user_info = client_secrets.get_user_info(access_token.access_token.as_str()).await;

    let user = database.create_or_update_user(&user_info, &access_token).await;
    let jwt_user_token = jwt_secret.encrypt_jwt_token(user);

    let mut map = BTreeMap::new();
    map.insert("token", jwt_user_token.as_str());

    HttpResponse::Ok().json(map)
}

#[get("/oauth-url")]
async fn oauth_url(data: web::Data<SharedResources>) -> impl Responder {
    let client_secrets = &data.get_ref().google_client_secret;
    client_secrets.create_oauth_uri()
}

#[get("/profile-info")]
async fn profile_info(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_some() {
        let user_auth_str = auth_header.unwrap().to_str().unwrap();
        let user = jwt_secret.decrypt_jwt_token::<User>(user_auth_str);
        return HttpResponse::Ok().json(user.claims()); //todo: shouldnt send back users own access token
    } else {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }
}