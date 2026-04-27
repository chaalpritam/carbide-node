//! # Carbide Client CLI
//!
//! Command-line interface for interacting with the Carbide Network

use std::path::PathBuf;

use carbide_client::file_registry::FileRegistry;
use carbide_client::payment::{CreateContractRequest, PaymentClient};
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
    /// Payment and contract commands
    Pay {
        #[command(subcommand)]
        action: PayAction,
    },
    /// File registry commands
    Files {
        #[command(subcommand)]
        action: FilesAction,
    },
}

#[derive(Parser)]
enum PayAction {
    /// Create a storage contract
    CreateContract {
        /// Provider ID
        #[arg(long)]
        provider_id: String,
        /// Client ID
        #[arg(long)]
        client_id: String,
        /// Price per GB per month
        #[arg(long)]
        price: String,
        /// Duration in days
        #[arg(long, default_value = "30")]
        duration_days: u32,
        /// Discovery service endpoint
        #[arg(long, default_value = "http://localhost:3000")]
        discovery_endpoint: String,
    },
    /// Record a deposit on a contract
    RecordDeposit {
        /// Contract ID
        #[arg(long)]
        contract_id: String,
        /// Deposit amount
        #[arg(long)]
        amount: String,
        /// Discovery service endpoint
        #[arg(long, default_value = "http://localhost:3000")]
        discovery_endpoint: String,
    },
    /// List contracts
    ListContracts {
        /// Filter by client ID
        #[arg(long)]
        client_id: Option<String>,
        /// Discovery service endpoint
        #[arg(long, default_value = "http://localhost:3000")]
        discovery_endpoint: String,
    },
    /// Show contract details
    ShowContract {
        /// Contract ID
        #[arg(long)]
        contract_id: String,
        /// Discovery service endpoint
        #[arg(long, default_value = "http://localhost:3000")]
        discovery_endpoint: String,
    },
}

#[derive(Parser)]
enum FilesAction {
    /// List stored files
    List {
        /// Filter by status (active, expired, deleted)
        #[arg(long)]
        status: Option<String>,
        /// Path to the file registry database
        #[arg(long, default_value = ".carbide/files.db")]
        db_path: PathBuf,
    },
    /// Show details of a specific file
    Show {
        /// File ID (content hash hex)
        #[arg(long)]
        file_id: String,
        /// Path to the file registry database
        #[arg(long, default_value = ".carbide/files.db")]
        db_path: PathBuf,
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

        Command::Files { action } => match action {
            FilesAction::List { status, db_path } => {
                let registry = FileRegistry::open(&db_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open registry: {}", e))?;
                let files = registry
                    .list_files(status.as_deref())
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                if files.is_empty() {
                    println!("No files found.");
                } else {
                    println!("Files ({}):", files.len());
                    for f in &files {
                        println!(
                            "  {} | {} | {} bytes | {}",
                            f.file_id, f.original_name, f.file_size, f.status
                        );
                    }
                }
            }
            FilesAction::Show { file_id, db_path } => {
                let registry = FileRegistry::open(&db_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open registry: {}", e))?;
                match registry.get_file(&file_id) {
                    Ok(Some(file)) => {
                        println!("{}", serde_json::to_string_pretty(&file)?);
                    }
                    Ok(None) => {
                        println!("File not found: {}", file_id);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        },

        Command::Pay { action } => match action {
            PayAction::CreateContract {
                provider_id,
                client_id,
                price,
                duration_days,
                discovery_endpoint,
            } => {
                let payment = PaymentClient::new(&discovery_endpoint)?;
                let contract = payment
                    .create_contract(&CreateContractRequest {
                        provider_id,
                        client_id,
                        price_per_gb_month: price,
                        duration_days,
                        total_size_bytes: None,
                        file_id: None,
                        chain_id: None,
                    })
                    .await?;
                println!("Contract created:");
                println!("{}", serde_json::to_string_pretty(&contract)?);
            }
            PayAction::RecordDeposit {
                contract_id,
                amount,
                discovery_endpoint,
            } => {
                let payment = PaymentClient::new(&discovery_endpoint)?;
                let contract = payment.record_deposit(&contract_id, &amount, None).await?;
                println!("Deposit recorded:");
                println!("{}", serde_json::to_string_pretty(&contract)?);
            }
            PayAction::ListContracts {
                client_id,
                discovery_endpoint,
            } => {
                let payment = PaymentClient::new(&discovery_endpoint)?;
                let contracts = payment.list_contracts(client_id.as_deref()).await?;
                println!("Contracts ({}): ", contracts.len());
                for c in &contracts {
                    println!(
                        "  {} | {} | {} | {}",
                        c.id, c.status, c.price_per_gb_month, c.created_at
                    );
                }
            }
            PayAction::ShowContract {
                contract_id,
                discovery_endpoint,
            } => {
                let payment = PaymentClient::new(&discovery_endpoint)?;
                let contract = payment.get_contract(&contract_id).await?;
                println!("{}", serde_json::to_string_pretty(&contract)?);
            }
        },
    }

    Ok(())
}
