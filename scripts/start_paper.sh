#!/bin/bash
# 📄 Start Paper Trading - Solana Arbitrage Bot

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_FILE="config.paper.yaml"
PID_FILE="$PROJECT_ROOT/paper.pid"
LOG_FILE="$PROJECT_ROOT/logs/paper.log"

echo -e "${BLUE}📄 Starting Solana Arbitrage Bot - PAPER TRADING MODE${NC}"
echo "=================================================="

# Safety checks
echo -e "\n${GREEN}🔍 Pre-flight Checks${NC}"

# Check if already running
if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo -e "${RED}❌ Paper trading bot is already running (PID: $(cat "$PID_FILE"))${NC}"
    echo "Use ./scripts/stop_paper.sh to stop first"
    exit 1
fi

# Create paper trading config if it doesn't exist
if [ ! -f "$PROJECT_ROOT/$CONFIG_FILE" ]; then
    echo -e "${YELLOW}⚠️  Creating paper trading configuration...${NC}"
    
    # Copy from production config and modify for paper trading
    if [ -f "$PROJECT_ROOT/config.production.enhanced.yaml" ]; then
        cp "$PROJECT_ROOT/config.production.enhanced.yaml" "$PROJECT_ROOT/$CONFIG_FILE"
    else
        echo -e "${RED}❌ No base configuration found${NC}"
        exit 1
    fi
    
    # Modify for paper trading
    cat > "$PROJECT_ROOT/$CONFIG_FILE" << 'EOF'
# 📄 Paper Trading Configuration - Solana Arbitrage Bot
# Safe simulation mode with no real money at risk

rpc:
  url: "https://api.mainnet-beta.solana.com"
  ws_url: "wss://api.mainnet-beta.solana.com"
  timeout_seconds: 30
  max_retries: 3

wallet:
  # Paper trading doesn't need real wallet
  path: "./wallet.paper.json"
  use_ledger: false
  ledger_path: "44'/501'/0'/0'"

dex:
  raydium:
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    sol_usdc_pool: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
    fee_percent: 0.25
    min_liquidity_sol: 1000.0
    
  orca:
    program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    sol_usdc_pool: "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"
    fee_percent: 0.30
    min_liquidity_sol: 1000.0

limits:
  # Paper trading limits (simulated $100 capital)
  max_position_sol: 0.5          # Smaller positions for testing
  min_profit_percent: 0.3        # Lower threshold for more opportunities
  min_profit_usd: 0.50          # Lower minimum profit
  max_slippage_percent: 0.5     # More lenient slippage
  
  # Daily limits
  max_daily_loss_usd: 20.0      # Simulated loss limit
  max_daily_trades: 100         # More trades for testing
  max_consecutive_losses: 5     # More lenient
  
  # Position sizing
  position_size_percent: 10.0   # Smaller positions
  max_position_percent: 15.0    # Conservative max

execution:
  # Paper trading settings
  priority_fee_lamports: 10000   # Lower priority for testing
  simulation_required: true      # Always simulate
  max_retries: 3                # More retries for testing
  timeout_seconds: 10           # Longer timeout
  
  # Confirmation settings
  commitment: "confirmed"
  skip_preflight: false         # Don't skip for testing
  max_retries_confirmation: 3

monitoring:
  enable_metrics: true
  metrics_interval_seconds: 30   # More frequent for testing
  log_level: "debug"            # Verbose logging
  
  health_check_interval_seconds: 15
  rpc_health_check: true
  wallet_balance_check: false   # Skip wallet checks

safety:
  enable_circuit_breaker: true
  circuit_breaker_loss_percent: 20.0  # More lenient
  circuit_breaker_error_count: 20
  
  check_market_volatility: false      # Skip volatility checks
  require_manual_restart: false       # Auto-restart for testing
  daily_report_required: true

discord:
  webhook_url: ""               # Disable Discord for paper trading
  enable_alerts: false

web:
  enable: true
  host: "127.0.0.1"
  port: 8081                    # Different port from production
  require_auth: false           # No auth for testing
  
  enable_trading_controls: true
  enable_real_time_charts: true
  enable_performance_metrics: true

database:
  url: "sqlite:./data/paper.db"
  max_connections: 5
  connection_timeout_seconds: 30

logging:
  level: "debug"
  file: "./logs/paper.log"
  max_file_size_mb: 50
  max_files: 5
  format: "json"

# Paper trading specific
environment: "paper"
debug_mode: true
dry_run: true                   # CRITICAL: No real transactions

features:
  enable_sniper: false
  enable_advanced_analytics: true
  enable_ml_scoring: false
  enable_multi_dex: true
  enable_auto_rebalancing: false

performance:
  calculator_threads: 2
  monitor_update_interval_ms: 200  # Slower for testing
  price_cache_duration_ms: 100
  max_concurrent_requests: 5
EOF
    
    echo -e "${GREEN}✅ Paper trading configuration created${NC}"
fi

# Check binary exists
if [ ! -f "$PROJECT_ROOT/target/release/solana-arbitrage-bot" ]; then
    echo -e "${RED}❌ Binary not found. Building...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release
fi

# Create log directory
mkdir -p "$(dirname "$LOG_FILE")"

# Start paper trading
echo -e "\n${GREEN}📄 Starting paper trading bot...${NC}"
echo "This is SIMULATION ONLY - no real money at risk"
echo "Configuration: $CONFIG_FILE"
echo "Dashboard: http://localhost:8081"
echo ""

# Export environment variables
export RUST_LOG=debug
export CONFIG_FILE="$CONFIG_FILE"

# Start bot in background
nohup "$PROJECT_ROOT/target/release/solana-arbitrage-bot" \
    --config "$PROJECT_ROOT/$CONFIG_FILE" \
    > "$LOG_FILE" 2>&1 &

BOT_PID=$!
echo $BOT_PID > "$PID_FILE"

# Wait and check if it started
sleep 3

if kill -0 $BOT_PID 2>/dev/null; then
    echo -e "${GREEN}✅ Paper trading bot started successfully!${NC}"
    echo "PID: $BOT_PID"
    echo "Log file: $LOG_FILE"
    echo ""
    echo "📊 Monitoring commands:"
    echo "  Status: ./scripts/status_paper.sh"
    echo "  Logs: tail -f $LOG_FILE"
    echo "  Stop: ./scripts/stop_paper.sh"
    echo "  Dashboard: http://localhost:8081"
    
    # Show initial log output
    echo -e "\n${BLUE}📋 Initial log output:${NC}"
    tail -n 10 "$LOG_FILE" || echo "No logs yet..."
    
else
    echo -e "${RED}❌ Failed to start paper trading bot${NC}"
    rm -f "$PID_FILE"
    echo "Check logs: $LOG_FILE"
    exit 1
fi

echo -e "\n${GREEN}🎉 Paper trading is now running!${NC}"
echo "Monitor for 24-48 hours before considering production deployment."
