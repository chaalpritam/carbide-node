//! # Carbide Network Interactive Demo
//!
//! This demo showcases the complete decentralized storage marketplace with:
//! - Multiple storage providers with different tiers and capabilities
//! - Client applications storing and retrieving files
//! - Provider discovery and marketplace functionality
//! - Reputation tracking and provider ranking
//! - Real-time network simulation
//!
//! ## Demo Scenarios
//!
//! 1. **Provider Network Setup**: Launch multiple providers with different characteristics
//! 2. **File Storage Operations**: Demonstrate file storage with replication
//! 3. **Provider Discovery**: Show how clients find suitable providers
//! 4. **Reputation System**: Track provider performance and build reputation
//! 5. **Market Dynamics**: Simulate real-world usage patterns
//! 6. **Error Scenarios**: Handle provider failures and recovery

pub mod interactive_demo;
pub mod network_simulation;
pub mod scenario_runner;
pub mod demo_ui;

use std::time::Duration;
use uuid::Uuid;

use carbide_core::*;
use carbide_client::{CarbideClient, StorageManager, StoragePreferences};
use carbide_reputation::{ReputationSystemBuilder, MemoryStorage, events::*};

/// Demo configuration for the entire network simulation
#[derive(Debug, Clone)]
pub struct DemoConfig {
    /// Number of providers to simulate
    pub provider_count: usize,
    /// Number of clients to simulate
    pub client_count: usize,
    /// Duration to run the demo
    pub demo_duration: Duration,
    /// Base directory for demo data
    pub data_dir: std::path::PathBuf,
    /// Whether to show verbose output
    pub verbose: bool,
    /// Network simulation parameters
    pub network_config: NetworkConfig,
}

/// Network simulation configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Network latency range (min, max) in milliseconds
    pub latency_range: (u64, u64),
    /// Packet loss probability (0.0 - 1.0)
    pub packet_loss: f32,
    /// Provider failure probability per operation
    pub failure_rate: f32,
    /// Bandwidth limit in MB/s (None = unlimited)
    pub bandwidth_limit: Option<f64>,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            provider_count: 5,
            client_count: 3,
            demo_duration: Duration::from_secs(120), // 2 minutes
            data_dir: std::env::temp_dir().join("carbide_demo"),
            verbose: true,
            network_config: NetworkConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            latency_range: (50, 200), // 50-200ms latency
            packet_loss: 0.01,        // 1% packet loss
            failure_rate: 0.05,       // 5% operation failure rate
            bandwidth_limit: Some(10.0), // 10 MB/s limit
        }
    }
}

/// Demo result summary
#[derive(Debug, Clone)]
pub struct DemoResults {
    /// Total operations performed
    pub total_operations: usize,
    /// Successful operations
    pub successful_operations: usize,
    /// Total data transferred
    pub bytes_transferred: u64,
    /// Demo duration
    pub duration: Duration,
    /// Provider statistics
    pub provider_stats: Vec<ProviderStats>,
    /// Reputation rankings
    pub reputation_rankings: Vec<(ProviderId, ReputationScore)>,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
}

/// Individual provider statistics
#[derive(Debug, Clone)]
pub struct ProviderStats {
    /// Provider ID
    pub provider_id: ProviderId,
    /// Provider name
    pub name: String,
    /// Total operations handled
    pub operations_handled: usize,
    /// Success rate
    pub success_rate: f64,
    /// Average response time
    pub avg_response_time: Duration,
    /// Total data stored
    pub data_stored: u64,
    /// Revenue earned
    pub revenue_earned: rust_decimal::Decimal,
    /// Final reputation score
    pub reputation: ReputationScore,
}

/// Performance metrics for the demo
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average throughput in MB/s
    pub avg_throughput: f64,
    /// Average operation latency
    pub avg_latency: Duration,
    /// Peak operations per second
    pub peak_ops_per_second: f64,
    /// Network efficiency (successful operations / total attempts)
    pub network_efficiency: f64,
}

