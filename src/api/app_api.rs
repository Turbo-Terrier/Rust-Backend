use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use crate::data_structs::device_meta::DeviceMeta;
use crate::data_structs::grant_level::GrantLevel;
use crate::data_structs::requests::application_start::ApplicationStart;
use crate::data_structs::requests::application_stopped::ApplicationStopped;
use crate::data_structs::requests::email_send_request::EmailSendRequest;
use crate::data_structs::requests::registration_notification::RegistrationNotification;
use crate::data_structs::requests::session_ping::SessionPing;
use crate::data_structs::responses::app_start_permission::{ApplicationStartPermission, SignedApplicationStartPermission};
use crate::data_structs::responses::status_response::{SignedStatusResponse, StatusResponse};
use crate::SharedResources;

#[get("/ping")]
async fn debug_ping() -> impl Responder {
    // just to test that the server is running
    "Pong!"
}

//todo: authenticate requests to make sure not just anyone can spam the api
//  1. maybe the make an account online.
//  2. get a free token
//  3. if they buy premium that same token becomes premium

#[post("/app-started")]
pub async fn app_start(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Json<ApplicationStart>) -> impl Responder {
    let mut start_data: ApplicationStart = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &start_data.credentials.kerberos_username,
        &start_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized"); //todo, fix this
    }

    // add client ip to the request
    start_data.device_meta.ip = Option::from(req.connection_info().realip_remote_addr().unwrap().to_string()); //todo test

    // check if there is another running session first
    let active_session = data.database.has_active_session(&start_data.credentials.kerberos_username).await;
    if active_session.is_some() {
        let device: DeviceMeta = active_session.unwrap();
        let mut message = format!("You already have an active session running on your {} device", device.os).to_string();
        let to_append = if device.ip.is_some() {
            format!(" with ip {}.", device.ip.unwrap())
        } else {
            ".".to_string()
        };
        message.push_str(to_append.as_str());
        message.push_str(" If you believe this is an error, please wait a few seconds and try \
                                 again. Otherwise, please contact us for support.");
        return HttpResponse::BadRequest().json(message);  //todo, fix this
    }

    let grant = data.database
        .get_user_grant(&start_data.credentials.kerberos_username).await;

    let session_id = data.database.create_session(&start_data, &grant).await;

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
async fn app_stop(data: web::Data<SharedResources>, payload: web::Json<ApplicationStopped>) -> impl Responder {
    let stop_data: ApplicationStopped = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &stop_data.credentials.kerberos_username,
        &stop_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized"); //todo, fix this
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
        Err(_) => HttpResponse::BadRequest().json("Invalid session id") //todo, fix this
    };
}

#[post("/ping")]
async fn ping(data: web::Data<SharedResources>, payload: web::Json<SessionPing>) -> impl Responder {
    let ping_data: SessionPing = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &ping_data.credentials.kerberos_username,
        &ping_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    return match data.database.session_ping(&ping_data).await {
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
        Err(_) => HttpResponse::BadRequest().json("Invalid session id") //todo, fix this
    };
}

#[post("/course-registered")]
async fn course_registered(data: web::Data<SharedResources>, payload: web::Json<RegistrationNotification>) -> impl Responder {
    let reg_notif_data: RegistrationNotification = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &reg_notif_data.credentials.kerberos_username,
        &reg_notif_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized"); //todo, fix this
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
        Err(_) => HttpResponse::BadRequest().json("Invalid session id") //todo, fix this
    };
}

#[post("/send-mail")]  //todo: remove?
async fn send_mail(data: web::Data<SharedResources>, payload: web::Json<EmailSendRequest>) -> impl Responder {
    let email_send_data: EmailSendRequest = payload.into_inner();

    let is_authenticated = data.database.is_authenticated(
        &email_send_data.credentials.kerberos_username,
        &email_send_data.credentials.authentication_key
    ).await;

    if !is_authenticated {
        return HttpResponse::Unauthorized().json("Unauthorized");
    }

    HttpResponse::Ok().json("blah")
}
