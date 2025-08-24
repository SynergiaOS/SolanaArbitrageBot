//! On-chain transaction verification module
//! Verifies arbitrage transactions after execution

use anyhow::Result;
use log::{debug, info, warn};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct VerificationConfig {
    pub max_confirmation_time_seconds: u64,
    pub confirmation_checks_interval_ms: u64,
    pub required_confirmations: usize,
    pub enable_detailed_verification: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_confirmation_time_seconds: 30,
            confirmation_checks_interval_ms: 1000,
            required_confirmations: 1,
            enable_detailed_verification: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum VerificationResult {
    Confirmed {
        signature: Signature,
        slot: u64,
        confirmations: usize,
        verification_time_ms: u64,
    },
    Failed {
        signature: Signature,
        reason: String,
        verification_time_ms: u64,
    },
    Timeout {
        signature: Signature,
        verification_time_ms: u64,
    },
}

pub struct TransactionVerifier {
    rpc_client: Arc<RpcClient>,
    config: VerificationConfig,
}

impl TransactionVerifier {
    pub fn new(rpc_client: Arc<RpcClient>, config: VerificationConfig) -> Self {
        Self { rpc_client, config }
    }

    /// Verify transaction confirmation on-chain
    pub async fn verify_transaction(&self, signature: Signature) -> Result<VerificationResult> {
        let start_time = Instant::now();
        info!("🔍 Starting verification for transaction: {}", signature);

        let max_duration = Duration::from_secs(self.config.max_confirmation_time_seconds);
        let check_interval = Duration::from_millis(self.config.confirmation_checks_interval_ms);

        while start_time.elapsed() < max_duration {
            match self.check_transaction_status(&signature).await {
                Ok(Some(result)) => {
                    let verification_time = start_time.elapsed().as_millis() as u64;
                    info!(
                        "✅ Transaction verified in {}ms: {}",
                        verification_time, signature
                    );
                    let (slot, confirmations) = result;
                    return Ok(VerificationResult::Confirmed {
                        signature,
                        slot,
                        confirmations,
                        verification_time_ms: verification_time,
                    });
                }
                Ok(None) => {
                    debug!("⏳ Transaction {} still pending...", signature);
                }
                Err(e) => {
                    let verification_time = start_time.elapsed().as_millis() as u64;
                    warn!("❌ Transaction verification failed: {}", e);
                    return Ok(VerificationResult::Failed {
                        signature,
                        reason: e.to_string(),
                        verification_time_ms: verification_time,
                    });
                }
            }

            sleep(check_interval).await;
        }

        let verification_time = start_time.elapsed().as_millis() as u64;
        warn!("⏰ Transaction verification timeout: {}", signature);
        Ok(VerificationResult::Timeout {
            signature,
            verification_time_ms: verification_time,
        })
    }

    /// Check transaction status and confirmations
    async fn check_transaction_status(
        &self,
        signature: &Signature,
    ) -> Result<Option<(u64, usize)>> {
        // Try to get transaction directly - if it exists, it's confirmed
        match self
            .rpc_client
            .get_transaction_with_config(
                signature,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
            .await
        {
            Ok(transaction) => {
                debug!("✅ Transaction {} confirmed", signature);
                let slot = transaction.slot;
                let confirmations = 1; // Confirmed level
                Ok(Some((slot, confirmations)))
            }
            Err(_) => {
                // Transaction not found or not confirmed yet
                debug!("⏳ Transaction {} still pending...", signature);
                Ok(None)
            }
        }
    }

    /// Verify arbitrage transaction details
    pub async fn verify_arbitrage_details(
        &self,
        signature: Signature,
    ) -> Result<ArbitrageVerification> {
        if !self.config.enable_detailed_verification {
            return Ok(ArbitrageVerification::Skipped);
        }

        info!("🔍 Verifying arbitrage transaction details: {}", signature);

        let transaction = self
            .rpc_client
            .get_transaction_with_config(
                &signature,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )
            .await?;

        // Analyze transaction logs for arbitrage success
        if let Some(meta) = transaction.transaction.meta {
            let logs = match meta.log_messages {
                solana_transaction_status::option_serializer::OptionSerializer::Some(logs) => logs,
                _ => return Ok(ArbitrageVerification::PartiallyVerified { signature }),
            };

            let sol_change = 0i64;
            let mut swap_count = 0;

            for log in logs {
                // Parse Jupiter/DEX logs for swap information
                if log.contains("Program log: Instruction: Swap") {
                    swap_count += 1;
                }

                // Parse SOL balance changes
                if log.contains("SOL balance change") {
                    // Extract balance change (simplified parsing)
                    // In real implementation, parse actual balance changes
                }
            }

            return Ok(ArbitrageVerification::Verified {
                signature,
                swap_count,
                net_sol_change: sol_change,
                gas_used: meta.fee,
                success: meta.err.is_none(),
            });
        }

        Ok(ArbitrageVerification::PartiallyVerified { signature })
    }
}

#[derive(Debug, Clone)]
pub enum ArbitrageVerification {
    Verified {
        signature: Signature,
        swap_count: u32,
        net_sol_change: i64,
        gas_used: u64,
        success: bool,
    },
    PartiallyVerified {
        signature: Signature,
    },
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verification_config_default() {
        let config = VerificationConfig::default();
        assert_eq!(config.max_confirmation_time_seconds, 30);
        assert_eq!(config.confirmation_checks_interval_ms, 1000);
        assert_eq!(config.required_confirmations, 1);
        assert!(config.enable_detailed_verification);
    }

    #[tokio::test]
    async fn test_verifier_creation() {
        let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
        let config = VerificationConfig::default();
        let verifier = TransactionVerifier::new(rpc_client, config);

        // Test that verifier is created successfully
        assert_eq!(verifier.config.max_confirmation_time_seconds, 30);
    }
}
