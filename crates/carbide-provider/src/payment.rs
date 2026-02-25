//! On-chain payment service for escrow interaction
//!
//! Provides the `PaymentService` that wraps an ethers `Provider` and wallet
//! to query escrow status, verify deposits, check USDC balances, and submit
//! payment release transactions on Arbitrum.

use std::sync::Arc;

use ethers::prelude::*;
use ethers::types::{Address, U256};

use carbide_core::{CarbideError, Result};

use crate::contracts::{CarbideEscrowContract, MockUsdcContract};

/// Service for interacting with on-chain escrow contracts
pub struct PaymentService {
    /// Ethers provider for read-only queries
    provider: Arc<Provider<Http>>,
    /// Escrow contract instance (read-only)
    escrow: CarbideEscrowContract<Provider<Http>>,
    /// USDC contract instance (read-only)
    usdc: MockUsdcContract<Provider<Http>>,
    /// Chain ID
    chain_id: u64,
}

/// Details of an on-chain escrow
#[derive(Debug, Clone)]
pub struct EscrowDetails {
    /// Client who created the escrow
    pub client: Address,
    /// Provider who receives payments
    pub provider: Address,
    /// ERC-20 token used
    pub token: Address,
    /// Total amount deposited
    pub total_amount: U256,
    /// Amount released so far
    pub released_amount: U256,
    /// Total payment periods
    pub total_periods: u32,
    /// Periods already released
    pub periods_released: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Whether escrow is active
    pub active: bool,
    /// Whether escrow is disputed
    pub disputed: bool,
}

impl PaymentService {
    /// Create a new payment service connected to a chain
    pub fn new(
        rpc_url: &str,
        escrow_address: Address,
        usdc_address: Address,
        chain_id: u64,
    ) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| CarbideError::Payment(format!("Failed to create provider: {e}")))?;

        let provider = Arc::new(provider);

        let escrow = CarbideEscrowContract::new(escrow_address, provider.clone());
        let usdc = MockUsdcContract::new(usdc_address, provider.clone());

        Ok(Self {
            provider,
            escrow,
            usdc,
            chain_id,
        })
    }

    /// Get the chain ID this service is connected to
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Query escrow details from on-chain state
    pub async fn get_escrow(&self, escrow_id: u64) -> Result<EscrowDetails> {
        let result = self
            .escrow
            .get_escrow(U256::from(escrow_id))
            .call()
            .await
            .map_err(|e| CarbideError::Payment(format!("Failed to query escrow: {e}")))?;

        Ok(EscrowDetails {
            client: result.0,
            provider: result.1,
            token: result.2,
            total_amount: result.3,
            released_amount: result.4,
            total_periods: result.5,
            periods_released: result.6,
            created_at: result.7,
            active: result.8,
            disputed: result.9,
        })
    }

    /// Check if an escrow is funded and active
    pub async fn is_escrow_funded(&self, escrow_id: u64) -> Result<bool> {
        let details = self.get_escrow(escrow_id).await?;
        Ok(details.active && details.total_amount > U256::zero())
    }

    /// Get USDC balance of an address
    pub async fn get_usdc_balance(&self, address: Address) -> Result<U256> {
        self.usdc
            .balance_of(address)
            .call()
            .await
            .map_err(|e| CarbideError::Payment(format!("Failed to query balance: {e}")))
    }

    /// Get remaining balance in an escrow
    pub async fn get_remaining_balance(&self, escrow_id: u64) -> Result<U256> {
        self.escrow
            .get_remaining_balance(U256::from(escrow_id))
            .call()
            .await
            .map_err(|e| {
                CarbideError::Payment(format!("Failed to query remaining balance: {e}"))
            })
    }

    /// Submit a payment release transaction using a signer
    pub async fn submit_release<S: Middleware + 'static>(
        &self,
        signer: Arc<S>,
        escrow_address: Address,
        escrow_id: u64,
        period: u32,
        proof_hash: [u8; 32],
        verifier_signature: Vec<u8>,
    ) -> Result<TxHash> {
        let escrow_with_signer = CarbideEscrowContract::new(escrow_address, signer);

        let tx = escrow_with_signer.release_payment(
            U256::from(escrow_id),
            period,
            proof_hash,
            verifier_signature.into(),
        );

        let pending = tx
            .send()
            .await
            .map_err(|e| CarbideError::Payment(format!("Failed to send release tx: {e}")))?;

        Ok(pending.tx_hash())
    }

    /// Wait for a transaction to be confirmed
    pub async fn wait_for_confirmation(
        &self,
        tx_hash: TxHash,
    ) -> Result<TransactionReceipt> {
        let receipt = self
            .provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| {
                CarbideError::Payment(format!("Failed to get tx receipt: {e}"))
            })?
            .ok_or_else(|| {
                CarbideError::Payment("Transaction receipt not found".to_string())
            })?;

        Ok(receipt)
    }
}
