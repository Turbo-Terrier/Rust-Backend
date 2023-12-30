use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use crate::data_structs::app_start_request::{ApplicationStart, ApplicationStopped, RegistrationNotification};
use crate::data_structs::signed_response::SignedApplicationStartPermission;
use crate::SharedResources;

#[get("/ping")]
async fn debug_ping(req_body: String) -> impl Responder {
    // just to test that the server is running
    "pong!"
}

#[post("/app-started")]
pub async fn app_start(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<ApplicationStart>) -> impl Responder {

    let start_data: ApplicationStart = payload.into_inner();
    let grant = data.database
        .get_user_grant(&start_data.credentials.kerberos_username).await;

    let session_id = data.database.create_session(&start_data, &grant).await;

    let response = SignedApplicationStartPermission::new(
        start_data.credentials.kerberos_username,
        grant,
        session_id,
        start_data.response_timestamp,
        String::from("signature")
    );

    // respond with a json object containing the signed response
    HttpResponse::Ok().json(response)
}

#[post("/app-stopped")]
async fn app_stop(req: HttpRequest, payload: web::Json<ApplicationStopped>) -> impl Responder {
    format!("Hello {}!", 1)
}

#[post("/ping")]
async fn ping(req: HttpRequest, payload: web::Payload) -> impl Responder {
    format!("Hello {}!", 1)
}

#[post("/course-registered")]
async fn course_registered(req: HttpRequest, payload: web::Json<RegistrationNotification>) -> impl Responder {
    format!("Hello {}!", 1)
}

#[post("/send-mail")]
async fn send_mail(req: HttpRequest, payload: web::Payload) -> impl Responder {
    format!("Hello {}!", 1)
}


