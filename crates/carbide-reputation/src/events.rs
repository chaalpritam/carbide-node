//! Reputation event types and processing
//!
//! This module defines the different types of events that can affect
//! a provider's reputation and provides utilities for event processing.

use carbide_core::ProviderId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A reputation-affecting event for a storage provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    /// Unique event identifier
    pub id: uuid::Uuid,
    /// Provider this event affects
    pub provider_id: ProviderId,
    /// Type of event
    pub event_type: EventType,
    /// Event severity/impact
    pub severity: EventSeverity,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Additional event details
    pub details: std::collections::HashMap<String, String>,
    /// Raw event value (for metrics)
    pub value: Option<f64>,
    /// Event context (e.g., contract_id, file_id)
    pub context: EventContext,
}

/// Types of reputation events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// Provider came online
    Online,
    /// Provider went offline
    Offline,
    /// Successful proof-of-storage response
    ProofSuccess {
        /// Response time in milliseconds
        response_time_ms: u64,
        /// Number of chunks proven
        chunks_proven: u32,
    },
    /// Failed proof-of-storage response
    ProofFailure {
        /// Failure reason
        reason: String,
        /// Expected vs actual response
        error_details: Option<String>,
    },
    /// Successful file upload
    UploadSuccess {
        /// File size in bytes
        file_size: u64,
        /// Upload time in milliseconds
        upload_time_ms: u64,
    },
    /// Failed file upload
    UploadFailure {
        /// Failure reason
        reason: String,
        /// Partial upload size if applicable
        partial_bytes: Option<u64>,
    },
    /// Successful file download
    DownloadSuccess {
        /// File size in bytes
        file_size: u64,
        /// Download time in milliseconds
        download_time_ms: u64,
    },
    /// Failed file download
    DownloadFailure {
        /// Failure reason
        reason: String,
    },
    /// Contract started
    ContractStarted {
        /// Contract value
        contract_value: rust_decimal::Decimal,
        /// Contract duration in months
        duration_months: u32,
    },
    /// Contract completed successfully
    ContractCompleted {
        /// Final contract value
        final_value: rust_decimal::Decimal,
        /// Duration actually served
        duration_served_days: u32,
    },
    /// Contract violated or terminated
    ContractViolated {
        /// Violation reason
        reason: String,
        /// Penalty amount
        penalty: Option<rust_decimal::Decimal>,
    },
    /// Health check response
    HealthCheck {
        /// Response time in milliseconds
        response_time_ms: u64,
        /// Health status
        status: String,
    },
    /// Community feedback
    CommunityFeedback {
        /// Rating (1-5)
        rating: u8,
        /// Feedback category
        category: FeedbackCategory,
        /// Optional comment
        comment: Option<String>,
    },
    /// Performance metrics update
    PerformanceUpdate {
        /// CPU usage percentage
        cpu_usage: f32,
        /// Memory usage percentage
        memory_usage: f32,
        /// Disk usage percentage
        disk_usage: f32,
        /// Network latency in ms
        latency_ms: f32,
    },
    /// Provider maintenance window
    MaintenanceWindow {
        /// Scheduled maintenance duration in minutes
        duration_minutes: u32,
        /// Whether it was announced in advance
        announced: bool,
    },
    /// Data corruption detected
    DataCorruption {
        /// Number of corrupted files
        corrupted_files: u32,
        /// Total size of corrupted data
        corrupted_bytes: u64,
        /// Recovery successful
        recovered: bool,
    },
    /// Suspicious activity detected
    SuspiciousActivity {
        /// Activity type
        activity_type: String,
        /// Confidence level (0-1)
        confidence: f32,
    },
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventSeverity {
    /// Very positive event (+2 impact)
    ExtremelyPositive,
    /// Positive event (+1 impact)
    Positive,
    /// Neutral event (no impact)
    Neutral,
    /// Negative event (-1 impact)
    Negative,
    /// Very negative event (-2 impact)
    ExtremelyNegative,
}

/// Community feedback categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackCategory {
    /// Overall service quality
    ServiceQuality,
    /// Reliability and uptime
    Reliability,
    /// Performance and speed
    Performance,
    /// Support and communication
    Support,
    /// Value for money
    Value,
}

/// Event context information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventContext {
    /// Associated contract ID
    pub contract_id: Option<uuid::Uuid>,
    /// Associated file ID
    pub file_id: Option<String>,
    /// Client/user ID who triggered event
    pub client_id: Option<String>,
    /// Geographic region where event occurred
    pub region: Option<String>,
    /// Provider tier at time of event
    pub tier: Option<String>,
    /// Additional tags for categorization
    pub tags: Vec<String>,
}

