use actix_web::{web, HttpResponse, get, put};
use crate::models::message::*;

use super::super::AppState;

#[get("/api/settings")]
pub async fn get_settings(state: web::Data<AppState>) -> HttpResponse {
    match state.db.get_all_settings() {
        Ok(settings) => HttpResponse::Ok().json(ApiResponse::ok(settings)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}

#[put("/api/settings")]
pub async fn update_settings(
    state: web::Data<AppState>,
    body: web::Json<Settings>,
) -> HttpResponse {
    if let Some(ref mode) = body.delivery_mode {
        if let Err(e) = state.db.set_setting("delivery_mode", mode) {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string()));
        }
    }
    if let Some(tor) = body.tor_enabled {
        if let Err(e) = state.db.set_setting("tor_enabled", &tor.to_string()) {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string()));
        }
    }
    if let Some(ttl) = body.dht_ttl_hours {
        if let Err(e) = state.db.set_setting("dht_ttl_hours", &ttl.to_string()) {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string()));
        }
    }

    match state.db.get_all_settings() {
        Ok(settings) => HttpResponse::Ok().json(ApiResponse::ok(settings)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}

/// Contacts API (bonus — needed for P2P to work)
#[get("/api/contacts")]
pub async fn list_contacts(state: web::Data<AppState>) -> HttpResponse {
    match state.db.get_contacts() {
        Ok(contacts) => HttpResponse::Ok().json(ApiResponse::ok(contacts)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}

#[actix_web::post("/api/contacts")]
pub async fn add_contact(
    state: web::Data<AppState>,
    body: web::Json<Contact>,
) -> HttpResponse {
    match state.db.upsert_contact(&body) {
        Ok(()) => HttpResponse::Ok().json(ApiResponse::ok("Contact added")),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e.to_string())),
    }
}
