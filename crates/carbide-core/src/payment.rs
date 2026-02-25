//! Payment types for the Carbide Network blockchain integration
//!
//! Defines payment status tracking, chain configuration, and payment
//! instruction types used across the escrow-based USDC payment system.

use serde::{Deserialize, Serialize};

/// Payment status for a storage contract
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// No payment configured
    None,
    /// Waiting for client to deposit USDC into escrow
    AwaitingDeposit,
    /// USDC deposited in escrow, storage can begin
    Deposited,
    /// Some monthly payments have been released to provider
    PartiallyReleased,
    /// All payments released, contract complete
    FullyReleased,
    /// Funds refunded to client (cancellation)
    Refunded,
    /// Payment is under dispute
    Disputed,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::AwaitingDeposit => write!(f, "awaiting_deposit"),
            Self::Deposited => write!(f, "deposited"),
            Self::PartiallyReleased => write!(f, "partially_released"),
            Self::FullyReleased => write!(f, "fully_released"),
            Self::Refunded => write!(f, "refunded"),
            Self::Disputed => write!(f, "disputed"),
        }
    }
}

impl PaymentStatus {
    /// Parse from string representation
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "awaiting_deposit" => Self::AwaitingDeposit,
            "deposited" => Self::Deposited,
            "partially_released" => Self::PartiallyReleased,
            "fully_released" => Self::FullyReleased,
            "refunded" => Self::Refunded,
            "disputed" => Self::Disputed,
            _ => Self::None,
        }
    }
}

/// Detailed payment information for a storage contract
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentInfo {
    /// Chain ID (e.g. 421614 for Arbitrum Sepolia)
    pub chain_id: u64,
    /// Escrow contract address
    pub escrow_contract: String,
    /// On-chain escrow identifier
    pub escrow_id: u64,
    /// USDC token contract address
    pub token_address: String,
    /// Total amount deposited (USDC with 6 decimals, as string)
    pub total_amount: String,
    /// Amount released to provider so far
    pub released_amount: String,
    /// Number of payment periods completed
    pub periods_paid: u32,
    /// Total payment periods in contract
    pub total_periods: u32,
    /// Transaction hashes for key events
    pub tx_hashes: Vec<String>,
}

/// Blockchain chain configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain ID
    pub chain_id: u64,
    /// Chain name
    pub name: String,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Block explorer URL
    pub explorer_url: String,
    /// USDC token contract address
    pub usdc_address: String,
    /// CarbideEscrow contract address
    pub escrow_address: String,
}

impl ChainConfig {
    /// Arbitrum Sepolia testnet configuration
    pub fn arbitrum_sepolia() -> Self {
        Self {
            chain_id: 421614,
            name: "Arbitrum Sepolia".to_string(),
            rpc_url: "https://sepolia-rollup.arbitrum.io/rpc".to_string(),
            explorer_url: "https://sepolia.arbiscan.io".to_string(),
            usdc_address: String::new(), // Set after deployment
            escrow_address: String::new(), // Set after deployment
        }
    }

    /// Load chain configuration from environment variables.
    ///
    /// Reads: `CARBIDE_CHAIN_ID`, `CARBIDE_RPC_URL`, `CARBIDE_ESCROW_ADDRESS`,
    /// `CARBIDE_USDC_ADDRESS`. Falls back to Arbitrum Sepolia defaults for
    /// chain_id, name, rpc_url, and explorer_url if not set.
    pub fn from_env() -> Self {
        let chain_id: u64 = std::env::var("CARBIDE_CHAIN_ID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(421614);

        let rpc_url = std::env::var("CARBIDE_RPC_URL")
            .unwrap_or_else(|_| "https://sepolia-rollup.arbitrum.io/rpc".to_string());

        let escrow_address = std::env::var("CARBIDE_ESCROW_ADDRESS")
            .unwrap_or_default();

        let usdc_address = std::env::var("CARBIDE_USDC_ADDRESS")
            .unwrap_or_default();

        let (name, explorer_url) = match chain_id {
            42161 => ("Arbitrum One".to_string(), "https://arbiscan.io".to_string()),
            421614 => ("Arbitrum Sepolia".to_string(), "https://sepolia.arbiscan.io".to_string()),
            _ => (format!("Chain {}", chain_id), String::new()),
        };

        Self {
            chain_id,
            name,
            rpc_url,
            explorer_url,
            usdc_address,
            escrow_address,
        }
    }

    /// Arbitrum One mainnet configuration
    pub fn arbitrum_one() -> Self {
        Self {
            chain_id: 42161,
            name: "Arbitrum One".to_string(),
            rpc_url: "https://arb1.arbitrum.io/rpc".to_string(),
            explorer_url: "https://arbiscan.io".to_string(),
            usdc_address: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string(),
            escrow_address: String::new(), // Set after deployment
        }
    }
}

/// Payment instructions sent to client after storage request is accepted
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentInstructions {
    /// CarbideEscrow contract address to call createEscrow()
    pub escrow_contract: String,
    /// Provider's Ethereum address (payee)
    pub provider_address: String,
    /// USDC token address for approval + deposit
    pub token_address: String,
    /// Total USDC amount to deposit (with 6 decimals, as string)
    pub total_amount: String,
    /// Chain ID to use
    pub chain_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_config_from_env() {
        // Set known env vars, test them, then clean up — all in one test
        // to avoid parallel test env-var races.
        std::env::set_var("CARBIDE_CHAIN_ID", "42161");
        std::env::set_var("CARBIDE_RPC_URL", "https://arb1.example.com/rpc");
        std::env::set_var("CARBIDE_ESCROW_ADDRESS", "0x1234567890abcdef1234567890abcdef12345678");
        std::env::set_var("CARBIDE_USDC_ADDRESS", "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd");

        let config = ChainConfig::from_env();
        assert_eq!(config.chain_id, 42161);
        assert_eq!(config.name, "Arbitrum One");
        assert_eq!(config.rpc_url, "https://arb1.example.com/rpc");
        assert_eq!(config.explorer_url, "https://arbiscan.io");
        assert_eq!(config.escrow_address, "0x1234567890abcdef1234567890abcdef12345678");
        assert_eq!(config.usdc_address, "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd");

        // Clean up and test defaults
        std::env::remove_var("CARBIDE_CHAIN_ID");
        std::env::remove_var("CARBIDE_RPC_URL");
        std::env::remove_var("CARBIDE_ESCROW_ADDRESS");
        std::env::remove_var("CARBIDE_USDC_ADDRESS");

        let defaults = ChainConfig::from_env();
        assert_eq!(defaults.chain_id, 421614);
        assert_eq!(defaults.name, "Arbitrum Sepolia");
        assert_eq!(defaults.rpc_url, "https://sepolia-rollup.arbitrum.io/rpc");
        assert!(defaults.escrow_address.is_empty());
        assert!(defaults.usdc_address.is_empty());
    }

    #[test]
    fn test_payment_status_display_roundtrip() {
        let statuses = [
            PaymentStatus::None,
            PaymentStatus::AwaitingDeposit,
            PaymentStatus::Deposited,
            PaymentStatus::PartiallyReleased,
            PaymentStatus::FullyReleased,
            PaymentStatus::Refunded,
            PaymentStatus::Disputed,
        ];
        for status in &statuses {
            let s = status.to_string();
            assert_eq!(PaymentStatus::from_str_lossy(&s), *status);
        }
    }
}
