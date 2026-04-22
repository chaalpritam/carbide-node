//! On-chain provider registration.
//!
//! Publishes this provider's endpoint, tier, region, capacity, and price to
//! `CarbideRegistry.sol` at boot so clients can discover it without depending
//! on any off-chain indexer. Idempotent: on restart it calls `update` instead
//! of `register`, and refreshes `setActive(true)` if the entry was previously
//! deactivated.

use std::str::FromStr;
use std::sync::Arc;

use ethers::middleware::SignerMiddleware;
use ethers::prelude::*;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::Address;
use tracing::{debug, info, warn};

use carbide_core::{CarbideError, Result};

mod bindings {
    use ethers::prelude::abigen;

    // Reuse the solc-emitted ABI that carbide-client bakes in. The provider
    // only calls write methods + isRegistered, all with primitive arg/return
    // types, but loading the full JSON keeps the two crates in sync.
    abigen!(CarbideRegistryContract, "./abi/CarbideRegistry.json");
}

use bindings::CarbideRegistryContract;

/// USDC has 6 decimals; the registry stores prices in base units.
const USDC_DECIMALS: u32 = 6;

/// Bytes-per-GB multiplier used when advertising capacity. Matches the
/// carbide-client reader so round-tripping preserves the original GB value.
const BYTES_PER_GB: u64 = 1_000_000_000;

/// Fields the provider wants reflected in its on-chain record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredRegistration {
    /// Public endpoint clients reach (e.g., `https://host.example:8080`).
    pub endpoint: String,
    /// Region tag accepted by `CarbideRegistry` (e.g., `NorthAmerica`).
    pub region: String,
    /// 0 = Home, 1 = Professional, 2 = Enterprise, 3 = GlobalCDN.
    pub tier: u8,
    /// Advertised capacity in GB.
    pub capacity_gb: u64,
    /// Price per GB per month, USDC base units (6 decimals).
    pub price_per_gb_month_base_units: u128,
}

impl DesiredRegistration {
    /// Convert the provider's in-app config fields into on-chain form.
    ///
    /// `tier_name` accepts the strings stored in `ProviderConfig.provider.tier`
    /// and is case-insensitive. `price_per_gb_month_usd` is the human price
    /// (e.g., `0.005`); we convert to base units here so the caller doesn't
    /// have to know about USDC decimals.
    pub fn from_config(
        endpoint: String,
        region: String,
        tier_name: &str,
        capacity_gb: u64,
        price_per_gb_month_usd: f64,
    ) -> Result<Self> {
        let tier = tier_u8_from_name(tier_name)?;
        let price_base = usd_to_base_units(price_per_gb_month_usd)?;
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
            price_per_gb_month_base_units: price_base,
        })
    }
}

/// Signed client bound to a specific `CarbideRegistry` deployment.
pub struct RegistryWriter {
    contract: CarbideRegistryContract<SignerMiddleware<Provider<Http>, LocalWallet>>,
    address: Address,
}

impl std::fmt::Debug for RegistryWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryWriter")
            .field("contract", &self.contract.address())
            .field("signer", &self.address)
            .finish()
    }
}

impl RegistryWriter {
    /// Build a writer from an RPC URL, chain id, signer private key, and
    /// the deployed `CarbideRegistry` address.
    pub fn new(
        rpc_url: &str,
        chain_id: u64,
        signer_private_key: &[u8; 32],
        registry_address: Address,
    ) -> Result<Self> {
        let rpc = Provider::<Http>::try_from(rpc_url).map_err(|e| {
            CarbideError::Internal(format!("invalid rpc url {rpc_url}: {e}"))
        })?;

        let wallet = LocalWallet::from_bytes(signer_private_key)
            .map_err(|e| CarbideError::Internal(format!("invalid signing key: {e}")))?
            .with_chain_id(chain_id);
        let address = wallet.address();

        let client = Arc::new(SignerMiddleware::new(rpc, wallet));
        let contract = CarbideRegistryContract::new(registry_address, client);

        Ok(Self { contract, address })
    }

    /// Ethereum address of the signer (this provider's on-chain identity).
    pub fn signer_address(&self) -> Address {
        self.address
    }

    /// Ensure an active entry exists for this signer that matches `desired`.
    ///
    /// - First call: emits `register`.
    /// - Subsequent calls: emits `update`, then `setActive(true)` if the
    ///   entry was deactivated previously.
    /// - Returns `true` if any transaction was actually sent so callers can
    ///   log/metric it.
    pub async fn ensure_registered(&self, desired: &DesiredRegistration) -> Result<bool> {
        let is_registered: bool = self
            .contract
            .is_registered(self.address)
            .call()
            .await
            .map_err(|e| {
                CarbideError::Internal(format!("registry isRegistered query failed: {e}"))
            })?;

        if is_registered {
            debug!("registry: entry already exists, issuing update");
            self.send_update(desired).await?;
            // setActive is idempotent in the contract (no-op if unchanged),
            // so calling it here keeps providers from getting stuck inactive
            // if they deregistered and re-registered.
            self.send_set_active(true).await?;
            Ok(true)
        } else {
            info!("registry: publishing new entry for {:?}", self.address);
            self.send_register(desired).await?;
            Ok(true)
        }
    }

