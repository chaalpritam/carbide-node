//! Client-side wallet wrapper around `carbide_crypto::wallet::CarbideWallet`.
//!
//! Adds filesystem lifecycle (create / load / import) and surfaces the
//! Solana base58 address + Ed25519 message signing for higher-level
//! payment flows.

use std::path::{Path, PathBuf};

use carbide_core::Result;
use carbide_crypto::wallet::{CarbideWallet, SolanaAddress, WalletSignature};

const WALLET_FILE: &str = "wallet.json";

/// Carbide client wallet with on-disk persistence.
pub struct ClientWallet {
    inner: CarbideWallet,
    path: PathBuf,
}

impl ClientWallet {
    /// Create a new wallet, encrypt it with `password`, and save to
    /// `wallet_dir/wallet.json`. Returns the wallet and its 12-word
    /// BIP-39 mnemonic backup phrase.
    pub fn create(wallet_dir: &Path, password: &str) -> Result<(Self, String)> {
        let (wallet, mnemonic) = CarbideWallet::generate()?;
        let path = wallet_dir.join(WALLET_FILE);
        wallet.save_encrypted(&path, password)?;
        Ok((
            Self {
                inner: wallet,
                path,
            },
            mnemonic,
        ))
    }

    /// Load an encrypted wallet from disk.
    pub fn load(wallet_path: &Path, password: &str) -> Result<Self> {
        let wallet = CarbideWallet::load_encrypted(wallet_path, password)?;
        Ok(Self {
            inner: wallet,
            path: wallet_path.to_path_buf(),
        })
    }

    /// Recover a wallet from a BIP-39 mnemonic phrase, encrypt it, and save.
    pub fn import_from_mnemonic(
        wallet_dir: &Path,
        mnemonic: &str,
        password: &str,
    ) -> Result<Self> {
        let wallet = CarbideWallet::from_mnemonic(mnemonic)?;
        let path = wallet_dir.join(WALLET_FILE);
        wallet.save_encrypted(&path, password)?;
        Ok(Self {
            inner: wallet,
            path,
        })
    }

    /// Public Solana address for this wallet.
    pub fn address(&self) -> SolanaAddress {
        self.inner.address()
    }

    /// Base58-encoded address string.
    pub fn address_base58(&self) -> String {
        self.inner.address().to_base58()
    }

    /// Sign an arbitrary message with the wallet's Ed25519 secret.
    pub fn sign_message(&self, message: &[u8]) -> WalletSignature {
        self.inner.sign_message(message)
    }

    /// Filesystem path of the wallet file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let password = "test_password_123";

        let (wallet, _mnemonic) = ClientWallet::create(dir.path(), password).unwrap();
        let original = wallet.address_base58();

        let path = dir.path().join(WALLET_FILE);
        let loaded = ClientWallet::load(&path, password).unwrap();
        assert_eq!(loaded.address_base58(), original);
    }

    #[test]
    fn import_from_mnemonic_matches_create() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let pw = "secure_pass_456";

        let (original, mnemonic) = ClientWallet::create(dir1.path(), pw).unwrap();
        let imported = ClientWallet::import_from_mnemonic(dir2.path(), &mnemonic, pw).unwrap();
        assert_eq!(imported.address_base58(), original.address_base58());
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let (wallet, _) = ClientWallet::create(dir.path(), "pw").unwrap();
        let message = b"deposit attestation";
        let sig = wallet.sign_message(message);
        assert!(CarbideWallet::verify(&wallet.address(), message, &sig));
    }
}
