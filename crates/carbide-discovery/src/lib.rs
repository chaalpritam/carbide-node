//! # Carbide Discovery Service
//!
//! Provider discovery and marketplace logic for the Carbide Network.
//! This service acts as a registry and matchmaker between storage clients
//! and storage providers.

use carbide_core::{
    network::*,
    Provider, ProviderId, Region, ProviderTier, CarbideError,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::time::interval;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Discovery service configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Service bind address
    pub host: String,
    /// Service port
    pub port: u16,
    /// Provider health check interval
    pub health_check_interval: Duration,
    /// How long to keep providers without heartbeat
    pub provider_timeout: Duration,
    /// Maximum providers to return in search results
    pub max_search_results: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 9090,
            health_check_interval: Duration::from_secs(30),
            provider_timeout: Duration::from_secs(300), // 5 minutes
            max_search_results: 100,
        }
    }
}

/// Provider registry entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Provider information
    pub provider: Provider,
    /// When provider was first registered
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat timestamp
    pub last_heartbeat: DateTime<Utc>,
    /// Current health status
    pub health_status: ServiceStatus,
    /// Number of consecutive failed health checks
    pub failed_health_checks: u32,
    /// Provider's current load (0.0 - 1.0)
    pub current_load: Option<f32>,
    /// Available storage space in bytes
    pub available_storage: Option<u64>,
    /// Total number of active contracts
    pub active_contracts: usize,
}

impl RegistryEntry {
    /// Create a new registry entry
    pub fn new(provider: Provider) -> Self {
        let now = Utc::now();
        Self {
            provider,
            registered_at: now,
            last_heartbeat: now,
            health_status: ServiceStatus::Healthy,
            failed_health_checks: 0,
            current_load: Some(0.0),
            available_storage: None,
            active_contracts: 0,
        }
    }
    
    /// Check if provider is considered online
    pub fn is_online(&self, timeout: Duration) -> bool {
        let elapsed = Utc::now() - self.last_heartbeat;
        elapsed.num_seconds() < timeout.as_secs() as i64
    }
    
    /// Update health status
    pub fn update_health(&mut self, status: ServiceStatus) {
        self.last_heartbeat = Utc::now();
        self.health_status = status.clone();
        
        if matches!(status, ServiceStatus::Healthy) {
            self.failed_health_checks = 0;
        } else {
            self.failed_health_checks += 1;
        }
    }
}

/// Discovery service for provider registry and marketplace
pub struct DiscoveryService {
    /// Configuration
    config: DiscoveryConfig,
    /// Provider registry (provider_id -> registry_entry)
    registry: Arc<DashMap<ProviderId, RegistryEntry>>,
    /// Regional provider indexes for fast lookup
    regional_index: Arc<DashMap<Region, Vec<ProviderId>>>,
    /// Tier-based provider indexes
    tier_index: Arc<DashMap<ProviderTier, Vec<ProviderId>>>,
    /// Marketplace statistics
    stats: Arc<tokio::sync::RwLock<MarketplaceStats>>,
}

/// Marketplace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    /// Total number of registered providers
    pub total_providers: usize,
    /// Providers currently online
    pub online_providers: usize,
    /// Total storage capacity across all providers
    pub total_capacity_bytes: u64,
    /// Available storage capacity
    pub available_capacity_bytes: u64,
    /// Average price per GB per month
    pub average_price_per_gb: rust_decimal::Decimal,
    /// Total number of storage requests processed
    pub total_requests: u64,
    /// Last statistics update
    pub last_updated: DateTime<Utc>,
}

impl Default for MarketplaceStats {
    fn default() -> Self {
        Self {
            total_providers: 0,
            online_providers: 0,
            total_capacity_bytes: 0,
            available_capacity_bytes: 0,
            average_price_per_gb: rust_decimal::Decimal::ZERO,
            total_requests: 0,
            last_updated: Utc::now(),
        }
    }
}

