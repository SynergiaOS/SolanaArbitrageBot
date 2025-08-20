#!/bin/bash
# 🚀 Solana Arbitrage Bot - Production Setup Script
# Automatyczne wdrożenie na Ubuntu 22.04/24.04

set -e  # Exit on any error

echo "🚀 Starting Solana Arbitrage Bot Production Setup..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BOT_USER="bot"
BOT_HOME="/home/$BOT_USER"
BOT_DIR="$BOT_HOME/SolanaArbitrageBot"
BACKUP_DIR="/secure/backups"

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
   print_error "Don't run this script as root! Run as regular user with sudo access."
   exit 1
fi

print_step "1. System Update and Dependencies"
sudo apt update && sudo apt upgrade -y
sudo apt install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libudev-dev \
    libusb-1.0-0-dev \
    libssl-dev \
    htop \
    iotop \
    nethogs \
    ufw \
    fail2ban \
    logrotate \
    bc

print_step "2. Install Rust"
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
    rustup update
    print_success "Rust installed"
else
    print_success "Rust already installed"
fi

print_step "3. Install Solana CLI"
if ! command -v solana &> /dev/null; then
    sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
    echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
    print_success "Solana CLI installed"
else
    print_success "Solana CLI already installed"
fi

print_step "4. Create Bot User"
if ! id "$BOT_USER" &>/dev/null; then
    sudo useradd -m -s /bin/bash "$BOT_USER"
    sudo usermod -aG sudo "$BOT_USER"
    print_success "Bot user created: $BOT_USER"
else
    print_success "Bot user already exists: $BOT_USER"
fi

print_step "5. Setup Directories"
sudo mkdir -p "$BACKUP_DIR"
sudo chown "$BOT_USER:$BOT_USER" "$BACKUP_DIR"
sudo chmod 700 "$BACKUP_DIR"

print_step "6. Clone Repository"
if [ ! -d "$BOT_DIR" ]; then
    sudo -u "$BOT_USER" git clone https://github.com/SynergiaOS/SolanaArbitrageBot.git "$BOT_DIR"
    print_success "Repository cloned"
else
    print_success "Repository already exists"
fi

print_step "7. Build Bot"
cd "$BOT_DIR"
sudo -u "$BOT_USER" cargo build --release
print_success "Bot compiled successfully"

print_step "8. Setup Ledger Support"
# Udev rules for Ledger
echo 'SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", MODE="0666"' | sudo tee /etc/udev/rules.d/20-ledger.rules
sudo udevadm control --reload-rules
print_success "Ledger udev rules configured"

print_step "9. Configure Firewall"
sudo ufw --force reset
sudo ufw default deny incoming
sudo ufw default allow outgoing

# SSH (change port if needed)
read -p "Enter SSH port (default 22): " SSH_PORT
SSH_PORT=${SSH_PORT:-22}
sudo ufw allow "$SSH_PORT"/tcp

# HTTPS for RPC
sudo ufw allow out 443/tcp
sudo ufw allow out 80/tcp

sudo ufw --force enable
print_success "Firewall configured"

print_step "10. Setup Systemd Service"
sudo tee /etc/systemd/system/arbitrage-bot.service > /dev/null <<EOF
[Unit]
Description=Solana Arbitrage Bot
After=network.target

[Service]
Type=simple
User=$BOT_USER
Group=$BOT_USER
WorkingDirectory=$BOT_DIR
ExecStart=$BOT_DIR/target/release/solana-arbitrage-bot
Restart=always
RestartSec=10
Environment=RUST_LOG=info

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$BOT_DIR

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable arbitrage-bot
print_success "Systemd service configured"

print_step "11. Setup Log Rotation"
sudo tee /etc/logrotate.d/arbitrage-bot > /dev/null <<EOF
$BOT_DIR/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 $BOT_USER $BOT_USER
}
EOF
print_success "Log rotation configured"

