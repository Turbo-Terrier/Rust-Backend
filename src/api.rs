use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use crate::data_structs::app_start_request::{ApplicationStart, ApplicationStopped, EmailSendRequest, RegistrationNotification, SessionPing};
use crate::data_structs::signed_response::{ApplicationStartPermission, GrantLevel, SignedApplicationStartPermission, SignedStatusResponse, StatusResponse};
use crate::SharedResources;

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

    // check if there is another running session first
    let active_session = data.database.has_active_session(&start_data.credentials.kerberos_username).await;
    if active_session.is_some() {
        let device = active_session.unwrap();
        let message = format!("You already have an active session running on device {:?}", device);
        return HttpResponse::BadRequest().json(message);
    }

    let grant = data.database
        .get_user_grant(&start_data.credentials.kerberos_username).await;

    let session_id = data.database.create_session(&start_data, &grant).await;

    let mut to_sign_vec: Vec<&str> = Vec::new();
    to_sign_vec.push(start_data.credentials.kerberos_username.as_str());
    to_sign_vec.push(grant.as_str());

    let response = ApplicationStartPermission::new(
        start_data.credentials.kerberos_username,
        grant,
        session_id,
        chrono::Local::now().timestamp()
    );

    let signed_str = data.private_key.sign(&response);

    HttpResponse::Ok().json(SignedApplicationStartPermission {
        data: response,
        signature: signed_str
    })
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
        Ok(_) => {
            let response = StatusResponse::new(
                stop_data.credentials.kerberos_username,
                "OK".to_string(),
                chrono::Local::now().timestamp()
            );
            let signed_str = data.private_key.sign(&response);
            HttpResponse::Ok().json(SignedStatusResponse {
                data: response,
                signature: signed_str
            })
        },
        Err(_) => HttpResponse::BadRequest().json("Invalid session id")
    };
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
        Ok(_) => {
            let response = StatusResponse::new(
                ping_data.credentials.kerberos_username,
                "OK".to_string(),
                chrono::Local::now().timestamp()
            );
            let signed_str = data.private_key.sign(&response);
            HttpResponse::Ok().json(SignedStatusResponse {
                data: response,
                signature: signed_str
            })
        },
        Err(_) => HttpResponse::BadRequest().json("Invalid session id")
    };
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
        Ok(_) => {
            // first if this is a demo user, mark demo over
            let grant = data.database.get_user_grant(&reg_notif_data.credentials.kerberos_username).await;
            if grant == GrantLevel::Demo {
                data.database.mark_demo_over(&reg_notif_data.credentials.kerberos_username).await;
                // todo: email them about discounted premium
            }

            let response = StatusResponse::new(
                reg_notif_data.credentials.kerberos_username,
                "OK".to_string(),
                chrono::Local::now().timestamp()
            );
            let signed_str = data.private_key.sign(&response);
            HttpResponse::Ok().json(SignedStatusResponse {
                data: response,
                signature: signed_str
            })
        },
        Err(_) => HttpResponse::BadRequest().json("Invalid session id")
    };
}

#[post("/send-mail")]  //todo: remove?
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