impl DemoResults {
    /// Generate a comprehensive demo report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("🎯 Carbide Network Demo Results\n");
        report.push_str("═══════════════════════════════════════\n\n");
        
        // Overall statistics
        report.push_str("📊 Overall Statistics\n");
        report.push_str("────────────────────────\n");
        report.push_str(&format!("Duration: {:?}\n", self.duration));
        report.push_str(&format!("Total Operations: {}\n", self.total_operations));
        report.push_str(&format!("Successful Operations: {} ({:.1}%)\n", 
            self.successful_operations, 
            (self.successful_operations as f64 / self.total_operations as f64) * 100.0));
        report.push_str(&format!("Data Transferred: {:.2} MB\n", 
            self.bytes_transferred as f64 / (1024.0 * 1024.0)));
        report.push_str(&format!("Network Efficiency: {:.1}%\n\n", 
            self.performance_metrics.network_efficiency * 100.0));
        
        // Performance metrics
        report.push_str("⚡ Performance Metrics\n");
        report.push_str("────────────────────────\n");
        report.push_str(&format!("Average Throughput: {:.2} MB/s\n", 
            self.performance_metrics.avg_throughput));
        report.push_str(&format!("Average Latency: {:?}\n", 
            self.performance_metrics.avg_latency));
        report.push_str(&format!("Peak Ops/Second: {:.1}\n\n", 
            self.performance_metrics.peak_ops_per_second));
        
        // Provider rankings
        report.push_str("🏆 Provider Rankings (by Reputation)\n");
        report.push_str("──────────────────────────────────────\n");
        for (i, (provider_id, reputation)) in self.reputation_rankings.iter().enumerate() {
            if let Some(stats) = self.provider_stats.iter().find(|s| s.provider_id == *provider_id) {
                report.push_str(&format!("{}. {} (ID: {})\n", 
                    i + 1, stats.name, provider_id));
                report.push_str(&format!("   Reputation: {:.3}\n", reputation.overall));
                report.push_str(&format!("   Success Rate: {:.1}%\n", stats.success_rate * 100.0));
                report.push_str(&format!("   Avg Response: {:?}\n", stats.avg_response_time));
                report.push_str(&format!("   Data Stored: {:.2} MB\n", 
                    stats.data_stored as f64 / (1024.0 * 1024.0)));
                report.push_str(&format!("   Revenue: ${:.4}\n\n", stats.revenue_earned));
            }
        }
        
        // Detailed provider statistics
        report.push_str("📈 Detailed Provider Statistics\n");
        report.push_str("──────────────────────────────────────\n");
        for stats in &self.provider_stats {
            report.push_str(&format!("Provider: {}\n", stats.name));
            report.push_str(&format!("  Operations: {}\n", stats.operations_handled));
            report.push_str(&format!("  Success Rate: {:.1}%\n", stats.success_rate * 100.0));
            report.push_str(&format!("  Avg Response Time: {:?}\n", stats.avg_response_time));
            report.push_str(&format!("  Data Stored: {:.2} MB\n", 
                stats.data_stored as f64 / (1024.0 * 1024.0)));
            report.push_str(&format!("  Revenue: ${:.4}\n", stats.revenue_earned));
            report.push_str(&format!("  Reputation Components:\n"));
            report.push_str(&format!("    Overall: {:.3}\n", stats.reputation.overall));
            report.push_str(&format!("    Uptime: {:.3}\n", stats.reputation.uptime));
            report.push_str(&format!("    Data Integrity: {:.3}\n", stats.reputation.data_integrity));
            report.push_str(&format!("    Response Time: {:.3}\n", stats.reputation.response_time));
            report.push_str(&format!("    Contract Compliance: {:.3}\n", stats.reputation.contract_compliance));
            report.push_str("\n");
        }
        
        report.push_str("═══════════════════════════════════════\n");
        
        report
    }
}

