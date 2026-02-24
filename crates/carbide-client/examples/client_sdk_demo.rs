//! # Carbide Client SDK Demo
//!
//! Comprehensive demonstration of the enhanced client SDK featuring:
//! - High-level storage operations with automatic provider selection
//! - Discovery service integration for marketplace access
//! - Progress tracking and error handling
//! - Simple APIs for easy mobile/desktop integration

use std::{sync::Arc, time::Duration};

use carbide_client::{
    simple, CarbideClient, ClientConfig, DiscoveryClient, MarketplaceQuery, ProgressCallback,
    ProviderFilter, StorageManager, StoragePreferences, StorageProgress,
};
use carbide_core::{ContentHash, ProviderRequirements, ProviderTier, Region};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("📱 Carbide Client SDK Demo");
    println!("=========================\n");

    // 1. Demonstrate basic client functionality
    demo_basic_client().await?;

    // 2. Demonstrate discovery service integration
    demo_discovery_integration().await?;

    // 3. Demonstrate high-level storage operations
    demo_storage_management().await?;

    // 4. Demonstrate simple convenience APIs
    demo_simple_apis().await?;

    println!("\n🎉 Client SDK Demo Complete!");
    println!("This demonstrates the complete Carbide Client SDK for:");
    println!("  • Mobile and desktop application integration");
    println!("  • High-level storage operations with progress tracking");
    println!("  • Automatic provider discovery and selection");
    println!("  • Marketplace statistics and quote comparison");
    println!("  • Simple APIs for common use cases");

    Ok(())
}

async fn demo_basic_client() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Basic Client Functionality");
    println!("----------------------------");

    // Create a configured client
    let config = ClientConfig {
        timeout: Duration::from_secs(15),
        max_retries: 2,
        user_agent: "CarbideDemo/1.0".to_string(),
        enable_logging: true,
    };

    let client = CarbideClient::new(config)?;
    println!("✅ Client created with custom configuration");

    // Test provider connectivity (these would be real endpoints in production)
    let test_endpoints = vec![
        "http://localhost:8080".to_string(),
        "http://localhost:8081".to_string(),
        "http://provider.example.com".to_string(),
    ];

    println!("🔍 Testing provider connectivity...");
    let test_results = client.test_providers(&test_endpoints).await;

    for result in test_results {
        let status_icon = if result.online { "✅" } else { "❌" };
        println!(
            "   {} {} ({} ms)",
            status_icon, result.endpoint, result.latency_ms
        );

        if let Some(error) = result.error {
            println!("      Error: {}", error);
        }
    }

    println!();
    Ok(())
}

async fn demo_discovery_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Discovery Service Integration");
    println!("------------------------------");

    let client = CarbideClient::with_defaults()?;
    let discovery = DiscoveryClient::new(client, "http://localhost:9090".to_string());

    // Get marketplace statistics
    println!("📊 Marketplace Statistics:");
    match discovery.get_marketplace_stats().await {
        Ok(stats) => {
            println!("   Total Providers: {}", stats.total_providers);
            println!("   Online Providers: {}", stats.online_providers);
            println!(
                "   Available Capacity: {:.2} GB",
                stats.available_capacity_gb
            );
            println!("   Average Price: ${}/GB/month", stats.average_price_per_gb);
            println!("   Utilization: {:.1}%", stats.utilization_percentage);
        }
        Err(e) => {
            println!("   ❌ Failed to get stats: {}", e);
        }
    }

    // Search for providers
    println!("\n🔎 Provider Search:");
    let query = MarketplaceQuery {
        region: Some(Region::NorthAmerica),
        tier: Some(ProviderTier::Professional),
        limit: Some(5),
        min_reputation: Some(rust_decimal::Decimal::new(30, 2)), // 0.30
    };

    match discovery.search_providers(query).await {
        Ok(providers) => {
            println!(
                "   Found {} Professional providers in North America:",
                providers.len()
            );
            for (i, provider) in providers.iter().enumerate() {
                println!(
                    "      {}. {} ({:?}) - ${}/GB/month",
                    i + 1,
                    provider.provider.name,
                    provider.provider.tier,
                    provider.provider.price_per_gb_month
                );

                if provider.online {
                    println!(
                        "         Status: Online, Load: {:.1}%",
                        provider.load.unwrap_or(0.0) * 100.0
                    );
                } else {
                    println!("         Status: Offline");
                }
            }
        }
        Err(e) => {
            println!("   ❌ Provider search failed: {}", e);
        }
    }

    // Advanced filtering
    println!("\n🎯 Advanced Provider Filtering:");
    let filter = ProviderFilter {
        regions: vec![Region::Europe, Region::Asia],
        tiers: vec![ProviderTier::Enterprise, ProviderTier::GlobalCDN],
        min_capacity: Some(1024 * 1024 * 1024 * 1024), // 1TB
        max_price: Some(rust_decimal::Decimal::new(8, 3)), // $0.008/GB/month
        min_reputation: Some(rust_decimal::Decimal::new(80, 2)), // 0.80
        max_load: Some(0.7),                           // 70% max load
    };

    match discovery.search_providers_advanced(filter).await {
        Ok(providers) => {
            println!(
                "   Found {} high-end providers in Europe/Asia:",
                providers.len()
            );
            for provider in providers {
                println!(
                    "      • {} ({:?}, {:?}) - Capacity: {:.1}TB",
                    provider.provider.name,
                    provider.provider.tier,
                    provider.provider.region,
                    provider.provider.total_capacity as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0)
                );
            }
        }
        Err(e) => {
            println!("   ❌ Advanced search failed: {}", e);
        }
    }

    println!();
    Ok(())
}

