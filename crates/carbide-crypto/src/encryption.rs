//! File encryption and decryption for secure storage
//!
//! This module provides AES-256-GCM encryption for client-side security,
//! ensuring that only users with the correct keys can access their data.

use std::fmt;

use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce as AesNonce};
use carbide_core::{CarbideError, Result};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

/// AES-256-GCM encryption key (32 bytes)
#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptionKey([u8; 32]);

impl EncryptionKey {
    /// Generate a new random encryption key
    pub fn generate() -> Result<Self> {
        let rng = SystemRandom::new();
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes)
            .map_err(|_| CarbideError::Internal("Failed to generate random key".to_string()))?;
        Ok(Self(key_bytes))
    }

    /// Create encryption key from existing bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get raw key bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string for storage
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)
            .map_err(|_| CarbideError::Internal("Invalid hex string".to_string()))?;

        if bytes.len() != 32 {
            return Err(CarbideError::Internal("Key must be 32 bytes".to_string()));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(Self(key))
    }
}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EncryptionKey([REDACTED])")
    }
}

/// Nonce for AES-256-GCM (96 bits / 12 bytes)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Nonce([u8; 12]);

impl Nonce {
    /// Generate a new random nonce
    pub fn generate() -> Result<Self> {
        let rng = SystemRandom::new();
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes)
            .map_err(|_| CarbideError::Internal("Failed to generate random nonce".to_string()))?;
        Ok(Self(nonce_bytes))
    }

    /// Create nonce from bytes
    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        Self(bytes)
    }

    /// Get raw nonce bytes
    pub fn as_bytes(&self) -> &[u8; 12] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)
            .map_err(|_| CarbideError::Internal("Invalid hex string".to_string()))?;

        if bytes.len() != 12 {
            return Err(CarbideError::Internal("Nonce must be 12 bytes".to_string()));
        }

        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes);
        Ok(Self(nonce))
    }
}

/// Encrypted data with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Encrypted ciphertext
    pub ciphertext: Vec<u8>,
    /// Nonce used for encryption
    pub nonce: Nonce,
    /// Authentication tag (included in ciphertext for AES-GCM)
    pub tag_size: usize,
}

/// File encryptor using AES-256-GCM
pub struct FileEncryptor {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for FileEncryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileEncryptor")
            .field("cipher", &"AES-256-GCM")
            .finish()
    }
}

impl FileEncryptor {
    /// Create a new encryptor with the given key
    pub fn new(encryption_key: &EncryptionKey) -> Result<Self> {
        let key = Key::<Aes256Gcm>::from_slice(encryption_key.as_bytes());
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Encrypt data with a random nonce
    pub fn encrypt(&self, data: &[u8]) -> Result<EncryptedData> {
        let nonce = Nonce::generate()?;
        self.encrypt_with_nonce(data, &nonce)
    }

    /// Encrypt data with a specific nonce
    pub fn encrypt_with_nonce(&self, data: &[u8], nonce: &Nonce) -> Result<EncryptedData> {
        let aes_nonce = AesNonce::from_slice(nonce.as_bytes());

        let ciphertext = self
            .cipher
            .encrypt(aes_nonce, data)
            .map_err(|_| CarbideError::Internal("Encryption failed".to_string()))?;

        Ok(EncryptedData {
            ciphertext,
            nonce: nonce.clone(),
            tag_size: 16, // AES-GCM tag is 16 bytes
        })
    }
}

/// File decryptor using AES-256-GCM
pub struct FileDecryptor {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for FileDecryptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileDecryptor")
            .field("cipher", &"AES-256-GCM")
            .finish()
    }
}

