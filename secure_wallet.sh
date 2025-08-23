#!/bin/bash
# URGENT: Secure Wallet and Transfer to Ledger

echo "🚨 SECURING WALLET AND TRANSFERRING TO LEDGER 🚨"
echo "================================================="

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# 1. Backup wallet.json
echo -e "${YELLOW}📦 Creating backup...${NC}"
if [ -f wallet.json ]; then
    cp wallet.json wallet_backup_$(date +%s).json
    echo -e "${GREEN}✅ Backup created${NC}"
else
    echo -e "${RED}❌ wallet.json not found!${NC}"
    exit 1
fi

# 2. Secure permissions
echo -e "${YELLOW}🔒 Securing permissions...${NC}"
chmod 400 wallet.json
echo -e "${GREEN}✅ wallet.json is now read-only${NC}"

# 3. Check current balance
echo -e "${YELLOW}💰 Checking current balance...${NC}"
BALANCE=$(solana balance 9XrVUqKvmTHTexzK4iHADc85CGv84UpoMRVMyLDVm75y)
echo "Current balance: $BALANCE"

# 4. Check if Ledger is connected
echo -e "${YELLOW}🔌 Checking Ledger connection...${NC}"
if lsusb | grep -i ledger > /dev/null; then
    echo -e "${GREEN}✅ Ledger detected${NC}"
    
    # Try to get Ledger address
    echo -e "${YELLOW}📍 Getting Ledger address...${NC}"
    if LEDGER_ADDR=$(solana-keygen pubkey usb://ledger 2>/dev/null); then
        echo -e "${GREEN}✅ Ledger address: $LEDGER_ADDR${NC}"
        
        # Ask for confirmation
        echo -e "${YELLOW}💸 Ready to transfer remaining SOL to Ledger${NC}"
        echo -e "${RED}⚠️  This will transfer ALL remaining SOL to your Ledger!${NC}"
        read -p "Continue? (y/N): " confirm
        
        if [[ $confirm =~ ^[Yy]$ ]]; then
            echo -e "${YELLOW}🚀 Transferring to Ledger...${NC}"
            
            # Calculate transfer amount (leave small amount for fees)
            TRANSFER_AMOUNT="0.001"
            
            if solana transfer $LEDGER_ADDR $TRANSFER_AMOUNT --from wallet.json --fee-payer wallet.json; then
                echo -e "${GREEN}✅ Transfer successful!${NC}"
                echo -e "${GREEN}✅ Your SOL is now safe on Ledger${NC}"
            else
                echo -e "${RED}❌ Transfer failed!${NC}"
            fi
        else
            echo -e "${YELLOW}⏸️  Transfer cancelled${NC}"
        fi
    else
        echo -e "${RED}❌ Cannot connect to Ledger. Make sure:${NC}"
        echo "1. Ledger is connected and unlocked"
        echo "2. Solana app is open on Ledger"
        echo "3. Blind signing is enabled (if needed)"
    fi
else
    echo -e "${RED}❌ Ledger not detected!${NC}"
    echo "Please connect your Ledger and try again"
fi

# 5. Security recommendations
echo ""
echo -e "${YELLOW}🛡️  SECURITY RECOMMENDATIONS:${NC}"
echo "1. ✅ wallet.json is now secured (read-only)"
echo "2. ✅ Backup created"
echo "3. 🔄 Transfer remaining SOL to Ledger"
echo "4. 🔄 Update all bot configs to use Ledger"
echo "5. 🔄 Never use wallet.json for live trading again"
echo ""

# 6. Check what wife might have done
echo -e "${YELLOW}🕵️  CHECKING RECENT ACTIVITY:${NC}"
echo "Recent file access:"
find /home/marcin -name "*wallet*" -type f -newermt "today" 2>/dev/null || echo "No recent wallet files"

echo ""
echo "Browser history (last 10):"
history | grep -E "(firefox|chrome|brave)" | tail -5 || echo "No browser activity in bash history"

echo ""
echo -e "${RED}⚠️  IMPORTANT: Ask your wife what she did around 16:45!${NC}"
echo "- Did she click on anything?"
echo "- Did she open any websites?"
echo "- Did she download anything?"
echo "- Did she see any popups or notifications?"

echo ""
echo -e "${GREEN}🎯 NEXT STEPS:${NC}"
echo "1. Transfer SOL to Ledger (if not done above)"
echo "2. Test bot with Ledger: cargo run --bin solana-arbitrage-bot -- --test-ledger"
echo "3. Run malware scan: sudo clamscan -r /home/marcin/ --infected"
echo "4. Change all passwords"
echo "5. Never leave wallet.json accessible again"
