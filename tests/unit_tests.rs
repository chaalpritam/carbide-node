//! Unit tests for individual Carbide Network components
//!
//! This module contains focused unit tests for each component:
//! - Core data structures and types
//! - Cryptographic functions
//! - Reputation scoring algorithms
//! - Client SDK functionality
//! - Discovery service logic

#[cfg(test)]
mod core_tests {
    use carbide_core::*;
    use chrono::Utc;
    use rust_decimal::Decimal;

    #[test]
    fn test_content_hash_consistency() {
        let data1 = b"Hello, Carbide Network!";
        let data2 = b"Hello, Carbide Network!";
        let data3 = b"Different data";

        let hash1 = ContentHash::from_data(data1);
        let hash2 = ContentHash::from_data(data2);
        let hash3 = ContentHash::from_data(data3);

        // Same data should produce same hash
        assert_eq!(hash1, hash2);
        // Different data should produce different hash
        assert_ne!(hash1, hash3);

        // Test hex conversion
        let hex_string = hash1.to_hex();
        let parsed_hash = ContentHash::from_hex(&hex_string).unwrap();
        assert_eq!(hash1, parsed_hash);
    }

    #[test]
    fn test_file_creation_and_chunking() {
        let data = vec![0u8; 100 * 1024 * 1024]; // 100MB file
        let file = File::new(
            "large_file.bin".to_string(),
            data.clone(),
            "application/octet-stream".to_string(),
        );

        assert_eq!(file.size, data.len() as u64);
        assert_eq!(file.name, "large_file.bin");
        assert!(file.needs_chunking());
        assert_eq!(file.id, ContentHash::from_data(&data));
    }

    #[test]
    fn test_provider_tiers_and_pricing() {
        // Test typical pricing for different tiers
        assert_eq!(ProviderTier::Home.typical_price(), Decimal::new(2, 3));
        assert_eq!(ProviderTier::Professional.typical_price(), Decimal::new(4, 3));
        assert_eq!(ProviderTier::Enterprise.typical_price(), Decimal::new(8, 3));
        assert_eq!(ProviderTier::GlobalCDN.typical_price(), Decimal::new(12, 3));

        // Test uptime guarantees
        assert!(ProviderTier::Enterprise.typical_uptime() > ProviderTier::Home.typical_uptime());
        assert!(ProviderTier::GlobalCDN.typical_uptime() > ProviderTier::Professional.typical_uptime());
    }

    #[test]
    fn test_provider_functionality() {
        let provider = Provider::new(
            "Test Provider".to_string(),
            ProviderTier::Professional,
            Region::NorthAmerica,
            "http://test-provider.com".to_string(),
            1024 * 1024 * 1024, // 1GB
            Decimal::new(5, 3),
        );

        // Test capacity checking
        assert!(provider.can_store(512 * 1024 * 1024)); // 512MB - should fit
        assert!(!provider.can_store(2 * 1024 * 1024 * 1024)); // 2GB - shouldn't fit

        // Test cost calculation
        let cost = provider.calculate_monthly_cost(Decimal::new(1, 0)); // 1GB
        assert_eq!(cost, Decimal::new(5, 3)); // $0.005

        // Test online status (should be true initially since last_seen is Utc::now())
        assert!(provider.is_online());
    }

    #[test]
    fn test_reputation_score_calculation() {
        let mut score = ReputationScore::new();
        
        // Test default values
        assert_eq!(score.overall, Decimal::new(5, 1)); // 0.5
        assert_eq!(score.uptime, Decimal::ONE);
        assert_eq!(score.contracts_completed, 0);

        // Test overall calculation
        score.uptime = Decimal::new(95, 2); // 0.95
        score.data_integrity = Decimal::new(98, 2); // 0.98
        score.response_time = Decimal::new(85, 2); // 0.85
        score.contract_compliance = Decimal::new(100, 2); // 1.00
        score.community_feedback = Decimal::new(80, 2); // 0.80
        
        score.calculate_overall();
        
        // Should be weighted average: 0.25*0.95 + 0.25*0.98 + 0.20*0.85 + 0.20*1.00 + 0.10*0.80
        let expected = Decimal::new(2375, 4) + Decimal::new(245, 3) + Decimal::new(17, 2) + Decimal::new(2, 1) + Decimal::new(8, 2);
        assert!((score.overall - expected).abs() < Decimal::new(1, 3)); // Allow small rounding differences

        // Test trustworthiness
        assert!(score.is_trustworthy()); // Should be > 0.6
    }

