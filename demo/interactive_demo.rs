//! Interactive demo runner for Carbide Network
//!
//! This module provides an interactive command-line demo that showcases
//! all aspects of the decentralized storage marketplace.

use crate::demo::{DemoConfig, DemoRunner, DemoResults, MockProvider, ProviderStats};
use carbide_core::*;
use carbide_reputation::{ReputationSystemBuilder, MemoryStorage, events::*};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

impl DemoRunner {
    /// Create a new demo runner with the specified configuration
    pub async fn new(config: DemoConfig) -> Result<Self> {
        // Create data directory
        std::fs::create_dir_all(&config.data_dir)?;
        
        // Initialize reputation system
        let storage = Box::new(MemoryStorage::new());
        let reputation_system = ReputationSystemBuilder::new()
            .with_storage(storage)
            .with_min_events(3) // Lower threshold for demo
            .build()?;
        
        // Create mock providers with different characteristics
        let providers = Self::create_demo_providers(&config);
        
        // Create mock clients
        let mut clients = Vec::new();
        for i in 0..config.client_count {
            let client = CarbideClient::new(format!("http://client-{}.demo.carbide", i))?;
            clients.push(client);
        }
        
        Ok(Self {
            config,
            providers,
            clients,
            reputation_system,
            start_time: None,
        })
    }
    
    /// Create a diverse set of demo providers
    fn create_demo_providers(config: &DemoConfig) -> Vec<MockProvider> {
        let mut providers = Vec::new();
        
        // Create providers with different tiers and characteristics
        let provider_configs = vec![
            ("Alice's Home Storage", ProviderTier::Home, Region::NorthAmerica, 2, rust_decimal::Decimal::new(2, 3), 0.15, (100, 300)),
            ("Bob's Business Cloud", ProviderTier::Professional, Region::Europe, 20, rust_decimal::Decimal::new(4, 3), 0.08, (50, 150)),
            ("DataCenter Pro", ProviderTier::Enterprise, Region::Asia, 100, rust_decimal::Decimal::new(7, 3), 0.03, (20, 80)),
            ("Global CDN Corp", ProviderTier::GlobalCDN, Region::NorthAmerica, 500, rust_decimal::Decimal::new(12, 3), 0.01, (10, 50)),
            ("Charlie's Storage", ProviderTier::Home, Region::Europe, 5, rust_decimal::Decimal::new(3, 3), 0.12, (80, 250)),
        ];
        
        for (name, tier, region, capacity_gb, price, failure_rate, latency_range) in provider_configs.into_iter().take(config.provider_count) {
            let provider = MockProvider::new(
                name.to_string(),
                tier,
                region,
                capacity_gb,
                price,
                failure_rate * (1.0 + config.network_config.failure_rate), // Add network failures
                latency_range,
            );
            providers.push(provider);
        }
        
        providers
    }
    
    /// Run the complete demo
    pub async fn run_demo(&mut self) -> Result<DemoResults> {
        println!("🚀 Starting Carbide Network Demo");
        println!("═══════════════════════════════════════");
        
        self.start_time = Some(Instant::now());
        
        // Phase 1: Provider Introduction
        self.introduce_providers().await?;
        
        // Phase 2: Network Activity Simulation
        let activity_results = self.simulate_network_activity().await?;
        
        // Phase 3: Reputation Building
        self.build_provider_reputations().await?;
        
        // Phase 4: Market Dynamics
        self.demonstrate_market_dynamics().await?;
        
        // Phase 5: Error Handling
        self.demonstrate_error_handling().await?;
        
        // Generate final results
        let results = self.generate_results().await?;
        
        println!("\n🏁 Demo completed successfully!");
        println!("{}", results.generate_report());
        
        Ok(results)
    }
    
    /// Phase 1: Introduce all providers
    async fn introduce_providers(&mut self) -> Result<()> {
        println!("\n🏪 Phase 1: Provider Network Introduction");
        println!("──────────────────────────────────────");
        
        for provider in &self.providers {
            println!("Provider: {}", provider.info.name);
            println!("  Tier: {:?}", provider.info.tier);
            println!("  Region: {:?}", provider.info.region);
            println!("  Capacity: {:.1} GB", provider.info.total_capacity as f64 / (1024.0 * 1024.0 * 1024.0));
            println!("  Price: ${:.3}/GB/month", provider.info.price_per_gb_month);
            println!("  Expected failure rate: {:.1}%", provider.failure_rate * 100.0);
            println!();
            
            // Generate initial reputation event (provider coming online)
            let event = EventBuilder::new(provider.id, EventType::Online)
                .severity(EventSeverity::Positive)
                .build();
            
            self.reputation_system.process_events_batch(vec![event]).await?;
        }
        
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }
    
