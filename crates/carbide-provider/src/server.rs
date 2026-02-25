//! HTTP server implementation for storage providers
//!
//! This module implements the REST API server that allows providers to:
//! - Accept file storage requests
//! - Handle file uploads/downloads
//! - Respond to proof-of-storage challenges
//! - Provide health/status information

use std::{collections::HashMap, fs, path::PathBuf, sync::Arc, time::Duration};

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use carbide_core::{network::*, *};
use carbide_crypto::ProviderKeyPair;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{info, warn};

use crate::auth::{auth_middleware, AuthConfig, AuthState};
use crate::metrics;
use crate::storage_db::StorageDb;
use crate::tls::TlsConfig;

/// Provider server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server bind address
    pub host: String,
    /// Server port
    pub port: u16,
    /// Maximum request timeout
    pub request_timeout: Duration,
    /// Maximum file upload size
    pub max_upload_size: usize,
    /// Enable CORS for web clients
    pub enable_cors: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout: Duration::from_secs(30),
            max_upload_size: 100 * 1024 * 1024, // 100MB
            enable_cors: true,
        }
    }
}

/// Provider server state
#[derive(Debug)]
pub struct ProviderServer {
    /// Server configuration
    config: ServerConfig,
    /// Provider information
    provider: Provider,
    /// Stored files metadata
    files: Arc<tokio::sync::RwLock<HashMap<FileId, StoredFile>>>,
    /// Active storage contracts
    contracts: Arc<tokio::sync::RwLock<HashMap<Uuid, StorageContract>>>,
    /// Storage statistics
    stats: Arc<tokio::sync::RwLock<StorageStats>>,
    /// Storage directory path
    storage_dir: PathBuf,
    /// Discovery service endpoint for registration and heartbeats
    discovery_endpoint: Option<String>,
    /// Ed25519 key pair for proof signing and identity
    key_pair: Option<Arc<ProviderKeyPair>>,
    /// Heartbeat interval in seconds
    heartbeat_interval_secs: u64,
    /// Authentication configuration
    auth_config: AuthConfig,
    /// TLS configuration
    tls_config: TlsConfig,
    /// SQLite database for persistent metadata (None = in-memory only)
    db: Option<Arc<StorageDb>>,
    /// Challenges we have issued that are awaiting proof responses
    active_challenges: Arc<tokio::sync::RwLock<HashMap<String, StorageChallengeData>>>,
}

/// Information about a stored file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    /// File metadata
    pub file_id: FileId,
    /// File size in bytes
    pub size: u64,
    /// Storage path on disk
    pub storage_path: String,
    /// When file was stored
    pub stored_at: DateTime<Utc>,
    /// Associated contract
    pub contract_id: Uuid,
    /// Content type
    pub content_type: String,
    /// Whether file is encrypted
    pub is_encrypted: bool,
}

/// Storage provider statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total files stored
    pub total_files: usize,
    /// Total bytes stored
    pub total_bytes_stored: u64,
    /// Available storage space
    pub available_space: u64,
    /// Total storage capacity
    pub total_capacity: u64,
    /// Number of active contracts
    pub active_contracts: usize,
    /// Server start time
    pub server_start_time: DateTime<Utc>,
    /// Last health check
    pub last_health_check: DateTime<Utc>,
}

impl ProviderServer {
    /// Create a new provider server.
    ///
    /// If `db_path` is `Some`, file and contract metadata is persisted to SQLite
    /// and restored on startup.  Pass `None` for a purely in-memory instance
    /// (useful for tests and the quick-start CLI mode).
    pub fn new(
        config: ServerConfig,
        provider: Provider,
        storage_path: PathBuf,
        discovery_endpoint: Option<String>,
        key_pair: Option<Arc<ProviderKeyPair>>,
        heartbeat_interval_secs: u64,
        auth_config: AuthConfig,
        tls_config: TlsConfig,
        db_path: Option<&std::path::Path>,
    ) -> Result<Self> {
        let storage_dir = storage_path.join(provider.id.to_string());

        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_dir).map_err(|e| {
            CarbideError::Internal(format!(
                "Failed to create storage directory {}: {e}",
                storage_dir.display()
            ))
        })?;

