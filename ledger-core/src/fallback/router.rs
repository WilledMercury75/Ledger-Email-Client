use tokio::sync::mpsc;

use crate::crypto::envelope::encrypt_message;
use crate::crypto::keys::LedgerIdentity;
use crate::dht;
use crate::gmail::smtp_client;
use crate::models::message::*;
use crate::p2p::node::P2PCommand;
use crate::store::db::Database;

/// Delivery result indicating which method was used
pub enum DeliveryResult {
    P2pDirect,
    DhtStored,
    GmailFallback,
    GmailDirect,
    Failed(String),
}

/// Route a message based on delivery mode settings
pub async fn route_message(
    identity: &LedgerIdentity,
    db: &Database,
    p2p_tx: &mpsc::Sender<P2PCommand>,
    to: &str,
    subject: &str,
    body: &str,
    mode: &str,
) -> DeliveryResult {
    let is_ledger_id = to.starts_with("ledger:");

    match mode {
        "p2p_only" => {
            if !is_ledger_id {
                return DeliveryResult::Failed("P2P mode requires a Ledger ID recipient".into());
            }
            try_p2p_delivery(identity, db, p2p_tx, to, subject, body).await
        }
        "gmail_only" => {
            try_gmail_delivery(identity, db, to, subject, body, false).await
        }
        "auto" | _ => {
            if is_ledger_id {
                // Try P2P first
                match try_p2p_delivery(identity, db, p2p_tx, to, subject, body).await {
                    DeliveryResult::P2pDirect => DeliveryResult::P2pDirect,
                    _ => {
                        // P2P failed, try DHT storage
                        tracing::info!("P2P delivery failed, trying DHT storage");
                        let dht_result = try_dht_delivery(identity, db, p2p_tx, to, subject, body).await;

                        // Also try Gmail fallback if configured
                        let gmail_result = try_gmail_delivery(identity, db, to, subject, body, true).await;

                        match gmail_result {
                            DeliveryResult::GmailFallback => DeliveryResult::GmailFallback,
                            _ => match dht_result {
                                DeliveryResult::DhtStored => DeliveryResult::DhtStored,
                                _ => DeliveryResult::Failed("All delivery methods failed".into()),
                            }
                        }
                    }
                }
            } else {
                // Regular email address — send via Gmail
                try_gmail_delivery(identity, db, to, subject, body, false).await
            }
        }
    }
}

/// Try P2P direct delivery
async fn try_p2p_delivery(
    identity: &LedgerIdentity,
    db: &Database,
    p2p_tx: &mpsc::Sender<P2PCommand>,
    to: &str,
    subject: &str,
    body: &str,
) -> DeliveryResult {
    // Look up recipient's encryption public key from contacts
    let contact = match db.get_contact(to) {
        Ok(Some(c)) => c,
        _ => {
            tracing::warn!("No contact found for {}", to);
            return DeliveryResult::Failed("Recipient not in contacts".into());
        }
    };

    // Decode recipient's encryption public key
    let recipient_enc_pubkey = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &contact.public_key,
    ) {
        Ok(bytes) => bytes,
        Err(e) => {
            return DeliveryResult::Failed(format!("Invalid contact public key: {}", e));
        }
    };

    // Encrypt the message
    let mut envelope = match encrypt_message(identity, &recipient_enc_pubkey, subject, body) {
        Ok(env) => env,
        Err(e) => {
            return DeliveryResult::Failed(format!("Encryption failed: {}", e));
        }
    };
    envelope.to_ledger_id = to.to_string();

    let envelope_json = match serde_json::to_string(&envelope) {
        Ok(j) => j,
        Err(e) => {
            return DeliveryResult::Failed(format!("Serialization failed: {}", e));
        }
    };

    // Try to find a connected peer for this Ledger ID
    // For prototype: we try sending to all connected peers
    let (tx, mut rx) = mpsc::channel(1);
    let _ = p2p_tx.send(P2PCommand::GetPeers { response_tx: tx }).await;

    if let Some(peers) = rx.recv().await {
        if peers.is_empty() {
            return DeliveryResult::Failed("No connected peers".into());
        }

        // Try the first connected peer (in production, we'd route to the right one)
        let peer_id_str = &peers[0].peer_id;
        if let Ok(peer_id) = peer_id_str.parse::<libp2p::PeerId>() {
            let (resp_tx, mut resp_rx) = mpsc::channel(1);
            let _ = p2p_tx.send(P2PCommand::SendMessage {
                peer_id,
                envelope_json,
                response_tx: resp_tx,
            }).await;

            if let Some(Ok(())) = resp_rx.recv().await {
                return DeliveryResult::P2pDirect;
            }
        }
    }

    DeliveryResult::Failed("P2P delivery failed".into())
}

