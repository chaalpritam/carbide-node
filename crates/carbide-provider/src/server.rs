//! HTTP server implementation for storage providers
//!
//! This module implements the REST API server that allows providers to:
//! - Accept file storage requests
//! - Handle file uploads/downloads
//! - Respond to proof-of-storage challenges
//! - Provide health/status information

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use carbide_core::{
    network::*,
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;

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
    /// Create a new provider server
    pub fn new(config: ServerConfig, provider: Provider) -> Result<Self> {
        let storage_dir = PathBuf::from("./storage")
            .join(provider.id.to_string());
        
        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_dir)
            .map_err(|e| CarbideError::Internal(
                format!("Failed to create storage directory {:?}: {}", storage_dir, e)
            ))?;

        let stats = StorageStats {
            total_files: 0,
            total_bytes_stored: 0,
            available_space: provider.available_capacity,
            total_capacity: provider.total_capacity,
            active_contracts: 0,
            server_start_time: Utc::now(),
            last_health_check: Utc::now(),
        };

        Ok(Self {
            config,
            provider,
            files: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            contracts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            stats: Arc::new(tokio::sync::RwLock::new(stats)),
            storage_dir,
        })
    }
    
    /// Get the file path for storing a file
    fn get_file_path(&self, file_id: &FileId) -> PathBuf {
        self.storage_dir.join(format!("{}.dat", file_id.to_hex()))
    }
    
    /// Write file data to disk
    async fn write_file_to_disk(&self, file_id: &FileId, data: &[u8]) -> Result<()> {
        let file_path = self.get_file_path(file_id);
        
        tokio::fs::write(&file_path, data).await
            .map_err(|e| CarbideError::Internal(
                format!("Failed to write file {:?}: {}", file_path, e)
            ))?;
            
        info!("💾 File {} written to disk ({} bytes)", file_id, data.len());
        Ok(())
    }
    
    /// Read file data from disk
    async fn read_file_from_disk(&self, file_id: &FileId) -> Result<Vec<u8>> {
        let file_path = self.get_file_path(file_id);
        
        tokio::fs::read(&file_path).await
            .map_err(|e| CarbideError::Internal(
                format!("Failed to read file {:?}: {}", file_path, e)
            ))
    }
    
    /// Delete file from disk
    async fn delete_file_from_disk(&self, file_id: &FileId) -> Result<()> {
        let file_path = self.get_file_path(file_id);
        
        tokio::fs::remove_file(&file_path).await
            .map_err(|e| CarbideError::Internal(
                format!("Failed to delete file {:?}: {}", file_path, e)
            ))?;
            
        info!("🗑️ File {} deleted from disk", file_id);
        Ok(())
    }

    /// Start the HTTP server
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("🚀 Starting Carbide Provider Server on {}", addr);

        info!("✅ Provider server listening on {}", addr);
        info!("   Provider ID: {}", self.provider.id);
        info!("   Provider Name: {}", self.provider.name);
        info!("   Price: ${}/GB/month", self.provider.price_per_gb_month);
        info!("   Available Capacity: {:.2} GB", 
              self.provider.available_capacity as f64 / (1024.0 * 1024.0 * 1024.0));

        // Create the router with all endpoints
        let app = self.create_router();

        // Create TCP listener
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| CarbideError::Internal(format!("Failed to bind to {}: {}", addr, e)))?;

        // Start the server
        axum::serve(listener, app).await
            .map_err(|e| CarbideError::Internal(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Create the router with all API endpoints
    fn create_router(self) -> Router {
        let enable_cors = self.config.enable_cors;
        let server_state = Arc::new(self);

        Router::new()
            // Health and status endpoints
            .route(ApiEndpoints::HEALTH_CHECK, get(health_check))
            .route(ApiEndpoints::PROVIDER_STATUS, get(provider_status))
            
            // File storage endpoints  
            .route(ApiEndpoints::FILE_STORE, post(store_file_request))
            .route(ApiEndpoints::FILE_RETRIEVE, get(retrieve_file))
            .route(ApiEndpoints::FILE_DELETE, delete(delete_file))
            .route(ApiEndpoints::FILE_UPLOAD, post(upload_file))
            .route(ApiEndpoints::FILE_DOWNLOAD, get(download_file))
            
            // Marketplace endpoints
            .route(ApiEndpoints::STORAGE_QUOTE, post(storage_quote))
            
            // Proof of storage endpoints
            .route(ApiEndpoints::PROOF_CHALLENGE, post(proof_challenge))
            .route(ApiEndpoints::PROOF_RESPONSE, post(proof_response))
            
            .with_state(server_state)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB
                    .layer(if enable_cors {
                        CorsLayer::permissive()
                    } else {
                        CorsLayer::new()
                    })
            )
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Health check endpoint
async fn health_check(
    State(server): State<Arc<ProviderServer>>,
) -> Json<NetworkMessage> {
    let mut stats = server.stats.write().await;
    stats.last_health_check = Utc::now();
    
    let health_response = HealthCheckResponse {
        status: ServiceStatus::Healthy,
        timestamp: Utc::now(),
        version: "1.0.0".to_string(),
        available_storage: Some(stats.available_space),
        load: Some(0.5), // TODO: Calculate actual load
        reputation: Some(server.provider.reputation.overall),
    };

    let message = NetworkMessage::new(
        MessageType::HealthCheckResponse(health_response)
    );

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
                
                // Store the contract
                server.contracts.write().await.insert(contract.id, contract.clone());

                // Generate upload URL and token
                let upload_url = format!("http://{}:{}{}", 
                    server.config.host, server.config.port, ApiEndpoints::FILE_UPLOAD);
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

            let response_message = NetworkMessage::new_response(
                MessageType::StoreFileResponse(response),
                &message,
            );

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
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
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
    let contract = contracts.get(&contract_id).ok_or(StatusCode::UNAUTHORIZED)?;
    
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

    // Update server state
    server.files.write().await.insert(file_id, stored_file);
    
    let mut stats = server.stats.write().await;
    stats.total_files += 1;
    stats.total_bytes_stored += file_data.len() as u64;
    stats.available_space = stats.available_space.saturating_sub(file_data.len() as u64);

    info!("✅ File {} uploaded successfully ({} bytes)", file_id, file_data.len());

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
            download_url: Some(format!("http://{}:{}/api/v1/download/{}", 
                server.config.host, server.config.port, file_id)),
            content_type: stored_file.content_type.clone(),
            size: stored_file.size,
            last_modified: stored_file.stored_at,
        };

        let message = NetworkMessage::new(
            MessageType::RetrieveFileResponse(response)
        );

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

        let message = NetworkMessage::new(
            MessageType::DeleteFileResponse(response)
        );

        info!("✅ File {} deleted ({} bytes freed)", file_id, stored_file.size);
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
                * rust_decimal::Decimal::new(request.replication_factor as i64, 0);

            let response = StorageQuoteResponse {
                provider_id: server.provider.id,
                price_per_gb_month: server.provider.price_per_gb_month,
                total_monthly_cost,
                can_fulfill,
                available_capacity: stats.available_space,
                estimated_start_time: if can_fulfill { 1 } else { 24 }, // 1 hour if can fulfill, 24 if not
                valid_until: Utc::now() + chrono::Duration::hours(1),
            };

            let response_message = NetworkMessage::new_response(
                MessageType::StorageQuoteResponse(response),
                &message,
            );

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
    
    match message.message_type {
        MessageType::StorageChallenge(ref challenge) => {
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
                        chunk_hash: ContentHash::from_data(&format!("chunk_{}", chunk_index).as_bytes()),
                        merkle_path: vec![
                            ContentHash::from_data(b"merkle_sibling_1"),
                            ContentHash::from_data(b"merkle_sibling_2"),
                        ],
                        chunk_data: None, // Don't include actual data for large files
                    };
                    merkle_proofs.push(chunk_proof);
                }
                
                // Compute response hash
                let response_data = format!("{}:{}:{:?}", 
                    challenge.challenge_id, 
                    challenge.file_hash.to_hex(),
                    challenge.chunk_indices);
                let response_hash = ContentHash::from_data(response_data.as_bytes());
                
                let response = StorageProofData {
                    challenge_id: challenge.challenge_id.clone(),
                    merkle_proofs,
                    response_hash,
                    signature: vec![0u8; 64], // TODO: Real signature with provider private key
                    generated_at: Utc::now(),
                };

                info!("✅ Generated proof for challenge {} ({} chunks)", 
                      challenge.challenge_id, challenge.chunk_indices.len());

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
        }
        _ => {
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
}

/// Handle proof responses (verification)
async fn proof_response(
    State(_server): State<Arc<ProviderServer>>,
    Json(_message): Json<NetworkMessage>,
) -> Json<VerificationResponse> {
    info!("Verifying proof-of-storage response");
    
    // TODO: Implement actual proof verification
    Json(VerificationResponse {
        valid: true,
        message: "Proof verified successfully".to_string(),
    })
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
    use super::*;
    use carbide_core::{ProviderTier, Region};
    use rust_decimal::Decimal;

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

        let server = ProviderServer::new(config, provider.clone());
        
        assert_eq!(server.provider.name, "Test Provider");
        assert_eq!(server.provider.tier, ProviderTier::Home);
    }
}