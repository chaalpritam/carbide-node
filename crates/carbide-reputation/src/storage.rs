//! Storage backends for reputation data persistence
//!
//! This module provides different storage implementations for reputation events,
//! scores, and alerts. Supports both in-memory and persistent storage.

use crate::{ReputationEvent, ReputationAlert};
use carbide_core::{ProviderId, ReputationScore, Result, CarbideError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::RwLock;

/// Trait for reputation data storage backends
#[async_trait::async_trait]
pub trait ReputationStorage: Send + Sync {
    /// Store a reputation event
    async fn store_event(&mut self, event: &ReputationEvent) -> Result<()>;

    /// Get all events for a provider since a specific date
    async fn get_events_since(
        &self, 
        provider_id: &ProviderId, 
        since: DateTime<Utc>
    ) -> Result<Vec<ReputationEvent>>;

    /// Get all events for a provider
    async fn get_all_events(&self, provider_id: &ProviderId) -> Result<Vec<ReputationEvent>>;

    /// Store reputation score for a provider
    async fn store_reputation(&mut self, provider_id: &ProviderId, score: &ReputationScore) -> Result<()>;

    /// Get current reputation score for a provider
    async fn get_reputation(&self, provider_id: &ProviderId) -> Result<ReputationScore>;

    /// Store a reputation alert
    async fn store_alert(&mut self, alert: &ReputationAlert) -> Result<()>;

    /// Get all active alerts
    async fn get_active_alerts(&self) -> Result<Vec<ReputationAlert>>;

    /// Get alerts for a specific provider
    async fn get_provider_alerts(&self, provider_id: &ProviderId) -> Result<Vec<ReputationAlert>>;

    /// Mark alert as resolved
    async fn resolve_alert(&mut self, alert_id: &uuid::Uuid) -> Result<()>;

    /// Get top providers by reputation score
    async fn get_top_providers(&self, limit: usize) -> Result<Vec<(ProviderId, ReputationScore)>>;

    /// Clean up old events before a cutoff date
    async fn cleanup_old_events(&mut self, cutoff: DateTime<Utc>) -> Result<u64>;

    /// Clean up old resolved alerts
    async fn cleanup_old_alerts(&mut self, cutoff: DateTime<Utc>) -> Result<u64>;

    /// Get storage statistics
    async fn get_statistics(&self) -> Result<StorageStatistics>;
}

/// Storage statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatistics {
    /// Total number of events stored
    pub total_events: u64,
    /// Total number of providers tracked
    pub total_providers: u64,
    /// Total number of active alerts
    pub active_alerts: u64,
    /// Storage size in bytes (if applicable)
    pub storage_size_bytes: Option<u64>,
    /// Last cleanup timestamp
    pub last_cleanup: Option<DateTime<Utc>>,
}

/// In-memory storage implementation for testing and development
pub struct MemoryStorage {
    /// Events by provider ID, sorted by timestamp
    events: RwLock<HashMap<ProviderId, Vec<ReputationEvent>>>,
    /// Current reputation scores
    scores: RwLock<HashMap<ProviderId, ReputationScore>>,
    /// Alerts by ID
    alerts: RwLock<HashMap<uuid::Uuid, ReputationAlert>>,
    /// Creation timestamp for statistics
    created_at: DateTime<Utc>,
}

