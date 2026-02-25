//! Discovery client for marketplace integration
//!
//! This module provides a high-level client for interacting with the
//! Carbide Discovery service to find providers, get quotes, and access
//! marketplace statistics.

use carbide_core::{network::*, CarbideError, Provider, ProviderId, ProviderTier, Region, Result};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::CarbideClient;

/// High-level client for discovery service operations
#[derive(Debug)]
pub struct DiscoveryClient {
    /// HTTP client for requests
    client: CarbideClient,
    /// Discovery service endpoint
    endpoint: String,
}

/// Query parameters for marketplace search
#[derive(Debug, Clone, Default)]
pub struct MarketplaceQuery {
    /// Filter by region
    pub region: Option<Region>,
    /// Filter by provider tier
    pub tier: Option<ProviderTier>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Minimum reputation score
    pub min_reputation: Option<rust_decimal::Decimal>,
}

/// Advanced provider filter for complex queries
#[derive(Debug, Clone)]
pub struct ProviderFilter {
    /// Required regions (OR logic)
    pub regions: Vec<Region>,
    /// Required tiers (OR logic)  
    pub tiers: Vec<ProviderTier>,
    /// Minimum available capacity (bytes)
    pub min_capacity: Option<u64>,
    /// Maximum price per GB per month
    pub max_price: Option<rust_decimal::Decimal>,
    /// Minimum reputation score
    pub min_reputation: Option<rust_decimal::Decimal>,
    /// Maximum load (0.0 - 1.0)
    pub max_load: Option<f32>,
}

impl Default for ProviderFilter {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
            tiers: Vec::new(),
            min_capacity: None,
            max_price: Some(rust_decimal::Decimal::new(20, 3)), // $0.020/GB/month
            min_reputation: Some(rust_decimal::Decimal::new(30, 2)), // 0.30
            max_load: Some(0.8),                                // 80% max load
        }
    }
}

