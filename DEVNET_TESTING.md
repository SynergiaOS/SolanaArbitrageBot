# 🧪 Devnet Testing - Bezpieczny Start

## Dlaczego Devnet Najpierw?

✅ **Darmowe tokeny** - nie ryzykujesz prawdziwych pieniędzy  
✅ **Pełne testowanie** - wszystkie funkcje jak na mainnet  
✅ **Ledger testing** - przetestujesz integrację z Ledger  
✅ **API validation** - sprawdzisz czy wszystko działa  
✅ **Confidence building** - nabierzesz pewności przed mainnet  

## 🚀 Step-by-Step Devnet Setup

### **1. Konfiguracja Devnet**

Stwórz `config_devnet.yaml`:

```yaml
# Devnet Configuration
rpc:
  url: "https://api.devnet.solana.com"
  ws_url: "wss://api.devnet.solana.com"
  backup_url: "https://devnet.helius-rpc.com/?api-key=YOUR_KEY"  # Opcjonalnie

wallet:
  use_ledger: false  # Na początek zwykły wallet
  path: "./devnet_wallet.json"
  
  # Później przetestuj Ledger:
  # use_ledger: true
  # ledger_path: "m/44'/501'/0'/0'"

dex:
  raydium:
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    sol_usdc_pool: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"  # Może nie istnieć na devnet
  orca:
    program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    sol_usdc_pool: "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"  # Może nie istnieć na devnet

limits:
  max_position_sol: 1.0           # Małe pozycje na devnet
  min_profit_percent: 0.1         # Niższy próg dla testów
  min_profit_usd: 0.10            # $0.10 minimum
  max_slippage_percent: 1.0       # Wyższy slippage OK na devnet
  max_daily_loss_usd: 10.0        # $10 max loss
  max_daily_trades: 100           # Więcej tradów dla testów
  min_pool_liquidity_usd: 1000.0  # Niższe wymagania

execution:
  priority_fee_lamports: 5000     # Niższe fees na devnet
  simulation_required: true       # Zawsze symuluj
  max_retries: 3

monitoring:
  log_level: "debug"              # Więcej logów dla testów
  db_path: "./devnet_trades.db"
  alert_profit_threshold_usd: 1.0
```

### **2. Generowanie Devnet Wallet**

```bash
# Generuj nowy wallet dla devnet
solana-keygen new --outfile ./devnet_wallet.json

# Ustaw devnet jako default
solana config set --url https://api.devnet.solana.com

# Sprawdź adres
solana address

# Zdobądź darmowe SOL (2 SOL na raz)
solana airdrop 2

# Sprawdź balance
solana balance
```

### **3. Test API Connections na Devnet**

```bash
# Kompiluj bot
cargo build --release

# Test API connections
./target/release/solana-arbitrage-bot \
  --config ./config_devnet.yaml \
  --test-apis \
  --network devnet
```

### **4. Dry Run na Devnet**

```bash
# Dry run z devnet config
./target/release/solana-arbitrage-bot \
  --config ./config_devnet.yaml \
  --dry-run \
  --network devnet \
  --max-position 0.5

# Obserwuj logi:
# 📊 Price update from Raydium: $XXX
# 📊 Price update from Orca: $XXX  
# 🎯 Opportunity found: ...
# 🏃 DRY RUN - Would execute trade...
```

### **5. Live Trading na Devnet**

```bash
# Prawdziwe transakcje z devnet tokenami
./target/release/solana-arbitrage-bot \
  --config ./config_devnet.yaml \
  --network devnet \
  --max-position 0.1

# Monitoruj pierwsze transakcje bardzo uważnie!
```

## 🔍 Co Testować na Devnet

### **Podstawowe Funkcje:**
- [ ] ✅ API connections (Raydium, Orca, Jupiter)
- [ ] ✅ Price monitoring w czasie rzeczywistym
- [ ] ✅ Arbitrage detection
- [ ] ✅ Transaction building
- [ ] ✅ Transaction simulation
- [ ] ✅ Transaction execution
- [ ] ✅ Profit calculation
- [ ] ✅ Safety limits

### **Advanced Features:**
- [ ] ✅ Ledger integration (zmień `use_ledger: true`)
- [ ] ✅ Network congestion handling
- [ ] ✅ Priority fee adjustment
- [ ] ✅ Slippage protection
- [ ] ✅ Error recovery
- [ ] ✅ Database logging

### **Performance Testing:**
- [ ] ✅ Latency measurement (detekcja -> egzekucja)
- [ ] ✅ Memory usage monitoring
- [ ] ✅ CPU usage under load
- [ ] ✅ Network bandwidth usage

## 📊 Devnet vs Mainnet Różnice

| Aspekt | Devnet | Mainnet |
|--------|--------|---------|
| **Tokeny** | Darmowe | Prawdziwe $ |
| **Pools** | Mniej płynności | Pełna płynność |
| **Latency** | Może być wyższa | Produkcyjna |
| **Fees** | Niższe | Normalne |
| **Risk** | Zero | Prawdziwe ryzyko |

## 🚨 Znane Problemy na Devnet

### **1. Brak Puli DEX**
```bash
# Jeśli pool nie istnieje na devnet:
Error: Pool not found: 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2
```

**Rozwiązanie**: Użyj testowych pool addresses lub stwórz własne.

### **2. Jupiter API Limitations**
```bash
# Jupiter może mieć ograniczone wsparcie dla devnet
Error: No route found for devnet
```

**Rozwiązanie**: Test z fallback na direct DEX calls.

### **3. Niska Płynność**
```bash
# Małe pools na devnet
Warning: Low liquidity detected: $100 USD
```

**Rozwiązanie**: Obniż `min_pool_liquidity_usd` w config.

## 🎯 Checklist Przed Mainnet

Po udanych testach na devnet:

### **Technical Readiness:**
- [ ] ✅ Bot działa stabilnie przez 24h na devnet
- [ ] ✅ Wszystkie API calls działają
- [ ] ✅ Ledger integration przetestowany
- [ ] ✅ Profit calculations zweryfikowane
- [ ] ✅ Safety limits działają poprawnie
- [ ] ✅ Error handling przetestowany

### **Operational Readiness:**
- [ ] ✅ Monitoring setup (logi, alerty)
- [ ] ✅ Backup procedures przetestowane
- [ ] ✅ Emergency stop procedures
- [ ] ✅ RPC provider wybrany (Helius/QuickNode)
- [ ] ✅ VPS/server przygotowany

### **Financial Readiness:**
- [ ] ✅ Mainnet wallet setup (Phantom + Ledger)
- [ ] ✅ Starting capital ready ($50 = ~0.3 SOL)
- [ ] ✅ Risk management plan
- [ ] ✅ Position sizing strategy

## 🚀 Przejście na Mainnet

Gdy wszystko działa na devnet:

```bash
# 1. Backup devnet config
cp config_devnet.yaml config_devnet_backup.yaml

# 2. Stwórz mainnet config
cp config_devnet.yaml config.yaml

# 3. Zmień na mainnet endpoints
sed -i 's/devnet/mainnet-beta/g' config.yaml

# 4. Ustaw konserwatywne limity
# max_position_sol: 0.1  # Start z 0.1 SOL
# min_profit_percent: 0.3  # Wyższy próg

# 5. Test z dry-run na mainnet
./target/release/solana-arbitrage-bot --dry-run

# 6. Go live z małą pozycją!
./target/release/solana-arbitrage-bot --max-position 0.05
```

---

**🎯 Następny krok**: Uruchom devnet testing i sprawdź czy wszystko działa przed użyciem prawdziwych 50$ z Phantom!
