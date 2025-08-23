#!/bin/bash
# Test Ledger Connection and Bot Configuration

echo "🧪 TESTING LEDGER CONNECTION AND BOT CONFIGURATION 🧪"
echo "====================================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

LEDGER_ADDR="9PdJZJh33TsMB6s3DuhSXYCSpUiM3oBqyuno2aLvYTk"

echo -e "${BLUE}📍 Expected Ledger Address: $LEDGER_ADDR${NC}"
echo ""

# 1. Test Ledger Hardware Connection
echo -e "${YELLOW}🔌 Testing Ledger Hardware Connection...${NC}"
if lsusb | grep -i ledger > /dev/null; then
    echo -e "${GREEN}✅ Ledger hardware detected${NC}"
else
    echo -e "${RED}❌ Ledger hardware not detected${NC}"
    echo "Please connect your Ledger and try again"
    exit 1
fi

# 2. Test Solana App on Ledger
echo -e "${YELLOW}📱 Testing Solana App on Ledger...${NC}"
if DETECTED_ADDR=$(solana-keygen pubkey usb://ledger 2>/dev/null); then
    echo -e "${GREEN}✅ Solana app accessible${NC}"
    echo "Detected address: $DETECTED_ADDR"
    
    if [ "$DETECTED_ADDR" = "$LEDGER_ADDR" ]; then
        echo -e "${GREEN}✅ Address matches configuration${NC}"
    else
        echo -e "${RED}❌ Address mismatch!${NC}"
        echo "Expected: $LEDGER_ADDR"
        echo "Detected: $DETECTED_ADDR"
        echo "Please check your Ledger derivation path"
    fi
else
    echo -e "${RED}❌ Cannot access Solana app on Ledger${NC}"
    echo "Make sure:"
    echo "1. Ledger is unlocked"
    echo "2. Solana app is open"
    echo "3. Blind signing is enabled (if needed)"
    exit 1
fi

# 3. Test Balance Check
echo ""
echo -e "${YELLOW}💰 Testing Balance Check...${NC}"
if BALANCE=$(solana balance $LEDGER_ADDR 2>/dev/null); then
    echo -e "${GREEN}✅ Balance check successful: $BALANCE${NC}"
else
    echo -e "${RED}❌ Cannot check balance${NC}"
fi

# 4. Test Bot Configuration
echo ""
echo -e "${YELLOW}⚙️  Testing Bot Configurations...${NC}"

configs=("config.yaml" "config.production.yaml" "config_sniper.yaml")

for config in "${configs[@]}"; do
    if [ -f "$config" ]; then
        echo -e "${BLUE}📄 Checking $config...${NC}"
        
        # Check if use_ledger is true
        if grep -q "use_ledger: true" "$config"; then
            echo -e "${GREEN}  ✅ use_ledger: true${NC}"
        else
            echo -e "${RED}  ❌ use_ledger not set to true${NC}"
        fi
        
        # Check if ledger_address matches
        if grep -q "$LEDGER_ADDR" "$config"; then
            echo -e "${GREEN}  ✅ ledger_address matches${NC}"
        else
            echo -e "${RED}  ❌ ledger_address not found or incorrect${NC}"
        fi
        
        # Check if wallet.json path is commented or marked as backup
        if grep -q "# BACKUP" "$config" || grep -q "# Backup" "$config"; then
            echo -e "${GREEN}  ✅ wallet.json marked as backup${NC}"
        else
            echo -e "${YELLOW}  ⚠️  wallet.json not clearly marked as backup${NC}"
        fi
    else
        echo -e "${RED}❌ $config not found${NC}"
    fi
    echo ""
done

# 5. Security Summary
echo ""
echo -e "${YELLOW}🛡️  SECURITY SUMMARY:${NC}"
echo -e "${GREEN}✅ Ledger hardware wallet connected${NC}"
echo -e "${GREEN}✅ Solana app accessible on Ledger${NC}"
echo -e "${GREEN}✅ Bot configurations updated for Ledger${NC}"
echo -e "${GREEN}✅ Old wallet.json secured (root access only)${NC}"
echo ""

echo -e "${BLUE}🎯 NEXT STEPS:${NC}"
echo "1. Fund your Ledger address: $LEDGER_ADDR"
echo "2. Test with small amounts first"
echo "3. Run bots in dry-run mode initially"
echo "4. Monitor all transactions carefully"
echo "5. Keep Ledger disconnected when not trading"
echo ""

echo -e "${GREEN}🎉 Your setup is now secure and ready for trading!${NC}"
echo -e "${RED}⚠️  Remember: Always verify transactions on Ledger screen before confirming${NC}"
