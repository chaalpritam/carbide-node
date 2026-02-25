//! # Carbide Marketplace Discovery Demo
//!
//! Demonstrates the complete discovery service and marketplace functionality:
//! - Provider registration and discovery
//! - Marketplace statistics and provider search
//! - Quote aggregation from multiple providers
//! - Health monitoring and registry management

use std::time::Duration;

use carbide_core::{network::*, ContentHash, Provider, ProviderTier, Region};
use carbide_discovery::{DiscoveryConfig, DiscoveryService};
use rust_decimal::Decimal;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🔍 Carbide Marketplace Discovery Demo");
    println!("====================================\n");

    // 1. Start discovery service
    println!("🚀 Starting Discovery Service...");
    let discovery_config = DiscoveryConfig {
        host: "127.0.0.1".to_string(),
        port: 9090,
        health_check_interval: Duration::from_secs(10),
        provider_timeout: Duration::from_secs(60),
        max_search_results: 50,
    };

    let discovery_service = DiscoveryService::new(discovery_config);

    // Start discovery service in background
    let discovery_handle = tokio::spawn(async move {
        if let Err(e) = discovery_service.start().await {
            eprintln!("Discovery service error: {}", e);
        }
    });

    // Wait for discovery service to start
    sleep(Duration::from_secs(2)).await;

    println!("✅ Discovery service running on http://127.0.0.1:9090");

    // 2. Register multiple test providers
    println!("\n📝 Registering Test Providers...");
    let client = reqwest::Client::new();
    let base_url = "http://127.0.0.1:9090";

    let test_providers = vec![
        Provider::new(
            "HomeStorage USA".to_string(),
            ProviderTier::Home,
            Region::NorthAmerica,
            "http://provider1.example.com".to_string(),
            500 * 1024 * 1024 * 1024, // 500GB
            Decimal::new(2, 3),       // $0.002/GB/month
        ),
        Provider::new(
            "EuroCloud Pro".to_string(),
            ProviderTier::Professional,
            Region::Europe,
            "http://provider2.example.com".to_string(),
            2 * 1024 * 1024 * 1024 * 1024, // 2TB
            Decimal::new(4, 3),            // $0.004/GB/month
        ),
        Provider::new(
            "Asia DataCenter".to_string(),
            ProviderTier::Enterprise,
            Region::Asia,
            "http://provider3.example.com".to_string(),
            10 * 1024 * 1024 * 1024 * 1024, // 10TB
            Decimal::new(6, 3),             // $0.006/GB/month
        ),
        Provider::new(
            "Global CDN Network".to_string(),
            ProviderTier::GlobalCDN,
            Region::NorthAmerica,
            "http://provider4.example.com".to_string(),
            50 * 1024 * 1024 * 1024 * 1024, // 50TB
            Decimal::new(10, 3),            // $0.010/GB/month
        ),
        Provider::new(
            "Home Storage EU".to_string(),
            ProviderTier::Home,
            Region::Europe,
            "http://provider5.example.com".to_string(),
            800 * 1024 * 1024 * 1024, // 800GB
            Decimal::new(2, 3),       // $0.002/GB/month
        ),
    ];

    for (i, provider) in test_providers.iter().enumerate() {
        println!(
            "   {}. Registering: {} ({:?}, {:?})",
            i + 1,
            provider.name,
            provider.tier,
            provider.region
        );

        let announcement = ProviderAnnouncement {
            provider: provider.clone(),
            endpoint: provider.endpoint.clone(),
            supported_versions: vec!["1.0".to_string()],
            public_key: Some("mock_public_key".to_string()),
            wallet_address: None,
        };

        let register_response = client
            .post(&format!("{}/api/v1/providers", base_url))
            .json(&announcement)
            .send()
            .await?;

        if register_response.status().is_success() {
            println!("      ✅ Registered successfully");
        } else {
            println!(
                "      ❌ Registration failed: {}",
                register_response.status()
            );
        }
    }

    // 3. Check marketplace statistics
    println!("\n📊 Marketplace Statistics:");
    let stats_response = client
        .get(&format!("{}/api/v1/marketplace/stats", base_url))
        .send()
        .await?;

    if stats_response.status().is_success() {
        let stats: serde_json::Value = stats_response.json().await?;
        println!(
            "   Total Providers: {}",
            stats["total_providers"].as_u64().unwrap_or(0)
        );
        println!(
            "   Online Providers: {}",
            stats["online_providers"].as_u64().unwrap_or(0)
        );

        if let Some(total_capacity) = stats["total_capacity_bytes"].as_u64() {
            println!(
                "   Total Capacity: {:.2} TB",
                total_capacity as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
            );
        }

        if let Some(available_capacity) = stats["available_capacity_bytes"].as_u64() {
            println!(
                "   Available Capacity: {:.2} TB",
                available_capacity as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
            );
        }

        if let Some(avg_price) = stats["average_price_per_gb"].as_str() {
            println!("   Average Price: ${}/GB/month", avg_price);
        }
    } else {
        println!(
            "   ❌ Failed to get statistics: {}",
            stats_response.status()
        );
    }

    // 4. Test provider discovery and filtering
    println!("\n🔍 Provider Discovery Tests:");

    // Test 1: List all providers
    println!("   1. All Providers:");
    let all_providers_response = client
        .get(&format!("{}/api/v1/providers?limit=10", base_url))
        .send()
        .await?;

    if all_providers_response.status().is_success() {
        let provider_list: serde_json::Value = all_providers_response.json().await?;
        if let Some(providers) = provider_list["providers"].as_array() {
            println!("      Found {} providers:", providers.len());
            for provider in providers {
                if let (Some(name), Some(tier), Some(region), Some(price)) = (
                    provider["name"].as_str(),
                    provider["tier"].as_str(),
                    provider["region"].as_str(),
                    provider["price_per_gb_month"].as_str(),
                ) {
                    println!(
                        "        • {} ({}, {}) - ${}/GB/month",
                        name, tier, region, price
                    );
                }
            }
        }
    }

    // Test 2: Filter by region
    println!("\n   2. North American Providers:");
    let na_providers_response = client
        .get(&format!(
            "{}/api/v1/providers?region=northamerica",
            base_url
        ))
        .send()
        .await?;

    if na_providers_response.status().is_success() {
        let provider_list: serde_json::Value = na_providers_response.json().await?;
        if let Some(providers) = provider_list["providers"].as_array() {
            println!("      Found {} North American providers:", providers.len());
            for provider in providers {
                if let Some(name) = provider["name"].as_str() {
                    println!("        • {}", name);
                }
            }
        }
    }

    // Test 3: Filter by tier
    println!("\n   3. Professional+ Providers:");
    let pro_providers_response = client
        .get(&format!("{}/api/v1/providers?tier=professional", base_url))
        .send()
        .await?;

    if pro_providers_response.status().is_success() {
        let provider_list: serde_json::Value = pro_providers_response.json().await?;
        if let Some(providers) = provider_list["providers"].as_array() {
            println!("      Found {} Professional providers:", providers.len());
            for provider in providers {
                if let Some(name) = provider["name"].as_str() {
                    println!("        • {}", name);
                }
            }
        }
    }

    // 5. Test storage quote requests
    println!("\n💰 Storage Quote Aggregation:");
    let quote_request = StorageQuoteRequest {
        file_size: 1024 * 1024 * 1024, // 1GB file
        replication_factor: 3,
        duration_months: 12,
        requirements: carbide_core::ProviderRequirements::important(),
        preferred_regions: vec![Region::NorthAmerica, Region::Europe],
    };

    println!("   Request: 1GB file, 3x replication, 12 months");
    println!("   Preferred regions: North America, Europe");

    let quotes_response = client
        .post(&format!("{}/api/v1/marketplace/quotes", base_url))
        .json(&quote_request)
        .send()
        .await?;

    if quotes_response.status().is_success() {
        let quotes: Vec<serde_json::Value> = quotes_response.json().await?;

        if quotes.is_empty() {
            println!("   ℹ️ No quotes returned (providers are mock endpoints)");
            println!("   💡 In a real deployment, providers would respond with quotes");
        } else {
            println!("   📋 Received {} quotes:", quotes.len());
            for (i, quote) in quotes.iter().enumerate() {
                println!(
                    "      {}. Provider: {}",
                    i + 1,
                    quote["provider_id"].as_str().unwrap_or("Unknown")
                );
                println!(
                    "         Price: ${}/GB/month",
                    quote["price_per_gb_month"].as_str().unwrap_or("?")
                );
                println!(
                    "         Total Cost: ${}/month",
                    quote["total_monthly_cost"].as_str().unwrap_or("?")
                );
                println!(
                    "         Can Fulfill: {}",
                    quote["can_fulfill"].as_bool().unwrap_or(false)
                );
            }
        }
    } else {
        println!("   ❌ Quote request failed: {}", quotes_response.status());
    }

    // 6. Test individual provider lookup
    println!("\n🔎 Individual Provider Lookup:");

    // Get a provider ID from the registry
    let providers_response = client
        .get(&format!("{}/api/v1/providers?limit=1", base_url))
        .send()
        .await?;

    if providers_response.status().is_success() {
        let provider_list: serde_json::Value = providers_response.json().await?;
        if let Some(providers) = provider_list["providers"].as_array() {
            if let Some(first_provider) = providers.first() {
                if let Some(provider_id) = first_provider["id"].as_str() {
                    println!("   Looking up provider: {}", provider_id);

                    let lookup_response = client
                        .get(&format!("{}/api/v1/providers/{}", base_url, provider_id))
                        .send()
                        .await?;

                    if lookup_response.status().is_success() {
                        let provider_details: serde_json::Value = lookup_response.json().await?;
                        println!("   ✅ Provider Details:");
                        println!(
                            "      Name: {}",
                            provider_details["provider"]["name"]
                                .as_str()
                                .unwrap_or("Unknown")
                        );
                        println!(
                            "      Tier: {}",
                            provider_details["provider"]["tier"]
                                .as_str()
                                .unwrap_or("Unknown")
                        );
                        println!(
                            "      Region: {}",
                            provider_details["provider"]["region"]
                                .as_str()
                                .unwrap_or("Unknown")
                        );
                        println!(
                            "      Registered: {}",
                            provider_details["registered_at"]
                                .as_str()
                                .unwrap_or("Unknown")
                        );
                        println!(
                            "      Health Status: {}",
                            provider_details["health_status"]
                                .as_str()
                                .unwrap_or("Unknown")
                        );
                    } else {
                        println!("   ❌ Lookup failed: {}", lookup_response.status());
                    }
                }
            }
        }
    }

    // 7. Test discovery service health
    println!("\n🏥 Discovery Service Health:");
    let health_response = client
        .get(&format!("{}/api/v1/health", base_url))
        .send()
        .await?;

    if health_response.status().is_success() {
        let health: serde_json::Value = health_response.json().await?;
        println!("   ✅ Discovery Service is healthy");
        println!(
            "   Status: {}",
            health["status"].as_str().unwrap_or("Unknown")
        );
        println!(
            "   Version: {}",
            health["version"].as_str().unwrap_or("Unknown")
        );
    } else {
        println!("   ❌ Health check failed: {}", health_response.status());
    }

    // 8. Summary
    println!("\n🎉 Marketplace Discovery Demo Complete!");
    println!("This demonstration showed:");
    println!("  • Provider registration and discovery service");
    println!("  • Regional and tier-based provider filtering");
    println!("  • Marketplace statistics and capacity tracking");
    println!("  • Multi-provider quote aggregation (architecture)");
    println!("  • Provider health monitoring and registry management");
    println!("  • RESTful API for marketplace interactions");

    println!("\n💡 Key Features Implemented:");
    println!("  • Centralized provider registry with DashMap for performance");
    println!("  • Automatic health checking and provider timeout handling");
    println!("  • Regional and tier-based indexing for fast provider lookup");
    println!("  • Marketplace statistics with real-time capacity tracking");
    println!("  • Quote aggregation system (ready for live provider integration)");
    println!("  • REST API with proper error handling and CORS support");

    // Shutdown
    discovery_handle.abort();

    Ok(())
}
