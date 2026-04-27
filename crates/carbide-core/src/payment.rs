//! Payment types shared across the Carbide Network.
//!
//! Defines the on-wire payment status enum and the high-level payment
//! instruction envelope passed back to clients when storage requests are
//! accepted. Concrete chain integration lives in the provider/client crates.

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

/// Payment instructions sent to the client after a storage request is accepted.
///
/// Carries the addresses and amounts the client needs to fund escrow. The
/// concrete encoding of `escrow_program`, `provider_address`, and
/// `token_mint` is chain-specific and filled in by the discovery service or
/// provider node based on the active deployment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentInstructions {
    /// Escrow program / contract identifier the client should deposit into
    pub escrow_program: String,
    /// Provider's on-chain address (payee)
    pub provider_address: String,
    /// Token mint / contract address used for payment
    pub token_mint: String,
    /// Total amount to deposit, in token base units (as string)
    pub total_amount: String,
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
}
