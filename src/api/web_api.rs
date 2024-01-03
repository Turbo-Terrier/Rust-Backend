use actix_web::{get, HttpRequest, HttpResponse, Responder, web};
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

    let code = info.0.code.as_str();
    // just to test that the server is running
    let access_token = client_secrets.get_access_token(code).await;
    let user_info = client_secrets.get_user_info(access_token.access_token.as_str()).await;
    // todo: may also need to store access_token.expiry

    database.create_or_update_user(&user_info, &access_token).await;

    HttpResponse::Ok().json(user_info) //todo: complete this, this is temp
}

#[get("/oauth-url")]
async fn oauth_url(data: web::Data<SharedResources>) -> impl Responder {
    let client_secrets = &data.get_ref().google_client_secret;
    client_secrets.create_oauth_uri()
}