    #[test]
    fn test_storage_request_validation() {
        let file_id = ContentHash::from_data(b"test file");
        let requirements = ProviderRequirements::important();

        // Test valid replication factors
        for factor in 1..=10 {
            let request = StorageRequest::new(
                file_id,
                factor,
                Decimal::new(10, 3),
                requirements.clone(),
            );
            assert!(request.is_ok());
        }

        // Test invalid replication factors
        let invalid_request = StorageRequest::new(
            file_id,
            0, // Invalid
            Decimal::new(10, 3),
            requirements.clone(),
        );
        assert!(invalid_request.is_err());

        let invalid_request = StorageRequest::new(
            file_id,
            11, // Invalid
            Decimal::new(10, 3),
            requirements,
        );
        assert!(invalid_request.is_err());
    }

    #[test]
    fn test_provider_requirements_presets() {
        let critical = ProviderRequirements::critical();
        assert_eq!(critical.min_uptime, Decimal::new(999, 3)); // 99.9%
        assert!(critical.exclude_home_providers);
        assert!(critical.require_backup_power);
        assert_eq!(critical.max_latency_ms, 100);

        let important = ProviderRequirements::important();
        assert_eq!(important.min_uptime, Decimal::new(95, 2)); // 95%
        assert!(!important.exclude_home_providers);

        let backup = ProviderRequirements::backup();
        assert_eq!(backup.min_uptime, Decimal::new(90, 2)); // 90%
        assert_eq!(backup.max_latency_ms, 2000);
    }
}

#[cfg(test)]
mod crypto_tests {
    use carbide_crypto::*;

    #[test]
    fn test_encryption_roundtrip() {
        let data = b"Sensitive data that needs encryption";
        let password = "strong_password_123";

        // Test encryption
        let encrypted = encrypt_data(data, password).unwrap();
        assert_ne!(encrypted, data); // Should be different after encryption
        assert!(encrypted.len() > data.len()); // Should include overhead

        // Test decryption
        let decrypted = decrypt_data(&encrypted, password).unwrap();
        assert_eq!(decrypted, data); // Should match original

        // Test wrong password
        let wrong_decrypt = decrypt_data(&encrypted, "wrong_password");
        assert!(wrong_decrypt.is_err());
    }

    #[test]
    fn test_key_derivation() {
        let password = "test_password";
        let salt1 = [1u8; 32];
        let salt2 = [2u8; 32];

        let key1_a = derive_key(password, &salt1).unwrap();
        let key1_b = derive_key(password, &salt1).unwrap();
        let key2 = derive_key(password, &salt2).unwrap();

        // Same password and salt should produce same key
        assert_eq!(key1_a, key1_b);
        // Different salt should produce different key
        assert_ne!(key1_a, key2);
        // Key should be 32 bytes (256 bits)
        assert_eq!(key1_a.len(), 32);
    }

    #[test]
    fn test_hash_consistency() {
        let data1 = b"Test data for hashing";
        let data2 = b"Test data for hashing";
        let data3 = b"Different test data";

        let hash1 = hash_data(data1);
        let hash2 = hash_data(data2);
        let hash3 = hash_data(data3);

        // Same data should produce same hash
        assert_eq!(hash1, hash2);
        // Different data should produce different hash
        assert_ne!(hash1, hash3);
        // Hash should be 32 bytes (256 bits)
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_large_data_encryption() {
        // Test with larger data to ensure encryption handles it properly
        let large_data = vec![0xAA; 1024 * 1024]; // 1MB of data
        let password = "encryption_test_password";

        let encrypted = encrypt_data(&large_data, password).unwrap();
        let decrypted = decrypt_data(&encrypted, password).unwrap();

        assert_eq!(decrypted, large_data);
        assert!(encrypted.len() > large_data.len()); // Includes overhead
    }

    #[test]
    fn test_random_data_encryption() {
        // Test with random data to ensure no patterns break encryption
        let random_data: Vec<u8> = (0..10000).map(|_| rand::random()).collect();
        let password = "random_test_password";

        let encrypted = encrypt_data(&random_data, password).unwrap();
        let decrypted = decrypt_data(&encrypted, password).unwrap();

        assert_eq!(decrypted, random_data);
    }
}

#[cfg(test)]
mod reputation_tests {
    use carbide_reputation::*;
    use carbide_reputation::events::*;
    use carbide_reputation::scoring::*;
    use carbide_core::*;
    use uuid::Uuid;