async fn demo_storage_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 High-Level Storage Management");
    println!("-------------------------------");

    let client = CarbideClient::with_defaults()?;

    // Create storage manager with custom preferences
    let preferences = StoragePreferences {
        preferred_regions: vec![Region::NorthAmerica, Region::Europe],
        preferred_tiers: vec![ProviderTier::Professional, ProviderTier::Enterprise],
        replication_factor: 2,
        max_price_per_gb: rust_decimal::Decimal::new(6, 3), // $0.006/GB/month
        requirements: ProviderRequirements::important(),
    };

    let storage_manager =
        StorageManager::with_preferences(client, "http://localhost:9090".to_string(), preferences);

    // Create progress callback
    let progress_callback: ProgressCallback = Box::new(|progress: StorageProgress| {
        println!(
            "   📈 {}: {:.1}% - {}",
            progress.operation,
            progress.progress * 100.0,
            progress.message
        );

        if progress.total_bytes > 0 {
            println!(
                "      Transferred: {} / {} bytes",
                progress.bytes_transferred, progress.total_bytes
            );
        }
    });

    // Test file storage
    println!("📤 Storing file with automatic provider selection:");
    let test_data = b"Hello from Carbide Client SDK! This demonstrates high-level storage operations with automatic provider discovery, quote comparison, and progress tracking.";
    let file_id = ContentHash::from_data(test_data);

    println!("   File ID: {}", file_id.to_hex());
    println!("   File Size: {} bytes", test_data.len());

    match storage_manager
        .store_file(test_data, 6, Some(progress_callback))
        .await
    {
        Ok(store_result) => {
            println!("   ✅ File stored successfully!");
            println!(
                "      Replicated to: {} providers",
                store_result.providers.len()
            );
            println!(
                "      Total Cost: ${}/month",
                store_result.total_monthly_cost
            );
            println!("      Duration: {} months", store_result.duration_months);

            for (i, location) in store_result.providers.iter().enumerate() {
                println!(
                    "         {}. {} ({:?})",
                    i + 1,
                    location.provider.name,
                    location.provider.region
                );
            }
        }
        Err(e) => {
            println!("   ❌ Storage failed: {}", e);
            println!("   💡 This is expected without running providers");
        }
    }

    // Test file retrieval
    println!("\n📥 Retrieving file:");
    let access_token = "mock_access_token";

    let progress_callback: ProgressCallback = Box::new(|progress: StorageProgress| {
        println!(
            "   📈 {}: {:.1}% - {}",
            progress.operation,
            progress.progress * 100.0,
            progress.message
        );
    });

    match storage_manager
        .retrieve_file(&file_id, access_token, Some(progress_callback))
        .await
    {
        Ok(retrieve_result) => {
            println!("   ✅ File retrieved successfully!");
            println!("      Size: {} bytes", retrieve_result.size);
            println!("      Content Type: {}", retrieve_result.content_type);
            println!("      Provider: {}", retrieve_result.provider.name);

            if retrieve_result.data == test_data {
                println!("      🎯 Data integrity verified!");
            } else {
                println!("      ⚠️ Data integrity check failed");
            }
        }
        Err(e) => {
            println!("   ❌ Retrieval failed: {}", e);
            println!("   💡 This is expected without file stored on providers");
        }
    }

    println!();
    Ok(())
}

