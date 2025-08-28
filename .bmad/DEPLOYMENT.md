# Solana Arbitrage Bot - Production Deployment Guide

## Overview

This document provides comprehensive deployment instructions for the Solana Arbitrage Bot in production environments. The deployment strategy is designed for security, reliability, and performance optimization while following the documented architecture patterns.

## Prerequisites

### Hardware Requirements
- **CPU**: 4+ cores, 2.5GHz+ (Intel/AMD)
- **RAM**: 8GB minimum, 16GB recommended
- **Storage**: 50GB+ SSD with high IOPS
- **Network**: Stable broadband with low latency to Solana RPC endpoints

### Software Requirements
- **OS**: Linux (Ubuntu 20.04+ LTS recommended)
- **Rust**: Latest stable version (1.70+)
- **PostgreSQL**: v13+ (for production database)
- **Docker**: Latest version (optional)
- **systemd**: For service management

## Environment Variable Setup

### Core Environment Variables (.env)

Create a `.env` file in the project root with the following variables:

```bash
# === Wallet Security ===
WALLET_ENCRYPTION_KEY=your_256_bit_encryption_key_here
WALLET_PATH=/opt/solana-bot/wallets/production.json
WALLET_BACKUP_PATH=/secure/backup/wallet.json

# === RPC Configuration ===
# Primary RPC endpoint (Helius recommended for production)
PRIMARY_RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_HELIUS_API_KEY
PRIMARY_WS_URL=wss://mainnet.helius-rpc.com/?api-key=YOUR_HELIUS_API_KEY

# Backup RPC endpoints
BACKUP_RPC_URL=https://solana-mainnet.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY
TERTIARY_RPC_URL=https://api.mainnet-beta.solana.com

# === Web Dashboard Security ===
ADMIN_TOKEN=admin_super_secure_token_256_chars
CONFIG_TOKEN=config_secure_token_256_chars
STATUS_TOKEN=status_readonly_token_256_chars

# === Database Configuration ===
DATABASE_URL=postgresql://bot_user:secure_password@localhost:5432/solana_bot
DATABASE_MAX_CONNECTIONS=20

# === Alert Systems ===
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
TELEGRAM_CHAT_ID=your_telegram_chat_id
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/your_webhook_url

# === Jito MEV Protection ===
JITO_API_KEY=your_jito_api_key
JITO_PRIVATE_KEY=your_jito_private_key

# === Performance Tuning ===
RUST_LOG=info
TOKIO_WORKER_THREADS=4
CONNECTION_POOL_SIZE=10

# === Security ===
ALLOWED_CONTROL_IPS=127.0.0.1/32,192.168.1.0/24
RATE_LIMIT_GLOBAL=100
RATE_LIMIT_CONTROL=10
```

### Environment Variable Descriptions

| Variable | Purpose | Security Level | Example |
|----------|---------|---------------|---------|
| `WALLET_ENCRYPTION_KEY` | Encrypts wallet private keys | **CRITICAL** | 64-char hex string |
| `PRIMARY_RPC_URL` | Main Solana RPC endpoint | Medium | Helius/QuickNode URL |
| `ADMIN_TOKEN` | Full system control access | **CRITICAL** | 256-bit random token |
| `DATABASE_URL` | PostgreSQL connection string | High | Standard PostgreSQL URL |
| `TELEGRAM_BOT_TOKEN` | Alert notifications | Medium | Bot token from @BotFather |

## Security Best Practices for Private Keys

### Option 1: Hardware Wallet Integration (Recommended)

**Ledger Hardware Wallet Setup:**

1. **Install Ledger dependencies:**
```bash
sudo apt-get install libudev-dev libusb-1.0-0-dev
cargo install ledger-tool
```

2. **Configure Ledger in config.yaml:**
```yaml
wallet:
  use_ledger: true
  ledger_wallet_index: 0
  derivation_path: "44'/501'/0'/0'"
  confirmation_required: true
```

3. **Security advantages:**
- Private keys never leave the hardware device
- Physical confirmation required for transactions
- Immune to malware and system compromises
- Support for multiple wallet accounts

