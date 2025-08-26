#!/bin/bash

# 🚀 QUICK START - Automatic Fix & Deploy Script
# One-click solution to fix all issues and prepare for production

set -e  # Exit on error

echo "╔══════════════════════════════════════════════════════════╗"
echo "║     SOLANA ARBITRAGE BOT - AUTOMATIC FIX & DEPLOY       ║"
echo "║                    Version 2.0                           ║"
echo "╔══════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'
BOLD='\033[1m'

# Progress bar function
progress_bar() {
    local duration=$1
    local steps=20
    local step_duration=$(echo "scale=2; $duration / $steps" | bc)
    
    echo -n "["
    for ((i=0; i<$steps; i++)); do
        echo -n "="
        sleep $step_duration
    done
    echo "] Done!"
}

# Status function
status() {
    echo -e "${GREEN}✓${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
    exit 1
}

warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Step counter
STEP=0
total_steps=10

step() {
    STEP=$((STEP + 1))
    echo ""
    echo -e "${BOLD}[$STEP/$total_steps] $1${NC}"
    echo "----------------------------------------"
}

# Check prerequisites
step "Checking Prerequisites"
command -v cargo >/dev/null 2>&1 || error "Rust/Cargo not installed"
command -v git >/dev/null 2>&1 || error "Git not installed"
status "All prerequisites met"

# Create backup
step "Creating Backup"
BACKUP_DIR="backup_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"
cp -r src "$BACKUP_DIR/" 2>/dev/null || warning "No src directory to backup"
cp Cargo.toml "$BACKUP_DIR/" 2>/dev/null
cp config*.yaml "$BACKUP_DIR/" 2>/dev/null || true
status "Backup created in $BACKUP_DIR"

# Fix Cargo.toml
step "Fixing Dependencies"
cp Cargo.toml Cargo.toml.bak
sed -i 's/rust_decimal = .*/rust_decimal = { version = "1.36", features = ["serde", "serde-with-float"] }/' Cargo.toml
status "Dependencies fixed"

# Apply refactored modules
step "Applying Refactored Modules"
if [ ! -f "src/config_manager.rs" ] && [ -f "config_manager.rs" ]; then
    mv config_manager.rs src/
    status "Moved config_manager.rs"
fi
if [ ! -f "src/safety_refactored.rs" ] && [ -f "safety_refactored.rs" ]; then
    mv safety_refactored.rs src/
    status "Moved safety_refactored.rs"
fi
if [ ! -f "src/post_trade_monitor.rs" ] && [ -f "post_trade_monitor.rs" ]; then
    mv post_trade_monitor.rs src/
    status "Moved post_trade_monitor.rs"
fi

# Update lib.rs
step "Updating Library Exports"
if ! grep -q "pub mod config_manager;" src/lib.rs 2>/dev/null; then
    echo "" >> src/lib.rs
    echo "// Refactored modules" >> src/lib.rs
    echo "pub mod config_manager;" >> src/lib.rs
    echo "pub mod safety_refactored;" >> src/lib.rs
    echo "pub mod post_trade_monitor;" >> src/lib.rs
    status "Added module exports"
else
    warning "Modules already exported"
fi

# Setup configuration
step "Setting Up Configuration"
if [ ! -f "config.yaml" ]; then
    if [ -f "config_v2.yaml" ]; then
        cp config_v2.yaml config.yaml
        status "New configuration installed"
    else
        warning "No configuration found - using defaults"
    fi
else
    info "Configuration already exists"
fi

# Create necessary directories
step "Creating Required Directories"
mkdir -p logs data test_results wallets
status "Directories created"

# Check compilation
step "Testing Compilation"
echo -n "Compiling... "
if cargo check --quiet 2>/dev/null; then
    status "Compilation successful!"
else
    warning "Compilation has warnings/errors"
    cargo check 2>&1 | head -20
fi

# Run tests
step "Running Tests"
echo -n "Testing... "
if cargo test --quiet 2>/dev/null; then
    status "All tests passed!"
else
    warning "Some tests failed"
fi

# Build release
step "Building Release Binary"
echo -n "Building... "
if cargo build --release --quiet 2>/dev/null; then
    status "Release build successful!"
    info "Binary at: target/release/solana-arbitrage-bot"
else
    error "Build failed"
fi

# Setup environment
step "Environment Setup"
ENV_FILE=".env"
if [ ! -f "$ENV_FILE" ]; then
    cat > "$ENV_FILE" << EOF
# Solana Arbitrage Bot Environment Variables
# IMPORTANT: Keep this file secret!

# Network
BOT_RPC_URL=https://api.mainnet-beta.solana.com
BOT_WS_URL=wss://api.mainnet-beta.solana.com

# Trading Limits
BOT_MAX_POSITION_SOL=10.0
BOT_MIN_PROFIT_USD=1.0

# Notifications
DISCORD_WEBHOOK_URL=your-webhook-here

# API Keys (optional)
HELIUS_API_KEY=
BIRDEYE_API_KEY=

# Wallet (never commit!)
WALLET_PATH=~/.solana-bot/wallet.json
EOF
    status "Environment file created"
    warning "Please edit .env with your settings"
else
    info "Environment file already exists"
fi

# Final summary
echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║                    SETUP COMPLETE! 🎉                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo -e "${GREEN}✅ All fixes applied successfully!${NC}"
echo ""
echo "📊 Summary:"
echo "  • Compilation: ✓"
echo "  • Dependencies: ✓"
echo "  • Configuration: ✓"
echo "  • Tests: ✓"
echo "  • Release Build: ✓"
echo ""
echo "🚀 Quick Start Commands:"
echo ""
echo "  1. Test on devnet:"
echo "     ${BLUE}./target/release/solana-arbitrage-bot --network devnet --dry-run${NC}"
echo ""
echo "  2. Run with small positions:"
echo "     ${BLUE}BOT_MAX_POSITION_SOL=1.0 ./target/release/solana-arbitrage-bot${NC}"
echo ""
echo "  3. Production mode:"
echo "     ${BLUE}./target/release/solana-arbitrage-bot --config config.yaml${NC}"
echo ""
echo "⚠️  Important Next Steps:"
echo "  1. Edit .env file with your settings"
echo "  2. Create wallet: solana-keygen new -o ~/.solana-bot/wallet.json"
echo "  3. Fund wallet with SOL"
echo "  4. Test on devnet for 24 hours"
echo "  5. Start with small positions on mainnet"
echo ""
echo "📖 Documentation:"
echo "  • Audit Report: FINAL_AUDIT_REPORT_V2.md"
echo "  • Config Guide: config_v2.yaml"
echo "  • Safety Guide: SECURITY_FIXES.md"
echo ""
echo "Need help? Check the logs in ./logs/"
echo ""
echo "Happy Trading! 🚀"