        // Open SQLite database if a path was provided
        let db = match db_path {
            Some(p) => {
                // Ensure parent directory exists
                if let Some(parent) = p.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        CarbideError::Internal(format!(
                            "Failed to create database directory {}: {e}",
                            parent.display()
                        ))
                    })?;
                }
                let db = StorageDb::open(p).map_err(|e| {
                    CarbideError::Internal(format!("Failed to open storage database: {e}"))
                })?;
                Some(Arc::new(db))
            }
            None => None,
        };

        // Warm in-memory caches from the database
        let mut files_map = HashMap::new();
        let mut contracts_map = HashMap::new();
        let mut total_bytes: u64 = 0;

        if let Some(ref db) = db {
            if let Ok(stored) = db.load_all_files() {
                for f in stored {
                    total_bytes += f.size;
                    files_map.insert(f.file_id, f);
                }
            }
            if let Ok(contracts) = db.load_all_contracts() {
                for c in contracts {
                    contracts_map.insert(c.id, c);
                }
            }
            info!(
                "Restored {} files and {} contracts from database",
                files_map.len(),
                contracts_map.len()
            );
        }

        let active_contracts = contracts_map
            .values()
            .filter(|c| matches!(c.status, ContractStatus::Active))
            .count();

        let stats = StorageStats {
            total_files: files_map.len(),
            total_bytes_stored: total_bytes,
            available_space: provider.total_capacity.saturating_sub(total_bytes),
            total_capacity: provider.total_capacity,
            active_contracts,
            server_start_time: Utc::now(),
            last_health_check: Utc::now(),
        };

        Ok(Self {
            config,
            provider,
            files: Arc::new(tokio::sync::RwLock::new(files_map)),
            contracts: Arc::new(tokio::sync::RwLock::new(contracts_map)),
            stats: Arc::new(tokio::sync::RwLock::new(stats)),
            storage_dir,
            discovery_endpoint,
            key_pair,
            heartbeat_interval_secs,
            auth_config,
            tls_config,
            db,
            active_challenges: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Get the file path for storing a file
    fn get_file_path(&self, file_id: &FileId) -> PathBuf {
        self.storage_dir.join(format!("{}.dat", file_id.to_hex()))
    }

    /// Write file data to disk
    async fn write_file_to_disk(&self, file_id: &FileId, data: &[u8]) -> Result<()> {
        let file_path = self.get_file_path(file_id);

        tokio::fs::write(&file_path, data).await.map_err(|e| {
            CarbideError::Internal(format!("Failed to write file {}: {e}", file_path.display()))
        })?;

        info!("💾 File {} written to disk ({} bytes)", file_id, data.len());
        Ok(())
    }

    /// Read file data from disk
    async fn read_file_from_disk(&self, file_id: &FileId) -> Result<Vec<u8>> {
        let file_path = self.get_file_path(file_id);

        tokio::fs::read(&file_path).await.map_err(|e| {
            CarbideError::Internal(format!("Failed to read file {}: {e}", file_path.display()))
        })
    }

    /// Delete file from disk
    async fn delete_file_from_disk(&self, file_id: &FileId) -> Result<()> {
        let file_path = self.get_file_path(file_id);

        tokio::fs::remove_file(&file_path).await.map_err(|e| {
            CarbideError::Internal(format!(
                "Failed to delete file {}: {e}",
                file_path.display()
            ))
        })?;

        info!("🗑️ File {} deleted from disk", file_id);
        Ok(())
    }

    /// Start the HTTP server
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);

        // Initialize Prometheus metrics
        metrics::register_metrics();

        // Seed gauges from current stats
        {
            let stats = self.stats.read().await;
            metrics::STORAGE_BYTES_USED.set(stats.total_bytes_stored as i64);
            metrics::FILES_STORED.set(stats.total_files as i64);
            metrics::ACTIVE_CONTRACTS.set(stats.active_contracts as i64);
        }

        info!("🚀 Starting Carbide Provider Server on {}", addr);

        info!("✅ Provider server listening on {}", addr);
        info!("   Provider ID: {}", self.provider.id);
        info!("   Provider Name: {}", self.provider.name);
        info!("   Price: ${}/GB/month", self.provider.price_per_gb_month);
        info!(
            "   Available Capacity: {:.2} GB",
            self.provider.available_capacity as f64 / (1024.0 * 1024.0 * 1024.0)
        );

        // Clone data needed for the registration loop before moving self
        let discovery_endpoint = self.discovery_endpoint.clone();
        let provider = self.provider.clone();
        let stats = Arc::clone(&self.stats);
        let key_pair = self.key_pair.clone();
        let heartbeat_interval = self.heartbeat_interval_secs;

        // Spawn registration & heartbeat loop if discovery is configured
        if let Some(endpoint) = discovery_endpoint {
            tokio::spawn(registration_loop(
                endpoint,
                provider,
                stats,
                key_pair,
                heartbeat_interval,
            ));
        }

        // Create the router with all endpoints
        let tls_config = self.tls_config.clone();
        let app = self.create_router();

        if tls_config.enabled {
            info!("TLS enabled, loading certificate...");
            let rustls_config =
                crate::tls::load_rustls_config(&tls_config)
                    .await
                    .map_err(|e| CarbideError::Internal(format!("TLS config error: {e}")))?;

            let addr_parsed: std::net::SocketAddr = addr.parse().map_err(|e| {
                CarbideError::Internal(format!("Invalid address {addr}: {e}"))
            })?;

            info!("Starting HTTPS server on {}", addr);
            axum_server::bind_rustls(addr_parsed, rustls_config)
                .serve(app.into_make_service())
                .await
                .map_err(|e| CarbideError::Internal(format!("TLS server error: {e}")))?;
        } else {
            // Create TCP listener (plain HTTP)
            let listener = TcpListener::bind(&addr)
                .await
                .map_err(|e| CarbideError::Internal(format!("Failed to bind to {addr}: {e}")))?;

            axum::serve(listener, app)
                .await
                .map_err(|e| CarbideError::Internal(format!("Server error: {e}")))?;
        }

        Ok(())
    }

    /// Create the router with all API endpoints
    fn create_router(self) -> Router {
        let enable_cors = self.config.enable_cors;
        let request_timeout = self.config.request_timeout;
        let max_upload_size = self.config.max_upload_size;
        let auth_state = Arc::new(AuthState {
            config: self.auth_config.clone(),
        });
        let server_state = Arc::new(self);

        // Public routes — health, status, and metrics (no auth required)
        let public_routes = Router::new()
            .route(ApiEndpoints::HEALTH_CHECK, get(health_check))
            .route(ApiEndpoints::PROVIDER_STATUS, get(provider_status))
            .route("/metrics", get(metrics::metrics_handler));

        // Protected routes — all file/marketplace/proof operations
        let protected_routes = Router::new()
            .route(ApiEndpoints::FILE_STORE, post(store_file_request))
            .route(ApiEndpoints::FILE_RETRIEVE, get(retrieve_file))
            .route(ApiEndpoints::FILE_DELETE, delete(delete_file))
            .route(ApiEndpoints::FILE_UPLOAD, post(upload_file))
            .route(ApiEndpoints::FILE_DOWNLOAD, get(download_file))
            .route(ApiEndpoints::STORAGE_QUOTE, post(storage_quote))
            .route(ApiEndpoints::PROOF_CHALLENGE, post(proof_challenge))
            .route(ApiEndpoints::PROOF_RESPONSE, post(proof_response))
            .route_layer(middleware::from_fn_with_state(
                auth_state,
                auth_middleware,
            ));

        public_routes
            .merge(protected_routes)
            .with_state(server_state)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(TimeoutLayer::new(request_timeout))
                    .layer(DefaultBodyLimit::max(max_upload_size))
                    .layer(if enable_cors {
                        CorsLayer::permissive()
                    } else {
                        CorsLayer::new()
                    }),
            )
    }
}

