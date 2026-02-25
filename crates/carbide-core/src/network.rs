//! Network communication protocols and message types for Carbide Network
//!
//! This module defines the message formats, HTTP endpoints, and networking
//! abstractions used for communication between clients, providers, and the marketplace.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ContentHash, FileId, ProviderId};

// ============================================================================
// Core Message Types
// ============================================================================

/// Network message envelope for all carbide communications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    /// Message identifier for tracking
    pub id: Uuid,
    /// Message type and payload
    pub message_type: MessageType,
    /// Timestamp when message was created
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID for request/response pairing
    pub correlation_id: Option<Uuid>,
    /// Message version for protocol compatibility
    pub version: String,
}

impl NetworkMessage {
    /// Create a new network message
    pub fn new(message_type: MessageType) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            timestamp: Utc::now(),
            correlation_id: None,
            version: "1.0".to_string(),
        }
    }

    /// Create a response to another message
    pub fn new_response(message_type: MessageType, request: &NetworkMessage) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            timestamp: Utc::now(),
            correlation_id: Some(request.id),
            version: request.version.clone(),
        }
    }
}

/// All supported message types in the Carbide Network
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageType {
    // Provider Registration and Discovery
    /// Provider announces itself to the network
    ProviderAnnouncement(ProviderAnnouncement),
    /// Request list of available providers
    ProviderListRequest(ProviderListRequest),
    /// Response with provider list
    ProviderListResponse(ProviderListResponse),

    // File Storage Operations
    /// Request to store a file
    StoreFileRequest(StoreFileRequest),
    /// Response to store file request
    StoreFileResponse(StoreFileResponse),
    /// Request to retrieve a file
    RetrieveFileRequest(RetrieveFileRequest),
    /// Response with file data
    RetrieveFileResponse(RetrieveFileResponse),
    /// Request to delete a file
    DeleteFileRequest(DeleteFileRequest),
    /// Response to delete request
    DeleteFileResponse(DeleteFileResponse),

    // Proof of Storage
    /// Challenge for proof of storage
    StorageChallenge(StorageChallengeData),
    /// Proof response from provider
    StorageProof(StorageProofData),

    // Health and Status
    /// Health check request
    HealthCheckRequest,
    /// Health check response
    HealthCheckResponse(HealthCheckResponse),

    // Marketplace Operations
    /// Request storage quotes from providers
    StorageQuoteRequest(StorageQuoteRequest),
    /// Quote response from provider
    StorageQuoteResponse(StorageQuoteResponse),

    // Error responses
    /// Error message
    Error(ErrorMessage),
}

// ============================================================================
// Provider Messages
// ============================================================================

/// Provider announces itself to the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAnnouncement {
    /// Provider information
    pub provider: super::Provider,
    /// Endpoint for direct communication
    pub endpoint: String,
    /// Supported protocol versions
    pub supported_versions: Vec<String>,
    /// Public key for verification
    pub public_key: Option<String>,
    /// Ethereum wallet address for payments
    pub wallet_address: Option<String>,
}

/// Request for available providers in a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderListRequest {
    /// Optional region filter
    pub region: Option<super::Region>,
    /// Optional tier filter
    pub tier: Option<super::ProviderTier>,
    /// Maximum number of providers to return
    pub limit: Option<usize>,
    /// Minimum reputation score
    pub min_reputation: Option<rust_decimal::Decimal>,
}

/// Response with list of available providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderListResponse {
    /// List of available providers
    pub providers: Vec<super::Provider>,
    /// Total number of providers matching criteria
    pub total_count: usize,
    /// Whether there are more providers available
    pub has_more: bool,
}

// ============================================================================
// File Operation Messages
// ============================================================================

/// Request to store a file with a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreFileRequest {
    /// File metadata
    pub file_id: FileId,
    /// File size in bytes
    pub file_size: u64,
    /// Desired storage duration (months)
    pub duration_months: u32,
    /// Encryption metadata (if encrypted)
    pub encryption_info: Option<EncryptionInfo>,
    /// Storage requirements
    pub requirements: super::ProviderRequirements,
    /// Maximum price willing to pay
    pub max_price: rust_decimal::Decimal,
}

