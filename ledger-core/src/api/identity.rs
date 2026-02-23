use actix_web::{web, HttpResponse, get};
use crate::models::message::{ApiResponse, IdentityInfo};

use super::super::AppState;

#[get("/api/identity")]
pub async fn get_identity(state: web::Data<AppState>) -> HttpResponse {
    let info = IdentityInfo {
        ledger_id: state.identity.ledger_id.clone(),
        public_key: bs58::encode(state.identity.public_key_bytes()).into_string(),
        peer_id: state.peer_id.to_string(),
    };
    HttpResponse::Ok().json(ApiResponse::ok(info))
}
