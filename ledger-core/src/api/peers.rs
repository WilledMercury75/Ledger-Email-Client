use actix_web::{web, HttpResponse, get, post};
use crate::models::message::*;
use crate::p2p::node::P2PCommand;
use tokio::sync::mpsc;

use super::super::AppState;

#[get("/api/peers")]
pub async fn list_peers(state: web::Data<AppState>) -> HttpResponse {
    let (tx, mut rx) = mpsc::channel(1);
    let _ = state.p2p_tx.send(P2PCommand::GetPeers { response_tx: tx }).await;

    match rx.recv().await {
        Some(peers) => HttpResponse::Ok().json(ApiResponse::ok(peers)),
        None => HttpResponse::InternalServerError().json(ApiResponse::<()>::err("Failed to get peers")),
    }
}

#[post("/api/peers")]
pub async fn connect_peer(
    state: web::Data<AppState>,
    body: web::Json<ConnectPeerRequest>,
) -> HttpResponse {
    let addr = match body.multiaddr.parse() {
        Ok(a) => a,
        Err(e) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()>::err(format!("Invalid multiaddr: {}", e)));
        }
    };

    let (tx, mut rx) = mpsc::channel(1);
    let _ = state.p2p_tx.send(P2PCommand::ConnectPeer {
        addr,
        response_tx: tx,
    }).await;

    match rx.recv().await {
        Some(Ok(peer_id)) => {
            HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
                "peer_id": peer_id.to_string(),
                "status": "connecting"
            })))
        }
        Some(Err(e)) => HttpResponse::InternalServerError().json(ApiResponse::<()>::err(e)),
        None => HttpResponse::InternalServerError().json(ApiResponse::<()>::err("No response")),
    }
}
