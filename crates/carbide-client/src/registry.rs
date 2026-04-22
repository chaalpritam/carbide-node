//! On-chain provider registry reader.
//!
//! Talks to `CarbideRegistry.sol` directly over JSON-RPC, so clients can
//! discover providers even when the centralized `carbide-discovery-service`
//! is unreachable. Treat this as the trust root: the discovery service is a
//! convenience indexer, the chain is authoritative.

use std::sync::Arc;

use ethers::prelude::*;
use ethers::types::{Address, U256};
use rust_decimal::Decimal;
use tracing::{debug, info, warn};
use uuid::Uuid;

use carbide_core::{
    CarbideError, Provider as CoreProvider, ProviderTier, Region, ReputationScore, Result,
};

use crate::discovery::{DiscoveryClient, MarketplaceProvider, MarketplaceQuery};

mod bindings {
    use ethers::prelude::abigen;

    // Parse the compiled artifact ABI directly; abigen's human-readable
    // parser mis-handles the `tuple[]` inside multi-value returns, so we
    // bake the solc-emitted JSON ABI into the build. Regenerate it from
    // `carbide-contracts/artifacts/.../CarbideRegistry.json` whenever the
    // contract surface changes.
    abigen!(CarbideRegistryContract, "./abi/CarbideRegistry.json");
}

use bindings::{CarbideRegistryContract, Provider as RegistryProvider};

/// How many entries to pull per `getProvidersPage` call.
const PAGE_SIZE: usize = 100;

/// USDC has 6 decimals; prices on-chain are in base units.
const USDC_DECIMALS: u32 = 6;

/// Read-only client for the on-chain provider registry.
#[derive(Clone)]
pub struct RegistryClient {
    contract: CarbideRegistryContract<ethers::providers::Provider<Http>>,
}

impl std::fmt::Debug for RegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegistryClient")
            .field("address", &self.contract.address())
            .finish()
    }
}

impl RegistryClient {
    /// Build a reader pointed at a deployed `CarbideRegistry` contract.
    pub fn new(rpc_url: &str, contract_address: Address) -> Result<Self> {
        let rpc = ethers::providers::Provider::<Http>::try_from(rpc_url).map_err(|e| {
            CarbideError::Internal(format!("invalid registry rpc url {rpc_url}: {e}"))
        })?;
        let contract = CarbideRegistryContract::new(contract_address, Arc::new(rpc));
        Ok(Self { contract })
    }

    /// Total number of currently registered providers.
    pub async fn provider_count(&self) -> Result<u64> {
        let count: U256 = self
            .contract
            .provider_count()
            .call()
            .await
            .map_err(|e| CarbideError::Discovery(format!("registry providerCount failed: {e}")))?;
        Ok(count.as_u64())
    }

    /// Fetch every provider entry. Internally paginates so very large
    /// registries do not blow past RPC response limits.
    pub async fn fetch_all_providers(&self) -> Result<Vec<CoreProvider>> {
        let total = self.provider_count().await?;
        if total == 0 {
            return Ok(Vec::new());
        }

        let mut out = Vec::with_capacity(total as usize);
        let mut offset: u64 = 0;
        while offset < total {
            let take = std::cmp::min(PAGE_SIZE as u64, total - offset);
            debug!("registry page offset={offset} limit={take}");

            let (owners, records): (Vec<Address>, Vec<RegistryProvider>) = self
                .contract
                .get_providers_page(U256::from(offset), U256::from(take))
                .call()
                .await
                .map_err(|e| {
                    CarbideError::Discovery(format!(
                        "registry getProvidersPage(offset={offset}, limit={take}) failed: {e}"
                    ))
                })?;

            for (owner, record) in owners.into_iter().zip(records.into_iter()) {
                match convert(owner, record) {
                    Ok(p) => out.push(p),
                    Err(e) => warn!("skipping registry entry for {owner:?}: {e}"),
                }
            }

            offset += take;
        }

        info!("registry: loaded {} providers from chain", out.len());
        Ok(out)
    }
}

/// Run a marketplace query against the discovery service first, falling
/// back to the on-chain registry if the service is unreachable or errors.
///
/// Filtering (region/tier/limit/min_reputation) is applied after the
/// chain fetch to match the discovery service's semantics. The on-chain
/// registry has no reputation data, so `min_reputation` always passes
/// through for the fallback path.
pub async fn search_providers_with_fallback(
    discovery: &DiscoveryClient,
    registry: &RegistryClient,
    query: MarketplaceQuery,
) -> Result<Vec<MarketplaceProvider>> {
    match discovery.search_providers(query.clone()).await {
        Ok(list) => Ok(list),
        Err(e) => {
            warn!("discovery service unavailable ({e}); falling back to on-chain registry");
            let providers = registry.fetch_all_providers().await?;
            Ok(filter_and_wrap(providers, &query))
        }
    }
}