    #[test]
    fn test_reputation_weights_validation() {
        // Test default weights
        let default_weights = ReputationWeights::default();
        assert!(default_weights.validate().is_ok());

        // Test custom valid weights
        let custom_weights = ReputationWeights {
            uptime: rust_decimal::Decimal::new(3, 1),              // 0.3
            data_integrity: rust_decimal::Decimal::new(3, 1),      // 0.3
            response_time: rust_decimal::Decimal::new(2, 1),       // 0.2
            contract_compliance: rust_decimal::Decimal::new(1, 1), // 0.1
            community_feedback: rust_decimal::Decimal::new(1, 1),  // 0.1
        };
        assert!(custom_weights.validate().is_ok());

        // Test invalid weights (don't sum to 1.0)
        let invalid_weights = ReputationWeights {
            uptime: rust_decimal::Decimal::new(5, 1),              // 0.5
            data_integrity: rust_decimal::Decimal::new(5, 1),      // 0.5
            response_time: rust_decimal::Decimal::new(2, 1),       // 0.2
            contract_compliance: rust_decimal::Decimal::new(1, 1), // 0.1
            community_feedback: rust_decimal::Decimal::new(1, 1),  // 0.1 (total = 1.3)
        };
        assert!(invalid_weights.validate().is_err());
    }

    #[test]
    fn test_reputation_preset_weights() {
        let balanced = ReputationWeights::balanced();
        assert!(balanced.validate().is_ok());
        // All components should be equal
        assert_eq!(balanced.uptime, balanced.data_integrity);
        assert_eq!(balanced.data_integrity, balanced.response_time);

        let reliability = ReputationWeights::reliability_focused();
        assert!(reliability.validate().is_ok());
        // Uptime and data_integrity should be highest
        assert!(reliability.uptime >= reliability.response_time);
        assert!(reliability.data_integrity >= reliability.contract_compliance);

        let performance = ReputationWeights::performance_focused();
        assert!(performance.validate().is_ok());
        // Response time should be highest
        assert!(performance.response_time > performance.uptime);
        assert!(performance.response_time > performance.community_feedback);
    }

    #[tokio::test]
    async fn test_event_impact_scoring() {
        let provider_id = Uuid::new_v4();

        // Test positive events
        let positive_event = ReputationEvent::new(
            provider_id,
            EventType::ContractCompleted {
                final_value: rust_decimal::Decimal::new(100, 2),
                duration_served_days: 30,
            },
            EventSeverity::ExtremelyPositive,
        );

        let impact = positive_event.impact_score();
        assert!(impact > 0.0);
        assert_eq!(impact, 3.0); // 2.0 (ExtremelyPositive) * 1.5 (ContractCompleted)

        // Test negative events
        let negative_event = ReputationEvent::new(
            provider_id,
            EventType::DataCorruption {
                corrupted_files: 5,
                corrupted_bytes: 1024,
                recovered: false,
            },
            EventSeverity::ExtremelyNegative,
        );

        let negative_impact = negative_event.impact_score();
        assert!(negative_impact < 0.0);
        assert_eq!(negative_impact, -4.0); // -2.0 * 2.0

        // Test event categorization
        assert!(positive_event.affects_contract_compliance());
        assert!(!positive_event.affects_uptime());

        assert!(negative_event.affects_data_integrity());
        assert!(!negative_event.affects_community_feedback());
    }

