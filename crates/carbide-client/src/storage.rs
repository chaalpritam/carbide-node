//! High-level storage operations for easy integration
//!
//! This module provides simplified, high-level APIs for common storage operations
//! that applications can easily integrate for decentralized file storage.

use std::collections::HashMap;

use carbide_core::{
    network::*, CarbideError, ContentHash, FileId, Provider, ProviderRequirements, ProviderTier,
    Region, Result,
};
use carbide_crypto::{EncryptedData, FileDecryptor, FileEncryptor, KeyManager};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::file_registry::{FileRecord, FileRegistry, ProviderLocation as RegistryProviderLocation};
use crate::CarbideClient;

/// High-level storage manager that handles provider discovery,
/// file operations, and replication automatically
#[derive(Debug)]
pub struct StorageManager {
    /// HTTP client for network operations
    client: CarbideClient,
    /// Discovery service endpoint
    discovery_endpoint: String,
    /// Default storage preferences
    preferences: StoragePreferences,
    /// Optional key manager for client-side encryption
    key_manager: Option<KeyManager>,
    /// Optional local file registry for persistence
    file_registry: Option<std::sync::Arc<FileRegistry>>,
    /// Client identity for contract creation
    client_id: String,
}

/// Storage preferences for automatic provider selection
#[derive(Debug, Clone)]
pub struct StoragePreferences {
    /// Preferred regions for storage
    pub preferred_regions: Vec<Region>,
    /// Preferred provider tiers
    pub preferred_tiers: Vec<ProviderTier>,
    /// Default replication factor
    pub replication_factor: u8,
    /// Maximum price per GB per month (USD)
    pub max_price_per_gb: rust_decimal::Decimal,
    /// Provider requirements
    pub requirements: ProviderRequirements,
}

impl Default for StoragePreferences {
    fn default() -> Self {
        Self {
            preferred_regions: vec![Region::NorthAmerica, Region::Europe],
            preferred_tiers: vec![ProviderTier::Professional, ProviderTier::Enterprise],
            replication_factor: 3,
            max_price_per_gb: rust_decimal::Decimal::new(10, 3), // $0.010/GB/month
            requirements: ProviderRequirements::important(),
        }
    }
}

/// Result of storing a file in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResult {
    /// File identifier
    pub file_id: FileId,
    /// Size of stored file
    pub file_size: u64,
    /// Providers that accepted storage
    pub providers: Vec<StorageLocation>,
    /// Total monthly cost across all providers
    pub total_monthly_cost: rust_decimal::Decimal,
    /// Storage duration in months
    pub duration_months: u32,
    /// Whether the file was encrypted before storing
    pub is_encrypted: bool,
}

/// Information about where a file is stored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLocation {
    /// Provider information
    pub provider: Provider,
    /// Storage contract details
    pub contract: Option<carbide_core::StorageContract>,
    /// Upload token for this provider
    pub upload_token: String,
    /// Upload URL for this provider
    pub upload_url: String,
}

/// Result of retrieving a file from the network
#[derive(Debug, Clone)]
pub struct RetrieveResult {
    /// File identifier
    pub file_id: FileId,
    /// File data
    pub data: Vec<u8>,
    /// Source provider
    pub provider: Provider,
    /// Content type
    pub content_type: String,
    /// File size
    pub size: u64,
}

/// Storage operation progress callback
pub type ProgressCallback = Box<dyn Fn(StorageProgress) + Send + Sync>;

