//! Payment types shared across the Carbide Network.
//!
//! Defines the on-wire payment status enum, the Solana cluster
//! configuration, and the high-level payment instruction envelope
//! passed back to clients when storage requests are accepted.
//! Concrete chain interaction lives in the provider/client crates.

use serde::{Deserialize, Serialize};

/// Payment status for a storage contract
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// No payment configured
    None,
    /// Waiting for client to deposit funds into escrow
    AwaitingDeposit,
    /// Funds deposited in escrow, storage can begin
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

/// Solana cluster configuration shared by clients, providers, and the
/// discovery service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolanaConfig {
    /// Cluster identifier — one of `mainnet-beta`, `devnet`, `testnet`,
    /// `localnet`, or a custom label.
    pub cluster: String,
    /// JSON-RPC endpoint for the cluster.
    pub rpc_url: String,
    /// Carbide registry program address (base58).
    pub registry_program_id: String,
    /// Carbide escrow program address (base58).
    pub escrow_program_id: String,
    /// SPL token mint used for payments (base58 — typically USDC).
    pub usdc_mint: String,
}

impl SolanaConfig {
    /// Devnet defaults targeting the Solana devnet USDC mint.
    pub fn devnet() -> Self {
        Self {
            cluster: "devnet".to_string(),
            rpc_url: "https://api.devnet.solana.com".to_string(),
            registry_program_id: "5rAsbS4ApXNyNqrSUXqC7ju24kpEudHxfU1Q5khmAZHD".to_string(),
            escrow_program_id: "FQLdMfgTtio51EiWmNC444BmVfAtG9DAdWp8dLeCycgZ".to_string(),
            usdc_mint: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string(),
        }
    }

    /// Mainnet-beta defaults using the canonical USDC SPL mint. Program
    /// IDs are placeholders until the operator deploys to mainnet and
    /// fills them in.
    pub fn mainnet_beta() -> Self {
        Self {
            cluster: "mainnet-beta".to_string(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            registry_program_id: String::new(),
            escrow_program_id: String::new(),
            usdc_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        }
    }

    /// Override individual fields from environment variables. Each field
    /// has its own override knob so operators can swap RPCs or program
    /// IDs without rebuilding the binary.
    pub fn from_env() -> Self {
        let mut cfg = Self::devnet();
        if let Ok(v) = std::env::var("CARBIDE_SOLANA_CLUSTER") {
            cfg.cluster = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_SOLANA_RPC_URL") {
            cfg.rpc_url = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_REGISTRY_PROGRAM_ID") {
            cfg.registry_program_id = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_ESCROW_PROGRAM_ID") {
            cfg.escrow_program_id = v;
        }
        if let Ok(v) = std::env::var("CARBIDE_USDC_MINT") {
            cfg.usdc_mint = v;
        }
        cfg
    }
}

/// Payment instructions sent to the client after a storage request is accepted.
///
/// Carries the addresses and amounts the client needs to fund the
/// escrow PDA on Solana.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentInstructions {
    /// Escrow program ID the client should target (base58).
    pub escrow_program: String,
    /// Provider's Solana address (payee, base58).
    pub provider_address: String,
    /// SPL token mint used for payment (base58, typically USDC).
    pub token_mint: String,
    /// Total amount to deposit, in token base units (as string).
    pub total_amount: String,
    /// Cluster the deal is hosted on (`devnet`, `mainnet-beta`, ...).
    pub cluster: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn solana_config_devnet_defaults() {
        let cfg = SolanaConfig::devnet();
        assert_eq!(cfg.cluster, "devnet");
        assert!(cfg.rpc_url.contains("devnet"));
        assert_eq!(cfg.usdc_mint, "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");
        assert!(!cfg.registry_program_id.is_empty());
        assert!(!cfg.escrow_program_id.is_empty());
    }

    #[test]
    fn solana_config_env_overrides() {
        std::env::set_var("CARBIDE_SOLANA_CLUSTER", "mainnet-beta");
        std::env::set_var("CARBIDE_USDC_MINT", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        let cfg = SolanaConfig::from_env();
        assert_eq!(cfg.cluster, "mainnet-beta");
        assert_eq!(cfg.usdc_mint, "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        std::env::remove_var("CARBIDE_SOLANA_CLUSTER");
        std::env::remove_var("CARBIDE_USDC_MINT");
    }
}