impl DiscoveryService {
    /// Create a new discovery service
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            config,
            registry: Arc::new(DashMap::new()),
            regional_index: Arc::new(DashMap::new()),
            tier_index: Arc::new(DashMap::new()),
            stats: Arc::new(tokio::sync::RwLock::new(MarketplaceStats::default())),
        }
    }
    
    /// Start the discovery service
    pub async fn start(self) -> carbide_core::Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("🔍 Starting Carbide Discovery Service on {}", addr);

        // Start background tasks
        let service = Arc::new(self);
        
        // Health checker task
        let health_service = service.clone();
        let health_task = tokio::spawn(async move {
            health_service.health_checker_task().await;
        });
        
        // Statistics updater task
        let stats_service = service.clone();
        let stats_task = tokio::spawn(async move {
            stats_service.statistics_updater_task().await;
        });

        // Create and start HTTP server
        let app = service.create_router();
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| CarbideError::Internal(format!("Failed to bind to {}: {}", addr, e)))?;

        info!("✅ Discovery service listening on {}", addr);
        
        // Start server
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app).await
                .map_err(|e| CarbideError::Internal(format!("Server error: {}", e)))
        });

        // Wait for any task to complete (or fail)
        tokio::select! {
            _ = health_task => info!("Health checker task completed"),
            _ = stats_task => info!("Statistics updater task completed"),
            result = server_task => {
                if let Err(e) = result {
                    error!("Server task failed: {:?}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Create HTTP router for the service
    fn create_router(self: Arc<Self>) -> axum::Router {
        use axum::{
            extract::{Path, Query, State},
            http::StatusCode,
            response::Json,
            routing::{get, post, delete},
            Router,
        };
        
        Router::new()
            // Provider management
            .route("/api/v1/providers", post(register_provider))
            .route("/api/v1/providers", get(list_providers))
            .route("/api/v1/providers/:provider_id", get(get_provider))
            .route("/api/v1/providers/:provider_id", delete(unregister_provider))
            .route("/api/v1/providers/:provider_id/heartbeat", post(provider_heartbeat))
            
            // Marketplace and discovery
            .route("/api/v1/marketplace/search", get(search_providers))
            .route("/api/v1/marketplace/quotes", post(request_quotes))
            .route("/api/v1/marketplace/stats", get(marketplace_stats))
            
            // Health and monitoring
            .route("/api/v1/health", get(discovery_health))
            
            .with_state(self)
            .layer(
                tower::ServiceBuilder::new()
                    .layer(tower_http::trace::TraceLayer::new_for_http())
                    .layer(tower_http::cors::CorsLayer::permissive())
            )
    }
    
    /// Register a new provider
    pub async fn register_provider(&self, provider: Provider) -> carbide_core::Result<()> {
        info!("📝 Registering provider: {} ({})", provider.name, provider.id);
        
        let entry = RegistryEntry::new(provider.clone());
        
        // Insert into main registry
        self.registry.insert(provider.id, entry);
        
        // Update regional index
        self.regional_index.entry(provider.region.clone())
            .or_insert_with(Vec::new)
            .push(provider.id);
            
        // Update tier index
        self.tier_index.entry(provider.tier.clone())
            .or_insert_with(Vec::new)
            .push(provider.id);
        
        info!("✅ Provider {} registered successfully", provider.id);
        Ok(())
    }
    
    /// Unregister a provider
    pub async fn unregister_provider(&self, provider_id: ProviderId) -> carbide_core::Result<()> {
        info!("🗑️ Unregistering provider: {}", provider_id);
        
        if let Some((_, entry)) = self.registry.remove(&provider_id) {
            // Remove from regional index
            if let Some(mut region_providers) = self.regional_index.get_mut(&entry.provider.region) {
                region_providers.retain(|&id| id != provider_id);
            }
            
            // Remove from tier index
            if let Some(mut tier_providers) = self.tier_index.get_mut(&entry.provider.tier) {
                tier_providers.retain(|&id| id != provider_id);
            }
            
            info!("✅ Provider {} unregistered", provider_id);
            Ok(())
        } else {
            Err(CarbideError::Internal("Provider not found".to_string()))
        }
    }
    
    /// Update provider heartbeat
    pub async fn update_heartbeat(&self, provider_id: ProviderId, status: ServiceStatus) -> carbide_core::Result<()> {
        if let Some(mut entry) = self.registry.get_mut(&provider_id) {
            entry.update_health(status);
            Ok(())
        } else {
            Err(CarbideError::Internal("Provider not found".to_string()))
        }
    }
    
    /// Search for providers based on criteria
    pub async fn search_providers(&self, request: &ProviderListRequest) -> ProviderListResponse {
        let mut matching_providers = Vec::new();
        
        // Start with all providers or filter by region/tier
        let candidate_ids: Vec<ProviderId> = if let Some(region) = &request.region {
            self.regional_index.get(region)
                .map(|ids| ids.value().clone())
                .unwrap_or_default()
        } else if let Some(tier) = &request.tier {
            self.tier_index.get(tier)
                .map(|ids| ids.value().clone())
                .unwrap_or_default()
        } else {
            self.registry.iter().map(|entry| *entry.key()).collect()
        };
        
        // Filter candidates
        for provider_id in candidate_ids {
            if let Some(entry) = self.registry.get(&provider_id) {
                // Check if provider is online
                if !entry.is_online(self.config.provider_timeout) {
                    continue;
                }
                
                // Check reputation filter
                if let Some(min_rep) = request.min_reputation {
                    if entry.provider.reputation.overall < min_rep {
                        continue;
                    }
                }
                
                matching_providers.push(entry.provider.clone());
                
                // Respect limit
                if let Some(limit) = request.limit {
                    if matching_providers.len() >= limit {
                        break;
                    }
                }
                
                if matching_providers.len() >= self.config.max_search_results {
                    break;
                }
            }
        }
        
        // Sort by reputation (descending)
        matching_providers.sort_by(|a, b| b.reputation.overall.cmp(&a.reputation.overall));
        
        let total_count = matching_providers.len();
        let has_more = total_count >= self.config.max_search_results;
        
        ProviderListResponse {
            providers: matching_providers,
            total_count,
            has_more,
        }
    }
    
    /// Background task for health checking providers
    async fn health_checker_task(&self) {
        let mut interval = interval(self.config.health_check_interval);
        
        loop {
            interval.tick().await;
            self.perform_health_checks().await;
        }
    }
    
    /// Perform health checks on all providers
    async fn perform_health_checks(&self) {
        let mut check_tasks = Vec::new();
        
        for entry in self.registry.iter() {
            let provider_id = *entry.key();
            let provider = entry.provider.clone();
            
            let task = tokio::spawn(async move {
                // Create HTTP client for health check
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build();
                
                match client {
                    Ok(client) => {
                        let health_url = format!("{}/api/v1/health", provider.endpoint);
                        
                        match client.get(&health_url).send().await {
                            Ok(response) if response.status().is_success() => {
                                (provider_id, ServiceStatus::Healthy)
                            }
                            Ok(_) => {
                                warn!("Provider {} returned non-success status", provider_id);
                                (provider_id, ServiceStatus::Degraded)
                            }
                            Err(e) => {
                                warn!("Health check failed for provider {}: {}", provider_id, e);
                                (provider_id, ServiceStatus::Unavailable)
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to create HTTP client: {}", e);
                        (provider_id, ServiceStatus::Unavailable)
                    }
                }
            });
            
            check_tasks.push(task);
        }
        
        // Wait for all health checks to complete
        for task in check_tasks {
            if let Ok((provider_id, status)) = task.await {
                if let Some(mut entry) = self.registry.get_mut(&provider_id) {
                    entry.update_health(status);
                    
                    // Remove providers that have been offline too long
                    if entry.failed_health_checks > 5 {
                        drop(entry); // Release the lock
                        info!("Removing unresponsive provider: {}", provider_id);
                        let _ = self.unregister_provider(provider_id).await;
                    }
                }
            }
        }
    }
    
    /// Background task for updating marketplace statistics
    async fn statistics_updater_task(&self) {
        let mut interval = interval(Duration::from_secs(60)); // Update every minute
        
        loop {
            interval.tick().await;
            self.update_marketplace_stats().await;
        }
    }
    
    /// Update marketplace statistics
    async fn update_marketplace_stats(&self) {
        let mut stats = self.stats.write().await;
        
        let total_providers = self.registry.len();
        let mut online_providers = 0;
        let mut total_capacity = 0u64;
        let mut available_capacity = 0u64;
        let mut price_sum = rust_decimal::Decimal::ZERO;
        let mut price_count = 0;
        
        for entry in self.registry.iter() {
            if entry.is_online(self.config.provider_timeout) {
                online_providers += 1;
                total_capacity += entry.provider.total_capacity;
                available_capacity += entry.available_storage.unwrap_or(entry.provider.available_capacity);
                price_sum += entry.provider.price_per_gb_month;
                price_count += 1;
            }
        }
        
        stats.total_providers = total_providers;
        stats.online_providers = online_providers;
        stats.total_capacity_bytes = total_capacity;
        stats.available_capacity_bytes = available_capacity;
        stats.average_price_per_gb = if price_count > 0 {
            price_sum / rust_decimal::Decimal::new(price_count as i64, 0)
        } else {
            rust_decimal::Decimal::ZERO
        };
        stats.last_updated = Utc::now();
        
        info!("📊 Marketplace stats updated: {} providers ({} online), {:.2} GB available", 
              total_providers, online_providers, available_capacity as f64 / (1024.0 * 1024.0 * 1024.0));
    }
}

// HTTP handler functions
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};

/// Register a provider
async fn register_provider(
    State(service): State<Arc<DiscoveryService>>,
    Json(announcement): Json<ProviderAnnouncement>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match service.register_provider(announcement.provider).await {
        Ok(_) => Ok(Json(serde_json::json!({"status": "registered"}))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// List providers
async fn list_providers(
    State(service): State<Arc<DiscoveryService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ProviderListResponse> {
    let request = ProviderListRequest {
        region: params.get("region").and_then(|r| {
            match r.to_lowercase().as_str() {
                "northamerica" => Some(Region::NorthAmerica),
                "europe" => Some(Region::Europe),
                "asia" => Some(Region::Asia),
                "southamerica" => Some(Region::SouthAmerica),
                "africa" => Some(Region::Africa),
                "oceania" => Some(Region::Oceania),
                _ => None,
            }
        }),
        tier: params.get("tier").and_then(|t| {
            match t.to_lowercase().as_str() {
                "home" => Some(ProviderTier::Home),
                "professional" => Some(ProviderTier::Professional),
                "enterprise" => Some(ProviderTier::Enterprise),
                "globalcdn" => Some(ProviderTier::GlobalCDN),
                _ => None,
            }
        }),
        limit: params.get("limit").and_then(|l| l.parse().ok()),
        min_reputation: params.get("min_reputation").and_then(|r| r.parse().ok()),
    };
    
    Json(service.search_providers(&request).await)
}

/// Get specific provider
async fn get_provider(
    State(service): State<Arc<DiscoveryService>>,
    Path(provider_id): Path<String>,
) -> Result<Json<RegistryEntry>, StatusCode> {
    let id = provider_id.parse::<Uuid>().map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if let Some(entry) = service.registry.get(&id) {
        Ok(Json(entry.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Unregister a provider
async fn unregister_provider(
    State(service): State<Arc<DiscoveryService>>,
    Path(provider_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = provider_id.parse::<Uuid>().map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match service.unregister_provider(id).await {
        Ok(_) => Ok(Json(serde_json::json!({"status": "unregistered"}))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Provider heartbeat
async fn provider_heartbeat(
    State(service): State<Arc<DiscoveryService>>,
    Path(provider_id): Path<String>,
    Json(heartbeat): Json<HealthCheckResponse>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = provider_id.parse::<Uuid>().map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match service.update_heartbeat(id, heartbeat.status).await {
        Ok(_) => Ok(Json(serde_json::json!({"status": "updated"}))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Search providers
async fn search_providers(
    State(service): State<Arc<DiscoveryService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<ProviderListResponse> {
    list_providers(State(service), Query(params)).await
}

/// Request quotes from multiple providers
async fn request_quotes(
    State(service): State<Arc<DiscoveryService>>,
    Json(quote_request): Json<StorageQuoteRequest>,
) -> Json<Vec<StorageQuoteResponse>> {
    let mut quotes = Vec::new();
    
    // Find relevant providers
    let provider_request = ProviderListRequest {
        region: if quote_request.preferred_regions.is_empty() {
            None
        } else {
            quote_request.preferred_regions.first().cloned()
        },
        tier: None,
        limit: Some(10),
        min_reputation: Some(rust_decimal::Decimal::new(30, 2)), // 0.30 minimum
    };
    
    let providers = service.search_providers(&provider_request).await;
    
    // Request quotes from each provider
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    
    let mut quote_tasks = Vec::new();
    
    for provider in providers.providers {
        let client = client.clone();
        let request = quote_request.clone();
        
        let task = tokio::spawn(async move {
            let quote_url = format!("{}/api/v1/marketplace/quote", provider.endpoint);
            let message = NetworkMessage::new(MessageType::StorageQuoteRequest(request));
            
            match client.post(&quote_url).json(&message).send().await {
                Ok(response) if response.status().is_success() => {
                    match response.json::<NetworkMessage>().await {
                        Ok(msg) => {
                            if let MessageType::StorageQuoteResponse(quote) = msg.message_type {
                                Some(quote)
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                }
                _ => None,
            }
        });
        
        quote_tasks.push(task);
    }
    
    // Collect all quotes
    for task in quote_tasks {
        if let Ok(Some(quote)) = task.await {
            quotes.push(quote);
        }
    }
    
    // Sort quotes by price (ascending)
    quotes.sort_by(|a, b| a.price_per_gb_month.cmp(&b.price_per_gb_month));
    
    Json(quotes)
}

/// Get marketplace statistics
async fn marketplace_stats(
    State(service): State<Arc<DiscoveryService>>,
) -> Json<MarketplaceStats> {
    let stats = service.stats.read().await;
    Json(stats.clone())
}

/// Discovery service health check
async fn discovery_health(
    State(service): State<Arc<DiscoveryService>>,
) -> Json<HealthCheckResponse> {
    let stats = service.stats.read().await;
    
    Json(HealthCheckResponse {
        status: ServiceStatus::Healthy,
        timestamp: Utc::now(),
        version: "1.0.0".to_string(),
        available_storage: Some(stats.available_capacity_bytes),
        load: Some(0.1), // Discovery service is always lightly loaded
        reputation: None,
    })
}