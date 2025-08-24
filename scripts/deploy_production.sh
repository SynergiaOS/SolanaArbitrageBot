#!/bin/bash
# 🚀 Production Deployment Script - Solana Arbitrage Bot
# Automated setup for production environment

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PRODUCTION_CONFIG="config.production.enhanced.yaml"
LOG_DIR="$PROJECT_ROOT/logs"
DATA_DIR="$PROJECT_ROOT/data"
BACKUP_DIR="$PROJECT_ROOT/backups"

echo -e "${BLUE}🚀 Solana Arbitrage Bot - Production Deployment${NC}"
echo "=================================================="

# Function to print status
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   print_error "This script should not be run as root for security reasons"
   exit 1
fi

# Phase 1: Environment Setup
echo -e "\n${BLUE}📋 Phase 1: Environment Setup${NC}"

# Check system requirements
print_status "Checking system requirements..."

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install Rust first."
    exit 1
fi

# Check Node.js (for dashboard)
if ! command -v node &> /dev/null; then
    print_warning "Node.js not found. Dashboard features will be limited."
fi

# Check available memory
MEMORY_GB=$(free -g | awk '/^Mem:/{print $2}')
if [ "$MEMORY_GB" -lt 4 ]; then
    print_warning "Less than 4GB RAM available. Consider upgrading for optimal performance."
fi

# Check disk space
DISK_SPACE_GB=$(df -BG "$PROJECT_ROOT" | awk 'NR==2{print $4}' | sed 's/G//')
if [ "$DISK_SPACE_GB" -lt 10 ]; then
    print_warning "Less than 10GB disk space available."
fi

print_status "System requirements check completed"

# Phase 2: Directory Structure
echo -e "\n${BLUE}📁 Phase 2: Directory Structure${NC}"

# Create necessary directories
mkdir -p "$LOG_DIR" "$DATA_DIR" "$BACKUP_DIR"
mkdir -p "$PROJECT_ROOT/scripts/monitoring"
mkdir -p "$PROJECT_ROOT/config/production"

print_status "Directory structure created"

# Phase 3: Build Production Binary
echo -e "\n${BLUE}🔨 Phase 3: Building Production Binary${NC}"

cd "$PROJECT_ROOT"

# Clean previous builds
cargo clean

# Build optimized release binary
print_status "Building optimized release binary..."
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Verify binary exists
if [ ! -f "$PROJECT_ROOT/target/release/solana-arbitrage-bot" ]; then
    print_error "Failed to build production binary"
    exit 1
fi

print_status "Production binary built successfully"

# Phase 4: Configuration Setup
echo -e "\n${BLUE}⚙️  Phase 4: Configuration Setup${NC}"

# Copy production config if it doesn't exist
if [ ! -f "$PROJECT_ROOT/$PRODUCTION_CONFIG" ]; then
    print_error "Production config file not found: $PRODUCTION_CONFIG"
    exit 1
fi

# Create environment file template
cat > "$PROJECT_ROOT/.env.production" << 'EOF'
# 🚀 Production Environment Variables
# Copy this file and fill in your actual values

# RPC Configuration (REQUIRED)
BOT__RPC__URL="https://solana-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
BOT__RPC__WS_URL="wss://solana-mainnet.g.alchemy.com/v2/YOUR_API_KEY"

# Discord Alerts (OPTIONAL)
BOT__DISCORD__WEBHOOK_URL="https://discord.com/api/webhooks/YOUR_WEBHOOK"

# Web Dashboard (OPTIONAL)
BOT__WEB__AUTH_TOKEN="your-secure-random-token-here"

# Database (OPTIONAL - defaults to SQLite)
BOT__DATABASE__URL="sqlite:./data/production.db"

# Wallet Configuration (SECURITY CRITICAL)
BOT__WALLET__USE_LEDGER=true
BOT__WALLET__LEDGER_PATH="44'/501'/0'/0'"

# Trading Limits (RISK MANAGEMENT)
BOT__LIMITS__MAX_DAILY_LOSS_USD=10.0
BOT__LIMITS__MAX_POSITION_SOL=2.0
EOF

print_status "Environment template created: .env.production"

# Phase 5: Security Setup
echo -e "\n${BLUE}🔐 Phase 5: Security Setup${NC}"

# Set proper file permissions
chmod 600 "$PROJECT_ROOT/.env.production"
chmod 700 "$PROJECT_ROOT/scripts"
chmod 755 "$PROJECT_ROOT/target/release/solana-arbitrage-bot"

# Create wallet directory with restricted permissions
mkdir -p "$PROJECT_ROOT/wallets"
chmod 700 "$PROJECT_ROOT/wallets"

print_status "Security permissions configured"

# Phase 6: Monitoring Setup
echo -e "\n${BLUE}📊 Phase 6: Monitoring Setup${NC}"

