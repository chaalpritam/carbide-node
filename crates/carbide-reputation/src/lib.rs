//! # Carbide Reputation System
//!
//! Multi-dimensional reputation tracking for storage providers that evaluates:
//! - Uptime and availability
//! - Data integrity (proof-of-storage success rate)
//! - Response time performance
//! - Contract compliance
//! - Community feedback
//!
//! The reputation system uses a weighted scoring algorithm to provide fair
//! and accurate provider rankings for marketplace discovery.

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::unnecessary_wraps,
    clippy::unused_self,
    clippy::return_self_not_must_use,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::too_many_lines
)]

pub mod events;
pub mod scoring;
pub mod storage;
pub mod tracker;

use carbide_core::{ProviderId, ReputationScore, Result};
use chrono::{DateTime, Utc};
pub use events::{EventSeverity, EventType, ReputationEvent};
pub use scoring::{ReputationWeights, ScoringConfig};
use serde::{Deserialize, Serialize};
pub use storage::{FileStorage, MemoryStorage, ReputationStorage};
pub use tracker::{ReputationTracker, ReputationUpdate};

/// Configuration for the reputation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Scoring algorithm weights
    pub weights: ReputationWeights,
    /// Decay factor for older events (per day)
    pub time_decay_factor: rust_decimal::Decimal,
    /// Minimum number of events before reputation is considered stable
    pub min_events_for_stability: u32,
    /// Maximum age of events to consider (days)
    pub max_event_age_days: u32,
    /// Penalty factor for negative events
    pub penalty_multiplier: rust_decimal::Decimal,
    /// Bonus factor for positive events
    pub bonus_multiplier: rust_decimal::Decimal,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            weights: ReputationWeights::default(),
            time_decay_factor: rust_decimal::Decimal::new(995, 3), // 0.995 per day
            min_events_for_stability: 10,
            max_event_age_days: 30,
            penalty_multiplier: rust_decimal::Decimal::new(15, 1), // 1.5x penalty
            bonus_multiplier: rust_decimal::Decimal::new(11, 1),   // 1.1x bonus
        }
    }
}

/// Provider reputation statistics for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationStatistics {
    /// Provider ID
    pub provider_id: ProviderId,
    /// Current reputation score
    pub current_score: ReputationScore,
    /// Total number of tracked events
    pub total_events: u64,
    /// Recent events (last 7 days)
    pub recent_events: u64,
    /// Average uptime over tracked period
    pub average_uptime: rust_decimal::Decimal,
    /// Success rate for proof-of-storage challenges
    pub proof_success_rate: rust_decimal::Decimal,
    /// Average response time (milliseconds)
    pub average_response_time: f64,
    /// Number of contract violations
    pub contract_violations: u64,
    /// Reputation trend (improving/declining)
    pub trend: ReputationTrend,
    /// Days since first tracked event
    pub tracking_duration_days: u32,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

/// Reputation trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationTrend {
    /// Reputation improving over time
    Improving {
        /// Rate of improvement per day
        rate: rust_decimal::Decimal,
    },
    /// Reputation declining over time  
    Declining {
        /// Rate of decline per day
        rate: rust_decimal::Decimal,
    },
    /// Reputation stable
    Stable {
        /// Variance in scores
        variance: rust_decimal::Decimal,
    },
    /// Not enough data for trend analysis
    Insufficient,
}

/// Reputation system alerts for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationAlert {
    /// Alert ID
    pub id: uuid::Uuid,
    /// Provider ID
    pub provider_id: ProviderId,
    /// Alert type
    pub alert_type: AlertType,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Additional context data
    pub context: std::collections::HashMap<String, String>,
    /// When alert was triggered
    pub triggered_at: DateTime<Utc>,
    /// Whether alert is still active
    pub active: bool,
}