    /// Flip the on-chain active flag to `false`. Use on graceful shutdown to
    /// stop clients from routing new uploads here while keeping the entry
    /// around for later re-activation.
    pub async fn deactivate(&self) -> Result<()> {
        self.send_set_active(false).await
    }

    async fn send_register(&self, d: &DesiredRegistration) -> Result<()> {
        let tx = self.contract.register(
            d.endpoint.clone(),
            d.region.clone(),
            d.tier,
            d.capacity_gb,
            d.price_per_gb_month_base_units,
        );
        let pending = tx.send().await.map_err(|e| {
            CarbideError::Internal(format!("registry register() send failed: {e}"))
        })?;
        let receipt = pending.await.map_err(|e| {
            CarbideError::Internal(format!("registry register() receipt failed: {e}"))
        })?;
        info!(
            "registry: register tx mined tx_hash={:?} block={:?}",
            receipt.as_ref().map(|r| r.transaction_hash),
            receipt.as_ref().map(|r| r.block_number)
        );
        Ok(())
    }

    async fn send_update(&self, d: &DesiredRegistration) -> Result<()> {
        let tx = self.contract.update(
            d.endpoint.clone(),
            d.region.clone(),
            d.tier,
            d.capacity_gb,
            d.price_per_gb_month_base_units,
        );
        let pending = tx.send().await.map_err(|e| {
            CarbideError::Internal(format!("registry update() send failed: {e}"))
        })?;
        let receipt = pending.await.map_err(|e| {
            CarbideError::Internal(format!("registry update() receipt failed: {e}"))
        })?;
        debug!(
            "registry: update tx mined tx_hash={:?}",
            receipt.as_ref().map(|r| r.transaction_hash)
        );
        Ok(())
    }

    async fn send_set_active(&self, active: bool) -> Result<()> {
        let tx = self.contract.set_active(active);
        let pending = tx.send().await.map_err(|e| {
            CarbideError::Internal(format!("registry setActive() send failed: {e}"))
        })?;
        let _ = pending.await.map_err(|e| {
            CarbideError::Internal(format!("registry setActive() receipt failed: {e}"))
        })?;
        Ok(())
    }
}

/// Spawn-friendly helper: run `ensure_registered` and log outcome. Intended
/// to be called from provider boot; we swallow the error path so a broken
/// registry doesn't take the whole node down.
pub async fn run_auto_register(writer: Arc<RegistryWriter>, desired: DesiredRegistration) {
    match writer.ensure_registered(&desired).await {
        Ok(_) => info!("registry: on-chain entry in sync for {:?}", writer.signer_address()),
        Err(e) => warn!(
            "registry: failed to publish on-chain entry ({e}); \
             provider will still run but clients relying on the registry \
             will not see it"
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

fn usd_to_base_units(usd: f64) -> Result<u128> {
    if !usd.is_finite() || usd < 0.0 {
        return Err(CarbideError::Internal(format!(
            "price_per_gb_month_usd must be a non-negative finite number, got {usd}"
        )));
    }
    let scale = 10_u128.pow(USDC_DECIMALS);
    // USD values in this project are always quoted to at most 4 decimals; a
    // rounded conversion avoids float drift producing e.g. 4999 vs 5000.
    let scaled = (usd * scale as f64).round();
    if scaled < 0.0 || scaled > u128::MAX as f64 {
        return Err(CarbideError::Internal(format!(
            "price {usd} out of range for uint128 base units"
        )));
    }
    Ok(scaled as u128)
}

/// Parse a `0x…` hex address; used by callers that pull the registry
/// address out of config strings.
pub fn parse_address(s: &str) -> Result<Address> {
    Address::from_str(s.trim())
        .map_err(|e| CarbideError::Internal(format!("invalid registry address {s:?}: {e}")))
}

#[allow(dead_code)]
fn _bytes_per_gb_unused() {
    // Silences dead-code on BYTES_PER_GB; kept exported as a doc constant
    // for the capacity helper above.
    let _ = BYTES_PER_GB;
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parse_address_accepts_standard_forms() {
        let a = parse_address("0x0000000000000000000000000000000000000001").unwrap();
        assert_eq!(a, Address::from_low_u64_be(1));
        assert!(parse_address("not-an-address").is_err());
    }
}
