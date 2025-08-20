#!/bin/bash
echo "🔐 Testing Ledger connection with Solana Arbitrage Bot..."

# Test with our bot
if [ -f "./target/release/solana-arbitrage-bot" ]; then
    echo "Testing bot Ledger integration..."
    ./target/release/solana-arbitrage-bot --test-ledger
else
    echo "Bot not compiled. Run: cargo build --release"
fi

# Test with Solana CLI
if command -v solana-keygen &> /dev/null; then
    echo ""
    echo "Testing Solana CLI Ledger integration..."
    echo "Please approve the request on your Ledger device..."
    timeout 30s solana-keygen pubkey usb://ledger?key=0/0
else
    echo "Solana CLI not found"
fi
