//! Provider configuration management
//!
//! Configuration is resolved with the following priority (highest first):
//! 1. Environment variables (CARBIDE_*)
//! 2. TOML config file
//! 3. Built-in defaults

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthSection,
    /// TLS configuration
    #[serde(default)]
    pub tls: TlsSection,
    /// Wallet and blockchain configuration
    #[serde(default)]
    pub wallet: WalletSection,
    /// Proof-of-storage scheduler configuration
    #[serde(default)]
    pub proof: ProofSection,
}

/// TLS configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsSection {
    /// Enable TLS (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Path to PEM certificate file
    #[serde(default = "default_cert_path")]
    pub cert_path: PathBuf,
    /// Path to PEM private key file
    #[serde(default = "default_key_path")]
    pub key_path: PathBuf,
    /// Auto-generate self-signed certificate if missing (default: true)
    #[serde(default = "default_auto_generate")]
    pub auto_generate: bool,
}

fn default_cert_path() -> PathBuf {
    PathBuf::from("certs/server.crt")
}

fn default_key_path() -> PathBuf {
    PathBuf::from("certs/server.key")
}

fn default_auto_generate() -> bool {
    true
}

impl Default for TlsSection {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: default_cert_path(),
            key_path: default_key_path(),
            auto_generate: default_auto_generate(),
        }
    }
}

/// Authentication configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSection {
    /// Enable authentication (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// JWT secret for verifying Bearer tokens
    #[serde(default)]
    pub jwt_secret: Option<String>,
    /// SHA-256 hashes of accepted API keys
    #[serde(default)]
    pub api_key_hashes: Vec<String>,
}

impl Default for AuthSection {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            api_key_hashes: Vec::new(),
        }
    }
}

/// Wallet and blockchain configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSection {
    /// Path to encrypted wallet file
    #[serde(default = "default_wallet_path")]
    pub wallet_path: PathBuf,
    /// Chain ID (421614 = Arbitrum Sepolia, 42161 = Arbitrum One)
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,
    /// JSON-RPC URL for the target chain
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,
    /// CarbideEscrow contract address (empty = blockchain features disabled)
    #[serde(default)]
    pub escrow_address: String,
    /// USDC token contract address
    #[serde(default)]
    pub usdc_address: String,
}

fn default_wallet_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".carbide/wallet/wallet.json")
}

fn default_chain_id() -> u64 {
    421614 // Arbitrum Sepolia
}

fn default_rpc_url() -> String {
    "https://sepolia-rollup.arbitrum.io/rpc".to_string()
}

impl Default for WalletSection {
    fn default() -> Self {
        Self {
            wallet_path: default_wallet_path(),
            chain_id: default_chain_id(),
            rpc_url: default_rpc_url(),
            escrow_address: String::new(),
            usdc_address: String::new(),
        }
    }
}

/// Proof-of-storage scheduler configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSection {
    /// Interval between proof rounds in seconds (default 21600 = 6 hours)
    #[serde(default = "default_proof_interval")]
    pub interval_secs: u64,
    /// Fraction of file to sample per proof (0.0–1.0, default 0.1 = 10%)
    #[serde(default = "default_challenge_percentage")]
    pub challenge_percentage: f64,
}

fn default_proof_interval() -> u64 {
    21600
}

fn default_challenge_percentage() -> f64 {
    0.1
}

impl Default for ProofSection {
    fn default() -> Self {
        Self {
            interval_secs: default_proof_interval(),
            challenge_percentage: default_challenge_percentage(),
        }
    }
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
    /// Heartbeat interval in seconds (default 60)
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
}

