//! # Carbide Provider Node
//!
//! The storage provider binary that allows anyone to earn money by contributing
//! storage capacity to the Carbide Network.

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use carbide_core::{Provider, ProviderTier, Region};
use carbide_provider::{ProviderConfig, ProviderServer, ServerConfig};
use clap::Parser;
use rust_decimal::Decimal;

#[derive(Parser)]
#[command(name = "carbide-provider")]
#[command(about = "Carbide Network Storage Provider - Earn money by providing storage")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Configuration file path
    #[arg(long, short = 'c', global = true)]
    config: Option<PathBuf>,
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
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Check if config file provided
    if let Some(config_path) = &cli.config {
        return run_with_config(config_path).await;
    }

    // If no command provided, try to find default config
    let command = cli.command.unwrap_or_else(|| {
        // Look for default config in common locations
        let default_configs = [
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".carbide/config/provider.toml")),
            Some(PathBuf::from("/usr/local/etc/carbide/provider.toml")),
            Some(PathBuf::from("./provider.toml")),
        ];

        for config_path in default_configs.into_iter().flatten() {
            if config_path.exists() {
                println!("📁 Found config file: {}", config_path.display());
                // We need to handle this differently - let's create a special status command
                return Command::Status {
                    endpoint: config_path.to_string_lossy().to_string(),
                };
            }
        }

        println!("❌ No configuration found. Please provide --config or use a subcommand.");
        println!("💡 Try running: carbide-provider init --help");
        std::process::exit(1);
    });

    match command {
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

            // Parse capacity string (e.g. "25GB", "1TB")
            let max_storage_gb = parse_capacity(&capacity)?;

            // Create storage directory
            let storage = PathBuf::from(&storage_path);
            tokio::fs::create_dir_all(&storage)
                .await
                .with_context(|| format!("Failed to create storage directory: {}", storage_path))?;

            // Build default config with user-provided values
            let mut config = ProviderConfig::default();
            config.provider.storage_path = storage;
            config.provider.max_storage_gb = max_storage_gb;
            config.provider.tier = tier;
            config.provider.region = region;

            // Determine config file location
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let config_dir = PathBuf::from(&home_dir).join(".carbide/config");
            tokio::fs::create_dir_all(&config_dir)
                .await
                .with_context(|| "Failed to create config directory")?;

            let config_path = config_dir.join("provider.toml");
            config
                .save_to_file(&config_path)
                .await
                .with_context(|| "Failed to save provider configuration")?;

            println!("✅ Provider initialized successfully!");
            println!("   Config saved to: {}", config_path.display());
            println!("   Storage directory: {}", storage_path);
            println!("   Capacity: {}GB", max_storage_gb);
            println!();
            println!("💡 Start the provider with:");
            println!("   carbide-provider --config {}", config_path.display());
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
            let storage_path = PathBuf::from("./storage");
            let server = ProviderServer::new(config, provider, storage_path)?;

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
            // Check if this is actually a config file path (from our default config detection)
            let endpoint_path = PathBuf::from(&endpoint);
            if endpoint_path.exists() && endpoint_path.extension().is_some_and(|ext| ext == "toml")
            {
                return run_with_config(&endpoint_path).await;
            }

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

/// Run provider using configuration file
async fn run_with_config(config_path: &PathBuf) -> Result<()> {
    println!("🔧 Loading configuration from: {}", config_path.display());

    // Load configuration with env var overrides
    let config = ProviderConfig::load(config_path)
        .await
        .with_context(|| format!("Failed to load config from: {}", config_path.display()))?;

    // Create storage directory if it doesn't exist
    tokio::fs::create_dir_all(&config.provider.storage_path)
        .await
        .with_context(|| "Failed to create storage directory")?;

    // Create log directory
    if let Some(log_dir) = config.logging.file.parent() {
        tokio::fs::create_dir_all(log_dir)
            .await
            .with_context(|| "Failed to create log directory")?;
    }

    println!("🏪 Starting Carbide Provider...");
    println!("   Name: {}", config.provider.name);
    println!("   Tier: {}", config.provider.tier);
    println!("   Region: {}", config.provider.region);
    println!(
        "   Storage: {} ({}GB max)",
        config.provider.storage_path.display(),
        config.provider.max_storage_gb
    );
    println!(
        "   Price: ${:.4}/GB/month",
        config.pricing.price_per_gb_month
    );
    println!("   Port: {}", config.provider.port);

    // Parse tier and region
    let provider_tier = parse_tier(&config.provider.tier)?;
    let provider_region = parse_region(&config.provider.region)?;

    // Create provider instance
    let capacity_bytes = config.provider.max_storage_gb * 1024 * 1024 * 1024;
    let price = Decimal::new((config.pricing.price_per_gb_month * 1000.0) as i64, 3);
    let endpoint = format!("http://{}", config.network.advertise_address);

    let provider = Provider::new(
        config.provider.name.clone(),
        provider_tier,
        provider_region,
        endpoint,
        capacity_bytes,
        price,
    );

    // Create server configuration
    let server_config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: config.provider.port,
        request_timeout: Duration::from_secs(30),
        max_upload_size: 100 * 1024 * 1024, // 100MB
        enable_cors: true,
    };

    // Create and start the server
    let server = ProviderServer::new(
        server_config,
        provider,
        config.provider.storage_path.clone(),
    )?;

    // Start server in background task
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("❌ Server error: {}", e);
        }
    });

    // Health check reporting task (if enabled)
    let health_check_handle = if config.reputation.enable_reporting {
        let provider_name = config.provider.name.clone();
        let interval = Duration::from_secs(config.reputation.health_check_interval);

        Some(tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;

                // Report health status to discovery service
                // In a real implementation, this would send actual metrics
                tracing::info!("Health check: Provider '{}' is healthy", provider_name);
            }
        }))
    } else {
        None
    };

    println!("✅ Provider started successfully!");
    println!("🌐 Listening on: http://localhost:{}", config.provider.port);
    println!(
        "📊 Status endpoint: http://localhost:{}/api/v1/provider/status",
        config.provider.port
    );
    println!(
        "💾 Storage directory: {}",
        config.provider.storage_path.display()
    );
    println!("📝 Logs: {}", config.logging.file.display());
    println!();
    println!("🛑 Press Ctrl+C to stop the provider");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\\n🛑 Received shutdown signal, stopping provider...");
        }
        _ = server_handle => {
            println!("🛑 Server stopped unexpectedly");
        }
    }

    // Clean shutdown
    if let Some(health_handle) = health_check_handle {
        health_handle.abort();
    }

    println!("✅ Provider shut down gracefully");
    Ok(())
}