// ============================================================================
// Registration & Heartbeat Loop
// ============================================================================

/// Background task that registers with discovery and sends periodic heartbeats
async fn registration_loop(
    discovery_endpoint: String,
    provider: Provider,
    stats: Arc<tokio::sync::RwLock<StorageStats>>,
    key_pair: Option<Arc<ProviderKeyPair>>,
    heartbeat_interval_secs: u64,
) {
    let http_client = reqwest::Client::new();
    let register_url = format!("{}/api/v1/providers", discovery_endpoint);
    let heartbeat_url = format!(
        "{}/api/v1/providers/{}/heartbeat",
        discovery_endpoint, provider.id
    );

    // Phase 1: Register with exponential backoff
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(60);

    loop {
        let public_key = key_pair.as_ref().map(|kp| kp.public_key_hex());
        let announcement = ProviderAnnouncement {
            provider: provider.clone(),
            endpoint: provider.endpoint.clone(),
            supported_versions: vec!["1.0.0".to_string()],
            public_key,
        };

        match http_client
            .post(&register_url)
            .json(&announcement)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "Registered with discovery service at {}",
                    discovery_endpoint
                );
                break;
            }
            Ok(resp) => {
                warn!(
                    "Discovery registration returned {}, retrying in {:?}",
                    resp.status(),
                    backoff
                );
            }
            Err(e) => {
                warn!(
                    "Discovery registration failed: {}, retrying in {:?}",
                    e, backoff
                );
            }
        }

        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }

    // Phase 2: Periodic heartbeats
    let mut interval = tokio::time::interval(Duration::from_secs(heartbeat_interval_secs));
    loop {
        interval.tick().await;

        let current_stats = stats.read().await;
        let load = if current_stats.total_capacity > 0 {
            (current_stats.total_bytes_stored as f64 / current_stats.total_capacity as f64) as f32
        } else {
            0.0
        };

        let health = HealthCheckResponse {
            status: ServiceStatus::Healthy,
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            available_storage: Some(current_stats.available_space),
            load: Some(load),
            reputation: Some(provider.reputation.overall),
        };
        drop(current_stats);

        match http_client
            .post(&heartbeat_url)
            .json(&health)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!("Heartbeat sent to discovery");
            }
            Ok(resp) => {
                warn!("Heartbeat returned {}", resp.status());
            }
            Err(e) => {
                warn!("Heartbeat failed: {}", e);
            }
        }
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Health check endpoint
async fn health_check(State(server): State<Arc<ProviderServer>>) -> Json<NetworkMessage> {
    let mut stats = server.stats.write().await;
    stats.last_health_check = Utc::now();

    // Calculate actual load from storage utilization
    let load = if stats.total_capacity > 0 {
        (stats.total_bytes_stored as f64 / stats.total_capacity as f64) as f32
    } else {
        0.0
    };

    let health_response = HealthCheckResponse {
        status: ServiceStatus::Healthy,
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        available_storage: Some(stats.available_space),
        load: Some(load),
        reputation: Some(server.provider.reputation.overall),
    };

    let message = NetworkMessage::new(MessageType::HealthCheckResponse(health_response));

    Json(message)
}

