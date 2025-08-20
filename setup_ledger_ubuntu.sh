_xccll       /bash

# Ledger Nano S+ Setup dla Ubuntu 24.04
# Kolory dla lepszej czytelności
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔐 Solana Ledger Setup for Ubuntu 24.04${NC}"
echo "=================================================="

# 1. Instalacja udev rules dla Ledger
echo -e "\n${YELLOW}Step 1: Installing Ledger udev rules...${NC}"
wget -q -O - https://raw.githubusercontent.com/LedgerHQ/udev-rules/master/add_udev_rules.sh | sudo bash

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Udev rules installed${NC}"
else
    echo -e "${RED}✗ Failed to install udev rules${NC}"
    exit 1
fi

# 2. Dodaj użytkownika do grupy plugdev
echo -e "\n${YELLOW}Step 2: Adding user to plugdev group...${NC}"
sudo usermod -aG plugdev $USER

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ User added to plugdev group${NC}"
else
    echo -e "${RED}✗ Failed to add user to plugdev group${NC}"
    exit 1
fi

# 3. Restart udev
echo -e "\n${YELLOW}Step 3: Restarting udev...${NC}"
sudo udevadm control --reload-rules
sudo udevadm trigger

echo -e "${GREEN}✓ Udev restarted${NC}"

# 4. Instalacja Solana CLI z obsługą Ledger
echo -e "\n${YELLOW}Step 4: Installing Solana CLI...${NC}"
if ! command -v solana &> /dev/null; then
    sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
    echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
    echo -e "${GREEN}✓ Solana CLI installed${NC}"
else
    echo -e "${GREEN}✓ Solana CLI already installed${NC}"
fi

# 5. Instalacja narzędzi dla Ledger
echo -e "\n${YELLOW}Step 5: Installing USB development libraries...${NC}"
sudo apt-get update
sudo apt-get install -y libusb-1.0-0-dev libudev-dev

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ USB libraries installed${NC}"
else
    echo -e "${RED}✗ Failed to install USB libraries${NC}"
    exit 1
fi

# 6. Instalacja Rust jeśli nie ma
echo -e "\n${YELLOW}Step 6: Checking Rust installation...${NC}"
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    echo -e "${GREEN}✓ Rust installed${NC}"
else
    echo -e "${GREEN}✓ Rust already installed${NC}"
fi

# 7. Test połączenia (opcjonalny)
echo -e "\n${YELLOW}Step 7: Testing Ledger connection...${NC}"
echo -e "${YELLOW}UWAGA: Podłącz Ledger i otwórz aplikację Solana!${NC}"
echo "Naciśnij ENTER gdy Ledger będzie gotowy do testu..."
read

# Test czy Ledger jest wykryty przez USB
if lsusb | grep -q "Ledger"; then
    echo -e "${GREEN}✓ Ledger wykryty przez USB${NC}"
    
    # Test połączenia przez Solana CLI
    echo "Testowanie połączenia z aplikacją Solana..."
    LEDGER_ADDRESS=$(timeout 30 solana-keygen pubkey usb://ledger?key=0/0 2>/dev/null)
    
    if [ $? -eq 0 ] && [ ! -z "$LEDGER_ADDRESS" ]; then
        echo -e "${GREEN}✓ Ledger connection successful!${NC}"
        echo -e "${GREEN}Address: $LEDGER_ADDRESS${NC}"
    else
        echo -e "${YELLOW}⚠️ Could not connect to Solana app on Ledger${NC}"
        echo "Make sure:"
        echo "1. Ledger is unlocked"
        echo "2. Solana app is open"
        echo "3. Blind signing is enabled (Settings in Solana app)"
    fi
else
    echo -e "${YELLOW}⚠️ Ledger not detected via USB${NC}"
    echo "Make sure Ledger is connected and unlocked"
fi

# 8. Podsumowanie
echo -e "\n${GREEN}=================================================${NC}"
echo -e "${GREEN}🎉 SETUP COMPLETE!${NC}"
echo -e "${GREEN}=================================================${NC}"

echo -e "\n${YELLOW}Important next steps:${NC}"
echo "1. ${RED}LOGOUT AND LOGIN AGAIN${NC} (required for group changes)"
echo "2. Connect your Ledger Nano S+"
echo "3. Install and open the Solana app on Ledger"
echo "4. Enable blind signing in Solana app settings"
echo "5. Update config.yaml: set use_ledger: true"

echo -e "\n${YELLOW}To test the bot with Ledger:${NC}"
echo -e "${GREEN}cargo build --release${NC}"
echo -e "${GREEN}./target/release/solana-arbitrage-bot --dry-run${NC}"

echo -e "\n${YELLOW}Security reminders:${NC}"
echo "• Use a SEPARATE Ledger for the bot (not your main wallet)"
echo "• Start with small amounts (1-10 SOL max)"
echo "• Always verify transactions on Ledger screen"
echo "• Keep your seed phrase secure and backed up"

echo -e "\n${RED}⚠️ IMPORTANT: You must logout and login again for USB permissions to take effect!${NC}"