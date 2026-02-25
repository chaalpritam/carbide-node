//! Periodic proof-of-storage scheduler
//!
//! Runs a background task that iterates active storage contracts, reads
//! the associated file from disk, generates a proof, and submits it to
//! the discovery service.  Proof submission frequency is configurable
//! (default every 6 hours).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use carbide_core::{
    network::{ChunkProofData, StorageProofData},
    ContractStatus, ContentHash, FileId, StorageContract,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::server::StoredFile;

/// Configuration for the proof scheduler.
#[derive(Debug, Clone)]
pub struct ProofSchedulerConfig {
    /// Interval between proof runs (seconds).
    pub interval_secs: u64,
    /// Fraction of file to sample per proof (0.0–1.0).
    pub challenge_percentage: f64,
    /// Discovery service base URL (e.g. `https://discovery.carbidenetwork.xyz`).
    pub discovery_endpoint: String,
    /// Provider identifier string for submission.
    pub provider_id: String,
}

impl Default for ProofSchedulerConfig {
    fn default() -> Self {
        Self {
            interval_secs: 21600, // 6 hours
            challenge_percentage: 0.1,
            discovery_endpoint: String::new(),
            provider_id: String::new(),
        }
    }
}

/// Background proof-of-storage scheduler.
///
/// Call [`ProofScheduler::spawn`] to launch the task on the Tokio runtime.
pub struct ProofScheduler {
    config: ProofSchedulerConfig,
    contracts: Arc<RwLock<HashMap<uuid::Uuid, StorageContract>>>,
    files: Arc<RwLock<HashMap<FileId, StoredFile>>>,
    storage_dir: PathBuf,
}

impl ProofScheduler {
    /// Create a new scheduler (does not start it yet).
    pub fn new(
        config: ProofSchedulerConfig,
        contracts: Arc<RwLock<HashMap<uuid::Uuid, StorageContract>>>,
        files: Arc<RwLock<HashMap<FileId, StoredFile>>>,
        storage_dir: PathBuf,
    ) -> Self {
        Self {
            config,
            contracts,
            files,
            storage_dir,
        }
    }

    /// Spawn the scheduler as a background Tokio task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    /// Main loop — runs forever, sleeping between rounds.
    async fn run(&self) {
        let interval = Duration::from_secs(self.config.interval_secs);
        let client = reqwest::Client::new();

        info!(
            "Proof scheduler started (interval={}s, sample={:.0}%)",
            self.config.interval_secs,
            self.config.challenge_percentage * 100.0
        );

        loop {
            tokio::time::sleep(interval).await;
            self.run_proof_round(&client).await;
        }
    }

    /// Execute one round: iterate active contracts and submit proofs.
    async fn run_proof_round(&self, client: &reqwest::Client) {
        let contracts = self.contracts.read().await;

        let active: Vec<_> = contracts
            .values()
            .filter(|c| matches!(c.status, ContractStatus::Active))
            .cloned()
            .collect();
        drop(contracts);

        if active.is_empty() {
            info!("Proof round: no active contracts, skipping");
            return;
        }

        info!("Proof round: processing {} active contracts", active.len());

        let mut success = 0u32;
        let mut failed = 0u32;

        for contract in &active {
            match self.generate_and_submit_proof(client, contract).await {
                Ok(()) => {
                    success += 1;
                    info!("Proof submitted for contract {}", contract.id);
                }
                Err(e) => {
                    failed += 1;
                    warn!("Proof failed for contract {}: {}", contract.id, e);
                }
            }
        }

        info!(
            "Proof round complete: {} succeeded, {} failed",
            success, failed
        );
    }

    /// Generate a proof for a single contract and POST it to the discovery service.
    async fn generate_and_submit_proof(
        &self,
        client: &reqwest::Client,
        contract: &StorageContract,
    ) -> Result<(), String> {
        // Look up the stored file
        let files = self.files.read().await;
        let stored_file = files
            .get(&contract.file_id)
            .ok_or_else(|| format!("File {} not found on disk", contract.file_id))?;
        let file_path = self
            .storage_dir
            .join(format!("{}.dat", stored_file.file_id.to_hex()));
        drop(files);

        // Read the file from disk
        let data = tokio::fs::read(&file_path)
            .await
            .map_err(|e| format!("Failed to read {}: {e}", file_path.display()))?;

        // Determine which chunks to sample
        let chunk_size: usize = 256 * 1024; // 256 KB chunks
        let total_chunks = (data.len() + chunk_size - 1) / chunk_size;
        let sample_count =
            ((total_chunks as f64 * self.config.challenge_percentage).ceil() as usize).max(1);

        let mut chunk_indices: Vec<usize> = (0..total_chunks).collect();
        // Deterministic but varied selection: pick evenly spaced indices
        let step = if sample_count >= total_chunks {
            1
        } else {
            total_chunks / sample_count
        };
        chunk_indices = chunk_indices.into_iter().step_by(step).take(sample_count).collect();

        // Build Merkle proofs for each sampled chunk
        let mut merkle_proofs = Vec::with_capacity(chunk_indices.len());
        for &idx in &chunk_indices {
            let start = idx * chunk_size;
            let end = (start + chunk_size).min(data.len());
            let chunk_data = &data[start..end];

            merkle_proofs.push(ChunkProofData {
                chunk_index: idx,
                chunk_hash: ContentHash::from_data(chunk_data),
                merkle_path: vec![
                    ContentHash::from_data(&data), // file-level hash as root
                ],
                chunk_data: None,
            });
        }

        // Build the challenge id and response hash
        let challenge_id = format!(
            "scheduled-{}-{}",
            contract.id,
            chrono::Utc::now().timestamp()
        );

        let response_data = format!(
            "{}:{}:{:?}",
            challenge_id,
            contract.file_id.to_hex(),
            chunk_indices
        );
        let response_hash = ContentHash::from_data(response_data.as_bytes());

        let proof = StorageProofData {
            challenge_id,
            merkle_proofs,
            response_hash,
            signature: vec![0u8; 64], // Scheduler proofs are not Ed25519-signed in v1
            generated_at: chrono::Utc::now(),
        };

        // Submit to discovery service
        if self.config.discovery_endpoint.is_empty() {
            return Err("No discovery endpoint configured".to_string());
        }

        let url = format!(
            "{}/api/v1/contracts/{}/proofs",
            self.config.discovery_endpoint, contract.id
        );

        let resp = client
            .post(&url)
            .json(&serde_json::json!({
                "provider_id": self.config.provider_id,
                "challenge_id": proof.challenge_id,
                "response_hash": hex::encode(proof.response_hash.as_bytes()),
                "chunk_proofs": proof.merkle_proofs.len(),
            }))
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Discovery returned {status}: {body}"));
        }

        Ok(())
    }
}
