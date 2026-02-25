//! Unified HTTP client for all discovery service API communication.
//!
//! Consolidates provider registration, proof submission, and reputation event
//! forwarding into a single client with consistent error handling.

use serde::{Deserialize, Serialize};

/// Payload for submitting a reputation event to the discovery service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEventPayload {
    /// Provider whose reputation is affected
    pub provider_id: String,
    /// Type of event (e.g., "proof_success", "proof_failure", "online")
    pub event_type: String,
    /// Severity: "positive", "negative", or "neutral"
    pub severity: String,
    /// Optional numeric value (e.g., response time in ms)
    pub value: Option<f64>,
    /// Optional JSON details
    pub details: Option<serde_json::Value>,
    /// Associated contract ID if applicable
    pub contract_id: Option<String>,
}

/// Unified HTTP client for communicating with the Carbide Discovery Service.
pub struct DiscoveryApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl DiscoveryApiClient {
    /// Create a new discovery API client pointing at the given base URL.
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Submit a reputation event to the discovery service.
    pub async fn submit_reputation_event(
        &self,
        event: &ReputationEventPayload,
    ) -> Result<(), String> {
        let url = format!("{}/api/v1/reputation/events", self.base_url);

        match self
            .client
            .post(&url)
            .json(event)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!(
                    "Discovery reputation API returned {status}: {body}"
                ))
            }
            Err(e) => Err(format!("Discovery reputation API request failed: {e}")),
        }
    }

    /// Submit a proof to the discovery service for a given contract.
    pub async fn submit_proof(
        &self,
        contract_id: &str,
        proof: &serde_json::Value,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/v1/contracts/{}/proofs",
            self.base_url, contract_id
        );

        match self
            .client
            .post(&url)
            .json(proof)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(format!("Discovery proof API returned {status}: {body}"))
            }
            Err(e) => Err(format!("Discovery proof API request failed: {e}")),
        }
    }

    /// Returns the base URL this client is configured with.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_client_and_verify_url() {
        let client = DiscoveryApiClient::new("http://localhost:3000".to_string());
        assert_eq!(client.base_url(), "http://localhost:3000");
    }

    #[test]
    fn reputation_event_payload_serializes_correctly() {
        let event = ReputationEventPayload {
            provider_id: "test-provider-id".to_string(),
            event_type: "proof_success".to_string(),
            severity: "positive".to_string(),
            value: Some(42.5),
            details: None,
            contract_id: Some("contract-123".to_string()),
        };

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["provider_id"], "test-provider-id");
        assert_eq!(json["event_type"], "proof_success");
        assert_eq!(json["severity"], "positive");
        assert_eq!(json["value"], 42.5);
        assert!(json["details"].is_null());
        assert_eq!(json["contract_id"], "contract-123");
    }

    #[test]
    fn reputation_event_payload_with_details() {
        let event = ReputationEventPayload {
            provider_id: "p1".to_string(),
            event_type: "upload_success".to_string(),
            severity: "positive".to_string(),
            value: Some(100.0),
            details: Some(serde_json::json!({"file_size": 1024})),
            contract_id: None,
        };

        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["details"]["file_size"], 1024);
        assert!(json["contract_id"].is_null());
    }
}
