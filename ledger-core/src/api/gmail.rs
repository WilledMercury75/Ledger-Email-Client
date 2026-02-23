use actix_web::{web, HttpResponse, get, post};
use crate::models::message::*;
use crate::gmail::{smtp_client, imap_client};

use super::super::AppState;

#[get("/api/gmail/config")]
pub async fn get_gmail_config(state: web::Data<AppState>) -> HttpResponse {
    let email = state.db.get_setting("gmail_email").ok().flatten();
    let configured = email.is_some();

    HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "configured": configured,
        "email": email,
    })))
}

#[post("/api/gmail/config")]
pub async fn set_gmail_config(
    state: web::Data<AppState>,
    body: web::Json<GmailConfig>,
) -> HttpResponse {
    if let Err(e) = state.db.set_setting("gmail_email", &body.email) {
        return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string()));
    }
    if let Err(e) = state.db.set_setting("gmail_app_password", &body.app_password) {
        return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string()));
    }
    if let Some(ref host) = body.imap_host {
        let _ = state.db.set_setting("gmail_imap_host", host);
    }
    if let Some(ref host) = body.smtp_host {
        let _ = state.db.set_setting("gmail_smtp_host", host);
    }

    HttpResponse::Ok().json(ApiResponse::ok("Gmail configured"))
}

#[post("/api/gmail/fetch")]
pub async fn fetch_gmail(state: web::Data<AppState>) -> HttpResponse {
    let email = match state.db.get_setting("gmail_email") {
        Ok(Some(e)) => e,
        _ => return HttpResponse::BadRequest().json(ApiResponse::<()>::err("Gmail not configured")),
    };
    let app_password = match state.db.get_setting("gmail_app_password") {
        Ok(Some(p)) => p,
        _ => return HttpResponse::BadRequest().json(ApiResponse::<()>::err("Gmail not configured")),
    };

    let config = GmailConfig {
        email,
        app_password,
        imap_host: state.db.get_setting("gmail_imap_host").ok().flatten(),
        smtp_host: None,
    };

    // Run IMAP fetch in a blocking task (it uses synchronous I/O)
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        imap_client::fetch_messages(&config, 20)
    }).await;

    match result {
        Ok(Ok(messages)) => {
            let count = messages.len();
            for msg in &messages {
                if let Err(e) = db.insert_message(msg) {
                    tracing::error!("Failed to store Gmail message: {}", e);
                }
            }
            HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
                "fetched": count,
                "messages": messages,
            })))
        }
        Ok(Err(e)) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}

#[post("/api/gmail/send")]
pub async fn send_gmail(
    state: web::Data<AppState>,
    body: web::Json<GmailSendRequest>,
) -> HttpResponse {
    let email = match state.db.get_setting("gmail_email") {
        Ok(Some(e)) => e,
        _ => return HttpResponse::BadRequest().json(ApiResponse::<()>::err("Gmail not configured")),
    };
    let app_password = match state.db.get_setting("gmail_app_password") {
        Ok(Some(p)) => p,
        _ => return HttpResponse::BadRequest().json(ApiResponse::<()>::err("Gmail not configured")),
    };

    let config = GmailConfig {
        email: email.clone(),
        app_password,
        imap_host: None,
        smtp_host: state.db.get_setting("gmail_smtp_host").ok().flatten(),
    };

    match smtp_client::send_email(&config, &body.to, &body.subject, &body.body).await {
        Ok(()) => {
            // Store in sent folder
            let msg = Message::new(
                email,
                body.to.clone(),
                body.subject.clone(),
                body.body.clone(),
            );
            let mut msg = msg;
            msg.folder = Folder::Sent;
            msg.delivery_method = DeliveryMethod::Gmail;
            let _ = state.db.insert_message(&msg);

            HttpResponse::Ok().json(ApiResponse::ok("Email sent"))
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}