/// Try DHT offline storage
async fn try_dht_delivery(
    identity: &LedgerIdentity,
    db: &Database,
    p2p_tx: &mpsc::Sender<P2PCommand>,
    to: &str,
    subject: &str,
    body: &str,
) -> DeliveryResult {
    let contact = match db.get_contact(to) {
        Ok(Some(c)) => c,
        _ => return DeliveryResult::Failed("No contact for DHT delivery".into()),
    };

    let recipient_enc_pubkey = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &contact.public_key,
    ) {
        Ok(bytes) => bytes,
        Err(_) => return DeliveryResult::Failed("Invalid contact public key".into()),
    };

    let mut envelope = match encrypt_message(identity, &recipient_enc_pubkey, subject, body) {
        Ok(env) => env,
        Err(e) => return DeliveryResult::Failed(format!("Encryption failed: {}", e)),
    };
    envelope.to_ledger_id = to.to_string();

    match dht::store::store_in_dht(p2p_tx, to, &envelope).await {
        Ok(()) => DeliveryResult::DhtStored,
        Err(e) => DeliveryResult::Failed(format!("DHT storage failed: {}", e)),
    }
}

/// Try Gmail delivery (direct or fallback)
async fn try_gmail_delivery(
    identity: &LedgerIdentity,
    db: &Database,
    to: &str,
    subject: &str,
    body: &str,
    encrypted_fallback: bool,
) -> DeliveryResult {
    // Get Gmail config
    let email = match db.get_setting("gmail_email") {
        Ok(Some(e)) => e,
        _ => return DeliveryResult::Failed("Gmail not configured".into()),
    };
    let app_password = match db.get_setting("gmail_app_password") {
        Ok(Some(p)) => p,
        _ => return DeliveryResult::Failed("Gmail not configured".into()),
    };

    let config = GmailConfig {
        email,
        app_password,
        imap_host: None,
        smtp_host: None,
    };

    // Determine recipient email
    let recipient_email = if to.starts_with("ledger:") {
        // Look up their Gmail address from contacts
        match db.get_contact(to) {
            Ok(Some(c)) => match c.gmail_address {
                Some(addr) => addr,
                None => return DeliveryResult::Failed("No Gmail address for Ledger contact".into()),
            },
            _ => return DeliveryResult::Failed("Contact not found".into()),
        }
    } else {
        to.to_string()
    };

    if encrypted_fallback {
        // Send encrypted payload as fallback
        let payload = serde_json::json!({
            "from": identity.ledger_id,
            "subject": subject,
            "body": body,
            "timestamp": chrono::Utc::now().timestamp(),
        });
        let payload_str = serde_json::to_string(&payload).unwrap_or_default();

        match smtp_client::send_encrypted_fallback(&config, &recipient_email, &payload_str).await {
            Ok(()) => DeliveryResult::GmailFallback,
            Err(e) => DeliveryResult::Failed(format!("Gmail fallback failed: {}", e)),
        }
    } else {
        match smtp_client::send_email(&config, &recipient_email, subject, body).await {
            Ok(()) => DeliveryResult::GmailDirect,
            Err(e) => DeliveryResult::Failed(format!("Gmail send failed: {}", e)),
        }
    }
}