fn default_heartbeat_interval() -> u64 {
    60
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
                name: format!(
                    "{}-carbide-provider",
                    gethostname::gethostname().to_string_lossy()
                ),
                tier: "Home".to_string(),
                region: "NorthAmerica".to_string(),
                port: 8080,
                storage_path: carbide_home.join("data/storage"),
                max_storage_gb: 25,
            },
            network: NetworkSection {
                discovery_endpoint: "https://discovery.carbidenetwork.xyz".to_string(),
                advertise_address: "127.0.0.1:8080".to_string(),
                heartbeat_interval_secs: 60,
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
            auth: AuthSection::default(),
            tls: TlsSection::default(),
            wallet: WalletSection::default(),
            proof: ProofSection::default(),
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

    /// Load configuration from TOML file, then apply environment variable overrides.
    ///
    /// Supported environment variables:
    /// - `CARBIDE_PROVIDER_NAME` - Provider display name
    /// - `CARBIDE_PROVIDER_PORT` - Port to listen on
    /// - `CARBIDE_PROVIDER_TIER` - Provider tier
    /// - `CARBIDE_PROVIDER_REGION` - Geographic region
    /// - `CARBIDE_STORAGE_PATH` - Storage directory path
    /// - `CARBIDE_MAX_STORAGE_GB` - Maximum storage in GB
    /// - `CARBIDE_DISCOVERY_ENDPOINT` - Discovery service URL
    /// - `CARBIDE_ADVERTISE_ADDRESS` - Address advertised to clients
    /// - `CARBIDE_PRICE_PER_GB` - Price per GB per month (USD)
    /// - `CARBIDE_LOG_LEVEL` - Log level (debug, info, warn, error)
    /// - `CARBIDE_LOG_FILE` - Log file path
    pub async fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let mut config = Self::load_from_file(path).await?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Apply environment variable overrides to the current config.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("CARBIDE_PROVIDER_NAME") {
            self.provider.name = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_PROVIDER_PORT") {
            if let Ok(port) = v.parse::<u16>() {
                self.provider.port = port;
            }
        }
        if let Ok(v) = std::env::var("CARBIDE_PROVIDER_TIER") {
            self.provider.tier = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_PROVIDER_REGION") {
            self.provider.region = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_STORAGE_PATH") {
            self.provider.storage_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("CARBIDE_MAX_STORAGE_GB") {
            if let Ok(gb) = v.parse::<u64>() {
                self.provider.max_storage_gb = gb;
            }
        }
        if let Ok(v) = std::env::var("CARBIDE_DISCOVERY_ENDPOINT") {
            self.network.discovery_endpoint = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_ADVERTISE_ADDRESS") {
            self.network.advertise_address = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_HEARTBEAT_INTERVAL") {
            if let Ok(secs) = v.parse::<u64>() {
                self.network.heartbeat_interval_secs = secs;
            }
        }
        if let Ok(v) = std::env::var("CARBIDE_PRICE_PER_GB") {
            if let Ok(price) = v.parse::<f64>() {
                self.pricing.price_per_gb_month = price;
            }
        }
        if let Ok(v) = std::env::var("CARBIDE_LOG_LEVEL") {
            self.logging.level = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_LOG_FILE") {
            self.logging.file = PathBuf::from(v);
        }

        // Auth overrides
        if let Ok(v) = std::env::var("CARBIDE_AUTH_ENABLED") {
            self.auth.enabled = v == "true" || v == "1";
        }
        if let Ok(v) = std::env::var("CARBIDE_AUTH_JWT_SECRET") {
            self.auth.jwt_secret = Some(v);
        }
        if let Ok(v) = std::env::var("CARBIDE_AUTH_API_KEY_HASHES") {
            self.auth.api_key_hashes = v.split(',').map(|s| s.trim().to_string()).collect();
        }

        // TLS overrides
        if let Ok(v) = std::env::var("CARBIDE_TLS_ENABLED") {
            self.tls.enabled = v == "true" || v == "1";
        }
        if let Ok(v) = std::env::var("CARBIDE_TLS_CERT_PATH") {
            self.tls.cert_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("CARBIDE_TLS_KEY_PATH") {
            self.tls.key_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("CARBIDE_TLS_AUTO_GENERATE") {
            self.tls.auto_generate = v == "true" || v == "1";
        }

        // Wallet overrides
        if let Ok(v) = std::env::var("CARBIDE_WALLET_PATH") {
            self.wallet.wallet_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("CARBIDE_CHAIN_ID") {
            if let Ok(id) = v.parse::<u64>() {
                self.wallet.chain_id = id;
            }
        }
        if let Ok(v) = std::env::var("CARBIDE_RPC_URL") {
            self.wallet.rpc_url = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_ESCROW_ADDRESS") {
            self.wallet.escrow_address = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_USDC_ADDRESS") {
            self.wallet.usdc_address = v;
        }

        // Proof scheduler overrides
        if let Ok(v) = std::env::var("CARBIDE_PROOF_INTERVAL") {
            if let Ok(secs) = v.parse::<u64>() {
                self.proof.interval_secs = secs;
            }
        }
        if let Ok(v) = std::env::var("CARBIDE_PROOF_CHALLENGE_PCT") {
            if let Ok(pct) = v.parse::<f64>() {
                self.proof.challenge_percentage = pct;
            }
        }
    }

    /// Validate configuration values before starting the server.
    ///
    /// Catches misconfigurations early (port 0, zero capacity, missing secrets)
    /// instead of crashing at runtime with confusing errors.
    pub fn validate(&self) -> anyhow::Result<()> {
        // Provider basics
        if self.provider.name.trim().is_empty() {
            anyhow::bail!("provider.name must not be empty");
        }
        if self.provider.max_storage_gb == 0 {
            anyhow::bail!("provider.max_storage_gb must be greater than 0");
        }
        if self.provider.storage_path.as_os_str().is_empty() {
            anyhow::bail!("provider.storage_path must not be empty");
        }

        // Pricing
        if self.pricing.price_per_gb_month < 0.0 {
            anyhow::bail!("pricing.price_per_gb_month must be >= 0");
        }

        // Network
        if self.network.heartbeat_interval_secs < 10 {
            anyhow::bail!(
                "network.heartbeat_interval_secs must be >= 10 (got {})",
                self.network.heartbeat_interval_secs
            );
        }

        // Auth: if enabled, a JWT secret is required
        if self.auth.enabled && self.auth.jwt_secret.is_none() {
            anyhow::bail!(
                "auth.enabled is true but auth.jwt_secret is not set. \
                 Set CARBIDE_AUTH_JWT_SECRET or add jwt_secret to [auth] in config."
            );
        }

        // TLS: if enabled without auto-generate, cert and key files must exist
        if self.tls.enabled && !self.tls.auto_generate {
            if !self.tls.cert_path.exists() {
                anyhow::bail!(
                    "tls.enabled is true and auto_generate is false, but cert_path does not exist: {}",
                    self.tls.cert_path.display()
                );
            }
            if !self.tls.key_path.exists() {
                anyhow::bail!(
                    "tls.enabled is true and auto_generate is false, but key_path does not exist: {}",
                    self.tls.key_path.display()
                );
            }
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProviderConfig::default();
        assert_eq!(config.provider.port, 8080);
        assert_eq!(config.provider.max_storage_gb, 25);
        assert_eq!(config.pricing.price_per_gb_month, 0.005);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_env_override() {
        // Set env vars for the test
        std::env::set_var("CARBIDE_PROVIDER_PORT", "9999");
        std::env::set_var("CARBIDE_LOG_LEVEL", "debug");

        let mut config = ProviderConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.provider.port, 9999);
        assert_eq!(config.logging.level, "debug");

        // Clean up
        std::env::remove_var("CARBIDE_PROVIDER_PORT");
        std::env::remove_var("CARBIDE_LOG_LEVEL");
    }

    #[test]
    fn test_validate_default_config_passes() {
        let config = ProviderConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let mut config = ProviderConfig::default();
        config.provider.name = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_capacity() {
        let mut config = ProviderConfig::default();
        config.provider.max_storage_gb = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_negative_price() {
        let mut config = ProviderConfig::default();
        config.pricing.price_per_gb_month = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_low_heartbeat() {
        let mut config = ProviderConfig::default();
        config.network.heartbeat_interval_secs = 5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_auth_enabled_no_secret() {
        let mut config = ProviderConfig::default();
        config.auth.enabled = true;
        config.auth.jwt_secret = None;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_auth_enabled_with_secret() {
        let mut config = ProviderConfig::default();
        config.auth.enabled = true;
        config.auth.jwt_secret = Some("my-secret".to_string());
        assert!(config.validate().is_ok());
    }
}
