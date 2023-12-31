use std::fmt::Debug;
use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use crate::data_structs::app_start_request::{ApplicationStart, ApplicationStopped, EmailSendRequest, RegistrationNotification, SessionPing};
use crate::data_structs::signed_response::SignedApplicationStartPermission;
use crate::{SharedResources, utils};

#[get("/ping")]
async fn debug_ping(req_body: String) -> impl Responder {
    // just to test that the server is running
    "pong!"
}

//todo: authenticate requests to make sure not just anyone can spam the api
//  1. maybe the make an account online.
//  2. get a free token
//  3. if they buy premium that same token becomes premium

#[post("/app-started")]
pub async fn app_start(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<ApplicationStart>) -> impl Responder {
    let start_data: ApplicationStart = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &start_data.credentials.kerberos_username,
        &start_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    let grant = data.database
        .get_user_grant(&start_data.credentials.kerberos_username).await;

    let session_id = data.database.create_session(&start_data, &grant).await;

    let mut to_sign_vec: Vec<&str> = Vec::new();
    to_sign_vec.push(start_data.credentials.kerberos_username.as_str());
    to_sign_vec.push(grant.as_str());

    let response = SignedApplicationStartPermission::new(
        start_data.credentials.kerberos_username,
        grant,
        session_id,
        chrono::Local::now().timestamp(),
        "TODO".to_string()
    );

    // todo respond with a json object containing the signed response
    HttpResponse::Ok().json(response)
}

#[post("/app-stopped")]
async fn app_stop(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<ApplicationStopped>) -> impl Responder {
    let stop_data: ApplicationStopped = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &stop_data.credentials.kerberos_username,
        &stop_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    return match data.database.end_session(&stop_data).await {
        Ok(_) => HttpResponse::Ok().json("OK"),
        Err(_) => HttpResponse::BadRequest().json("Invalid session id")
    }; //TODO: return a signed response
}

#[post("/ping")]
async fn ping(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<SessionPing>) -> impl Responder {
    let ping_data: SessionPing = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &ping_data.credentials.kerberos_username,
        &ping_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    return match data.database.session_ping(ping_data.session_id).await {
        Ok(_) => HttpResponse::Ok().json("OK"),
        Err(_) => HttpResponse::BadRequest().json("Invalid session id")
    }; //TODO: return a signed response
}

#[post("/course-registered")]
async fn course_registered(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<RegistrationNotification>) -> impl Responder {
    let reg_notif_data: RegistrationNotification = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &reg_notif_data.credentials.kerberos_username,
        &reg_notif_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    return match data.database.mark_course_registered(
        reg_notif_data.session_id,
        reg_notif_data.timestamp,
        reg_notif_data.course).await {
        Ok(_) => HttpResponse::Ok().json("OK"),
        Err(_) => HttpResponse::BadRequest().json("Invalid session id")
    }; //TODO: return a signed response
}

#[post("/send-mail")]
async fn send_mail(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<EmailSendRequest>) -> impl Responder {
    let email_send_data: EmailSendRequest = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &email_send_data.credentials.kerberos_username,
        &email_send_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    HttpResponse::Ok().json("TODO") //todo
}


