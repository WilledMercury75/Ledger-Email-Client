use tokio::sync::mpsc;

use crate::p2p::node::P2PCommand;
use crate::models::message::EncryptedEnvelope;

/// Store an encrypted envelope in the DHT for offline retrieval
pub async fn store_in_dht(
    p2p_tx: &mpsc::Sender<P2PCommand>,
    recipient_ledger_id: &str,
    envelope: &EncryptedEnvelope,
) -> Result<(), String> {
    let key = format!("ledger:msg:{}", recipient_ledger_id);
    let value = serde_json::to_vec(envelope).map_err(|e| format!("Serialize error: {}", e))?;

    let (tx, mut rx) = mpsc::channel(1);
    p2p_tx.send(P2PCommand::DhtPut {
        key: key.into_bytes(),
        value,
        response_tx: tx,
    }).await.map_err(|e| format!("Channel send error: {}", e))?;

    rx.recv().await
        .ok_or_else(|| "No response from DHT put".to_string())?
}

/// Retrieve pending messages from the DHT for the local identity
pub async fn retrieve_from_dht(
    p2p_tx: &mpsc::Sender<P2PCommand>,
    own_ledger_id: &str,
) -> Result<Option<Vec<EncryptedEnvelope>>, String> {
    let key = format!("ledger:msg:{}", own_ledger_id);

    let (tx, mut rx) = mpsc::channel(1);
    p2p_tx.send(P2PCommand::DhtGet {
        key: key.into_bytes(),
        response_tx: tx,
    }).await.map_err(|e| format!("Channel send error: {}", e))?;

    let result = rx.recv().await
        .ok_or_else(|| "No response from DHT get".to_string())?;

    match result {
        Ok(Some(data)) => {
            let envelope: EncryptedEnvelope = serde_json::from_slice(&data)
                .map_err(|e| format!("Deserialize error: {}", e))?;
            Ok(Some(vec![envelope]))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}
