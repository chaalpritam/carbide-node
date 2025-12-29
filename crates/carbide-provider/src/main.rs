//! # Carbide Provider Node
//!
//! The storage provider binary that allows anyone to earn money by contributing
//! storage capacity to the Carbide Network.

use carbide_core::{Provider, ProviderTier, Region};
use carbide_provider::{ProviderServer, ServerConfig};
use clap::Parser;
use rust_decimal::Decimal;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "carbide-provider")]
#[command(about = "Carbide Network Storage Provider - Earn money by providing storage")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Initialize a new provider node
    Init {
        /// Path to storage directory
        #[arg(long)]
        storage_path: String,
        /// Available storage capacity (e.g., "1TB", "500GB")
        #[arg(long)]
        capacity: String,
        /// Provider tier (home, professional, enterprise, globalcdn)
        #[arg(long, default_value = "home")]
        tier: String,
        /// Provider region
        #[arg(long, default_value = "northamerica")]
        region: String,
    },
    /// Start the provider node
    Start {
        /// Provider name
        #[arg(long, default_value = "My Storage Provider")]
        name: String,
        /// Price per GB per month in USD
        #[arg(long, default_value = "0.002")]
        price_per_gb_month: f64,
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Provider tier (home, professional, enterprise, globalcdn)
        #[arg(long, default_value = "home")]
        tier: String,
        /// Provider region
        #[arg(long, default_value = "northamerica")]
        region: String,
        /// Available storage capacity in GB
        #[arg(long, default_value = "100")]
        capacity_gb: u64,
    },
    /// Show provider status
    Status {
        /// Provider API endpoint
        #[arg(long, default_value = "http://localhost:8080")]
        endpoint: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init {
            storage_path,
            capacity,
            tier,
            region,
        } => {
            println!("🚀 Initializing Carbide Provider...");
            println!("   Storage Path: {}", storage_path);
            println!("   Capacity: {}", capacity);
            println!("   Tier: {}", tier);
            println!("   Region: {}", region);
            
            // TODO: Create storage directory, generate provider config
            // TODO: Save configuration to disk
            
            println!("✅ Provider initialized successfully!");
        }
        Command::Start {
            name,
            price_per_gb_month,
            port,
            tier,
            region,
            capacity_gb,
        } => {
            println!("🏪 Starting Carbide Provider...");
            println!("   Name: {}", name);
            println!("   Price: ${:.4}/GB/month", price_per_gb_month);
            println!("   Tier: {}", tier);
            println!("   Region: {}", region);
            println!("   Capacity: {}GB", capacity_gb);
            println!("   Port: {}", port);

            // Parse tier and region
            let provider_tier = parse_tier(&tier)?;
            let provider_region = parse_region(&region)?;
            
            // Create provider instance
            let capacity_bytes = capacity_gb * 1024 * 1024 * 1024; // Convert GB to bytes
            let price = Decimal::new((price_per_gb_month * 1000.0) as i64, 3);
            let endpoint = format!("http://localhost:{}", port);
            
            let provider = Provider::new(
                name,
                provider_tier,
                provider_region,
                endpoint,
                capacity_bytes,
                price,
            );

            // Create server configuration
            let config = ServerConfig {
                host: "0.0.0.0".to_string(),
                port,
                request_timeout: Duration::from_secs(30),
                max_upload_size: 100 * 1024 * 1024, // 100MB
                enable_cors: true,
            };

            // Create and start the server
            let server = ProviderServer::new(config, provider)?;
            
            // Start server in background task
            let server_handle = tokio::spawn(async move {
                if let Err(e) = server.start().await {
                    eprintln!("❌ Server error: {}", e);
                }
            });

            // Wait for shutdown signal
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("🛑 Received shutdown signal, stopping provider...");
                }
                _ = server_handle => {
                    println!("🛑 Server stopped unexpectedly");
                }
            }
            
            println!("✅ Provider shut down gracefully");
        }
        Command::Status { endpoint } => {
            println!("📊 Checking Provider Status at {}...", endpoint);
            
            // Make HTTP request to provider's status endpoint
            let client = reqwest::Client::new();
            let status_url = format!("{}/api/v1/provider/status", endpoint);
            
            match client.get(&status_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => {
                                println!("✅ Provider is online:");
                                println!("{}", serde_json::to_string_pretty(&json)?);
                            }
                            Err(e) => {
                                println!("❌ Failed to parse response: {}", e);
                            }
                        }
                    } else {
                        println!("❌ Provider returned error: {}", response.status());
                    }
                }
                Err(e) => {
                    println!("❌ Failed to connect to provider: {}", e);
                }
            }
        }
    }

    Ok(())
}

/// Parse provider tier from string
fn parse_tier(tier: &str) -> anyhow::Result<ProviderTier> {
    match tier.to_lowercase().as_str() {
        "home" => Ok(ProviderTier::Home),
        "professional" => Ok(ProviderTier::Professional),
        "enterprise" => Ok(ProviderTier::Enterprise),
        "globalcdn" => Ok(ProviderTier::GlobalCDN),
        _ => Err(anyhow::anyhow!("Invalid tier: {}. Valid options: home, professional, enterprise, globalcdn", tier)),
    }
}

/// Parse provider region from string
fn parse_region(region: &str) -> anyhow::Result<Region> {
    match region.to_lowercase().as_str() {
        "northamerica" => Ok(Region::NorthAmerica),
        "europe" => Ok(Region::Europe),
        "asia" => Ok(Region::Asia),
        "southamerica" => Ok(Region::SouthAmerica),
        "africa" => Ok(Region::Africa),
        "oceania" => Ok(Region::Oceania),
        _ => Err(anyhow::anyhow!("Invalid region: {}. Valid options: northamerica, europe, asia, southamerica, africa, oceania", region)),
    }
}