### Option 2: HashiCorp Vault Integration

**Setup Vault for key management:**

1. **Install and configure Vault:**
```bash
# Install Vault
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo apt-key add -
sudo apt-add-repository "deb [arch=amd64] https://apt.releases.hashicorp.com $(lsb_release -cs) main"
sudo apt-get update && sudo apt-get install vault

# Initialize Vault
vault server -dev &
export VAULT_ADDR='http://127.0.0.1:8200'
vault auth
```

2. **Store wallet keys in Vault:**
```bash
# Store wallet private key
vault kv put secret/solana-bot wallet_key=@/path/to/wallet.json

# Store encryption keys
vault kv put secret/solana-bot/encryption key=$(openssl rand -hex 32)
```

3. **Configure bot for Vault:**
```yaml
vault:
  enabled: true
  address: "http://127.0.0.1:8200"
  token: "${VAULT_TOKEN}"
  wallet_path: "secret/solana-bot/wallet_key"
```

### Option 3: AWS Secrets Manager

**Setup AWS Secrets Manager:**

1. **Install AWS CLI:**
```bash
sudo apt-get install awscli
aws configure
```

2. **Create secrets:**
```bash
# Store wallet private key
aws secretsmanager create-secret \
  --name "solana-bot/wallet-private-key" \
  --description "Solana wallet private key" \
  --secret-string file://wallet.json

# Store encryption key
aws secretsmanager create-secret \
  --name "solana-bot/encryption-key" \
  --description "Wallet encryption key" \
  --secret-string "$(openssl rand -hex 32)"
```

3. **Configure IAM permissions:**
```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "secretsmanager:GetSecretValue"
            ],
            "Resource": "arn:aws:secretsmanager:region:account:secret:solana-bot/*"
        }
    ]
}
```

### Option 4: File-Based Key Management (Basic)

**Secure file-based approach:**

1. **Create secure directory structure:**
```bash
# Create secure wallet directory
sudo mkdir -p /opt/solana-bot/wallets
sudo chown $USER:$USER /opt/solana-bot/wallets
chmod 700 /opt/solana-bot/wallets

# Generate wallet with proper permissions
solana-keygen new --outfile /opt/solana-bot/wallets/production.json
chmod 600 /opt/solana-bot/wallets/production.json
```

2. **Encrypt wallet file:**
```bash
# Encrypt the wallet file
openssl enc -aes-256-cbc -salt -in production.json -out production.json.enc -k "your_encryption_key"
rm production.json
```

3. **Configure decryption in bot:**
```yaml
wallet:
  path: "/opt/solana-bot/wallets/production.json.enc"
  encryption_key: "${WALLET_ENCRYPTION_KEY}"
  encrypted: true
```

### Key Rotation Strategy

**Automated key rotation process:**

1. **Create rotation script:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/rotate_keys.sh

# Generate new wallet
NEW_WALLET="/tmp/new_wallet_$(date +%s).json"
solana-keygen new --outfile "$NEW_WALLET"

# Transfer funds from old to new wallet
OLD_BALANCE=$(solana balance --url mainnet-beta)
solana transfer --from /opt/solana-bot/wallets/production.json \
  $(solana-keygen pubkey "$NEW_WALLET") "$OLD_BALANCE" --url mainnet-beta

# Backup old wallet
cp /opt/solana-bot/wallets/production.json "/secure/backup/wallet_$(date +%s).json.bak"

# Deploy new wallet
cp "$NEW_WALLET" /opt/solana-bot/wallets/production.json
chmod 600 /opt/solana-bot/wallets/production.json

# Restart bot service
systemctl restart solana-arbitrage-bot
```

2. **Schedule rotation (monthly):**
```bash
# Add to crontab
0 2 1 * * /opt/solana-bot/scripts/rotate_keys.sh
```

## Building and Running in Monitor-Only Mode

### Monitor-Only Build Configuration

The monitor-only build is the recommended production configuration, optimized for minimal resource usage and maximum stability.

**Build monitor-only binary:**
```bash
# Navigate to project directory
cd /path/to/solana-arbitrage-bot

