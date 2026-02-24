//! Reputation scoring algorithms and configuration
//!
//! This module implements the core reputation calculation logic,
//! including weighted scoring, time decay, and trend analysis.

use carbide_core::{CarbideError, ReputationScore, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{utils::calculate_time_decay, ReputationEvent};

/// Weights for different reputation components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationWeights {
    /// Uptime and availability weight (0.0 - 1.0)
    pub uptime: Decimal,
    /// Data integrity (proof success) weight
    pub data_integrity: Decimal,
    /// Response time performance weight
    pub response_time: Decimal,
    /// Contract compliance weight
    pub contract_compliance: Decimal,
    /// Community feedback weight
    pub community_feedback: Decimal,
}

impl Default for ReputationWeights {
    fn default() -> Self {
        Self {
            uptime: Decimal::new(25, 2),              // 0.25 (25%)
            data_integrity: Decimal::new(25, 2),      // 0.25 (25%)
            response_time: Decimal::new(20, 2),       // 0.20 (20%)
            contract_compliance: Decimal::new(20, 2), // 0.20 (20%)
            community_feedback: Decimal::new(10, 2),  // 0.10 (10%)
        }
    }
}

impl ReputationWeights {
    /// Validate that weights sum to 1.0
    pub fn validate(&self) -> Result<()> {
        let total = self.uptime
            + self.data_integrity
            + self.response_time
            + self.contract_compliance
            + self.community_feedback;

        if (total - Decimal::ONE).abs() > Decimal::new(1, 3) {
            // Allow 0.001 tolerance
            return Err(CarbideError::Internal(format!(
                "Reputation weights must sum to 1.0, got {total}"
            )));
        }

        Ok(())
    }

    /// Create balanced weights (equal distribution)
    pub fn balanced() -> Self {
        Self {
            uptime: Decimal::new(20, 2),              // 0.20 (20%)
            data_integrity: Decimal::new(20, 2),      // 0.20 (20%)
            response_time: Decimal::new(20, 2),       // 0.20 (20%)
            contract_compliance: Decimal::new(20, 2), // 0.20 (20%)
            community_feedback: Decimal::new(20, 2),  // 0.20 (20%)
        }
    }

    /// Create weights focused on reliability
    pub fn reliability_focused() -> Self {
        Self {
            uptime: Decimal::new(35, 2),              // 0.35 (35%)
            data_integrity: Decimal::new(35, 2),      // 0.35 (35%)
            response_time: Decimal::new(15, 2),       // 0.15 (15%)
            contract_compliance: Decimal::new(10, 2), // 0.10 (10%)
            community_feedback: Decimal::new(5, 2),   // 0.05 (5%)
        }
    }

    /// Create weights focused on performance
    pub fn performance_focused() -> Self {
        Self {
            uptime: Decimal::new(20, 2),              // 0.20 (20%)
            data_integrity: Decimal::new(20, 2),      // 0.20 (20%)
            response_time: Decimal::new(40, 2),       // 0.40 (40%)
            contract_compliance: Decimal::new(15, 2), // 0.15 (15%)
            community_feedback: Decimal::new(5, 2),   // 0.05 (5%)
        }
    }
}

/// Configuration for reputation scoring algorithm
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Component weights
    pub weights: ReputationWeights,
    /// Time decay factor for older events
    pub time_decay_factor: Decimal,
    /// Penalty multiplier for negative events
    pub penalty_multiplier: Decimal,
    /// Bonus multiplier for positive events
    pub bonus_multiplier: Decimal,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            weights: ReputationWeights::default(),
            time_decay_factor: Decimal::new(995, 3), // 0.995 per day
            penalty_multiplier: Decimal::new(15, 1), // 1.5x penalty
            bonus_multiplier: Decimal::new(11, 1),   // 1.1x bonus
        }
    }
}

/// Reputation calculator that applies scoring algorithms
pub struct ReputationCalculator {
    /// Scoring configuration
    config: ScoringConfig,
}

/// Component scores for detailed analysis
#[derive(Debug, Clone)]
pub struct ComponentScores {
    /// Uptime score (0.0 - 1.0)
    pub uptime: Decimal,
    /// Data integrity score
    pub data_integrity: Decimal,
    /// Response time score
    pub response_time: Decimal,
    /// Contract compliance score
    pub contract_compliance: Decimal,
    /// Community feedback score
    pub community_feedback: Decimal,
    /// Raw event counts for transparency
    pub event_counts: EventCounts,
}

