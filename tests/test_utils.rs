//! Test utilities for Carbide Network integration tests
//!
//! This module provides utilities for:
//! - Mock implementations for testing
//! - Test data generation
//! - Performance benchmarking
//! - Test environment setup helpers

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;
use tokio::sync::RwLock;

use carbide_core::*;
use carbide_reputation::events::*;

/// Mock provider for testing without full provider implementation
#[derive(Debug, Clone)]
pub struct MockProvider {
    pub id: ProviderId,
    pub info: Provider,
    pub stored_files: Arc<Mutex<HashMap<FileId, Vec<u8>>>>,
    pub latency_ms: u64,
    pub failure_rate: f32, // 0.0 = never fails, 1.0 = always fails
    pub is_online: Arc<Mutex<bool>>,
}

impl MockProvider {
    /// Create a new mock provider
    pub fn new(
        name: String,
        tier: ProviderTier,
        region: Region,
        capacity_gb: u64,
        price_per_gb: rust_decimal::Decimal,
    ) -> Self {
        let provider = Provider::new(
            name,
            tier,
            region,
            format!("http://mock-provider-{}.test", Uuid::new_v4()),
            capacity_gb * 1024 * 1024 * 1024, // Convert GB to bytes
            price_per_gb,
        );

        Self {
            id: provider.id,
            info: provider,
            stored_files: Arc::new(Mutex::new(HashMap::new())),
            latency_ms: 100, // Default 100ms latency
            failure_rate: 0.0, // No failures by default
            is_online: Arc::new(Mutex::new(true)),
        }
    }

    /// Set provider latency
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Set provider failure rate (0.0 - 1.0)
    pub fn with_failure_rate(mut self, failure_rate: f32) -> Self {
        self.failure_rate = failure_rate.clamp(0.0, 1.0);
        self
    }

    /// Simulate storing a file
    pub async fn store_file(&self, file_id: FileId, data: &[u8]) -> Result<()> {
        // Simulate latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;

        // Check if provider is online
        {
            let online = *self.is_online.lock().unwrap();
            if !online {
                return Err(CarbideError::Provider("Provider is offline".to_string()));
            }
        }

        // Simulate random failures
        if self.failure_rate > 0.0 {
            let random_val: f32 = rand::random();
            if random_val < self.failure_rate {
                return Err(CarbideError::Provider("Random storage failure".to_string()));
            }
        }

        // Check capacity
        let current_usage = {
            let files = self.stored_files.lock().unwrap();
            files.values().map(|data| data.len() as u64).sum::<u64>()
        };

        if current_usage + data.len() as u64 > self.info.total_capacity {
            return Err(CarbideError::Provider("Insufficient capacity".to_string()));
        }

        // Store the file
        {
            let mut files = self.stored_files.lock().unwrap();
            files.insert(file_id, data.to_vec());
        }

        Ok(())
    }

    /// Simulate retrieving a file
    pub async fn retrieve_file(&self, file_id: &FileId) -> Result<Vec<u8>> {
        // Simulate latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;

        // Check if provider is online
        {
            let online = *self.is_online.lock().unwrap();
            if !online {
                return Err(CarbideError::Provider("Provider is offline".to_string()));
            }
        }

        // Simulate random failures
        if self.failure_rate > 0.0 {
            let random_val: f32 = rand::random();
            if random_val < self.failure_rate {
                return Err(CarbideError::Provider("Random retrieval failure".to_string()));
            }
        }

        // Retrieve the file
        let files = self.stored_files.lock().unwrap();
        files.get(file_id)
            .cloned()
            .ok_or_else(|| CarbideError::NotFound(format!("File {} not found", file_id)))
    }

    /// Set provider online/offline status
    pub fn set_online(&self, online: bool) {
        *self.is_online.lock().unwrap() = online;
    }

    /// Get current storage usage
    pub fn storage_usage(&self) -> u64 {
        let files = self.stored_files.lock().unwrap();
        files.values().map(|data| data.len() as u64).sum()
    }

    /// Get number of stored files
    pub fn file_count(&self) -> usize {
        self.stored_files.lock().unwrap().len()
    }
}

