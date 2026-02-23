use crate::models::message::{GmailConfig, Message, DeliveryMethod, Folder};

/// Fetch new messages from Gmail via IMAP
pub fn fetch_messages(
    config: &GmailConfig,
    max_count: u32,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    let imap_host = config.imap_host.as_deref().unwrap_or("imap.gmail.com");
    let tls = native_tls::TlsConnector::builder().build()?;

    let client = imap::connect((imap_host, 993), imap_host, &tls)?;
    let mut session = client.login(&config.email, &config.app_password)
        .map_err(|e| format!("IMAP login failed: {}", e.0))?;

    session.select("INBOX")?;

    // Fetch the last N messages
    let sequence = if max_count > 0 {
        let exists = session.select("INBOX")?.exists;
        if exists == 0 {
            session.logout()?;
            return Ok(vec![]);
        }
        let start = if exists > max_count { exists - max_count + 1 } else { 1 };
        format!("{}:{}", start, exists)
    } else {
        "1:*".to_string()
    };

    let messages_result = session.fetch(&sequence, "RFC822")?;

    let mut messages = Vec::new();

    for fetch in messages_result.iter() {
        if let Some(body) = fetch.body() {
            match mailparse::parse_mail(body) {
                Ok(parsed) => {
                    let from = parsed.headers.iter()
                        .find(|h| h.get_key().eq_ignore_ascii_case("from"))
                        .map(|h| h.get_value())
                        .unwrap_or_default();

                    let to = parsed.headers.iter()
                        .find(|h| h.get_key().eq_ignore_ascii_case("to"))
                        .map(|h| h.get_value())
                        .unwrap_or_default();

                    let subject = parsed.headers.iter()
                        .find(|h| h.get_key().eq_ignore_ascii_case("subject"))
                        .map(|h| h.get_value())
                        .unwrap_or_default();

                    let body_text = parsed.get_body().unwrap_or_default();

                    // Check if this is a Ledger fallback message
                    let is_fallback = subject.contains("[Ledger Encrypted Fallback]");
                    let delivery = if is_fallback {
                        DeliveryMethod::Fallback
                    } else {
                        DeliveryMethod::Gmail
                    };

                    let msg = Message {
                        id: uuid::Uuid::new_v4().to_string(),
                        from_id: from,
                        to_id: to,
                        subject,
                        body: body_text,
                        timestamp: chrono::Utc::now().timestamp(),
                        delivery_method: delivery,
                        is_read: false,
                        folder: Folder::Inbox,
                        signature: None,
                        encrypted: is_fallback,
                    };

                    messages.push(msg);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse email: {}", e);
                }
            }
        }
    }

    session.logout()?;

    Ok(messages)
}

/// Extract encrypted payload from a fallback message body
pub fn extract_encrypted_payload(body: &str) -> Option<String> {
    let start_marker = "--- BEGIN LEDGER ENCRYPTED MESSAGE ---";
    let end_marker = "--- END LEDGER ENCRYPTED MESSAGE ---";

    let start = body.find(start_marker)? + start_marker.len();
    let end = body.find(end_marker)?;

    let payload = body[start..end].trim().to_string();
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}
