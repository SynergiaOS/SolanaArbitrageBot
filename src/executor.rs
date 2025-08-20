//! Transaction Executor with Jupiter Integration
//! Executes arbitrage trades using Jupiter aggregator for best routing

use anyhow::{Result, anyhow, Context};
use log::{info, warn, error, debug};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
    pubkey::Pubkey,
    instruction::Instruction,
    commitment_config::CommitmentConfig,
    message::Message,
    compute_budget::ComputeBudgetInstruction,
    system_instruction,
};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::calculator::ArbitrageOpportunity;

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

pub struct TransactionExecutor {
    rpc_client: RpcClient,
    wallet: WalletType,
    dry_run: bool,
    priority_fee: u64,
    simulation_required: bool,
    slippage_bps: u16,
    http_client: reqwest::Client,
}

impl TransactionExecutor {
    pub fn get_wallet_address(&self) -> String {
        match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey().to_string(),
            WalletType::Ledger(_) => "Ledger".to_string(),
        }
    }

    pub fn new(config: &crate::Config, dry_run: bool) -> Result<Self> {
        // Load wallet
        let wallet = if config.wallet.use_ledger.unwrap_or(false) {
            warn!("Ledger support not fully implemented yet - using keypair");
            Self::setup_keypair_wallet(&config.wallet.path)?
        } else {
            Self::setup_keypair_wallet(&config.wallet.path)?
        };
        
        // Create RPC client
        let rpc_client = RpcClient::new_with_commitment(
            config.rpc.url.clone(),
            CommitmentConfig::confirmed(),
        );
        
        let pubkey = match &wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger not yet fully implemented"));
            }
        };
        
        info!("💳 Wallet loaded: {}", pubkey);
        info!("🌐 RPC endpoint: {}", config.rpc.url);
        info!("🏃 Mode: {}", if dry_run { "DRY RUN" } else { "LIVE TRADING" });
        
        Ok(Self {
            rpc_client,
            wallet,
            dry_run,
            priority_fee: config.execution.priority_fee_lamports,
            simulation_required: config.execution.simulation_required,
            slippage_bps: (config.limits.max_slippage_percent * 100.0) as u16,
            http_client: reqwest::Client::new(),
        })
    }
    
    fn setup_keypair_wallet(path: &str) -> Result<WalletType> {
        // Try reading as JSON array first
        let wallet_str = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read wallet from {}: {}", path, e))?;
        
        // Try parsing as JSON array
        let wallet_bytes: Vec<u8> = if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(&wallet_str) {
            bytes
        } else if let Ok(bytes) = serde_json::from_str::<Vec<i8>>(&wallet_str) {
            // Handle signed bytes (convert i8 to u8)
            bytes.into_iter().map(|b| b as u8).collect()
        } else {
            // Try base58 or other formats
            return Err(anyhow!("Invalid wallet format in {}", path));
        };
        
        if wallet_bytes.len() != 64 {
            return Err(anyhow!("Invalid wallet size: expected 64 bytes, got {}", wallet_bytes.len()));
        }
        
        let wallet = Keypair::from_bytes(&wallet_bytes)
            .map_err(|e| anyhow!("Invalid wallet format: {}", e))?;
        
        Ok(WalletType::Keypair(wallet))
    }
    
    pub async fn execute_arbitrage(&self, opportunity: &ArbitrageOpportunity) -> Result<Signature> {
        info!(
            "🔨 Executing arbitrage: {} SOL {} -> {} (expected profit: ${:.2})",
            opportunity.amount_sol,
            opportunity.buy_dex,
            opportunity.sell_dex,
            opportunity.expected_profit_usd
        );
        
        // In dry run mode, just simulate
        if self.dry_run {
            warn!("🏃 DRY RUN - Would execute trade for ${:.2} profit", opportunity.expected_profit_usd);
            return Ok(Signature::default());
        }
        
        // Get Jupiter quote for the arbitrage
        let quote = self.get_jupiter_quote_for_arbitrage(opportunity).await?;
        
        // Build swap transaction using Jupiter
        let transaction = self.build_jupiter_swap_transaction(quote).await?;
        
        // Simulate if required
        if self.simulation_required {
            info!("🔬 Simulating transaction...");
            match self.rpc_client.simulate_transaction(&transaction) {
                Ok(result) => {
                    if result.value.err.is_some() {
                        error!("❌ Simulation failed: {:?}", result.value.err);
                        return Err(anyhow!("Transaction simulation failed"));
                    }
                    info!("✅ Simulation successful! Units: {:?}", result.value.units_consumed);
                }
                Err(e) => {
                    error!("❌ Simulation error: {}", e);
                    return Err(anyhow!("Failed to simulate: {}", e));
                }
            }
        }
        
        // Execute transaction
        info!("📤 Sending transaction...");
        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        info!("✅ Transaction confirmed: {}", signature);
        
        Ok(signature)
    }
    
    async fn get_jupiter_quote_for_arbitrage(&self, opportunity: &ArbitrageOpportunity) -> Result<JupiterQuoteResponse> {
        // Convert SOL amount to lamports
        let amount_lamports = (opportunity.amount_sol * 1_000_000_000.0) as u64;
        
        // First swap: SOL -> USDC on the cheaper DEX
        info!("📊 Getting Jupiter quote for {} SOL -> USDC", opportunity.amount_sol);
        
        let quote_url = format!(
            "https://quote-api.jup.ag/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false",
            SOL_MINT, USDC_MINT, amount_lamports, self.slippage_bps
        );
        
        let response = self.http_client.get(&quote_url)
            .send()
            .await
            .context("Failed to get Jupiter quote")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter quote failed: {}", error_text));
        }
        
        let quote: JupiterQuoteResponse = response.json()
            .await
            .context("Failed to parse Jupiter quote")?;
        
        info!("📈 Jupiter route: {} -> {}", 
            quote.route_plan.iter()
                .map(|s| s.swap_info.label.clone())
                .collect::<Vec<_>>()
                .join(" -> "),
            quote.out_amount.parse::<u64>().unwrap_or(0) as f64 / 1_000_000.0
        );
        
        Ok(quote)
    }
    
    async fn build_jupiter_swap_transaction(&self, quote: JupiterQuoteResponse) -> Result<Transaction> {
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
            compute_unit_price_micro_lamports: Some(self.priority_fee * 1000), // Convert to micro-lamports
            priority_level: Some("high".to_string()),
            dynamic_slippage: Some(DynamicSlippage {
                min_bps: 10,
                max_bps: self.slippage_bps,
            }),
        };
        
        // Get swap transaction from Jupiter
        let response = self.http_client.post("https://quote-api.jup.ag/v6/swap")
            .json(&swap_request)
            .send()
            .await
            .context("Failed to get swap transaction")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter swap failed: {}", error_text));
        }
        
        let swap_response: JupiterSwapResponse = response.json()
            .await
            .context("Failed to parse swap response")?;
        
        // Deserialize the transaction
        let tx_bytes = base64::decode(&swap_response.swap_transaction)
            .context("Failed to decode transaction")?;
        
        let mut transaction: Transaction = bincode::deserialize(&tx_bytes)
            .context("Failed to deserialize transaction")?;
        
        // Sign the transaction
        match &self.wallet {
            WalletType::Keypair(kp) => {
                let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
                transaction.partial_sign(&[kp], recent_blockhash);
            }
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger signing not yet implemented"));
            }
        }
        
        Ok(transaction)
    }
    
    // Fallback method: Build custom swap instructions (without Jupiter)
    pub async fn execute_direct_swap(&self, opportunity: &ArbitrageOpportunity) -> Result<Signature> {
        info!("🔄 Executing direct DEX swap (fallback method)");
        
        let payer = match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_) => return Err(anyhow!("Ledger not yet implemented")),
        };
        
        // Build instructions
        let mut instructions = vec![];
        
        // Add compute budget instruction
        instructions.push(
            ComputeBudgetInstruction::set_compute_unit_price(self.priority_fee)
        );
        
        // Add compute unit limit
        instructions.push(
            ComputeBudgetInstruction::set_compute_unit_limit(300_000)
        );
        
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
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        // Create message
        let message = Message::new_with_blockhash(
            &instructions,
            Some(&payer),
            &recent_blockhash,
        );
        
        // Sign transaction
        let transaction = match &self.wallet {
            WalletType::Keypair(kp) => {
                Transaction::new(&[kp], message, recent_blockhash)
            }
            WalletType::Ledger(_) => {
                return Err(anyhow!("Ledger signing not yet implemented"));
            }
        };
        
        // Send transaction
        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        Ok(signature)
    }
}

// Dependencies to add to Cargo.toml:
// reqwest = { version = "0.11", features = ["json"] }
// base64 = "0.21"