/// Response to file storage request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreFileResponse {
    /// Whether storage was accepted
    pub accepted: bool,
    /// Storage contract if accepted
    pub contract: Option<super::StorageContract>,
    /// Upload URL for file transfer
    pub upload_url: Option<String>,
    /// Upload token for authentication
    pub upload_token: Option<String>,
    /// Reason for rejection if not accepted
    pub rejection_reason: Option<String>,
    /// Alternative price suggestion
    pub counter_offer_price: Option<rust_decimal::Decimal>,
    /// Payment instructions for the client (escrow deposit details)
    pub payment_instructions: Option<crate::payment::PaymentInstructions>,
}

/// Request to retrieve a stored file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveFileRequest {
    /// File to retrieve
    pub file_id: FileId,
    /// Optional byte range (start, end)
    pub byte_range: Option<(u64, u64)>,
    /// Access token for authentication
    pub access_token: String,
}

/// Response with file data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveFileResponse {
    /// File metadata
    pub file_id: FileId,
    /// File data (for small files) or download URL
    pub data: Option<Vec<u8>>,
    /// Download URL for large files
    pub download_url: Option<String>,
    /// Content type
    pub content_type: String,
    /// File size
    pub size: u64,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
}

/// Request to delete a stored file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFileRequest {
    /// File to delete
    pub file_id: FileId,
    /// Reason for deletion
    pub reason: Option<String>,
    /// Access token for authentication
    pub access_token: String,
}

/// Response to delete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFileResponse {
    /// Whether deletion was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Amount of storage freed (bytes)
    pub freed_bytes: Option<u64>,
}

// ============================================================================
// Health and Status Messages
// ============================================================================

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Service status
    pub status: ServiceStatus,
    /// Current timestamp
    pub timestamp: DateTime<Utc>,
    /// Service version
    pub version: String,
    /// Available storage space
    pub available_storage: Option<u64>,
    /// Current load (0.0 - 1.0)
    pub load: Option<f32>,
    /// Reputation score
    pub reputation: Option<rust_decimal::Decimal>,
}

/// Service status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is healthy and accepting requests
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is overloaded
    Overloaded,
    /// Service is in maintenance mode
    Maintenance,
    /// Service is unavailable
    Unavailable,
}

// ============================================================================
// Marketplace Messages
// ============================================================================

/// Request for storage quotes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuoteRequest {
    /// File size to store
    pub file_size: u64,
    /// Desired replication factor
    pub replication_factor: u8,
    /// Storage duration in months
    pub duration_months: u32,
    /// Storage requirements
    pub requirements: super::ProviderRequirements,
    /// Preferred regions
    pub preferred_regions: Vec<super::Region>,
}

/// Quote response from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuoteResponse {
    /// Provider offering the quote
    pub provider_id: ProviderId,
    /// Quoted price per GB per month
    pub price_per_gb_month: rust_decimal::Decimal,
    /// Total monthly cost
    pub total_monthly_cost: rust_decimal::Decimal,
    /// Whether provider can meet requirements
    pub can_fulfill: bool,
    /// Available capacity
    pub available_capacity: u64,
    /// Estimated time to start storage (hours)
    pub estimated_start_time: u32,
    /// Quote validity period (hours)
    pub valid_until: DateTime<Utc>,
}

// ============================================================================
// Proof of Storage Types
// ============================================================================

/// Network-serializable storage challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageChallengeData {
    /// Unique challenge identifier  
    pub challenge_id: String,
    /// File being challenged
    pub file_hash: ContentHash,
    /// Specific chunk indices to prove (random subset)
    pub chunk_indices: Vec<usize>,
    /// Random nonce for this challenge
    pub nonce: [u8; 32],
    /// When the challenge was issued
    pub issued_at: DateTime<Utc>,
    /// Challenge expiry time
    pub expires_at: DateTime<Utc>,
    /// Expected response hash (for verification)
    pub expected_response_hash: ContentHash,
}

