#!/bin/bash
# 🚀 Start Production Trading - Solana Arbitrage Bot

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="config.production.enhanced.yaml"
PID_FILE="$PROJECT_ROOT/production.pid"
LOG_FILE="$PROJECT_ROOT/logs/production.log"

echo -e "${BLUE}🚀 Starting Solana Arbitrage Bot - PRODUCTION MODE${NC}"
echo "=================================================="

# Safety checks
echo -e "\n${YELLOW}🔍 Pre-flight Safety Checks${NC}"

# Check if already running
if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo -e "${RED}❌ Bot is already running (PID: $(cat "$PID_FILE"))${NC}"
    echo "Use ./scripts/stop_production.sh to stop first"
    exit 1
fi

# Check configuration exists
if [ ! -f "$PROJECT_ROOT/$CONFIG_FILE" ]; then
    echo -e "${RED}❌ Production config not found: $CONFIG_FILE${NC}"
    exit 1
fi

# Check environment file
if [ ! -f "$PROJECT_ROOT/.env.production" ]; then
    echo -e "${RED}❌ Environment file not found: .env.production${NC}"
    echo "Run ./scripts/deploy_production.sh first"
    exit 1
fi

# Load environment variables
source "$PROJECT_ROOT/.env.production"

# Check critical environment variables
if [ -z "${BOT__RPC__URL:-}" ]; then
    echo -e "${RED}❌ BOT__RPC__URL not set in .env.production${NC}"
    exit 1
fi

# Check wallet setup
if [ "${BOT__WALLET__USE_LEDGER:-false}" = "true" ]; then
    echo -e "${GREEN}✅ Using Ledger hardware wallet (secure)${NC}"
else
    echo -e "${YELLOW}⚠️  WARNING: Not using hardware wallet${NC}"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check binary exists
if [ ! -f "$PROJECT_ROOT/target/release/solana-arbitrage-bot" ]; then
    echo -e "${RED}❌ Production binary not found${NC}"
    echo "Run 'cargo build --release' first"
    exit 1
fi

# Final confirmation
echo -e "\n${YELLOW}⚠️  PRODUCTION TRADING CONFIRMATION${NC}"
echo "This will start LIVE trading with REAL money!"
echo "Configuration: $CONFIG_FILE"
echo "Max daily loss: ${BOT__LIMITS__MAX_DAILY_LOSS_USD:-10.0} USD"
echo "Max position: ${BOT__LIMITS__MAX_POSITION_SOL:-2.0} SOL"
echo ""
read -p "Are you sure you want to start PRODUCTION trading? (yes/no): " -r
if [[ ! $REPLY =~ ^yes$ ]]; then
    echo "Aborted by user"
    exit 1
fi

# Create log directory
mkdir -p "$(dirname "$LOG_FILE")"

# Start the bot
echo -e "\n${GREEN}🚀 Starting production bot...${NC}"

# Export environment variables
export RUST_LOG=info
export CONFIG_FILE="$CONFIG_FILE"

# Start bot in background
nohup "$PROJECT_ROOT/target/release/solana-arbitrage-bot" \
    --config "$PROJECT_ROOT/$CONFIG_FILE" \
    > "$LOG_FILE" 2>&1 &

BOT_PID=$!
echo $BOT_PID > "$PID_FILE"

# Wait a moment and check if it started successfully
sleep 3

if kill -0 $BOT_PID 2>/dev/null; then
    echo -e "${GREEN}✅ Bot started successfully!${NC}"
    echo "PID: $BOT_PID"
    echo "Log file: $LOG_FILE"
    echo ""
    echo "📊 Monitoring commands:"
    echo "  Status: ./scripts/status.sh"
    echo "  Logs: tail -f $LOG_FILE"
    echo "  Stop: ./scripts/stop_production.sh"
    echo "  Monitor: ./scripts/monitor.sh"
    
    # Show initial log output
    echo -e "\n${BLUE}📋 Initial log output:${NC}"
    tail -n 10 "$LOG_FILE" || echo "No logs yet..."
    
else
    echo -e "${RED}❌ Failed to start bot${NC}"
    rm -f "$PID_FILE"
    echo "Check logs: $LOG_FILE"
    exit 1
fi

echo -e "\n${GREEN}🎉 Production bot is now running!${NC}"
echo "Monitor performance closely for the first few hours."