impl MemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
            scores: RwLock::new(HashMap::new()),
            alerts: RwLock::new(HashMap::new()),
            created_at: Utc::now(),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ReputationStorage for MemoryStorage {
    async fn store_event(&mut self, event: &ReputationEvent) -> Result<()> {
        let mut events = self.events.write().await;
        let provider_events = events.entry(event.provider_id).or_default();
        
        // Insert event in chronological order
        let insert_pos = provider_events
            .binary_search_by(|e| e.timestamp.cmp(&event.timestamp))
            .unwrap_or_else(|e| e);
        
        provider_events.insert(insert_pos, event.clone());
        
        Ok(())
    }

    async fn get_events_since(
        &self, 
        provider_id: &ProviderId, 
        since: DateTime<Utc>
    ) -> Result<Vec<ReputationEvent>> {
        let events = self.events.read().await;
        
        match events.get(provider_id) {
            Some(provider_events) => {
                let filtered = provider_events
                    .iter()
                    .filter(|e| e.timestamp >= since)
                    .cloned()
                    .collect();
                Ok(filtered)
            }
            None => Err(CarbideError::NotFound(
                format!("No events found for provider {}", provider_id)
            )),
        }
    }

    async fn get_all_events(&self, provider_id: &ProviderId) -> Result<Vec<ReputationEvent>> {
        let events = self.events.read().await;
        
        match events.get(provider_id) {
            Some(provider_events) => Ok(provider_events.clone()),
            None => Err(CarbideError::NotFound(
                format!("No events found for provider {}", provider_id)
            )),
        }
    }

    async fn store_reputation(&mut self, provider_id: &ProviderId, score: &ReputationScore) -> Result<()> {
        let mut scores = self.scores.write().await;
        scores.insert(*provider_id, score.clone());
        Ok(())
    }

    async fn get_reputation(&self, provider_id: &ProviderId) -> Result<ReputationScore> {
        let scores = self.scores.read().await;
        
        match scores.get(provider_id) {
            Some(score) => Ok(score.clone()),
            None => Err(CarbideError::NotFound(
                format!("No reputation found for provider {}", provider_id)
            )),
        }
    }

    async fn store_alert(&mut self, alert: &ReputationAlert) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        alerts.insert(alert.id, alert.clone());
        Ok(())
    }

    async fn get_active_alerts(&self) -> Result<Vec<ReputationAlert>> {
        let alerts = self.alerts.read().await;
        
        let active = alerts
            .values()
            .filter(|alert| alert.active)
            .cloned()
            .collect();
        
        Ok(active)
    }

    async fn get_provider_alerts(&self, provider_id: &ProviderId) -> Result<Vec<ReputationAlert>> {
        let alerts = self.alerts.read().await;
        
        let provider_alerts = alerts
            .values()
            .filter(|alert| alert.provider_id == *provider_id)
            .cloned()
            .collect();
        
        Ok(provider_alerts)
    }

    async fn resolve_alert(&mut self, alert_id: &uuid::Uuid) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        
        match alerts.get_mut(alert_id) {
            Some(alert) => {
                alert.active = false;
                Ok(())
            }
            None => Err(CarbideError::NotFound(
                format!("Alert {} not found", alert_id)
            )),
        }
    }

    async fn get_top_providers(&self, limit: usize) -> Result<Vec<(ProviderId, ReputationScore)>> {
        let scores = self.scores.read().await;
        
        let mut provider_scores: Vec<_> = scores.iter()
            .map(|(id, score)| (*id, score.clone()))
            .collect();
        
        // Sort by overall reputation score (descending)
        provider_scores.sort_by(|a, b| b.1.overall.cmp(&a.1.overall));
        
        provider_scores.truncate(limit);
        
        Ok(provider_scores)
    }

    async fn cleanup_old_events(&mut self, cutoff: DateTime<Utc>) -> Result<u64> {
        let mut events = self.events.write().await;
        let mut cleaned_count = 0u64;
        
        for provider_events in events.values_mut() {
            let original_len = provider_events.len();
            provider_events.retain(|event| event.timestamp >= cutoff);
            cleaned_count += (original_len - provider_events.len()) as u64;
        }
        
        // Remove empty provider entries
        events.retain(|_, events| !events.is_empty());
        
        Ok(cleaned_count)
    }

    async fn cleanup_old_alerts(&mut self, cutoff: DateTime<Utc>) -> Result<u64> {
        let mut alerts = self.alerts.write().await;
        let original_len = alerts.len() as u64;
        
        alerts.retain(|_, alert| {
            alert.active || alert.triggered_at >= cutoff
        });
        
        Ok(original_len - alerts.len() as u64)
    }

    async fn get_statistics(&self) -> Result<StorageStatistics> {
        let events = self.events.read().await;
        let alerts = self.alerts.read().await;
        
        let total_events = events.values()
            .map(|v| v.len() as u64)
            .sum();
        
        let total_providers = events.len() as u64;
        
        let active_alerts = alerts.values()
            .filter(|alert| alert.active)
            .count() as u64;
        
        Ok(StorageStatistics {
            total_events,
            total_providers,
            active_alerts,
            storage_size_bytes: None, // Not applicable for memory storage
            last_cleanup: None,
        })
    }
}

