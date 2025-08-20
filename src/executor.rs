//! Transaction Executor with Ledger Support
//! Secure hardware wallet integration for production

use anyhow::{Result, anyhow};
use log::{info, warn, error};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
    pubkey::Pubkey,
    instruction::Instruction,
    commitment_config::CommitmentConfig,
    message::Message,
};
use solana_remote_wallet::{
    ledger::LedgerWallet,
    remote_wallet::{RemoteWallet, RemoteWalletInfo},
    locator::{Locator, Manufacturer},
};
use std::sync::Arc;
use std::str::FromStr;
use crate::calculator::ArbitrageOpportunity;

pub enum WalletType {
    Keypair(Keypair),
    Ledger(Arc<LedgerWallet>, Pubkey, String), // wallet, pubkey, derivation_path
}

pub struct TransactionExecutor {
    rpc_client: RpcClient,
    wallet: WalletType,
    dry_run: bool,
    priority_fee: u64,
    simulation_required: bool,
    requires_confirmation: bool, // Dla Ledger - wymaga potwierdzenia na urządzeniu
}

impl TransactionExecutor {
    pub fn new(config: &crate::Config, dry_run: bool) -> Result<Self> {
        // Sprawdź czy używamy Ledger czy zwykły keypair
        let wallet = if config.wallet.use_ledger.unwrap_or(false) {
            Self::setup_ledger_wallet(&config.wallet.ledger_path.as_deref().unwrap_or("m/44'/501'/0'/0'"))?
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
            WalletType::Ledger(_, pk, _) => *pk,
        };
        
        info!("💳 Wallet loaded: {}", pubkey);
        info!("🔐 Wallet type: {}", match wallet {
            WalletType::Keypair(_) => "Software Keypair",
            WalletType::Ledger(_, _, _) => "Hardware Ledger",
        });
        info!("🌐 RPC endpoint: {}", config.rpc.url);
        
        Ok(Self {
            rpc_client,
            wallet,
            dry_run,
            priority_fee: config.execution.priority_fee_lamports,
            simulation_required: config.execution.simulation_required,
            requires_confirmation: config.wallet.use_ledger.unwrap_or(false),
        })
    }
    
    fn setup_ledger_wallet(derivation_path: &str) -> Result<WalletType> {
        info!("🔐 Connecting to Ledger Nano S+...");
        
        // Domyślna ścieżka dla Solana
        let derivation_path = derivation_path.to_string();
        
        // For now, return an error indicating Ledger support is not fully implemented
        // This will be completed in later tasks
        Err(anyhow::anyhow!(
            "Ledger support is not fully implemented yet. \
            Please use software wallet for now or wait for task completion."
        ))
    }
    
    fn setup_keypair_wallet(path: &str) -> Result<WalletType> {
        let wallet_bytes = std::fs::read(path)
            .map_err(|e| anyhow!("Failed to read wallet from {}: {}", path, e))?;
        let wallet = Keypair::try_from(&wallet_bytes[..])
            .map_err(|e| anyhow!("Invalid wallet format: {}", e))?;
        
        warn!("⚠️ Using software keypair - consider using Ledger for production!");
        
        Ok(WalletType::Keypair(wallet))
    }
    
