//! On-chain provider registration for the Solana `carbide_registry` program.
//!
//! At boot we publish (or refresh) this provider's record so clients can
//! discover us without depending on the off-chain discovery service. The
//! flow is idempotent: if the PDA already exists we issue `update`, else
//! `register`. Failures here are non-fatal — the HTTP API still serves —
//! but we log loudly so operators notice.

use std::str::FromStr;
use std::sync::Arc;

use borsh::BorshSerialize;
use sha2::{Digest, Sha256};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature, Signer};
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use tracing::{debug, info, warn};

use carbide_core::{CarbideError, Result};

/// USDC has 6 decimals; the registry stores prices in base units.
pub const USDC_DECIMALS: u32 = 6;

/// Anchor 8-byte instruction discriminator: `sha256("global:<name>")[..8]`.
fn instruction_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}").as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

/// Fields the provider wants reflected in its on-chain record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredRegistration {
    /// Public endpoint clients reach (e.g., `https://host.example:8080`).
    pub endpoint: String,
    /// Region tag accepted by `carbide_registry` (e.g., `NorthAmerica`).
    pub region: String,
    /// 0 = Home, 1 = Professional, 2 = Enterprise, 3 = GlobalCDN.
    pub tier: u8,
    /// Advertised capacity in GB.
    pub capacity_gb: u64,
    /// Price per GB per month, USDC base units (6 decimals).
    pub price_per_gb_month_base_units: u64,
}

impl DesiredRegistration {
    /// Convert config-level values into on-chain form: tier name → u8,
    /// USD price → token base units. Returns an error for invalid input
    /// so it never leaves a half-built record.
    pub fn from_config(
        endpoint: String,
        region: String,
        tier_name: &str,
        capacity_gb: u64,
        price_per_gb_month_usd: f64,
    ) -> Result<Self> {
        let tier = tier_u8_from_name(tier_name)?;
        let price = usd_to_base_units(price_per_gb_month_usd)?;
        if capacity_gb == 0 {
            return Err(CarbideError::Internal(
                "cannot register with capacity_gb = 0".to_string(),
            ));
        }
        Ok(Self {
            endpoint,
            region,
            tier,
            capacity_gb,
            price_per_gb_month_base_units: price,
        })
    }
}

#[derive(BorshSerialize)]
struct RegisterArgs {
    endpoint: String,
    region: String,
    tier: u8,
    capacity_gb: u64,
    price_per_gb_month: u64,
}

#[derive(BorshSerialize)]
struct UpdateArgs {
    endpoint: String,
    region: String,
    tier: u8,
    capacity_gb: u64,
    price_per_gb_month: u64,
}

#[derive(BorshSerialize)]
struct SetActiveArgs {
    active: bool,
}

/// Signing client for the on-chain `carbide_registry` program.
pub struct RegistryWriter {
    rpc: Arc<RpcClient>,
    program_id: Pubkey,
    signer: Arc<Keypair>,
    pda: Pubkey,
}

impl std::fmt::Debug for RegistryWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryWriter")
            .field("program_id", &self.program_id)
            .field("signer", &self.signer.pubkey())
            .field("pda", &self.pda)
            .finish()
    }
}

impl RegistryWriter {
    /// Build a writer pointed at `program_id` with `signer` paying for
    /// transactions. Derives the provider's PDA up front so each
    /// instruction reuses it.
    pub fn new(rpc_url: &str, program_id: &str, signer: Keypair) -> Result<Self> {
        let program_id = Pubkey::from_str(program_id.trim()).map_err(|e| {
            CarbideError::Internal(format!("invalid program id {program_id:?}: {e}"))
        })?;
        let rpc = Arc::new(RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));
        let signer = Arc::new(signer);
        let (pda, _bump) =
            Pubkey::find_program_address(&[b"provider", signer.pubkey().as_ref()], &program_id);
        Ok(Self {
            rpc,
            program_id,
            signer,
            pda,
        })
    }

    /// Wallet pubkey of the provider — also their PDA seed.
    pub fn signer_address(&self) -> Pubkey {
        self.signer.pubkey()
    }

    /// PDA address of the provider's on-chain record.
    pub fn provider_pda(&self) -> Pubkey {
        self.pda
    }

    /// Publish or refresh the provider record so it matches `desired`.
    /// Returns `true` if a transaction was sent, `false` if the chain
    /// already had the right state (idempotent).
    pub async fn ensure_registered(&self, desired: &DesiredRegistration) -> Result<bool> {
        match self.rpc.get_account(&self.pda).await {
            Ok(_) => {
                debug!(pda = %self.pda, "registry: PDA exists, sending update");
                self.send_update(desired).await?;
                Ok(true)
            }
            Err(e) if e.to_string().contains("AccountNotFound") => {
                info!(pda = %self.pda, "registry: publishing new provider entry");
                self.send_register(desired).await?;
                Ok(true)
            }
            Err(e) => Err(CarbideError::Internal(format!(
                "registry: failed to query PDA state: {e}"
            ))),
        }
    }

    /// Flip the active flag on the provider's PDA.
    pub async fn set_active(&self, active: bool) -> Result<Signature> {
        let mut data = instruction_discriminator("set_active").to_vec();
        SetActiveArgs { active }
            .serialize(&mut data)
            .map_err(|e| CarbideError::Internal(format!("borsh serialize: {e}")))?;

        let ix = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.signer.pubkey(), true),
                AccountMeta::new(self.pda, false),
            ],
            data,
        };
        self.send(ix).await
    }

    async fn send_register(&self, d: &DesiredRegistration) -> Result<Signature> {
        let mut data = instruction_discriminator("register").to_vec();
        RegisterArgs {
            endpoint: d.endpoint.clone(),
            region: d.region.clone(),
            tier: d.tier,
            capacity_gb: d.capacity_gb,
            price_per_gb_month: d.price_per_gb_month_base_units,
        }
        .serialize(&mut data)
        .map_err(|e| CarbideError::Internal(format!("borsh serialize: {e}")))?;

        let ix = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.signer.pubkey(), true),
                AccountMeta::new(self.pda, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data,
        };
        self.send(ix).await
    }

    async fn send_update(&self, d: &DesiredRegistration) -> Result<Signature> {
        let mut data = instruction_discriminator("update").to_vec();
        UpdateArgs {
            endpoint: d.endpoint.clone(),
            region: d.region.clone(),
            tier: d.tier,
            capacity_gb: d.capacity_gb,
            price_per_gb_month: d.price_per_gb_month_base_units,
        }
        .serialize(&mut data)
        .map_err(|e| CarbideError::Internal(format!("borsh serialize: {e}")))?;

        let ix = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(self.signer.pubkey(), true),
                AccountMeta::new(self.pda, false),
            ],
            data,
        };
        self.send(ix).await
    }

    async fn send(&self, ix: Instruction) -> Result<Signature> {
        let recent_blockhash = self
            .rpc
            .get_latest_blockhash()
            .await
            .map_err(|e| CarbideError::Internal(format!("get_latest_blockhash: {e}")))?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer.pubkey()),
            &[&*self.signer],
            recent_blockhash,
        );

        self.rpc
            .send_and_confirm_transaction(&tx)
            .await
            .map_err(|e| CarbideError::Internal(format!("send_and_confirm_transaction: {e}")))
    }
}