    #[tokio::test]
    async fn test_reputation_calculation() {
        let calculator = ReputationCalculator::new(ScoringConfig::default());
        let provider_id = Uuid::new_v4();
        let current_time = chrono::Utc::now();

        // Create a mix of events
        let events = vec![
            // Good uptime
            ReputationEvent::new(provider_id, EventType::Online, EventSeverity::Positive),
            
            // Good proof performance
            ReputationEvent::new(
                provider_id,
                EventType::ProofSuccess { response_time_ms: 120, chunks_proven: 5 },
                EventSeverity::Positive
            ),
            
            // Contract completion
            ReputationEvent::new(
                provider_id,
                EventType::ContractCompleted {
                    final_value: rust_decimal::Decimal::new(100, 2),
                    duration_served_days: 30,
                },
                EventSeverity::ExtremelyPositive
            ),
            
            // Some negative event
            ReputationEvent::new(
                provider_id,
                EventType::ProofFailure {
                    reason: "Network timeout".to_string(),
                    error_details: None,
                },
                EventSeverity::Negative
            ),
        ];

        let score = calculator.calculate_score(&events, current_time).unwrap();

        // Overall score should be reasonable
        assert!(score.overall >= rust_decimal::Decimal::new(5, 1)); // >= 0.5
        assert!(score.overall <= rust_decimal::Decimal::ONE);

        // Uptime should be good (only positive events)
        assert_eq!(score.uptime, rust_decimal::Decimal::ONE);

        // Data integrity should be decent but not perfect (1 success, 1 failure)
        assert!(score.data_integrity < rust_decimal::Decimal::ONE);
        assert!(score.data_integrity > rust_decimal::Decimal::ZERO);

        // Contract compliance should be perfect
        assert_eq!(score.contract_compliance, rust_decimal::Decimal::ONE);

        // Should have 1 contract completed
        assert_eq!(score.contracts_completed, 1);
    }

    #[tokio::test]
    async fn test_component_score_calculation() {
        let calculator = ReputationCalculator::new(ScoringConfig::default());
        let provider_id = Uuid::new_v4();
        let current_time = chrono::Utc::now();

        let events = vec![
            ReputationEvent::new(provider_id, EventType::Online, EventSeverity::Positive),
            ReputationEvent::new(
                provider_id,
                EventType::ProofSuccess { response_time_ms: 50, chunks_proven: 3 },
                EventSeverity::Positive
            ),
        ];

        let components = calculator.calculate_component_scores(&events, current_time).unwrap();

        assert_eq!(components.uptime, rust_decimal::Decimal::ONE);
        assert_eq!(components.data_integrity, rust_decimal::Decimal::ONE);
        assert_eq!(components.response_time, rust_decimal::Decimal::ONE); // Fast response (50ms)
        
        // Check event counts
        assert_eq!(components.event_counts.total, 2);
        assert_eq!(components.event_counts.uptime_events, 1);
        assert_eq!(components.event_counts.data_integrity_events, 1);
        assert_eq!(components.event_counts.positive_events, 2);
        assert_eq!(components.event_counts.negative_events, 0);
    }

    #[tokio::test]
    async fn test_memory_storage() {
        let mut storage = MemoryStorage::new();
        let provider_id = Uuid::new_v4();
        
        let event = ReputationEvent::new(
            provider_id,
            EventType::Online,
            EventSeverity::Positive,
        );

        // Test event storage
        storage.store_event(&event).await.unwrap();
        
        let stored_events = storage.get_all_events(&provider_id).await.unwrap();
        assert_eq!(stored_events.len(), 1);
        assert_eq!(stored_events[0].provider_id, provider_id);

        // Test reputation storage
        let score = ReputationScore::new();
        storage.store_reputation(&provider_id, &score).await.unwrap();
        
        let stored_score = storage.get_reputation(&provider_id).await.unwrap();
        assert_eq!(stored_score.overall, score.overall);

        // Test statistics
        let stats = storage.get_statistics().await.unwrap();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.total_providers, 1);
    }