/// Event counts for transparency and debugging
#[derive(Debug, Clone)]
pub struct EventCounts {
    /// Total number of events processed
    pub total: usize,
    /// Events by category
    pub uptime_events: usize,
    /// Data integrity related events
    pub data_integrity_events: usize,
    /// Response time related events
    pub response_time_events: usize,
    /// Contract related events  
    pub contract_events: usize,
    /// Community feedback events
    pub feedback_events: usize,
    /// Positive vs negative event counts
    pub positive_events: usize,
    /// Negative events count
    pub negative_events: usize,
}

impl ReputationCalculator {
    /// Create a new reputation calculator
    pub fn new(config: ScoringConfig) -> Self {
        Self { config }
    }

    /// Calculate comprehensive reputation score from events
    pub fn calculate_score(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<ReputationScore> {
        if events.is_empty() {
            return Ok(ReputationScore::new());
        }

        let components = self.calculate_component_scores(events, current_time)?;

        // Calculate weighted overall score
        let overall = (components.uptime * self.config.weights.uptime)
            + (components.data_integrity * self.config.weights.data_integrity)
            + (components.response_time * self.config.weights.response_time)
            + (components.contract_compliance * self.config.weights.contract_compliance)
            + (components.community_feedback * self.config.weights.community_feedback);

        let contracts_completed = self.count_completed_contracts(events);

        Ok(ReputationScore {
            overall: self.normalize_score(overall),
            uptime: components.uptime,
            data_integrity: components.data_integrity,
            response_time: components.response_time,
            contract_compliance: components.contract_compliance,
            community_feedback: components.community_feedback,
            contracts_completed,
            last_updated: current_time,
        })
    }

    /// Calculate detailed component scores
    pub fn calculate_component_scores(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<ComponentScores> {
        let event_counts = self.count_events(events);

        // Calculate uptime score
        let uptime = self.calculate_uptime_score(events, current_time)?;

        // Calculate data integrity score
        let data_integrity = self.calculate_data_integrity_score(events, current_time)?;

        // Calculate response time score
        let response_time = self.calculate_response_time_score(events, current_time)?;

        // Calculate contract compliance score
        let contract_compliance = self.calculate_contract_compliance_score(events, current_time)?;

        // Calculate community feedback score
        let community_feedback = self.calculate_community_feedback_score(events, current_time)?;

        Ok(ComponentScores {
            uptime,
            data_integrity,
            response_time,
            contract_compliance,
            community_feedback,
            event_counts,
        })
    }

    /// Calculate uptime score from online/offline events
    fn calculate_uptime_score(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<Decimal> {
        let uptime_events: Vec<_> = events.iter().filter(|e| e.affects_uptime()).collect();

        if uptime_events.is_empty() {
            return Ok(Decimal::ONE); // Default to 100% uptime
        }

        let mut total_weighted_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for event in uptime_events {
            let weight = self.calculate_event_weight(event, current_time);
            let score = match event.event_type {
                crate::events::EventType::Online => Decimal::ONE,
                crate::events::EventType::Offline => Decimal::ZERO,
                crate::events::EventType::MaintenanceWindow {
                    announced: true, ..
                } => Decimal::new(9, 1), // 0.9
                crate::events::EventType::MaintenanceWindow {
                    announced: false, ..
                } => Decimal::new(5, 1), // 0.5
                _ => Decimal::new(8, 1), // 0.8 default
            };

            total_weighted_score += score * weight;
            total_weight += weight;
        }

        if total_weight == Decimal::ZERO {
            return Ok(Decimal::ONE);
        }

        Ok(self.normalize_score(total_weighted_score / total_weight))
    }

    /// Calculate data integrity score from proof events
    fn calculate_data_integrity_score(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<Decimal> {
        let integrity_events: Vec<_> = events
            .iter()
            .filter(|e| e.affects_data_integrity())
            .collect();

        if integrity_events.is_empty() {
            return Ok(Decimal::ONE); // Default to 100% integrity
        }

        let mut total_weighted_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for event in integrity_events {
            let weight = self.calculate_event_weight(event, current_time);
            let score = match event.event_type {
                crate::events::EventType::ProofSuccess { .. } => Decimal::ONE,
                crate::events::EventType::ProofFailure { .. } => Decimal::ZERO,
                crate::events::EventType::DataCorruption {
                    recovered: true, ..
                } => Decimal::new(3, 1), // 0.3
                crate::events::EventType::DataCorruption {
                    recovered: false, ..
                } => Decimal::ZERO,
                _ => Decimal::new(8, 1), // 0.8 default
            };

            total_weighted_score += score * weight;
            total_weight += weight;
        }

        if total_weight == Decimal::ZERO {
            return Ok(Decimal::ONE);
        }

        Ok(self.normalize_score(total_weighted_score / total_weight))
    }

    /// Calculate response time score from performance events
    fn calculate_response_time_score(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<Decimal> {
        let response_events: Vec<_> = events
            .iter()
            .filter(|e| e.affects_response_time())
            .collect();

        if response_events.is_empty() {
            return Ok(Decimal::new(8, 1)); // Default to 0.8
        }

        let mut total_weighted_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for event in response_events {
            let weight = self.calculate_event_weight(event, current_time);
            let score = if let Some(response_time_ms) = event.response_time_ms() {
                // Score based on response time (lower is better)
                // 0-100ms: 1.0, 100-500ms: 0.8, 500-2000ms: 0.6, >2000ms: 0.2
                if response_time_ms <= 100 {
                    Decimal::ONE
                } else if response_time_ms <= 500 {
                    Decimal::new(8, 1)
                } else if response_time_ms <= 2000 {
                    Decimal::new(6, 1)
                } else {
                    Decimal::new(2, 1)
                }
            } else {
                Decimal::new(8, 1) // Default score
            };

            total_weighted_score += score * weight;
            total_weight += weight;
        }

        if total_weight == Decimal::ZERO {
            return Ok(Decimal::new(8, 1));
        }

        Ok(self.normalize_score(total_weighted_score / total_weight))
    }

    /// Calculate contract compliance score
    fn calculate_contract_compliance_score(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<Decimal> {
        let contract_events: Vec<_> = events
            .iter()
            .filter(|e| e.affects_contract_compliance())
            .collect();

        if contract_events.is_empty() {
            return Ok(Decimal::ONE); // Default to 100% compliance
        }

        let mut total_weighted_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for event in contract_events {
            let weight = self.calculate_event_weight(event, current_time);
            let score = match event.event_type {
                crate::events::EventType::ContractCompleted { .. } => Decimal::ONE,
                crate::events::EventType::ContractViolated { .. } => Decimal::ZERO,
                crate::events::EventType::ContractStarted { .. } => Decimal::new(8, 1), // Neutral
                _ => Decimal::new(8, 1),
            };

            total_weighted_score += score * weight;
            total_weight += weight;
        }

        if total_weight == Decimal::ZERO {
            return Ok(Decimal::ONE);
        }

        Ok(self.normalize_score(total_weighted_score / total_weight))
    }

    /// Calculate community feedback score
    fn calculate_community_feedback_score(
        &self,
        events: &[ReputationEvent],
        current_time: DateTime<Utc>,
    ) -> Result<Decimal> {
        let feedback_events: Vec<_> = events
            .iter()
            .filter(|e| e.affects_community_feedback())
            .collect();

        if feedback_events.is_empty() {
            return Ok(Decimal::new(5, 1)); // Default to 0.5 (neutral)
        }

        let mut total_weighted_score = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for event in feedback_events {
            let weight = self.calculate_event_weight(event, current_time);
            let score = if let crate::events::EventType::CommunityFeedback { rating, .. } =
                event.event_type
            {
                // Convert 1-5 rating to 0.0-1.0 score
                Decimal::new(i64::from(rating) - 1, 1) / Decimal::new(4, 0) // (rating - 1) / 4
            } else {
                Decimal::new(5, 1) // Default neutral score
            };

            total_weighted_score += score * weight;
            total_weight += weight;
        }

        if total_weight == Decimal::ZERO {
            return Ok(Decimal::new(5, 1));
        }

        Ok(self.normalize_score(total_weighted_score / total_weight))
    }

    /// Calculate time-decayed weight for an event
    fn calculate_event_weight(
        &self,
        event: &ReputationEvent,
        current_time: DateTime<Utc>,
    ) -> Decimal {
        let time_decay =
            calculate_time_decay(event.timestamp, current_time, self.config.time_decay_factor);

        let base_weight = Decimal::from_f64_retain(event.weight()).unwrap_or(Decimal::ONE);

        // Apply penalty/bonus multipliers
        let multiplier = if event.impact_score() > 0.0 {
            self.config.bonus_multiplier
        } else if event.impact_score() < 0.0 {
            self.config.penalty_multiplier
        } else {
            Decimal::ONE
        };

        base_weight * time_decay * multiplier
    }

    /// Count completed contracts from events
    fn count_completed_contracts(&self, events: &[ReputationEvent]) -> u64 {
        events
            .iter()
            .filter(|e| {
                matches!(
                    e.event_type,
                    crate::events::EventType::ContractCompleted { .. }
                )
            })
            .count() as u64
    }

    /// Count events by category for transparency
    fn count_events(&self, events: &[ReputationEvent]) -> EventCounts {
        let total = events.len();
        let uptime_events = events.iter().filter(|e| e.affects_uptime()).count();
        let data_integrity_events = events.iter().filter(|e| e.affects_data_integrity()).count();
        let response_time_events = events.iter().filter(|e| e.affects_response_time()).count();
        let contract_events = events
            .iter()
            .filter(|e| e.affects_contract_compliance())
            .count();
        let feedback_events = events
            .iter()
            .filter(|e| e.affects_community_feedback())
            .count();

        let positive_events = events.iter().filter(|e| e.impact_score() > 0.0).count();
        let negative_events = events.iter().filter(|e| e.impact_score() < 0.0).count();

        EventCounts {
            total,
            uptime_events,
            data_integrity_events,
            response_time_events,
            contract_events,
            feedback_events,
            positive_events,
            negative_events,
        }
    }

    /// Normalize score to 0.0-1.0 range
    fn normalize_score(&self, score: Decimal) -> Decimal {
        if score > Decimal::ONE {
            Decimal::ONE
        } else if score < Decimal::ZERO {
            Decimal::ZERO
        } else {
            score
        }
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ScoringConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &ScoringConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventSeverity, EventType, ReputationEvent};

    #[test]
    fn test_weights_validation() {
        let valid_weights = ReputationWeights::default();
        assert!(valid_weights.validate().is_ok());

        let invalid_weights = ReputationWeights {
            uptime: Decimal::new(5, 1),              // 0.5
            data_integrity: Decimal::new(3, 1),      // 0.3
            response_time: Decimal::new(1, 1),       // 0.1
            contract_compliance: Decimal::new(2, 1), // 0.2
            community_feedback: Decimal::new(1, 1),  // 0.1 (total = 1.2)
        };
        assert!(invalid_weights.validate().is_err());
    }

    #[test]
    fn test_calculator_creation() {
        let config = ScoringConfig::default();
        let calculator = ReputationCalculator::new(config);
        assert!(calculator.config.weights.validate().is_ok());
    }

    #[test]
    fn test_empty_events_scoring() {
        let calculator = ReputationCalculator::new(ScoringConfig::default());
        let events = vec![];
        let score = calculator.calculate_score(&events, Utc::now()).unwrap();

        assert_eq!(score.overall, ReputationScore::new().overall);
    }

    #[test]
    fn test_online_event_scoring() {
        let calculator = ReputationCalculator::new(ScoringConfig::default());
        let provider_id = uuid::Uuid::new_v4();

        let event = ReputationEvent::new(provider_id, EventType::Online, EventSeverity::Positive);

        let events = vec![event];
        let score = calculator.calculate_score(&events, Utc::now()).unwrap();

        // Should have perfect uptime score
        assert_eq!(score.uptime, Decimal::ONE);
    }

    #[test]
    fn test_proof_success_scoring() {
        let calculator = ReputationCalculator::new(ScoringConfig::default());
        let provider_id = uuid::Uuid::new_v4();

        let event = ReputationEvent::new(
            provider_id,
            EventType::ProofSuccess {
                response_time_ms: 150,
                chunks_proven: 5,
            },
            EventSeverity::Positive,
        );

        let events = vec![event];
        let score = calculator.calculate_score(&events, Utc::now()).unwrap();

        // Should have perfect data integrity and good response time
        assert_eq!(score.data_integrity, Decimal::ONE);
        assert!(score.response_time >= Decimal::new(8, 1)); // >= 0.8 (150ms is in good range)
    }

    #[test]
    fn test_component_scores() {
        let calculator = ReputationCalculator::new(ScoringConfig::default());
        let provider_id = uuid::Uuid::new_v4();

        let events = vec![
            ReputationEvent::new(provider_id, EventType::Online, EventSeverity::Positive),
            ReputationEvent::new(
                provider_id,
                EventType::ProofSuccess {
                    response_time_ms: 100,
                    chunks_proven: 3,
                },
                EventSeverity::Positive,
            ),
        ];

        let components = calculator
            .calculate_component_scores(&events, Utc::now())
            .unwrap();

        assert_eq!(components.uptime, Decimal::ONE);
        assert_eq!(components.data_integrity, Decimal::ONE);
        assert_eq!(components.event_counts.total, 2);
        assert_eq!(components.event_counts.positive_events, 2);
    }
}
