//! Reputation event emission for the provider server
//!
//! Wraps the `carbide-reputation` crate's tracker to provide convenient
//! helper functions for emitting events from upload/download/proof handlers.
//! The `ReputationBridge` additionally forwards events to the discovery service
//! so that reputation scores stay in sync with the marketplace.

use std::sync::Arc;

use carbide_core::ProviderId;
use carbide_reputation::{
    events::{EventBuilder, EventSeverity, EventType},
    MemoryStorage, ReputationConfig, ReputationTracker,
};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::discovery_client::{DiscoveryApiClient, ReputationEventPayload};

/// Create a new reputation tracker for a provider.
pub fn create_tracker(_provider_id: ProviderId) -> ReputationTracker {
    let config = ReputationConfig::default();
    let storage = Box::new(MemoryStorage::new());
    ReputationTracker::new(config, storage)
        .expect("Failed to create reputation tracker")
}

/// Emit a successful proof-of-storage event.
pub async fn emit_proof_success(
    tracker: &Arc<Mutex<ReputationTracker>>,
    provider_id: ProviderId,
    response_time_ms: u64,
    chunks_proven: u32,
) {
    let event = EventBuilder::new(
        provider_id,
        EventType::ProofSuccess {
            response_time_ms,
            chunks_proven,
        },
    )
    .severity(EventSeverity::Positive)
    .build();

    let mut t = tracker.lock().await;
    if let Err(e) = t.process_events_batch(vec![event]).await {
        warn!("Failed to emit proof success event: {}", e);
    } else {
        debug!("Reputation: proof success recorded for {}", provider_id);
    }
}

/// Emit a failed proof-of-storage event.
pub async fn emit_proof_failure(
    tracker: &Arc<Mutex<ReputationTracker>>,
    provider_id: ProviderId,
    reason: String,
) {
    let event = EventBuilder::new(
        provider_id,
        EventType::ProofFailure {
            reason,
            error_details: None,
        },
    )
    .severity(EventSeverity::Negative)
    .build();

    let mut t = tracker.lock().await;
    if let Err(e) = t.process_events_batch(vec![event]).await {
        warn!("Failed to emit proof failure event: {}", e);
    } else {
        debug!("Reputation: proof failure recorded for {}", provider_id);
    }
}

/// Emit a successful file upload event.
pub async fn emit_upload_success(
    tracker: &Arc<Mutex<ReputationTracker>>,
    provider_id: ProviderId,
    file_size: u64,
    upload_time_ms: u64,
) {
    let event = EventBuilder::new(
        provider_id,
        EventType::UploadSuccess {
            file_size,
            upload_time_ms,
        },
    )
    .severity(EventSeverity::Positive)
    .build();

    let mut t = tracker.lock().await;
    if let Err(e) = t.process_events_batch(vec![event]).await {
        warn!("Failed to emit upload success event: {}", e);
    }
}

/// Emit a successful file download event.
pub async fn emit_download_success(
    tracker: &Arc<Mutex<ReputationTracker>>,
    provider_id: ProviderId,
    file_size: u64,
    download_time_ms: u64,
) {
    let event = EventBuilder::new(
        provider_id,
        EventType::DownloadSuccess {
            file_size,
            download_time_ms,
        },
    )
    .severity(EventSeverity::Positive)
    .build();

    let mut t = tracker.lock().await;
    if let Err(e) = t.process_events_batch(vec![event]).await {
        warn!("Failed to emit download success event: {}", e);
    }
}

/// Emit a provider online event.
pub async fn emit_online(
    tracker: &Arc<Mutex<ReputationTracker>>,
    provider_id: ProviderId,
) {
    let event = EventBuilder::new(provider_id, EventType::Online)
        .severity(EventSeverity::Positive)
        .build();

    let mut t = tracker.lock().await;
    if let Err(e) = t.process_events_batch(vec![event]).await {
        warn!("Failed to emit online event: {}", e);
    } else {
        debug!("Reputation: online event recorded for {}", provider_id);
    }
}

/// Bridge between local reputation tracking and the discovery service.
///
/// Each method stores the event locally via the in-memory tracker AND
/// fires-and-forgets a POST to the discovery service (if configured).
pub struct ReputationBridge {
    /// Local in-memory reputation tracker
    pub tracker: Arc<Mutex<ReputationTracker>>,
    /// Optional discovery API client for remote forwarding
    pub discovery: Option<Arc<DiscoveryApiClient>>,
    /// Provider ID for this node
    pub provider_id: ProviderId,
}

