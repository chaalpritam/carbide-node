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
