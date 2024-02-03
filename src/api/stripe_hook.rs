use actix_web::{HttpRequest, HttpResponse, post, web};
use stripe::{CheckoutSession, CheckoutSessionPaymentStatus, EventObject, EventType, Webhook};

use crate::SharedResources;

#[post("webhook")]
pub async fn webhook_handler(data: web::Data<SharedResources>, req: HttpRequest, payload: web::Bytes) -> HttpResponse {
    let signing_secret = data.get_ref().stripe_handler.get_webhook_signing_secret();
    let payload_str = std::str::from_utf8(payload.as_ref()).unwrap();
    let stripe_signature = get_header_value(&req, "Stripe-Signature").unwrap_or_default();

    if let Ok(event) = Webhook::construct_event(payload_str, stripe_signature, signing_secret.as_str()) {
        match event.type_ {
            EventType::CheckoutSessionCompleted => {
                if let EventObject::CheckoutSession(session) = event.data.object {
                    handle_checkout_complete(data, session).await;
                    return HttpResponse::Ok().finish();
                }
                return HttpResponse::BadRequest().finish();
            }
            EventType::CheckoutSessionExpired => {
                if let EventObject::CheckoutSession(session) = event.data.object {
                    handle_checkout_expired(data, session).await;
                    return HttpResponse::Ok().finish();
                }
                return HttpResponse::BadRequest().finish();
            }
            EventType::CheckoutSessionAsyncPaymentSucceeded => {
                if let EventObject::CheckoutSession(session) = event.data.object {
                    handle_checkout_complete(data, session).await;
                    return HttpResponse::Ok().finish();
                }
                return HttpResponse::BadRequest().finish();
            }
            EventType::CheckoutSessionAsyncPaymentFailed => {
                if let EventObject::CheckoutSession(session) = event.data.object {
                    handle_checkout_expired(data, session).await;
                    return HttpResponse::Ok().finish();
                }
                return HttpResponse::BadRequest().finish();
            }
            _ => {
                println!("Unknown event encountered in webhook: {:?}", event.type_);
                HttpResponse::InternalServerError().finish()
            }
        }
    } else {
        println!("Failed to construct webhook event, ensure your webhook secret is correct.");
        HttpResponse::Unauthorized().finish()
    }
}

fn get_header_value<'b>(req: &'b HttpRequest, key: &'b str) -> Option<&'b str> {
    req.headers().get(key)?.to_str().ok()
}

async fn handle_checkout_complete(data: web::Data<SharedResources>, session: CheckoutSession) {
    println!("Checkout session completed/paid: {:?}", session);
    let database = &data.database;

    if session.payment_status == CheckoutSessionPaymentStatus::Paid {
        let success = database.close_purchase_session(session.id.as_str(), true, session.amount_total.map(|o| o as f64/100.0), None).await;
        if !success {
            eprintln!("Error, no such session id!");
        }
    }

}

async fn handle_checkout_expired(data: web::Data<SharedResources>, session: CheckoutSession) {
    println!("Checkout session expired/failed: {:?}", session);
    let database = &data.database;
    let success = database.close_purchase_session(session.id.as_str(), false, None, None).await;
    if !success {
        eprintln!("Error, no such session id!");
    }
}