    pub async fn execute_arbitrage(&self, opportunity: &ArbitrageOpportunity) -> Result<Signature> {
        info!(
            "🔨 Building arbitrage transaction: {} SOL {} -> {}",
            opportunity.amount_sol,
            opportunity.buy_dex,
            opportunity.sell_dex
        );
        
        // Build the transaction
        let instructions = self.build_arbitrage_instructions(opportunity)?;
        
        // Get recent blockhash
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        // Get payer pubkey
        let payer = match &self.wallet {
            WalletType::Keypair(kp) => kp.pubkey(),
            WalletType::Ledger(_, pk, _) => *pk,
        };
        
        // Create message
        let message = Message::new_with_blockhash(
            &instructions,
            Some(&payer),
            &recent_blockhash,
        );
        
        // Sign transaction based on wallet type
        let transaction = match &self.wallet {
            WalletType::Keypair(kp) => {
                Transaction::new(&[kp], message, recent_blockhash)
            }
            WalletType::Ledger(_ledger, _pubkey, _derivation_path) => {
                // For now, return an error since full Ledger integration is not complete
                return Err(anyhow::anyhow!(
                    "Ledger transaction signing not yet implemented. \
                    This will be completed in subsequent tasks."
                ));
            }
        };
        
        // Simulate if required
        if self.simulation_required {
            info!("🔬 Simulating transaction...");
            match self.rpc_client.simulate_transaction(&transaction) {
                Ok(result) => {
                    if result.value.err.is_some() {
                        error!("❌ Simulation failed: {:?}", result.value.err);
                        return Err(anyhow!("Transaction simulation failed"));
                    }
                    info!("✅ Simulation successful! Units consumed: {:?}", result.value.units_consumed);
                }
                Err(e) => {
                    error!("❌ Simulation error: {}", e);
                    return Err(anyhow!("Failed to simulate transaction: {}", e));
                }
            }
        }
        
        // Safety check dla Ledger - ostatnie potwierdzenie
        if self.requires_confirmation && !self.dry_run {
            warn!("⚠️ OSTATNIE OSTRZEŻENIE: Transakcja za ${:.2} zostanie wykonana!",
                 opportunity.expected_profit_usd);
            info!("Czekam 3 sekundy na ewentualne przerwanie (Ctrl+C)...");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
        
        // Execute or dry run
        if self.dry_run {
            warn!("🏃 DRY RUN - Transaction would be sent");
            Ok(Signature::default())
        } else {
            info!("📤 Sending transaction...");
            let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
            info!("✅ Transaction confirmed: {}", signature);
            Ok(signature)
        }
    }
    
    fn build_arbitrage_instructions(&self, opportunity: &ArbitrageOpportunity) -> Result<Vec<Instruction>> {
        // NOTE: This is a simplified example
        // Real implementation would:
        // 1. Use Jupiter Aggregator for optimal routing
        // 2. Build atomic swap instructions
        // 3. Add priority fee instruction
        // 4. Handle token accounts properly
        
        let mut instructions = vec![];
        
        // Add priority fee instruction (for faster inclusion)
        instructions.push(self.build_priority_fee_instruction()?);
        
        // Add buy instruction (simplified - would use actual DEX program)
        instructions.push(self.build_swap_instruction(
            &opportunity.buy_dex,
            opportunity.amount_sol,
            true, // buying SOL
        )?);
        
        // Add sell instruction
        instructions.push(self.build_swap_instruction(
            &opportunity.sell_dex,
            opportunity.amount_sol,
            false, // selling SOL
        )?);
        
        Ok(instructions)
    }
    
    fn build_priority_fee_instruction(&self) -> Result<Instruction> {
        // Use actual compute budget program
        use solana_sdk::compute_budget::ComputeBudgetInstruction;
        
        Ok(ComputeBudgetInstruction::set_compute_unit_price(
            self.priority_fee
        ))
    }
    
    fn build_swap_instruction(
        &self,
        dex: &str,
        amount: f64,
        is_buy: bool,
    ) -> Result<Instruction> {
        // NOTE: Simplified placeholder
        // Real implementation would:
        // 1. Build proper swap instruction for Raydium/Orca
        // 2. Calculate exact amounts with decimals
        // 3. Include proper accounts and data
        
        let program_id = match dex {
            "Raydium" => Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?,
            "Orca" => Pubkey::from_str("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP")?,
            _ => return Err(anyhow!("Unknown DEX: {}", dex)),
        };
        
        // This is just a placeholder - real swap instruction would be much more complex
        Ok(Instruction::new_with_bytes(
            program_id,
            &[],
            vec![],
        ))
    }
}
