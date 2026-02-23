use actix_web::{web, HttpResponse, get, post, delete};
use crate::models::message::*;
use crate::fallback::router;

use super::super::AppState;

#[get("/api/messages")]
pub async fn list_messages(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let folder = query.get("folder").map(|s| s.as_str());
    match state.db.get_messages(folder) {
        Ok(messages) => HttpResponse::Ok().json(ApiResponse::ok(messages)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}

#[get("/api/messages/{id}")]
pub async fn get_message(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    match state.db.get_message(&id) {
        Ok(Some(msg)) => {
            let _ = state.db.mark_read(&id);
            HttpResponse::Ok().json(ApiResponse::ok(msg))
        }
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()>::err("Message not found")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}

#[post("/api/messages")]
pub async fn send_message(
    state: web::Data<AppState>,
    body: web::Json<SendMessageRequest>,
) -> HttpResponse {
    let mode = body.mode.as_deref().unwrap_or("auto");

    // Route through fallback logic
    let result = router::route_message(
        &state.identity,
        &state.db,
        &state.p2p_tx,
        &body.to,
        &body.subject,
        &body.body,
        mode,
    ).await;

    let (delivery_method, success) = match result {
        router::DeliveryResult::P2pDirect => (DeliveryMethod::P2p, true),
        router::DeliveryResult::DhtStored => (DeliveryMethod::P2p, true),
        router::DeliveryResult::GmailFallback => (DeliveryMethod::Fallback, true),
        router::DeliveryResult::GmailDirect => (DeliveryMethod::Gmail, true),
        router::DeliveryResult::Failed(ref e) => {
            tracing::warn!("Delivery failed: {}", e);
            (DeliveryMethod::P2p, false)
        }
    };

    if !success {
        if let router::DeliveryResult::Failed(e) = result {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e));
        }
    }

    // Store in sent folder
    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from_id: state.identity.ledger_id.clone(),
        to_id: body.to.clone(),
        subject: body.subject.clone(),
        body: body.body.clone(),
        timestamp: chrono::Utc::now().timestamp(),
        delivery_method,
        is_read: true,
        folder: Folder::Sent,
        signature: None,
        encrypted: body.to.starts_with("ledger:"),
    };

    if let Err(e) = state.db.insert_message(&msg) {
        tracing::error!("Failed to store sent message: {}", e);
    }

    HttpResponse::Ok().json(ApiResponse::ok(msg))
}

#[delete("/api/messages/{id}")]
pub async fn delete_message(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    match state.db.delete_message(&id) {
        Ok(true) => HttpResponse::Ok().json(ApiResponse::ok("Deleted")),
        Ok(false) => HttpResponse::NotFound().json(ApiResponse::<()>::err("Message not found")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}
