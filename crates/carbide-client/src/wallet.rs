//! Wallet management for the Carbide client SDK
//!
//! Wraps `carbide_crypto::wallet::CarbideWallet` with filesystem lifecycle
//! (create, load, import) and exposes address/signing for payment flows.

use std::path::{Path, PathBuf};

use carbide_core::Result;
use carbide_crypto::wallet::{CarbideWallet, EthAddress, WalletSignature};

/// Default wallet file name within the wallet directory
const WALLET_FILE: &str = "wallet.json";

/// Client-side wallet with filesystem persistence.
pub struct ClientWallet {
    inner: CarbideWallet,
    path: PathBuf,
}

impl ClientWallet {
    /// Create a new wallet, encrypt it with `password`, and save to `wallet_dir`.
    ///
    /// Returns the wallet and the 12-word BIP-39 mnemonic backup phrase.
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

    /// Load an existing encrypted wallet from `wallet_path`.
    pub fn load(wallet_path: &Path, password: &str) -> Result<Self> {
        let wallet = CarbideWallet::load_encrypted(wallet_path, password)?;
        Ok(Self {
            inner: wallet,
            path: wallet_path.to_path_buf(),
        })
    }

    /// Import a wallet from a BIP-39 mnemonic phrase, encrypt it, and save.
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

    /// Get the wallet's Ethereum address.
    pub fn address(&self) -> &EthAddress {
        self.inner.address()
    }

    /// Get the EIP-55 checksummed hex address string.
    pub fn address_hex(&self) -> String {
        self.inner.address().to_checksum()
    }

    /// Sign EIP-712 typed data given a domain separator and struct hash.
    pub fn sign_typed_data(
        &self,
        domain_separator: &[u8; 32],
        struct_hash: &[u8; 32],
    ) -> Result<WalletSignature> {
        self.inner.sign_typed_data(domain_separator, struct_hash)
    }

    /// Get the filesystem path where this wallet is stored.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbide_crypto::wallet::keccak256;

    #[test]
    fn create_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let password = "test_password";

        let (wallet, _mnemonic) = ClientWallet::create(dir.path(), password).unwrap();
        let original_addr = wallet.address_hex();

        let wallet_path = dir.path().join(WALLET_FILE);
        let loaded = ClientWallet::load(&wallet_path, password).unwrap();
        assert_eq!(loaded.address_hex(), original_addr);
    }

    #[test]
    fn import_from_mnemonic_roundtrip() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let password = "secure_pass";

        let (original, mnemonic) = ClientWallet::create(dir1.path(), password).unwrap();

        let imported =
            ClientWallet::import_from_mnemonic(dir2.path(), &mnemonic, password).unwrap();

        assert_eq!(imported.address_hex(), original.address_hex());
    }

    #[test]
    fn sign_and_recover() {
        let dir = tempfile::tempdir().unwrap();
        let password = "pass123";

        let (wallet, _) = ClientWallet::create(dir.path(), password).unwrap();

        let domain = keccak256(b"test domain");
        let struct_hash = keccak256(b"test struct");

        let sig = wallet.sign_typed_data(&domain, &struct_hash).unwrap();

        // Compute expected EIP-712 digest
        use tiny_keccak::{Hasher, Keccak};
        let mut hasher = Keccak::v256();
        hasher.update(&[0x19, 0x01]);
        hasher.update(&domain);
        hasher.update(&struct_hash);
        let mut digest = [0u8; 32];
        hasher.finalize(&mut digest);

        let recovered = CarbideWallet::recover_address(&digest, &sig).unwrap();
        assert_eq!(&recovered, wallet.address());
    }
}