/// Provider status endpoint
async fn provider_status(
    State(server): State<Arc<ProviderServer>>,
) -> Json<ProviderStatusResponse> {
    let stats = server.stats.read().await;

    Json(ProviderStatusResponse {
        provider: server.provider.clone(),
        stats: stats.clone(),
        uptime_seconds: (Utc::now() - stats.server_start_time).num_seconds() as u64,
        is_accepting_new_files: stats.available_space > 1024 * 1024 * 1024, // 1GB threshold
    })
}

/// Handle file storage requests
async fn store_file_request(
    State(server): State<Arc<ProviderServer>>,
    Json(message): Json<NetworkMessage>,
) -> std::result::Result<Json<NetworkMessage>, StatusCode> {
    match message.message_type {
        MessageType::StoreFileRequest(ref request) => {
            info!("Received store file request for {}", request.file_id);

            // Check if we can accept the file
            let stats = server.stats.read().await;

            let can_store = request.file_size <= stats.available_space
                && server.provider.price_per_gb_month <= request.max_price;

            let response = if can_store {
                // Accept the storage request
                let contract = StorageContract {
                    id: Uuid::new_v4(),
                    request_id: Uuid::new_v4(),
                    file_id: request.file_id,
                    provider_id: server.provider.id,
                    price_per_gb_month: server.provider.price_per_gb_month,
                    duration_months: request.duration_months,
                    started_at: Utc::now(),
                    status: ContractStatus::Active,
                    last_proof_at: None,
                };

                // Store the contract (in-memory + SQLite) and update metrics
                if let Some(ref db) = server.db {
                    if let Err(e) = db.insert_contract(&contract) {
                        tracing::error!("Failed to persist contract to database: {}", e);
                    }
                }
                server
                    .contracts
                    .write()
                    .await
                    .insert(contract.id, contract.clone());
                metrics::ACTIVE_CONTRACTS.inc();

                // Generate upload URL using the provider's public endpoint
                let upload_url =
                    format!("{}{}", server.provider.endpoint, ApiEndpoints::FILE_UPLOAD);
                let upload_token = contract.id.to_string();

                StoreFileResponse {
                    accepted: true,
                    contract: Some(contract),
                    upload_url: Some(upload_url),
                    upload_token: Some(upload_token),
                    rejection_reason: None,
                    counter_offer_price: None,
                }
            } else {
                // Reject the storage request
                let rejection_reason = if request.file_size > stats.available_space {
                    "Insufficient storage capacity"
                } else {
                    "Price too low"
                };

                StoreFileResponse {
                    accepted: false,
                    contract: None,
                    upload_url: None,
                    upload_token: None,
                    rejection_reason: Some(rejection_reason.to_string()),
                    counter_offer_price: Some(server.provider.price_per_gb_month),
                }
            };

            let response_message =
                NetworkMessage::new_response(MessageType::StoreFileResponse(response), &message);

            Ok(Json(response_message))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handle file uploads
async fn upload_file(
    State(server): State<Arc<ProviderServer>>,
    mut multipart: Multipart,
) -> std::result::Result<Json<UploadResponse>, StatusCode> {
    info!("Handling file upload");

    let mut file_data = Vec::new();
    let mut file_id = None;
    let mut upload_token = None;

    // Process multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file_data = data.to_vec();
            }
            "file_id" => {
                let value = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file_id = Some(ContentHash::from_hex(&value).map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "token" => {
                let value = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                upload_token = Some(value);
            }
            _ => {}
        }
    }

    let file_id = file_id.ok_or(StatusCode::BAD_REQUEST)?;
    let token = upload_token.ok_or(StatusCode::BAD_REQUEST)?;

    // Validate upload token against active contracts
    let contract_id = Uuid::parse_str(&token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let contracts = server.contracts.read().await;
    let contract = contracts
        .get(&contract_id)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify the file_id matches the contract
    if contract.file_id != file_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Write file to disk
    if let Err(e) = server.write_file_to_disk(&file_id, &file_data).await {
        tracing::error!("Failed to write file to disk: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Create stored file record
    let stored_file = StoredFile {
        file_id,
        size: file_data.len() as u64,
        storage_path: server.get_file_path(&file_id).to_string_lossy().to_string(),
        stored_at: Utc::now(),
        contract_id,
        content_type: "application/octet-stream".to_string(),
        is_encrypted: false,
    };

    // Update server state (in-memory + SQLite)
    if let Some(ref db) = server.db {
        if let Err(e) = db.insert_file(&stored_file) {
            tracing::error!("Failed to persist file record to database: {}", e);
        }
    }
    server.files.write().await.insert(file_id, stored_file);

    let mut stats = server.stats.write().await;
    stats.total_files += 1;
    stats.total_bytes_stored += file_data.len() as u64;
    stats.available_space = stats.available_space.saturating_sub(file_data.len() as u64);

    // Update Prometheus gauges
    metrics::FILES_STORED.inc();
    metrics::STORAGE_BYTES_USED.add(file_data.len() as i64);

    info!(
        "✅ File {} uploaded successfully ({} bytes)",
        file_id,
        file_data.len()
    );

    // Fire-and-forget: notify discovery about file-provider mapping
    if let Some(ref endpoint) = server.discovery_endpoint {
        let url = format!("{}/api/v1/files/{}/providers", endpoint, file_id.to_hex());
        let provider_id = server.provider.id.to_string();
        let file_size = file_data.len() as u64;
        let http_client = reqwest::Client::new();
        tokio::spawn(async move {
            let body = serde_json::json!({
                "provider_id": provider_id,
                "file_size": file_size,
            });
            if let Err(e) = http_client.post(&url).json(&body).send().await {
                tracing::warn!("Failed to notify discovery of file mapping: {}", e);
            }
        });
    }

    Ok(Json(UploadResponse {
        success: true,
        file_id,
        size: file_data.len() as u64,
        message: "File uploaded successfully".to_string(),
    }))
}

/// Handle file retrieval requests
async fn retrieve_file(
    Path(file_id): Path<String>,
    State(server): State<Arc<ProviderServer>>,
) -> std::result::Result<Json<NetworkMessage>, StatusCode> {
    info!("Retrieving file: {}", file_id);

    let file_hash = ContentHash::from_hex(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let files = server.files.read().await;

    if let Some(stored_file) = files.get(&file_hash) {
        let response = RetrieveFileResponse {
            file_id: stored_file.file_id,
            data: None, // For large files, provide download URL instead
            download_url: Some(format!(
                "{}/api/v1/download/{}",
                server.provider.endpoint, file_id
            )),
            content_type: stored_file.content_type.clone(),
            size: stored_file.size,
            last_modified: stored_file.stored_at,
        };

        let message = NetworkMessage::new(MessageType::RetrieveFileResponse(response));

        Ok(Json(message))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Handle file deletion
async fn delete_file(
    Path(file_id): Path<String>,
    State(server): State<Arc<ProviderServer>>,
) -> std::result::Result<Json<NetworkMessage>, StatusCode> {
    info!("Deleting file: {}", file_id);

    let file_hash = ContentHash::from_hex(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut files = server.files.write().await;

    if let Some(stored_file) = files.remove(&file_hash) {
        // Delete file from disk
        if let Err(e) = server.delete_file_from_disk(&file_hash).await {
            tracing::error!("Failed to delete file from disk: {}", e);
            // Continue with removal from memory even if disk deletion fails
        }

        // Remove from database
        if let Some(ref db) = server.db {
            if let Err(e) = db.delete_file(&file_hash) {
                tracing::error!("Failed to delete file record from database: {}", e);
            }
        }

        // Update statistics
        let mut stats = server.stats.write().await;
        stats.total_files = stats.total_files.saturating_sub(1);
        stats.total_bytes_stored = stats.total_bytes_stored.saturating_sub(stored_file.size);
        stats.available_space += stored_file.size;

        let response = DeleteFileResponse {
            success: true,
            error: None,
            freed_bytes: Some(stored_file.size),
        };

        let message = NetworkMessage::new(MessageType::DeleteFileResponse(response));

        // Update Prometheus gauges
        metrics::FILES_STORED.dec();
        metrics::STORAGE_BYTES_USED.sub(stored_file.size as i64);

        info!(
            "✅ File {} deleted ({} bytes freed)",
            file_id, stored_file.size
        );
        Ok(Json(message))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Handle file downloads
async fn download_file(
    Path(file_id): Path<String>,
    State(server): State<Arc<ProviderServer>>,
) -> std::result::Result<Vec<u8>, StatusCode> {
    info!("Downloading file: {}", file_id);

    let file_hash = ContentHash::from_hex(&file_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check if file exists in our records
    let files = server.files.read().await;
    if !files.contains_key(&file_hash) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Read file data from disk
    match server.read_file_from_disk(&file_hash).await {
        Ok(data) => {
            info!("✅ File {} downloaded ({} bytes)", file_id, data.len());
            Ok(data)
        }
        Err(e) => {
            tracing::error!("Failed to read file from disk: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle storage quote requests
async fn storage_quote(
    State(server): State<Arc<ProviderServer>>,
    Json(message): Json<NetworkMessage>,
) -> std::result::Result<Json<NetworkMessage>, StatusCode> {
    match message.message_type {
        MessageType::StorageQuoteRequest(ref request) => {
            let stats = server.stats.read().await;

            let can_fulfill = request.file_size <= stats.available_space;
            let total_monthly_cost = server.provider.price_per_gb_month
                * rust_decimal::Decimal::new(request.file_size as i64, 9) // Convert bytes to GB
                * rust_decimal::Decimal::new(i64::from(request.replication_factor), 0);

            let response = StorageQuoteResponse {
                provider_id: server.provider.id,
                price_per_gb_month: server.provider.price_per_gb_month,
                total_monthly_cost,
                can_fulfill,
                available_capacity: stats.available_space,
                estimated_start_time: if can_fulfill { 1 } else { 24 }, /* 1 hour if can fulfill,
                                                                         * 24 if not */
                valid_until: Utc::now() + chrono::Duration::hours(1),
            };

            let response_message =
                NetworkMessage::new_response(MessageType::StorageQuoteResponse(response), &message);

            Ok(Json(response_message))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handle proof-of-storage challenges
async fn proof_challenge(
    State(server): State<Arc<ProviderServer>>,
    Json(message): Json<NetworkMessage>,
) -> Json<NetworkMessage> {
    info!("Received proof-of-storage challenge");

    if let MessageType::StorageChallenge(ref challenge) = message.message_type {
        let files = server.files.read().await;

        // Check if we have the file being challenged
        if let Some(_stored_file) = files.get(&challenge.file_hash) {
            // Verify the challenge is still valid
            if challenge.expires_at < Utc::now() {
                info!("⏰ Challenge expired: {}", challenge.challenge_id);
                // Return error response
                let error_msg = ErrorMessage {
                    code: "CHALLENGE_EXPIRED".to_string(),
                    message: "Challenge has expired".to_string(),
                    details: None,
                };
                return Json(NetworkMessage::new_response(
                    MessageType::Error(error_msg),
                    &message,
                ));
            }

            // Generate proof for the requested chunks
            let mut merkle_proofs = Vec::new();
            for &chunk_index in &challenge.chunk_indices {
                // For now, create a simple proof (in production this would use actual Merkle trees)
                let chunk_proof = ChunkProofData {
                    chunk_index,
                    chunk_hash: ContentHash::from_data(format!("chunk_{chunk_index}").as_bytes()),
                    merkle_path: vec![
                        ContentHash::from_data(b"merkle_sibling_1"),
                        ContentHash::from_data(b"merkle_sibling_2"),
                    ],
                    chunk_data: None, // Don't include actual data for large files
                };
                merkle_proofs.push(chunk_proof);
            }

            // Compute response hash
            let response_data = format!(
                "{}:{}:{:?}",
                challenge.challenge_id,
                challenge.file_hash.to_hex(),
                challenge.chunk_indices
            );
            let response_hash = ContentHash::from_data(response_data.as_bytes());

            // Sign the response hash with provider's Ed25519 key pair
            let signature = if let Some(ref kp) = server.key_pair {
                kp.sign(response_hash.as_bytes())
            } else {
                vec![0u8; 64] // No key pair configured — unsigned proof
            };

            let response = StorageProofData {
                challenge_id: challenge.challenge_id.clone(),
                merkle_proofs,
                response_hash,
                signature,
                generated_at: Utc::now(),
            };

            // Store the challenge so proof_response can look it up later
            server
                .active_challenges
                .write()
                .await
                .insert(challenge.challenge_id.clone(), challenge.clone());

            info!(
                "✅ Generated proof for challenge {} ({} chunks)",
                challenge.challenge_id,
                challenge.chunk_indices.len()
            );

            Json(NetworkMessage::new_response(
                MessageType::StorageProof(response),
                &message,
            ))
        } else {
            // File not found
            info!("❌ File not found for challenge: {}", challenge.file_hash);
            let error_msg = ErrorMessage {
                code: ErrorCodes::FILE_NOT_FOUND.to_string(),
                message: "File not found in storage".to_string(),
                details: None,
            };
            Json(NetworkMessage::new_response(
                MessageType::Error(error_msg),
                &message,
            ))
        }
    } else {
        // Invalid message type
        let error_msg = ErrorMessage {
            code: ErrorCodes::INVALID_REQUEST.to_string(),
            message: "Expected storage challenge message".to_string(),
            details: None,
        };
        Json(NetworkMessage::new_response(
            MessageType::Error(error_msg),
            &message,
        ))
    }
}

/// Handle proof responses (verification)
///
/// Receives a StorageProof message, looks up the original challenge,
/// and delegates to `carbide_crypto::proofs::ProofVerifier` for
/// cryptographic verification.
async fn proof_response(
    State(server): State<Arc<ProviderServer>>,
    Json(message): Json<NetworkMessage>,
) -> Json<VerificationResponse> {
    info!("Verifying proof-of-storage response");

    let proof_data = match message.message_type {
        MessageType::StorageProof(ref p) => p.clone(),
        _ => {
            return Json(VerificationResponse {
                valid: false,
                message: "Expected StorageProof message".to_string(),
            });
        }
    };

    // Look up the original challenge
    let challenges = server.active_challenges.read().await;
    let challenge_data = match challenges.get(&proof_data.challenge_id) {
        Some(c) => c.clone(),
        None => {
            return Json(VerificationResponse {
                valid: false,
                message: format!(
                    "Unknown challenge id: {}",
                    proof_data.challenge_id
                ),
            });
        }
    };
    drop(challenges);

    // Convert network types → carbide_crypto types
    let crypto_challenge = carbide_crypto::proofs::StorageChallenge {
        challenge_id: challenge_data.challenge_id.clone(),
        file_hash: challenge_data.file_hash,
        chunk_indices: challenge_data.chunk_indices.clone(),
        nonce: challenge_data.nonce,
        issued_at: challenge_data.issued_at,
        expires_at: challenge_data.expires_at,
        expected_response_hash: challenge_data.expected_response_hash,
    };

    let crypto_proof = carbide_crypto::proofs::StorageProof {
        challenge_id: proof_data.challenge_id.clone(),
        merkle_proofs: proof_data
            .merkle_proofs
            .iter()
            .map(|p| carbide_crypto::proofs::ChunkProof {
                chunk_index: p.chunk_index,
                chunk_hash: p.chunk_hash,
                merkle_path: p.merkle_path.clone(),
                chunk_data: p.chunk_data.clone(),
            })
            .collect(),
        response_hash: proof_data.response_hash,
        signature: proof_data.signature.clone(),
        generated_at: proof_data.generated_at,
    };

    // For the merkle root we currently use the file's content hash as a
    // stand-in. A full implementation would build and persist a Merkle tree
    // per file; for now the verifier checks proof structure and response hash.
    let file_merkle_root = challenge_data.file_hash;

    match carbide_crypto::proofs::ProofVerifier::verify_proof(
        &crypto_challenge,
        &crypto_proof,
        file_merkle_root,
    ) {
        Ok(true) => {
            // Remove the challenge so it cannot be replayed
            server
                .active_challenges
                .write()
                .await
                .remove(&proof_data.challenge_id);

            info!("✅ Proof verified for challenge {}", proof_data.challenge_id);
            Json(VerificationResponse {
                valid: true,
                message: "Proof verified successfully".to_string(),
            })
        }
        Ok(false) => {
            warn!(
                "❌ Proof verification failed for challenge {}",
                proof_data.challenge_id
            );
            Json(VerificationResponse {
                valid: false,
                message: "Proof verification failed".to_string(),
            })
        }
        Err(e) => {
            warn!("Proof verification error: {}", e);
            Json(VerificationResponse {
                valid: false,
                message: format!("Verification error: {e}"),
            })
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Provider status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderStatusResponse {
    /// Provider information
    pub provider: Provider,
    /// Current storage statistics
    pub stats: StorageStats,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Whether accepting new files
    pub is_accepting_new_files: bool,
}

/// File upload response
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadResponse {
    /// Whether upload was successful
    pub success: bool,
    /// File ID
    pub file_id: ContentHash,
    /// File size in bytes
    pub size: u64,
    /// Response message
    pub message: String,
}

/// Proof verification response
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResponse {
    /// Whether proof is valid
    pub valid: bool,
    /// Verification message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use carbide_core::{ProviderTier, Region};
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_upload_size, 100 * 1024 * 1024);
        assert!(config.enable_cors);
    }

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let provider = Provider::new(
            "Test Provider".to_string(),
            ProviderTier::Home,
            Region::NorthAmerica,
            "https://test.example.com".to_string(),
            1024 * 1024 * 1024, // 1GB
            Decimal::new(2, 3), // $0.002
        );

        let storage_path = PathBuf::from("./test_storage");
        let server = ProviderServer::new(config, provider.clone(), storage_path, None, None, 60, Default::default(), Default::default(), None).unwrap();

        assert_eq!(server.provider.name, "Test Provider");
        assert_eq!(server.provider.tier, ProviderTier::Home);
    }
}
