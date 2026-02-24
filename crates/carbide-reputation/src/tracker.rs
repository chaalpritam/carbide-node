//! Reputation tracker implementation
//!
//! This module provides the main ReputationTracker that processes events,
//! calculates scores, and maintains provider reputation data.

use std::collections::HashMap;

use carbide_core::{CarbideError, ProviderId, ReputationScore, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tracing::info;

use crate::{
    scoring::{ReputationCalculator, ScoringConfig},
    utils::analyze_trend,
    AlertSeverity, AlertType, ReputationAlert, ReputationConfig, ReputationEvent,
    ReputationStatistics, ReputationStorage, ReputationSystem,
};

/// Main reputation tracking system
pub struct ReputationTracker {
    /// System configuration
    config: ReputationConfig,
    /// Storage backend
    storage: Box<dyn ReputationStorage>,
    /// Reputation calculator
    calculator: ReputationCalculator,
    /// Active alerts cache
    active_alerts: Vec<ReputationAlert>,
    /// Last maintenance timestamp
    last_maintenance: DateTime<Utc>,
}

/// Reputation update result
#[derive(Debug, Clone)]
pub struct ReputationUpdate {
    /// Provider ID
    pub provider_id: ProviderId,
    /// Updated reputation score
    pub new_score: ReputationScore,
    /// Previous score for comparison
    pub previous_score: Option<ReputationScore>,
    /// Events that contributed to this update
    pub contributing_events: Vec<uuid::Uuid>,
    /// Any new alerts generated
    pub new_alerts: Vec<ReputationAlert>,
}

impl ReputationTracker {
    /// Create a new reputation tracker
    pub fn new(config: ReputationConfig, storage: Box<dyn ReputationStorage>) -> Result<Self> {
        let scoring_config = ScoringConfig {
            weights: config.weights.clone(),
            time_decay_factor: config.time_decay_factor,
            penalty_multiplier: config.penalty_multiplier,
            bonus_multiplier: config.bonus_multiplier,
        };

        let calculator = ReputationCalculator::new(scoring_config);

        Ok(Self {
            config,
            storage,
            calculator,
            active_alerts: Vec::new(),
            last_maintenance: Utc::now(),
        })
    }

    /// Process multiple events in batch for efficiency
    pub async fn process_events_batch(
        &mut self,
        events: Vec<ReputationEvent>,
    ) -> Result<Vec<ReputationUpdate>> {
        let mut updates = Vec::new();
        let mut provider_events: HashMap<ProviderId, Vec<ReputationEvent>> = HashMap::new();

        // Group events by provider
        for event in events {
            provider_events
                .entry(event.provider_id)
                .or_default()
                .push(event);
        }

        // Process events for each provider
        for (provider_id, provider_events) in provider_events {
            // Store all events
            for event in &provider_events {
                self.storage.store_event(event).await?;
            }

            // Update reputation (this should be done directly in async context)
            let previous_score = self.storage.get_reputation(&provider_id).await.ok();
            let new_score = self.calculate_reputation_score(&provider_id).await?;

            // Store updated score
            self.storage
                .store_reputation(&provider_id, &new_score)
                .await?;

            // Create update record
            let update = ReputationUpdate {
                provider_id,
                new_score: new_score.clone(),
                previous_score,
                contributing_events: provider_events.iter().map(|e| e.id).collect(),
                new_alerts: Vec::new(),
            };

            updates.push(update);
        }

        // Check for new alerts
        self.check_for_alerts(&updates).await?;

        Ok(updates)
    }

    /// Calculate reputation score from recent events
    async fn calculate_reputation_score(
        &self,
        provider_id: &ProviderId,
    ) -> Result<ReputationScore> {
        let cutoff_date =
            Utc::now() - chrono::Duration::days(i64::from(self.config.max_event_age_days));
        let events = self
            .storage
            .get_events_since(provider_id, cutoff_date)
            .await?;

        if events.is_empty() {
            return Ok(ReputationScore::new());
        }

        self.calculator.calculate_score(&events, Utc::now())
    }

    /// Generate reputation statistics for a provider
    async fn generate_statistics(&self, provider_id: &ProviderId) -> Result<ReputationStatistics> {
        let cutoff_date =
            Utc::now() - chrono::Duration::days(i64::from(self.config.max_event_age_days));
        let events = self
            .storage
            .get_events_since(provider_id, cutoff_date)
            .await?;
        let all_events = self.storage.get_all_events(provider_id).await?;

        let current_score = self.calculator.calculate_score(&events, Utc::now())?;

        let recent_cutoff = Utc::now() - chrono::Duration::days(7);
        let recent_events = events
            .iter()
            .filter(|e| e.timestamp > recent_cutoff)
            .count() as u64;

        // Calculate uptime from online/offline events
        let uptime_events: Vec<_> = events.iter().filter(|e| e.affects_uptime()).collect();

        let average_uptime = if uptime_events.is_empty() {
            Decimal::new(100, 2) // 100% if no uptime data
        } else {
            let online_time: i64 = uptime_events
                .iter()
                .filter(|e| matches!(e.event_type, crate::events::EventType::Online))
                .count() as i64;
            let total_events = uptime_events.len() as i64;
            Decimal::new(online_time * 100, 2) / Decimal::new(total_events, 0)
        };

        // Calculate proof success rate
        let proof_events: Vec<_> = events
            .iter()
            .filter(|e| e.affects_data_integrity())
            .collect();

        let proof_success_rate = if proof_events.is_empty() {
            Decimal::new(100, 2) // 100% if no proof data
        } else {
            let successful_proofs = proof_events
                .iter()
                .filter(|e| matches!(e.event_type, crate::events::EventType::ProofSuccess { .. }))
                .count() as i64;
            let total_proofs = proof_events.len() as i64;
            Decimal::new(successful_proofs * 100, 2) / Decimal::new(total_proofs, 0)
        };

        // Calculate average response time
        let response_times: Vec<u64> = events
            .iter()
            .filter_map(super::events::ReputationEvent::response_time_ms)
            .collect();

        let average_response_time = if response_times.is_empty() {
            0.0
        } else {
            response_times.iter().sum::<u64>() as f64 / response_times.len() as f64
        };

        // Count contract violations
        let contract_violations = events
            .iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    crate::events::EventType::ContractViolated { .. }
                )
            })
            .count() as u64;

        // Analyze trend
        let historical_scores: Vec<Decimal> = all_events
            .iter()
            .rev()
            .take(30) // Last 30 data points
            .map(|_| current_score.overall) // Simplified - would need historical scores
            .collect();

        let trend = analyze_trend(&historical_scores);

        // Calculate tracking duration
        let first_event = all_events.first().map_or_else(Utc::now, |e| e.timestamp);
        let tracking_duration_days = (Utc::now() - first_event).num_days() as u32;

        let last_activity = events
            .iter()
            .map(|e| e.timestamp)
            .max()
            .unwrap_or_else(Utc::now);

        Ok(ReputationStatistics {
            provider_id: *provider_id,
            current_score,
            total_events: all_events.len() as u64,
            recent_events,
            average_uptime,
            proof_success_rate,
            average_response_time,
            contract_violations,
            trend,
            tracking_duration_days,
            last_activity,
        })
    }

    /// Check for reputation alerts based on recent updates
    async fn check_for_alerts(&mut self, updates: &[ReputationUpdate]) -> Result<()> {
        for update in updates {
            let stats = self.generate_statistics(&update.provider_id).await?;

            let mut new_alerts = Vec::new();

            // Check for reputation drops
            if let Some(prev_score) = &update.previous_score {
                let score_drop = prev_score.overall - update.new_score.overall;
                if score_drop > Decimal::new(2, 1) {
                    // 0.2 drop
                    new_alerts.push(ReputationAlert {
                        id: uuid::Uuid::new_v4(),
                        provider_id: update.provider_id,
                        alert_type: AlertType::ReputationDrop,
                        severity: if score_drop > Decimal::new(4, 1) {
                            AlertSeverity::Critical
                        } else {
                            AlertSeverity::High
                        },
                        message: format!(
                            "Reputation dropped by {:.1}%",
                            score_drop * Decimal::new(100, 0)
                        ),
                        context: std::collections::HashMap::new(),
                        triggered_at: Utc::now(),
                        active: true,
                    });
                }
            }

            // Check for high failure rates
            if stats.proof_success_rate < Decimal::new(80, 2) {
                // Below 80%
                new_alerts.push(ReputationAlert {
                    id: uuid::Uuid::new_v4(),
                    provider_id: update.provider_id,
                    alert_type: AlertType::HighFailureRate,
                    severity: AlertSeverity::Medium,
                    message: format!("Proof success rate: {:.1}%", stats.proof_success_rate),
                    context: std::collections::HashMap::new(),
                    triggered_at: Utc::now(),
                    active: true,
                });
            }

            // Check for low uptime
            if stats.average_uptime < Decimal::new(95, 2) {
                // Below 95%
                new_alerts.push(ReputationAlert {
                    id: uuid::Uuid::new_v4(),
                    provider_id: update.provider_id,
                    alert_type: AlertType::DowntimePattern,
                    severity: if stats.average_uptime < Decimal::new(90, 2) {
                        AlertSeverity::High
                    } else {
                        AlertSeverity::Medium
                    },
                    message: format!("Average uptime: {:.1}%", stats.average_uptime),
                    context: std::collections::HashMap::new(),
                    triggered_at: Utc::now(),
                    active: true,
                });
            }

            // Store new alerts
            for alert in &new_alerts {
                self.storage.store_alert(alert).await?;
            }

            self.active_alerts.extend(new_alerts);
        }

        Ok(())
    }

    /// Perform maintenance tasks (cleanup old data, recalculate scores)
    pub async fn perform_maintenance(&mut self) -> Result<u64> {
        info!("Starting reputation system maintenance");

        let cutoff_date =
            Utc::now() - chrono::Duration::days(i64::from(self.config.max_event_age_days) * 2);

        // Clean up old events
        let cleaned_events = self.storage.cleanup_old_events(cutoff_date).await?;

        // Clean up resolved alerts older than 7 days
        let alert_cutoff = Utc::now() - chrono::Duration::days(7);
        let cleaned_alerts = self.storage.cleanup_old_alerts(alert_cutoff).await?;

        // Refresh active alerts cache
        self.active_alerts = self.storage.get_active_alerts().await?;

        // Update maintenance timestamp
        self.last_maintenance = Utc::now();

        info!(
            "Maintenance completed. Cleaned {} events and {} alerts",
            cleaned_events, cleaned_alerts
        );

        Ok(cleaned_events + cleaned_alerts)
    }

    /// Get reputation configuration
    pub fn config(&self) -> &ReputationConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ReputationConfig) {
        self.config = config;

        // Update calculator configuration
        let scoring_config = ScoringConfig {
            weights: self.config.weights.clone(),
            time_decay_factor: self.config.time_decay_factor,
            penalty_multiplier: self.config.penalty_multiplier,
            bonus_multiplier: self.config.bonus_multiplier,
        };

        self.calculator = ReputationCalculator::new(scoring_config);
    }
}