/// File-based storage implementation for persistence
pub struct FileStorage {
    /// Base directory for data files
    data_dir: std::path::PathBuf,
    /// In-memory cache for performance
    cache: MemoryStorage,
    /// Last save timestamp
    last_save: RwLock<DateTime<Utc>>,
    /// Auto-save interval in seconds
    auto_save_interval: u64,
}

impl FileStorage {
    /// Create a new file-based storage
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| CarbideError::Internal(
                format!("Failed to create data directory: {}", e)
            ))?;
        
        let mut storage = Self {
            data_dir,
            cache: MemoryStorage::new(),
            last_save: RwLock::new(Utc::now()),
            auto_save_interval: 300, // 5 minutes
        };
        
        // Load existing data
        let _ = storage.load_from_disk().await; // Ignore errors on initial load
        
        Ok(storage)
    }

    /// Set auto-save interval
    pub fn with_auto_save_interval(mut self, interval_seconds: u64) -> Self {
        self.auto_save_interval = interval_seconds;
        self
    }

    /// Load data from disk
    async fn load_from_disk(&mut self) -> Result<()> {
        // Load events
        let events_file = self.data_dir.join("events.json");
        if events_file.exists() {
            let data = tokio::fs::read_to_string(&events_file).await
                .map_err(|e| CarbideError::Internal(format!("Failed to read events file: {}", e)))?;
            
            let events: HashMap<ProviderId, Vec<ReputationEvent>> = serde_json::from_str(&data)
                .map_err(|e| CarbideError::Internal(format!("Failed to parse events file: {}", e)))?;
            
            *self.cache.events.write().await = events;
        }

        // Load scores
        let scores_file = self.data_dir.join("scores.json");
        if scores_file.exists() {
            let data = tokio::fs::read_to_string(&scores_file).await
                .map_err(|e| CarbideError::Internal(format!("Failed to read scores file: {}", e)))?;
            
            let scores: HashMap<ProviderId, ReputationScore> = serde_json::from_str(&data)
                .map_err(|e| CarbideError::Internal(format!("Failed to parse scores file: {}", e)))?;
            
            *self.cache.scores.write().await = scores;
        }

        // Load alerts
        let alerts_file = self.data_dir.join("alerts.json");
        if alerts_file.exists() {
            let data = tokio::fs::read_to_string(&alerts_file).await
                .map_err(|e| CarbideError::Internal(format!("Failed to read alerts file: {}", e)))?;
            
            let alerts: HashMap<uuid::Uuid, ReputationAlert> = serde_json::from_str(&data)
                .map_err(|e| CarbideError::Internal(format!("Failed to parse alerts file: {}", e)))?;
            
            *self.cache.alerts.write().await = alerts;
        }

        Ok(())
    }

    /// Save data to disk
    async fn save_to_disk(&self) -> Result<()> {
        // Save events
        let events = self.cache.events.read().await;
        let events_data = serde_json::to_string_pretty(&*events)
            .map_err(|e| CarbideError::Internal(format!("Failed to serialize events: {}", e)))?;
        
        let events_file = self.data_dir.join("events.json");
        tokio::fs::write(&events_file, &events_data).await
            .map_err(|e| CarbideError::Internal(format!("Failed to write events file: {}", e)))?;

        // Save scores
        let scores = self.cache.scores.read().await;
        let scores_data = serde_json::to_string_pretty(&*scores)
            .map_err(|e| CarbideError::Internal(format!("Failed to serialize scores: {}", e)))?;
        
        let scores_file = self.data_dir.join("scores.json");
        tokio::fs::write(&scores_file, &scores_data).await
            .map_err(|e| CarbideError::Internal(format!("Failed to write scores file: {}", e)))?;

        // Save alerts
        let alerts = self.cache.alerts.read().await;
        let alerts_data = serde_json::to_string_pretty(&*alerts)
            .map_err(|e| CarbideError::Internal(format!("Failed to serialize alerts: {}", e)))?;
        
        let alerts_file = self.data_dir.join("alerts.json");
        tokio::fs::write(&alerts_file, &alerts_data).await
            .map_err(|e| CarbideError::Internal(format!("Failed to write alerts file: {}", e)))?;

        *self.last_save.write().await = Utc::now();
        
        Ok(())
    }

    /// Check if auto-save is needed
    async fn should_auto_save(&self) -> bool {
        let last_save = *self.last_save.read().await;
        let elapsed = (Utc::now() - last_save).num_seconds() as u64;
        elapsed >= self.auto_save_interval
    }
}