    /// Phase 2: Simulate network activity
    async fn simulate_network_activity(&mut self) -> Result<usize> {
        println!("📊 Phase 2: Network Activity Simulation");
        println!("──────────────────────────────────────");
        
        let mut total_operations = 0;
        let simulation_duration = Duration::from_secs(30); // 30 seconds of activity
        let start_time = Instant::now();
        
        while start_time.elapsed() < simulation_duration {
            // Simulate file storage operations
            let file_sizes = vec![1024, 10_000, 100_000, 1_000_000, 5_000_000]; // Various file sizes
            let file_size = file_sizes[total_operations % file_sizes.len()];
            
            // Select a random provider (simulating provider discovery)
            let provider_idx = total_operations % self.providers.len();
            let provider = &self.providers[provider_idx];
            
            println!("Operation {}: Storing {} bytes with {}", 
                total_operations + 1, file_size, provider.info.name);
            
            // Attempt to store file
            let result = provider.store_file(file_size as u64, provider.info.price_per_gb_month).await;
            
            // Generate reputation events based on result
            let event = match result {
                Ok(response_time) => {
                    println!("  ✅ Success in {:?}", response_time);
                    
                    // Generate multiple positive events
                    let mut events = vec![
                        EventBuilder::new(provider.id, EventType::UploadSuccess {
                            file_size: file_size as u64,
                            upload_time_ms: response_time.as_millis() as u64,
                        })
                        .severity(EventSeverity::Positive)
                        .build(),
                    ];
                    
                    // Good response time gets additional positive event
                    if response_time < Duration::from_millis(100) {
                        events.push(
                            EventBuilder::new(provider.id, EventType::PerformanceUpdate {
                                cpu_usage: 45.0,
                                memory_usage: 60.0,
                                disk_usage: 30.0,
                                latency_ms: response_time.as_millis() as f32,
                            })
                            .severity(EventSeverity::Positive)
                            .build()
                        );
                    }
                    
                    self.reputation_system.process_events_batch(events).await?;
                    None
                }
                Err(e) => {
                    println!("  ❌ Failed: {}", e);
                    
                    let event = EventBuilder::new(provider.id, EventType::UploadFailure {
                        reason: e.to_string(),
                        partial_bytes: Some((file_size / 2) as u64),
                    })
                    .severity(EventSeverity::Negative)
                    .build();
                    
                    self.reputation_system.process_events_batch(vec![event]).await?;
                    None
                }
            };
            
            total_operations += 1;
            
            // Brief pause between operations
            sleep(Duration::from_millis(200)).await;
        }
        
        println!("Completed {} operations in network activity phase", total_operations);
        Ok(total_operations)
    }
    
    /// Phase 3: Build provider reputations with various scenarios
    async fn build_provider_reputations(&mut self) -> Result<()> {
        println!("\n🏆 Phase 3: Building Provider Reputations");
        println!("────────────────────────────────────────");
        
        // Simulate various reputation-affecting events
        for provider in &self.providers {
            println!("Generating reputation events for {}", provider.info.name);
            
            // Generate events based on provider tier (better tiers get better events)
            let event_scenarios = match provider.info.tier {
                ProviderTier::GlobalCDN => {
                    vec![
                        EventType::ProofSuccess { response_time_ms: 25, chunks_proven: 10 },
                        EventType::ContractCompleted { 
                            final_value: rust_decimal::Decimal::new(150, 2), 
                            duration_served_days: 30 
                        },
                        EventType::HealthCheck { response_time_ms: 15, status: "excellent".to_string() },
                        EventType::CommunityFeedback { 
                            rating: 5, 
                            category: crate::events::FeedbackCategory::Performance,
                            comment: Some("Extremely fast and reliable!".to_string())
                        },
                    ]
                },
                ProviderTier::Enterprise => {
                    vec![
                        EventType::ProofSuccess { response_time_ms: 50, chunks_proven: 8 },
                        EventType::ContractCompleted { 
                            final_value: rust_decimal::Decimal::new(100, 2), 
                            duration_served_days: 30 
                        },
                        EventType::HealthCheck { response_time_ms: 40, status: "good".to_string() },
                    ]
                },
                ProviderTier::Professional => {
                    vec![
                        EventType::ProofSuccess { response_time_ms: 120, chunks_proven: 5 },
                        EventType::HealthCheck { response_time_ms: 80, status: "ok".to_string() },
                        EventType::ProofFailure { 
                            reason: "Temporary network issue".to_string(), 
                            error_details: None 
                        },
                    ]
                },
                ProviderTier::Home => {
                    vec![
                        EventType::ProofSuccess { response_time_ms: 200, chunks_proven: 3 },
                        EventType::Offline, // Home providers go offline occasionally
                        EventType::Online,
                        EventType::MaintenanceWindow { 
                            duration_minutes: 30, 
                            announced: true 
                        },
                    ]
                },
            };
            
            let mut events = Vec::new();
            for event_type in event_scenarios {
                let severity = match &event_type {
                    EventType::ProofSuccess { .. } | EventType::ContractCompleted { .. } | 
                    EventType::HealthCheck { .. } | EventType::Online => EventSeverity::Positive,
                    EventType::CommunityFeedback { rating, .. } if *rating >= 4 => EventSeverity::ExtremelyPositive,
                    EventType::CommunityFeedback { .. } => EventSeverity::Positive,
                    EventType::ProofFailure { .. } | EventType::Offline => EventSeverity::Negative,
                    EventType::MaintenanceWindow { announced: true, .. } => EventSeverity::Neutral,
                    _ => EventSeverity::Neutral,
                };
                
                events.push(EventBuilder::new(provider.id, event_type).severity(severity).build());
            }
            
            self.reputation_system.process_events_batch(events).await?;
        }
        
        // Show current reputation rankings
        let top_providers = self.reputation_system.get_top_providers(self.providers.len())?;
        println!("\nCurrent Reputation Rankings:");
        for (i, (provider_id, reputation)) in top_providers.iter().enumerate() {
            if let Some(provider) = self.providers.iter().find(|p| p.id == *provider_id) {
                println!("  {}. {} - Reputation: {:.3}", 
                    i + 1, provider.info.name, reputation.overall);
            }
        }
        
        Ok(())
    }
    