/// Performance benchmark utilities
pub struct PerformanceBenchmark {
    operations: Vec<BenchmarkOperation>,
    start_time: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkOperation {
    pub name: String,
    pub duration: Duration,
    pub success: bool,
    pub bytes_processed: Option<u64>,
}

impl PerformanceBenchmark {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            start_time: None,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn record_operation(
        &mut self,
        name: String,
        duration: Duration,
        success: bool,
        bytes_processed: Option<u64>,
    ) {
        self.operations.push(BenchmarkOperation {
            name,
            duration,
            success,
            bytes_processed,
        });
    }

    pub fn total_duration(&self) -> Duration {
        self.start_time
            .map(|start| start.elapsed())
            .unwrap_or_default()
    }

    pub fn success_rate(&self) -> f64 {
        if self.operations.is_empty() {
            return 0.0;
        }
        let successful = self.operations.iter().filter(|op| op.success).count();
        successful as f64 / self.operations.len() as f64
    }

    pub fn average_operation_time(&self) -> Duration {
        if self.operations.is_empty() {
            return Duration::from_secs(0);
        }
        let total: Duration = self.operations.iter().map(|op| op.duration).sum();
        total / self.operations.len() as u32
    }

    pub fn throughput_mbps(&self) -> f64 {
        let total_bytes: u64 = self.operations
            .iter()
            .filter_map(|op| op.bytes_processed)
            .sum();
        
        if total_bytes == 0 || self.total_duration().is_zero() {
            return 0.0;
        }

        let mb = total_bytes as f64 / (1024.0 * 1024.0);
        let seconds = self.total_duration().as_secs_f64();
        mb / seconds
    }

    pub fn report(&self) -> String {
        format!(
            "Performance Report:\n\
             - Total operations: {}\n\
             - Success rate: {:.1}%\n\
             - Total duration: {:?}\n\
             - Average operation time: {:?}\n\
             - Throughput: {:.2} MB/s",
            self.operations.len(),
            self.success_rate() * 100.0,
            self.total_duration(),
            self.average_operation_time(),
            self.throughput_mbps()
        )
    }
}

/// Test data generator for various scenarios
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate test file data of specified size
    pub fn generate_file_data(size_bytes: usize) -> Vec<u8> {
        let pattern = b"Carbide Network Test Data - ";
        let mut data = Vec::with_capacity(size_bytes);
        
        while data.len() < size_bytes {
            let remaining = size_bytes - data.len();
            if remaining >= pattern.len() {
                data.extend_from_slice(pattern);
            } else {
                data.extend_from_slice(&pattern[..remaining]);
            }
        }
        
        data
    }

    /// Generate random binary data
    pub fn generate_random_data(size_bytes: usize) -> Vec<u8> {
        (0..size_bytes).map(|_| rand::random::<u8>()).collect()
    }

    /// Generate test provider configuration
    pub fn generate_provider_config(
        index: usize,
        tier: ProviderTier,
        region: Region,
    ) -> (String, u64, rust_decimal::Decimal) {
        let name = format!("Test Provider {}", index);
        let capacity_gb = match tier {
            ProviderTier::Home => 1 + (index as u64 % 5),          // 1-5 GB
            ProviderTier::Professional => 10 + (index as u64 % 50), // 10-60 GB
            ProviderTier::Enterprise => 100 + (index as u64 % 900), // 100-1000 GB
            ProviderTier::GlobalCDN => 1000 + (index as u64 % 9000), // 1-10 TB
        };
        
        let price = tier.typical_price() + rust_decimal::Decimal::new((index % 3) as i64, 3);
        
        (name, capacity_gb, price)
    }

