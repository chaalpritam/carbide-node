//! HTTP client for interacting with Carbide Network providers
//!
//! This module provides a high-level HTTP client for communicating with
//! storage providers and marketplace services in the Carbide Network.

use carbide_core::{
    network::*,
    ContentHash, FileId, ProviderId, Region, ProviderTier, Result, CarbideError,
};
use reqwest::{Client, Response};
use serde_json;
use std::time::Duration;
use tracing::{debug, info, warn, error};

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retries for failed requests
    pub max_retries: u32,
    /// User agent string
    pub user_agent: String,
    /// Enable request/response logging
    pub enable_logging: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            user_agent: "Carbide-Client/1.0".to_string(),
            enable_logging: true,
        }
    }
}

/// High-level HTTP client for Carbide Network
#[derive(Debug)]
pub struct CarbideClient {
    /// HTTP client
    client: Client,
    /// Client configuration
    config: ClientConfig,
}

impl CarbideClient {
    /// Create a new Carbide client
    pub fn new(config: ClientConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| CarbideError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Create a client with default configuration
    pub fn default() -> Result<Self> {
        Self::new(ClientConfig::default())
    }

    // ============================================================================
    // Provider Discovery and Health
    // ============================================================================

    /// Get provider health status
    pub async fn get_provider_health(&self, provider_endpoint: &str) -> Result<HealthCheckResponse> {
        let url = format!("{}{}", provider_endpoint, ApiEndpoints::HEALTH_CHECK);
        
        if self.config.enable_logging {
            debug!("Checking provider health: {}", url);
        }

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Health check failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Health check returned status: {}", response.status())));
        }

        let network_message: NetworkMessage = response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse health response: {}", e)))?;

        match network_message.message_type {
            MessageType::HealthCheckResponse(health) => {
                if self.config.enable_logging {
                    info!("Provider health: {:?}", health.status);
                }
                Ok(health)
            }
            _ => Err(CarbideError::Internal("Unexpected message type for health check".to_string())),
        }
    }

    /// Get detailed provider status
    pub async fn get_provider_status(&self, provider_endpoint: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", provider_endpoint, ApiEndpoints::PROVIDER_STATUS);
        
        if self.config.enable_logging {
            debug!("Getting provider status: {}", url);
        }

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Status request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Status request returned: {}", response.status())));
        }