/// Demo runner that coordinates all components
pub struct DemoRunner {
    config: DemoConfig,
    providers: Vec<MockProvider>,
    clients: Vec<CarbideClient>,
    reputation_system: carbide_reputation::ReputationTracker,
    start_time: Option<std::time::Instant>,
}

/// Mock provider for demo purposes
#[derive(Debug, Clone)]
pub struct MockProvider {
    pub id: ProviderId,
    pub info: Provider,
    pub operations_count: std::sync::Arc<std::sync::Mutex<usize>>,
    pub success_count: std::sync::Arc<std::sync::Mutex<usize>>,
    pub data_stored: std::sync::Arc<std::sync::Mutex<u64>>,
    pub revenue: std::sync::Arc<std::sync::Mutex<rust_decimal::Decimal>>,
    pub response_times: std::sync::Arc<std::sync::Mutex<Vec<Duration>>>,
    pub failure_rate: f32,
    pub latency_range: (u64, u64),
}

impl MockProvider {
    pub fn new(
        name: String,
        tier: ProviderTier,
        region: Region,
        capacity_gb: u64,
        price_per_gb: rust_decimal::Decimal,
        failure_rate: f32,
        latency_range: (u64, u64),
    ) -> Self {
        let provider_info = Provider::new(
            name,
            tier,
            region,
            format!("http://provider-{}.demo.carbide", Uuid::new_v4()),
            capacity_gb * 1024 * 1024 * 1024,
            price_per_gb,
        );

        Self {
            id: provider_info.id,
            info: provider_info,
            operations_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            success_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            data_stored: std::sync::Arc::new(std::sync::Mutex::new(0)),
            revenue: std::sync::Arc::new(std::sync::Mutex::new(rust_decimal::Decimal::ZERO)),
            response_times: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            failure_rate,
            latency_range,
        }
    }

    /// Simulate storing a file on this provider
    pub async fn store_file(&self, file_size: u64, price_per_gb: rust_decimal::Decimal) -> Result<Duration> {
        use rand::Rng;
        
        *self.operations_count.lock().unwrap() += 1;
        
        // Simulate network latency
        let latency = rand::thread_rng().gen_range(self.latency_range.0..=self.latency_range.1);
        tokio::time::sleep(std::time::Duration::from_millis(latency)).await;
        
        let response_time = std::time::Duration::from_millis(latency);
        
        // Simulate random failures
        if rand::thread_rng().gen::<f32>() < self.failure_rate {
            return Err(CarbideError::Provider("Simulated provider failure".to_string()));
        }
        
        // Success - update statistics
        *self.success_count.lock().unwrap() += 1;
        *self.data_stored.lock().unwrap() += file_size;
        
        // Calculate revenue
        let size_gb = rust_decimal::Decimal::new(file_size as i64, 9); // Convert bytes to GB
        let revenue = size_gb * price_per_gb;
        *self.revenue.lock().unwrap() += revenue;
        
        self.response_times.lock().unwrap().push(response_time);
        
        Ok(response_time)
    }

    /// Get provider statistics
    pub fn get_stats(&self) -> ProviderStats {
        let operations = *self.operations_count.lock().unwrap();
        let successes = *self.success_count.lock().unwrap();
        let data_stored = *self.data_stored.lock().unwrap();
        let revenue = *self.revenue.lock().unwrap();
        let response_times = self.response_times.lock().unwrap();
        
        let success_rate = if operations > 0 {
            successes as f64 / operations as f64
        } else {
            0.0
        };
        
        let avg_response_time = if !response_times.is_empty() {
            let total_time: std::time::Duration = response_times.iter().sum();
            total_time / response_times.len() as u32
        } else {
            std::time::Duration::from_secs(0)
        };

        ProviderStats {
            provider_id: self.id,
            name: self.info.name.clone(),
            operations_handled: operations,
            success_rate,
            avg_response_time,
            data_stored,
            revenue_earned: revenue,
            reputation: ReputationScore::new(), // Will be updated by reputation system
        }
    }
}