/// Types of reputation alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    /// Provider reputation dropped significantly
    ReputationDrop,
    /// High failure rate detected
    HighFailureRate,
    /// Unusual downtime pattern
    DowntimePattern,
    /// Proof-of-storage failures
    ProofFailures,
    /// Contract violations
    ContractViolation,
    /// Suspiciously high performance (potential gaming)
    SuspiciousActivity,
    /// Provider went offline unexpectedly
    UnexpectedOffline,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Low priority alert
    Low,
    /// Medium priority alert
    Medium,
    /// High priority alert requiring attention
    High,
    /// Critical alert requiring immediate action
    Critical,
}

/// Reputation system interface
pub trait ReputationSystem {
    /// Record a new reputation event for a provider
    fn record_event(&mut self, event: ReputationEvent) -> Result<()>;

    /// Get current reputation score for a provider
    fn get_reputation(&self, provider_id: &ProviderId) -> Result<Option<ReputationScore>>;

    /// Update reputation score based on recent events
    fn update_reputation(&mut self, provider_id: &ProviderId) -> Result<ReputationScore>;

    /// Get detailed statistics for a provider
    fn get_statistics(&self, provider_id: &ProviderId) -> Result<Option<ReputationStatistics>>;

    /// Get all active alerts
    fn get_active_alerts(&self) -> Result<Vec<ReputationAlert>>;

    /// Get providers ranked by reputation
    fn get_top_providers(&self, limit: usize) -> Result<Vec<(ProviderId, ReputationScore)>>;

    /// Cleanup old events and recalculate reputations
    fn maintenance(&mut self) -> Result<u64>;
}

/// Builder for configuring reputation system
pub struct ReputationSystemBuilder {
    config: ReputationConfig,
    storage: Option<Box<dyn ReputationStorage>>,
}

impl ReputationSystemBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ReputationConfig::default(),
            storage: None,
        }
    }

    /// Set custom scoring weights
    pub fn with_weights(mut self, weights: ReputationWeights) -> Self {
        self.config.weights = weights;
        self
    }

    /// Set time decay factor for older events
    pub fn with_time_decay(mut self, factor: rust_decimal::Decimal) -> Self {
        self.config.time_decay_factor = factor;
        self
    }

    /// Set storage backend
    pub fn with_storage(mut self, storage: Box<dyn ReputationStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set minimum events required for stable reputation
    pub fn with_min_events(mut self, count: u32) -> Self {
        self.config.min_events_for_stability = count;
        self
    }

    /// Build the reputation tracker
    pub fn build(self) -> Result<ReputationTracker> {
        let storage = self
            .storage
            .unwrap_or_else(|| Box::new(MemoryStorage::new()));
        ReputationTracker::new(self.config, storage)
    }
}

impl Default for ReputationSystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for reputation calculations
pub mod utils {
    use num_traits::ToPrimitive;
    use rust_decimal::Decimal;

    use super::{DateTime, ReputationTrend, Utc};

    /// Calculate time decay factor based on event age
    pub fn calculate_time_decay(
        event_time: DateTime<Utc>,
        current_time: DateTime<Utc>,
        decay_factor: Decimal,
    ) -> Decimal {
        let age_days = (current_time - event_time).num_days();
        if age_days <= 0 {
            return Decimal::ONE;
        }

        // Apply exponential decay (simplified approximation)
        let mut result = Decimal::ONE;
        for _ in 0..age_days {
            result *= decay_factor;
        }
        result
    }

    /// Normalize score to 0.0 - 1.0 range
    pub fn normalize_score(score: Decimal) -> Decimal {
        if score > Decimal::ONE {
            Decimal::ONE
        } else if score < Decimal::ZERO {
            Decimal::ZERO
        } else {
            score
        }
    }

    /// Calculate moving average for trend analysis
    pub fn calculate_moving_average(scores: &[Decimal], window_size: usize) -> Vec<Decimal> {
        let mut averages = Vec::new();

        for i in 0..scores.len() {
            let start = if i >= window_size {
                i - window_size + 1
            } else {
                0
            };
            let end = i + 1;
            let window = &scores[start..end];

            let sum: Decimal = window.iter().sum();
            let avg = sum / Decimal::new(window.len() as i64, 0);
            averages.push(avg);
        }

        averages
    }