async fn demo_simple_apis() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Simple Convenience APIs");
    println!("-------------------------");

    // These are the simplest APIs for basic use cases
    println!("📝 Simple file storage (one-liner):");
    let test_data = b"Simple API test - store this data for 3 months";

    match simple::store_file(test_data, 3).await {
        Ok(result) => {
            println!(
                "   ✅ Stored with {} replicas, cost: ${}/month",
                result.providers.len(),
                result.total_monthly_cost
            );
        }
        Err(e) => {
            println!("   ❌ Storage failed: {}", e);
            println!("   💡 Expected - needs running discovery and providers");
        }
    }

    // Store file from local path
    println!("\n📁 Store file from local path:");
    // Create a temporary test file
    let test_file_path = "/tmp/carbide_test_file.txt";
    let test_content = "This is a test file for the Carbide SDK demo.\nIt demonstrates storing \
                        files from local paths.";

    if let Err(e) = tokio::fs::write(test_file_path, test_content).await {
        println!("   ❌ Failed to create test file: {}", e);
    } else {
        match simple::store_file_from_path(test_file_path, 12).await {
            Ok((file_id, result)) => {
                println!("   ✅ File stored from path");
                println!("      File ID: {}", file_id);
                println!("      Providers: {}", result.providers.len());

                // Clean up
                let _ = tokio::fs::remove_file(test_file_path).await;
            }
            Err(e) => {
                println!("   ❌ Failed to store from path: {}", e);
                // Clean up
                let _ = tokio::fs::remove_file(test_file_path).await;
            }
        }
    }

    // Retrieve file to local path
    println!("\n💾 Retrieve file to local path:");
    let dummy_file_id = ContentHash::from_data(b"dummy_file");
    let output_path = "/tmp/carbide_retrieved_file.txt";

    match simple::retrieve_file_to_path(&dummy_file_id, "access_token", output_path).await {
        Ok(size) => {
            println!("   ✅ File retrieved to path ({} bytes)", size);
            // Clean up
            let _ = tokio::fs::remove_file(output_path).await;
        }
        Err(e) => {
            println!("   ❌ Retrieval to path failed: {}", e);
            println!("   💡 Expected - file doesn't exist on any provider");
        }
    }

    // Health check
    println!("\n🏥 Provider health monitoring:");
    let client = CarbideClient::with_defaults()?;
    let manager = StorageManager::new(client, "http://localhost:9090".to_string());

    match manager.health_check().await {
        Ok(health_map) => {
            println!("   📊 Provider health status:");
            for (endpoint, status) in health_map {
                println!("      {} - {:?}", endpoint, status);
            }
        }
        Err(e) => {
            println!("   ❌ Health check failed: {}", e);
        }
    }

    println!("\n🎯 SDK Features Summary:");
    println!("   • High-level storage operations with progress callbacks");
    println!("   • Automatic provider discovery and selection");
    println!("   • Marketplace integration with quote comparison");
    println!("   • Simple one-liner APIs for basic operations");
    println!("   • File path utilities for local storage integration");
    println!("   • Health monitoring and provider testing");
    println!("   • Configurable storage preferences and filtering");
    println!("   • Error handling with detailed error messages");

    println!("\n📱 Perfect for mobile/desktop integration:");
    println!("   • Async/await support for non-blocking operations");
    println!("   • Progress tracking for UI feedback");
    println!("   • Configurable timeouts and retry logic");
    println!("   • Minimal dependencies for small app bundles");

    println!();
    Ok(())
}
