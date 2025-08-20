# 🔐 Infrastruktura i Bezpieczeństwo - Przewodnik Produkcyjny

## 🌐 RPC Endpoints - Jakie IP Potrzebujesz

### **1. Darmowe RPC (Start/Testowanie)**
```yaml
# config.yaml - Darmowe opcje
rpc:
  url: "https://api.mainnet-beta.solana.com"          # Solana Labs (limit: 100 req/s)
  backup_url: "https://solana-api.projectserum.com"   # Serum (backup)
```

**Limity darmowych RPC:**
- ⚠️ 100 requests/second
- ⚠️ Brak gwarancji uptime
- ⚠️ Może być wolne w peak hours
- ⚠️ Brak priority routing

### **2. Płatne RPC (Produkcja) - REKOMENDOWANE**

#### **Helius (Najlepszy dla arbitrażu)**
```yaml
rpc:
  url: "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY"
  ws_url: "wss://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY"
```
- ✅ 1000+ req/s na planie Pro ($99/miesiąc)
- ✅ Priority routing
- ✅ Enhanced APIs (Jupiter integration)
- ✅ 99.9% uptime SLA

#### **QuickNode**
```yaml
rpc:
  url: "https://your-endpoint.solana-mainnet.quiknode.pro/YOUR_TOKEN/"
```
- ✅ Dedykowane endpoints
- ✅ Global infrastructure
- ✅ $49/miesiąc za 50M requests

#### **Alchemy**
```yaml
rpc:
  url: "https://solana-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
```
- ✅ Dobre dla beginnerów
- ✅ Free tier: 300M requests/miesiąc

### **3. VPS/Serwer Requirements**

#### **Minimalne wymagania:**
- **CPU**: 2 vCPU (Intel/AMD)
- **RAM**: 4GB
- **Storage**: 20GB SSD
- **Network**: 100 Mbps
- **OS**: Ubuntu 22.04 LTS

#### **Rekomendowane VPS providery:**
```bash
# Hetzner (Europa) - €4.15/miesiąc
# 2 vCPU, 4GB RAM, 40GB SSD
# Lokalizacja: Niemcy/Finlandia

# DigitalOcean - $24/miesiąc  
# 2 vCPU, 4GB RAM, 80GB SSD
# Lokalizacja: Frankfurt/Amsterdam

# Vultr - $12/miesiąc
# 2 vCPU, 4GB RAM, 80GB SSD
# Lokalizacja: Frankfurt
```

## 🔑 Zarządzanie Kluczami - Security Best Practices

### **1. Poziomy Bezpieczeństwa**

#### **Level 1: Development/Testing**
```bash
# Generowanie test wallet
solana-keygen new --outfile ./test_wallet.json

# W config.yaml
wallet:
  use_ledger: false
  path: "./test_wallet.json"
```
- ⚠️ Tylko do testów!
- ⚠️ Maksymalnie 1-2 SOL

#### **Level 2: Small Production (1-10 SOL)**
```bash
# Bezpieczne generowanie
solana-keygen new --outfile ./wallet.json

# Backup offline
cp wallet.json /secure/backup/location/
chmod 600 wallet.json

# W config.yaml
wallet:
  use_ledger: false
  path: "./wallet.json"
```

#### **Level 3: Large Production (10+ SOL) - LEDGER REQUIRED**
```bash
# Setup Ledger
./setup_ledger_ubuntu.sh

# Test connection
./test_ledger.sh

# W config.yaml
wallet:
  use_ledger: true
  ledger_path: "m/44'/501'/0'/0'"
  path: "./wallet.json"  # Fallback tylko
```

### **2. Secure Wallet Setup**

#### **Generowanie Production Wallet:**
```bash
#!/bin/bash
# secure_wallet_setup.sh

# 1. Generuj na offline machine
solana-keygen new --outfile ./production_wallet.json

# 2. Backup w 3 lokalizacjach
cp production_wallet.json /backup1/
cp production_wallet.json /backup2/
cp production_wallet.json /backup3/

# 3. Ustaw permissions
chmod 600 production_wallet.json
chown bot:bot production_wallet.json

# 4. Zaszyfruj backup
gpg --symmetric --cipher-algo AES256 production_wallet.json

# 5. Usuń plaintext z backup locations
rm /backup1/production_wallet.json
rm /backup2/production_wallet.json  
rm /backup3/production_wallet.json
```