impl ReputationEvent {
    /// Create a new reputation event
    pub fn new(provider_id: ProviderId, event_type: EventType, severity: EventSeverity) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            provider_id,
            event_type,
            severity,
            timestamp: Utc::now(),
            details: std::collections::HashMap::new(),
            value: None,
            context: EventContext::default(),
        }
    }

    /// Create event with additional details
    pub fn with_details(mut self, details: std::collections::HashMap<String, String>) -> Self {
        self.details = details;
        self
    }

    /// Create event with context
    pub fn with_context(mut self, context: EventContext) -> Self {
        self.context = context;
        self
    }

    /// Create event with value
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// Get impact score based on event type and severity
    pub fn impact_score(&self) -> f64 {
        let base_score = match self.severity {
            EventSeverity::ExtremelyPositive => 2.0,
            EventSeverity::Positive => 1.0,
            EventSeverity::Neutral => 0.0,
            EventSeverity::Negative => -1.0,
            EventSeverity::ExtremelyNegative => -2.0,
        };

        // Adjust based on event type
        let type_multiplier = match self.event_type {
            EventType::DataCorruption { .. } => 2.0,
            EventType::ContractViolated { .. } => 1.5,
            EventType::ProofFailure { .. } => 1.2,
            EventType::SuspiciousActivity { .. } => 1.8,
            EventType::ContractCompleted { .. } => 1.5,
            EventType::ProofSuccess { .. } => 1.0,
            _ => 1.0,
        };

        base_score * type_multiplier
    }

    /// Check if event affects uptime
    pub fn affects_uptime(&self) -> bool {
        matches!(
            self.event_type,
            EventType::Online | EventType::Offline | EventType::MaintenanceWindow { .. }
        )
    }

    /// Check if event affects data integrity
    pub fn affects_data_integrity(&self) -> bool {
        matches!(
            self.event_type,
            EventType::ProofSuccess { .. }
                | EventType::ProofFailure { .. }
                | EventType::DataCorruption { .. }
        )
    }

    /// Check if event affects response time
    pub fn affects_response_time(&self) -> bool {
        matches!(
            self.event_type,
            EventType::ProofSuccess { .. }
                | EventType::HealthCheck { .. }
                | EventType::UploadSuccess { .. }
                | EventType::DownloadSuccess { .. }
                | EventType::PerformanceUpdate { .. }
        )
    }

    /// Check if event affects contract compliance
    pub fn affects_contract_compliance(&self) -> bool {
        matches!(
            self.event_type,
            EventType::ContractStarted { .. }
                | EventType::ContractCompleted { .. }
                | EventType::ContractViolated { .. }
        )
    }

    /// Check if event affects community feedback
    pub fn affects_community_feedback(&self) -> bool {
        matches!(self.event_type, EventType::CommunityFeedback { .. })
    }

    /// Extract response time from event if available
    pub fn response_time_ms(&self) -> Option<u64> {
        match &self.event_type {
            EventType::ProofSuccess {
                response_time_ms, ..
            } => Some(*response_time_ms),
            EventType::HealthCheck {
                response_time_ms, ..
            } => Some(*response_time_ms),
            EventType::UploadSuccess { upload_time_ms, .. } => Some(*upload_time_ms),
            EventType::DownloadSuccess {
                download_time_ms, ..
            } => Some(*download_time_ms),
            _ => None,
        }
    }

    /// Get event weight for reputation calculation
    pub fn weight(&self) -> f64 {
        match &self.event_type {
            // Critical events have higher weight
            EventType::DataCorruption { .. } => 3.0,
            EventType::ContractViolated { .. } => 2.5,
            EventType::SuspiciousActivity { .. } => 2.0,

            // Important events
            EventType::ProofFailure { .. } => 1.5,
            EventType::ContractCompleted { .. } => 1.5,
            EventType::UploadFailure { .. } => 1.2,
            EventType::DownloadFailure { .. } => 1.2,

            // Standard events
            EventType::ProofSuccess { .. } => 1.0,
            EventType::UploadSuccess { .. } => 1.0,
            EventType::DownloadSuccess { .. } => 1.0,
            EventType::HealthCheck { .. } => 1.0,

            // Lower weight events
            EventType::Online => 0.8,
            EventType::Offline => 1.2,
            EventType::MaintenanceWindow {
                announced: true, ..
            } => 0.5,
            EventType::MaintenanceWindow {
                announced: false, ..
            } => 1.0,

            // Community feedback weight varies by category
            EventType::CommunityFeedback { category, .. } => match category {
                FeedbackCategory::ServiceQuality => 1.2,
                FeedbackCategory::Reliability => 1.5,
                FeedbackCategory::Performance => 1.0,
                FeedbackCategory::Support => 0.8,
                FeedbackCategory::Value => 0.6,
            },

            _ => 1.0,
        }
    }
}

