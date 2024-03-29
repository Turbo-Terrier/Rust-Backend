use std::collections::HashMap;

use actix_web::{delete, get, HttpRequest, HttpResponse, post, Responder, web};
use actix_web::cookie::Cookie;
use actix_web::cookie::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stripe::CheckoutSession;

use crate::data_structs::app_config::UserApplicationSettings;
use crate::data_structs::bu_course::{BUCourseSection, CourseSection};
use crate::data_structs::responses::web_register_response::WebRegisterResponse;
use crate::data_structs::semester::Semester;
use crate::data_structs::user::User;
use crate::google_oauth::{GoogleAuthCode, GoogleClientSecret};
use crate::SharedResources;

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
    let jwt_user_token = jwt_secret.encrypt_jwt_token(user.kerberos_username.clone());

    HttpResponse::Ok().json(WebRegisterResponse {
        jwt_cookie: jwt_user_token.as_str().to_string(),
        user: user
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
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    return if kerberos_username.is_none() {
        HttpResponse::Unauthorized().json("Invalid")
    } else {
        let unwrapped_user = database.get_user(&kerberos_username.unwrap().claims()).await.unwrap();
        HttpResponse::Ok().json(unwrapped_user)
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
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let kerberos_username = kerberos_username.unwrap().claims().to_owned();
    database.reset_authentication_key(&kerberos_username).await;
    let user = database.get_user(&kerberos_username).await;

    return HttpResponse::Ok()
        .json(user);
}

#[derive(Deserialize)]
struct Quantity(u64);
#[post("/create-checkout-session")]
pub async fn create_checkout_session(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<Quantity>) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let stripe_handler = &data.get_ref().stripe_handler;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let mut quantity = info.into_inner().0;
    // make sure quantity is between 1-16 and if its not min/max it
    quantity = quantity.max(1).min(16);
    let user = database.get_user(&kerberos_username.unwrap().claims().to_owned()).await.unwrap();

    let checkout_session: CheckoutSession = stripe_handler.create_stripe_checkout_session(
        &data.get_ref().base_url,
        user.stripe_id.as_str().parse().unwrap(),
        quantity,
        stripe_handler.get_unit_price(quantity)
    ).await;

    database.create_purchase_session(
        &user.kerberos_username,
        quantity,
        stripe_handler.get_unit_price(quantity),
        checkout_session.id.as_str()
    ).await;

    HttpResponse::Ok().json(checkout_session.url)
}

#[post("/custom-course")]
pub async fn add_custom_course(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<BUCourseSection>) -> impl Responder {
    let database = &data.get_ref().database;
    let jwt_secret = &data.get_ref().jwt_secret;
    let course = info.into_inner();

    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let added_course = database.add_course(
        course.course.semester.clone(),
        course.course.to_full_course_code_str(),
        None, None, false, vec![
            CourseSection {
                section: course.section.section,
                ..CourseSection::default()
            }
        ]
    ).await;

    HttpResponse::Ok().json(&added_course[0])
}

#[post("/user-app-settings")]
pub async fn update_user_app_settings(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<HashMap<String, Value>>) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let token = kerberos_username.unwrap();
    let kerberos_username = token.claims();

    // todo: this middle processing you having to first fetch existing settings is very inefficient
    //  and instead the create_or_update_user_application_settings should be rewritten

    // get the current application settings
    let current_settings = match database.get_user_application_settings(&kerberos_username).await {
        Some(settings) => settings,
        None => UserApplicationSettings::default()
    };

    // Update fields dynamically based on the provided JSON
    let mut updated_settings_map = info.into_inner();
    let json_str = serde_json::to_string(&current_settings).unwrap();
    let mut field_map = serde_json::from_str::<HashMap<String, Value>>(json_str.as_str()).unwrap();
    for (field, value) in updated_settings_map {
        field_map.insert(field, value);
    }
    // convert back into settings
    let json_str = serde_json::to_string(&field_map).unwrap();
    let updated_settings = serde_json::from_str::<UserApplicationSettings>(json_str.as_str()).unwrap();

    database.create_or_update_user_application_settings(&kerberos_username, &updated_settings).await;

    return HttpResponse::Ok().finish();
}

#[derive(Deserialize)]
#[derive(Serialize)]
struct CourseReference {
    course_id: u32,
    section_id: String
}

#[delete("course-update")]
pub async fn del_course(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<CourseReference>) -> impl Responder {

    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    let info = info.into_inner();
    let course_id = info.course_id;
    let section_id = &info.section_id;

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let token = kerberos_username.unwrap();
    let kerberos_username = token.claims();

    // todo: return status?
    database.user_course_settings_delete_course(kerberos_username, course_id, section_id).await;

    return HttpResponse::Ok().finish();
}

#[post("course-update")]
pub async fn add_course(data: web::Data<SharedResources>, req: HttpRequest, info: web::Json<CourseReference>) -> impl Responder {

    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    let info = info.into_inner();
    let course_id = info.course_id;
    let section_id = &info.section_id;

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let token = kerberos_username.unwrap();
    let kerberos_username = token.claims();

    // todo: return status?
    database.user_course_settings_add_course(kerberos_username, course_id, section_id).await;

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
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let token = kerberos_username.unwrap();
    let kerberos_username = token.claims();
    let settings = match database.get_user_application_settings(&kerberos_username).await {
        Some(settings) => settings,
        None => UserApplicationSettings::default()
    };

    return HttpResponse::Ok()
        .json(settings);
}

#[get("/active-semesters")]
pub async fn get_active_semesters(data: web::Data<SharedResources>, req: HttpRequest) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let semesters = database.get_semesters_in_db().await;
    let active_semesters = Semester::get_current_and_upcoming_semesters();

    // find the intersection of both methods
    let semesters: Vec<Semester> = semesters.into_iter().filter(|semester| {
        active_semesters.contains(semester)
    }).collect();

    return HttpResponse::Ok()
        .json(semesters);
}

#[get("/get-available-courses")]
pub async fn get_available_courses(data: web::Data<SharedResources>, req: HttpRequest, info: web::Query<Semester>) -> impl Responder {
    let jwt_secret = &data.get_ref().jwt_secret;
    let database = &data.get_ref().database;
    let auth_header = req.headers().get("Authorization");

    if auth_header.is_none() {
        return HttpResponse::Unauthorized().json("No authorization key supplied");
    }

    let user_auth_str = auth_header.unwrap().to_str().unwrap();
    let kerberos_username = jwt_secret.decrypt_jwt_token::<String>(user_auth_str);

    if kerberos_username.is_none() {
        return HttpResponse::Unauthorized().json("Invalid");
    }

    let courses = database.get_courses(&info.into_inner()).await;

    HttpResponse::Ok()
        .json(courses)
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