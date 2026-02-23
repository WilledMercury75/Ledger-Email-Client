use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use super::keys::LedgerIdentity;
use crate::models::message::EncryptedEnvelope;

/// Encrypt a message for a recipient
pub fn encrypt_message(
    sender: &LedgerIdentity,
    recipient_encryption_pubkey: &[u8],
    subject: &str,
    plaintext: &str,
) -> Result<EncryptedEnvelope, Box<dyn std::error::Error>> {
    // Generate ephemeral X25519 keypair for this message
    let ephemeral_secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

    // Perform DH with recipient's public key
    let recipient_pubkey = X25519PublicKey::from(
        <[u8; 32]>::try_from(recipient_encryption_pubkey)
            .map_err(|_| "Invalid recipient public key length")?
    );
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pubkey);

    // Derive symmetric key via HKDF
    let hk = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut sym_key = [0u8; 32];
    hk.expand(b"ledger-message-key", &mut sym_key)
        .map_err(|e| format!("HKDF error: {}", e))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt with ChaCha20-Poly1305
    let cipher = ChaCha20Poly1305::new_from_slice(&sym_key)
        .map_err(|e| format!("Cipher init error: {}", e))?;
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption error: {}", e))?;

    // Sign the ciphertext with sender's Ed25519 key
    let signature = sender.sign(&ciphertext);

    let envelope = EncryptedEnvelope {
        id: uuid::Uuid::new_v4().to_string(),
        from_ledger_id: sender.ledger_id.clone(),
        to_ledger_id: String::new(), // filled by caller
        ephemeral_pubkey: BASE64.encode(ephemeral_public.as_bytes()),
        encrypted_body: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(&nonce_bytes),
        signature: BASE64.encode(&signature),
        timestamp: chrono::Utc::now().timestamp(),
        subject_hint: subject.to_string(),
    };

    Ok(envelope)
}

/// Decrypt a received envelope
pub fn decrypt_envelope(
    recipient: &LedgerIdentity,
    envelope: &EncryptedEnvelope,
) -> Result<String, Box<dyn std::error::Error>> {
    // Decode ephemeral public key
    let ephemeral_bytes = BASE64.decode(&envelope.ephemeral_pubkey)?;
    let ephemeral_pubkey = X25519PublicKey::from(
        <[u8; 32]>::try_from(ephemeral_bytes.as_slice())
            .map_err(|_| "Invalid ephemeral public key")?
    );

    // Perform DH with our secret key
    let shared_secret = recipient.encryption_secret.diffie_hellman(&ephemeral_pubkey);

    // Derive symmetric key
    let hk = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut sym_key = [0u8; 32];
    hk.expand(b"ledger-message-key", &mut sym_key)
        .map_err(|e| format!("HKDF error: {}", e))?;

    // Decode nonce and ciphertext
    let nonce_bytes = BASE64.decode(&envelope.nonce)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = BASE64.decode(&envelope.encrypted_body)?;

    // Verify signature
    let sender_pubkey = LedgerIdentity::pubkey_from_ledger_id(&envelope.from_ledger_id)?;
    let signature_bytes = BASE64.decode(&envelope.signature)?;
    let valid = LedgerIdentity::verify(&sender_pubkey, &ciphertext, &signature_bytes)?;
    if !valid {
        return Err("Signature verification failed".into());
    }

    // Decrypt
    let cipher = ChaCha20Poly1305::new_from_slice(&sym_key)
        .map_err(|e| format!("Cipher init error: {}", e))?;
    let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
        .map_err(|e| format!("Decryption error: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let sender = LedgerIdentity::generate().unwrap();
        let recipient = LedgerIdentity::generate().unwrap();

        let mut envelope = encrypt_message(
            &sender,
            &recipient.encryption_public_bytes(),
            "Test Subject",
            "Hello, this is a secret message!",
        ).unwrap();

        envelope.to_ledger_id = recipient.ledger_id.clone();

        let decrypted = decrypt_envelope(&recipient, &envelope).unwrap();
        assert_eq!(decrypted, "Hello, this is a secret message!");
    }

    #[test]
    fn test_wrong_recipient_fails() {
        let sender = LedgerIdentity::generate().unwrap();
        let recipient = LedgerIdentity::generate().unwrap();
        let wrong_recipient = LedgerIdentity::generate().unwrap();

        let mut envelope = encrypt_message(
            &sender,
            &recipient.encryption_public_bytes(),
            "Test",
            "Secret message",
        ).unwrap();

        envelope.to_ledger_id = recipient.ledger_id.clone();

        // Wrong recipient should fail to decrypt
        let result = decrypt_envelope(&wrong_recipient, &envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let sender = LedgerIdentity::generate().unwrap();
        let recipient = LedgerIdentity::generate().unwrap();

        let mut envelope = encrypt_message(
            &sender,
            &recipient.encryption_public_bytes(),
            "Test",
            "Secret message",
        ).unwrap();

        envelope.to_ledger_id = recipient.ledger_id.clone();

        // Tamper with the encrypted body
        let mut body_bytes = BASE64.decode(&envelope.encrypted_body).unwrap();
        if let Some(byte) = body_bytes.first_mut() {
            *byte ^= 0xFF;
        }
        envelope.encrypted_body = BASE64.encode(&body_bytes);

        let result = decrypt_envelope(&recipient, &envelope);
        assert!(result.is_err());
    }
}