/// Network-serializable storage proof response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProofData {
    /// Original challenge identifier
    pub challenge_id: String,
    /// Merkle proofs for the requested chunks
    pub merkle_proofs: Vec<ChunkProofData>,
    /// Response hash computed from the challenged data
    pub response_hash: ContentHash,
    /// Provider signature over the response
    pub signature: Vec<u8>,
    /// When the proof was generated
    pub generated_at: DateTime<Utc>,
}

/// Network-serializable chunk proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProofData {
    /// Index of the chunk being proven
    pub chunk_index: usize,
    /// Hash of the chunk data
    pub chunk_hash: ContentHash,
    /// Merkle tree proof path
    pub merkle_path: Vec<ContentHash>,
    /// Chunk data (if requested for small chunks)
    pub chunk_data: Option<Vec<u8>>,
}

// ============================================================================
// Supporting Types
// ============================================================================

/// File encryption information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    /// Encryption algorithm used
    pub algorithm: String,
    /// Key derivation info (for client)
    pub key_derivation: Option<KeyDerivationInfo>,
    /// Whether file is encrypted
    pub is_encrypted: bool,
}

/// Key derivation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationInfo {
    /// Derivation method (e.g., "PBKDF2")
    pub method: String,
    /// Salt used for key derivation
    pub salt: String,
    /// Number of iterations
    pub iterations: u32,
}

/// Error message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    pub details: Option<HashMap<String, String>>,
}

// ============================================================================
// HTTP Endpoints and REST API
// ============================================================================

/// Standard HTTP endpoints for the Carbide Network API
pub struct ApiEndpoints;

impl ApiEndpoints {
    // Provider endpoints
    /// Provider announcement endpoint
    pub const PROVIDER_ANNOUNCE: &'static str = "/api/v1/provider/announce";
    /// Provider list endpoint
    pub const PROVIDER_LIST: &'static str = "/api/v1/providers";
    /// Provider status endpoint
    pub const PROVIDER_STATUS: &'static str = "/api/v1/provider/status";

    // File operations
    /// File storage request endpoint
    pub const FILE_STORE: &'static str = "/api/v1/files/store";
    /// File retrieval endpoint
    pub const FILE_RETRIEVE: &'static str = "/api/v1/files";
    /// File deletion endpoint
    pub const FILE_DELETE: &'static str = "/api/v1/files";
    /// File upload endpoint
    pub const FILE_UPLOAD: &'static str = "/api/v1/upload";
    /// File download endpoint
    pub const FILE_DOWNLOAD: &'static str = "/api/v1/download";

    // Marketplace
    /// Storage quote request endpoint
    pub const STORAGE_QUOTE: &'static str = "/api/v1/marketplace/quote";
    /// Storage contract endpoint
    pub const STORAGE_CONTRACT: &'static str = "/api/v1/marketplace/contract";

    // Proof of storage
    /// Proof challenge endpoint
    pub const PROOF_CHALLENGE: &'static str = "/api/v1/proof/challenge";
    /// Proof response endpoint
    pub const PROOF_RESPONSE: &'static str = "/api/v1/proof/response";

    // Health and monitoring
    /// Health check endpoint
    pub const HEALTH_CHECK: &'static str = "/api/v1/health";
    /// Metrics endpoint
    pub const METRICS: &'static str = "/api/v1/metrics";
}