#[async_trait::async_trait]
impl ReputationStorage for FileStorage {
    async fn store_event(&mut self, event: &ReputationEvent) -> Result<()> {
        self.cache.store_event(event).await?;
        
        if self.should_auto_save().await {
            self.save_to_disk().await?;
        }
        
        Ok(())
    }

    async fn get_events_since(
        &self, 
        provider_id: &ProviderId, 
        since: DateTime<Utc>
    ) -> Result<Vec<ReputationEvent>> {
        self.cache.get_events_since(provider_id, since).await
    }

    async fn get_all_events(&self, provider_id: &ProviderId) -> Result<Vec<ReputationEvent>> {
        self.cache.get_all_events(provider_id).await
    }

    async fn store_reputation(&mut self, provider_id: &ProviderId, score: &ReputationScore) -> Result<()> {
        self.cache.store_reputation(provider_id, score).await?;
        
        if self.should_auto_save().await {
            self.save_to_disk().await?;
        }
        
        Ok(())
    }

    async fn get_reputation(&self, provider_id: &ProviderId) -> Result<ReputationScore> {
        self.cache.get_reputation(provider_id).await
    }

    async fn store_alert(&mut self, alert: &ReputationAlert) -> Result<()> {
        self.cache.store_alert(alert).await?;
        
        if self.should_auto_save().await {
            self.save_to_disk().await?;
        }
        
        Ok(())
    }

    async fn get_active_alerts(&self) -> Result<Vec<ReputationAlert>> {
        self.cache.get_active_alerts().await
    }

    async fn get_provider_alerts(&self, provider_id: &ProviderId) -> Result<Vec<ReputationAlert>> {
        self.cache.get_provider_alerts(provider_id).await
    }

    async fn resolve_alert(&mut self, alert_id: &uuid::Uuid) -> Result<()> {
        self.cache.resolve_alert(alert_id).await?;
        
        if self.should_auto_save().await {
            self.save_to_disk().await?;
        }
        
        Ok(())
    }

    async fn get_top_providers(&self, limit: usize) -> Result<Vec<(ProviderId, ReputationScore)>> {
        self.cache.get_top_providers(limit).await
    }

    async fn cleanup_old_events(&mut self, cutoff: DateTime<Utc>) -> Result<u64> {
        let cleaned = self.cache.cleanup_old_events(cutoff).await?;
        
        // Force save after cleanup
        self.save_to_disk().await?;
        
        Ok(cleaned)
    }

