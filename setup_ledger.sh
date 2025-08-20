#!/bin/bash
# 🔐 Ledger Hardware Wallet Setup for Solana Arbitrage Bot
# Based on official Ledger documentation: https://developers.ledger.com/docs/device-app/getting-started

set -e  # Exit on any error

echo "🔐 Setting up Ledger Hardware Wallet Support..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   print_error "Don't run this script as root! Run as regular user."
   exit 1
fi

print_step "1. Checking system requirements"

# Check Ubuntu version
if ! command -v lsb_release &> /dev/null; then
    print_warning "lsb_release not found, assuming Ubuntu-compatible system"
else
    UBUNTU_VERSION=$(lsb_release -rs)
    print_success "Detected Ubuntu $UBUNTU_VERSION"
fi

# Check if dependencies are installed
print_step "2. Checking dependencies"

MISSING_DEPS=()

if ! dpkg -l | grep -q "libudev-dev"; then
    MISSING_DEPS+=("libudev-dev")
fi

if ! dpkg -l | grep -q "libusb-1.0-0"; then
    MISSING_DEPS+=("libusb-1.0-0-dev")
fi

if ! dpkg -l | grep -q "pkg-config"; then
    MISSING_DEPS+=("pkg-config")
fi

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    print_warning "Missing dependencies: ${MISSING_DEPS[*]}"
    echo "Please install them with:"
    echo "sudo apt update && sudo apt install -y ${MISSING_DEPS[*]}"
    echo ""
    read -p "Do you want to continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    print_success "All dependencies are installed"
fi

print_step "3. Setting up udev rules for Ledger"

# Create udev rules for Ledger devices
UDEV_RULES_FILE="/etc/udev/rules.d/20-hw1.rules"

if [ -f "$UDEV_RULES_FILE" ]; then
    print_success "Ledger udev rules already exist"
else
    print_warning "Ledger udev rules not found. Creating them..."
    
    # Create temporary rules file
    cat > /tmp/20-hw1.rules << 'EOF'
# Ledger Hardware Wallets
# Nano S
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0001", MODE="0666", TAG+="uaccess", TAG+="udev-acl"
# Nano X
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0004", MODE="0666", TAG+="uaccess", TAG+="udev-acl"
# Nano S Plus
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0005", MODE="0666", TAG+="uaccess", TAG+="udev-acl"
# Ledger Blue
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0000", MODE="0666", TAG+="uaccess", TAG+="udev-acl"
# All Ledger devices (fallback)
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", MODE="0666", TAG+="uaccess", TAG+="udev-acl"
EOF

    echo "Please run the following command to install udev rules:"
    echo "sudo cp /tmp/20-hw1.rules $UDEV_RULES_FILE"
    echo "sudo udevadm control --reload-rules"
    echo "sudo udevadm trigger"
    echo ""
    
    read -p "Do you want to continue without installing udev rules? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_error "Please install udev rules and run this script again"
        exit 1
    fi
fi

print_step "4. Checking user permissions"

# Check if user is in plugdev group
if groups $USER | grep -q "plugdev"; then
    print_success "User $USER is in plugdev group"
else
    print_warning "User $USER is not in plugdev group"
    echo "Please add yourself to plugdev group:"
    echo "sudo usermod -a -G plugdev $USER"
    echo "Then log out and log back in"
    echo ""
fi

print_step "5. Building Ledger support"

# Build with Ledger support
echo "Building Solana Arbitrage Bot with Ledger support..."
if cargo build --release; then
    print_success "Build completed successfully"
else
    print_error "Build failed. Check the error messages above."
    exit 1
fi

print_step "6. Testing Ledger detection"

# Check if any Ledger devices are connected
echo "Checking for connected Ledger devices..."
LEDGER_DEVICES=$(lsusb | grep -i "2c97" || true)

if [ -n "$LEDGER_DEVICES" ]; then
    print_success "Ledger device(s) detected:"
    echo "$LEDGER_DEVICES"
else
    print_warning "No Ledger devices detected"
    echo "Please connect your Ledger device and unlock it"
fi

print_step "7. Testing Solana CLI with Ledger"

# Test Solana CLI Ledger integration
echo "Testing Solana CLI Ledger integration..."
if command -v solana &> /dev/null; then
    echo "Attempting to get Ledger public key..."
    if timeout 10s solana-keygen pubkey usb://ledger?key=0/0 2>/dev/null; then
        print_success "Solana CLI can communicate with Ledger"
    else
        print_warning "Solana CLI Ledger test failed or timed out"
        echo "This is normal if:"
        echo "- Ledger is not connected"
        echo "- Solana app is not open on Ledger"
        echo "- User didn't approve the request"
    fi
else
    print_warning "Solana CLI not found. Install it for better Ledger support."
fi

print_step "8. Creating Ledger test script"

# Create test script
cat > test_ledger.sh << 'EOF'
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
EOF

chmod +x test_ledger.sh
print_success "Created test_ledger.sh script"

print_step "9. Setup complete!"

echo ""
print_success "🎉 Ledger setup completed!"
echo ""
echo -e "${YELLOW}NEXT STEPS:${NC}"
echo "1. Connect your Ledger device"
echo "2. Unlock it and open the Solana app"
echo "3. Run: ./test_ledger.sh"
echo "4. Configure bot to use Ledger in config.yaml:"
echo "   wallet:"
echo "     use_ledger: true"
echo "     ledger_path: \"m/44'/501'/0'/0'\""
echo ""
echo -e "${RED}IMPORTANT SECURITY NOTES:${NC}"
echo "- Always verify transactions on Ledger screen"
echo "- Start with small amounts for testing"
echo "- Keep your Ledger firmware updated"
echo "- Never share your recovery phrase"
echo ""
echo -e "${GREEN}Happy secure trading! 🚀${NC}"

# Final system info
echo ""
echo "=== System Information ==="
echo "OS: $(uname -a)"
echo "User: $USER"
echo "Groups: $(groups $USER)"
echo "USB devices: $(lsusb | grep -c "Bus" || echo "0") total"
echo "Ledger devices: $(lsusb | grep -c "2c97" || echo "0") detected"
