//! Payment client for contract and escrow operations
//!
//! Provides `PaymentClient` that communicates with the discovery service's
//! contract endpoints to create storage contracts, record deposits, and
//! track payment events.

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use carbide_core::{CarbideError, Result};

/// Request to create a new storage contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContractRequest {
    /// Provider to contract with
    pub provider_id: String,
    /// Client creating the contract
    pub client_id: String,
    /// Price per GB per month (decimal string)
    pub price_per_gb_month: String,
    /// Storage duration in days
    pub duration_days: u32,
    /// Total storage size in bytes
    pub total_size_bytes: Option<u64>,
    /// Optional file ID
    pub file_id: Option<String>,
    /// Optional blockchain chain ID
    pub chain_id: Option<u64>,
}

/// Response from contract operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResponse {
    /// Contract ID
    pub id: String,
    /// Provider ID
    pub provider_id: String,
    /// Client ID
    pub client_id: String,
    /// Contract status
    pub status: String,
    /// Price per GB per month
    pub price_per_gb_month: String,
    /// Duration in days
    pub duration_days: u32,
    /// Total escrowed amount (if any)
    pub total_escrowed: Option<String>,
    /// Total released amount (if any)
    pub total_released: Option<String>,
    /// Creation timestamp
    pub created_at: String,
}

/// A payment event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEvent {
    /// Event ID
    pub id: String,
    /// Contract ID
    pub contract_id: String,
    /// Event type (deposit, release, etc.)
    pub event_type: String,
    /// Amount
    pub amount: String,
    /// Event timestamp
    pub created_at: String,
}

/// Client for interacting with discovery service contract/payment endpoints.
pub struct PaymentClient {
    client: Client,
    endpoint: String,
}

impl PaymentClient {
    /// Create a new payment client.
    ///
    /// `discovery_endpoint` is the base URL of the discovery service
    /// (e.g., `http://localhost:3000`).
    pub fn new(discovery_endpoint: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| CarbideError::Internal(format!("HTTP client error: {e}")))?;

        Ok(Self {
            client,
            endpoint: discovery_endpoint.trim_end_matches('/').to_string(),
        })
    }

    /// Create a new storage contract.
    pub async fn create_contract(
        &self,
        request: &CreateContractRequest,
    ) -> Result<ContractResponse> {
        let url = format!("{}/api/v1/contracts", self.endpoint);
        debug!("Creating contract: POST {}", url);

        let resp = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| CarbideError::Internal(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CarbideError::Internal(format!(
                "Create contract failed ({status}): {body}"
            )));
        }

        resp.json::<ContractResponse>()
            .await
            .map_err(|e| CarbideError::Internal(format!("Parse error: {e}")))
    }

    /// Get a contract by ID.
    pub async fn get_contract(&self, id: &str) -> Result<ContractResponse> {
        let url = format!("{}/api/v1/contracts/{}", self.endpoint, id);
        debug!("Getting contract: GET {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Internal(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CarbideError::Internal(format!(
                "Get contract failed ({status}): {body}"
            )));
        }

        resp.json::<ContractResponse>()
            .await
            .map_err(|e| CarbideError::Internal(format!("Parse error: {e}")))
    }

    /// Record a deposit on a contract.
    pub async fn record_deposit(
        &self,
        contract_id: &str,
        amount: &str,
        tx_hash: Option<&str>,
    ) -> Result<ContractResponse> {
        let url = format!("{}/api/v1/contracts/{}/deposit", self.endpoint, contract_id);
        debug!("Recording deposit: POST {}", url);

        let mut body = serde_json::json!({ "amount": amount });
        if let Some(hash) = tx_hash {
            body["tx_hash"] = serde_json::Value::String(hash.to_string());
        }

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CarbideError::Internal(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CarbideError::Internal(format!(
                "Record deposit failed ({status}): {text}"
            )));
        }

        resp.json::<ContractResponse>()
            .await
            .map_err(|e| CarbideError::Internal(format!("Parse error: {e}")))
    }

    /// Get payment events for a contract.
    pub async fn get_payments(&self, contract_id: &str) -> Result<Vec<PaymentEvent>> {
        let url = format!(
            "{}/api/v1/contracts/{}/payments",
            self.endpoint, contract_id
        );
        debug!("Getting payments: GET {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Internal(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CarbideError::Internal(format!(
                "Get payments failed ({status}): {body}"
            )));
        }

        #[derive(Deserialize)]
        struct PaymentsWrapper {
            payments: Vec<PaymentEvent>,
        }

        let wrapper = resp
            .json::<PaymentsWrapper>()
            .await
            .map_err(|e| CarbideError::Internal(format!("Parse error: {e}")));

        // If the response is an array directly, try that too
        match wrapper {
            Ok(w) => Ok(w.payments),
            Err(_) => Ok(Vec::new()),
        }
    }

    /// List contracts (optionally filtered by client_id).
    pub async fn list_contracts(
        &self,
        client_id: Option<&str>,
    ) -> Result<Vec<ContractResponse>> {
        let mut url = format!("{}/api/v1/contracts", self.endpoint);
        if let Some(cid) = client_id {
            url = format!("{}?client_id={}", url, cid);
        }
        debug!("Listing contracts: GET {}", url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CarbideError::Internal(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CarbideError::Internal(format!(
                "List contracts failed ({status}): {body}"
            )));
        }

        #[derive(Deserialize)]
        struct ContractsWrapper {
            contracts: Vec<ContractResponse>,
        }

        let wrapper = resp
            .json::<ContractsWrapper>()
            .await
            .map_err(|e| CarbideError::Internal(format!("Parse error: {e}")));

        match wrapper {
            Ok(w) => Ok(w.contracts),
            Err(_) => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_contract_request_serialization() {
        let req = CreateContractRequest {
            provider_id: "test-provider".to_string(),
            client_id: "test-client".to_string(),
            price_per_gb_month: "0.005".to_string(),
            duration_days: 30,
            total_size_bytes: Some(1_000_000),
            file_id: Some("file-123".to_string()),
            chain_id: Some(1),
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: CreateContractRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider_id, "test-provider");
        assert_eq!(parsed.duration_days, 30);
        assert_eq!(parsed.total_size_bytes, Some(1_000_000));
    }

    #[test]
    fn contract_response_deserialization() {
        let json = r#"{
            "id": "contract-1",
            "provider_id": "prov-1",
            "client_id": "client-1",
            "status": "pending_deposit",
            "price_per_gb_month": "0.005",
            "duration_days": 30,
            "total_escrowed": null,
            "total_released": null,
            "created_at": "2025-01-01T00:00:00Z"
        }"#;

        let resp: ContractResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "contract-1");
        assert_eq!(resp.status, "pending_deposit");
    }

    #[test]
    fn payment_client_construction() {
        let client = PaymentClient::new("http://localhost:3000").unwrap();
        assert_eq!(client.endpoint, "http://localhost:3000");

        // Trailing slash should be trimmed
        let client2 = PaymentClient::new("http://localhost:3000/").unwrap();
        assert_eq!(client2.endpoint, "http://localhost:3000");
    }
}
