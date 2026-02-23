use serde::{Deserialize, Serialize};

/// Protocol name for Ledger message exchange
pub const PROTOCOL_NAME: libp2p::StreamProtocol = libp2p::StreamProtocol::new("/ledger/msg/1.0.0");

/// Request sent from one Ledger peer to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerRequest {
    pub envelope_json: String,
}

/// Response after receiving a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerResponse {
    pub accepted: bool,
    pub error: Option<String>,
}
