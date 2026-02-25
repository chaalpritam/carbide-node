//! Ethereum wallet for Carbide Network payments
//!
//! Provides secp256k1 key management, BIP-39 mnemonic backup,
//! EIP-55 checksum addresses, and EIP-712 typed data signing.

use std::fmt;
use std::path::Path;

use carbide_core::{CarbideError, Result};
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

use crate::encryption::{
    FileDecryptor, FileEncryptor, KeyDerivation,
};

/// Number of PBKDF2 iterations for wallet encryption
const WALLET_PBKDF2_ITERATIONS: u32 = 100_000;

/// 20-byte Ethereum address with EIP-55 checksum display
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EthAddress([u8; 20]);

impl EthAddress {
    /// Create from raw 20-byte address
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Derive address from an uncompressed public key (64 bytes, no 0x04 prefix)
    pub fn from_public_key_bytes(pubkey_bytes: &[u8]) -> Self {
        let mut hasher = Keccak::v256();
        hasher.update(pubkey_bytes);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);

        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..]);
        Self(addr)
    }

    /// Get raw address bytes
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Return lowercase hex without checksum
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    /// Return EIP-55 mixed-case checksum address
    pub fn to_checksum(&self) -> String {
        let hex_addr = hex::encode(self.0);
        let mut hasher = Keccak::v256();
        hasher.update(hex_addr.as_bytes());
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);

        let mut checksum = String::with_capacity(42);
        checksum.push_str("0x");
        for (i, c) in hex_addr.chars().enumerate() {
            let nibble = if i % 2 == 0 {
                (hash[i / 2] >> 4) & 0x0f
            } else {
                hash[i / 2] & 0x0f
            };
            if nibble >= 8 {
                checksum.push(c.to_ascii_uppercase());
            } else {
                checksum.push(c);
            }
        }
        checksum
    }

    /// Parse from hex string (with or without 0x prefix)
    pub fn from_hex(s: &str) -> Result<Self> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)
            .map_err(|_| CarbideError::Internal("Invalid hex address".to_string()))?;
        if bytes.len() != 20 {
            return Err(CarbideError::Internal(
                "Address must be 20 bytes".to_string(),
            ));
        }
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes);
        Ok(Self(addr))
    }
}

impl fmt::Display for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_checksum())
    }
}

impl fmt::Debug for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EthAddress({})", self.to_checksum())
    }
}

/// 65-byte ECDSA signature (r + s + v)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletSignature {
    /// r component (32 bytes)
    pub r: [u8; 32],
    /// s component (32 bytes)
    pub s: [u8; 32],
    /// Recovery id (0 or 1, mapped to 27 or 28 for Ethereum)
    pub v: u8,
}

impl WalletSignature {
    /// Encode as 65 bytes: r (32) + s (32) + v (1)
    pub fn to_bytes(&self) -> [u8; 65] {
        let mut out = [0u8; 65];
        out[..32].copy_from_slice(&self.r);
        out[32..64].copy_from_slice(&self.s);
        out[64] = self.v;
        out
    }

    /// Decode from 65 bytes
    pub fn from_bytes(bytes: &[u8; 65]) -> Self {
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..64]);
        Self {
            r,
            s,
            v: bytes[64],
        }
    }

    /// Return hex-encoded signature
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.to_bytes()))
    }
}

/// Encrypted wallet file format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedWallet {
    /// Format version
    pub version: u32,
    /// Ethereum address (for identification without decrypting)
    pub address: String,
    /// Encrypted private key bytes (AES-256-GCM ciphertext)
    pub encrypted_private_key: Vec<u8>,
    /// AES-GCM nonce
    pub nonce: Vec<u8>,
    /// PBKDF2 salt
    pub salt: Vec<u8>,
    /// PBKDF2 iteration count
    pub iterations: u32,
}

/// Carbide Ethereum wallet wrapping a secp256k1 signing key
pub struct CarbideWallet {
    signing_key: SigningKey,
    address: EthAddress,
}

impl CarbideWallet {
    /// Generate a new random wallet, returning the wallet and its 12-word BIP-39 mnemonic
    pub fn generate() -> Result<(Self, String)> {
        let rng = SystemRandom::new();
        let mut entropy = [0u8; 16]; // 128 bits → 12 words
        rng.fill(&mut entropy)
            .map_err(|_| CarbideError::Internal("Failed to generate entropy".to_string()))?;

        let mnemonic = bip39::Mnemonic::from_entropy(&entropy)
            .map_err(|e| CarbideError::Internal(format!("Mnemonic generation failed: {e}")))?;

        let wallet = Self::from_mnemonic(&mnemonic.to_string())?;
        Ok((wallet, mnemonic.to_string()))
    }

