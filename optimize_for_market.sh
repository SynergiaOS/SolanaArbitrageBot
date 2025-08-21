#!/bin/bash
# 🚀 Quick Optimization for Current Market Conditions

echo "🔧 Optimizing bot for current tight spreads..."

# Create optimized config
cat > config_optimized.yaml << 'EOF'
# Optimized for tight spreads (0.005% - 0.1%)
rpc:
  url: "https://api.mainnet-beta.solana.com"
  ws_url: "wss://api.mainnet-beta.solana.com"

wallet:
  use_ledger: false
  path: "./wallet.json"

dex:
  raydium:
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    sol_usdc_pool: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
  orca:
    program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    sol_usdc_pool: "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"

limits:
  # OPTIMIZED FOR TIGHT SPREADS
  max_position_sol: 50.0         # Increased 5x for better profits
  min_profit_percent: 0.05       # Lowered to 0.05% (from 0.3%)
  min_profit_usd: 0.25           # Lowered to $0.25 (from $1)
  max_slippage_percent: 0.2      # Tighter slippage control
  max_daily_loss_usd: 100.0
  max_daily_trades: 500          # More trades allowed

execution:
  priority_fee_lamports: 5000    # Lower fee for small profits
  simulation_required: false     # Skip simulation for speed
  max_retries: 2
EOF

echo "✅ Config optimized for tight spreads!"
echo ""
echo "📊 New Settings:"
echo "  - Position: 50 SOL (was 10)"
echo "  - Min profit: 0.05% (was 0.3%)" 
echo "  - Min USD: $0.25 (was $1)"
echo "  - Faster execution (no simulation)"
echo ""
echo "🚀 Starting bot with optimized settings..."
echo ""

# Run bot with new config
./target/release/solana-arbitrage-bot \
    --config config_optimized.yaml \
    --dry-run \
    --max-position 50 2>&1 | tee optimized_run.log &

BOT_PID=$!
echo "Bot PID: $BOT_PID"
echo ""

# Monitor for opportunities
echo "👀 Monitoring for opportunities..."
echo "─────────────────────────────────"

while true; do
    # Check latest logs
    OPPORTUNITIES=$(grep -c "Opportunity" optimized_run.log 2>/dev/null || echo "0")
    LAST_PRICE=$(grep "Price update" optimized_run.log 2>/dev/null | tail -n 1 || echo "Waiting...")
    
    # Clear and show status
    printf "\r📊 Opportunities found: %s | %s" "$OPPORTUNITIES" "$LAST_PRICE"
    
    # Check if opportunity found
    if grep -q "Opportunity" optimized_run.log 2>/dev/null; then
        echo ""
        echo "🎯 OPPORTUNITY DETECTED!"
        grep "Opportunity" optimized_run.log | tail -n 1
        echo ""
    fi
    
    sleep 2
done
