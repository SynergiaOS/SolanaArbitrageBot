# Solana Arbitrage Bot - Minimalna Wykonalna Implementacja

## 🎯 Cel

Jeden bot. Jedna strategia. Zero bullshitu.

**Arbitraż między Raydium ↔ Orca na parach SOL/USDC**

## 📊 Metryki Sukcesu

- **Zysk**: >50 USD/dzień
- **Latencja**: <300ms (detekcja + egzekucja)
- **Uptime**: >95%
- **Kod**: <3000 linii

## 🏗️ Architektura (KISS)

```text
[Raydium WebSocket] ─┐
                     ├─→ [ArbitrageBot] ─→ [Executor] ─→ [Profit 💰]
[Orca WebSocket] ────┘        ↑
                              │
                        [Config: 0.3% min profit]
```

## 🛠️ Stack Technologiczny
- **Język**: Rust (100%)
- **Async Runtime**: Tokio
- **Solana SDK**: solana-client 2.0
- **WebSocket**: tokio-tungstenite
- **Serializacja**: bincode (szybsza niż JSON)
- **Baza**: SQLite (tylko logi transakcji)

## 📁 Struktura Projektu

```text
solana-arbitrage-bot/
├── Cargo.toml           # Jedna definicja dependencji
├── config.yaml          # Konfiguracja (RPC, wallet, progi)
├── src/
│   ├── main.rs         # Entry point + event loop
│   ├── monitor.rs      # WebSocket monitoring DEXów
│   ├── calculator.rs   # Kalkulacja zysku (gas, slippage)
│   ├── executor.rs     # Budowanie i wysyłanie transakcji
│   └── safety.rs       # Podstawowe zabezpieczenia
└── README.md          # Ten plik
```

## ⚡ Kluczowe Komponenty

### 1. Monitor (monitor.rs)

```rust
pub struct DexMonitor {
    raydium_price: Arc<Mutex<f64>>,
    orca_price: Arc<Mutex<f64>>,
}

impl DexMonitor {
    pub async fn start_monitoring(&self) {
        // WebSocket do Raydium/Orca
        // Aktualizacja cen w czasie rzeczywistym
    }
}
```

### 2. Calculator (calculator.rs)

```rust
pub struct ProfitCalculator {
    gas_cost: f64,      // ~0.00025 SOL
    slippage: f64,      // 0.5%
    min_profit: f64,    // 0.3%
}

impl ProfitCalculator {
    pub fn calculate(&self, price_a: f64, price_b: f64, amount: f64) -> Option<Opportunity> {
        let spread = (price_b - price_a).abs() / price_a;
        let profit = spread - self.gas_cost - self.slippage;

        if profit > self.min_profit {
            Some(Opportunity {
                buy_dex: if price_a < price_b { "Raydium" } else { "Orca" },
                sell_dex: if price_a < price_b { "Orca" } else { "Raydium" },
                profit_usd: profit * amount,
            })
        } else {
            None
        }
    }
}
```

### 3. Executor (executor.rs)

```rust
pub struct TransactionExecutor {
    rpc_client: RpcClient,
    wallet: Keypair,
}

impl TransactionExecutor {
    pub async fn execute_arbitrage(&self, opp: Opportunity) -> Result<Signature> {
        // 1. Buduj transakcję atomową (swap A -> swap B)
        // 2. Symuluj transakcję
        // 3. Jeśli OK, wyślij z wysokim priority fee
        // 4. Czekaj na potwierdzenie
    }
}
```

### 4. Safety (safety.rs)

```rust
pub struct SafetyLimits {
    max_position_size: f64,     // Max 100 SOL per trade
    max_daily_trades: u32,       // Max 50 trades/day
    max_daily_loss: f64,         // Stop po stracie 50 USD
    min_liquidity: f64,          // Min 10k USD w poolu
}
```

## 🚀 Plan Implementacji

### Tydzień 1: Fundament
- [ ] Setup projektu Rust + Cargo.toml
- [ ] WebSocket monitoring (Raydium + Orca)
- [ ] Podstawowy kalkulator profitu
- [ ] Testy jednostkowe

### Tydzień 2: Egzekucja
- [ ] Transaction builder (Jupiter Aggregator API)
- [ ] Symulacja transakcji przed wysłaniem
- [ ] Podstawowe safety checks
- [ ] Testnet deployment