print_step "12. Create Backup Script"
sudo tee "$BOT_DIR/backup_bot.sh" > /dev/null <<'EOF'
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/secure/backups"
BOT_DIR="/home/bot/SolanaArbitrageBot"

# Backup wallet (encrypted)
if [ -f "$BOT_DIR/wallet.json" ]; then
    gpg --batch --yes --symmetric --cipher-algo AES256 \
        --passphrase-file "$BOT_DIR/.backup_passphrase" \
        --output "$BACKUP_DIR/wallet_$DATE.gpg" \
        "$BOT_DIR/wallet.json"
fi

# Backup config
cp "$BOT_DIR/config.yaml" "$BACKUP_DIR/config_$DATE.yaml"

# Backup trade database
if [ -f "$BOT_DIR/trades.db" ]; then
    cp "$BOT_DIR/trades.db" "$BACKUP_DIR/trades_$DATE.db"
fi

# Clean old backups (keep 30 days)
find "$BACKUP_DIR" -name "*.gpg" -mtime +30 -delete
find "$BACKUP_DIR" -name "*.yaml" -mtime +30 -delete
find "$BACKUP_DIR" -name "*.db" -mtime +30 -delete

echo "Backup completed: $DATE"
EOF

sudo chmod +x "$BOT_DIR/backup_bot.sh"
sudo chown "$BOT_USER:$BOT_USER" "$BOT_DIR/backup_bot.sh"

# Add to crontab
sudo -u "$BOT_USER" crontab -l 2>/dev/null | { cat; echo "0 2 * * * $BOT_DIR/backup_bot.sh"; } | sudo -u "$BOT_USER" crontab -
print_success "Backup script configured"

print_step "13. Create Health Check Script"
sudo tee "$BOT_DIR/health_check.sh" > /dev/null <<'EOF'
#!/bin/bash
BOT_PID=$(pgrep -f "solana-arbitrage-bot")

if [ -z "$BOT_PID" ]; then
    echo "🚨 Bot not running! Restarting..."
    systemctl restart arbitrage-bot
    
    # Log restart
    echo "$(date): Bot restarted" >> /var/log/arbitrage-bot-restarts.log
fi

# Check if config exists
if [ ! -f "/home/bot/SolanaArbitrageBot/config.yaml" ]; then
    echo "🚨 Config file missing!"
fi

# Check if wallet exists
if [ ! -f "/home/bot/SolanaArbitrageBot/wallet.json" ]; then
    echo "🚨 Wallet file missing!"
fi
EOF

sudo chmod +x "$BOT_DIR/health_check.sh"
sudo chown "$BOT_USER:$BOT_USER" "$BOT_DIR/health_check.sh"

# Add to crontab (every 5 minutes)
sudo -u "$BOT_USER" crontab -l 2>/dev/null | { cat; echo "*/5 * * * * $BOT_DIR/health_check.sh"; } | sudo -u "$BOT_USER" crontab -
print_success "Health check configured"

print_step "14. Setup Complete!"
echo ""
print_success "🎉 Production setup completed successfully!"
echo ""
echo -e "${YELLOW}NEXT STEPS:${NC}"
echo "1. Configure your RPC endpoint in config.yaml"
echo "2. Setup your wallet:"
echo "   - For testing: solana-keygen new --outfile $BOT_DIR/wallet.json"
echo "   - For production: Use Ledger hardware wallet"
echo "3. Fund your wallet with SOL"
echo "4. Test the bot: sudo -u $BOT_USER $BOT_DIR/target/release/solana-arbitrage-bot --dry-run"
echo "5. Start the service: sudo systemctl start arbitrage-bot"
echo "6. Monitor logs: journalctl -u arbitrage-bot -f"
echo ""
echo -e "${RED}SECURITY REMINDERS:${NC}"
echo "- Change SSH port and disable password auth"
echo "- Setup Telegram alerts"
echo "- Use Ledger for production"
echo "- Start with small positions"
echo "- Monitor 24/7 for first week"
echo ""
echo -e "${GREEN}Happy Trading! 🚀${NC}"