/// Parse provider tier from string
fn parse_tier(tier: &str) -> anyhow::Result<ProviderTier> {
    match tier.to_lowercase().as_str() {
        "home" => Ok(ProviderTier::Home),
        "professional" => Ok(ProviderTier::Professional),
        "enterprise" => Ok(ProviderTier::Enterprise),
        "globalcdn" => Ok(ProviderTier::GlobalCDN),
        _ => Err(anyhow::anyhow!(
            "Invalid tier: {}. Valid options: home, professional, enterprise, globalcdn",
            tier
        )),
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
        _ => Err(anyhow::anyhow!(
            "Invalid region: {}. Valid options: northamerica, europe, asia, southamerica, africa, \
             oceania",
            region
        )),
    }
}

/// Parse capacity string like "25GB", "1TB", "500MB" into GB
fn parse_capacity(capacity: &str) -> anyhow::Result<u64> {
    let capacity = capacity.trim().to_uppercase();

    if let Some(num) = capacity.strip_suffix("TB") {
        let val: u64 = num
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid capacity number: {}", num))?;
        Ok(val * 1024)
    } else if let Some(num) = capacity.strip_suffix("GB") {
        let val: u64 = num
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid capacity number: {}", num))?;
        Ok(val)
    } else if let Some(num) = capacity.strip_suffix("MB") {
        let val: u64 = num
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid capacity number: {}", num))?;
        if val < 1024 {
            anyhow::bail!("Minimum capacity is 1GB (1024MB), got {}MB", val);
        }
        Ok(val / 1024)
    } else {
        // Assume GB if no suffix
        let val: u64 = capacity.parse().map_err(|_| {
            anyhow::anyhow!(
                "Invalid capacity: {}. Use format like '25GB', '1TB'",
                capacity
            )
        })?;
        Ok(val)
    }
}
