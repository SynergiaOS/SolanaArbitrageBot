//! Trade Executor - Fast execution of sniper trades
//! Uses Jupiter API for optimal routing and execution

use anyhow::{anyhow, Result};
use log::{debug, info};
use reqwest::Client;
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::sync::Arc;

pub struct TradeExecutor {
    keypair: Arc<Keypair>,
    rpc_client: Arc<RpcClient>,
    http_client: Client,
    jupiter_api_url: String,
}

#[derive(Debug, Clone)]
pub struct TradeParams {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
    pub priority_fee_lamports: u64,
}

#[derive(Debug)]
pub struct TradeResult {
    pub signature: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact: f64,
}

impl TradeExecutor {
    pub fn new(keypair: Arc<Keypair>, rpc_client: Arc<RpcClient>, jupiter_api_url: String) -> Self {
        Self {
            keypair,
            rpc_client,
            http_client: Client::new(),
            jupiter_api_url,
        }
    }

    pub async fn buy_token(
        &self,
        mint: &Pubkey,
        sol_amount: u64,
        max_slippage_bps: u16,
    ) -> Result<TradeResult> {
        info!("🔫 SNIPING: {} with {} lamports", mint, sol_amount);

        let params = TradeParams {
            input_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
            output_mint: mint.to_string(),
            amount: sol_amount,
            slippage_bps: max_slippage_bps,
            priority_fee_lamports: 100_000, // High priority for speed
        };

        self.execute_swap(params).await
    }

    pub async fn sell_token(
        &self,
        mint: &Pubkey,
        token_amount: u64,
        max_slippage_bps: u16,
    ) -> Result<TradeResult> {
        info!("💰 SELLING: {} tokens of {}", token_amount, mint);

        let params = TradeParams {
            input_mint: mint.to_string(),
            output_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
            amount: token_amount,
            slippage_bps: max_slippage_bps,
            priority_fee_lamports: 50_000, // Lower priority for sells
        };

        self.execute_swap(params).await
    }

    async fn execute_swap(&self, params: TradeParams) -> Result<TradeResult> {
        // Step 1: Get quote from Jupiter
        let quote = self.get_jupiter_quote(&params).await?;

        // Step 2: Get swap transaction
        let swap_transaction = self.get_swap_transaction(&quote, &params).await?;

        // Step 3: Sign and send transaction
        let signature = self.send_transaction(swap_transaction).await?;

        // Step 4: Parse results
        let input_amount = quote["inAmount"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing inAmount"))?
            .parse::<u64>()?;

        let output_amount = quote["outAmount"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing outAmount"))?
            .parse::<u64>()?;

        let price_impact = quote["priceImpactPct"]
            .as_str()
            .unwrap_or("0")
            .parse::<f64>()
            .unwrap_or(0.0);

        Ok(TradeResult {
            signature,
            input_amount,
            output_amount,
            price_impact,
        })
    }

    pub async fn get_jupiter_quote(&self, params: &TradeParams) -> Result<Value> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.jupiter_api_url,
            params.input_mint,
            params.output_mint,
            params.amount,
            params.slippage_bps
        );

        debug!("Getting Jupiter quote: {}", url);

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter quote failed: {}", error_text));
        }

        let quote: Value = response.json().await?;

        // Validate quote
        if quote.get("error").is_some() {
            return Err(anyhow!("Jupiter quote error: {}", quote));
        }

        Ok(quote)
    }

    pub async fn get_estimated_price_lamports_per_token(
        &self,
        mint: &Pubkey,
        amount_tokens: u64,
    ) -> Result<f64> {
        // Quote swapping `amount_tokens` of the token into SOL to estimate price
        if amount_tokens == 0 {
            return Ok(0.0);
        }
        let params = TradeParams {
            input_mint: mint.to_string(),
            output_mint: "So11111111111111111111111111111111111111112".to_string(),
            amount: amount_tokens,
            slippage_bps: 1000, // high slippage for quote tolerance
            priority_fee_lamports: 0,
        };
        let quote = self.get_jupiter_quote(&params).await?;
        let out_amount = quote["outAmount"]
            .as_str()
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0);
        let price_lamports_per_token = out_amount as f64 / amount_tokens as f64; // lamports per token base unit
        Ok(price_lamports_per_token)
    }

    async fn get_swap_transaction(
        &self,
        quote: &Value,
        params: &TradeParams,
    ) -> Result<Transaction> {
        let swap_url = format!("{}/swap", self.jupiter_api_url);

        let request_body = serde_json::json!({
            "quoteResponse": quote,
            "userPublicKey": self.keypair.pubkey().to_string(),
            "wrapAndUnwrapSol": true,
            "dynamicComputeUnitLimit": true,
            "prioritizationFeeLamports": params.priority_fee_lamports,
            "asLegacyTransaction": false
        });

        debug!("Getting swap transaction from Jupiter");

        let response = self
            .http_client
            .post(&swap_url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Jupiter swap failed: {}", error_text));
        }

        let swap_response: Value = response.json().await?;

        let transaction_data = swap_response["swapTransaction"]
            .as_str()
            .ok_or_else(|| anyhow!("No swapTransaction in response"))?;

        use base64::{engine::general_purpose, Engine as _};
        let transaction_bytes = general_purpose::STANDARD.decode(transaction_data)?;
        let transaction: Transaction = bincode::deserialize(&transaction_bytes)?;

        Ok(transaction)
    }

    async fn send_transaction(&self, mut transaction: Transaction) -> Result<String> {
        // Get fresh blockhash
        let recent_blockhash = self.rpc_client.get_latest_blockhash().await?;

        // Sign transaction
        transaction.sign(&[self.keypair.as_ref()], recent_blockhash);

        // Send with confirmation
        info!("📡 Sending transaction...");

        let signature = self
            .rpc_client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .await?;

        info!("✅ Transaction confirmed: {}", signature);

        Ok(signature.to_string())
    }

    pub async fn get_token_balance(&self, _mint: &Pubkey) -> Result<u64> {
        // TODO: Implement proper SPL token balance fetching
        // For now, return 0 to allow compilation
        Ok(0)
    }

    pub async fn get_sol_balance(&self) -> Result<u64> {
        let balance = self.rpc_client.get_balance(&self.keypair.pubkey()).await?;

        Ok(balance)
    }
}

// Helper function to validate mint address
pub fn is_valid_mint(mint_str: &str) -> bool {
    mint_str.len() >= 32
        && mint_str.len() <= 44
        && mint_str.chars().all(|c| c.is_ascii_alphanumeric())
}
