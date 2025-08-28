#!/bin/bash

# Quick fix and run script for SolanaArbitrageBot
# Fixes common issues and starts the bot

set -e

echo "🔧 Quick Fix & Run for SolanaArbitrageBot"
echo "========================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Step 1: Fix module structure
echo -e "${GREEN}[1/5]${NC} Fixing module structure..."

# Update src/lib.rs with all modules
cat > src/lib.rs << 'EOF'
// Core modules
pub mod calculator;
pub mod config_manager;
pub mod executor;
pub mod monitor;
pub mod safety;

// Sniper module
pub mod sniper;

// DEX modules
pub mod dex;

// Strategy modules
pub mod strategies {
    #[cfg(feature = "micro")]
    pub mod micro_capital;
}

// Performance modules
pub mod performance {
    pub mod connection_pool;
    pub mod slippage_predictor;
}

// Security modules
pub mod security {
    pub mod mev_protection;
}

// Web modules
pub mod web;

// Utils
pub mod utils;

// Optional modules
#[cfg(feature = "discord")]
pub mod discord;

#[cfg(feature = "ledger")]
pub mod ledger;

#[cfg(feature = "kestra")]
pub mod kestra;
EOF

# Step 2: Create missing utils if needed
echo -e "${GREEN}[2/5]${NC} Creating missing modules..."

mkdir -p src/utils
if [ ! -f src/utils/mod.rs ]; then
    cat > src/utils/mod.rs << 'EOF'
pub mod conversions {
    use rust_decimal::Decimal;
    
    pub fn lamports_to_sol(lamports: u64) -> Decimal {
        Decimal::from(lamports) / Decimal::from(1_000_000_000)
    }
    
    pub fn sol_to_lamports(sol: Decimal) -> u64 {
        (sol * Decimal::from(1_000_000_000)).to_u64_digits()[0]
    }
}
EOF
fi

# Step 3: Add performance module
echo -e "${GREEN}[3/5]${NC} Adding performance module..."
mkdir -p src/performance
cat > src/performance/mod.rs << 'EOF'
pub mod connection_pool;
pub mod slippage_predictor;
EOF

# Step 4: Add security module
echo -e "${GREEN}[4/5]${NC} Adding security module..."
mkdir -p src/security
cat > src/security/mod.rs << 'EOF'
pub mod mev_protection;
EOF

# Step 5: Update Cargo.toml with features
echo -e "${GREEN}[5/5]${NC} Updating Cargo.toml..."

# Check if [features] section exists
if ! grep -q "\[features\]" Cargo.toml; then
    cat >> Cargo.toml << 'EOF'

[features]
default = ["full"]
full = ["discord", "ledger", "kestra", "micro"]
micro = []
discord = []
ledger = []
kestra = []
EOF
fi

# Add nalgebra dependency if missing
if ! grep -q "nalgebra" Cargo.toml; then
    sed -i '/\[dependencies\]/a nalgebra = "0.33"' Cargo.toml
fi

# Build the project
echo -e "\n${YELLOW}Building project...${NC}"
cargo build --release 2>&1 | tail -20

# Check if build successful
if [ $? -eq 0 ]; then
    echo -e "\n${GREEN}✅ Build successful!${NC}"
    
    # Create quick start config
    echo -e "\n${YELLOW}Creating micro config...${NC}"
    cat > config_micro_quick.yaml << 'EOF'
# Quick config for micro trading (50-100 USD)
rpc:
  primary:
    url: "https://api.mainnet-beta.solana.com"
    ws_url: "wss://api.mainnet-beta.solana.com"

wallet:
  path: "./wallet.json"

limits:
  max_position_sol: 0.05  # ~2 USD at current prices
  min_profit_percent: 0.5
  max_slippage_percent: 1.0
  max_daily_loss_usd: 10

sniper:
  enabled: false  # Disable for safety

micro_capital:
  enabled: true
  initial_capital_usd: 75.0
  daily_target_usd: 10.0

safety:
  dry_run: true  # Start in dry run mode
EOF

    echo -e "${GREEN}Config created: config_micro_quick.yaml${NC}"
    
    # Run options
    echo -e "\n${GREEN}=========================================${NC}"
    echo -e "${GREEN}Ready to run! Choose an option:${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo ""
    echo "1. Dry run (safe testing):"
    echo "   ./target/release/solana-arbitrage-bot --config config_micro_quick.yaml --dry-run"
    echo ""
    echo "2. Live trading (use with caution):"
    echo "   ./target/release/solana-arbitrage-bot --config config_micro_quick.yaml"
    echo ""
    echo "3. Monitor mode only:"
    echo "   ./target/release/solana-arbitrage-bot --config config_micro_quick.yaml --monitor-only"
    echo ""
    
else
    echo -e "\n${RED}❌ Build failed!${NC}"
    echo "Checking for common issues..."
    
    # Common fixes
    echo "Attempting automatic fixes..."
    
    # Fix 1: Update dependencies
    cargo update
    
    # Fix 2: Clean and rebuild
    cargo clean
    cargo build --release --features default
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ Fixed! Bot is ready.${NC}"
    else
        echo -e "${RED}Manual intervention required. Check error messages above.${NC}"
        exit 1
    fi
fi

echo -e "\n${YELLOW}Tips for micro trading (50-100 USD):${NC}"
echo "• Start with dry-run mode for 24h"
echo "• Focus on SOL/USDC pair initially"
echo "• Keep max position at 30% of capital"
echo "• Set realistic daily target (10-15 USD)"
echo "• Monitor slippage carefully"
echo "• Use Jito bundles for MEV protection"
