//! # Carbide Discovery Service Binary
//!
//! Provider discovery and marketplace coordination service that acts as
//! the central registry for storage providers in the Carbide Network.

use std::time::Duration;

use carbide_discovery::{DiscoveryConfig, DiscoveryService};
use clap::Parser;

#[derive(Parser)]
#[command(name = "carbide-discovery")]
#[command(about = "Carbide Network Discovery Service - Provider registry and marketplace")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Start the discovery service
    Start {
        /// Port to listen on
        #[arg(long, default_value = "9090")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Health check interval in seconds
        #[arg(long, default_value = "30")]
        health_check_interval: u64,
        /// Provider timeout in seconds
        #[arg(long, default_value = "300")]
        provider_timeout: u64,
        /// Maximum search results
        #[arg(long, default_value = "100")]
        max_results: usize,
    },
    /// Show marketplace statistics
    Stats {
        /// Discovery service endpoint
        #[arg(long, default_value = "http://localhost:9090")]
        endpoint: String,
    },
    /// List all registered providers
    Providers {
        /// Discovery service endpoint
        #[arg(long, default_value = "http://localhost:9090")]
        endpoint: String,
        /// Filter by region
        #[arg(long)]
        region: Option<String>,
        /// Filter by tier
        #[arg(long)]
        tier: Option<String>,
        /// Maximum number of providers to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Start {
            port,
            host,
            health_check_interval,
            provider_timeout,
            max_results,
        } => {
            println!("🔍 Starting Carbide Discovery Service...");
            println!("   Host: {}", host);
            println!("   Port: {}", port);
            println!("   Health Check Interval: {}s", health_check_interval);
            println!("   Provider Timeout: {}s", provider_timeout);
            println!("   Max Search Results: {}", max_results);

            let config = DiscoveryConfig {
                host,
                port,
                health_check_interval: Duration::from_secs(health_check_interval),
                provider_timeout: Duration::from_secs(provider_timeout),
                max_search_results: max_results,
            };

            let service = DiscoveryService::new(config);

            // Handle shutdown gracefully
            tokio::select! {
                result = service.start() => {
                    if let Err(e) = result {
                        eprintln!("❌ Discovery service error: {}", e);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("🛑 Received shutdown signal, stopping discovery service...");
                }
            }

            println!("✅ Discovery service shut down gracefully");
        }

        Command::Stats { endpoint } => {
            println!("📊 Fetching marketplace statistics from {}...", endpoint);

            let client = reqwest::Client::new();
            let stats_url = format!("{}/api/v1/marketplace/stats", endpoint);

            match client.get(&stats_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(stats) => {
                                println!("✅ Marketplace Statistics:");
                                println!("{}", serde_json::to_string_pretty(&stats)?);
                            }
                            Err(e) => {
                                println!("❌ Failed to parse statistics: {}", e);
                            }
                        }
                    } else {
                        println!("❌ Discovery service returned error: {}", response.status());
                    }
                }
                Err(e) => {
                    println!("❌ Failed to connect to discovery service: {}", e);
                }
            }
        }

        Command::Providers {
            endpoint,
            region,
            tier,
            limit,
        } => {
            println!("👥 Fetching providers from {}...", endpoint);

            let client = reqwest::Client::new();
            let mut providers_url = format!("{}/api/v1/providers?limit={}", endpoint, limit);

            if let Some(r) = region {
                providers_url.push_str(&format!("&region={}", r));
            }
            if let Some(t) = tier {
                providers_url.push_str(&format!("&tier={}", t));
            }

            match client.get(&providers_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(provider_list) => {
                                println!("✅ Registered Providers:");

                                if let Some(providers) =
                                    provider_list.get("providers").and_then(|p| p.as_array())
                                {
                                    if providers.is_empty() {
                                        println!("   No providers found matching criteria");
                                    } else {
                                        for (i, provider) in providers.iter().enumerate() {
                                            println!("   {}. Provider:", i + 1);

                                            if let Some(name) =
                                                provider.get("name").and_then(|n| n.as_str())
                                            {
                                                println!("      Name: {}", name);
                                            }
                                            if let Some(id) =
                                                provider.get("id").and_then(|i| i.as_str())
                                            {
                                                println!("      ID: {}", id);
                                            }
                                            if let Some(region) =
                                                provider.get("region").and_then(|r| r.as_str())
                                            {
                                                println!("      Region: {}", region);
                                            }
                                            if let Some(tier) =
                                                provider.get("tier").and_then(|t| t.as_str())
                                            {
                                                println!("      Tier: {}", tier);
                                            }
                                            if let Some(price) = provider
                                                .get("price_per_gb_month")
                                                .and_then(|p| p.as_str())
                                            {
                                                println!("      Price: ${}/GB/month", price);
                                            }
                                            if let Some(capacity) = provider
                                                .get("total_capacity")
                                                .and_then(|c| c.as_u64())
                                            {
                                                println!(
                                                    "      Capacity: {:.2} GB",
                                                    capacity as f64 / (1024.0 * 1024.0 * 1024.0)
                                                );
                                            }

                                            if let Some(reputation) = provider
                                                .get("reputation")
                                                .and_then(|r| r.get("overall"))
                                                .and_then(|o| o.as_str())
                                            {
                                                println!("      Reputation: {}", reputation);
                                            }

                                            println!();
                                        }

                                        if let Some(total) = provider_list
                                            .get("total_count")
                                            .and_then(|t| t.as_u64())
                                        {
                                            println!("   Total providers: {}", total);
                                        }
                                        if let Some(has_more) =
                                            provider_list.get("has_more").and_then(|h| h.as_bool())
                                        {
                                            if has_more {
                                                println!(
                                                    "   (More providers available - increase \
                                                     limit to see more)"
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    println!("   Invalid response format");
                                }
                            }
                            Err(e) => {
                                println!("❌ Failed to parse provider list: {}", e);
                            }
                        }
                    } else {
                        println!("❌ Discovery service returned error: {}", response.status());
                    }
                }
                Err(e) => {
                    println!("❌ Failed to connect to discovery service: {}", e);
                }
            }
        }
    }

    Ok(())
}