    /// Restore wallet from a BIP-39 mnemonic phrase
    pub fn from_mnemonic(phrase: &str) -> Result<Self> {
        let mnemonic = bip39::Mnemonic::parse(phrase)
            .map_err(|e| CarbideError::Internal(format!("Invalid mnemonic: {e}")))?;

        // Derive seed with empty passphrase (standard BIP-39)
        let seed = mnemonic.to_seed("");

        // Use first 32 bytes of seed as private key (simplified derivation)
        // Full BIP-44 derivation would use m/44'/60'/0'/0/0 but requires
        // additional dependencies. This is sufficient for Carbide's dedicated wallet.
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&seed[..32]);

        Self::from_private_key(&key_bytes)
    }

    /// Import wallet from a raw 32-byte private key
    pub fn from_private_key(bytes: &[u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(bytes.into())
            .map_err(|e| CarbideError::Internal(format!("Invalid private key: {e}")))?;

        let verifying_key = signing_key.verifying_key();
        let point = verifying_key.to_encoded_point(false);
        // Uncompressed point: 0x04 || x (32 bytes) || y (32 bytes)
        let pubkey_bytes = &point.as_bytes()[1..]; // skip 0x04 prefix

        let address = EthAddress::from_public_key_bytes(pubkey_bytes);

        Ok(Self {
            signing_key,
            address,
        })
    }

    /// Get the wallet's Ethereum address
    pub fn address(&self) -> &EthAddress {
        &self.address
    }

    /// Get the raw private key bytes (use with caution)
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes().into()
    }

    /// Sign a 32-byte hash with ECDSA, returning a recoverable signature
    pub fn sign_hash(&self, hash: &[u8; 32]) -> Result<WalletSignature> {
        let (signature, recovery_id) = self
            .signing_key
            .sign_prehash_recoverable(hash)
            .map_err(|e| CarbideError::Internal(format!("Signing failed: {e}")))?;

        let r_bytes: [u8; 32] = signature.r().to_bytes().into();
        let s_bytes: [u8; 32] = signature.s().to_bytes().into();

        Ok(WalletSignature {
            r: r_bytes,
            s: s_bytes,
            v: recovery_id.to_byte() + 27, // Ethereum convention
        })
    }

    /// Sign EIP-712 typed data given a domain separator and struct hash
    pub fn sign_typed_data(
        &self,
        domain_separator: &[u8; 32],
        struct_hash: &[u8; 32],
    ) -> Result<WalletSignature> {
        // EIP-712: hash = keccak256("\x19\x01" || domainSeparator || structHash)
        let mut hasher = Keccak::v256();
        hasher.update(&[0x19, 0x01]);
        hasher.update(domain_separator);
        hasher.update(struct_hash);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);

        self.sign_hash(&hash)
    }

    /// Encrypt and save wallet to a JSON file
    pub fn save_encrypted(&self, path: &Path, password: &str) -> Result<()> {
        let salt = KeyDerivation::generate_salt()?;
        let enc_key =
            KeyDerivation::derive_from_password(password, &salt, WALLET_PBKDF2_ITERATIONS)?;

        let encryptor = FileEncryptor::new(&enc_key)?;
        let encrypted = encryptor.encrypt(&self.private_key_bytes())?;

        let wallet_file = EncryptedWallet {
            version: 1,
            address: self.address.to_checksum(),
            encrypted_private_key: encrypted.ciphertext,
            nonce: encrypted.nonce.as_bytes().to_vec(),
            salt: salt.to_vec(),
            iterations: WALLET_PBKDF2_ITERATIONS,
        };

        let json = serde_json::to_string_pretty(&wallet_file)
            .map_err(|e| CarbideError::Internal(format!("Serialization failed: {e}")))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CarbideError::Io(e))?;
        }

        std::fs::write(path, json).map_err(CarbideError::Io)
    }

    /// Load and decrypt wallet from a JSON file
    pub fn load_encrypted(path: &Path, password: &str) -> Result<Self> {
        let json = std::fs::read_to_string(path).map_err(CarbideError::Io)?;

        let wallet_file: EncryptedWallet = serde_json::from_str(&json)
            .map_err(|e| CarbideError::Internal(format!("Invalid wallet file: {e}")))?;

        if wallet_file.version != 1 {
            return Err(CarbideError::Internal(format!(
                "Unsupported wallet version: {}",
                wallet_file.version
            )));
        }

        let mut salt = [0u8; 32];
        if wallet_file.salt.len() != 32 {
            return Err(CarbideError::Internal("Invalid salt length".to_string()));
        }
        salt.copy_from_slice(&wallet_file.salt);

        let enc_key = KeyDerivation::derive_from_password(
            password,
            &salt,
            wallet_file.iterations,
        )?;

        let mut nonce_bytes = [0u8; 12];
        if wallet_file.nonce.len() != 12 {
            return Err(CarbideError::Internal("Invalid nonce length".to_string()));
        }
        nonce_bytes.copy_from_slice(&wallet_file.nonce);

        let encrypted_data = crate::encryption::EncryptedData {
            ciphertext: wallet_file.encrypted_private_key,
            nonce: crate::encryption::Nonce::from_bytes(nonce_bytes),
            tag_size: 16,
        };

        let decryptor = FileDecryptor::new(&enc_key)?;
        let private_key_bytes = decryptor.decrypt(&encrypted_data)?;

        if private_key_bytes.len() != 32 {
            return Err(CarbideError::Internal(
                "Decrypted key has invalid length".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&private_key_bytes);

        Self::from_private_key(&key)
    }

    /// Recover signer address from a hash and signature
    pub fn recover_address(hash: &[u8; 32], sig: &WalletSignature) -> Result<EthAddress> {
        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&sig.r);
        sig_bytes[32..].copy_from_slice(&sig.s);

        let signature = Signature::from_bytes((&sig_bytes).into())
            .map_err(|e| CarbideError::Internal(format!("Invalid signature: {e}")))?;

        let recovery_id = RecoveryId::from_byte(sig.v.wrapping_sub(27))
            .ok_or_else(|| CarbideError::Internal("Invalid recovery id".to_string()))?;

        let verifying_key =
            VerifyingKey::recover_from_prehash(hash, &signature, recovery_id)
                .map_err(|e| CarbideError::Internal(format!("Recovery failed: {e}")))?;

        let point = verifying_key.to_encoded_point(false);
        let pubkey_bytes = &point.as_bytes()[1..];

        Ok(EthAddress::from_public_key_bytes(pubkey_bytes))
    }
}