impl FileDecryptor {
    /// Create a new decryptor with the given key
    pub fn new(encryption_key: &EncryptionKey) -> Result<Self> {
        let key = Key::<Aes256Gcm>::from_slice(encryption_key.as_bytes());
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Decrypt data
    pub fn decrypt(&self, encrypted_data: &EncryptedData) -> Result<Vec<u8>> {
        let aes_nonce = AesNonce::from_slice(encrypted_data.nonce.as_bytes());

        let plaintext = self
            .cipher
            .decrypt(aes_nonce, encrypted_data.ciphertext.as_slice())
            .map_err(|_| {
                CarbideError::Internal("Decryption failed - invalid ciphertext or key".to_string())
            })?;

        Ok(plaintext)
    }
}

/// Key derivation functions for generating encryption keys from user inputs
pub struct KeyDerivation;

impl KeyDerivation {
    /// Derive encryption key from password using PBKDF2
    pub fn derive_from_password(
        password: &str,
        salt: &[u8],
        iterations: u32,
    ) -> Result<EncryptionKey> {
        if password.is_empty() {
            return Err(CarbideError::Internal(
                "Password cannot be empty".to_string(),
            ));
        }

        if salt.len() < 16 {
            return Err(CarbideError::Internal(
                "Salt must be at least 16 bytes".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        ring::pbkdf2::derive(
            ring::pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(iterations).unwrap(),
            salt,
            password.as_bytes(),
            &mut key,
        );

        Ok(EncryptionKey(key))
    }

    /// Generate a random salt for password-based key derivation
    pub fn generate_salt() -> Result<[u8; 32]> {
        let rng = SystemRandom::new();
        let mut salt = [0u8; 32];
        rng.fill(&mut salt)
            .map_err(|_| CarbideError::Internal("Failed to generate random salt".to_string()))?;
        Ok(salt)
    }

    /// Derive key from a master key and context (for hierarchical keys)
    pub fn derive_from_master_key(
        master_key: &EncryptionKey,
        context: &[u8],
    ) -> Result<EncryptionKey> {
        // Use HKDF to derive a new key from master key + context
        let salt = ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, &[]);
        let prk = salt.extract(master_key.as_bytes());

        let mut derived_key = [0u8; 32];
        prk.expand(&[context], ring::hkdf::HKDF_SHA256)
            .map_err(|_| CarbideError::Internal("Key derivation failed".to_string()))?
            .fill(&mut derived_key)
            .map_err(|_| CarbideError::Internal("Key derivation failed".to_string()))?;

        Ok(EncryptionKey(derived_key))
    }
}

/// Secure storage for user encryption keys
#[derive(Debug)]
pub struct KeyManager {
    master_key: EncryptionKey,
}

impl KeyManager {
    /// Create a new key manager with a master key
    pub fn new(master_key: EncryptionKey) -> Self {
        Self { master_key }
    }

    /// Generate a new key manager with a random master key
    pub fn generate() -> Result<Self> {
        let master_key = EncryptionKey::generate()?;
        Ok(Self { master_key })
    }

    /// Derive a file-specific encryption key
    pub fn derive_file_key(&self, file_id: &str) -> Result<EncryptionKey> {
        KeyDerivation::derive_from_master_key(&self.master_key, file_id.as_bytes())
    }

    /// Get the master key (for backup purposes)
    pub fn master_key(&self) -> &EncryptionKey {
        &self.master_key
    }

    /// Encrypt and store keys securely
    pub fn export_encrypted_master_key(&self, password: &str) -> Result<EncryptedMasterKey> {
        let salt = KeyDerivation::generate_salt()?;
        let password_key = KeyDerivation::derive_from_password(password, &salt, 100_000)?; // 100k iterations

        let encryptor = FileEncryptor::new(&password_key)?;
        let encrypted_data = encryptor.encrypt(self.master_key.as_bytes())?;

        Ok(EncryptedMasterKey {
            encrypted_key: encrypted_data,
            salt,
            iterations: 100_000,
        })
    }

    /// Import and decrypt master key
    pub fn import_encrypted_master_key(
        encrypted_master: &EncryptedMasterKey,
        password: &str,
    ) -> Result<Self> {
        let password_key = KeyDerivation::derive_from_password(
            password,
            &encrypted_master.salt,
            encrypted_master.iterations,
        )?;

        let decryptor = FileDecryptor::new(&password_key)?;
        let master_key_bytes = decryptor.decrypt(&encrypted_master.encrypted_key)?;

        if master_key_bytes.len() != 32 {
            return Err(CarbideError::Internal(
                "Invalid master key length".to_string(),
            ));
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&master_key_bytes);
        let master_key = EncryptionKey::from_bytes(key_array);

        Ok(Self { master_key })
    }
}

/// Encrypted master key for secure storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedMasterKey {
    /// Encrypted master key data
    pub encrypted_key: EncryptedData,
    /// Salt used for password-based key derivation
    pub salt: [u8; 32],
    /// Number of PBKDF2 iterations
    pub iterations: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let key = EncryptionKey::generate().unwrap();
        let encryptor = FileEncryptor::new(&key).unwrap();
        let decryptor = FileDecryptor::new(&key).unwrap();

        let original_data = b"Hello, Carbide Network! This is test data for encryption.";

        // Encrypt the data
        let encrypted = encryptor.encrypt(original_data).unwrap();
        assert_ne!(encrypted.ciphertext, original_data);
        assert_eq!(encrypted.tag_size, 16);

        // Decrypt the data
        let decrypted = decryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original_data);
    }

