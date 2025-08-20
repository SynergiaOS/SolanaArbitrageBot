#!/bin/bash

# Test script for Ledger integration
# This script tests the basic Ledger connection functionality

set -e

echo "🔐 Ledger Integration Test Script"
echo "================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Check if dependencies are installed
echo -e "\n${YELLOW}Step 1: Checking dependencies...${NC}"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust/Cargo not found. Please install Rust first.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Rust/Cargo found${NC}"

# Check if USB libraries are available
if ! pkg-config --exists libusb-1.0; then
    echo -e "${RED}❌ libusb-1.0 not found. Run: sudo apt-get install libusb-1.0-0-dev${NC}"
    exit 1
fi
echo -e "${GREEN}✅ USB libraries found${NC}"

# Step 2: Build the project
echo -e "\n${YELLOW}Step 2: Building project...${NC}"
if cargo build --release; then
    echo -e "${GREEN}✅ Project built successfully${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Step 3: Check if Ledger is connected
echo -e "\n${YELLOW}Step 3: Checking USB connection...${NC}"
if lsusb | grep -q "Ledger"; then
    echo -e "${GREEN}✅ Ledger device detected via USB${NC}"
else
    echo -e "${YELLOW}⚠️ No Ledger device detected via USB${NC}"
    echo "Please ensure:"
    echo "1. Ledger is connected via USB"
    echo "2. Ledger is unlocked"
    echo "3. You have proper USB permissions"
fi

# Step 4: Test Ledger connection (if available)
echo -e "\n${YELLOW}Step 4: Testing Ledger connection...${NC}"
echo "This will test the connection to your Ledger device."
echo "Make sure:"
echo "1. Ledger is connected and unlocked"
echo "2. Solana app is open on Ledger"
echo "3. Blind signing is enabled in Solana app settings"
echo ""
echo "Press ENTER to continue or Ctrl+C to cancel..."
read

# Run the Ledger test
if ./target/release/solana-arbitrage-bot --test-ledger; then
    echo -e "\n${GREEN}🎉 All tests passed!${NC}"
    echo -e "${GREEN}Your Ledger is ready for use with the bot.${NC}"
else
    echo -e "\n${YELLOW}⚠️ Ledger test completed with warnings.${NC}"
    echo "This might be normal if you rejected the test signature."
fi

echo -e "\n${YELLOW}Next steps:${NC}"
echo "1. Update config.yaml to set use_ledger: true"
echo "2. Test with dry-run mode: ./target/release/solana-arbitrage-bot --dry-run"
echo "3. Start with small position sizes for initial testing"

echo -e "\n${GREEN}Test script completed!${NC}"