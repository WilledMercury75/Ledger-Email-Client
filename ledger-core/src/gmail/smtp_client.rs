use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor,
};

use crate::models::message::GmailConfig;

/// Send an email via Gmail SMTP
pub async fn send_email(
    config: &GmailConfig,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let smtp_host = config.smtp_host.as_deref().unwrap_or("smtp.gmail.com");

    let email = LettreMessage::builder()
        .from(config.email.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())?;

    let creds = Credentials::new(config.email.clone(), config.app_password.clone());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)?
        .credentials(creds)
        .build();

    mailer.send(email).await?;

    tracing::info!("Email sent to {} via Gmail SMTP", to);
    Ok(())
}

/// Send an encrypted fallback email (encrypted body as base64 in the message)
pub async fn send_encrypted_fallback(
    config: &GmailConfig,
    to: &str,
    encrypted_payload: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "[Ledger Encrypted Fallback]";
    let body = format!(
        "This message was sent by the Ledger encrypted mail client.\n\
         The recipient's Ledger node was unreachable, so this encrypted fallback was sent.\n\
         \n\
         --- BEGIN LEDGER ENCRYPTED MESSAGE ---\n\
         {}\n\
         --- END LEDGER ENCRYPTED MESSAGE ---\n",
        encrypted_payload
    );

    send_email(config, to, subject, &body).await
}