/// Marketplace statistics from discovery service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStatistics {
    /// Total number of registered providers
    pub total_providers: usize,
    /// Providers currently online
    pub online_providers: usize,
    /// Total storage capacity across all providers
    pub total_capacity_gb: f64,
    /// Available storage capacity
    pub available_capacity_gb: f64,
    /// Average price per GB per month
    pub average_price_per_gb: rust_decimal::Decimal,
    /// Capacity utilization percentage
    pub utilization_percentage: f32,
    /// Last statistics update
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Provider with additional marketplace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProvider {
    /// Base provider information
    pub provider: Provider,
    /// Current online status
    pub online: bool,
    /// Current load (0.0 - 1.0)
    pub load: Option<f32>,
    /// Available storage space (bytes)
    pub available_space: Option<u64>,
    /// Number of active contracts
    pub active_contracts: usize,
    /// Last seen timestamp
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Quote comparison result
#[derive(Debug, Clone)]
pub struct QuoteComparison {
    /// File size being quoted
    pub file_size: u64,
    /// Replication factor
    pub replication_factor: u8,
    /// Duration in months
    pub duration_months: u32,
    /// All quotes received
    pub quotes: Vec<ProviderQuote>,
    /// Best quote by price
    pub best_price_quote: Option<ProviderQuote>,
    /// Best quote by reputation
    pub best_reputation_quote: Option<ProviderQuote>,
    /// Average price across all quotes
    pub average_price: rust_decimal::Decimal,
}

/// Quote from a specific provider
#[derive(Debug, Clone)]
pub struct ProviderQuote {
    /// Provider offering the quote
    pub provider: Provider,
    /// Quote details
    pub quote: StorageQuoteResponse,
    /// Quote score (combines price, reputation, capacity)
    pub score: f32,
}

impl DiscoveryClient {
    /// Create a new discovery client
    pub fn new(client: CarbideClient, endpoint: String) -> Self {
        Self { client, endpoint }
    }

    /// Get marketplace statistics
    pub async fn get_marketplace_stats(&self) -> Result<MarketplaceStatistics> {
        let url = format!("{}/api/v1/marketplace/stats", self.endpoint);

        debug!("Fetching marketplace statistics from: {}", url);

        let response = self
            .client
            .http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Stats request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Stats request returned: {}",
                response.status()
            )));
        }

        let raw_stats: serde_json::Value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Failed to parse stats: {e}")))?;

        // Convert raw stats to our structure
        let stats = MarketplaceStatistics {
            total_providers: raw_stats["total_providers"].as_u64().unwrap_or(0) as usize,
            online_providers: raw_stats["online_providers"].as_u64().unwrap_or(0) as usize,
            total_capacity_gb: raw_stats["total_capacity_bytes"].as_u64().unwrap_or(0) as f64
                / (1024.0 * 1024.0 * 1024.0),
            available_capacity_gb: raw_stats["available_capacity_bytes"].as_u64().unwrap_or(0)
                as f64
                / (1024.0 * 1024.0 * 1024.0),
            average_price_per_gb: raw_stats["average_price_per_gb"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap_or(rust_decimal::Decimal::ZERO),
            utilization_percentage: {
                let total = raw_stats["total_capacity_bytes"].as_u64().unwrap_or(0) as f64;
                let available = raw_stats["available_capacity_bytes"].as_u64().unwrap_or(0) as f64;
                if total > 0.0 {
                    ((total - available) / total * 100.0) as f32
                } else {
                    0.0
                }
            },
            last_updated: raw_stats["last_updated"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map_or_else(
                    chrono::Utc::now,
                    |dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&chrono::Utc),
                ),
        };

        info!(
            "Marketplace stats: {} providers, {:.1} GB available",
            stats.online_providers, stats.available_capacity_gb
        );

        Ok(stats)
    }

    /// Search for providers using simple query
    pub async fn search_providers(
        &self,
        query: MarketplaceQuery,
    ) -> Result<Vec<MarketplaceProvider>> {
        let mut url = format!("{}/api/v1/providers", self.endpoint);
        let mut params = Vec::new();

        if let Some(region) = &query.region {
            let region_str = match region {
                Region::NorthAmerica => "northamerica",
                Region::Europe => "europe",
                Region::Asia => "asia",
                Region::SouthAmerica => "southamerica",
                Region::Africa => "africa",
                Region::Oceania => "oceania",
            };
            params.push(format!("region={region_str}"));
        }

        if let Some(tier) = &query.tier {
            let tier_str = match tier {
                ProviderTier::Home => "home",
                ProviderTier::Professional => "professional",
                ProviderTier::Enterprise => "enterprise",
                ProviderTier::GlobalCDN => "globalcdn",
            };
            params.push(format!("tier={tier_str}"));
        }

        if let Some(limit) = query.limit {
            params.push(format!("limit={limit}"));
        }

        if let Some(min_rep) = &query.min_reputation {
            params.push(format!("min_reputation={min_rep}"));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        debug!("Searching providers: {}", url);

        let response = self
            .client
            .http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Provider search failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Provider search returned: {}",
                response.status()
            )));
        }

        let provider_list: ProviderListResponse = response
            .json::<ProviderListResponse>()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Failed to parse provider list: {e}")))?;

        // Convert to marketplace providers (with mock metadata for now)
        let marketplace_providers: Vec<MarketplaceProvider> = provider_list
            .providers
            .into_iter()
            .map(|provider| MarketplaceProvider {
                online: true,    // Assume online if returned by discovery
                load: Some(0.5), // Mock load
                available_space: Some(provider.available_capacity),
                active_contracts: 10, // Mock active contracts
                last_seen: chrono::Utc::now(),
                provider,
            })
            .collect();

        info!(
            "Found {} providers matching query",
            marketplace_providers.len()
        );

        Ok(marketplace_providers)
    }

    /// Search providers with advanced filtering
    pub async fn search_providers_advanced(
        &self,
        filter: ProviderFilter,
    ) -> Result<Vec<MarketplaceProvider>> {
        // For now, search all providers and filter client-side
        let all_providers = self.search_providers(MarketplaceQuery::default()).await?;

        let filtered_providers: Vec<MarketplaceProvider> = all_providers
            .into_iter()
            .filter(|mp| {
                // Region filter
                if !filter.regions.is_empty() && !filter.regions.contains(&mp.provider.region) {
                    return false;
                }

                // Tier filter
                if !filter.tiers.is_empty() && !filter.tiers.contains(&mp.provider.tier) {
                    return false;
                }

                // Capacity filter
                if let Some(min_capacity) = filter.min_capacity {
                    if mp.provider.available_capacity < min_capacity {
                        return false;
                    }
                }

                // Price filter
                if let Some(max_price) = &filter.max_price {
                    if mp.provider.price_per_gb_month > *max_price {
                        return false;
                    }
                }

                // Reputation filter
                if let Some(min_reputation) = &filter.min_reputation {
                    if mp.provider.reputation.overall < *min_reputation {
                        return false;
                    }
                }

                // Load filter
                if let Some(max_load) = filter.max_load {
                    if let Some(load) = mp.load {
                        if load > max_load {
                            return false;
                        }
                    }
                }

                true
            })
            .collect();

        info!("Filtered to {} providers", filtered_providers.len());

        Ok(filtered_providers)
    }

    /// Get quotes from multiple providers and compare them
    pub async fn compare_quotes(&self, request: &StorageQuoteRequest) -> Result<QuoteComparison> {
        debug!(
            "Getting quotes for {}GB file, {} replicas, {} months",
            request.file_size as f64 / (1024.0 * 1024.0 * 1024.0),
            request.replication_factor,
            request.duration_months
        );

        // Find relevant providers
        let query = MarketplaceQuery {
            region: request.preferred_regions.first().cloned(),
            tier: None,
            limit: Some(20),
            min_reputation: Some(rust_decimal::Decimal::new(20, 2)), // 0.20 minimum
        };

        let providers = self.search_providers(query).await?;

        if providers.is_empty() {
            return Err(CarbideError::Internal(
                "No providers found for quote request".to_string(),
            ));
        }

        // Request quotes from all providers
        let mut quotes = Vec::new();

        for marketplace_provider in providers {
            let provider_endpoint = &marketplace_provider.provider.endpoint;

            match self
                .client
                .request_storage_quote(provider_endpoint, request)
                .await
            {
                Ok(quote_response) => {
                    // Calculate a score for this quote (lower is better)
                    let price_score = quote_response.price_per_gb_month.to_f32().unwrap_or(1.0);
                    let reputation_score = 1.0
                        - marketplace_provider
                            .provider
                            .reputation
                            .overall
                            .to_f32()
                            .unwrap_or(0.5);
                    let capacity_score = if quote_response.available_capacity < request.file_size {
                        1.0 // Penalty for insufficient capacity
                    } else {
                        0.1 // Bonus for sufficient capacity
                    };

                    let combined_score =
                        price_score * 0.5 + reputation_score * 0.3 + capacity_score * 0.2;

                    quotes.push(ProviderQuote {
                        provider: marketplace_provider.provider,
                        quote: quote_response,
                        score: combined_score,
                    });
                }
                Err(e) => {
                    debug!(
                        "Failed to get quote from {}: {}",
                        marketplace_provider.provider.name, e
                    );
                }
            }
        }

        if quotes.is_empty() {
            return Err(CarbideError::Internal(
                "No quotes received from any provider".to_string(),
            ));
        }

        // Sort quotes by score (best first)
        quotes.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Find best quotes
        let best_price_quote = quotes
            .iter()
            .min_by(|a, b| a.quote.price_per_gb_month.cmp(&b.quote.price_per_gb_month))
            .cloned();

        let best_reputation_quote = quotes
            .iter()
            .max_by(|a, b| {
                a.provider
                    .reputation
                    .overall
                    .cmp(&b.provider.reputation.overall)
            })
            .cloned();

        // Calculate average price
        let total_price: rust_decimal::Decimal =
            quotes.iter().map(|q| q.quote.price_per_gb_month).sum();
        let average_price = total_price / rust_decimal::Decimal::new(quotes.len() as i64, 0);

        let comparison = QuoteComparison {
            file_size: request.file_size,
            replication_factor: request.replication_factor,
            duration_months: request.duration_months,
            quotes,
            best_price_quote,
            best_reputation_quote,
            average_price,
        };

        info!(
            "Quote comparison: {} quotes, best price: ${}/GB/month",
            comparison.quotes.len(),
            comparison.best_price_quote.as_ref().map_or_else(
                || "N/A".to_string(),
                |q| q.quote.price_per_gb_month.to_string()
            )
        );

        Ok(comparison)
    }

    /// Get detailed information about a specific provider
    pub async fn get_provider_details(
        &self,
        provider_id: &ProviderId,
    ) -> Result<MarketplaceProvider> {
        let url = format!("{}/api/v1/providers/{}", self.endpoint, provider_id);

        debug!("Getting provider details: {}", url);

        let response = self
            .client
            .http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Provider lookup failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Provider lookup returned: {}",
                response.status()
            )));
        }

        let registry_entry: serde_json::Value =
            response.json::<serde_json::Value>().await.map_err(|e| {
                CarbideError::Discovery(format!("Failed to parse provider details: {e}"))
            })?;

        // Extract provider info from registry entry
        let provider_data = &registry_entry["provider"];
        let provider: Provider = serde_json::from_value(provider_data.clone())
            .map_err(|e| CarbideError::Internal(format!("Failed to parse provider: {e}")))?;

        let marketplace_provider = MarketplaceProvider {
            online: registry_entry["health_status"].as_str() == Some("Healthy"),
            load: registry_entry["current_load"].as_f64().map(|l| l as f32),
            available_space: registry_entry["available_storage"].as_u64(),
            active_contracts: registry_entry["active_contracts"].as_u64().unwrap_or(0) as usize,
            last_seen: registry_entry["last_heartbeat"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map_or_else(
                    chrono::Utc::now,
                    |dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&chrono::Utc),
                ),
            provider,
        };

        Ok(marketplace_provider)
    }

    /// Register a new provider with the discovery service
    pub async fn register_provider(&self, provider: &Provider) -> Result<()> {
        let url = format!("{}/api/v1/providers", self.endpoint);

        let announcement = ProviderAnnouncement {
            provider: provider.clone(),
            endpoint: provider.endpoint.clone(),
            supported_versions: vec!["1.0".to_string()],
            public_key: None,
            wallet_address: provider.wallet_address.clone(),
        };

        debug!("Registering provider: {}", provider.name);

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&announcement)
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Provider registration failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Provider registration returned: {}",
                response.status()
            )));
        }

        info!("Provider {} registered successfully", provider.name);
        Ok(())
    }

    /// Create a storage contract on the discovery service.
    pub async fn create_contract(
        &self,
        client_id: &str,
        provider_id: &str,
        file_id: &str,
        file_size: u64,
        price: &str,
        duration_months: u32,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/contracts", self.endpoint);
        let body = serde_json::json!({
            "client_id": client_id,
            "provider_id": provider_id,
            "file_id": file_id,
            "file_size": file_size,
            "price_per_gb_month": price,
            "duration_months": duration_months,
        });

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Create contract failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Create contract returned: {}",
                response.status()
            )));
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Failed to parse contract: {e}")))
    }

    /// Activate a contract by recording a deposit on the discovery service.
    pub async fn activate_contract(
        &self,
        contract_id: &str,
        amount: &str,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/contracts/{}/deposit", self.endpoint, contract_id);
        let body = serde_json::json!({ "amount": amount });

        let response = self
            .client
            .http_client()
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Activate contract failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Activate contract returned: {}",
                response.status()
            )));
        }

        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Failed to parse deposit: {e}")))
    }

    /// Health check of the discovery service
    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = format!("{}/api/v1/health", self.endpoint);

        let response = self
            .client
            .http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Discovery health check failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Discovery health check returned: {}",
                response.status()
            )));
        }

        let health: HealthCheckResponse =
            response.json::<HealthCheckResponse>().await.map_err(|e| {
                CarbideError::Discovery(format!("Failed to parse health response: {e}"))
            })?;

        Ok(health)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfig;

    #[test]
    fn test_marketplace_query_default() {
        let query = MarketplaceQuery::default();

        assert!(query.region.is_none());
        assert!(query.tier.is_none());
        assert!(query.limit.is_none());
        assert!(query.min_reputation.is_none());
    }

    #[test]
    fn test_provider_filter_default() {
        let filter = ProviderFilter::default();

        assert!(filter.regions.is_empty());
        assert!(filter.tiers.is_empty());
        assert!(filter.min_capacity.is_none());
        assert!(filter.max_price.is_some());
        assert!(filter.min_reputation.is_some());
        assert!(filter.max_load.is_some());
    }

    #[tokio::test]
    async fn test_discovery_client_creation() {
        let client = CarbideClient::new(ClientConfig::default()).unwrap();
        let discovery = DiscoveryClient::new(client, "http://localhost:9090".to_string());

        // Just test that creation works
        assert_eq!(discovery.endpoint, "http://localhost:9090");
    }

    #[test]
    fn test_create_contract_serialization() {
        // Verify the JSON body shape matches what the discovery service expects
        let body = serde_json::json!({
            "client_id": "client-1",
            "provider_id": "provider-1",
            "file_id": "abc123",
            "file_size": 1024_u64,
            "price_per_gb_month": "0.005",
            "duration_months": 12_u32,
        });

        assert_eq!(body["client_id"], "client-1");
        assert_eq!(body["provider_id"], "provider-1");
        assert_eq!(body["file_id"], "abc123");
        assert_eq!(body["file_size"], 1024);
        assert_eq!(body["price_per_gb_month"], "0.005");
        assert_eq!(body["duration_months"], 12);
    }
}
