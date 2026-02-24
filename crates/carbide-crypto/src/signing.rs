//! Ed25519 key pair management for provider identity and proof signing
//!
//! Provides key generation, persistence, signing, and verification using
//! the `ring` library's Ed25519 implementation.

use std::path::Path;

use carbide_core::{CarbideError, Result};
use ring::rand::SystemRandom;
use ring::signature::{self, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Persistent key pair data for JSON serialization
#[derive(Serialize, Deserialize)]
struct KeyPairData {
    /// PKCS#8 v2 encoded private key (hex)
    private_key_pkcs8: String,
}

/// Ed25519 key pair for provider identity and proof signing
pub struct ProviderKeyPair {
    key_pair: Ed25519KeyPair,
    pkcs8_bytes: Vec<u8>,
}

impl std::fmt::Debug for ProviderKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderKeyPair")
            .field("public_key", &self.public_key_hex())
            .finish()
    }
}

impl ProviderKeyPair {
    /// Generate a new random Ed25519 key pair
    pub fn generate() -> Result<Self> {
        let rng = SystemRandom::new();
        let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|_| CarbideError::Internal("Failed to generate Ed25519 key pair".to_string()))?;
        let pkcs8_vec = pkcs8_bytes.as_ref().to_vec();

        let key_pair = Ed25519KeyPair::from_pkcs8(&pkcs8_vec)
            .map_err(|_| CarbideError::Internal("Failed to parse generated key pair".to_string()))?;

        Ok(Self {
            key_pair,
            pkcs8_bytes: pkcs8_vec,
        })
    }

    /// Load key pair from a JSON file, or generate and save a new one if the file doesn't exist
    pub fn load_or_generate(path: &Path) -> Result<Self> {
        if path.exists() {
            info!("Loading provider key pair from {}", path.display());
            let contents = std::fs::read_to_string(path).map_err(|e| {
                CarbideError::Internal(format!("Failed to read key file {}: {e}", path.display()))
            })?;

            let data: KeyPairData = serde_json::from_str(&contents).map_err(|e| {
                CarbideError::Internal(format!("Failed to parse key file: {e}"))
            })?;

            let pkcs8_bytes = hex::decode(&data.private_key_pkcs8).map_err(|e| {
                CarbideError::Internal(format!("Failed to decode key hex: {e}"))
            })?;

            let key_pair = Ed25519KeyPair::from_pkcs8(&pkcs8_bytes).map_err(|e| {
                CarbideError::Internal(format!("Failed to parse Ed25519 key pair: {e}"))
            })?;

            Ok(Self {
                key_pair,
                pkcs8_bytes,
            })
        } else {
            info!("Generating new provider key pair at {}", path.display());
            let kp = Self::generate()?;
            kp.save_to_file(path)?;
            Ok(kp)
        }
    }

    /// Save key pair to a JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CarbideError::Internal(format!(
                    "Failed to create key directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        let data = KeyPairData {
            private_key_pkcs8: hex::encode(&self.pkcs8_bytes),
        };

        let json = serde_json::to_string_pretty(&data).map_err(|e| {
            CarbideError::Internal(format!("Failed to serialize key pair: {e}"))
        })?;

        std::fs::write(path, json).map_err(|e| {
            CarbideError::Internal(format!("Failed to write key file {}: {e}", path.display()))
        })?;

        Ok(())
    }

    /// Get the hex-encoded public key (for registration and verification)
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.key_pair.public_key().as_ref())
    }

    /// Sign data and return the signature bytes
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.key_pair.sign(data).as_ref().to_vec()
    }

    /// Verify a signature against a public key (static method)
    pub fn verify(public_key_bytes: &[u8], data: &[u8], signature_bytes: &[u8]) -> bool {
        let public_key =
            UnparsedPublicKey::new(&signature::ED25519, public_key_bytes);
        public_key.verify(data, signature_bytes).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_sign() {
        let kp = ProviderKeyPair::generate().unwrap();
        let data = b"hello world";
        let sig = kp.sign(data);

        assert_eq!(sig.len(), 64);
        assert_ne!(sig, vec![0u8; 64]);
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let kp = ProviderKeyPair::generate().unwrap();
        let data = b"proof response hash data";
        let sig = kp.sign(data);

        let pub_key_bytes = hex::decode(kp.public_key_hex()).unwrap();
        assert!(ProviderKeyPair::verify(&pub_key_bytes, data, &sig));
    }

    #[test]
    fn test_verify_wrong_data_fails() {
        let kp = ProviderKeyPair::generate().unwrap();
        let sig = kp.sign(b"original data");

        let pub_key_bytes = hex::decode(kp.public_key_hex()).unwrap();
        assert!(!ProviderKeyPair::verify(&pub_key_bytes, b"tampered data", &sig));
    }

    #[test]
    fn test_verify_wrong_key_fails() {
        let kp1 = ProviderKeyPair::generate().unwrap();
        let kp2 = ProviderKeyPair::generate().unwrap();

        let data = b"some data";
        let sig = kp1.sign(data);

        let wrong_pub = hex::decode(kp2.public_key_hex()).unwrap();
        assert!(!ProviderKeyPair::verify(&wrong_pub, data, &sig));
    }

    #[test]
    fn test_load_or_generate_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("test_provider.key.json");

        // Generate new
        let kp1 = ProviderKeyPair::load_or_generate(&key_path).unwrap();
        let pub1 = kp1.public_key_hex();

        // Load existing
        let kp2 = ProviderKeyPair::load_or_generate(&key_path).unwrap();
        let pub2 = kp2.public_key_hex();

        assert_eq!(pub1, pub2);

        // Verify signing still works after reload
        let data = b"test data";
        let sig = kp2.sign(data);
        let pub_bytes = hex::decode(&pub2).unwrap();
        assert!(ProviderKeyPair::verify(&pub_bytes, data, &sig));
    }
}
