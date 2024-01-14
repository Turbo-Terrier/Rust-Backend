use actix_web::cookie::time::Duration;
use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use actix_web::cookie::Cookie;
use actix_web::http::header;
use actix_web::web::Redirect;
use stripe::{PriceId, Product};
use crate::data_structs::user::User;
use crate::google_oauth::{GoogleAuthCode, GoogleClientSecret};
use crate::{SharedResources, stripe_util};
use crate::data_structs::responses::web_register_response::{WebRegisterResponse};

#[get("/ping")]
async fn debug_ping() -> impl Responder {
    // just to test that the server is running
    "Pong!"
}

#[post("/register")]
async fn oauth_register(data: web::Data<SharedResources>, info: web::Json<GoogleAuthCode>) -> impl Responder {
    let client_secrets = &data.get_ref().google_client_secret;
    let database = &data.get_ref().database;
    let jwt_secret = &data.get_ref().jwt_secret;
    let stripe_handler = &data.get_ref().stripe_handler;
    let base_url = &data.get_ref().base_url;

    let code = &info.code;
    // just to test that the server is running
    let access_token = client_secrets.get_access_token(code).await; //todo error handling for wrong code
    let user_info = client_secrets.get_user_info(access_token.access_token.as_str()).await;

    let user = database.create_or_update_user(&user_info, &access_token, &stripe_handler).await;
    let jwt_user_token = jwt_secret.encrypt_jwt_token(user);

    HttpResponse::Ok().json(WebRegisterResponse {
        jwt_cookie: jwt_user_token.as_str().to_string(),
        user: jwt_user_token.claims().clone()
    })
}

#[get("/oauth-url")]
async fn oauth_url(data: web::Data<SharedResources>) -> impl Responder {
    let client_secrets = &data.get_ref().google_client_secret;
    let oauth_url = client_secrets.create_oauth_uri();
    HttpResponse::TemporaryRedirect()
        .header("Location", oauth_url)
        .finish()
}

#[get("/logout")]
async fn logout(data: web::Data<SharedResources>) -> impl Responder {
    let client_secrets: &GoogleClientSecret = &data.get_ref().google_client_secret;
    let oauth_uri: String = client_secrets.create_oauth_uri();
    HttpResponse::Ok()
        .cookie(Cookie::build("jwt-token", "").max_age(Duration::new(0, 0)).finish())
        .header("Location", "/")
        .finish()
}

#[get("/profile-info")]
async fn profile_info(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let user = jwt_secret.decrypt_jwt_token::<User>(user_auth_str);

    return if user.is_none() {
        HttpResponse::Unauthorized().json("Invalid")
    } else {
        let unwrapped_user = user.unwrap();
        HttpResponse::Ok().json(unwrapped_user.claims())
    }


}

#[get("/reset-app-token")]
async fn reset_app_token(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let user = jwt_secret.decrypt_jwt_token::<User>(user_auth_str);

    if user.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let mut user = user.unwrap().claims().to_owned();
    let new_auth_token = database.reset_authentication_key(&user.kerberos_username).await;
    user.authentication_key = new_auth_token;

    let updated_jwt = jwt_secret.encrypt_jwt_token(user);

    return HttpResponse::Ok()
        .json(WebRegisterResponse {
            jwt_cookie: updated_jwt.as_str().to_string(),
            user: updated_jwt.claims().clone()
        });
}

#[post("/create-checkout-session")]
pub async fn create_checkout_session(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let stripe_handler = &data.get_ref().stripe_handler;
    let auth_header = req.headers().get("Authorization");

    // let price: Expandable<Price> = product.default_price.unwrap();
    //         let price_id = &price.as_object().unwrap().id;
    let target_product: PriceId = "TODO".parse().unwrap(); //todo

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let user = jwt_secret.decrypt_jwt_token::<User>(user_auth_str);

    if user.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let user = user.unwrap().claims().to_owned();

    let checkout_session = stripe_handler.create_stripe_checkout_session(
        &data.get_ref().base_url,
        user.stripe_id.as_str().parse().unwrap(),
        &target_product
    ).await;

    HttpResponse::Ok().json(checkout_session) //todo maybe just return url?
}

#[post("/contact-request")]
pub async fn contact_request(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .json("None")
}

#[get("/payment/{status}")]  //todo: actually on second thought this should route back to the portal page or whatever
pub async fn payment_status(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .json("None")
}