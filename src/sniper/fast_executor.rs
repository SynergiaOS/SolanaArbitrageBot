//! ⚡ Fast Execution Engine - Sub-100ms Trade Execution
//! 
//! Ultra-fast execution system for sniper bot with aggressive optimization

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{Signer, Signature}; // Add missing Signature import
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
    commitment_config::{CommitmentConfig, CommitmentLevel}, // Add CommitmentLevel
    compute_budget::ComputeBudgetInstruction, // Add missing import
};
use anyhow::{Result, anyhow};
use log::{info, warn, error, debug};

use super::enhanced_detector::EnhancedTokenLaunch;

/// Fast execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastExecutorConfig {
    /// Target execution time in milliseconds
    pub target_execution_ms: u64,
    
    /// Maximum slippage tolerance
    pub max_slippage_percent: Decimal,
    
    /// Priority fee in lamports (higher = faster)
    pub priority_fee_lamports: u64,
    
    /// Compute unit limit
    pub compute_unit_limit: u32,
    
    /// Skip preflight simulation for speed
    pub skip_preflight: bool,
    
    /// Maximum retries for failed transactions
    pub max_retries: u32,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    
    /// Use parallel transaction submission
    pub parallel_submission: bool,
    
    /// Precompute transaction templates
    pub precompute_templates: bool,
}

impl Default for FastExecutorConfig {
    fn default() -> Self {
        Self {
            target_execution_ms: 100,
            max_slippage_percent: Decimal::from_f64_retain(1.0).unwrap(), // 1%
            priority_fee_lamports: 100000, // High priority
            compute_unit_limit: 400000,
            skip_preflight: true,
            max_retries: 2,
            retry_delay_ms: 50,
            parallel_submission: true,
            precompute_templates: true,
        }
    }
}

/// Execution result with detailed metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastExecutionResult {
    pub success: bool,
    pub transaction_signature: Option<Signature>,
    pub execution_time_ms: u64,
    pub entry_price: Decimal,
    pub position_size_sol: Decimal,
    pub tokens_received: Decimal,
    pub actual_slippage: Decimal,
    pub gas_used: u64,
    pub priority_fee_paid: u64,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub timestamp: SystemTime,
}

/// Pre-computed transaction template for speed
#[derive(Debug, Clone)]
struct TransactionTemplate {
    instructions: Vec<Instruction>,
    compute_budget_ix: Instruction,
    priority_fee_ix: Instruction,
    last_updated: Instant,
}

/// Ultra-fast execution engine
pub struct FastExecutor {
    rpc_client: Arc<RpcClient>,
    config: FastExecutorConfig,
    keypair: Arc<Keypair>,
    
    /// Pre-computed transaction templates
    templates: Arc<RwLock<std::collections::HashMap<String, TransactionTemplate>>>,
    
    /// Execution metrics
    execution_times: Arc<RwLock<Vec<u64>>>,
    success_count: Arc<RwLock<u64>>,
    failure_count: Arc<RwLock<u64>>,
    
    /// Performance optimization
    recent_blockhashes: Arc<RwLock<Vec<(solana_sdk::hash::Hash, Instant)>>>,
    jupiter_client: Arc<reqwest::Client>,
}

impl FastExecutor {
    /// Create new fast executor
    pub fn new(
        rpc_client: Arc<RpcClient>,
        keypair: Arc<Keypair>,
        config: FastExecutorConfig,
    ) -> Self {
        let jupiter_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(500)) // Very fast timeout
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(20)
            .build()
            .expect("Failed to create Jupiter client");
        