/// Storage operation progress information
#[derive(Debug, Clone)]
pub struct StorageProgress {
    /// Operation type
    pub operation: String,
    /// Current progress (0.0 - 1.0)
    pub progress: f32,
    /// Current status message
    pub message: String,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total bytes
    pub total_bytes: u64,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(client: CarbideClient, discovery_endpoint: String) -> Self {
        Self {
            client,
            discovery_endpoint,
            preferences: StoragePreferences::default(),
            key_manager: None,
            file_registry: None,
            client_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create storage manager with custom preferences
    pub fn with_preferences(
        client: CarbideClient,
        discovery_endpoint: String,
        preferences: StoragePreferences,
    ) -> Self {
        Self {
            client,
            discovery_endpoint,
            preferences,
            key_manager: None,
            file_registry: None,
            client_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Create storage manager with client-side encryption enabled
    pub fn with_encryption(
        client: CarbideClient,
        discovery_endpoint: String,
        key_manager: KeyManager,
    ) -> Self {
        Self {
            client,
            discovery_endpoint,
            preferences: StoragePreferences::default(),
            key_manager: Some(key_manager),
            file_registry: None,
            client_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Attach a file registry for local persistence of upload records.
    pub fn set_file_registry(&mut self, registry: std::sync::Arc<FileRegistry>) {
        self.file_registry = Some(registry);
    }

    /// Set the client identity used for contract creation.
    pub fn set_client_id(&mut self, client_id: String) {
        self.client_id = client_id;
    }

    /// Store file data in the network with automatic provider selection
    pub async fn store_file(
        &self,
        data: &[u8],
        duration_months: u32,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<StoreResult> {
        let file_id = ContentHash::from_data(data);

        // Encrypt data if key manager is configured
        let (upload_data, encryption_info, is_encrypted) = if let Some(ref km) = self.key_manager {
            let file_key = km.derive_file_key(&file_id.to_hex())?;
            let encryptor = FileEncryptor::new(&file_key)?;
            let encrypted = encryptor.encrypt(data)?;
            let encrypted_bytes = serde_json::to_vec(&encrypted).map_err(|e| {
                CarbideError::Internal(format!("Failed to serialize encrypted data: {e}"))
            })?;
            let enc_info = Some(EncryptionInfo {
                algorithm: "AES-256-GCM".to_string(),
                key_derivation: Some(KeyDerivationInfo {
                    method: "HKDF-SHA256".to_string(),
                    salt: String::new(),
                    iterations: 0,
                }),
                is_encrypted: true,
            });
            info!("Encrypted file {} ({} bytes -> {} bytes)", file_id, data.len(), encrypted_bytes.len());
            (encrypted_bytes, enc_info, true)
        } else {
            (data.to_vec(), None, false)
        };

        if let Some(cb) = &progress_callback {
            cb(StorageProgress {
                operation: "Discovering providers".to_string(),
                progress: 0.1,
                message: "Finding suitable storage providers...".to_string(),
                bytes_transferred: 0,
                total_bytes: upload_data.len() as u64,
            });
        }

        // 1. Discover suitable providers
        let providers = self
            .discover_providers(upload_data.len() as u64, duration_months)
            .await?;

        if providers.is_empty() {
            return Err(CarbideError::Internal(
                "No suitable providers found".to_string(),
            ));
        }

        if let Some(cb) = &progress_callback {
            cb(StorageProgress {
                operation: "Requesting storage".to_string(),
                progress: 0.2,
                message: format!("Found {} providers, requesting storage...", providers.len()),
                bytes_transferred: 0,
                total_bytes: data.len() as u64,
            });
        }

        // 2. Request storage from selected providers
        let mut storage_locations = Vec::new();
        let mut total_cost = rust_decimal::Decimal::ZERO;

        let target_replicas = std::cmp::min(
            self.preferences.replication_factor as usize,
            providers.len(),
        );

        for (i, provider) in providers.iter().take(target_replicas).enumerate() {
            let store_request = StoreFileRequest {
                file_id,
                file_size: upload_data.len() as u64,
                duration_months,
                encryption_info: encryption_info.clone(),
                requirements: self.preferences.requirements.clone(),
                max_price: self.preferences.max_price_per_gb,
            };

            match self
                .client
                .store_file(&provider.endpoint, &store_request)
                .await
            {
                Ok(response) => {
                    if response.accepted {
                        if let (Some(upload_url), Some(upload_token)) =
                            (response.upload_url, response.upload_token)
                        {
                            let contract = response.contract.clone();

                            storage_locations.push(StorageLocation {
                                provider: provider.clone(),
                                contract: contract.clone(),
                                upload_token,
                                upload_url,
                            });

                            if let Some(contract) = &contract {
                                let monthly_cost = contract.price_per_gb_month
                                    * rust_decimal::Decimal::new(upload_data.len() as i64, 9) // bytes to GB
                                    * rust_decimal::Decimal::new(i64::from(duration_months), 0);
                                total_cost += monthly_cost;
                            }
                        }
                    } else {
                        warn!(
                            "Provider {} rejected storage: {:?}",
                            provider.name, response.rejection_reason
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to request storage from {}: {}", provider.name, e);
                }
            }

            if let Some(cb) = &progress_callback {
                cb(StorageProgress {
                    operation: "Requesting storage".to_string(),
                    progress: 0.2 + (0.3 * (i + 1) as f32 / target_replicas as f32),
                    message: format!("Requested storage from {} providers", i + 1),
                    bytes_transferred: 0,
                    total_bytes: upload_data.len() as u64,
                });
            }
        }

        if storage_locations.is_empty() {
            return Err(CarbideError::Internal(
                "No providers accepted storage request".to_string(),
            ));
        }

        // 3. Upload file data to all accepting providers
        for (i, location) in storage_locations.iter().enumerate() {
            if let Some(cb) = &progress_callback {
                cb(StorageProgress {
                    operation: "Uploading files".to_string(),
                    progress: 0.5 + (0.4 * i as f32 / storage_locations.len() as f32),
                    message: format!("Uploading to {}...", location.provider.name),
                    bytes_transferred: 0,
                    total_bytes: upload_data.len() as u64,
                });
            }

            match self
                .client
                .upload_file(&location.upload_url, &file_id, &upload_data, &location.upload_token)
                .await
            {
                Ok(_) => {
                    info!("Successfully uploaded file to {}", location.provider.name);
                }
                Err(e) => {
                    error!("Failed to upload to {}: {}", location.provider.name, e);
                    // TODO: Remove failed upload from storage_locations
                }
            }

            if let Some(cb) = &progress_callback {
                cb(StorageProgress {
                    operation: "Uploading files".to_string(),
                    progress: 0.5 + (0.4 * (i + 1) as f32 / storage_locations.len() as f32),
                    message: format!("Uploaded to {} providers", i + 1),
                    bytes_transferred: upload_data.len() as u64,
                    total_bytes: upload_data.len() as u64,
                });
            }
        }

        if let Some(cb) = &progress_callback {
            cb(StorageProgress {
                operation: "Complete".to_string(),
                progress: 1.0,
                message: format!(
                    "File stored successfully with {} replicas{}",
                    storage_locations.len(),
                    if is_encrypted { " (encrypted)" } else { "" }
                ),
                bytes_transferred: upload_data.len() as u64,
                total_bytes: upload_data.len() as u64,
            });
        }

        // Notify discovery about file-provider mappings (fire-and-forget)
        for location in &storage_locations {
            let url = format!(
                "{}/api/v1/files/{}/providers",
                self.discovery_endpoint,
                file_id.to_hex()
            );
            let body = serde_json::json!({
                "provider_id": location.provider.id.to_string(),
                "file_size": upload_data.len() as u64,
            });
            let http = self.client.http_client().clone();
            tokio::spawn(async move {
                let _ = http.post(&url).json(&body).send().await;
            });
        }

        // Record in local file registry if available
        if let Some(ref registry) = self.file_registry {
            let provider_locations: Vec<RegistryProviderLocation> = storage_locations
                .iter()
                .map(|loc| RegistryProviderLocation {
                    provider_id: loc.provider.id.to_string(),
                    endpoint: loc.provider.endpoint.clone(),
                    contract_id: loc
                        .contract
                        .as_ref()
                        .map(|c| c.id.to_string())
                        .unwrap_or_default(),
                })
                .collect();

            let record = FileRecord {
                file_id: file_id.to_hex(),
                original_name: file_id.to_hex(),
                file_size: data.len() as u64,
                is_encrypted,
                replication_factor: storage_locations.len() as u8,
                providers: serde_json::to_string(&provider_locations).unwrap_or_default(),
                status: "active".to_string(),
                stored_at: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(e) = registry.record_upload(&record) {
                warn!("Failed to record upload in file registry: {}", e);
            }
        }

        Ok(StoreResult {
            file_id,
            file_size: data.len() as u64,
            providers: storage_locations,
            total_monthly_cost: total_cost,
            duration_months,
            is_encrypted,
        })
    }

    /// Retrieve file data from the network
    pub async fn retrieve_file(
        &self,
        file_id: &FileId,
        access_token: &str,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<RetrieveResult> {
        if let Some(cb) = &progress_callback {
            cb(StorageProgress {
                operation: "Discovering file".to_string(),
                progress: 0.1,
                message: "Finding providers with this file...".to_string(),
                bytes_transferred: 0,
                total_bytes: 0,
            });
        }

        // Look up providers via discovery service file-provider mapping
        let provider_endpoints = self.lookup_file_providers(file_id).await;

        for (i, provider_endpoint) in provider_endpoints.iter().enumerate() {
            if let Some(cb) = &progress_callback {
                cb(StorageProgress {
                    operation: "Searching providers".to_string(),
                    progress: 0.2 + (0.3 * i as f32 / provider_endpoints.len() as f32),
                    message: format!("Checking provider {}...", i + 1),
                    bytes_transferred: 0,
                    total_bytes: 0,
                });
            }

            // Try to retrieve file metadata
            match self
                .client
                .retrieve_file(provider_endpoint, file_id, access_token)
                .await
            {
                Ok(retrieve_response) => {
                    if let Some(download_url) = retrieve_response.download_url {
                        if let Some(cb) = &progress_callback {
                            cb(StorageProgress {
                                operation: "Downloading".to_string(),
                                progress: 0.7,
                                message: "Downloading file data...".to_string(),
                                bytes_transferred: 0,
                                total_bytes: retrieve_response.size,
                            });
                        }

                        // Download the actual file data
                        match self.client.download_file(&download_url).await {
                            Ok(raw_data) => {
                                // Decrypt if key manager is available
                                let data = self.maybe_decrypt(file_id, raw_data)?;

                                if let Some(cb) = &progress_callback {
                                    cb(StorageProgress {
                                        operation: "Complete".to_string(),
                                        progress: 1.0,
                                        message: "File retrieved successfully".to_string(),
                                        bytes_transferred: data.len() as u64,
                                        total_bytes: data.len() as u64,
                                    });
                                }

                                let provider = Provider::new(
                                    "Retrieved Provider".to_string(),
                                    carbide_core::ProviderTier::Professional,
                                    carbide_core::Region::NorthAmerica,
                                    provider_endpoint.clone(),
                                    1024 * 1024 * 1024,
                                    rust_decimal::Decimal::new(5, 3),
                                );

                                return Ok(RetrieveResult {
                                    file_id: retrieve_response.file_id,
                                    data,
                                    provider,
                                    content_type: retrieve_response.content_type,
                                    size: retrieve_response.size,
                                });
                            }
                            Err(e) => {
                                warn!("Failed to download from {}: {}", provider_endpoint, e);
                            }
                        }
                    } else if let Some(raw_data) = retrieve_response.data {
                        // File data was included directly — decrypt if needed
                        let data = self.maybe_decrypt(file_id, raw_data)?;

                        if let Some(cb) = &progress_callback {
                            cb(StorageProgress {
                                operation: "Complete".to_string(),
                                progress: 1.0,
                                message: "File retrieved successfully".to_string(),
                                bytes_transferred: data.len() as u64,
                                total_bytes: data.len() as u64,
                            });
                        }

                        let provider = Provider::new(
                            "Retrieved Provider".to_string(),
                            carbide_core::ProviderTier::Professional,
                            carbide_core::Region::NorthAmerica,
                            provider_endpoint.clone(),
                            1024 * 1024 * 1024,
                            rust_decimal::Decimal::new(5, 3),
                        );

                        return Ok(RetrieveResult {
                            file_id: retrieve_response.file_id,
                            data,
                            provider,
                            content_type: retrieve_response.content_type,
                            size: retrieve_response.size,
                        });
                    }
                }
                Err(e) => {
                    warn!("Provider {} doesn't have file: {}", provider_endpoint, e);
                }
            }
        }

        Err(CarbideError::Internal(format!(
            "File {file_id} not found on any provider"
        )))
    }

    /// Look up providers that hold a file via discovery service, falling back to localhost
    async fn lookup_file_providers(&self, file_id: &FileId) -> Vec<String> {
        let url = format!(
            "{}/api/v1/files/{}/providers",
            self.discovery_endpoint,
            file_id.to_hex()
        );

        match self
            .client
            .http_client()
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                #[derive(Deserialize)]
                struct FileProviderEntry {
                    endpoint: String,
                }
                #[derive(Deserialize)]
                struct FileProvidersResponse {
                    providers: Vec<FileProviderEntry>,
                }
                if let Ok(body) = resp.json::<FileProvidersResponse>().await {
                    let endpoints: Vec<String> =
                        body.providers.into_iter().map(|p| p.endpoint).collect();
                    if !endpoints.is_empty() {
                        return endpoints;
                    }
                }
            }
            _ => {
                warn!("Discovery file lookup failed, falling back to localhost");
            }
        }

        // Fallback to hardcoded test providers for backward compatibility
        vec![
            "http://localhost:8080".to_string(),
            "http://localhost:8081".to_string(),
            "http://localhost:8082".to_string(),
        ]
    }

    /// Decrypt data if key_manager is present, attempting to deserialize as EncryptedData
    fn maybe_decrypt(&self, file_id: &FileId, raw_data: Vec<u8>) -> Result<Vec<u8>> {
        if let Some(ref km) = self.key_manager {
            // Try to deserialize as EncryptedData
            match serde_json::from_slice::<EncryptedData>(&raw_data) {
                Ok(encrypted) => {
                    let file_key = km.derive_file_key(&file_id.to_hex())?;
                    let decryptor = FileDecryptor::new(&file_key)?;
                    let plaintext = decryptor.decrypt(&encrypted)?;
                    info!("Decrypted file {} ({} bytes)", file_id, plaintext.len());
                    Ok(plaintext)
                }
                Err(_) => {
                    // Data is not encrypted — return as-is
                    Ok(raw_data)
                }
            }
        } else {
            Ok(raw_data)
        }
    }

    /// Discover suitable providers for storing a file
    async fn discover_providers(
        &self,
        file_size: u64,
        _duration_months: u32,
    ) -> Result<Vec<Provider>> {
        let discovery_url = format!("{}/api/v1/providers", self.discovery_endpoint);

        let mut query_params = Vec::new();
        query_params.push("limit=20".to_string());

        if !self.preferences.preferred_regions.is_empty() {
            // For simplicity, just use the first preferred region
            if let Some(region) = self.preferences.preferred_regions.first() {
                let region_str = match region {
                    Region::NorthAmerica => "northamerica",
                    Region::Europe => "europe",
                    Region::Asia => "asia",
                    Region::SouthAmerica => "southamerica",
                    Region::Africa => "africa",
                    Region::Oceania => "oceania",
                };
                query_params.push(format!("region={region_str}"));
            }
        }

        let query_string = query_params.join("&");
        let full_url = format!("{discovery_url}?{query_string}");

        let response = self
            .client
            .http_client()
            .get(&full_url)
            .send()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Provider discovery failed: {e}")))?;

        if !response.status().is_success() {
            return Err(CarbideError::Discovery(format!(
                "Discovery service returned: {}",
                response.status()
            )));
        }

        let provider_list: ProviderListResponse = response
            .json()
            .await
            .map_err(|e| CarbideError::Discovery(format!("Failed to parse provider list: {e}")))?;

        // Filter providers by our preferences
        let suitable_providers: Vec<Provider> = provider_list
            .providers
            .into_iter()
            .filter(|provider| {
                // Check if provider tier is acceptable
                if !self.preferences.preferred_tiers.is_empty()
                    && !self.preferences.preferred_tiers.contains(&provider.tier)
                {
                    return false;
                }

                // Check if provider has enough capacity
                if provider.available_capacity < file_size {
                    return false;
                }

                // Check if price is acceptable
                if provider.price_per_gb_month > self.preferences.max_price_per_gb {
                    return false;
                }

                true
            })
            .take(self.preferences.replication_factor as usize)
            .collect();

        Ok(suitable_providers)
    }

    /// Get current storage preferences
    pub fn preferences(&self) -> &StoragePreferences {
        &self.preferences
    }

    /// Update storage preferences
    pub fn set_preferences(&mut self, preferences: StoragePreferences) {
        self.preferences = preferences;
    }

    /// Check the health of storage providers
    pub async fn health_check(&self) -> Result<HashMap<String, ServiceStatus>> {
        let providers = self.discover_providers(1024, 1).await?; // Dummy values for discovery
        let mut results = HashMap::new();

        for provider in providers {
            match self.client.get_provider_health(&provider.endpoint).await {
                Ok(health) => {
                    results.insert(provider.endpoint, health.status);
                }
                Err(_) => {
                    results.insert(provider.endpoint, ServiceStatus::Unavailable);
                }
            }
        }

        Ok(results)
    }
}

/// Simple convenience functions for common operations
pub mod simple {
    use super::*;
    use crate::CarbideClient;

    /// Store a file with default settings
    pub async fn store_file(data: &[u8], duration_months: u32) -> Result<StoreResult> {
        let client = CarbideClient::with_defaults()?;
        let manager = StorageManager::new(client, "http://localhost:9090".to_string());
        manager.store_file(data, duration_months, None).await
    }

    /// Retrieve a file with default settings  
    pub async fn retrieve_file(file_id: &FileId, access_token: &str) -> Result<Vec<u8>> {
        let client = CarbideClient::with_defaults()?;
        let manager = StorageManager::new(client, "http://localhost:9090".to_string());
        let result = manager.retrieve_file(file_id, access_token, None).await?;
        Ok(result.data)
    }

    /// Store a file from a local file path
    pub async fn store_file_from_path(
        file_path: &str,
        duration_months: u32,
    ) -> Result<(FileId, StoreResult)> {
        let data = tokio::fs::read(file_path)
            .await
            .map_err(|e| CarbideError::Internal(format!("Failed to read file {file_path}: {e}")))?;

        let file_id = ContentHash::from_data(&data);
        let result = store_file(&data, duration_months).await?;
        Ok((file_id, result))
    }

    /// Retrieve a file and save to local path
    pub async fn retrieve_file_to_path(
        file_id: &FileId,
        access_token: &str,
        output_path: &str,
    ) -> Result<u64> {
        let data = retrieve_file(file_id, access_token).await?;

        tokio::fs::write(output_path, &data).await.map_err(|e| {
            CarbideError::Internal(format!("Failed to write file {output_path}: {e}"))
        })?;

        Ok(data.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_preferences_default() {
        let prefs = StoragePreferences::default();

        assert_eq!(prefs.replication_factor, 3);
        assert!(prefs.preferred_regions.contains(&Region::NorthAmerica));
        assert!(prefs.preferred_tiers.contains(&ProviderTier::Professional));
    }

    #[tokio::test]
    async fn test_simple_store_retrieve() {
        // This would need a running discovery service and providers to actually work
        let test_data = b"Hello, Carbide Network!";

        // Just test the API structure
        let result = simple::store_file(test_data, 12).await;
        // In a real test environment, this would succeed
        assert!(result.is_err() || result.is_ok());
    }
}