    async fn cleanup_old_alerts(&mut self, cutoff: DateTime<Utc>) -> Result<u64> {
        let cleaned = self.cache.cleanup_old_alerts(cutoff).await?;
        
        // Force save after cleanup
        self.save_to_disk().await?;
        
        Ok(cleaned)
    }

    async fn get_statistics(&self) -> Result<StorageStatistics> {
        let mut stats = self.cache.get_statistics().await?;
        
        // Add file size information
        let mut total_size = 0u64;
        for file_name in &["events.json", "scores.json", "alerts.json"] {
            let file_path = self.data_dir.join(file_name);
            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                total_size += metadata.len();
            }
        }
        
        stats.storage_size_bytes = Some(total_size);
        stats.last_cleanup = Some(*self.last_save.read().await);
        
        Ok(stats)
    }
}

impl Drop for FileStorage {
    fn drop(&mut self) {
        // Save data on drop (synchronous)
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let _ = runtime.block_on(self.save_to_disk());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventType, EventSeverity, ReputationEvent};

    #[tokio::test]
    async fn test_memory_storage_events() {
        let mut storage = MemoryStorage::new();
        let provider_id = uuid::Uuid::new_v4();
        
        let event = ReputationEvent::new(
            provider_id,
            EventType::Online,
            EventSeverity::Positive,
        );
        
        storage.store_event(&event).await.unwrap();
        
        let events = storage.get_all_events(&provider_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].provider_id, provider_id);
    }

    #[tokio::test]
    async fn test_memory_storage_reputation() {
        let mut storage = MemoryStorage::new();
        let provider_id = uuid::Uuid::new_v4();
        let score = ReputationScore::new();
        
        storage.store_reputation(&provider_id, &score).await.unwrap();
        
        let retrieved = storage.get_reputation(&provider_id).await.unwrap();
        assert_eq!(retrieved.overall, score.overall);
    }

    #[tokio::test]
    async fn test_memory_storage_alerts() {
        let mut storage = MemoryStorage::new();
        let provider_id = uuid::Uuid::new_v4();
        
        let alert = crate::ReputationAlert {
            id: uuid::Uuid::new_v4(),
            provider_id,
            alert_type: crate::AlertType::ReputationDrop,
            severity: crate::AlertSeverity::Medium,
            message: "Test alert".to_string(),
            context: std::collections::HashMap::new(),
            triggered_at: Utc::now(),
            active: true,
        };
        
        storage.store_alert(&alert).await.unwrap();
        
        let active_alerts = storage.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 1);
        
        storage.resolve_alert(&alert.id).await.unwrap();
        
        let active_alerts = storage.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_storage_cleanup() {
        let mut storage = MemoryStorage::new();
        let provider_id = uuid::Uuid::new_v4();
        
        // Create old event
        let mut old_event = ReputationEvent::new(
            provider_id,
            EventType::Online,
            EventSeverity::Positive,
        );
        old_event.timestamp = Utc::now() - chrono::Duration::days(40);
        
        // Create recent event
        let recent_event = ReputationEvent::new(
            provider_id,
            EventType::ProofSuccess { response_time_ms: 100, chunks_proven: 3 },
            EventSeverity::Positive,
        );
        
        storage.store_event(&old_event).await.unwrap();
        storage.store_event(&recent_event).await.unwrap();
        
        let cutoff = Utc::now() - chrono::Duration::days(30);
        let cleaned = storage.cleanup_old_events(cutoff).await.unwrap();
        
        assert_eq!(cleaned, 1);
        
        let remaining = storage.get_all_events(&provider_id).await.unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[tokio::test]
    async fn test_storage_statistics() {
        let mut storage = MemoryStorage::new();
        let provider_id = uuid::Uuid::new_v4();
        
        let event = ReputationEvent::new(
            provider_id,
            EventType::Online,
            EventSeverity::Positive,
        );
        
        storage.store_event(&event).await.unwrap();
        
        let stats = storage.get_statistics().await.unwrap();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.total_providers, 1);
    }
}