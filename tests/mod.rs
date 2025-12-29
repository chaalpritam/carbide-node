//! Carbide Network Test Suite
//!
//! This module provides comprehensive testing for the Carbide Network:
//!
//! ## Test Categories
//!
//! ### 1. Unit Tests (`unit_tests.rs`)
//! - Core data structures and types
//! - Cryptographic functions
//! - Reputation scoring algorithms
//! - Client SDK functionality
//! - Network protocol serialization
//!
//! ### 2. Integration Tests (`integration_tests.rs`)
//! - End-to-end file storage and retrieval
//! - Multi-provider replication
//! - Discovery service functionality
//! - Reputation tracking across components
//! - Error handling and resilience
//! - Concurrent operations
//! - Performance benchmarking
//!
//! ### 3. Test Utilities (`test_utils.rs`)
//! - Mock provider implementations
//! - Performance benchmarking tools
//! - Test data generators
//! - Network simulation utilities
//! - Test assertion helpers
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all tests
//! cargo test
//!
//! # Run only unit tests
//! cargo test unit_tests
//!
//! # Run only integration tests (requires more setup)
//! cargo test integration_tests
//!
//! # Run specific test
//! cargo test test_reputation_tracking
//!
//! # Run tests with output
//! cargo test -- --nocapture
//!
//! # Run tests in release mode (for performance tests)
//! cargo test --release
//! ```
//!
//! ## Test Environment Variables
//!
//! - `CARBIDE_TEST_TIMEOUT`: Override default test timeout (seconds)
//! - `CARBIDE_TEST_LOG_LEVEL`: Set log level for tests (debug, info, warn, error)
//! - `CARBIDE_TEST_TEMP_DIR`: Custom temporary directory for test data
//!
//! ## Test Data Management
//!
//! All tests use temporary directories that are automatically cleaned up.
//! Test files are prefixed with `carbide_test_` for easy identification.

pub mod integration_tests;
pub mod unit_tests;
pub mod test_utils;

use std::sync::Once;
use tracing::Level;

static INIT: Once = Once::new();

/// Initialize test logging (call once per test session)
pub fn init_test_logging() {
    INIT.call_once(|| {
        let log_level = std::env::var("CARBIDE_TEST_LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());
        
        let level = match log_level.to_lowercase().as_str() {
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };

        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_test_writer()
            .init();
    });
}

/// Test configuration for consistent test parameters
pub struct TestConfig {
    pub timeout_seconds: u64,
    pub temp_dir: std::path::PathBuf,
    pub log_level: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        let timeout = std::env::var("CARBIDE_TEST_TIMEOUT")
            .and_then(|s| s.parse().ok())
            .unwrap_or(30); // 30 second default timeout

        let temp_dir = std::env::var("CARBIDE_TEST_TEMP_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());

        let log_level = std::env::var("CARBIDE_TEST_LOG_LEVEL")
            .unwrap_or_else(|_| "info".to_string());

        Self {
            timeout_seconds: timeout,
            temp_dir,
            log_level,
        }
    }
}

impl TestConfig {
    /// Get test timeout as Duration
    pub fn timeout_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_seconds)
    }

    /// Create a unique temporary directory for a test
    pub fn create_test_dir(&self, test_name: &str) -> std::io::Result<std::path::PathBuf> {
        let test_dir = self.temp_dir.join(format!("carbide_test_{}_{}", 
            test_name, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir)?;
        Ok(test_dir)
    }
}

/// Common test assertions and utilities
pub mod assertions {
    use std::time::Duration;

    /// Assert that an operation completes within the expected time
    pub fn assert_duration_within(actual: Duration, expected: Duration, tolerance: f32) {
        let expected_secs = expected.as_secs_f32();
        let actual_secs = actual.as_secs_f32();
        let tolerance_secs = expected_secs * tolerance;
        
        assert!(
            (actual_secs - expected_secs).abs() <= tolerance_secs,
            "Duration {} not within {}% of expected {}", 
            actual_secs, tolerance * 100.0, expected_secs
        );
    }

    /// Assert that a success rate is above the minimum threshold
    pub fn assert_success_rate(successful: usize, total: usize, min_rate: f32) {
        assert!(total > 0, "Cannot calculate success rate with zero total operations");
        
        let actual_rate = successful as f32 / total as f32;
        assert!(
            actual_rate >= min_rate,
            "Success rate {:.1}% below minimum {:.1}%",
            actual_rate * 100.0, min_rate * 100.0
        );
    }

    /// Assert that throughput is above minimum threshold
    pub fn assert_throughput(bytes: u64, duration: Duration, min_mbps: f64) {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        let seconds = duration.as_secs_f64();
        let actual_mbps = mb / seconds;
        
        assert!(
            actual_mbps >= min_mbps,
            "Throughput {:.2} MB/s below minimum {:.2} MB/s",
            actual_mbps, min_mbps
        );
    }
}

/// Test result reporting utilities
pub mod reporting {
    use std::time::Duration;
    use crate::test_utils::PerformanceBenchmark;

    /// Generate a summary report for a test suite
    pub fn generate_test_summary(
        suite_name: &str,
        total_tests: usize,
        passed_tests: usize,
        duration: Duration,
        benchmark: Option<&PerformanceBenchmark>,
    ) -> String {
        let mut report = format!(
            "\n🧪 {} Test Summary\n\
             ═══════════════════════════════════════\n\
             Total Tests:  {}\n\
             Passed:       {} ({:.1}%)\n\
             Failed:       {}\n\
             Duration:     {:?}\n",
            suite_name,
            total_tests,
            passed_tests,
            (passed_tests as f32 / total_tests as f32) * 100.0,
            total_tests - passed_tests,
            duration
        );

        if let Some(bench) = benchmark {
            report.push_str(&format!(
                "\n📊 Performance Metrics\n\
                 ────────────────────────\n\
                 {}\n",
                bench.report()
            ));
        }

        report.push_str("═══════════════════════════════════════\n");
        report
    }

    /// Print a test status with emoji
    pub fn print_test_status(test_name: &str, passed: bool, duration: Option<Duration>) {
        let status_icon = if passed { "✅" } else { "❌" };
        let duration_str = duration
            .map(|d| format!(" ({:?})", d))
            .unwrap_or_default();
        
        println!("{} {}{}", status_icon, test_name, duration_str);
    }
}

#[cfg(test)]
mod test_framework_tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = TestConfig::default();
        assert!(config.timeout_seconds > 0);
        assert!(config.temp_dir.exists() || config.temp_dir == std::env::temp_dir());
    }

    #[test]
    fn test_temp_dir_creation() {
        let config = TestConfig::default();
        let test_dir = config.create_test_dir("framework_test").unwrap();
        
        assert!(test_dir.exists());
        assert!(test_dir.to_string_lossy().contains("carbide_test_framework_test"));
        
        // Cleanup
        std::fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn test_assertions() {
        use crate::assertions::*;
        
        // Test duration assertion
        assert_duration_within(
            Duration::from_millis(105),
            Duration::from_millis(100),
            0.1 // 10% tolerance
        );

        // Test success rate assertion
        assert_success_rate(8, 10, 0.7); // 80% > 70%

        // This should panic
        let result = std::panic::catch_unwind(|| {
            assert_success_rate(5, 10, 0.8); // 50% < 80%
        });
        assert!(result.is_err());
    }
}