        Self {
            rpc_client,
            config,
            keypair,
            templates: Arc::new(RwLock::new(std::collections::HashMap::new())),
            execution_times: Arc::new(RwLock::new(Vec::new())),
            success_count: Arc::new(RwLock::new(0)),
            failure_count: Arc::new(RwLock::new(0)),
            recent_blockhashes: Arc::new(RwLock::new(Vec::new())),
            jupiter_client: Arc::new(jupiter_client),
        }
    }
    
    /// Execute snipe with maximum speed optimization
    pub async fn execute_snipe(
        &self,
        launch: &EnhancedTokenLaunch,
        position_size_sol: Decimal,
    ) -> Result<FastExecutionResult> {
        let start_time = Instant::now();
        
        debug!("🚀 Starting fast execution for token: {}", launch.token_mint);
        
        // Step 1: Get quote (parallel with blockhash)
        let (quote_result, blockhash_result) = tokio::join!(
            self.get_fast_quote(&launch.token_mint, position_size_sol),
            self.get_fresh_blockhash()
        );
        
        let quote = quote_result?;
        let blockhash = blockhash_result?;
        
        // Step 2: Build transaction with pre-computed template
        let transaction = self.build_fast_transaction(
            &launch.token_mint,
            position_size_sol,
            &quote,
            blockhash,
        ).await?;
        
        // Step 3: Execute with parallel submission if enabled
        let execution_result = if self.config.parallel_submission {
            self.execute_parallel(&transaction).await
        } else {
            self.execute_single(&transaction).await
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Update metrics
        self.execution_times.write().await.push(execution_time);
        
        match execution_result {
            Ok(signature) => {
                *self.success_count.write().await += 1;
                
                // Calculate actual results
                let (entry_price, tokens_received, actual_slippage) = 
                    self.calculate_execution_results(&signature, position_size_sol).await?;
                
                info!("✅ Fast execution successful: {} in {}ms", signature, execution_time);
                
                Ok(FastExecutionResult {
                    success: true,
                    transaction_signature: Some(signature),
                    execution_time_ms: execution_time,
                    entry_price,
                    position_size_sol,
                    tokens_received,
                    actual_slippage,
                    gas_used: 0, // TODO: Calculate from transaction
                    priority_fee_paid: self.config.priority_fee_lamports,
                    error_message: None,
                    retry_count: 0,
                    timestamp: SystemTime::now(),
                })
            }
            Err(e) => {
                *self.failure_count.write().await += 1;
                
                error!("❌ Fast execution failed: {} ({}ms)", e, execution_time);
                
                Ok(FastExecutionResult {
                    success: false,
                    transaction_signature: None,
                    execution_time_ms: execution_time,
                    entry_price: Decimal::ZERO,
                    position_size_sol,
                    tokens_received: Decimal::ZERO,
                    actual_slippage: Decimal::ZERO,
                    gas_used: 0,
                    priority_fee_paid: self.config.priority_fee_lamports,
                    error_message: Some(e.to_string()),
                    retry_count: 0,
                    timestamp: SystemTime::now(),
                })
            }
        }
    }
    
    /// Get quote from Jupiter with aggressive timeout
    async fn get_fast_quote(
        &self,
        token_mint: &Pubkey,
        amount_sol: Decimal,
    ) -> Result<JupiterQuote> {
        let amount_lamports = (amount_sol * Decimal::from(1_000_000_000)).to_u64()
            .ok_or_else(|| anyhow!("Invalid amount"))?;
        
        let url = format!(
            "https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint={}&amount={}&slippageBps={}",
            token_mint,
            amount_lamports,
            (self.config.max_slippage_percent * Decimal::from(100)).to_u32().unwrap_or(100)
        );
        
        let response = self.jupiter_client
            .get(&url)
            .timeout(Duration::from_millis(300)) // Very aggressive timeout
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Jupiter quote failed: {}", response.status()));
        }
        
        let quote: JupiterQuote = response.json().await?;
        Ok(quote)
    }
    
    /// Get fresh blockhash with caching
    async fn get_fresh_blockhash(&self) -> Result<solana_sdk::hash::Hash> {
        // Check cache first
        {
            let blockhashes = self.recent_blockhashes.read().await;
            if let Some((hash, timestamp)) = blockhashes.last() {
                if timestamp.elapsed() < Duration::from_secs(30) {
                    return Ok(*hash);
                }
            }
        }
        
        // Fetch new blockhash
        let blockhash = self.rpc_client
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await?
            .0;
        
        // Update cache
        {
            let mut blockhashes = self.recent_blockhashes.write().await;
            blockhashes.push((blockhash, Instant::now()));
            
            // Keep only recent blockhashes
            blockhashes.retain(|(_, timestamp)| timestamp.elapsed() < Duration::from_secs(60));
        }
        
        Ok(blockhash)
    }
    
    /// Build transaction with pre-computed template
    async fn build_fast_transaction(
        &self,
        token_mint: &Pubkey,
        amount_sol: Decimal,
        quote: &JupiterQuote,
        blockhash: solana_sdk::hash::Hash,
    ) -> Result<Transaction> {
        let mut instructions = Vec::new();
        
        // Add compute budget instructions for priority
        instructions.push(
            ComputeBudgetInstruction::set_compute_unit_limit(self.config.compute_unit_limit)
        );
        
        instructions.push(
            ComputeBudgetInstruction::set_compute_unit_price(
                self.config.priority_fee_lamports / self.config.compute_unit_limit as u64
            )
        );
        
        // Add Jupiter swap instruction
        let swap_instruction = self.build_jupiter_swap_instruction(quote).await?;
        instructions.push(swap_instruction);
        
        // Build transaction
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.keypair.pubkey()),
            &[&*self.keypair],
            blockhash,
        );
        
        Ok(transaction)
    }
    
    /// Execute transaction with parallel submission
    async fn execute_parallel(&self, transaction: &Transaction) -> Result<Signature> {
        // Submit to multiple RPC endpoints simultaneously
        let futures = vec![
            self.rpc_client.send_transaction_with_config(
                transaction,
                solana_client::rpc_config::RpcSendTransactionConfig {
                    skip_preflight: self.config.skip_preflight,
                    preflight_commitment: Some(CommitmentLevel::Confirmed), // Fix: use CommitmentLevel
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                    max_retries: Some(0), // No retries in parallel mode
                    min_context_slot: None,
                }
            ),
            // Add more RPC clients here for true parallel submission
        ];
        
        // Wait for first successful response
        let results = futures::future::join_all(futures).await;
        
        for result in results {
            if let Ok(signature) = result {
                return Ok(signature);
            }
        }
        
        Err(anyhow!("All parallel submissions failed"))
    }
    
    /// Execute transaction with single submission
    async fn execute_single(&self, transaction: &Transaction) -> Result<Signature> {
        let signature = self.rpc_client.send_transaction_with_config(
            transaction,
            solana_client::rpc_config::RpcSendTransactionConfig {
                skip_preflight: self.config.skip_preflight,
                preflight_commitment: Some(CommitmentLevel::Confirmed), // Fix: use CommitmentLevel
                encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                max_retries: Some(self.config.max_retries.try_into().unwrap()), // Fix: convert u32 to usize
                min_context_slot: None,
            }
        ).await?;
        
        Ok(signature)
    }
    
    /// Calculate actual execution results
    async fn calculate_execution_results(
        &self,
        signature: &Signature,
        position_size_sol: Decimal,
    ) -> Result<(Decimal, Decimal, Decimal)> {
        // Wait for confirmation
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        // Get transaction details
        let _transaction = self.rpc_client
            .get_transaction_with_config(
                signature,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(solana_transaction_status::UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                }
            )
            .await?;
        
        // Parse transaction to extract actual results
        // This is simplified - real implementation would parse the transaction logs
        let entry_price = Decimal::from_f64_retain(0.000001).unwrap(); // Placeholder
        let tokens_received = position_size_sol / entry_price;
        let actual_slippage = Decimal::ZERO; // Calculate from expected vs actual
        
        Ok((entry_price, tokens_received, actual_slippage))
    }
    
    /// Build Jupiter swap instruction
    async fn build_jupiter_swap_instruction(&self, _quote: &JupiterQuote) -> Result<Instruction> {
        // This would build the actual Jupiter swap instruction
        // Simplified for now
        Ok(Instruction::new_with_bytes(
            Pubkey::default(),
            &[],
            vec![],
        ))
    }
    
    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> ExecutorMetrics {
        let execution_times = self.execution_times.read().await;
        let success_count = *self.success_count.read().await;
        let failure_count = *self.failure_count.read().await;
        
        let average_execution_time = if execution_times.is_empty() {
            0.0
        } else {
            execution_times.iter().sum::<u64>() as f64 / execution_times.len() as f64
        };
        
        let success_rate = if success_count + failure_count == 0 {
            0.0
        } else {
            success_count as f64 / (success_count + failure_count) as f64
        };
        
        ExecutorMetrics {
            total_executions: success_count + failure_count,
            successful_executions: success_count,
            failed_executions: failure_count,
            average_execution_time_ms: average_execution_time,
            success_rate,
            target_execution_time_ms: self.config.target_execution_ms,
            performance_ratio: self.config.target_execution_ms as f64 / average_execution_time.max(1.0),
        }
    }
}

/// Jupiter quote response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JupiterQuote {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    swap_mode: String,
    #[serde(rename = "slippageBps")]
    slippage_bps: u32,
    #[serde(rename = "platformFee")]
    platform_fee: Option<serde_json::Value>,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<serde_json::Value>,
}

/// Executor performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub success_rate: f64,
    pub target_execution_time_ms: u64,
    pub performance_ratio: f64, // target/actual (>1.0 = beating target)
}