    /// Detect reputation trend from historical scores
    pub fn analyze_trend(scores: &[Decimal]) -> ReputationTrend {
        if scores.len() < 5 {
            return ReputationTrend::Insufficient;
        }

        // Calculate linear regression slope
        let n = scores.len() as f64;
        let x_sum: f64 = (0..scores.len()).sum::<usize>() as f64;
        let y_sum = scores
            .iter()
            .map(|s| s.to_f64().unwrap_or(0.0))
            .sum::<f64>();
        let xy_sum = scores
            .iter()
            .enumerate()
            .map(|(i, s)| i as f64 * s.to_f64().unwrap_or(0.0))
            .sum::<f64>();
        let x2_sum: f64 = (0..scores.len()).map(|i| (i * i) as f64).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum * x_sum);

        let slope_decimal = Decimal::from_f64_retain(slope).unwrap_or(Decimal::ZERO);
        let threshold = Decimal::new(1, 3); // 0.001

        if slope_decimal > threshold {
            ReputationTrend::Improving {
                rate: slope_decimal,
            }
        } else if slope_decimal < -threshold {
            ReputationTrend::Declining {
                rate: slope_decimal.abs(),
            }
        } else {
            // Calculate variance for stability
            let mean = Decimal::from_f64_retain(y_sum / n).unwrap_or(Decimal::ZERO);
            let variance_sum = scores
                .iter()
                .map(|s| {
                    let diff = *s - mean;
                    diff * diff // Square manually since powi not available
                })
                .sum::<Decimal>();
            let variance = variance_sum / Decimal::new(scores.len() as i64, 0);

            ReputationTrend::Stable { variance }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::*;

    #[test]
    fn test_reputation_config_default() {
        let config = ReputationConfig::default();

        assert_eq!(config.min_events_for_stability, 10);
        assert_eq!(config.max_event_age_days, 30);
        assert!(config.time_decay_factor < rust_decimal::Decimal::ONE);
    }

    #[test]
    fn test_time_decay_calculation() {
        let now = Utc::now();
        let one_day_ago = now - chrono::Duration::days(1);
        let decay_factor = rust_decimal::Decimal::new(995, 3); // 0.995

        let decay = calculate_time_decay(one_day_ago, now, decay_factor);
        assert_eq!(decay, decay_factor);
    }

    #[test]
    fn test_score_normalization() {
        assert_eq!(
            normalize_score(rust_decimal::Decimal::new(15, 1)),
            rust_decimal::Decimal::ONE
        );
        assert_eq!(
            normalize_score(rust_decimal::Decimal::new(-5, 1)),
            rust_decimal::Decimal::ZERO
        );
        assert_eq!(
            normalize_score(rust_decimal::Decimal::new(5, 1)),
            rust_decimal::Decimal::new(5, 1)
        );
    }

    #[test]
    fn test_trend_analysis() {
        use rust_decimal::Decimal;

        // Improving trend
        let improving_scores = vec![
            Decimal::new(3, 1), // 0.3
            Decimal::new(4, 1), // 0.4
            Decimal::new(5, 1), // 0.5
            Decimal::new(6, 1), // 0.6
            Decimal::new(7, 1), // 0.7
        ];

        match analyze_trend(&improving_scores) {
            ReputationTrend::Improving { rate } => assert!(rate > Decimal::ZERO),
            _ => panic!("Expected improving trend"),
        }

        // Insufficient data
        let insufficient_scores = vec![Decimal::new(5, 1)];
        match analyze_trend(&insufficient_scores) {
            ReputationTrend::Insufficient => (),
            _ => panic!("Expected insufficient data"),
        }
    }

    #[test]
    fn test_builder_pattern() {
        let weights = ReputationWeights::default();
        let builder = ReputationSystemBuilder::new()
            .with_weights(weights)
            .with_min_events(20)
            .with_time_decay(rust_decimal::Decimal::new(99, 2));

        // Builder should be configured correctly
        assert_eq!(builder.config.min_events_for_stability, 20);
    }
}
