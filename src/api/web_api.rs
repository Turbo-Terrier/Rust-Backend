use actix_web::cookie::time::Duration;
use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use actix_web::cookie::Cookie;
use actix_web::http::header;
use actix_web::web::Redirect;
use serde::Deserialize;
use stripe::{CheckoutSession, PriceId, Product};
use crate::data_structs::user::User;
use crate::google_oauth::{GoogleAuthCode, GoogleClientSecret};
use crate::{SharedResources, stripe_util};
use crate::data_structs::app_config::UserApplicationSettings;
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

    let code = &info.code;
    // just to test that the server is running
    let access_token = client_secrets.get_access_token(code).await; //todo error handling for wrong code
    let user_info = client_secrets.get_user_info(access_token.access_token.as_str()).await;

    // todo: if /register happens multiple times before first call finishes error happen
    // mutex needed

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

#[derive(Deserialize)]
struct Quantity(u64);
#[post("/create-checkout-session")]
pub async fn create_checkout_session(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<Quantity>) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let stripe_handler = &data.get_ref().stripe_handler;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let user = jwt_secret.decrypt_jwt_token::<User>(user_auth_str);

    if user.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let quantity = info.into_inner().0;
    let user = user.unwrap().claims().to_owned();

    let checkout_session: CheckoutSession = stripe_handler.create_stripe_checkout_session(
        &data.get_ref().base_url,
        user.stripe_id.as_str().parse().unwrap(),
        quantity,
        stripe_handler.get_unit_price(quantity)
    ).await;

    HttpResponse::Ok().json(checkout_session.url)
}

#[post("/user-app-settings")]
pub async fn update_user_app_settings(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<UserApplicationSettings>) -> impl Responder {
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

    let token = user.unwrap();
    let user = token.claims();
    let settings = info.into_inner();
    database.create_or_update_user_application_settings(&user.kerberos_username, &settings).await;
    println!("{:#?}", &settings);

    return HttpResponse::Ok().finish();
}

#[get("/user-app-settings")]
pub async fn get_user_app_settings(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
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

    let token = user.unwrap();
    let user = token.claims();
    let settings = match database.get_user_application_settings(&user.kerberos_username).await {
        Some(settings) => settings,
        None => UserApplicationSettings::default()
    };

    return HttpResponse::Ok()
        .json(settings);
}


#[post("/contact-request")]
pub async fn contact_request(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .json("None") //todo finish
}

#[get("/pricing")]
pub async fn pricing(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    let pricing = data.stripe_handler.get_tiered_prices();
    HttpResponse::Ok().json(pricing)
}

#[get("/payment/{status}")]  //todo: actually on second thought this should route back to the portal page or whatever
pub async fn payment_status(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .json("None")
}

#[get("/subscribe-user")]
pub async fn subscribe_user_events(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json("pricing")
}