    #[test]
    fn test_key_serialization() {
        let key = EncryptionKey::generate().unwrap();
        let hex = key.to_hex();
        let restored_key = EncryptionKey::from_hex(&hex).unwrap();

        assert_eq!(key.as_bytes(), restored_key.as_bytes());
    }

    #[test]
    fn test_nonce_generation() {
        let nonce1 = Nonce::generate().unwrap();
        let nonce2 = Nonce::generate().unwrap();

        // Nonces should be different
        assert_ne!(nonce1.as_bytes(), nonce2.as_bytes());

        // Test serialization
        let hex = nonce1.to_hex();
        let restored_nonce = Nonce::from_hex(&hex).unwrap();
        assert_eq!(nonce1.as_bytes(), restored_nonce.as_bytes());
    }

    #[test]
    fn test_password_based_key_derivation() {
        let password = "super_secure_password123";
        let salt = KeyDerivation::generate_salt().unwrap();

        let key1 = KeyDerivation::derive_from_password(password, &salt, 1000).unwrap();
        let key2 = KeyDerivation::derive_from_password(password, &salt, 1000).unwrap();

        // Same password and salt should produce same key
        assert_eq!(key1.as_bytes(), key2.as_bytes());

        // Different salt should produce different key
        let different_salt = KeyDerivation::generate_salt().unwrap();
        let key3 = KeyDerivation::derive_from_password(password, &different_salt, 1000).unwrap();
        assert_ne!(key1.as_bytes(), key3.as_bytes());
    }

    #[test]
    fn test_key_manager() {
        let key_manager = KeyManager::generate().unwrap();

        // Derive file-specific keys
        let file_key1 = key_manager.derive_file_key("file1.txt").unwrap();
        let file_key2 = key_manager.derive_file_key("file2.txt").unwrap();

        // Different files should have different keys
        assert_ne!(file_key1.as_bytes(), file_key2.as_bytes());

        // Same file should have same key
        let file_key1_again = key_manager.derive_file_key("file1.txt").unwrap();
        assert_eq!(file_key1.as_bytes(), file_key1_again.as_bytes());
    }

    #[test]
    fn test_encrypted_master_key() {
        let original_manager = KeyManager::generate().unwrap();
        let password = "test_password123";

        // Export encrypted master key
        let encrypted_master = original_manager
            .export_encrypted_master_key(password)
            .unwrap();

        // Import and verify
        let restored_manager =
            KeyManager::import_encrypted_master_key(&encrypted_master, password).unwrap();

        // Both managers should derive the same file keys
        let file_key1 = original_manager.derive_file_key("test.txt").unwrap();
        let file_key2 = restored_manager.derive_file_key("test.txt").unwrap();
        assert_eq!(file_key1.as_bytes(), file_key2.as_bytes());
    }

    #[test]
    fn test_encryption_with_wrong_key() {
        let key1 = EncryptionKey::generate().unwrap();
        let key2 = EncryptionKey::generate().unwrap();

        let encryptor = FileEncryptor::new(&key1).unwrap();
        let wrong_decryptor = FileDecryptor::new(&key2).unwrap();

        let data = b"secret data";
        let encrypted = encryptor.encrypt(data).unwrap();

        // Decryption with wrong key should fail
        let result = wrong_decryptor.decrypt(&encrypted);
        assert!(result.is_err());
    }
}
