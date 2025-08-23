#!/bin/bash
# URGENT: Investigate Unauthorized Transaction Script

echo "🚨 INVESTIGATING UNAUTHORIZED TRANSACTION 🚨"
echo "=============================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get wallet address
read -p "Enter your wallet address: " WALLET_ADDRESS

if [ -z "$WALLET_ADDRESS" ]; then
    echo -e "${RED}❌ Wallet address required!${NC}"
    exit 1
fi

echo -e "${YELLOW}🔍 Analyzing wallet: $WALLET_ADDRESS${NC}"
echo ""

# Check current balance
echo -e "${YELLOW}💰 Current Balance:${NC}"
solana balance $WALLET_ADDRESS
echo ""

# Get recent transactions
echo -e "${YELLOW}📋 Recent Transactions (last 20):${NC}"
solana transaction-history $WALLET_ADDRESS --limit 20
echo ""

# Check for transactions around 16:45 today
echo -e "${YELLOW}🕐 Looking for transactions around 16:45...${NC}"
echo "Please check the transaction list above for any around 16:45"
echo ""

# Security recommendations
echo -e "${RED}🛡️  IMMEDIATE SECURITY ACTIONS:${NC}"
echo "1. ✅ Disconnect Ledger from computer"
echo "2. ✅ Scan computer for malware"
echo "3. ✅ Check Ledger Live app authenticity"
echo "4. ✅ If funds remain - transfer to NEW wallet immediately"
echo "5. ✅ Never reuse this wallet if compromised"
echo ""

# Bot analysis
echo -e "${YELLOW}🤖 Checking if our bot was involved:${NC}"
if pgrep -f "solana-arbitrage-bot" > /dev/null; then
    echo -e "${RED}⚠️  Arbitrage bot is currently running${NC}"
    echo "Check logs: tail -f logs/bot.log"
else
    echo -e "${GREEN}✅ Arbitrage bot is not running${NC}"
fi

if pgrep -f "sniper" > /dev/null; then
    echo -e "${RED}⚠️  Sniper bot is currently running${NC}"
else
    echo -e "${GREEN}✅ Sniper bot is not running${NC}"
fi

echo ""
echo -e "${YELLOW}🔗 Explorer Links:${NC}"
echo "Solana Explorer: https://explorer.solana.com/address/$WALLET_ADDRESS"
echo "SolScan: https://solscan.io/account/$WALLET_ADDRESS"
echo ""

echo -e "${RED}⚠️  If you see unauthorized transactions:${NC}"
echo "1. 🚨 STOP using this wallet immediately"
echo "2. 📱 Create new wallet on different device"
echo "3. 💸 Transfer remaining funds to new wallet"
echo "4. 🔍 Report to relevant authorities if significant loss"
echo "5. 🛡️  Review all connected dApps and revoke permissions"