# Clean previous builds
cargo clean

# Build optimized monitor-only release
RUSTFLAGS="-C target-cpu=native" cargo build --release --no-default-features --features monitor

# Verify build
./target/release/solana-arbitrage-bot --version
```

**Validate monitor-only dependencies:**
```bash
# Check feature gates (should show minimal dependencies)
cargo tree --no-default-features --features monitor -e features

# Run tests for monitor-only build
cargo test --no-default-features --features monitor --workspace --verbose
```

### systemd Service Configuration

**Create systemd service file:**
```bash
sudo tee /etc/systemd/system/solana-arbitrage-bot.service > /dev/null <<EOF
[Unit]
Description=Solana Arbitrage Bot - Monitor Only Mode
After=network.target
Wants=network.target
StartLimitIntervalSec=60
StartLimitBurst=3

[Service]
Type=simple
User=arbitrage
Group=arbitrage
WorkingDirectory=/opt/solana-bot
ExecStart=/opt/solana-bot/target/release/solana-arbitrage-bot --config /opt/solana-bot/config/production.yaml --monitor-only
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=solana-bot

# Resource Limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryMax=512M
CPUQuota=200%

# Security Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/solana-bot/data /opt/solana-bot/logs
PrivateTmp=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
EnvironmentFile=/opt/solana-bot/.env

[Install]
WantedBy=multi-user.target
EOF
```

**Create dedicated user:**
```bash
# Create system user for bot
sudo useradd -r -s /bin/false -d /opt/solana-bot arbitrage

# Set proper ownership
sudo chown -R arbitrage:arbitrage /opt/solana-bot
sudo chmod 755 /opt/solana-bot
sudo chmod 600 /opt/solana-bot/.env
```

**Enable and start service:**
```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable solana-arbitrage-bot

# Start the service
sudo systemctl start solana-arbitrage-bot

# Check service status
sudo systemctl status solana-arbitrage-bot
```

### Process Management and Monitoring

**Service management commands:**
```bash
# Check service status
sudo systemctl status solana-arbitrage-bot

# View logs
sudo journalctl -u solana-arbitrage-bot -f

# Restart service
sudo systemctl restart solana-arbitrage-bot

# Stop service
sudo systemctl stop solana-arbitrage-bot

# Check service configuration
systemctl show solana-arbitrage-bot
```

**Health check script:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/health_check.sh

SERVICE_NAME="solana-arbitrage-bot"
HEALTH_ENDPOINT="http://localhost:3001/health"

# Check if service is running
if ! systemctl is-active --quiet "$SERVICE_NAME"; then
    echo "CRITICAL: Service $SERVICE_NAME is not running"
    exit 2
fi

# Check health endpoint (if web features enabled)
if command -v curl >/dev/null 2>&1; then
    if ! curl -f -s "$HEALTH_ENDPOINT" >/dev/null; then
        echo "WARNING: Health endpoint not responding"
        exit 1
    fi
fi

# Check log for errors in last 5 minutes
ERROR_COUNT=$(journalctl -u "$SERVICE_NAME" --since "5 minutes ago" | grep -c "ERROR\|CRITICAL" || true)
if [ "$ERROR_COUNT" -gt 5 ]; then
    echo "WARNING: $ERROR_COUNT errors found in last 5 minutes"
    exit 1
fi

echo "OK: Service is healthy"
exit 0
```

### Log Management and Rotation

**Configure logrotate:**
```bash
sudo tee /etc/logrotate.d/solana-arbitrage-bot > /dev/null <<EOF
/opt/solana-bot/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 arbitrage arbitrage
    postrotate
        systemctl reload solana-arbitrage-bot
    endscript
}
EOF
```

**Log monitoring script:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/log_monitor.sh

LOG_FILE="/opt/solana-bot/logs/bot.log"
ALERT_KEYWORDS="CRITICAL|ERROR|PANIC|Failed to execute trade"

