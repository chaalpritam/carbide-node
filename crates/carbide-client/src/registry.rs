//! On-chain provider registry reader.
//!
//! Talks to the `carbide_registry` Solana program directly so clients can
//! discover providers without depending on the centralised discovery
//! service. The chain is the trust root; the discovery service is just a
//! fast cache.
//!
//! Each `ProviderAccount` is a PDA (`[b"provider", owner_pubkey]`) carrying
//! the provider's advertised endpoint, region, tier, capacity, and price.
//! We fetch the whole set with a single `getProgramAccounts` call and
//! borsh-decode the bodies after the 8-byte Anchor discriminator.

use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::account::Account;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;

use carbide_core::{CarbideError, Result};

/// USDC on Solana has 6 decimals; on-chain prices are stored in base units.
pub const USDC_DECIMALS: u32 = 6;

/// Anchor 8-byte discriminator for `account:ProviderAccount`.
fn provider_account_discriminator() -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"account:ProviderAccount");
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

/// Borsh layout mirroring `carbide_registry::ProviderAccount` (sans the
/// 8-byte discriminator, which we strip before decoding).
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct ProviderRecord {
    /// Owner pubkey (Solana address that registered).
    pub owner: [u8; 32],
    /// Public endpoint clients reach (e.g., `https://host.example:8080`).
    pub endpoint: String,
    /// Region tag advertised on-chain.
    pub region: String,
    /// Price per GB per month in token base units (USDC = 6 decimals).
    pub price_per_gb_month: u64,
    /// Advertised capacity in GB.
    pub capacity_gb: u64,
    /// Unix timestamp of the original `register` call.
    pub registered_at: i64,
    /// Unix timestamp of the most recent update.
    pub updated_at: i64,
    /// Tier index (0=Home, 1=Professional, 2=Enterprise, 3=GlobalCDN).
    pub tier: u8,
    /// Active flag — providers can pause without deregistering.
    pub active: bool,
    /// PDA bump (kept by the program for cheap signer derivation).
    pub bump: u8,
}

impl ProviderRecord {
    /// Render the owner as a base58 address.
    pub fn owner_base58(&self) -> String {
        bs58::encode(self.owner).into_string()
    }
}

/// Read-only client for the on-chain `carbide_registry` program.
pub struct RegistryClient {
    rpc: RpcClient,
    program_id: Pubkey,
}

impl RegistryClient {
    /// Build a client targeting `program_id` over `rpc_url`.
    pub fn new(rpc_url: &str, program_id: &str) -> Result<Self> {
        let program_id = Pubkey::from_str(program_id.trim()).map_err(|e| {
            CarbideError::Internal(format!("invalid program id {program_id:?}: {e}"))
        })?;
        let rpc = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );
        Ok(Self { rpc, program_id })
    }

    /// The registry program ID this client is bound to.
    pub fn program_id(&self) -> Pubkey {
        self.program_id
    }

    /// Fetch every `ProviderAccount` currently held by the program.
    pub fn fetch_all_providers(&self) -> Result<Vec<ProviderRecord>> {
        let disc = provider_account_discriminator();
        let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, disc.to_vec()))];
        let config = RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: RpcAccountInfoConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };

        let accounts = self
            .rpc
            .get_program_accounts_with_config(&self.program_id, config)
            .map_err(|e| {
                CarbideError::Discovery(format!("getProgramAccounts failed: {e}"))
            })?;

        let mut records = Vec::with_capacity(accounts.len());
        for (pda, account) in accounts {
            match decode_provider(&account) {
                Ok(record) => records.push(record),
                Err(e) => tracing::warn!(%pda, error = %e, "skipping malformed provider account"),
            }
        }
        Ok(records)
    }

    /// Fetch a single provider by their owner pubkey, deriving the PDA
    /// (`[b"provider", owner]`) under the registry program ID.
    pub fn fetch_provider(&self, owner: &Pubkey) -> Result<Option<ProviderRecord>> {
        let (pda, _bump) = Pubkey::find_program_address(
            &[b"provider", owner.as_ref()],
            &self.program_id,
        );
        match self.rpc.get_account(&pda) {
            Ok(account) => Ok(Some(decode_provider(&account)?)),
            Err(e) if e.to_string().contains("AccountNotFound") => Ok(None),
            Err(e) => Err(CarbideError::Discovery(format!("getAccountInfo failed: {e}"))),
        }
    }
}

fn decode_provider(account: &Account) -> Result<ProviderRecord> {
    let data = &account.data;
    if data.len() < 8 {
        return Err(CarbideError::Discovery(
            "provider account data shorter than 8-byte discriminator".to_string(),
        ));
    }
    if data[..8] != provider_account_discriminator() {
        return Err(CarbideError::Discovery(
            "discriminator does not match ProviderAccount".to_string(),
        ));
    }
    ProviderRecord::try_from_slice(&data[8..])
        .map_err(|e| CarbideError::Discovery(format!("borsh decode failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminator_is_first_eight_sha256_bytes() {
        let bytes = provider_account_discriminator();
        // Computed independently for "account:ProviderAccount":
        // sha256("account:ProviderAccount")[0..8]
        let mut hasher = Sha256::new();
        hasher.update(b"account:ProviderAccount");
        let expected = &hasher.finalize()[..8];
        assert_eq!(&bytes[..], expected);
    }

    #[test]
    fn provider_record_decodes_round_trip() {
        // Build a synthetic on-chain account body and confirm we
        // round-trip it back to ProviderRecord.
        let original = ProviderRecord {
            owner: [7u8; 32],
            endpoint: "https://example:8080".to_string(),
            region: "Europe".to_string(),
            price_per_gb_month: 5_000,
            capacity_gb: 250,
            registered_at: 1_700_000_000,
            updated_at: 1_700_000_500,
            tier: 1,
            active: true,
            bump: 254,
        };

        let buf = borsh::to_vec(&original).unwrap();
        let mut data = provider_account_discriminator().to_vec();
        data.extend_from_slice(&buf);

        let account = Account {
            lamports: 0,
            data,
            owner: Pubkey::default(),
            executable: false,
            rent_epoch: 0,
        };
        let decoded = decode_provider(&account).unwrap();
        assert_eq!(decoded.owner_base58(), original.owner_base58());
        assert_eq!(decoded.endpoint, original.endpoint);
        assert_eq!(decoded.tier, original.tier);
        assert_eq!(decoded.active, original.active);
    }
}