fn filter_and_wrap(
    providers: Vec<CoreProvider>,
    query: &MarketplaceQuery,
) -> Vec<MarketplaceProvider> {
    let mut filtered: Vec<MarketplaceProvider> = providers
        .into_iter()
        .filter(|p| query.region.as_ref().map_or(true, |r| &p.region == r))
        .filter(|p| query.tier.as_ref().map_or(true, |t| &p.tier == t))
        .map(|provider| MarketplaceProvider {
            online: true,
            load: None,
            available_space: Some(provider.available_capacity),
            active_contracts: 0,
            last_seen: chrono::Utc::now(),
            provider,
        })
        .collect();

    if let Some(limit) = query.limit {
        filtered.truncate(limit);
    }
    filtered
}

fn convert(owner: Address, r: RegistryProvider) -> Result<CoreProvider> {
    if !r.active {
        return Err(CarbideError::Discovery(format!(
            "{owner:?} is marked inactive"
        )));
    }

    let tier = tier_from_u8(r.tier)?;
    let region = region_from_str(&r.region)?;
    let price = price_from_base_units(r.price_per_gb_month);
    let capacity_bytes = r.capacity_gb.saturating_mul(1_000_000_000);
    let last_seen = chrono::DateTime::<chrono::Utc>::from_timestamp(r.updated_at as i64, 0)
        .unwrap_or_else(chrono::Utc::now);

    Ok(CoreProvider {
        id: stable_provider_id(owner),
        name: format!("on-chain:{owner:#x}"),
        tier,
        region,
        endpoint: r.endpoint,
        available_capacity: capacity_bytes,
        total_capacity: capacity_bytes,
        price_per_gb_month: price,
        reputation: ReputationScore::new(),
        last_seen,
        metadata: std::collections::HashMap::from([
            ("source".to_string(), "carbide-registry".to_string()),
            ("chain_owner".to_string(), format!("{owner:#x}")),
            ("registered_at".to_string(), r.registered_at.to_string()),
        ]),
        wallet_address: Some(format!("{owner:#x}")),
    })
}

fn tier_from_u8(v: u8) -> Result<ProviderTier> {
    match v {
        0 => Ok(ProviderTier::Home),
        1 => Ok(ProviderTier::Professional),
        2 => Ok(ProviderTier::Enterprise),
        3 => Ok(ProviderTier::GlobalCDN),
        other => Err(CarbideError::Discovery(format!(
            "unknown tier {other} from registry"
        ))),
    }
}

fn region_from_str(s: &str) -> Result<Region> {
    // Case-insensitive. The contract stores whatever the provider wrote;
    // be forgiving across common spellings.
    match s.to_lowercase().as_str() {
        "northamerica" | "north_america" | "na" => Ok(Region::NorthAmerica),
        "europe" | "eu" => Ok(Region::Europe),
        "asia" | "ap" | "asiapacific" => Ok(Region::Asia),
        "southamerica" | "south_america" | "sa" => Ok(Region::SouthAmerica),
        "africa" | "af" => Ok(Region::Africa),
        "oceania" | "oc" => Ok(Region::Oceania),
        other => Err(CarbideError::Discovery(format!(
            "unknown region {other:?} from registry"
        ))),
    }
}

fn price_from_base_units(base_units: u128) -> Decimal {
    Decimal::from(base_units) / Decimal::new(10_i64.pow(USDC_DECIMALS), 0)
}

/// Derive a deterministic `ProviderId` from the owner address so the same
/// chain entry yields the same UUID across calls (indexing, dedupe, etc.).
/// Uses the first 16 bytes of the 20-byte address and stamps the v4 / RFC
/// 4122 variant bits so the result is a well-formed UUID.
fn stable_provider_id(owner: Address) -> Uuid {
    let bytes = owner.as_bytes();
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes[..16]);
    arr[6] = (arr[6] & 0x0f) | 0x40; // version 4
    arr[8] = (arr[8] & 0x3f) | 0x80; // RFC 4122 variant
    Uuid::from_bytes(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_mapping_round_trips() {
        assert_eq!(tier_from_u8(0).unwrap(), ProviderTier::Home);
        assert_eq!(tier_from_u8(1).unwrap(), ProviderTier::Professional);
        assert_eq!(tier_from_u8(2).unwrap(), ProviderTier::Enterprise);
        assert_eq!(tier_from_u8(3).unwrap(), ProviderTier::GlobalCDN);
        assert!(tier_from_u8(4).is_err());
    }

    #[test]
    fn region_mapping_is_forgiving() {
        assert_eq!(region_from_str("NorthAmerica").unwrap(), Region::NorthAmerica);
        assert_eq!(region_from_str("northamerica").unwrap(), Region::NorthAmerica);
        assert_eq!(region_from_str("EU").unwrap(), Region::Europe);
        assert!(region_from_str("MARS").is_err());
    }

    #[test]
    fn price_scales_by_usdc_decimals() {
        // 5_000 base units @ 6 decimals => 0.005
        assert_eq!(price_from_base_units(5_000), Decimal::new(5, 3));
        // 1_000_000 base units => 1.000
        assert_eq!(price_from_base_units(1_000_000), Decimal::new(1, 0));
    }

    #[test]
    fn stable_id_is_deterministic() {
        let a: Address = "0x0000000000000000000000000000000000000001".parse().unwrap();
        assert_eq!(stable_provider_id(a), stable_provider_id(a));
    }
}