    /// Generate reputation events for testing
    pub fn generate_reputation_events(
        provider_id: ProviderId,
        event_count: usize,
        positive_ratio: f32, // 0.0-1.0, ratio of positive events
    ) -> Vec<ReputationEvent> {
        let mut events = Vec::with_capacity(event_count);
        
        for i in 0..event_count {
            let is_positive = (i as f32 / event_count as f32) < positive_ratio;
            
            let (event_type, severity) = if is_positive {
                match i % 4 {
                    0 => (EventType::Online, EventSeverity::Positive),
                    1 => (
                        EventType::ProofSuccess { 
                            response_time_ms: 100 + (i % 400) as u64, 
                            chunks_proven: 3 + (i % 5) as u32 
                        }, 
                        EventSeverity::Positive
                    ),
                    2 => (
                        EventType::UploadSuccess {
                            file_size: 1024 + (i % 1024) as u64,
                            upload_time_ms: 500 + (i % 2000) as u64,
                        },
                        EventSeverity::Positive
                    ),
                    _ => (
                        EventType::ContractCompleted {
                            final_value: rust_decimal::Decimal::new((10 + i % 90) as i64, 2),
                            duration_served_days: 30,
                        },
                        EventSeverity::ExtremelyPositive
                    ),
                }
            } else {
                match i % 3 {
                    0 => (EventType::Offline, EventSeverity::Negative),
                    1 => (
                        EventType::ProofFailure {
                            reason: "Timeout".to_string(),
                            error_details: Some("Network timeout".to_string()),
                        },
                        EventSeverity::Negative
                    ),
                    _ => (
                        EventType::UploadFailure {
                            reason: "Storage full".to_string(),
                            partial_bytes: Some(512),
                        },
                        EventSeverity::Negative
                    ),
                }
            };
            
            let mut event = ReputationEvent::new(provider_id, event_type, severity);
            
            // Add some time variation
            event.timestamp = chrono::Utc::now() 
                - chrono::Duration::minutes((event_count - i) as i64 * 5);
            
            events.push(event);
        }
        
        events
    }

    /// Generate test file metadata
    pub fn generate_file_metadata(index: usize) -> (String, String, usize) {
        let extensions = ["txt", "pdf", "jpg", "mp4", "zip"];
        let types = ["text/plain", "application/pdf", "image/jpeg", "video/mp4", "application/zip"];
        
        let ext_index = index % extensions.len();
        let filename = format!("test_file_{}.{}", index, extensions[ext_index]);
        let mime_type = types[ext_index].to_string();
        let size = match ext_index {
            0 => 1024 + (index % 10240),      // Text: 1KB - 10KB
            1 => 102400 + (index % 1048576),  // PDF: 100KB - 1MB
            2 => 51200 + (index % 5242880),   // Image: 50KB - 5MB
            3 => 1048576 + (index % 104857600), // Video: 1MB - 100MB
            _ => 10240 + (index % 1048576),   // Archive: 10KB - 1MB
        };
        
        (filename, mime_type, size)
    }
}

/// Network simulation utilities for testing under various conditions
pub struct NetworkSimulator {
    latency_ms: u64,
    packet_loss_rate: f32,
    bandwidth_mbps: f64,
}

impl NetworkSimulator {
    pub fn new() -> Self {
        Self {
            latency_ms: 0,
            packet_loss_rate: 0.0,
            bandwidth_mbps: f64::INFINITY,
        }
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    pub fn with_packet_loss(mut self, loss_rate: f32) -> Self {
        self.packet_loss_rate = loss_rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_bandwidth_limit(mut self, bandwidth_mbps: f64) -> Self {
        self.bandwidth_mbps = bandwidth_mbps;
        self
    }

    /// Simulate network delay
    pub async fn simulate_delay(&self) {
        if self.latency_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
        }
    }

    /// Simulate packet loss (returns true if packet should be dropped)
    pub fn should_drop_packet(&self) -> bool {
        if self.packet_loss_rate <= 0.0 {
            return false;
        }
        rand::random::<f32>() < self.packet_loss_rate
    }

    /// Calculate transfer time based on bandwidth limit
    pub fn transfer_duration(&self, bytes: u64) -> Duration {
        if self.bandwidth_mbps.is_infinite() {
            return Duration::from_secs(0);
        }
        
        let mb = bytes as f64 / (1024.0 * 1024.0);
        let seconds = mb / self.bandwidth_mbps;
        Duration::from_secs_f64(seconds.max(0.0))
    }

    /// Simulate network conditions for an operation
    pub async fn simulate_network_operation<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Simulate initial latency
        self.simulate_delay().await;
        
        // Check for packet loss
        if self.should_drop_packet() {
            return Err(CarbideError::Provider("Network packet loss".to_string()));
        }
        
        // Execute operation
        let result = operation().await;
        
        // Simulate additional latency for response
        self.simulate_delay().await;
        
        result
    }
}

