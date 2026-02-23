use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Delivery method for a message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMethod {
    P2p,
    Gmail,
    Fallback,
}

impl std::fmt::Display for DeliveryMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryMethod::P2p => write!(f, "p2p"),
            DeliveryMethod::Gmail => write!(f, "gmail"),
            DeliveryMethod::Fallback => write!(f, "fallback"),
        }
    }
}

impl DeliveryMethod {
    pub fn from_str(s: &str) -> Self {
        match s {
            "p2p" => DeliveryMethod::P2p,
            "gmail" => DeliveryMethod::Gmail,
            "fallback" => DeliveryMethod::Fallback,
            _ => DeliveryMethod::P2p,
        }
    }
}

/// Message folder
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Folder {
    Inbox,
    Sent,
    Drafts,
}

impl std::fmt::Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Folder::Inbox => write!(f, "inbox"),
            Folder::Sent => write!(f, "sent"),
            Folder::Drafts => write!(f, "drafts"),
        }
    }
}

impl Folder {
    pub fn from_str(s: &str) -> Self {
        match s {
            "inbox" => Folder::Inbox,
            "sent" => Folder::Sent,
            "drafts" => Folder::Drafts,
            _ => Folder::Inbox,
        }
    }
}

/// Delivery mode preference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    P2pOnly,
    GmailOnly,
    Auto,
}

/// A Ledger message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub subject: String,
    pub body: String,
    pub timestamp: i64,
    pub delivery_method: DeliveryMethod,
    pub is_read: bool,
    pub folder: Folder,
    pub signature: Option<String>,
    pub encrypted: bool,
}

impl Message {
    pub fn new(from_id: String, to_id: String, subject: String, body: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_id,
            to_id,
            subject,
            body,
            timestamp: chrono::Utc::now().timestamp(),
            delivery_method: DeliveryMethod::P2p,
            is_read: false,
            folder: Folder::Inbox,
            signature: None,
            encrypted: false,
        }
    }
}

/// Request to send a message
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub mode: Option<String>, // "p2p_only", "gmail_only", "auto"
}

/// Request to connect to a peer
#[derive(Debug, Deserialize)]
pub struct ConnectPeerRequest {
    pub multiaddr: String,
}

/// Gmail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailConfig {
    pub email: String,
    pub app_password: String,
    pub imap_host: Option<String>,
    pub smtp_host: Option<String>,
}

/// Request to send Gmail
#[derive(Debug, Deserialize)]
pub struct GmailSendRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Settings update request
#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub delivery_mode: Option<String>,
    pub tor_enabled: Option<bool>,
    pub dht_ttl_hours: Option<u64>,
}

/// Peer info
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub ledger_id: Option<String>,
}

/// Identity info
#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityInfo {
    pub ledger_id: String,
    pub public_key: String,
    pub peer_id: String,
}

/// Encrypted envelope for P2P transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    pub id: String,
    pub from_ledger_id: String,
    pub to_ledger_id: String,
    pub ephemeral_pubkey: String,
    pub encrypted_body: String,
    pub nonce: String,
    pub signature: String,
    pub timestamp: i64,
    pub subject_hint: String,
}

/// Contact entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub ledger_id: String,
    pub public_key: String,
    pub display_name: Option<String>,
    pub gmail_address: Option<String>,
}

/// Generic API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}