    /// Phase 4: Demonstrate market dynamics
    async fn demonstrate_market_dynamics(&mut self) -> Result<()> {
        println!("\n💰 Phase 4: Market Dynamics Demonstration");
        println!("─────────────────────────────────────────");
        
        // Simulate price competition and provider selection
        println!("Simulating client provider selection based on price and reputation...");
        
        for client_idx in 0..self.clients.len() {
            println!("\nClient {} needs to store a 10MB file for 6 months", client_idx + 1);
            
            // Get provider options sorted by value (price vs reputation)
            let mut provider_options = Vec::new();
            for provider in &self.providers {
                if let Ok(Some(reputation)) = self.reputation_system.get_reputation(&provider.id) {
                    let monthly_cost = provider.info.calculate_monthly_cost(
                        rust_decimal::Decimal::new(10, 0) // 10 MB
                    );
                    let value_score = reputation.overall / monthly_cost; // Reputation per dollar
                    
                    provider_options.push((provider, reputation, monthly_cost, value_score));
                }
            }
            
            // Sort by value score (descending)
            provider_options.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());
            
            println!("Provider options (sorted by value):");
            for (i, (provider, reputation, cost, value)) in provider_options.iter().enumerate() {
                println!("  {}. {} - Reputation: {:.3}, Cost: ${:.4}/month, Value: {:.1}", 
                    i + 1, provider.info.name, reputation.overall, cost, value);
            }
            
            // Client selects the best value provider
            if let Some((selected_provider, _, _, _)) = provider_options.first() {
                println!("  → Client selected: {}", selected_provider.info.name);
                
                // Simulate the storage operation
                let storage_result = selected_provider.store_file(10_485_760, selected_provider.info.price_per_gb_month).await;
                match storage_result {
                    Ok(_) => {
                        println!("    ✅ File stored successfully");
                        
                        // Generate contract completion event
                        let event = EventBuilder::new(selected_provider.id, EventType::ContractCompleted {
                            final_value: selected_provider.info.price_per_gb_month * rust_decimal::Decimal::new(10, 0),
                            duration_served_days: 180, // 6 months
                        })
                        .severity(EventSeverity::ExtremelyPositive)
                        .build();
                        
                        self.reputation_system.process_events_batch(vec![event]).await?;
                    }
                    Err(e) => {
                        println!("    ❌ Storage failed: {}", e);
                        
                        let event = EventBuilder::new(selected_provider.id, EventType::ContractViolated {
                            reason: e.to_string(),
                            penalty: Some(rust_decimal::Decimal::new(10, 2)), // $0.10 penalty
                        })
                        .severity(EventSeverity::ExtremelyNegative)
                        .build();
                        
                        self.reputation_system.process_events_batch(vec![event]).await?;
                    }
                }
            }
            
            sleep(Duration::from_millis(1000)).await;
        }
        