# Monitor for critical issues
tail -F "$LOG_FILE" | while read line; do
    if echo "$line" | grep -E "$ALERT_KEYWORDS"; then
        # Send alert (implement your preferred notification method)
        echo "ALERT: $line" | mail -s "Solana Bot Alert" admin@yourcompany.com
    fi
done
```

## Additional Production Considerations

### Network Security

**Firewall Configuration (UFW):**
```bash
# Enable UFW
sudo ufw enable

# Allow SSH (change port if needed)
sudo ufw allow 22/tcp

# Allow monitor-only dashboard (if enabled)
sudo ufw allow from 192.168.1.0/24 to any port 3001

# Allow Prometheus monitoring (optional)
sudo ufw allow from 127.0.0.1 to any port 9090

# Block all other incoming traffic
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Check firewall status
sudo ufw status verbose
```

**Network monitoring:**
```bash
# Install network monitoring tools
sudo apt-get install nethogs iftop

# Monitor network usage
sudo nethogs  # Show per-process bandwidth usage
sudo iftop    # Show real-time network traffic
```

### Resource Limits and Tuning

**System-level optimization:**
```bash
# Increase file descriptor limits
echo 'arbitrage soft nofile 65536' | sudo tee -a /etc/security/limits.conf
echo 'arbitrage hard nofile 65536' | sudo tee -a /etc/security/limits.conf

# Optimize network settings for high-frequency trading
echo 'net.core.rmem_max = 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.core.wmem_max = 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.ipv4.tcp_rmem = 4096 65536 16777216' | sudo tee -a /etc/sysctl.conf
echo 'net.ipv4.tcp_wmem = 4096 65536 16777216' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

**Resource monitoring:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/resource_monitor.sh

# Monitor CPU and memory usage
while true; do
    CPU_USAGE=$(top -bn1 | grep "solana-arbitrage-bot" | awk '{print $9}')
    MEMORY_USAGE=$(ps -o pid,vsz,rss,comm -p $(pgrep solana-arbitrage-bot))
    
    echo "$(date): CPU: ${CPU_USAGE}%, Memory: ${MEMORY_USAGE}"
    
    # Alert if usage is too high
    if (( $(echo "$CPU_USAGE > 80" | bc -l) )); then
        echo "ALERT: High CPU usage detected"
    fi
    
    sleep 60
done
```

### Monitoring and Alerting

**Prometheus configuration:**
```yaml
# /opt/solana-bot/config/prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'solana-arbitrage-bot'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 10s
    metrics_path: /metrics
```

**Grafana dashboard setup:**
```bash
# Install Grafana
sudo apt-get install -y software-properties-common
sudo add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
wget -q -O - https://packages.grafana.com/gpg.key | sudo apt-key add -
sudo apt-get update && sudo apt-get install grafana

# Enable and start Grafana
sudo systemctl enable grafana-server
sudo systemctl start grafana-server
```

**Custom alert script:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/alerts.sh

# Check trading performance
DAILY_PROFIT=$(curl -s http://localhost:3001/api/stats/daily | jq '.profit_usd')
MIN_DAILY_PROFIT=10

if (( $(echo "$DAILY_PROFIT < $MIN_DAILY_PROFIT" | bc -l) )); then
    curl -X POST "$DISCORD_WEBHOOK_URL" \
      -H "Content-Type: application/json" \
      -d "{\"content\":\"⚠️ Daily profit below threshold: \$${DAILY_PROFIT}\"}"
fi

# Check system health
if ! systemctl is-active --quiet solana-arbitrage-bot; then
    curl -X POST "$DISCORD_WEBHOOK_URL" \
      -H "Content-Type: application/json" \
      -d "{\"content\":\"🚨 CRITICAL: Solana bot service is down!\"}"
fi
```

### Backup and Recovery Procedures

**Automated backup script:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/backup.sh

BACKUP_DIR="/secure/backup/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup configuration
cp -r /opt/solana-bot/config "$BACKUP_DIR/"