# Create systemd service file
cat > "$PROJECT_ROOT/scripts/solana-arbitrage-bot.service" << EOF
[Unit]
Description=Solana Arbitrage Bot
After=network.target
Wants=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$PROJECT_ROOT
Environment=RUST_LOG=info
Environment=CONFIG_FILE=$PRODUCTION_CONFIG
ExecStart=$PROJECT_ROOT/target/release/solana-arbitrage-bot
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$PROJECT_ROOT

[Install]
WantedBy=multi-user.target
EOF

print_status "Systemd service file created"

# Create monitoring script
cat > "$PROJECT_ROOT/scripts/monitor.sh" << 'EOF'
#!/bin/bash
# Production monitoring script

LOG_FILE="./logs/production.log"
ALERT_WEBHOOK="$BOT__DISCORD__WEBHOOK_URL"

# Check if bot is running
if ! pgrep -f "solana-arbitrage-bot" > /dev/null; then
    echo "❌ Bot is not running!"
    if [ -n "$ALERT_WEBHOOK" ]; then
        curl -X POST "$ALERT_WEBHOOK" \
             -H "Content-Type: application/json" \
             -d '{"content": "🚨 **ALERT**: Solana Arbitrage Bot is DOWN!"}'
    fi
    exit 1
fi

# Check recent errors
if [ -f "$LOG_FILE" ]; then
    ERROR_COUNT=$(tail -n 100 "$LOG_FILE" | grep -c "ERROR" || true)
    if [ "$ERROR_COUNT" -gt 5 ]; then
        echo "⚠️ High error count: $ERROR_COUNT errors in last 100 lines"
    fi
fi

echo "✅ Bot is running normally"
EOF

chmod +x "$PROJECT_ROOT/scripts/monitor.sh"

print_status "Monitoring scripts created"

# Phase 7: Testing Setup
echo -e "\n${BLUE}🧪 Phase 7: Testing Setup${NC}"

# Create paper trading config
cp "$PROJECT_ROOT/$PRODUCTION_CONFIG" "$PROJECT_ROOT/config.paper.yaml"
sed -i 's/dry_run: false/dry_run: true/' "$PROJECT_ROOT/config.paper.yaml"
sed -i 's/max_position_sol: 2.0/max_position_sol: 0.1/' "$PROJECT_ROOT/config.paper.yaml"

print_status "Paper trading configuration created"

# Phase 8: Backup Setup
echo -e "\n${BLUE}💾 Phase 8: Backup Setup${NC}"

# Create backup script
cat > "$PROJECT_ROOT/scripts/backup.sh" << 'EOF'
#!/bin/bash
# Automated backup script

BACKUP_DIR="./backups"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/backup_$DATE.tar.gz"

# Create backup
tar -czf "$BACKUP_FILE" \
    --exclude='./target' \
    --exclude='./backups' \
    --exclude='./.git' \
    ./

# Keep only last 30 backups
find "$BACKUP_DIR" -name "backup_*.tar.gz" -type f -mtime +30 -delete

echo "✅ Backup created: $BACKUP_FILE"
EOF

chmod +x "$PROJECT_ROOT/scripts/backup.sh"

print_status "Backup system configured"

# Phase 9: Final Checks
echo -e "\n${BLUE}🔍 Phase 9: Final Checks${NC}"

# Verify binary works
if ! "$PROJECT_ROOT/target/release/solana-arbitrage-bot" --version &> /dev/null; then
    print_error "Binary verification failed"
    exit 1
fi

print_status "Binary verification passed"

# Summary
echo -e "\n${GREEN}🎉 Production Deployment Setup Complete!${NC}"
echo "=============================================="
echo ""
echo "📋 Next Steps:"
echo "1. Edit .env.production with your actual values"
echo "2. Set up your Ledger hardware wallet"
echo "3. Fund your wallet with initial capital ($50-100)"
echo "4. Run paper trading first: ./scripts/start_paper.sh"
echo "5. Monitor performance for 24-48 hours"
echo "6. Start production: ./scripts/start_production.sh"
echo ""
echo "📊 Monitoring:"
echo "- Logs: tail -f $LOG_DIR/production.log"
echo "- Monitor: ./scripts/monitor.sh"
echo "- Dashboard: http://localhost:8080 (if enabled)"
echo ""
echo "🔐 Security Reminders:"
echo "- Never commit real wallet keys to git"
echo "- Use hardware wallet for production"
echo "- Keep hot wallet funds minimal"
echo "- Monitor alerts regularly"
echo ""
echo "📞 Support:"
echo "- Check logs first: $LOG_DIR/"
echo "- Run diagnostics: ./scripts/diagnostics.sh"
echo "- Review configuration: $PRODUCTION_CONFIG"

print_status "Deployment script completed successfully!"
EOF