        Ok(())
    }
    
    /// Phase 5: Demonstrate error handling and recovery
    async fn demonstrate_error_handling(&mut self) -> Result<()> {
        println!("\n⚠️  Phase 5: Error Handling and Recovery");
        println!("──────────────────────────────────────────");
        
        // Simulate various error scenarios
        println!("Simulating provider failures and recovery scenarios...");
        
        // Scenario 1: Data corruption detection and recovery
        if let Some(provider) = self.providers.first() {
            println!("\nScenario 1: Data corruption detected at {}", provider.info.name);
            
            let corruption_event = EventBuilder::new(provider.id, EventType::DataCorruption {
                corrupted_files: 2,
                corrupted_bytes: 1_048_576, // 1MB
                recovered: true,
            })
            .severity(EventSeverity::Negative)
            .build();
            
            self.reputation_system.process_events_batch(vec![corruption_event]).await?;
            
            println!("  ✅ Data corruption detected and recovered");
        }
        
        // Scenario 2: Provider going offline unexpectedly
        if let Some(provider) = self.providers.get(1) {
            println!("\nScenario 2: {} goes offline unexpectedly", provider.info.name);
            
            let offline_event = EventBuilder::new(provider.id, EventType::Offline)
                .severity(EventSeverity::Negative)
                .build();
            
            self.reputation_system.process_events_batch(vec![offline_event]).await?;
            
            sleep(Duration::from_millis(2000)).await; // Simulate downtime
            
            let online_event = EventBuilder::new(provider.id, EventType::Online)
                .severity(EventSeverity::Positive)
                .build();
            
            self.reputation_system.process_events_batch(vec![online_event]).await?;
            
            println!("  ✅ Provider recovered and is back online");
        }
        
        // Scenario 3: Suspicious activity detection
        if let Some(provider) = self.providers.get(2) {
            println!("\nScenario 3: Suspicious activity detected at {}", provider.info.name);
            
            let suspicious_event = EventBuilder::new(provider.id, EventType::SuspiciousActivity {
                activity_type: "Unusual storage patterns".to_string(),
                confidence: 0.85,
            })
            .severity(EventSeverity::ExtremelyNegative)
            .build();
            
            self.reputation_system.process_events_batch(vec![suspicious_event]).await?;
            
            println!("  ⚠️  Provider flagged for review");
        }
        
        // Show final reputation rankings after error scenarios
        let final_rankings = self.reputation_system.get_top_providers(self.providers.len())?;
        println!("\nFinal Reputation Rankings (after error scenarios):");
        for (i, (provider_id, reputation)) in final_rankings.iter().enumerate() {
            if let Some(provider) = self.providers.iter().find(|p| p.id == *provider_id) {
                println!("  {}. {} - Reputation: {:.3}", 
                    i + 1, provider.info.name, reputation.overall);
            }
        }
        
        Ok(())
    }
    
    /// Generate comprehensive demo results
    async fn generate_results(&mut self) -> Result<DemoResults> {
        let duration = self.start_time.unwrap().elapsed();
        
        // Collect provider statistics
        let mut provider_stats = Vec::new();
        let mut total_operations = 0;
        let mut successful_operations = 0;
        let mut bytes_transferred = 0;
        
        for provider in &self.providers {
            let mut stats = provider.get_stats();
            
            // Update reputation from reputation system
            if let Ok(Some(reputation)) = self.reputation_system.get_reputation(&provider.id) {
                stats.reputation = reputation;
            }
            
            total_operations += stats.operations_handled;
            successful_operations += (stats.operations_handled as f64 * stats.success_rate) as usize;
            bytes_transferred += stats.data_stored;
            
            provider_stats.push(stats);
        }
        
        // Get reputation rankings
        let reputation_rankings = self.reputation_system.get_top_providers(self.providers.len())?;
        
        // Calculate performance metrics
        let performance_metrics = crate::demo::PerformanceMetrics {
            avg_throughput: if duration.as_secs() > 0 {
                (bytes_transferred as f64 / (1024.0 * 1024.0)) / duration.as_secs_f64()
            } else {
                0.0
            },
            avg_latency: Duration::from_millis(100), // Average simulated latency
            peak_ops_per_second: if duration.as_secs() > 0 {
                total_operations as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
            network_efficiency: if total_operations > 0 {
                successful_operations as f64 / total_operations as f64
            } else {
                0.0
            },
        };
        
        Ok(DemoResults {
            total_operations,
            successful_operations,
            bytes_transferred,
            duration,
            provider_stats,
            reputation_rankings,
            performance_metrics,
        })
    }
}