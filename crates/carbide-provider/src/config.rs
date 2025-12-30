//! Provider configuration management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Provider configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider-specific configuration
    pub provider: ProviderSection,
    /// Network configuration
    pub network: NetworkSection,
    /// Pricing configuration
    pub pricing: PricingSection,
    /// Logging configuration
    pub logging: LoggingSection,
    /// Reputation system configuration
    pub reputation: ReputationSection,
}

/// Provider-specific configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSection {
    /// Display name for this provider
    pub name: String,
    /// Provider tier (Home, Professional, Enterprise, GlobalCDN)
    pub tier: String,
    /// Geographic region
    pub region: String,
    /// Port number to listen on
    pub port: u16,
    /// Path to storage directory
    pub storage_path: PathBuf,
    /// Maximum storage allocation in GB
    pub max_storage_gb: u64,
}

/// Network configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    /// Discovery service endpoint URL
    pub discovery_endpoint: String,
    /// Address to advertise to clients
    pub advertise_address: String,
}

/// Pricing configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingSection {
    /// Price per GB per month in USD
    pub price_per_gb_month: f64,
}

/// Logging configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSection {
    /// Log level (debug, info, warn, error)
    pub level: String,
    /// Log file path
    pub file: PathBuf,
}

/// Reputation system configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationSection {
    /// Enable reporting to reputation system
    pub enable_reporting: bool,
    /// Health check interval in seconds
    pub health_check_interval: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let carbide_home = PathBuf::from(&home_dir).join(".carbide");
        
        Self {
            provider: ProviderSection {
                name: format!("{}-carbide-provider", gethostname::gethostname().to_string_lossy()),
                tier: "Home".to_string(),
                region: "NorthAmerica".to_string(),
                port: 8080,
                storage_path: carbide_home.join("data/storage"),
                max_storage_gb: 25,
            },
            network: NetworkSection {
                discovery_endpoint: "http://localhost:3000".to_string(),
                advertise_address: "127.0.0.1:8080".to_string(),
            },
            pricing: PricingSection {
                price_per_gb_month: 0.005,
            },
            logging: LoggingSection {
                level: "info".to_string(),
                file: carbide_home.join("logs/provider.log"),
            },
            reputation: ReputationSection {
                enable_reporting: true,
                health_check_interval: 300,
            },
        }
    }
}

impl ProviderConfig {
    /// Load configuration from TOML file
    pub async fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: ProviderConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to TOML file
    pub async fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(path, content).await?;
        Ok(())
    }
}