/// Test assertion helpers
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that a reputation score is within expected bounds
    pub fn assert_reputation_bounds(
        score: &ReputationScore,
        min_overall: f32,
        max_overall: f32,
    ) {
        use rust_decimal::prelude::ToPrimitive;
        
        let overall = score.overall.to_f32().unwrap_or(0.0);
        assert!(
            overall >= min_overall && overall <= max_overall,
            "Reputation score {} not in range [{}, {}]",
            overall, min_overall, max_overall
        );
        
        // All component scores should be in [0, 1] range
        let components = [
            score.uptime,
            score.data_integrity,
            score.response_time,
            score.contract_compliance,
            score.community_feedback,
        ];
        
        for (i, component) in components.iter().enumerate() {
            let value = component.to_f32().unwrap_or(-1.0);
            assert!(
                value >= 0.0 && value <= 1.0,
                "Component score {} ({}) not in range [0, 1]",
                i, value
            );
        }
    }

    /// Assert that providers are properly sorted by reputation
    pub fn assert_providers_sorted_by_reputation(providers: &[(ProviderId, ReputationScore)]) {
        use rust_decimal::prelude::ToPrimitive;
        
        for window in providers.windows(2) {
            let score1 = window[0].1.overall.to_f32().unwrap_or(0.0);
            let score2 = window[1].1.overall.to_f32().unwrap_or(0.0);
            assert!(
                score1 >= score2,
                "Providers not sorted by reputation: {} < {}",
                score1, score2
            );
        }
    }

    /// Assert that file content matches expected hash
    pub fn assert_file_integrity(data: &[u8], expected_hash: &ContentHash) {
        let actual_hash = ContentHash::from_data(data);
        assert_eq!(
            actual_hash, *expected_hash,
            "File integrity check failed: expected {}, got {}",
            expected_hash, actual_hash
        );
    }

    /// Assert that response time is within acceptable bounds
    pub fn assert_response_time(duration: Duration, max_expected: Duration) {
        assert!(
            duration <= max_expected,
            "Response time {:?} exceeds maximum expected {:?}",
            duration, max_expected
        );
    }
}

#[cfg(test)]
mod test_utils_tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider::new(
            "Test Provider".to_string(),
            ProviderTier::Professional,
            Region::NorthAmerica,
            10,
            rust_decimal::Decimal::new(5, 3),
        );

        let test_data = b"Hello, World!";
        let file_id = ContentHash::from_data(test_data);

        // Test successful storage
        assert!(provider.store_file(file_id, test_data).await.is_ok());
        assert_eq!(provider.file_count(), 1);

        // Test successful retrieval
        let retrieved = provider.retrieve_file(&file_id).await.unwrap();
        assert_eq!(retrieved, test_data);

        // Test offline behavior
        provider.set_online(false);
        assert!(provider.store_file(ContentHash::from_data(b"test"), b"test").await.is_err());
        assert!(provider.retrieve_file(&file_id).await.is_err());
    }

    #[test]
    fn test_performance_benchmark() {
        let mut benchmark = PerformanceBenchmark::new();
        benchmark.start();

        benchmark.record_operation(
            "test_op_1".to_string(),
            Duration::from_millis(100),
            true,
            Some(1024),
        );

        benchmark.record_operation(
            "test_op_2".to_string(),
            Duration::from_millis(200),
            false,
            Some(2048),
        );

        assert_eq!(benchmark.operations.len(), 2);
        assert_eq!(benchmark.success_rate(), 0.5);
        assert_eq!(benchmark.average_operation_time(), Duration::from_millis(150));
    }

    #[test]
    fn test_data_generator() {
        let data = TestDataGenerator::generate_file_data(100);
        assert_eq!(data.len(), 100);

        let random_data = TestDataGenerator::generate_random_data(50);
        assert_eq!(random_data.len(), 50);

        let (name, capacity, price) = TestDataGenerator::generate_provider_config(
            1,
            ProviderTier::Professional,
            Region::NorthAmerica,
        );
        assert_eq!(name, "Test Provider 1");
        assert!(capacity >= 10);
    }
}