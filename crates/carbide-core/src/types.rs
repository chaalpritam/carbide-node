//! Core data types for the Carbide Network
//!
//! This module contains the fundamental types for the decentralized storage marketplace:
//! - File, Chunk, `ContentHash` - Content-addressed storage
//! - Provider, `StorageRequest`, `StorageContract` - Marketplace interactions  
//! - `ReputationScore`, `NetworkNode` - Trust and networking

use crate::{CarbideError, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ============================================================================
// Content-Addressed Storage Types
// ============================================================================

/// A cryptographic hash that uniquely identifies file content
/// 
/// Uses BLAKE3 for fast, secure content addressing similar to IPFS
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Create a new content hash from bytes
    pub fn new(hash: [u8; 32]) -> Self {
        Self(hash)
    }
    
    /// Create a content hash from data using BLAKE3
    pub fn from_data(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(*hash.as_bytes())
    }
    
    /// Get the raw hash bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// Convert to hex string for display/storage
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)
            .map_err(|_| CarbideError::Internal("Invalid hex string".to_string()))?;
        
        if bytes.len() != 32 {
            return Err(CarbideError::Internal("Hash must be 32 bytes".to_string()));
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self(hash))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Unique identifier for a file in the network
pub type FileId = ContentHash;

/// A file chunk for efficient transfer and storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChunk {
    /// Hash of this chunk's content
    pub hash: ContentHash,
    /// Chunk data (up to 64MB like Storj)
    pub data: Vec<u8>,
    /// Position in the original file
    pub offset: u64,
    /// Total file size
    pub total_size: u64,
}

