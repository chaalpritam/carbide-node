//! # Carbide Client CLI
//!
//! Command-line interface for interacting with the Carbide Network

use std::path::PathBuf;
use std::str::FromStr;

use carbide_client::file_registry::FileRegistry;
use carbide_client::payment::{CreateContractRequest, PaymentClient};
use carbide_client::wallet::ClientWallet;
use carbide_client::CarbideClient;
use carbide_core::network::*;
use carbide_core::{ContentHash, ProviderRequirements};
use clap::Parser;
use rust_decimal::Decimal;

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
    /// Solana wallet management
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
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
    /// Upload a file directly to a single provider (no discovery, no payment).
    /// Use this for a LAN/two-laptop demo where the provider endpoint is known.
    Upload {
        /// Provider endpoint, e.g. http://192.168.1.42:8080
        #[arg(long)]
        provider: String,
        /// Path to the file to upload
        #[arg(long)]
        file: PathBuf,
        /// Storage duration in months
        #[arg(long, default_value = "1")]
        duration_months: u32,
        /// Maximum price the client is willing to pay per GB-month (decimal)
        #[arg(long, default_value = "1.0")]
        max_price: String,
    },
    /// Download a file directly from a single provider by content hash.
    Download {
        /// Provider endpoint, e.g. http://192.168.1.42:8080
        #[arg(long)]
        provider: String,
        /// File ID (BLAKE3 content hash, 64-char hex) printed by `upload`
        #[arg(long)]
        file_id: String,
        /// Output path
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Parser)]
enum WalletAction {
    /// Create a new Solana wallet (Ed25519, BIP-44 path 501)
    Create {
        /// Directory to store the encrypted wallet file
        #[arg(long, default_value = ".carbide")]
        wallet_dir: PathBuf,
        /// Encryption password
        #[arg(long)]
        password: String,
    },
    /// Show the wallet's Solana address
    Show {
        /// Path to the encrypted wallet file
        #[arg(long, default_value = ".carbide/wallet.json")]
        wallet_path: PathBuf,
        /// Decryption password
        #[arg(long)]
        password: String,
    },
    /// Import a wallet from a 12-word BIP-39 mnemonic
    Import {
        /// 12-word recovery phrase
        #[arg(long)]
        mnemonic: String,
        /// Directory to store the encrypted wallet file
        #[arg(long, default_value = ".carbide")]
        wallet_dir: PathBuf,
        /// Encryption password
        #[arg(long)]
        password: String,
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

        Command::Wallet { action } => match action {
            WalletAction::Create {
                wallet_dir,
                password,
            } => {
                println!("Creating new Solana wallet in {:?}...", wallet_dir);
                let (wallet, mnemonic) = ClientWallet::create(&wallet_dir, &password)?;
                println!("Wallet created!");
                println!("  Address: {}", wallet.address_base58());
                println!("  Saved to: {:?}", wallet.path());
                println!();
                println!("IMPORTANT — save your 12-word recovery phrase:");
                println!("  {}", mnemonic);
                println!();
                println!("This phrase is the ONLY way to recover the wallet if you lose the password.");
            }
            WalletAction::Show {
                wallet_path,
                password,
            } => {
                let wallet = ClientWallet::load(&wallet_path, &password)?;
                println!("Wallet address: {}", wallet.address_base58());
            }
            WalletAction::Import {
                mnemonic,
                wallet_dir,
                password,
            } => {
                println!("Importing wallet from mnemonic into {:?}...", wallet_dir);
                let wallet =
                    ClientWallet::import_from_mnemonic(&wallet_dir, &mnemonic, &password)?;
                println!("Wallet imported!");
                println!("  Address: {}", wallet.address_base58());
                println!("  Saved to: {:?}", wallet.path());
            }
        },

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

        Command::Upload {
            provider,
            file,
            duration_months,
            max_price,
        } => {
            let data = std::fs::read(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read {:?}: {}", file, e))?;
            let file_size = data.len() as u64;
            let file_id = ContentHash::from_data(&data);
            let max_price_decimal = Decimal::from_str(&max_price)
                .map_err(|e| anyhow::anyhow!("Invalid --max-price '{}': {}", max_price, e))?;

            let provider = provider.trim_end_matches('/').to_string();

            println!("Uploading {:?} ({} bytes) to {}", file, file_size, provider);
            println!("  file_id: {}", file_id.to_hex());

            let store_request = StoreFileRequest {
                file_id,
                file_size,
                duration_months,
                encryption_info: None,
                requirements: ProviderRequirements::important(),
                max_price: max_price_decimal,
            };

            let store_response = client.store_file(&provider, &store_request).await?;
            if !store_response.accepted {
                let reason = store_response
                    .rejection_reason
                    .unwrap_or_else(|| "no reason given".to_string());
                anyhow::bail!("Provider rejected store request: {}", reason);
            }
            let upload_url = store_response
                .upload_url
                .ok_or_else(|| anyhow::anyhow!("Provider accepted but returned no upload_url"))?;
            let upload_token = store_response
                .upload_token
                .ok_or_else(|| anyhow::anyhow!("Provider accepted but returned no upload_token"))?;

            client
                .upload_file(&upload_url, &file_id, &data, &upload_token)
                .await?;

            println!("✅ Upload complete.");
            println!("   file_id: {}", file_id.to_hex());
            println!(
                "   download with: carbide-client download --provider {} --file-id {} --out <path>",
                provider,
                file_id.to_hex()
            );
        }

        Command::Download {
            provider,
            file_id,
            out,
        } => {
            let provider = provider.trim_end_matches('/').to_string();
            let _ = ContentHash::from_hex(&file_id)
                .map_err(|e| anyhow::anyhow!("Invalid --file-id '{}': {}", file_id, e))?;

            let url = format!("{}{}/{}", provider, ApiEndpoints::FILE_DOWNLOAD, file_id);
            println!("Downloading {} from {}", file_id, provider);

            let response = reqwest::get(&url)
                .await
                .map_err(|e| anyhow::anyhow!("Download request failed: {}", e))?;
            if !response.status().is_success() {
                anyhow::bail!("Provider returned status {} for {}", response.status(), url);
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read body: {}", e))?;

            std::fs::write(&out, &bytes)
                .map_err(|e| anyhow::anyhow!("Failed to write {:?}: {}", out, e))?;
            println!("✅ Wrote {} bytes to {:?}", bytes.len(), out);
        }

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
