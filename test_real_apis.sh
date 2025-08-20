#!/bin/bash
# Quick test script for new API integrations

echo "🔧 Building with new features..."
cargo build --release 2>&1 | tail -20

echo ""
echo "🧪 Running integration tests..."
cargo test --test integration -- --nocapture 2>&1 | grep -E "(test result:|Testing|✅|⚠️|Jupiter|Raydium|Orca)"

echo ""
echo "🌐 Testing API connections..."
./target/release/solana-arbitrage-bot --test-apis 2>&1 | grep -E "(Testing|✅|⚠️|OK|error)"

echo ""
echo "📊 Testing dry-run mode with real data..."
timeout 10 ./target/release/solana-arbitrage-bot --dry-run 2>&1 | grep -E "(Starting|Price|Opportunity|Monitoring|SOL|USDC)" || true

echo ""
echo "✅ Test complete!"