/// Complete file metadata and content reference
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    /// Unique content-based identifier
    pub id: FileId,
    /// Original filename
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// MIME type
    pub mime_type: String,
    /// File chunks for large files
    pub chunks: Vec<ContentHash>,
    /// Upload timestamp
    pub created_at: DateTime<Utc>,
    /// Optional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl File {
    /// Create a new file from content
    pub fn new(name: String, data: Vec<u8>, mime_type: String) -> Self {
        let id = ContentHash::from_data(&data);
        
        Self {
            id,
            name,
            size: data.len() as u64,
            mime_type,
            chunks: vec![id], // Single chunk for now, will split in crypto module
            created_at: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    /// Check if file needs chunking (>64MB)
    pub fn needs_chunking(&self) -> bool {
        self.size > 64 * 1024 * 1024 // 64MB
    }
}

// ============================================================================
// Provider and Marketplace Types  
// ============================================================================

/// Unique identifier for storage providers
pub type ProviderId = Uuid;

/// Different tiers of storage providers with varying guarantees
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderTier {
    /// Home users with spare storage ($0.002/GB, 95% uptime)
    Home,
    /// Small businesses and enthusiasts ($0.004/GB, 99% uptime)
    Professional, 
    /// Data centers and hosting companies ($0.008/GB, 99.9% uptime)
    Enterprise,
    /// Global CDN providers ($0.012/GB, 99.99% uptime)
    GlobalCDN,
}

impl ProviderTier {
    /// Get typical pricing for this tier (USD per GB per month)
    pub fn typical_price(&self) -> Decimal {
        match self {
            Self::Home => Decimal::new(2, 3),          // $0.002
            Self::Professional => Decimal::new(4, 3),  // $0.004  
            Self::Enterprise => Decimal::new(8, 3),    // $0.008
            Self::GlobalCDN => Decimal::new(12, 3),    // $0.012
        }
    }
    
    /// Get typical uptime guarantee for this tier
    pub fn typical_uptime(&self) -> Decimal {
        match self {
            Self::Home => Decimal::new(95, 2),         // 95%
            Self::Professional => Decimal::new(99, 2), // 99%
            Self::Enterprise => Decimal::new(999, 3),  // 99.9%
            Self::GlobalCDN => Decimal::new(9999, 4),  // 99.99%
        }
    }
}

/// Geographic region for provider location
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Region {
    /// North America (US, Canada, Mexico)
    NorthAmerica,
    /// Europe (EU, UK, etc.)
    Europe,
    /// Asia Pacific region
    Asia,
    /// South America  
    SouthAmerica,
    /// Africa
    Africa,
    /// Oceania (Australia, New Zealand, etc.)
    Oceania,
}

/// Storage provider information and capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provider {
    /// Unique provider identifier
    pub id: ProviderId,
    /// Human-readable name
    pub name: String,
    /// Provider tier and guarantees
    pub tier: ProviderTier,
    /// Geographic location
    pub region: Region,
    /// API endpoint for communication
    pub endpoint: String,
    /// Available storage capacity (bytes)
    pub available_capacity: u64,
    /// Total storage capacity (bytes)
    pub total_capacity: u64,
    /// Price per GB per month (USD)
    pub price_per_gb_month: Decimal,
    /// Current reputation score (0.0 - 1.0)
    pub reputation: ReputationScore,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
    /// Provider-specific metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl Provider {
    /// Create a new provider
    pub fn new(
        name: String,
        tier: ProviderTier, 
        region: Region,
        endpoint: String,
        capacity: u64,
        price: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            tier,
            region,
            endpoint,
            available_capacity: capacity,
            total_capacity: capacity,
            price_per_gb_month: price,
            reputation: ReputationScore::new(),
            last_seen: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    /// Check if provider has enough capacity for a file
    pub fn can_store(&self, file_size: u64) -> bool {
        self.available_capacity >= file_size
    }
    
    /// Check if provider is online (seen within last 5 minutes)
    pub fn is_online(&self) -> bool {
        (Utc::now() - self.last_seen).num_minutes() < 5
    }
    
    /// Calculate monthly cost to store a file
    pub fn calculate_monthly_cost(&self, file_size_gb: Decimal) -> Decimal {
        file_size_gb * self.price_per_gb_month
    }
}

// ============================================================================
// Reputation System Types
// ============================================================================

/// Multi-dimensional reputation score for providers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReputationScore {
    /// Overall reputation (0.0 - 1.0)
    pub overall: Decimal,
    /// Historical uptime percentage
    pub uptime: Decimal,
    /// Data integrity score (successful proofs)
    pub data_integrity: Decimal,
    /// Average response time score
    pub response_time: Decimal,
    /// Contract compliance rate
    pub contract_compliance: Decimal,
    /// Community feedback score
    pub community_feedback: Decimal,
    /// Number of successful storage contracts
    pub contracts_completed: u64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl ReputationScore {
    /// Create a new reputation score with default values
    pub fn new() -> Self {
        Self {
            overall: Decimal::new(5, 1),              // 0.5 starting score
            uptime: Decimal::ONE,                     // 100% until proven otherwise
            data_integrity: Decimal::ONE,             // 100% until proven otherwise
            response_time: Decimal::new(8, 1),        // 0.8 average
            contract_compliance: Decimal::ONE,        // 100% until proven otherwise
            community_feedback: Decimal::new(5, 1),   // 0.5 neutral
            contracts_completed: 0,
            last_updated: Utc::now(),
        }
    }
    
    /// Calculate overall reputation from components
    pub fn calculate_overall(&mut self) {
        // Weighted average of components
        let weights = (
            Decimal::new(25, 2), // uptime: 25%
            Decimal::new(25, 2), // data_integrity: 25% 
            Decimal::new(20, 2), // response_time: 20%
            Decimal::new(20, 2), // contract_compliance: 20%
            Decimal::new(10, 2), // community_feedback: 10%
        );
        
        self.overall = (self.uptime * weights.0)
            + (self.data_integrity * weights.1)
            + (self.response_time * weights.2)
            + (self.contract_compliance * weights.3)
            + (self.community_feedback * weights.4);
            
        self.last_updated = Utc::now();
    }
    
    /// Check if reputation is above threshold for storage
    pub fn is_trustworthy(&self) -> bool {
        self.overall > Decimal::new(6, 1) // > 0.6
    }
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Storage Request and Contract Types
// ============================================================================

/// Client requirements for storage
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderRequirements {
    /// Minimum uptime percentage required
    pub min_uptime: Decimal,
    /// Preferred geographic regions
    pub preferred_regions: Vec<Region>,
    /// Exclude home providers?
    pub exclude_home_providers: bool,
    /// Require backup power?
    pub require_backup_power: bool,
    /// Maximum acceptable latency (ms)
    pub max_latency_ms: u32,
    /// Minimum reputation score
    pub min_reputation: Decimal,
}

impl ProviderRequirements {
    /// Create requirements for critical data
    pub fn critical() -> Self {
        Self {
            min_uptime: Decimal::new(999, 3),        // 99.9%
            preferred_regions: vec![],                // No preference
            exclude_home_providers: true,             // Only professional+
            require_backup_power: true,              // UPS required
            max_latency_ms: 100,                     // Fast response
            min_reputation: Decimal::new(8, 1),      // 0.8+ reputation
        }
    }
    
    /// Create requirements for important data
    pub fn important() -> Self {
        Self {
            min_uptime: Decimal::new(95, 2),         // 95%
            preferred_regions: vec![],
            exclude_home_providers: false,           // Allow home providers
            require_backup_power: false,
            max_latency_ms: 500,
            min_reputation: Decimal::new(6, 1),      // 0.6+ reputation
        }
    }
    
    /// Create requirements for backup data
    pub fn backup() -> Self {
        Self {
            min_uptime: Decimal::new(90, 2),         // 90%
            preferred_regions: vec![],
            exclude_home_providers: false,
            require_backup_power: false,
            max_latency_ms: 2000,                    // 2 seconds OK
            min_reputation: Decimal::new(4, 1),      // 0.4+ reputation
        }
    }
}

/// A client's request to store a file with specific requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageRequest {
    /// Unique request identifier
    pub id: Uuid,
    /// File to be stored
    pub file_id: FileId,
    /// How many copies to store (1-10)
    pub replication_factor: u8,
    /// Maximum price willing to pay per GB per month
    pub max_price_per_gb_month: Decimal,
    /// Provider requirements
    pub requirements: ProviderRequirements,
    /// Request creation time
    pub created_at: DateTime<Utc>,
    /// Optional client metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl StorageRequest {
    /// Create a new storage request
    pub fn new(
        file_id: FileId,
        replication_factor: u8,
        max_price: Decimal,
        requirements: ProviderRequirements,
    ) -> Result<Self> {
        if !(1..=10).contains(&replication_factor) {
            return Err(CarbideError::Internal(
                "Replication factor must be between 1 and 10".to_string()
            ));
        }
        
        Ok(Self {
            id: Uuid::new_v4(),
            file_id,
            replication_factor,
            max_price_per_gb_month: max_price,
            requirements,
            created_at: Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }
    
    /// Calculate total monthly budget for this request
    pub fn calculate_monthly_budget(&self, file_size_gb: Decimal) -> Decimal {
        file_size_gb * self.max_price_per_gb_month * Decimal::from(self.replication_factor)
    }
}

/// Storage contract between client and provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageContract {
    /// Unique contract identifier
    pub id: Uuid,
    /// Original storage request
    pub request_id: Uuid,
    /// File being stored
    pub file_id: FileId,
    /// Storage provider
    pub provider_id: ProviderId,
    /// Agreed price per GB per month
    pub price_per_gb_month: Decimal,
    /// Contract duration
    pub duration_months: u32,
    /// Contract start time
    pub started_at: DateTime<Utc>,
    /// Contract status
    pub status: ContractStatus,
    /// Last proof of storage submission
    pub last_proof_at: Option<DateTime<Utc>>,
}

/// Status of a storage contract
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    /// Contract is active and provider should be storing file
    Active,
    /// Contract completed successfully
    Completed,
    /// Contract cancelled by client
    Cancelled,
    /// Contract terminated due to provider failure
    Failed,
}

// ============================================================================
// Marketplace and Discovery Types
// ============================================================================

/// Result of provider selection for a storage request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderMatch {
    /// Matched provider
    pub provider: Provider,
    /// Match score (0.0 - 1.0)
    pub score: Decimal,
    /// Estimated monthly cost for this provider
    pub monthly_cost: Decimal,
    /// Match reasoning for transparency
    pub match_reason: String,
}