/// HTTP status codes used in the API
#[derive(Debug, Clone, Copy)]
pub enum StatusCode {
    /// 200 OK - Request successful
    Ok = 200,
    /// 201 Created - Resource created
    Created = 201,
    /// 400 Bad Request - Invalid request
    BadRequest = 400,
    /// 401 Unauthorized - Authentication required
    Unauthorized = 401,
    /// 403 Forbidden - Access denied
    Forbidden = 403,
    /// 404 Not Found - Resource not found
    NotFound = 404,
    /// 409 Conflict - Resource conflict
    Conflict = 409,
    /// 413 Payload Too Large - Request too large
    PayloadTooLarge = 413,
    /// 500 Internal Server Error - Server error
    InternalError = 500,
    /// 503 Service Unavailable - Service unavailable
    ServiceUnavailable = 503,
}

/// Standard error codes used across the network
pub struct ErrorCodes;

impl ErrorCodes {
    /// Invalid or malformed request
    pub const INVALID_REQUEST: &'static str = "INVALID_REQUEST";
    /// Authentication required
    pub const UNAUTHORIZED: &'static str = "UNAUTHORIZED";
    /// Provider not found or offline
    pub const PROVIDER_NOT_FOUND: &'static str = "PROVIDER_NOT_FOUND";
    /// File not found in storage
    pub const FILE_NOT_FOUND: &'static str = "FILE_NOT_FOUND";
    /// Insufficient storage capacity
    pub const INSUFFICIENT_STORAGE: &'static str = "INSUFFICIENT_STORAGE";
    /// Offered price is too low
    pub const PRICE_TOO_LOW: &'static str = "PRICE_TOO_LOW";
    /// Invalid proof of storage
    pub const INVALID_PROOF: &'static str = "INVALID_PROOF";
    /// Storage is full
    pub const STORAGE_FULL: &'static str = "STORAGE_FULL";
    /// Network communication error
    pub const NETWORK_ERROR: &'static str = "NETWORK_ERROR";
    /// Request rate limited
    pub const RATE_LIMITED: &'static str = "RATE_LIMITED";
}

/// Network configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Maximum message size (bytes)
    pub max_message_size: usize,
    /// Request timeout (seconds)
    pub request_timeout: u64,
    /// Keep-alive timeout (seconds)
    pub keep_alive_timeout: u64,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Enable compression
    pub compression: bool,
    /// API rate limits (requests per minute)
    pub rate_limit: Option<u32>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024 * 1024, // 64MB
            request_timeout: 30,                // 30 seconds
            keep_alive_timeout: 60,             // 60 seconds
            max_connections: 1000,              // 1000 concurrent connections
            compression: true,
            rate_limit: Some(60), // 60 requests per minute
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_message_creation() {
        let message = NetworkMessage::new(MessageType::HealthCheckRequest);

        assert_eq!(message.version, "1.0");
        assert!(message.correlation_id.is_none());
        assert!(message.timestamp <= Utc::now());
    }

    #[test]
    fn test_response_message() {
        let request = NetworkMessage::new(MessageType::HealthCheckRequest);
        let response = NetworkMessage::new_response(
            MessageType::HealthCheckResponse(HealthCheckResponse {
                status: ServiceStatus::Healthy,
                timestamp: Utc::now(),
                version: "1.0".to_string(),
                available_storage: Some(1000000),
                load: Some(0.5),
                reputation: Some(rust_decimal::Decimal::new(85, 2)),
            }),
            &request,
        );

        assert_eq!(response.correlation_id, Some(request.id));
        assert_eq!(response.version, request.version);
    }

    #[test]
    fn test_message_serialization() {
        let health_response = HealthCheckResponse {
            status: ServiceStatus::Healthy,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
            available_storage: Some(1000000),
            load: Some(0.5),
            reputation: Some(rust_decimal::Decimal::new(85, 2)),
        };

        let message = NetworkMessage::new(MessageType::HealthCheckResponse(health_response));

        // Test JSON serialization
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: NetworkMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.version, deserialized.version);
    }

    #[test]
    fn test_network_config_defaults() {
        let config = NetworkConfig::default();

        assert_eq!(config.max_message_size, 64 * 1024 * 1024);
        assert_eq!(config.request_timeout, 30);
        assert!(config.compression);
        assert_eq!(config.rate_limit, Some(60));
    }
}