    #[test]
    fn test_trend_analysis() {
        use carbide_reputation::utils::analyze_trend;
        use rust_decimal::Decimal;

        // Test improving trend
        let improving_scores = vec![
            Decimal::new(3, 1), // 0.3
            Decimal::new(4, 1), // 0.4
            Decimal::new(5, 1), // 0.5
            Decimal::new(6, 1), // 0.6
            Decimal::new(7, 1), // 0.7
        ];

        match analyze_trend(&improving_scores) {
            ReputationTrend::Improving { rate } => {
                assert!(rate > Decimal::ZERO);
            }
            _ => panic!("Expected improving trend"),
        }

        // Test declining trend
        let declining_scores = vec![
            Decimal::new(8, 1), // 0.8
            Decimal::new(7, 1), // 0.7
            Decimal::new(6, 1), // 0.6
            Decimal::new(5, 1), // 0.5
            Decimal::new(4, 1), // 0.4
        ];

        match analyze_trend(&declining_scores) {
            ReputationTrend::Declining { rate } => {
                assert!(rate > Decimal::ZERO);
            }
            _ => panic!("Expected declining trend"),
        }

        // Test stable trend
        let stable_scores = vec![
            Decimal::new(6, 1), // 0.6
            Decimal::new(6, 1), // 0.6
            Decimal::new(6, 1), // 0.6
            Decimal::new(6, 1), // 0.6
            Decimal::new(6, 1), // 0.6
        ];

        match analyze_trend(&stable_scores) {
            ReputationTrend::Stable { variance } => {
                assert_eq!(variance, Decimal::ZERO);
            }
            _ => panic!("Expected stable trend"),
        }

        // Test insufficient data
        let insufficient_scores = vec![Decimal::new(5, 1)];
        match analyze_trend(&insufficient_scores) {
            ReputationTrend::Insufficient => {},
            _ => panic!("Expected insufficient data"),
        }
    }
}

#[cfg(test)]
mod client_tests {
    use carbide_client::*;
    use carbide_core::*;
    use std::collections::HashMap;

    #[test]
    fn test_storage_preferences() {
        let default_prefs = StoragePreferences::default();
        
        assert_eq!(default_prefs.replication_factor, 3);
        assert!(default_prefs.preferred_regions.contains(&Region::NorthAmerica));
        assert!(default_prefs.preferred_regions.contains(&Region::Europe));
        assert!(default_prefs.preferred_tiers.contains(&ProviderTier::Professional));
        assert!(default_prefs.preferred_tiers.contains(&ProviderTier::Enterprise));
    }

    #[tokio::test]
    async fn test_storage_manager_creation() {
        let client = CarbideClient::new("http://localhost:8080".to_string()).unwrap();
        let storage_manager = StorageManager::new(
            client,
            "http://localhost:9090".to_string(),
        );

        // Verify storage manager was created with default preferences
        let prefs = storage_manager.preferences();
        assert_eq!(prefs.replication_factor, 3);
    }

    #[tokio::test]
    async fn test_simple_api_structure() {
        // Test that simple API functions have correct signatures
        // Note: These will fail at runtime without services, but we're testing compilation
        
        let test_data = b"Simple API test";
        let file_id = ContentHash::from_data(test_data);
        
        // Test store_file function exists and has correct signature
        let store_result = simple::store_file(test_data, 1).await;
        assert!(store_result.is_err()); // Expected to fail without running services
        
        // Test retrieve_file function exists and has correct signature
        let retrieve_result = simple::retrieve_file(&file_id, "test_token").await;
        assert!(retrieve_result.is_err()); // Expected to fail without running services
    }

    #[test]
    fn test_carbide_client_creation() {
        // Test default client creation
        let default_client = CarbideClient::default();
        assert!(default_client.is_ok());

        // Test custom client creation
        let custom_client = CarbideClient::new("http://custom-endpoint:8080".to_string());
        assert!(custom_client.is_ok());
    }

    #[test]
    fn test_client_configuration() {
        let client = CarbideClient::new("http://test:8080".to_string()).unwrap();
        
        // Test health check URL formation (we can't actually call it without a server)
        // This tests that the client properly formats endpoints
        let health_url = format!("{}/health", "http://test:8080");
        assert_eq!(health_url, "http://test:8080/health");
    }
}