        response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse status response: {}", e)))
    }

    // ============================================================================
    // Storage Operations
    // ============================================================================

    /// Request storage quote from a provider
    pub async fn request_storage_quote(
        &self,
        provider_endpoint: &str,
        request: &StorageQuoteRequest,
    ) -> Result<StorageQuoteResponse> {
        let url = format!("{}{}", provider_endpoint, ApiEndpoints::STORAGE_QUOTE);
        
        let network_message = NetworkMessage::new(
            MessageType::StorageQuoteRequest(request.clone())
        );

        if self.config.enable_logging {
            debug!("Requesting storage quote from: {}", url);
        }

        let response = self.client
            .post(&url)
            .json(&network_message)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Quote request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Quote request returned: {}", response.status())));
        }

        let response_message: NetworkMessage = response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse quote response: {}", e)))?;

        match response_message.message_type {
            MessageType::StorageQuoteResponse(quote) => {
                if self.config.enable_logging {
                    info!("Received quote: ${}/month for {} bytes", 
                          quote.total_monthly_cost, request.file_size);
                }
                Ok(quote)
            }
            MessageType::Error(error) => {
                Err(CarbideError::Network(format!("Provider error: {}", error.message)))
            }
            _ => Err(CarbideError::Internal("Unexpected response type for quote request".to_string())),
        }
    }

    /// Request file storage from a provider
    pub async fn store_file(
        &self,
        provider_endpoint: &str,
        request: &StoreFileRequest,
    ) -> Result<StoreFileResponse> {
        let url = format!("{}{}", provider_endpoint, ApiEndpoints::FILE_STORE);
        
        let network_message = NetworkMessage::new(
            MessageType::StoreFileRequest(request.clone())
        );

        if self.config.enable_logging {
            debug!("Requesting file storage from: {}", url);
        }

        let response = self.client
            .post(&url)
            .json(&network_message)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Store request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Store request returned: {}", response.status())));
        }

        let response_message: NetworkMessage = response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse store response: {}", e)))?;

        match response_message.message_type {
            MessageType::StoreFileResponse(store_response) => {
                if self.config.enable_logging {
                    if store_response.accepted {
                        info!("Storage request accepted by provider");
                    } else {
                        warn!("Storage request rejected: {:?}", store_response.rejection_reason);
                    }
                }
                Ok(store_response)
            }
            MessageType::Error(error) => {
                Err(CarbideError::Network(format!("Provider error: {}", error.message)))
            }
            _ => Err(CarbideError::Internal("Unexpected response type for store request".to_string())),
        }
    }

    /// Upload file data to a provider
    pub async fn upload_file(
        &self,
        upload_url: &str,
        file_id: &FileId,
        file_data: &[u8],
        upload_token: &str,
    ) -> Result<serde_json::Value> {
        if self.config.enable_logging {
            debug!("Uploading file {} to: {}", file_id, upload_url);
        }

        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .text("file_id", file_id.to_hex())
            .text("token", upload_token.to_string())
            .part("file", reqwest::multipart::Part::bytes(file_data.to_vec())
                .file_name("upload")
                .mime_str("application/octet-stream").unwrap());

        let response = self.client
            .post(upload_url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("File upload failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Upload returned: {}", response.status())));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse upload response: {}", e)))?;

        if self.config.enable_logging {
            info!("File {} uploaded successfully", file_id);
        }

        Ok(result)
    }

    /// Retrieve file from a provider
    pub async fn retrieve_file(
        &self,
        provider_endpoint: &str,
        file_id: &FileId,
        access_token: &str,
    ) -> Result<RetrieveFileResponse> {
        let url = format!("{}{}/{}", provider_endpoint, ApiEndpoints::FILE_RETRIEVE, file_id);
        
        if self.config.enable_logging {
            debug!("Retrieving file {} from: {}", file_id, url);
        }

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Retrieve request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Retrieve request returned: {}", response.status())));
        }

        let response_message: NetworkMessage = response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse retrieve response: {}", e)))?;

        match response_message.message_type {
            MessageType::RetrieveFileResponse(retrieve_response) => {
                if self.config.enable_logging {
                    info!("File {} metadata retrieved", file_id);
                }
                Ok(retrieve_response)
            }
            MessageType::Error(error) => {
                Err(CarbideError::Network(format!("Provider error: {}", error.message)))
            }
            _ => Err(CarbideError::Internal("Unexpected response type for retrieve request".to_string())),
        }
    }

    /// Download file data from a provider
    pub async fn download_file(&self, download_url: &str) -> Result<Vec<u8>> {
        if self.config.enable_logging {
            debug!("Downloading file from: {}", download_url);
        }

        let response = self.client
            .get(download_url)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Download returned: {}", response.status())));
        }

        let data = response
            .bytes()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to read download data: {}", e)))?;

        if self.config.enable_logging {
            info!("Downloaded {} bytes", data.len());
        }

        Ok(data.to_vec())
    }

    // ============================================================================
    // Proof of Storage
    // ============================================================================

    /// Send proof-of-storage challenge to a provider
    pub async fn send_storage_challenge(
        &self,
        provider_endpoint: &str,
        challenge: &StorageChallengeData,
    ) -> Result<StorageProofData> {
        let url = format!("{}{}", provider_endpoint, ApiEndpoints::PROOF_CHALLENGE);
        
        let network_message = NetworkMessage::new(
            MessageType::StorageChallenge(challenge.clone())
        );

        if self.config.enable_logging {
            debug!("Sending storage challenge to: {}", url);
        }

        let response = self.client
            .post(&url)
            .json(&network_message)
            .send()
            .await
            .map_err(|e| CarbideError::Network(format!("Challenge request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CarbideError::Network(format!("Challenge returned: {}", response.status())));
        }

        let response_message: NetworkMessage = response
            .json()
            .await
            .map_err(|e| CarbideError::Network(format!("Failed to parse challenge response: {}", e)))?;

        match response_message.message_type {
            MessageType::StorageProof(proof) => {
                if self.config.enable_logging {
                    info!("Received storage proof for challenge {}", challenge.challenge_id);
                }
                Ok(proof)
            }
            MessageType::Error(error) => {
                Err(CarbideError::Network(format!("Challenge error: {}", error.message)))
            }
            _ => Err(CarbideError::Internal("Unexpected response type for challenge".to_string())),
        }
    }

    // ============================================================================
    // Utility Methods
    // ============================================================================

    /// Test connectivity to multiple providers
    pub async fn test_providers(&self, endpoints: &[String]) -> Vec<ProviderTestResult> {
        let mut results = Vec::new();

        for endpoint in endpoints {
            if self.config.enable_logging {
                debug!("Testing provider: {}", endpoint);
            }

            let start_time = std::time::Instant::now();
            
            let result = match self.get_provider_health(endpoint).await {
                Ok(health) => ProviderTestResult {
                    endpoint: endpoint.clone(),
                    online: true,
                    latency_ms: start_time.elapsed().as_millis() as u32,
                    status: health.status,
                    error: None,
                },
                Err(e) => ProviderTestResult {
                    endpoint: endpoint.clone(),
                    online: false,
                    latency_ms: start_time.elapsed().as_millis() as u32,
                    status: ServiceStatus::Unavailable,
                    error: Some(e.to_string()),
                },
            };

            results.push(result);
        }

        results
    }

    /// Get the underlying HTTP client for custom requests
    pub fn http_client(&self) -> &Client {
        &self.client
    }
}

/// Result of testing a provider's connectivity
#[derive(Debug, Clone)]
pub struct ProviderTestResult {
    /// Provider endpoint
    pub endpoint: String,
    /// Whether provider is online
    pub online: bool,
    /// Response latency in milliseconds
    pub latency_ms: u32,
    /// Provider status
    pub status: ServiceStatus,
    /// Error message if failed
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.user_agent, "Carbide-Client/1.0");
        assert!(config.enable_logging);
    }

    #[test]
    fn test_client_creation() {
        let config = ClientConfig::default();
        let client = CarbideClient::new(config);
        
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_default() {
        let client = CarbideClient::default();
        assert!(client.is_ok());
    }
}