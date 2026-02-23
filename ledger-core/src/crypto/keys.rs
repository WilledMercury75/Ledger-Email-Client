use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use x25519_dalek::{StaticSecret, PublicKey as X25519PublicKey};
use rand::rngs::OsRng;
use std::path::PathBuf;
use std::fs;

/// Holds both signing (Ed25519) and encryption (X25519) keys
#[derive(Clone)]
pub struct LedgerIdentity {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub encryption_secret: StaticSecret,
    pub encryption_public: X25519PublicKey,
    pub ledger_id: String,
}

impl LedgerIdentity {
    /// Generate a new identity or load from disk
    pub fn load_or_create(data_dir: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let key_path = data_dir.join("identity.key");

        if key_path.exists() {
            tracing::info!("Loading existing identity from {:?}", key_path);
            Self::load_from_file(&key_path)
        } else {
            tracing::info!("Generating new identity...");
            let identity = Self::generate()?;
            fs::create_dir_all(data_dir)?;
            identity.save_to_file(&key_path)?;
            tracing::info!("Identity saved to {:?}", key_path);
            Ok(identity)
        }
    }

    /// Generate a brand new identity
    pub fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        let mut csprng = OsRng;

        // Ed25519 signing key
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        // X25519 encryption key (derived deterministically from signing key seed)
        // We use the Ed25519 seed to derive a separate X25519 key via HKDF
        let seed = signing_key.to_bytes();
        let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(b"ledger-x25519"), &seed);
        let mut x25519_bytes = [0u8; 32];
        hk.expand(b"encryption-key", &mut x25519_bytes)
            .map_err(|e| format!("HKDF expand error: {}", e))?;

        let encryption_secret = StaticSecret::from(x25519_bytes);
        let encryption_public = X25519PublicKey::from(&encryption_secret);

        let ledger_id = format!("ledger:{}", bs58::encode(verifying_key.as_bytes()).into_string());

        Ok(Self {
            signing_key,
            verifying_key,
            encryption_secret,
            encryption_public,
            ledger_id,
        })
    }

    /// Save identity to file (Ed25519 seed only — X25519 is derived)
    fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let seed = self.signing_key.to_bytes();
        fs::write(path, seed)?;
        // Restrict permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Load identity from file
    fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let seed_bytes = fs::read(path)?;
        if seed_bytes.len() != 32 {
            return Err("Invalid identity file: expected 32 bytes".into());
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&seed_bytes);

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(b"ledger-x25519"), &seed);
        let mut x25519_bytes = [0u8; 32];
        hk.expand(b"encryption-key", &mut x25519_bytes)
            .map_err(|e| format!("HKDF expand error: {}", e))?;

        let encryption_secret = StaticSecret::from(x25519_bytes);
        let encryption_public = X25519PublicKey::from(&encryption_secret);

        let ledger_id = format!("ledger:{}", bs58::encode(verifying_key.as_bytes()).into_string());

        Ok(Self {
            signing_key,
            verifying_key,
            encryption_secret,
            encryption_public,
            ledger_id,
        })
    }

    /// Sign a message
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(data);
        signature.to_bytes().to_vec()
    }

    /// Verify a signature from a given public key
    pub fn verify(pubkey_bytes: &[u8], data: &[u8], signature_bytes: &[u8]) -> Result<bool, Box<dyn std::error::Error>> {
        let pubkey = VerifyingKey::from_bytes(
            pubkey_bytes.try_into().map_err(|_| "Invalid public key length")?
        )?;
        let signature = Signature::from_bytes(
            signature_bytes.try_into().map_err(|_| "Invalid signature length")?
        );
        Ok(pubkey.verify(data, &signature).is_ok())
    }

    /// Get public key bytes (Ed25519)
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.verifying_key.as_bytes().to_vec()
    }

    /// Get encryption public key bytes (X25519)
    pub fn encryption_public_bytes(&self) -> Vec<u8> {
        self.encryption_public.as_bytes().to_vec()
    }

    /// Parse a Ledger ID back to public key bytes
    pub fn pubkey_from_ledger_id(ledger_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let id = ledger_id.strip_prefix("ledger:").ok_or("Invalid Ledger ID format")?;
        let bytes = bs58::decode(id).into_vec()?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity() {
        let identity = LedgerIdentity::generate().unwrap();
        assert!(identity.ledger_id.starts_with("ledger:"));
        assert_eq!(identity.public_key_bytes().len(), 32);
        assert_eq!(identity.encryption_public_bytes().len(), 32);
    }

    #[test]
    fn test_sign_verify() {
        let identity = LedgerIdentity::generate().unwrap();
        let data = b"Hello, Ledger!";
        let signature = identity.sign(data);
        let valid = LedgerIdentity::verify(&identity.public_key_bytes(), data, &signature).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_sign_verify_tampered() {
        let identity = LedgerIdentity::generate().unwrap();
        let data = b"Hello, Ledger!";
        let signature = identity.sign(data);
        let valid = LedgerIdentity::verify(&identity.public_key_bytes(), b"Tampered!", &signature).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_ledger_id_roundtrip() {
        let identity = LedgerIdentity::generate().unwrap();
        let pubkey = LedgerIdentity::pubkey_from_ledger_id(&identity.ledger_id).unwrap();
        assert_eq!(pubkey, identity.public_key_bytes());
    }

    #[test]
    fn test_save_load() {
        let tmp = std::env::temp_dir().join("ledger_test_identity");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let original = LedgerIdentity::load_or_create(&tmp).unwrap();
        let loaded = LedgerIdentity::load_or_create(&tmp).unwrap();

        assert_eq!(original.ledger_id, loaded.ledger_id);
        assert_eq!(original.public_key_bytes(), loaded.public_key_bytes());

        let _ = fs::remove_dir_all(&tmp);
    }
}