# Backup database
pg_dump solana_bot > "$BACKUP_DIR/database_backup.sql"

# Backup trade data
cp -r /opt/solana-bot/data "$BACKUP_DIR/"

# Backup logs (last 7 days)
find /opt/solana-bot/logs -name "*.log" -mtime -7 -exec cp {} "$BACKUP_DIR/" \;

# Compress backup
tar -czf "$BACKUP_DIR.tar.gz" -C /secure/backup "$(basename $BACKUP_DIR)"
rm -rf "$BACKUP_DIR"

echo "Backup completed: $BACKUP_DIR.tar.gz"
```

**Recovery procedures:**
```bash
#!/bin/bash
# /opt/solana-bot/scripts/recovery.sh

BACKUP_FILE="$1"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file.tar.gz>"
    exit 1
fi

# Stop service
sudo systemctl stop solana-arbitrage-bot

# Extract backup
TEMP_DIR="/tmp/recovery_$(date +%s)"
mkdir -p "$TEMP_DIR"
tar -xzf "$BACKUP_FILE" -C "$TEMP_DIR"

# Restore configuration
cp -r "$TEMP_DIR"/*/config/* /opt/solana-bot/config/

# Restore database
psql solana_bot < "$TEMP_DIR"/*/database_backup.sql

# Restore data
cp -r "$TEMP_DIR"/*/data/* /opt/solana-bot/data/

# Set permissions
sudo chown -R arbitrage:arbitrage /opt/solana-bot

# Start service
sudo systemctl start solana-arbitrage-bot

echo "Recovery completed from $BACKUP_FILE"
```

## Deployment Checklist

### Pre-Deployment
- [ ] Hardware requirements met
- [ ] Software dependencies installed
- [ ] Network connectivity tested
- [ ] RPC endpoints configured and tested
- [ ] Private keys securely stored
- [ ] Environment variables configured
- [ ] Database setup completed

### Deployment
- [ ] Monitor-only binary built successfully
- [ ] Configuration files validated
- [ ] systemd service created and configured
- [ ] Dedicated user account created
- [ ] File permissions set correctly
- [ ] Service started and running
- [ ] Health checks passing

### Post-Deployment
- [ ] Monitoring systems configured
- [ ] Alerting mechanisms tested
- [ ] Backup procedures implemented
- [ ] Log rotation configured
- [ ] Network security verified
- [ ] Performance monitoring active
- [ ] Emergency procedures documented
- [ ] Team training completed

## Troubleshooting Guide

### Common Issues and Solutions

**Service fails to start:**
```bash
# Check service logs
sudo journalctl -u solana-arbitrage-bot -n 50

# Check configuration syntax
/opt/solana-bot/target/release/solana-arbitrage-bot --config /opt/solana-bot/config/production.yaml --check-config

# Verify permissions
ls -la /opt/solana-bot/
```

**High memory usage:**
```bash
# Check memory usage
ps aux | grep solana-arbitrage-bot

# Monitor memory over time
watch -n 1 'ps -o pid,vsz,rss,comm -p $(pgrep solana-arbitrage-bot)'

# Check for memory leaks
valgrind --tool=massif /opt/solana-bot/target/release/solana-arbitrage-bot
```

**Network connectivity issues:**
```bash
# Test RPC connectivity
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' "$PRIMARY_RPC_URL"

# Test WebSocket connectivity
wscat -c "$PRIMARY_WS_URL"

# Check DNS resolution
nslookup mainnet.helius-rpc.com
```

### Performance Optimization Tips

1. **Use dedicated RPC endpoints** (Helius, QuickNode) for production
2. **Tune connection pool sizes** based on expected load
3. **Monitor and adjust resource limits** in systemd service
4. **Implement proper caching** for frequently accessed data
5. **Use SSD storage** for database and logs
6. **Optimize network settings** for low latency
7. **Regular performance profiling** to identify bottlenecks

This deployment guide provides a comprehensive approach to securely deploying the Solana Arbitrage Bot in production while maintaining the high performance and security standards required for automated trading operations.