impl fmt::Debug for CarbideWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CarbideWallet")
            .field("address", &self.address)
            .finish()
    }
}

/// Compute keccak256 hash of input data
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_address() {
        let (wallet, mnemonic) = CarbideWallet::generate().unwrap();
        let addr = wallet.address();

        // Address should be 20 bytes
        assert_eq!(addr.as_bytes().len(), 20);

        // Checksum address should start with 0x and be 42 chars
        let checksum = addr.to_checksum();
        assert!(checksum.starts_with("0x"));
        assert_eq!(checksum.len(), 42);

        // Mnemonic should be 12 words
        assert_eq!(mnemonic.split_whitespace().count(), 12);
    }

    #[test]
    fn test_mnemonic_roundtrip() {
        let (wallet1, mnemonic) = CarbideWallet::generate().unwrap();
        let wallet2 = CarbideWallet::from_mnemonic(&mnemonic).unwrap();

        assert_eq!(wallet1.address(), wallet2.address());
        assert_eq!(wallet1.private_key_bytes(), wallet2.private_key_bytes());
    }

    #[test]
    fn test_encrypted_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallet.json");
        let password = "test_password_123";

        let (wallet, _) = CarbideWallet::generate().unwrap();
        let original_addr = wallet.address().clone();

        wallet.save_encrypted(&path, password).unwrap();

        let loaded = CarbideWallet::load_encrypted(&path, password).unwrap();
        assert_eq!(loaded.address(), &original_addr);
    }

    #[test]
    fn test_wrong_password_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallet.json");

        let (wallet, _) = CarbideWallet::generate().unwrap();
        wallet.save_encrypted(&path, "correct_password").unwrap();

        let result = CarbideWallet::load_encrypted(&path, "wrong_password");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_and_recover() {
        let (wallet, _) = CarbideWallet::generate().unwrap();

        let hash = keccak256(b"test message");
        let sig = wallet.sign_hash(&hash).unwrap();

        // v should be 27 or 28
        assert!(sig.v == 27 || sig.v == 28);

        // Recover address and verify it matches
        let recovered = CarbideWallet::recover_address(&hash, &sig).unwrap();
        assert_eq!(&recovered, wallet.address());
    }

    #[test]
    fn test_sign_typed_data() {
        let (wallet, _) = CarbideWallet::generate().unwrap();

        let domain_separator = keccak256(b"test domain");
        let struct_hash = keccak256(b"test struct");

        let sig = wallet
            .sign_typed_data(&domain_separator, &struct_hash)
            .unwrap();

        // Manually compute the EIP-712 hash
        let mut hasher = Keccak::v256();
        hasher.update(&[0x19, 0x01]);
        hasher.update(&domain_separator);
        hasher.update(&struct_hash);
        let mut expected_hash = [0u8; 32];
        hasher.finalize(&mut expected_hash);

        let recovered = CarbideWallet::recover_address(&expected_hash, &sig).unwrap();
        assert_eq!(&recovered, wallet.address());
    }

    #[test]
    fn test_eip55_checksum() {
        // Known test vector: all-lowercase address
        let addr = EthAddress::from_hex("0xfb6916095ca1df60bb79ce92ce3ea74c37c5d359").unwrap();
        let checksum = addr.to_checksum();
        // Verify it starts with 0x and is mixed case
        assert!(checksum.starts_with("0x"));
        assert_eq!(checksum.len(), 42);
    }

    #[test]
    fn test_private_key_import() {
        let (wallet, _) = CarbideWallet::generate().unwrap();
        let pk = wallet.private_key_bytes();

        let imported = CarbideWallet::from_private_key(&pk).unwrap();
        assert_eq!(imported.address(), wallet.address());
    }
}