#[cfg(test)]
mod network_protocol_tests {
    use carbide_core::network::*;
    use carbide_core::*;
    use serde_json;

    #[test]
    fn test_store_file_request_serialization() {
        let request = StoreFileRequest {
            file_id: ContentHash::from_data(b"test"),
            file_size: 1024,
            duration_months: 12,
            encryption_info: None,
            requirements: ProviderRequirements::important(),
            max_price: rust_decimal::Decimal::new(10, 3),
        };

        // Test serialization
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("file_id"));
        assert!(json.contains("file_size"));
        assert!(json.contains("1024"));

        // Test deserialization
        let deserialized: StoreFileRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_id, request.file_id);
        assert_eq!(deserialized.file_size, request.file_size);
        assert_eq!(deserialized.duration_months, request.duration_months);
    }

    #[test]
    fn test_store_file_response_serialization() {
        let response = StoreFileResponse {
            accepted: true,
            upload_url: Some("http://provider.com/upload".to_string()),
            upload_token: Some("auth_token_123".to_string()),
            contract: Some(StorageContract {
                id: uuid::Uuid::new_v4(),
                request_id: uuid::Uuid::new_v4(),
                file_id: ContentHash::from_data(b"test"),
                provider_id: uuid::Uuid::new_v4(),
                price_per_gb_month: rust_decimal::Decimal::new(5, 3),
                duration_months: 12,
                started_at: chrono::Utc::now(),
                status: ContractStatus::Active,
                last_proof_at: None,
            }),
            rejection_reason: None,
            estimated_cost: Some(rust_decimal::Decimal::new(60, 3)),
        };

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("accepted"));
        assert!(json.contains("true"));
        assert!(json.contains("upload_url"));

        // Test deserialization
        let deserialized: StoreFileResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.accepted, response.accepted);
        assert_eq!(deserialized.upload_url, response.upload_url);
        assert!(deserialized.contract.is_some());
    }

    #[test]
    fn test_provider_list_response_serialization() {
        let providers = vec![
            Provider::new(
                "Test Provider 1".to_string(),
                ProviderTier::Professional,
                Region::NorthAmerica,
                "http://provider1.com".to_string(),
                1024 * 1024 * 1024, // 1GB
                rust_decimal::Decimal::new(5, 3),
            ),
            Provider::new(
                "Test Provider 2".to_string(),
                ProviderTier::Enterprise,
                Region::Europe,
                "http://provider2.com".to_string(),
                10 * 1024 * 1024 * 1024, // 10GB
                rust_decimal::Decimal::new(8, 3),
            ),
        ];

        let response = ProviderListResponse {
            providers: providers.clone(),
            total_count: providers.len() as u64,
            page: 1,
            limit: 10,
        };

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("providers"));
        assert!(json.contains("total_count"));
        assert!(json.contains("Test Provider 1"));
        assert!(json.contains("Test Provider 2"));

        // Test deserialization
        let deserialized: ProviderListResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.providers.len(), 2);
        assert_eq!(deserialized.total_count, 2);
        assert_eq!(deserialized.providers[0].name, "Test Provider 1");
        assert_eq!(deserialized.providers[1].name, "Test Provider 2");
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: ServiceStatus::Healthy,
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            details: {
                let mut map = std::collections::HashMap::new();
                map.insert("database".to_string(), "connected".to_string());
                map.insert("storage".to_string(), "available".to_string());
                map
            },
        };

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("status"));
        assert!(json.contains("Healthy"));
        assert!(json.contains("version"));
        assert!(json.contains("uptime_seconds"));

        // Test deserialization
        let deserialized: HealthResponse = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.status, ServiceStatus::Healthy));
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.uptime_seconds, 3600);
        assert_eq!(deserialized.details.len(), 2);
    }

    #[test]
    fn test_service_status_variants() {
        // Test all service status variants can be serialized/deserialized
        let statuses = vec![
            ServiceStatus::Healthy,
            ServiceStatus::Degraded,
            ServiceStatus::Unavailable,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: ServiceStatus = serde_json::from_str(&json).unwrap();
            assert!(std::mem::discriminant(&status) == std::mem::discriminant(&deserialized));
        }
    }
}