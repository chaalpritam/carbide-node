//! Type-safe Rust bindings for Carbide smart contracts
//!
//! Generated via ethers `abigen!` from compiled Solidity ABI.
//! Only available with the `blockchain` feature flag.

#[allow(missing_docs, clippy::all)]
mod bindings {
    use ethers::prelude::abigen;

    abigen!(
        CarbideEscrowContract,
        r#"[
            function createEscrow(address provider, address token, uint256 totalAmount, uint32 totalPeriods) external returns (uint256 escrowId)
            function releasePayment(uint256 escrowId, uint32 period, bytes32 proofHash, bytes memory signature) external
            function cancelEscrow(uint256 escrowId) external
            function raiseDispute(uint256 escrowId) external
            function resolveDispute(uint256 escrowId, uint256 providerAmount, uint256 clientAmount) external
            function getEscrow(uint256 escrowId) external view returns (address client, address provider, address token, uint256 totalAmount, uint256 releasedAmount, uint32 totalPeriods, uint32 periodsReleased, uint64 createdAt, bool active, bool disputed)
            function getRemainingBalance(uint256 escrowId) external view returns (uint256)
            function getCurrentPeriod(uint256 escrowId) external view returns (uint32)
            function nextEscrowId() external view returns (uint256)
            function owner() external view returns (address)
            function authorizedVerifiers(address) external view returns (bool)
            function addVerifier(address verifier) external
            function removeVerifier(address verifier) external
            function DOMAIN_SEPARATOR() external view returns (bytes32)
            event EscrowCreated(uint256 indexed escrowId, address indexed client, address indexed provider, uint256 amount, uint32 totalPeriods)
            event PaymentReleased(uint256 indexed escrowId, uint32 period, uint256 amount, bytes32 proofHash)
            event EscrowCompleted(uint256 indexed escrowId)
            event EscrowCancelled(uint256 indexed escrowId, uint256 refundedAmount)
            event EscrowDisputed(uint256 indexed escrowId, address disputedBy)
            event DisputeResolved(uint256 indexed escrowId, uint256 providerAmount, uint256 clientAmount)
        ]"#
    );

    abigen!(
        MockUsdcContract,
        r#"[
            function name() external view returns (string)
            function symbol() external view returns (string)
            function decimals() external view returns (uint8)
            function totalSupply() external view returns (uint256)
            function balanceOf(address account) external view returns (uint256)
            function allowance(address owner, address spender) external view returns (uint256)
            function approve(address spender, uint256 amount) external returns (bool)
            function transfer(address to, uint256 amount) external returns (bool)
            function transferFrom(address from, address to, uint256 amount) external returns (bool)
            function mint(address to, uint256 amount) external
            function faucet() external
            event Transfer(address indexed from, address indexed to, uint256 value)
            event Approval(address indexed owner, address indexed spender, uint256 value)
        ]"#
    );
}

pub use bindings::*;

/// Get chain configuration for a given chain ID
pub mod chain_config {
    use carbide_core::payment::ChainConfig;

    /// Look up chain config by chain ID
    pub fn get_chain_config(chain_id: u64) -> Option<ChainConfig> {
        match chain_id {
            421614 => Some(ChainConfig::arbitrum_sepolia()),
            42161 => Some(ChainConfig::arbitrum_one()),
            _ => None,
        }
    }
}
