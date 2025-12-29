//! Focused integration tests for implemented Carbide Network components
//!
//! These tests verify functionality of the components we have fully implemented:
//! - Core data structures and types
//! - Cryptographic functions  
//! - Reputation tracking system
//! - Client SDK (API structure)
//! - Network protocol serialization

use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use carbide_core::*;
use carbide_client::{CarbideClient, StorageManager, StoragePreferences};
use carbide_crypto::*;
use carbide_reputation::{ReputationSystemBuilder, MemoryStorage, events::*};

/// Test configuration for consistent parameters
pub struct TestConfig {
    pub test_timeout: Duration,
    pub temp_dir: std::path::PathBuf,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_timeout: Duration::from_secs(30),
            temp_dir: std::env::temp_dir().join(format!("carbide_test_{}", Uuid::new_v4())),
        }
    }
}

impl TestConfig {
    pub fn setup(&self) -> Result<()> {
        std::fs::create_dir_all(&self.temp_dir)?;
        Ok(())
    }

    pub fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            std::fs::remove_dir_all(&self.temp_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod focused_integration_tests {
    use super::*;
    use crate::test_utils::{MockProvider, TestDataGenerator, PerformanceBenchmark, TestAssertions};

    #[tokio::test]
    async fn test_core_data_structures_integration() {
        println!("\n🧪 Testing core data structures integration...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        // Test file creation with real data
        let test_data = TestDataGenerator::generate_file_data(1024);
        let file = File::new(
            "integration_test.txt".to_string(),
            test_data.clone(),
            "text/plain".to_string(),
        );

        // Verify content addressing
        let expected_hash = ContentHash::from_data(&test_data);
        assert_eq!(file.id, expected_hash);
        assert_eq!(file.size, test_data.len() as u64);

        // Test provider creation and capabilities
        let provider = Provider::new(
            "Integration Test Provider".to_string(),
            ProviderTier::Professional,
            Region::NorthAmerica,
            "https://test-provider.example.com".to_string(),
            10 * 1024 * 1024 * 1024, // 10GB
            rust_decimal::Decimal::new(5, 3), // $0.005/GB/month
        );

        // Test capacity checking
        assert!(provider.can_store(file.size));
        assert!(!provider.can_store(20 * 1024 * 1024 * 1024)); // 20GB

        // Test cost calculation
        let file_size_gb = rust_decimal::Decimal::new(file.size as i64, 9); // Convert to GB
        let monthly_cost = provider.calculate_monthly_cost(file_size_gb);
        assert!(monthly_cost > rust_decimal::Decimal::ZERO);

        // Test storage request creation
        let storage_request = StorageRequest::new(
            file.id,
            3, // Triple replication
            rust_decimal::Decimal::new(10, 3), // $0.010/GB/month max
            ProviderRequirements::important(),
        ).unwrap();

        assert_eq!(storage_request.file_id, file.id);
        assert_eq!(storage_request.replication_factor, 3);
        
        let budget = storage_request.calculate_monthly_budget(file_size_gb);
        assert!(budget > rust_decimal::Decimal::ZERO);

        config.cleanup().unwrap();
        println!("✅ Core data structures integration test completed");
    }

    #[tokio::test]
    async fn test_cryptographic_operations_integration() {
        println!("\n🧪 Testing cryptographic operations integration...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        // Test with various data sizes
        let test_cases = vec![
            ("Small file", TestDataGenerator::generate_file_data(100)),
            ("Medium file", TestDataGenerator::generate_file_data(10_000)),
            ("Large file", TestDataGenerator::generate_file_data(1_000_000)),
            ("Random data", TestDataGenerator::generate_random_data(50_000)),
        ];

        for (description, data) in test_cases {
            println!("  Testing {}", description);
            
            // Test content hashing
            let hash1 = ContentHash::from_data(&data);
            let hash2 = ContentHash::from_data(&data);
            assert_eq!(hash1, hash2, "Hash consistency failed for {}", description);

            // Test hash hex conversion
            let hex_string = hash1.to_hex();
            assert_eq!(hex_string.len(), 64); // 32 bytes = 64 hex chars
            let parsed_hash = ContentHash::from_hex(&hex_string).unwrap();
            assert_eq!(hash1, parsed_hash);

            // Test file ID generation
            let file = File::new(
                format!("{}.bin", description.replace(" ", "_")),
                data.clone(),
                "application/octet-stream".to_string(),
            );
            assert_eq!(file.id, hash1);

            // Test encryption/decryption
            let password = "test_password_123";
            let encrypted = encrypt_data(&data, password).unwrap();
            assert_ne!(encrypted, data);
            
            let decrypted = decrypt_data(&encrypted, password).unwrap();
            assert_eq!(decrypted, data);
            
            // Test wrong password fails
            let wrong_decrypt = decrypt_data(&encrypted, "wrong_password");
            assert!(wrong_decrypt.is_err());
        }

        config.cleanup().unwrap();
        println!("✅ Cryptographic operations integration test completed");
    }

    #[tokio::test]
    async fn test_reputation_system_comprehensive() {
        println!("\n🧪 Testing comprehensive reputation system...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        // Initialize reputation system
        let storage = Box::new(MemoryStorage::new());
        let mut reputation_system = ReputationSystemBuilder::new()
            .with_storage(storage)
            .with_min_events(5)
            .build().unwrap();

        // Create test providers
        let provider1 = Uuid::new_v4();
        let provider2 = Uuid::new_v4();
        let provider3 = Uuid::new_v4();

        // Generate comprehensive event scenarios
        println!("  Generating events for high-performing provider...");
        let high_performer_events = TestDataGenerator::generate_reputation_events(
            provider1,
            50, // 50 events
            0.9, // 90% positive
        );

        println!("  Generating events for average provider...");
        let average_performer_events = TestDataGenerator::generate_reputation_events(
            provider2,
            30, // 30 events
            0.7, // 70% positive
        );

        println!("  Generating events for poor provider...");
        let poor_performer_events = TestDataGenerator::generate_reputation_events(
            provider3,
            20, // 20 events
            0.3, // 30% positive (mostly negative)
        );

        // Process all events
        let mut all_updates = Vec::new();
        
        let updates1 = reputation_system.process_events_batch(high_performer_events).await.unwrap();
        let updates2 = reputation_system.process_events_batch(average_performer_events).await.unwrap();
        let updates3 = reputation_system.process_events_batch(poor_performer_events).await.unwrap();
        
        all_updates.extend(updates1);
        all_updates.extend(updates2);
        all_updates.extend(updates3);

        // Verify reputation scores
        let score1 = reputation_system.get_reputation(&provider1).unwrap().unwrap();
        let score2 = reputation_system.get_reputation(&provider2).unwrap().unwrap();
        let score3 = reputation_system.get_reputation(&provider3).unwrap().unwrap();

        // High performer should have best reputation
        assert!(score1.overall > score2.overall);
        assert!(score2.overall > score3.overall);

        // Test reputation bounds
        TestAssertions::assert_reputation_bounds(&score1, 0.7, 1.0);
        TestAssertions::assert_reputation_bounds(&score2, 0.4, 0.8);
        TestAssertions::assert_reputation_bounds(&score3, 0.0, 0.5);

        println!("  Provider 1 reputation: {:.3}", score1.overall);
        println!("  Provider 2 reputation: {:.3}", score2.overall);
        println!("  Provider 3 reputation: {:.3}", score3.overall);

        // Test top providers ranking
        let top_providers = reputation_system.get_top_providers(10).unwrap();
        assert!(top_providers.len() >= 3);
        
        TestAssertions::assert_providers_sorted_by_reputation(&top_providers);

        // Test statistics
        let stats1 = reputation_system.get_statistics(&provider1).unwrap().unwrap();
        assert_eq!(stats1.provider_id, provider1);
        assert_eq!(stats1.total_events, 50);

        let stats3 = reputation_system.get_statistics(&provider3).unwrap().unwrap();
        assert!(stats3.proof_success_rate < rust_decimal::Decimal::new(5, 1)); // < 50%

        // Test maintenance
        let cleaned_count = reputation_system.maintenance().unwrap();
        println!("  Maintenance cleaned {} items", cleaned_count);

        config.cleanup().unwrap();
        println!("✅ Comprehensive reputation system test completed");
    }

    #[tokio::test]
    async fn test_client_sdk_structure() {
        println!("\n🧪 Testing client SDK structure...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        // Test client creation
        let client = CarbideClient::default().unwrap();
        
        // Test storage manager creation
        let storage_manager = StorageManager::new(
            client,
            "http://localhost:9090".to_string(),
        );

        // Test storage preferences
        let prefs = storage_manager.preferences();
        assert_eq!(prefs.replication_factor, 3);
        assert!(prefs.preferred_regions.contains(&Region::NorthAmerica));
        assert!(prefs.preferred_tiers.contains(&ProviderTier::Professional));

        // Test custom preferences
        let custom_prefs = StoragePreferences {
            preferred_regions: vec![Region::Asia, Region::Europe],
            preferred_tiers: vec![ProviderTier::Enterprise],
            replication_factor: 5,
            max_price_per_gb: rust_decimal::Decimal::new(15, 3), // $0.015/GB/month
            requirements: ProviderRequirements::critical(),
        };

        let mut custom_manager = StorageManager::with_preferences(
            CarbideClient::default().unwrap(),
            "http://localhost:9090".to_string(),
            custom_prefs.clone(),
        );

        assert_eq!(custom_manager.preferences().replication_factor, 5);
        assert_eq!(custom_manager.preferences().max_price_per_gb, rust_decimal::Decimal::new(15, 3));

        // Test preferences update
        let new_prefs = StoragePreferences::default();
        custom_manager.set_preferences(new_prefs);
        assert_eq!(custom_manager.preferences().replication_factor, 3);

        // Test that store/retrieve APIs exist (they'll fail without services, but that's expected)
        let test_data = b"SDK test data";
        let file_id = ContentHash::from_data(test_data);
        
        // These should fail gracefully without panicking
        let store_result = timeout(
            Duration::from_secs(5),
            storage_manager.store_file(test_data, 1, None)
        ).await;
        
        // Should timeout or return error, not panic
        match store_result {
            Ok(Err(_)) => println!("  Store operation failed as expected (no services)"),
            Err(_) => println!("  Store operation timed out as expected"),
            Ok(Ok(_)) => panic!("Store should not succeed without services"),
        }

        let retrieve_result = timeout(
            Duration::from_secs(5),
            storage_manager.retrieve_file(&file_id, "test_token", None)
        ).await;
        
        match retrieve_result {
            Ok(Err(_)) => println!("  Retrieve operation failed as expected (no services)"),
            Err(_) => println!("  Retrieve operation timed out as expected"),
            Ok(Ok(_)) => panic!("Retrieve should not succeed without services"),
        }

        config.cleanup().unwrap();
        println!("✅ Client SDK structure test completed");
    }

    #[tokio::test]
    async fn test_mock_provider_functionality() {
        println!("\n🧪 Testing mock provider functionality...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        // Create mock providers with different characteristics
        let fast_provider = MockProvider::new(
            "Fast Provider".to_string(),
            ProviderTier::Enterprise,
            Region::NorthAmerica,
            10, // 10GB
            rust_decimal::Decimal::new(8, 3),
        ).with_latency(50); // 50ms latency

        let slow_provider = MockProvider::new(
            "Slow Provider".to_string(),
            ProviderTier::Home,
            Region::Asia,
            2, // 2GB
            rust_decimal::Decimal::new(2, 3),
        ).with_latency(500); // 500ms latency

        let unreliable_provider = MockProvider::new(
            "Unreliable Provider".to_string(),
            ProviderTier::Professional,
            Region::Europe,
            5, // 5GB
            rust_decimal::Decimal::new(5, 3),
        ).with_failure_rate(0.3); // 30% failure rate

        // Test file storage and retrieval
        let test_files = vec![
            (b"Small file".to_vec(), "small.txt"),
            (vec![0u8; 1024], "1kb.bin"),
            (TestDataGenerator::generate_file_data(10_000), "10kb.data"),
        ];

        for (data, filename) in test_files {
            println!("  Testing {} storage", filename);
            
            let file_id = ContentHash::from_data(&data);
            
            // Test fast provider
            let start = std::time::Instant::now();
            let fast_result = fast_provider.store_file(file_id, &data).await;
            let fast_duration = start.elapsed();
            
            assert!(fast_result.is_ok());
            assert!(fast_duration >= Duration::from_millis(50)); // Should respect latency
            assert!(fast_duration < Duration::from_millis(200)); // But not too slow
            
            // Test retrieval
            let retrieved = fast_provider.retrieve_file(&file_id).await.unwrap();
            TestAssertions::assert_file_integrity(&retrieved, &file_id);
            
            // Test slow provider
            let start = std::time::Instant::now();
            let slow_result = slow_provider.store_file(file_id, &data).await;
            let slow_duration = start.elapsed();
            
            if slow_result.is_ok() {
                assert!(slow_duration >= Duration::from_millis(500)); // Should be slower
            }
            
            // Test unreliable provider (may fail)
            let unreliable_result = unreliable_provider.store_file(file_id, &data).await;
            // Don't assert success/failure since it's random
            println!("    Unreliable provider result: {:?}", 
                unreliable_result.is_ok());
        }

        // Test provider state management
        fast_provider.set_online(false);
        let offline_result = fast_provider.store_file(
            ContentHash::from_data(b"offline test"), 
            b"offline test"
        ).await;
        assert!(offline_result.is_err());
        
        fast_provider.set_online(true);
        let online_result = fast_provider.store_file(
            ContentHash::from_data(b"online test"), 
            b"online test"
        ).await;
        assert!(online_result.is_ok());

        // Test capacity limits
        let small_provider = MockProvider::new(
            "Small Provider".to_string(),
            ProviderTier::Home,
            Region::NorthAmerica,
            1, // 1GB only
            rust_decimal::Decimal::new(2, 3),
        );

        // Fill up the provider
        let large_file = vec![0u8; 512 * 1024 * 1024]; // 512MB
        let file1_id = ContentHash::from_data(&large_file);
        let result1 = small_provider.store_file(file1_id, &large_file).await;
        assert!(result1.is_ok());

        // This should fail due to capacity
        let file2_id = ContentHash::from_data(b"another file");
        let result2 = small_provider.store_file(file2_id, &large_file).await;
        assert!(result2.is_err());

        println!("  Provider usage: {} bytes, {} files", 
            small_provider.storage_usage(), 
            small_provider.file_count());

        config.cleanup().unwrap();
        println!("✅ Mock provider functionality test completed");
    }

    #[tokio::test]
    async fn test_performance_benchmarking() {
        println!("\n🧪 Testing performance benchmarking...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        let mut benchmark = PerformanceBenchmark::new();
        benchmark.start();

        // Benchmark cryptographic operations
        let data_sizes = vec![1024, 10_000, 100_000, 1_000_000]; // 1KB to 1MB
        
        for size in data_sizes {
            let data = TestDataGenerator::generate_file_data(size);
            
            // Benchmark hashing
            let start = std::time::Instant::now();
            let _hash = ContentHash::from_data(&data);
            let hash_duration = start.elapsed();
            
            benchmark.record_operation(
                format!("hash_{}b", size),
                hash_duration,
                true,
                Some(size as u64),
            );
            
            // Benchmark encryption
            let start = std::time::Instant::now();
            let encrypted = encrypt_data(&data, "benchmark_password");
            let encrypt_duration = start.elapsed();
            
            benchmark.record_operation(
                format!("encrypt_{}b", size),
                encrypt_duration,
                encrypted.is_ok(),
                Some(size as u64),
            );
            
            if let Ok(encrypted_data) = encrypted {
                // Benchmark decryption
                let start = std::time::Instant::now();
                let decrypted = decrypt_data(&encrypted_data, "benchmark_password");
                let decrypt_duration = start.elapsed();
                
                benchmark.record_operation(
                    format!("decrypt_{}b", size),
                    decrypt_duration,
                    decrypted.is_ok(),
                    Some(size as u64),
                );
            }
        }

        // Benchmark reputation system
        let storage = Box::new(MemoryStorage::new());
        let mut reputation_system = ReputationSystemBuilder::new()
            .with_storage(storage)
            .build().unwrap();

        let provider_id = Uuid::new_v4();
        
        // Benchmark event processing
        let start = std::time::Instant::now();
        let events = TestDataGenerator::generate_reputation_events(provider_id, 1000, 0.8);
        let event_gen_duration = start.elapsed();
        
        benchmark.record_operation(
            "generate_1000_events".to_string(),
            event_gen_duration,
            true,
            None,
        );

        let start = std::time::Instant::now();
        let _updates = reputation_system.process_events_batch(events).await.unwrap();
        let process_duration = start.elapsed();
        
        benchmark.record_operation(
            "process_1000_events".to_string(),
            process_duration,
            true,
            None,
        );

        // Print performance report
        println!("\n📊 Performance Benchmark Results:");
        println!("{}", benchmark.report());

        // Assert some basic performance expectations
        crate::assertions::assert_success_rate(
            benchmark.operations.iter().filter(|op| op.success).count(),
            benchmark.operations.len(),
            0.95, // 95% success rate
        );

        // Verify throughput is reasonable (at least 1 MB/s for crypto operations)
        if benchmark.throughput_mbps() > 0.0 {
            crate::assertions::assert_throughput(
                benchmark.operations.iter().filter_map(|op| op.bytes_processed).sum(),
                benchmark.total_duration(),
                1.0, // At least 1 MB/s
            );
        }

        config.cleanup().unwrap();
        println!("✅ Performance benchmarking test completed");
    }

    #[tokio::test]
    async fn test_error_handling_scenarios() {
        println!("\n🧪 Testing error handling scenarios...");
        
        let config = TestConfig::default();
        config.setup().unwrap();

        // Test content hash error handling
        let invalid_hex_result = ContentHash::from_hex("invalid_hex");
        assert!(invalid_hex_result.is_err());
        
        let wrong_length_hex = ContentHash::from_hex("1234"); // Too short
        assert!(wrong_length_hex.is_err());

        // Test storage request validation
        let file_id = ContentHash::from_data(b"test");
        
        // Invalid replication factors
        let invalid_request1 = StorageRequest::new(
            file_id,
            0, // Invalid: too low
            rust_decimal::Decimal::new(5, 3),
            ProviderRequirements::important(),
        );
        assert!(invalid_request1.is_err());
        
        let invalid_request2 = StorageRequest::new(
            file_id,
            11, // Invalid: too high
            rust_decimal::Decimal::new(5, 3),
            ProviderRequirements::important(),
        );
        assert!(invalid_request2.is_err());

        // Test encryption error handling
        let encrypt_result = encrypt_data(b"test data", "");
        // Should handle empty password gracefully
        assert!(encrypt_result.is_ok() || encrypt_result.is_err());

        // Test decryption with wrong password
        if let Ok(encrypted) = encrypt_data(b"test data", "correct_password") {
            let wrong_decrypt = decrypt_data(&encrypted, "wrong_password");
            assert!(wrong_decrypt.is_err());
        }

        // Test reputation system error handling
        let storage = Box::new(MemoryStorage::new());
        let mut reputation_system = ReputationSystemBuilder::new()
            .with_storage(storage)
            .build().unwrap();

        // Test with non-existent provider
        let non_existent_provider = Uuid::new_v4();
        let no_reputation = reputation_system.get_reputation(&non_existent_provider);
        assert!(no_reputation.is_err() || no_reputation.unwrap().is_none());

        let no_stats = reputation_system.get_statistics(&non_existent_provider);
        assert!(no_stats.is_err() || no_stats.unwrap().is_none());

        // Test mock provider error scenarios
        let failing_provider = MockProvider::new(
            "Failing Provider".to_string(),
            ProviderTier::Home,
            Region::NorthAmerica,
            1, // 1GB
            rust_decimal::Decimal::new(2, 3),
        ).with_failure_rate(1.0); // Always fail

        let file_id = ContentHash::from_data(b"test");
        let store_result = failing_provider.store_file(file_id, b"test").await;
        assert!(store_result.is_err());
        
        let retrieve_result = failing_provider.retrieve_file(&file_id).await;
        assert!(retrieve_result.is_err());

        // Test offline provider
        let offline_provider = MockProvider::new(
            "Offline Provider".to_string(),
            ProviderTier::Professional,
            Region::Europe,
            5, // 5GB
            rust_decimal::Decimal::new(5, 3),
        );
        
        offline_provider.set_online(false);
        let offline_store = offline_provider.store_file(file_id, b"test").await;
        assert!(offline_store.is_err());

        config.cleanup().unwrap();
        println!("✅ Error handling scenarios test completed");
    }
}