impl ReputationSystem for ReputationTracker {
    fn record_event(&mut self, event: ReputationEvent) -> Result<()> {
        // For sync interface, we'll use tokio runtime
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CarbideError::Internal("No async runtime available".to_string()))?;

        runtime.block_on(async { self.storage.store_event(&event).await })
    }

    fn get_reputation(&self, provider_id: &ProviderId) -> Result<Option<ReputationScore>> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CarbideError::Internal("No async runtime available".to_string()))?;

        runtime.block_on(async {
            match self.storage.get_reputation(provider_id).await {
                Ok(score) => Ok(Some(score)),
                Err(CarbideError::NotFound(_)) => Ok(None),
                Err(e) => Err(e),
            }
        })
    }

    fn update_reputation(&mut self, provider_id: &ProviderId) -> Result<ReputationScore> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CarbideError::Internal("No async runtime available".to_string()))?;

        runtime.block_on(async {
            let previous_score = self.storage.get_reputation(provider_id).await.ok();
            let new_score = self.calculate_reputation_score(provider_id).await?;

            // Store updated score
            self.storage
                .store_reputation(provider_id, &new_score)
                .await?;

            // Create update record
            let update = ReputationUpdate {
                provider_id: *provider_id,
                new_score: new_score.clone(),
                previous_score,
                contributing_events: Vec::new(), // Would need to track this
                new_alerts: Vec::new(),
            };

            // Check for alerts
            self.check_for_alerts(&[update]).await?;

            Ok(new_score)
        })
    }

    fn get_statistics(&self, provider_id: &ProviderId) -> Result<Option<ReputationStatistics>> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CarbideError::Internal("No async runtime available".to_string()))?;

        runtime.block_on(async {
            match self.generate_statistics(provider_id).await {
                Ok(stats) => Ok(Some(stats)),
                Err(CarbideError::NotFound(_)) => Ok(None),
                Err(e) => Err(e),
            }
        })
    }

    fn get_active_alerts(&self) -> Result<Vec<ReputationAlert>> {
        Ok(self.active_alerts.clone())
    }

    fn get_top_providers(&self, limit: usize) -> Result<Vec<(ProviderId, ReputationScore)>> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CarbideError::Internal("No async runtime available".to_string()))?;

        runtime.block_on(async { self.storage.get_top_providers(limit).await })
    }

    fn maintenance(&mut self) -> Result<u64> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| CarbideError::Internal("No async runtime available".to_string()))?;

        runtime.block_on(async { self.perform_maintenance().await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{events::EventBuilder, EventSeverity, EventType, MemoryStorage};

    #[tokio::test]
    async fn test_reputation_tracker_creation() {
        let config = ReputationConfig::default();
        let storage = Box::new(MemoryStorage::new());

        let tracker = ReputationTracker::new(config, storage);
        assert!(tracker.is_ok());
    }

    #[tokio::test]
    async fn test_event_processing() {
        let config = ReputationConfig::default();
        let storage = Box::new(MemoryStorage::new());
        let mut tracker = ReputationTracker::new(config, storage).unwrap();

        let provider_id = uuid::Uuid::new_v4();

        let event = EventBuilder::new(provider_id, EventType::Online)
            .severity(EventSeverity::Positive)
            .build();

        let events = vec![event];
        let updates = tracker.process_events_batch(events).await.unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].provider_id, provider_id);
    }

    #[tokio::test]
    async fn test_reputation_calculation() {
        let config = ReputationConfig::default();
        let mut storage_impl = MemoryStorage::new();
        let provider_id = uuid::Uuid::new_v4();

        // First store an event so the provider exists
        let event = EventBuilder::new(provider_id, EventType::Online)
            .severity(EventSeverity::Positive)
            .build();

        storage_impl.store_event(&event).await.unwrap();

        let storage = Box::new(storage_impl);
        let tracker = ReputationTracker::new(config, storage).unwrap();

        let score = tracker
            .calculate_reputation_score(&provider_id)
            .await
            .unwrap();

        // Should have calculated reputation based on the event
        assert!(score.overall > rust_decimal::Decimal::ZERO);
    }
}