#### **Environment Variables (Alternatywa):**
```bash
# .env file (nigdy nie commituj!)
SOLANA_PRIVATE_KEY="[57,132,181,10,245,111,186,100,...]"
RPC_API_KEY="your_helius_api_key"

# W kodzie:
use std::env;

fn load_wallet_from_env() -> Result<Keypair> {
    let key_str = env::var("SOLANA_PRIVATE_KEY")?;
    let bytes: Vec<u8> = serde_json::from_str(&key_str)?;
    Ok(Keypair::from_bytes(&bytes)?)
}
```

### **3. Ledger Integration (Najbezpieczniejsze)**

#### **Setup na Ubuntu:**
```bash
# 1. Install dependencies
sudo apt update
sudo apt install libudev-dev libusb-1.0-0-dev

# 2. Udev rules dla Ledger
echo 'SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", MODE="0666"' | sudo tee /etc/udev/rules.d/20-ledger.rules
sudo udevadm control --reload-rules

# 3. Test connection
lsusb | grep Ledger
```

#### **Bot Configuration z Ledger:**
```yaml
# config.yaml - Production setup
wallet:
  use_ledger: true
  ledger_path: "m/44'/501'/0'/0'"  # Standard Solana path
  
  # Fallback tylko dla emergency (mały balance!)
  path: "./emergency_wallet.json"

limits:
  max_position_sol: 50.0  # Większe pozycje z Ledger
```

### **4. Network Security**

#### **Firewall Setup:**
```bash
# UFW configuration
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Allow SSH (zmień port!)
sudo ufw allow 2222/tcp

# Allow tylko potrzebne connections
sudo ufw allow out 443/tcp  # HTTPS dla RPC
sudo ufw allow out 80/tcp   # HTTP backup

sudo ufw enable
```

#### **SSH Hardening:**
```bash
# /etc/ssh/sshd_config
Port 2222                    # Zmień default port
PermitRootLogin no
PasswordAuthentication no    # Tylko SSH keys
PubkeyAuthentication yes
MaxAuthTries 3
```

### **5. Monitoring i Alerty**

#### **System Monitoring:**
```bash
# Install monitoring
sudo apt install htop iotop nethogs

# Log monitoring
tail -f /var/log/syslog | grep arbitrage
journalctl -u arbitrage-bot -f
```

#### **Balance Monitoring:**
```rust
// W main.rs - dodaj balance check
async fn check_wallet_balance(executor: &TransactionExecutor) -> Result<f64> {
    let balance = executor.get_balance().await?;
    
    if balance < 0.1 {
        error!("🚨 LOW BALANCE: {} SOL", balance);
        // Send alert (Telegram/Discord/Email)
    }
    
    Ok(balance)
}
```

## 🚨 Emergency Procedures

### **1. Jeśli Wallet Compromised:**
```bash
# 1. STOP BOT IMMEDIATELY
sudo systemctl stop arbitrage-bot

# 2. Transfer funds to safe wallet
solana transfer --from ./compromised_wallet.json \
  NEW_SAFE_ADDRESS ALL --allow-unfunded-recipient

# 3. Generate new wallet
solana-keygen new --outfile ./new_wallet.json

# 4. Update config and restart
```

### **2. Jeśli RPC Down:**
```yaml
# Backup RPC w config.yaml
rpc:
  url: "https://mainnet.helius-rpc.com/?api-key=KEY1"
  backup_urls:
    - "https://your-endpoint.quiknode.pro/TOKEN/"
    - "https://api.mainnet-beta.solana.com"
    - "https://solana-api.projectserum.com"
```

## 📋 Production Checklist

### **Pre-deployment:**
- [ ] Ledger setup i test
- [ ] Płatny RPC provider
- [ ] VPS z proper security
- [ ] Backup wallet w 3 lokalizacjach
- [ ] Monitoring setup
- [ ] Emergency procedures tested

### **Go-live:**
- [ ] Start z małą pozycją (1-2 SOL)
- [ ] Monitor przez pierwsze 24h
- [ ] Sprawdź wszystkie alerty
- [ ] Verify profit calculations
- [ ] Scale up gradually

## 🌍 Konkretne IP i Endpoints

### **Helius RPC Endpoints:**
```bash
# Mainnet
https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
wss://mainnet.helius-rpc.com/?api-key=YOUR_KEY

# IP ranges (dla firewall whitelist):
# 34.102.136.180/32
# 35.247.65.190/32
# 34.145.67.134/32
```

