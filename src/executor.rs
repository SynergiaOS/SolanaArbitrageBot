//! Transaction Executor with Jupiter Integration
//! Optimized for high-frequency arbitrage execution with async operations and retry logic

use crate::calculator::ArbitrageOpportunity;
use crate::config_manager::BotConfig;
use crate::utils::conversions::*;
use anyhow::{Context, Result};
use base64::Engine;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use std::sync::Arc;
use std::fs;
use tokio::time::{sleep, Duration, Instant};

// Token mint addresses
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// DEX Program IDs
const RAYDIUM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

pub enum WalletType {
    Keypair(Keypair),
    Ledger(String), // Simplified for now - full impl in ledger.rs
}

#[derive(Debug, Serialize)]
struct JupiterSwapRequest {
    user_public_key: String,
    quote_response: JupiterQuoteResponse,
    wrap_and_unwrap_sol: bool,
    use_shared_accounts: bool,
    fee_account: Option<String>,
    tracking_account: Option<String>,
    compute_unit_price_micro_lamports: Option<u64>,
    priority_level: Option<String>,
    dynamic_slippage: Option<DynamicSlippage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DynamicSlippage {
    min_bps: u16,
    max_bps: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct JupiterQuoteResponse {
    input_mint: String,
    output_mint: String,
    in_amount: String,
    out_amount: String,
    other_amount_threshold: String,
    swap_mode: String,
    slippage_bps: u16,
    platform_fee: Option<PlatformFee>,
    price_impact_pct: String,
    route_plan: Vec<RoutePlanStep>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlatformFee {
    amount: String,
    fee_bps: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct RoutePlanStep {
    swap_info: SwapInfo,
    percent: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct SwapInfo {
    amm_key: String,
    label: String,
    input_mint: String,
    output_mint: String,
    in_amount: String,
    out_amount: String,
    fee_amount: String,
    fee_mint: String,
}

#[derive(Debug, Deserialize)]
struct JupiterSwapResponse {
    swap_transaction: String,
    last_valid_block_height: u64,
    prioritization_fee: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ExecutionSettings {
    pub dry_run: bool,
    pub priority_fee: u64,
    pub simulation_required: bool,
    pub slippage_bps: u16,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

// Add missing JupiterClient struct
pub struct JupiterClient {
    api_url: String,
    slippage_bps: u16,
}

impl JupiterClient {
    pub fn new(api_url: String, slippage_bps: u16) -> Result<Self> {
        Ok(Self {
            api_url,
            slippage_bps,
        })
    }
}

pub struct TransactionExecutor {
    rpc_client: Arc<RpcClient>, // Wrapped in Arc for sharing
    wallet: WalletType,
    dry_run: bool,
    jupiter_client: JupiterClient,
    settings: ExecutionSettings,

    // Performance tracking
    execution_count: std::sync::atomic::AtomicU64,
    total_execution_time_ms: std::sync::atomic::AtomicU64,
    success_count: std::sync::atomic::AtomicU64,
}

impl TransactionExecutor {
    pub fn get_wallet_address(&self) -> String {
        match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey().to_string(),
            WalletType::Ledger(_) => "Ledger".to_string(),
        }
    }

    pub fn new(config: &BotConfig, dry_run: bool) -> Result<Self> {
        info!("🔧 Initializing executor (dry run: {})", dry_run);
        
        let wallet = if config.wallet.use_ledger {
            Self::setup_keypair_wallet(&config.wallet.wallet_path)? // Use keypair for now, ledger support later
        } else {
            Self::setup_keypair_wallet(&config.wallet.wallet_path)?
        };
        
        info!("✅ Wallet initialized");
        
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.network.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));
        
        // Test connection - remove async call for now
        info!("🔗 Connecting to Solana RPC");
        info!("🌐 RPC endpoint: {}", config.network.rpc_url);
        
        // Setup Jupiter client
        let jupiter_client = JupiterClient::new(
            config.dex.jupiter.api_url.clone(),
            config.dex.jupiter.max_slippage_bps,
        )?;
        
        // Execution settings - fix field references
        let settings = ExecutionSettings {
            dry_run,
            priority_fee: config.trading.priority_fee_lamports,
            simulation_required: config.trading.simulation_required,
            slippage_bps: (decimal_to_f64(config.trading.max_slippage_percent) * 100.0) as u16,
            max_retries: config.network.max_retries,
            timeout_ms: (config.network.timeout_seconds * 1000) as u64,
        };

        let pubkey = match &wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger not yet fully implemented"));
            }
        };

        info!("💳 Wallet loaded: {}", pubkey);
        info!("🌐 RPC endpoint: {}", config.network.rpc_url);
        info!(
            "🏃 Mode: {}",
            if dry_run { "DRY RUN" } else { "LIVE TRADING" }
        );

        Ok(Self {
            rpc_client,
            wallet,
            jupiter_client,
            settings,
            execution_count: std::sync::atomic::AtomicU64::new(0),
            total_execution_time_ms: std::sync::atomic::AtomicU64::new(0),
            success_count: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, f64, f64) {
        let executions = self
            .execution_count
            .load(std::sync::atomic::Ordering::Relaxed);
        let total_time = self
            .total_execution_time_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        let successes = self
            .success_count
            .load(std::sync::atomic::Ordering::Relaxed);

        let avg_time_ms = if executions > 0 {
            total_time as f64 / executions as f64
        } else {
            0.0
        };

        let success_rate = if executions > 0 {
            successes as f64 / executions as f64 * 100.0
        } else {
            0.0
        };

        (executions, avg_time_ms, success_rate)
    }

    fn setup_keypair_wallet(wallet_path: &str) -> Result<Keypair> {
        let wallet_data = fs::read_to_string(wallet_path)
            .context("Failed to read wallet file")?;
    
        let keypair_bytes: Vec<u8> = serde_json::from_str(&wallet_data)
            .context("Failed to parse wallet JSON")?;
    
        let keypair = Keypair::from_bytes(&keypair_bytes)
            .context("Failed to create keypair from bytes")?;
    
        info!("📁 Loaded wallet from: {}", wallet_path);
        Ok(keypair)
    }

    pub async fn execute_arbitrage(&self, opportunity: &ArbitrageOpportunity) -> Result<Signature> {
        let start_time = Instant::now();
        self.execution_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        info!(
            "🔨 Executing arbitrage: {} SOL {} -> {} (expected profit: ${:.2})",
            opportunity.amount_sol,
            opportunity.buy_dex,
            opportunity.sell_dex,
            opportunity.expected_profit_usd
        );

        // In dry run mode, just simulate
        if self.dry_run {
            warn!(
                "🏃 DRY RUN - Would execute trade for ${:.2} profit",
                opportunity.expected_profit_usd
            );
            self.success_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(Signature::default());
        }

        let result = self.execute_arbitrage_with_retry(opportunity).await;

        // Update performance metrics
        let execution_time = start_time.elapsed().as_millis() as u64;
        self.total_execution_time_ms
            .fetch_add(execution_time, std::sync::atomic::Ordering::Relaxed);

        if result.is_ok() {
            self.success_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        result
    }

    async fn execute_arbitrage_with_retry(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<Signature> {
        let max_retries = 2; // Reduced retries for speed
        let mut last_error = None;

        for attempt in 1..=max_retries {
            let attempt_start = Instant::now();

            match self.execute_arbitrage_attempt(opportunity).await {
                Ok(signature) => {
                    if attempt > 1 {
                        info!(
                            "✅ Arbitrage succeeded on attempt {}/{} in {:?}",
                            attempt,
                            max_retries,
                            attempt_start.elapsed()
                        );
                    }
                    return Ok(signature);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    warn!(
                        "❌ Attempt {}/{} failed in {:?}: {}",
                        attempt,
                        max_retries,
                        attempt_start.elapsed(),
                        error_msg
                    );

                    // Smart retry logic - don't retry certain errors
                    if error_msg.contains("insufficient funds")
                        || error_msg.contains("slippage")
                        || error_msg.contains("price impact")
                    {
                        return Err(e); // Don't retry these errors
                    }

                    last_error = Some(e);

                    if attempt < max_retries {
                        // Faster backoff for arbitrage
                        let delay = Duration::from_millis(50 * attempt as u64);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }

    async fn execute_arbitrage_attempt(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<Signature> {
        let start_time = Instant::now();

        // Parallel execution: Get quote and fresh blockhash simultaneously
        let (quote_result, blockhash_result) = tokio::join!(
            self.get_jupiter_quote_for_arbitrage(opportunity),
            self.rpc_client.get_latest_blockhash()
        );

        let quote = quote_result?;
        let recent_blockhash = blockhash_result?;

        debug!("⚡ Quote + blockhash fetched in {:?}", start_time.elapsed());

        // Build swap transaction using Jupiter (optimized)
        let transaction = self
            .build_jupiter_swap_transaction_fast(quote, recent_blockhash)
            .await?;

        // Fast simulation if required
        if self.settings.simulation_required {
            self.simulate_transaction_fast(&transaction).await?;
        }

        // Execute transaction with optimized settings
        info!("📤 Sending transaction...");
        let signature = self.send_transaction_fast(&transaction).await?;
        info!(
            "✅ Transaction confirmed: {} in {:?}",
            signature,
            start_time.elapsed()
        );

        Ok(signature)
    }

    async fn simulate_transaction_fast(&self, transaction: &Transaction) -> Result<()> {
        debug!("🧪 Fast simulating transaction...");

        // Use fast simulation config
        let config = solana_client::rpc_config::RpcSimulateTransactionConfig {
            sig_verify: false, // Skip signature verification for speed
            replace_recent_blockhash: true,
            commitment: Some(CommitmentConfig::processed()), // Fastest commitment
            encoding: Some(UiTransactionEncoding::Base64),
            accounts: None, // Don't return account data for speed
            min_context_slot: None,
            inner_instructions: false, // Skip inner instructions for speed
        };

        // Simulate with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(2000), // Fast timeout
            self.rpc_client
                .simulate_transaction_with_config(transaction, config),
        )
        .await
        .context("Simulation timeout")?
        .context("Simulation RPC error")?;

        if let Some(err) = result.value.err {
            return Err(anyhow!("Transaction simulation failed: {:?}", err));
        }

        debug!(
            "✅ Fast simulation passed - Units: {:?}",
            result.value.units_consumed
        );
        Ok(())
    }

    async fn get_jupiter_quote_for_arbitrage(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<JupiterQuoteResponse> {
        // Convert SOL amount to lamports
        let amount_lamports = (opportunity.amount_sol * 1_000_000_000.0) as u64;

        debug!(
            "📊 Getting Jupiter quote for {} SOL -> USDC",
            opportunity.amount_sol
        );

        let quote_url = format!(
            "https://quote-api.jup.ag/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=true&maxAccounts=20",
            SOL_MINT, USDC_MINT, amount_lamports, self.settings.slippage_bps
        );

        // Optimized HTTP request with aggressive timeouts
        let response = self
            .jupiter_client
            .http_client
            .get(&quote_url)
            .timeout(Duration::from_millis(2000)) // Very fast timeout for arbitrage
            .send()
            .await
            .context("Failed to get Jupiter quote")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter quote failed: {}", error_text));
        }

        let quote: JupiterQuoteResponse = response
            .json()
            .await
            .context("Failed to parse Jupiter quote")?;

        debug!(
            "📈 Jupiter route: {} -> {}",
            quote
                .route_plan
                .iter()
                .map(|s| s.swap_info.label.clone())
                .collect::<Vec<_>>()
                .join(" -> "),
            quote.out_amount.parse::<u64>().unwrap_or(0) as f64 / 1_000_000.0
        );

        Ok(quote)
    }

    async fn build_jupiter_swap_transaction(
        &self,
        quote: JupiterQuoteResponse,
    ) -> Result<Transaction> {
        let payer = match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => return Err(anyhow!("Ledger not yet implemented")),
        };

        // Build swap request
        let swap_request = JupiterSwapRequest {
            user_public_key: payer.to_string(),
            quote_response: quote,
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: Some(self.jupiter_client.priority_fee * 1000), // Convert to micro-lamports
            priority_level: Some("high".to_string()),
            dynamic_slippage: Some(DynamicSlippage {
                min_bps: 10,
                max_bps: self.slippage_bps,
            }),
        };

        // Get swap transaction from Jupiter with timeout
        let response = self
            .jupiter_client
            .http_client
            .post("https://quote-api.jup.ag/v6/swap")
            .json(&swap_request)
            .timeout(Duration::from_secs(8)) // Fast timeout for arbitrage
            .send()
            .await
            .context("Failed to get swap transaction")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter swap failed: {}", error_text));
        }

        let swap_response: JupiterSwapResponse = response
            .json()
            .await
            .context("Failed to parse swap response")?;

        // Deserialize the transaction
        use base64::{engine::general_purpose, Engine as _};
        let tx_bytes = general_purpose::STANDARD
            .decode(&swap_response.swap_transaction)
            .context("Failed to decode transaction")?;

        let mut transaction: Transaction =
            bincode::deserialize(&tx_bytes).context("Failed to deserialize transaction")?;

        // Sign the transaction
        match &self.wallet {
            WalletType::Keypair(kp) => {
                let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;
                transaction.partial_sign(&[kp], recent_blockhash);
            }
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger signing not yet implemented"));
            }
        }

        Ok(transaction)
    }

    /// Optimized transaction building with pre-fetched blockhash
    async fn build_jupiter_swap_transaction_fast(
        &self,
        quote: JupiterQuoteResponse,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<Transaction> {
        let payer = match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => return Err(anyhow!("Ledger not yet implemented")),
        };

        // Build swap request with optimized settings
        let swap_request = JupiterSwapRequest {
            user_public_key: payer.to_string(),
            quote_response: quote,
            wrap_and_unwrap_sol: true,
            use_shared_accounts: true,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: Some(self.settings.priority_fee * 1000),
            priority_level: Some("veryHigh".to_string()), // Highest priority
            dynamic_slippage: Some(DynamicSlippage {
                min_bps: 5, // Tighter slippage for speed
                max_bps: self.settings.slippage_bps,
            }),
        };

        debug!("🔨 Building Jupiter swap transaction");

        // Fast HTTP request to Jupiter swap API
        let response = self
            .jupiter_client
            .http_client
            .post("https://quote-api.jup.ag/v6/swap")
            .json(&swap_request)
            .timeout(Duration::from_millis(3000)) // Fast timeout
            .send()
            .await
            .context("Failed to get Jupiter swap transaction")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter swap API error: {}", error_text));
        }

        let swap_response: JupiterSwapResponse = response.json().await?;
        let tx_bytes = base64::engine::general_purpose::STANDARD
            .decode(&swap_response.swap_transaction)
            .context("Failed to decode swap transaction")?;

        let mut transaction: Transaction =
            bincode::deserialize(&tx_bytes).context("Failed to deserialize transaction")?;

        // Sign with pre-fetched blockhash
        match &self.wallet {
            WalletType::Keypair(kp) => {
                transaction.partial_sign(&[kp], recent_blockhash);
            }
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger signing not yet implemented"));
            }
        }

        Ok(transaction)
    }

    /// Fast transaction sending with optimized confirmation strategy
    async fn send_transaction_fast(&self, transaction: &Transaction) -> Result<Signature> {
        // Send transaction with skipPreflight for speed
        let config = solana_client::rpc_config::RpcSendTransactionConfig {
            skip_preflight: true, // Skip simulation for speed
            preflight_commitment: Some(solana_sdk::commitment_config::CommitmentLevel::Processed),
            encoding: Some(UiTransactionEncoding::Base64),
            max_retries: Some(3),
            min_context_slot: None,
        };

        // Send transaction
        let signature = self
            .rpc_client
            .send_transaction_with_config(transaction, config)
            .await?;

        // Fast confirmation check (don't wait for finalized)
        let start_time = Instant::now();
        let timeout = Duration::from_secs(30);

        while start_time.elapsed() < timeout {
            match self.rpc_client.confirm_transaction(&signature).await {
                Ok(confirmed) => {
                    if confirmed {
                        return Ok(signature);
                    }
                }
                Err(e) => {
                    debug!("Confirmation check failed: {}", e);
                }
            }

            // Short sleep before retry
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(anyhow!("Transaction confirmation timeout"))
    }

    // Fallback method: Build custom swap instructions (without Jupiter)
    pub async fn execute_direct_swap(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<Signature> {
        info!("🔄 Executing direct DEX swap (fallback method)");

        let payer = match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => return Err(anyhow!("Ledger not yet implemented")),
        };

        // Build instructions
        let mut instructions = vec![];

        // Add compute budget instruction
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
            self.settings.priority_fee,
        ));

        // Add compute unit limit
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(300_000));

        // Note: Real swap instructions would go here
        // This requires integration with Raydium/Orca SDKs
        // For now, this is a placeholder

        if instructions.len() == 2 {
            warn!("Direct swap not fully implemented - would need Raydium/Orca SDK integration");
            if self.dry_run {
                return Ok(Signature::default());
            } else {
                return Err(anyhow!("Direct swap not yet implemented"));
            }
        }

        // Get recent blockhash
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;

        // Create message
        let message = Message::new_with_blockhash(&instructions, Some(&payer), &recent_blockhash);

        // Sign transaction
        let transaction = match &self.wallet {
            WalletType::Keypair(kp) => Transaction::new(&[kp], message, recent_blockhash),
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger signing not yet implemented"));
            }
        };

        // Send transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&transaction)
            .await?;
        Ok(signature)
    }

    pub async fn verify_transaction(&self, signature: &Signature) -> Result<f64> {
        let wallet_pubkey = match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger not yet implemented for verification"))
            }
        };

        let mut attempts = 0;
        let max_attempts = 30; // ~30 seconds timeout

        info!("🔍 Verifying transaction on-chain: {}", signature);

        loop {
            if attempts >= max_attempts {
                return Err(anyhow!(
                    "Transaction verification timed out for signature {}",
                    signature
                ));
            }
            attempts += 1;

            match self
                .rpc_client
                .get_transaction(signature, UiTransactionEncoding::JsonParsed)
                .await
            {
                Ok(tx) => {
                    return self.parse_transaction_profit(tx, &wallet_pubkey, signature);
                }
                Err(e) => {
                    debug!(
                        "Attempt {} to fetch transaction {}: {}. Retrying...",
                        attempts, signature, e
                    );
                    sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }

    fn parse_transaction_profit(
        &self,
        tx: EncodedConfirmedTransactionWithStatusMeta,
        wallet_pubkey: &Pubkey,
        signature: &Signature,
    ) -> Result<f64> {
        let meta = tx
            .transaction
            .meta
            .ok_or_else(|| anyhow!("Transaction metadata not found for signature {}", signature))?;

        if let Some(err) = meta.err {
            error!("❌ Transaction {} failed on-chain: {:?}", signature, err);
            let fee_lamports = meta.fee;
            // This is a simplification. We'd need SOL price for an accurate USD value.
            // Returning a negative value representing the fee in SOL.
            return Ok(-(fee_lamports as f64 / 1_000_000_000.0));
        }

        let usdc_mint_pubkey = Pubkey::from_str(USDC_MINT)?;

        let pre_balances = meta.pre_token_balances.unwrap_or(vec![]);
        let post_balances = meta.post_token_balances.unwrap_or(vec![]);

        let pre_usdc_balance = pre_balances
            .iter()
            .find(|balance| {
                balance.owner.as_ref().map_or(false, |owner| {
                    Pubkey::from_str(owner).unwrap_or_default() == *wallet_pubkey
                }) && balance.mint == usdc_mint_pubkey.to_string()
            })
            .and_then(|balance| balance.ui_token_amount.amount.parse::<u64>().ok());

        let post_usdc_balance = post_balances
            .iter()
            .find(|balance| {
                balance.owner.as_ref().map_or(false, |owner| {
                    Pubkey::from_str(owner).unwrap_or_default() == *wallet_pubkey
                }) && balance.mint == usdc_mint_pubkey.to_string()
            })
            .and_then(|balance| balance.ui_token_amount.amount.parse::<u64>().ok());

        match (pre_usdc_balance, post_usdc_balance) {
            (Some(pre), Some(post)) => {
                let diff = post as i64 - pre as i64;
                // USDC has 6 decimals
                let actual_profit_usd = diff as f64 / 1_000_000.0;
                info!("✅ Transaction {} verified. Pre-USDC: {}, Post-USDC: {}. Actual profit: ${:.4}", signature, pre, post, actual_profit_usd);
                Ok(actual_profit_usd)
            }
            _ => {
                warn!("Could not find USDC token balances for wallet {} in transaction {}. Assuming zero profit/loss from this tx.", wallet_pubkey, signature);
                // This can happen in complex transactions (e.g. with temporary accounts).
                // A zero profit is a safe assumption if we can't parse it, to avoid breaking the bot.
                Ok(0.0)
            }
        }
    }
}