### Tydzień 3: Produkcja
- [ ] Mainnet deployment (małe kwoty)
- [ ] Monitoring & logi
- [ ] Optymalizacja gas fees
- [ ] Skalowanie pozycji

## ⚙️ Konfiguracja (config.yaml)

```yaml
rpc:
  url: "https://api.mainnet-beta.solana.com"
  ws_url: "wss://api.mainnet-beta.solana.com"

wallet:
  path: "./wallet.json"  # Na początek zwykły keypair
  
dex:
  raydium:
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    ws_endpoint: "wss://api.raydium.io/v2/ws"
  orca:
    program_id: "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP"
    ws_endpoint: "wss://api.orca.so/v1/ws"

limits:
  max_position_sol: 100
  min_profit_percent: 0.3
  max_slippage_percent: 0.5
  max_daily_loss_usd: 50
```

## 📈 Monitoring

### Metryki (SQLite)
```sql
CREATE TABLE trades (
    id INTEGER PRIMARY KEY,
    timestamp DATETIME,
    buy_dex TEXT,
    sell_dex TEXT,
    amount_sol REAL,
    profit_usd REAL,
    gas_cost_sol REAL,
    signature TEXT
);

-- Dzienny P&L
SELECT DATE(timestamp), SUM(profit_usd) 
FROM trades 
GROUP BY DATE(timestamp);
```

### Alerty (stdout + opcjonalnie Telegram)
- ✅ Udany arbitraż > 10 USD
- ⚠️ Spread > 1% (duża okazja)
- 🛑 Strata > 20 USD (stop trading)

## 🔐 Bezpieczeństwo

### Faza 1 (Tydzień 1-2)
- Hardcoded limity pozycji
- Testnet only
- Dry-run mode

### Faza 2 (Tydzień 3+)
- Rate limiting
- Slippage protection
- Circuit breaker przy stratach

### Faza 3 (Miesiąc 2+)
- Hardware wallet (Ledger)
- Multi-sig dla dużych pozycji
- Zaawansowany risk management

## 🎮 Uruchomienie

```bash
# Kompilacja
cargo build --release

# Testy (lokalne)
cargo test
# Testy sieciowe (opcjonalne, zewnętrzne API) – uruchom ręcznie:
cargo test --test integration -- --ignored --nocapture

# Docker (produkcja, non-root user 'arbitrage')
docker build -t solana-arb-bot:prod .
docker run --rm -p 3001:3001 -v $(pwd)/data:/app/data -v $(pwd)/logs:/app/logs solana-arb-bot:prod

# REST API przykłady
curl -s http://127.0.0.1:3001/api/status | jq
curl -s -X POST -H 'Content-Type: application/json' \
  -d '{"min_profit_usd":0.5,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}' \
  http://127.0.0.1:3001/api/config | jq
curl -s -X POST -H 'Content-Type: application/json' \
  -d '{"reason":"manual stop","source":"ops"}' \
  http://127.0.0.1:3001/api/control/emergency | jq
```

## 📊 Oczekiwane Wyniki

### Pesymistyczny (70% czasu)
- 10-20 okazji/dzień
- 2-5 USD zysku per trade
- 20-50 USD dziennie

### Realistyczny (20% czasu)
- 20-50 okazji/dzień
- 5-10 USD zysku per trade
- 100-200 USD dziennie

### Optymistyczny (10% czasu)
- Duża zmienność rynku
- 10-50 USD per trade
- 500+ USD dziennie

## ❌ Czego NIE robimy

- ❌ AI/LLM - niepotrzebne dla arbitrażu
- ❌ Skomplikowana orchestracja - jeden proces
- ❌ Multi-chain - tylko Solana
- ❌ Frontend - tylko CLI + logi
- ❌ Microservices - monolit
- ❌ 10 różnych strategii - tylko arbitraż

## ✅ Następne Kroki (po sukcesie)

Jeśli bot zarabia stabilnie 50+ USD/dzień przez 2 tygodnie:

1. **Skalowanie**: Więcej par (USDT, wBTC, ETH)
2. **Więcej DEXów**: Jupiter, Serum
3. **MEV Protection**: Prywatne mempoole (Jito)
4. **Druga Strategia**: JIT liquidity lub sandwich

---

**TL;DR**: Prosty bot arbitrażowy w Rust. Monitoruje 2 DEXy, wykonuje atomowe swapy gdy spread > 0.3%. Cel: 50 USD/dzień. Timeline: 3 tygodnie do produkcji.