### **QuickNode Endpoints:**
```bash
# Format: https://[name].solana-mainnet.quiknode.pro/[token]/
# IP: Dynamiczne (używaj DNS)
```

### **Alchemy:**
```bash
# Mainnet
https://solana-mainnet.g.alchemy.com/v2/YOUR_API_KEY

# IP ranges:
# 18.144.73.0/24
# 52.8.0.0/16
```

## 🔧 Advanced Security Setup

### **1. Systemd Service (Production Deployment):**
```bash
# /etc/systemd/system/arbitrage-bot.service
[Unit]
Description=Solana Arbitrage Bot
After=network.target

[Service]
Type=simple
User=bot
Group=bot
WorkingDirectory=/home/bot/SolanaArbitrageBot
ExecStart=/home/bot/SolanaArbitrageBot/target/release/solana-arbitrage-bot
Restart=always
RestartSec=10
Environment=RUST_LOG=info

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/bot/SolanaArbitrageBot

[Install]
WantedBy=multi-user.target
```

### **2. Log Rotation:**
```bash
# /etc/logrotate.d/arbitrage-bot
/home/bot/SolanaArbitrageBot/logs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 bot bot
}
```

### **3. Automated Backup Script:**
```bash
#!/bin/bash
# backup_bot.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/secure/backups"

# Backup wallet (encrypted)
gpg --symmetric --cipher-algo AES256 \
    --output "$BACKUP_DIR/wallet_$DATE.gpg" \
    /home/bot/SolanaArbitrageBot/wallet.json

# Backup config
cp /home/bot/SolanaArbitrageBot/config.yaml \
   "$BACKUP_DIR/config_$DATE.yaml"

# Backup trade database
cp /home/bot/SolanaArbitrageBot/trades.db \
   "$BACKUP_DIR/trades_$DATE.db"

# Clean old backups (keep 30 days)
find "$BACKUP_DIR" -name "*.gpg" -mtime +30 -delete
find "$BACKUP_DIR" -name "*.yaml" -mtime +30 -delete
find "$BACKUP_DIR" -name "*.db" -mtime +30 -delete

echo "Backup completed: $DATE"
```

### **4. Health Check Script:**
```bash
#!/bin/bash
# health_check.sh

BOT_PID=$(pgrep -f "solana-arbitrage-bot")

if [ -z "$BOT_PID" ]; then
    echo "🚨 Bot not running! Restarting..."
    systemctl restart arbitrage-bot

    # Send alert
    curl -X POST "https://api.telegram.org/bot$TELEGRAM_TOKEN/sendMessage" \
         -d chat_id="$CHAT_ID" \
         -d text="🚨 Arbitrage Bot restarted on $(hostname)"
fi

# Check balance
BALANCE=$(solana balance --url https://api.mainnet-beta.solana.com)
if (( $(echo "$BALANCE < 0.1" | bc -l) )); then
    echo "🚨 Low balance: $BALANCE SOL"
    # Send alert
fi
```

## 📱 Telegram Alerts Setup

### **1. Create Telegram Bot:**
```bash
# 1. Message @BotFather on Telegram
# 2. Create new bot: /newbot
# 3. Get token: 123456789:ABCdefGHIjklMNOpqrsTUVwxyz
# 4. Get your chat_id: message bot, then visit:
#    https://api.telegram.org/bot<TOKEN>/getUpdates
```

### **2. Alert Integration:**
```rust
// src/alerts.rs
use reqwest;

pub struct TelegramAlert {
    token: String,
    chat_id: String,
    client: reqwest::Client,
}

impl TelegramAlert {
    pub async fn send_alert(&self, message: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);

        let payload = serde_json::json!({
            "chat_id": self.chat_id,
            "text": message,
            "parse_mode": "HTML"
        });

        self.client.post(&url).json(&payload).send().await?;
        Ok(())
    }

    pub async fn profit_alert(&self, profit: f64, signature: &str) -> Result<()> {
        let message = format!(
            "💰 <b>Profit Alert!</b>\n\
             Amount: ${:.2}\n\
             Signature: <code>{}</code>",
            profit, signature
        );
        self.send_alert(&message).await
    }
}
```

---

**⚠️ PAMIĘTAJ**:
- Nigdy nie trzymaj więcej niż 100 SOL w hot wallet
- Używaj Ledger dla pozycji > 10 SOL
- Backup wallet w 3 różnych lokalizacjach
- Monitor balance 24/7
- Test wszystkie emergency procedures!