impl EventSeverity {
    /// Convert to numeric impact value
    pub fn to_impact(&self) -> f64 {
        match self {
            Self::ExtremelyPositive => 2.0,
            Self::Positive => 1.0,
            Self::Neutral => 0.0,
            Self::Negative => -1.0,
            Self::ExtremelyNegative => -2.0,
        }
    }
}

/// Event builder for convenient event creation
pub struct EventBuilder {
    event: ReputationEvent,
}

impl EventBuilder {
    /// Create a new event builder
    pub fn new(provider_id: ProviderId, event_type: EventType) -> Self {
        let severity = EventSeverity::Neutral; // Default severity
        Self {
            event: ReputationEvent::new(provider_id, event_type, severity),
        }
    }

    /// Set event severity
    pub fn severity(mut self, severity: EventSeverity) -> Self {
        self.event.severity = severity;
        self
    }

    /// Add detail
    pub fn detail(mut self, key: String, value: String) -> Self {
        self.event.details.insert(key, value);
        self
    }

    /// Set context
    pub fn context(mut self, context: EventContext) -> Self {
        self.event.context = context;
        self
    }

    /// Set value
    pub fn value(mut self, value: f64) -> Self {
        self.event.value = Some(value);
        self
    }

    /// Build the event
    pub fn build(self) -> ReputationEvent {
        self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let provider_id = uuid::Uuid::new_v4();
        let event = ReputationEvent::new(provider_id, EventType::Online, EventSeverity::Positive);

        assert_eq!(event.provider_id, provider_id);
        assert!(matches!(event.event_type, EventType::Online));
        assert!(matches!(event.severity, EventSeverity::Positive));
    }

    #[test]
    fn test_impact_score() {
        let provider_id = uuid::Uuid::new_v4();

        // Positive proof success
        let event = ReputationEvent::new(
            provider_id,
            EventType::ProofSuccess {
                response_time_ms: 100,
                chunks_proven: 5,
            },
            EventSeverity::Positive,
        );
        assert_eq!(event.impact_score(), 1.0);

        // Data corruption is severely negative
        let event = ReputationEvent::new(
            provider_id,
            EventType::DataCorruption {
                corrupted_files: 2,
                corrupted_bytes: 1024,
                recovered: false,
            },
            EventSeverity::ExtremelyNegative,
        );
        assert_eq!(event.impact_score(), -4.0); // -2.0 * 2.0
    }

    #[test]
    fn test_event_categorization() {
        let provider_id = uuid::Uuid::new_v4();

        let uptime_event =
            ReputationEvent::new(provider_id, EventType::Online, EventSeverity::Positive);
        assert!(uptime_event.affects_uptime());
        assert!(!uptime_event.affects_data_integrity());

        let proof_event = ReputationEvent::new(
            provider_id,
            EventType::ProofSuccess {
                response_time_ms: 100,
                chunks_proven: 3,
            },
            EventSeverity::Positive,
        );
        assert!(proof_event.affects_data_integrity());
        assert!(proof_event.affects_response_time());
    }

    #[test]
    fn test_event_builder() {
        let provider_id = uuid::Uuid::new_v4();

        let event = EventBuilder::new(
            provider_id,
            EventType::UploadSuccess {
                file_size: 1024,
                upload_time_ms: 500,
            },
        )
        .severity(EventSeverity::Positive)
        .detail("client".to_string(), "test_client".to_string())
        .value(1024.0)
        .build();

        assert!(matches!(event.severity, EventSeverity::Positive));
        assert_eq!(
            event.details.get("client"),
            Some(&"test_client".to_string())
        );
        assert_eq!(event.value, Some(1024.0));
    }

    #[test]
    fn test_response_time_extraction() {
        let provider_id = uuid::Uuid::new_v4();

        let event = ReputationEvent::new(
            provider_id,
            EventType::ProofSuccess {
                response_time_ms: 150,
                chunks_proven: 2,
            },
            EventSeverity::Positive,
        );

        assert_eq!(event.response_time_ms(), Some(150));
    }
}