/// Spawn-friendly: run `ensure_registered` and log the outcome. Used at
/// provider boot so a transient RPC error doesn't kill the node.
pub async fn run_auto_register(writer: Arc<RegistryWriter>, desired: DesiredRegistration) {
    match writer.ensure_registered(&desired).await {
        Ok(true) => info!(
            signer = %writer.signer_address(),
            pda = %writer.provider_pda(),
            "registry: on-chain entry in sync"
        ),
        Ok(false) => debug!(
            signer = %writer.signer_address(),
            "registry: chain state already matches; no tx sent"
        ),
        Err(e) => warn!(
            "registry: failed to publish on-chain entry ({e}); provider \
             will still run but clients relying on the registry will not see it"
        ),
    }
}

fn tier_u8_from_name(name: &str) -> Result<u8> {
    match name.trim().to_lowercase().as_str() {
        "home" => Ok(0),
        "professional" | "pro" => Ok(1),
        "enterprise" => Ok(2),
        "globalcdn" | "global_cdn" | "cdn" => Ok(3),
        other => Err(CarbideError::Internal(format!(
            "unknown tier {other:?}; expected Home|Professional|Enterprise|GlobalCDN"
        ))),
    }
}

fn usd_to_base_units(usd: f64) -> Result<u64> {
    if !usd.is_finite() || usd < 0.0 {
        return Err(CarbideError::Internal(format!(
            "price_per_gb_month_usd must be a non-negative finite number, got {usd}"
        )));
    }
    let scale = 10_u64.pow(USDC_DECIMALS);
    let scaled = (usd * scale as f64).round();
    if scaled < 0.0 || scaled > u64::MAX as f64 {
        return Err(CarbideError::Internal(format!(
            "price {usd} out of range for u64 base units"
        )));
    }
    Ok(scaled as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_discriminator_is_first_eight_sha256_bytes() {
        let bytes = instruction_discriminator("register");
        let mut hasher = Sha256::new();
        hasher.update(b"global:register");
        let expected = &hasher.finalize()[..8];
        assert_eq!(&bytes[..], expected);
    }

    #[test]
    fn tier_strings_map_to_u8() {
        assert_eq!(tier_u8_from_name("Home").unwrap(), 0);
        assert_eq!(tier_u8_from_name("professional").unwrap(), 1);
        assert_eq!(tier_u8_from_name("Pro").unwrap(), 1);
        assert_eq!(tier_u8_from_name("Enterprise").unwrap(), 2);
        assert_eq!(tier_u8_from_name("GlobalCDN").unwrap(), 3);
        assert_eq!(tier_u8_from_name("cdn").unwrap(), 3);
        assert!(tier_u8_from_name("wat").is_err());
    }

    #[test]
    fn price_converts_to_usdc_base_units() {
        assert_eq!(usd_to_base_units(0.005).unwrap(), 5_000);
        assert_eq!(usd_to_base_units(1.0).unwrap(), 1_000_000);
        assert_eq!(usd_to_base_units(0.0).unwrap(), 0);
    }

    #[test]
    fn price_rejects_nan_and_negative() {
        assert!(usd_to_base_units(f64::NAN).is_err());
        assert!(usd_to_base_units(-0.001).is_err());
    }

    #[test]
    fn desired_from_config_happy_path() {
        let d = DesiredRegistration::from_config(
            "https://x.example:8080".to_string(),
            "NorthAmerica".to_string(),
            "Home",
            100,
            0.005,
        )
        .unwrap();
        assert_eq!(d.tier, 0);
        assert_eq!(d.capacity_gb, 100);
        assert_eq!(d.price_per_gb_month_base_units, 5_000);
    }

    #[test]
    fn desired_from_config_rejects_zero_capacity() {
        let err = DesiredRegistration::from_config(
            "https://x.example:8080".to_string(),
            "NorthAmerica".to_string(),
            "Home",
            0,
            0.005,
        )
        .unwrap_err();
        match err {
            CarbideError::Internal(msg) => assert!(msg.contains("capacity_gb")),
            other => panic!("expected Internal, got {other:?}"),
        }
    }
}