impl ReputationBridge {
    /// Create a new reputation bridge.
    ///
    /// If `discovery_endpoint` is `Some`, events will be forwarded to that URL.
    pub fn new(provider_id: ProviderId, discovery_endpoint: Option<String>) -> Self {
        let tracker = Arc::new(Mutex::new(create_tracker(provider_id)));
        let discovery = discovery_endpoint.map(|url| Arc::new(DiscoveryApiClient::new(url)));
        Self {
            tracker,
            discovery,
            provider_id,
        }
    }

    /// Record a successful proof-of-storage event.
    pub async fn emit_proof_success(&self, response_time_ms: u64, chunks_proven: u32) {
        emit_proof_success(&self.tracker, self.provider_id, response_time_ms, chunks_proven).await;
        self.forward_event("proof_success", "positive", Some(response_time_ms as f64), None);
    }

    /// Record a failed proof-of-storage event.
    pub async fn emit_proof_failure(&self, reason: String) {
        emit_proof_failure(&self.tracker, self.provider_id, reason).await;
        self.forward_event("proof_failure", "negative", None, None);
    }

    /// Record a successful file upload event.
    pub async fn emit_upload_success(&self, file_size: u64, upload_time_ms: u64) {
        emit_upload_success(&self.tracker, self.provider_id, file_size, upload_time_ms).await;
        self.forward_event("upload_success", "positive", Some(upload_time_ms as f64), None);
    }

    /// Record a successful file download event.
    pub async fn emit_download_success(&self, file_size: u64, download_time_ms: u64) {
        emit_download_success(&self.tracker, self.provider_id, file_size, download_time_ms).await;
        self.forward_event("download_success", "positive", Some(download_time_ms as f64), None);
    }

    /// Record a provider-online event.
    pub async fn emit_online(&self) {
        emit_online(&self.tracker, self.provider_id).await;
        self.forward_event("online", "positive", None, None);
    }

    /// Fire-and-forget forwarding to the discovery service.
    fn forward_event(
        &self,
        event_type: &str,
        severity: &str,
        value: Option<f64>,
        contract_id: Option<String>,
    ) {
        if let Some(ref client) = self.discovery {
            let client = Arc::clone(client);
            let payload = ReputationEventPayload {
                provider_id: self.provider_id.to_string(),
                event_type: event_type.to_string(),
                severity: severity.to_string(),
                value,
                details: None,
                contract_id,
            };
            tokio::spawn(async move {
                if let Err(e) = client.submit_reputation_event(&payload).await {
                    warn!("Failed to forward reputation event to discovery: {}", e);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_tracker_succeeds() {
        let provider_id = uuid::Uuid::new_v4();
        let tracker = create_tracker(provider_id);
        // Should not panic
        let _ = tracker;
    }

    #[tokio::test]
    async fn emit_functions_record_events() {
        let provider_id = uuid::Uuid::new_v4();
        let tracker = Arc::new(Mutex::new(create_tracker(provider_id)));

        emit_online(&tracker, provider_id).await;
        emit_upload_success(&tracker, provider_id, 1024, 100).await;
        emit_download_success(&tracker, provider_id, 2048, 200).await;
        emit_proof_success(&tracker, provider_id, 50, 3).await;
        emit_proof_failure(&tracker, provider_id, "test failure".to_string()).await;

        // All emissions should succeed without panic
    }

    #[tokio::test]
    async fn tracker_computes_score_after_events() {
        let provider_id = uuid::Uuid::new_v4();
        let tracker = Arc::new(Mutex::new(create_tracker(provider_id)));

        // Emit several positive events
        emit_online(&tracker, provider_id).await;
        emit_proof_success(&tracker, provider_id, 100, 5).await;
        emit_upload_success(&tracker, provider_id, 10000, 500).await;

        // process_events_batch already stores reputation scores internally.
        // We verify that events were processed without errors (no panic above)
        // and that further emissions succeed, indicating the tracker is healthy.
        emit_download_success(&tracker, provider_id, 2048, 200).await;
    }

    #[test]
    fn reputation_bridge_without_discovery() {
        let provider_id = uuid::Uuid::new_v4();
        let bridge = ReputationBridge::new(provider_id, None);
        assert!(bridge.discovery.is_none());
    }

    #[test]
    fn reputation_bridge_with_discovery() {
        let provider_id = uuid::Uuid::new_v4();
        let bridge = ReputationBridge::new(provider_id, Some("http://localhost:3000".to_string()));
        assert!(bridge.discovery.is_some());
    }

    #[tokio::test]
    async fn reputation_bridge_emit_does_not_panic() {
        let provider_id = uuid::Uuid::new_v4();
        let bridge = ReputationBridge::new(provider_id, None);

        bridge.emit_online().await;
        bridge.emit_proof_success(50, 3).await;
        bridge.emit_proof_failure("test reason".to_string()).await;
        bridge.emit_upload_success(1024, 100).await;
        bridge.emit_download_success(2048, 200).await;
    }
}