/// Network node information for discovery
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkNode {
    /// Node identifier
    pub id: Uuid,
    /// Node type
    pub node_type: NodeType,
    /// Network endpoint
    pub endpoint: String,
    /// Node region
    pub region: Region,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
    /// Node version
    pub version: String,
}

/// Type of network node
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// Storage provider node
    Provider,
    /// Discovery/marketplace node
    Discovery,
    /// Client node
    Client,
    /// Reputation tracking node
    Reputation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_from_data() {
        let data = b"Hello, Carbide Network!";
        let hash = ContentHash::from_data(data);
        
        // Content addressing: same data = same hash
        let hash2 = ContentHash::from_data(data);
        assert_eq!(hash, hash2);
        
        // Different data = different hash
        let hash3 = ContentHash::from_data(b"Different data");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_provider_tier_pricing() {
        assert_eq!(ProviderTier::Home.typical_price(), Decimal::new(2, 3));
        assert_eq!(ProviderTier::Professional.typical_price(), Decimal::new(4, 3));
        assert_eq!(ProviderTier::Enterprise.typical_price(), Decimal::new(8, 3));
        assert_eq!(ProviderTier::GlobalCDN.typical_price(), Decimal::new(12, 3));
    }

    #[test]
    fn test_marketplace_scenario() {
        // Create a file to store
        let file_data = b"Important business document".to_vec();
        let file = File::new(
            "business_plan.pdf".to_string(),
            file_data,
            "application/pdf".to_string(),
        );
        
        // Create providers with different tiers
        let home_provider = Provider::new(
            "Alice's Home Storage".to_string(),
            ProviderTier::Home,
            Region::NorthAmerica,
            "https://alice.example.com:8080".to_string(),
            2_000_000_000, // 2GB capacity
            Decimal::new(2, 3), // $0.002/GB/month
        );
        
        let pro_provider = Provider::new(
            "Bob's Business Storage".to_string(),
            ProviderTier::Professional,
            Region::Europe,
            "https://bob-storage.com".to_string(),
            10_000_000_000, // 10GB capacity
            Decimal::new(4, 3), // $0.004/GB/month
        );
        
        // Client creates storage request
        let _storage_request = StorageRequest::new(
            file.id,
            2, // Want 2 copies
            Decimal::new(5, 3), // Willing to pay $0.005/GB/month max
            ProviderRequirements::important(),
        ).expect("Should create valid request");
        
        // Verify marketplace logic
        assert!(home_provider.can_store(file.size));
        assert!(pro_provider.can_store(file.size));
        
        let file_size_gb = Decimal::new(file.size as i64, 9); // Convert to GB
        assert!(pro_provider.calculate_monthly_cost(file_size_gb) > 
                home_provider.calculate_monthly_cost(file_size_gb));
    }

    #[test]
    fn test_storage_request_validation() {
        let file_id = ContentHash::from_data(b"test");
        let requirements = ProviderRequirements::backup();
        
        // Valid replication factors
        assert!(StorageRequest::new(file_id, 1, Decimal::new(1, 3), requirements.clone()).is_ok());
        assert!(StorageRequest::new(file_id, 10, Decimal::new(1, 3), requirements.clone()).is_ok());
        
        // Invalid replication factors
        assert!(StorageRequest::new(file_id, 0, Decimal::new(1, 3), requirements.clone()).is_err());
        assert!(StorageRequest::new(file_id, 11, Decimal::new(1, 3), requirements).is_err());
    }
}
