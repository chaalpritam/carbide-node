//! # Carbide Client CLI
//!
//! Command-line interface for interacting with the Carbide Network

use carbide_client::CarbideClient;
use carbide_core::network::*;
use clap::Parser;

#[derive(Parser)]
#[command(name = "carbide-client")]
#[command(
    about = "Carbide Network Client - Store and retrieve files from the decentralized network"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Test connectivity to providers
    Test {
        /// Provider endpoints to test (comma-separated)
        #[arg(long)]
        providers: String,
    },
    /// Get provider health status
    Health {
        /// Provider endpoint
        #[arg(long)]
        endpoint: String,
    },
    /// Get provider status details
    Status {
        /// Provider endpoint
        #[arg(long)]
        endpoint: String,
    },
    /// Request storage quote from providers
    Quote {
        /// File size in bytes
        #[arg(long)]
        file_size: u64,
        /// Replication factor (1-10)
        #[arg(long, default_value = "3")]
        replication: u8,
        /// Storage duration in months
        #[arg(long, default_value = "12")]
        duration: u32,
        /// Provider endpoints (comma-separated)
        #[arg(long)]
        providers: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Create client with default config
    let client = CarbideClient::with_defaults()?;

    match cli.command {
        Command::Test { providers } => {
            println!("🧪 Testing Provider Connectivity...");

            let endpoints: Vec<String> =
                providers.split(',').map(|s| s.trim().to_string()).collect();

            let results = client.test_providers(&endpoints).await;

            println!("\n📊 Test Results:");
            for result in results {
                let status_icon = if result.online { "✅" } else { "❌" };
                println!(
                    "  {} {} ({} ms)",
                    status_icon, result.endpoint, result.latency_ms
                );

                if let Some(error) = result.error {
                    println!("     Error: {}", error);
                } else {
                    println!("     Status: {:?}", result.status);
                }
            }
        }

        Command::Health { endpoint } => {
            println!("🏥 Checking Provider Health: {}", endpoint);

            match client.get_provider_health(&endpoint).await {
                Ok(health) => {
                    println!("✅ Provider is healthy!");
                    println!("   Status: {:?}", health.status);
                    println!("   Version: {}", health.version);

                    if let Some(storage) = health.available_storage {
                        println!(
                            "   Available Storage: {:.2} GB",
                            storage as f64 / (1024.0 * 1024.0 * 1024.0)
                        );
                    }

                    if let Some(load) = health.load {
                        println!("   Load: {:.1}%", load * 100.0);
                    }

                    if let Some(reputation) = health.reputation {
                        println!("   Reputation: {:.2}/1.0", reputation);
                    }
                }
                Err(e) => {
                    println!("❌ Health check failed: {}", e);
                }
            }
        }

        Command::Status { endpoint } => {
            println!("📊 Getting Provider Status: {}", endpoint);

            match client.get_provider_status(&endpoint).await {
                Ok(status) => {
                    println!("✅ Provider Status:");
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
                Err(e) => {
                    println!("❌ Status request failed: {}", e);
                }
            }
        }

        Command::Quote {
            file_size,
            replication,
            duration,
            providers,
        } => {
            println!("💰 Requesting Storage Quotes...");
            println!(
                "   File Size: {} bytes ({:.2} MB)",
                file_size,
                file_size as f64 / (1024.0 * 1024.0)
            );
            println!("   Replication: {} copies", replication);
            println!("   Duration: {} months", duration);

            let endpoints: Vec<String> =
                providers.split(',').map(|s| s.trim().to_string()).collect();

            let quote_request = StorageQuoteRequest {
                file_size,
                replication_factor: replication,
                duration_months: duration,
                requirements: carbide_core::ProviderRequirements::important(),
                preferred_regions: vec![],
            };

            println!("\n📋 Quotes from {} providers:", endpoints.len());

            for (i, endpoint) in endpoints.iter().enumerate() {
                print!("  {}. {} ... ", i + 1, endpoint);

                match client.request_storage_quote(endpoint, &quote_request).await {
                    Ok(quote) => {
                        println!("✅");
                        println!("     Can fulfill: {}", quote.can_fulfill);
                        println!("     Price: ${}/GB/month", quote.price_per_gb_month);
                        println!("     Total cost: ${}/month", quote.total_monthly_cost);
                        println!(
                            "     Available capacity: {:.2} GB",
                            quote.available_capacity as f64 / (1024.0 * 1024.0 * 1024.0)
                        );
                        println!("     Start time: {} hours", quote.estimated_start_time);
                        println!(
                            "     Valid until: {}",
                            quote.valid_until.format("%Y-%m-%d %H:%M:%S")
                        );
                    }
                    Err(e) => {
                        println!("❌ {}", e);
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}
