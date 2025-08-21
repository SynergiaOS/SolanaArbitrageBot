## 📋 PR1: Wczytywanie config.safety + podpięcie do SafetyChecker

### 🎯 **Cel PR**: 
Pełna integracja konfiguracji safety z pliku YAML do runtime SafetyChecker, eliminując hardcoded wartości.

---

## 📝 **Task 1: Dodanie struktur konfiguracji safety**

### Pliki do modyfikacji:
- `src/sniper/mod.rs`
- `src/bin/sniper.rs`

### Implementacja:

```rust
// src/sniper/mod.rs - DODAJ nowe struktury
#[derive(Debug, Clone, Deserialize)]
pub struct SafetyConfig {
    // Liquidity & Market Cap
    pub min_liquidity_sol: f64,
    pub max_market_cap_usd: f64,
    
    // Taxes
    pub max_buy_tax_percent: f64,
    pub max_sell_tax_percent: f64,
    
    // Token Age & Holders
    pub max_token_age_minutes: u64,
    pub min_holders: u32,
    pub max_dev_percentage: f64,
    
    // Blacklists
    pub blacklist_mints: Vec<String>,
    pub blacklisted_creators: Vec<String>,
    pub blacklist_keywords: Vec<String>,
    
    // API Endpoints
    pub honeypot_api: Option<String>,
    pub rugcheck_api: Option<String>,
    pub enable_safety_checks: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            min_liquidity_sol: 3.0,
            max_market_cap_usd: 100_000.0,
            max_buy_tax_percent: 5.0,
            max_sell_tax_percent: 5.0,
            max_token_age_minutes: 60,
            min_holders: 10,
            max_dev_percentage: 30.0,
            blacklist_mints: vec![],
            blacklisted_creators: vec![],
            blacklist_keywords: vec![
                "test".to_string(),
                "fake".to_string(),
                "scam".to_string(),
                "rug".to_string(),
                "honeypot".to_string(),
            ],
            honeypot_api: Some("https://api.honeypot.is/v2/IsHoneypot".to_string()),
            rugcheck_api: Some("https://api.rugcheck.xyz/v1/tokens".to_string()),
            enable_safety_checks: true,
        }
    }
}
```

### DoD (Definition of Done):
- ✅ Struktura `SafetyConfig` zdefiniowana w `src/sniper/mod.rs`
- ✅ Implementacja `Default` z sensownymi wartościami
- ✅ Struktura jest `Deserialize` dla YAML
- ✅ Eksportowana w module publicznym

---

## 📝 **Task 2: Rozszerzenie parsowania YAML w sniper.rs**

### Pliki do modyfikacji:
- `src/bin/sniper.rs`

### Implementacja:

```rust
// src/bin/sniper.rs - MODYFIKUJ struktury YAML
#[derive(Debug, Deserialize)]
struct RootYamlConfig {
    rpc: Option<RpcSection>,
    wallet: Option<WalletSection>,
    sniper: Option<SniperSection>,
    safety: Option<SafetySection>,  // Już jest, ale rozszerz
    risk: Option<RiskSection>,      // DODAJ
}

#[derive(Debug, Deserialize, Clone)]
struct SafetySection {
    // Rozszerz istniejącą strukturę
    min_liquidity_sol: Option<f64>,
    max_market_cap_usd: Option<f64>,
    max_buy_tax_percent: Option<f64>,
    max_sell_tax_percent: Option<f64>,
    max_token_age_minutes: Option<u64>,
    min_holders: Option<u32>,
    max_dev_percentage: Option<f64>,
    
    // Blacklists
    blacklist_mints: Option<Vec<String>>,
    blacklisted_creators: Option<Vec<String>>,
    blacklist_keywords: Option<Vec<String>>,
    
    // APIs
    honeypot_api: Option<String>,
    rugcheck_api: Option<String>,
    enable_safety_checks: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
struct RiskSection {
    max_daily_loss_sol: Option<f64>,
    max_positions: Option<u32>,
    position_timeout_minutes: Option<u64>,
}

// MODYFIKUJ funkcję load_config_sources
fn load_config_sources(args: &Args) -> Result<LoadedConfig> {
    // ... existing code ...
    
    // Dodaj inicjalizację SafetyConfig
    let mut safety_config = SafetyConfig::default();
    
    // Load YAML if present
    if Path::new(&args.config).exists() {
        let content = std::fs::read_to_string(&args.config)?;
        let root: RootYamlConfig = serde_yaml::from_str(&content)?;
        
        // Mapuj safety section
        if let Some(safe) = root.safety {
            if let Some(v) = safe.min_liquidity_sol { 
                safety_config.min_liquidity_sol = v; 
            }
            if let Some(v) = safe.max_market_cap_usd { 
                safety_config.max_market_cap_usd = v; 
            }
            if let Some(v) = safe.max_buy_tax_percent { 
                safety_config.max_buy_tax_percent = v; 
            }
            if let Some(v) = safe.max_sell_tax_percent { 
                safety_config.max_sell_tax_percent = v; 
            }
            if let Some(v) = safe.max_token_age_minutes { 
                safety_config.max_token_age_minutes = v; 
            }
            if let Some(v) = safe.min_holders { 
                safety_config.min_holders = v; 
            }
            if let Some(v) = safe.max_dev_percentage { 
                safety_config.max_dev_percentage = v; 
            }
            if let Some(v) = safe.blacklist_mints { 
                safety_config.blacklist_mints = v; 
            }
            if let Some(v) = safe.blacklisted_creators { 
                safety_config.blacklisted_creators = v; 
            }
            if let Some(v) = safe.blacklist_keywords { 
                safety_config.blacklist_keywords = v; 
            }
            if let Some(v) = safe.honeypot_api { 
                safety_config.honeypot_api = Some(v); 
            }
            if let Some(v) = safe.rugcheck_api { 
                safety_config.rugcheck_api = Some(v); 
            }
            if let Some(v) = safe.enable_safety_checks { 
                safety_config.enable_safety_checks = v; 
            }
        }
        
        // Podobnie dla risk section
        if let Some(risk) = root.risk {
            if let Some(v) = risk.position_timeout_minutes {
                config.position_timeout_minutes = v; // Dodaj to pole do SniperConfig
            }
        }
    }
    
    // Zwróć z safety_config
    Ok(LoadedConfig {
        sniper_config: config,
        safety_config,  // NOWE
        rpc_url,
        ws_url,
        wallet_path,
    })
}

// Rozszerz LoadedConfig
struct LoadedConfig {
    sniper_config: SniperConfig,
    safety_config: SafetyConfig,  // NOWE
    rpc_url: String,
    ws_url: String,
    wallet_path: String,
}
```

### DoD:
- ✅ `SafetySection` zawiera wszystkie pola z config.yaml
- ✅ `RiskSection` dodana dla timeout pozycji
- ✅ Mapowanie z YAML do `SafetyConfig` działa
- ✅ `LoadedConfig` zawiera `safety_config`
- ✅ Wartości domyślne używane gdy brak w YAML

---

## 📝 **Task 3: Przekazanie SafetyConfig do SniperEngine**

### Pliki do modyfikacji:
- `src/sniper/mod.rs`
- `src/bin/sniper.rs`

### Implementacja:

```rust
// src/sniper/mod.rs - MODYFIKUJ SniperEngine
pub struct SniperEngine {
    keypair: Arc<Keypair>,
    rpc_client: Arc<RpcClient>,
    config: SniperConfig,
    safety_config: SafetyConfig,  // NOWE
    token_monitor: TokenMonitor,
    trade_executor: TradeExecutor,
    safety_checker: SafetyChecker,
    position_manager: PositionManager,
    dry_run: bool,
}

impl SniperEngine {
    pub fn new(
        keypair: Arc<Keypair>,
        rpc_url: String,
        ws_url: String,
        config: SniperConfig,
        safety_config: SafetyConfig,  // NOWE
        dry_run: bool,
    ) -> Self {
        let rpc_client = Arc::new(RpcClient::new(rpc_url));
        
        let token_monitor = TokenMonitor::new(rpc_client.clone(), ws_url);
        let trade_executor = TradeExecutor::new(
            keypair.clone(),
            rpc_client.clone(),
            "https://quote-api.jup.ag/v6".to_string(),
        );
        
        // Przekaż safety_config do SafetyChecker
        let safety_checker = SafetyChecker::from_config(&safety_config);  // NOWE
        
        let position_manager = PositionManager::new(config.clone());
        
        Self {
            keypair,
            rpc_client,
            config,
            safety_config,  // NOWE
            token_monitor,
            trade_executor,
            safety_checker,
            position_manager,
            dry_run,
        }
    }
}
```

```rust
// src/bin/sniper.rs - MODYFIKUJ tworzenie SniperEngine
let sniper = SniperEngine::new(
    Arc::new(wallet),
    loaded.rpc_url,
    loaded.ws_url,
    loaded.sniper_config,
    loaded.safety_config,  // NOWE
    args.dry_run,
);
```

### DoD:
- ✅ `SniperEngine` przyjmuje `SafetyConfig` w konstruktorze
- ✅ `SafetyConfig` przekazywana do `SafetyChecker`
- ✅ Binary `sniper.rs` przekazuje config z YAML

---

## 📝 **Task 4: Refaktoryzacja SafetyChecker z użyciem SafetyConfig**

### Pliki do modyfikacji:
- `src/sniper/safety.rs`

### Implementacja:

```rust
// src/sniper/safety.rs - REFAKTORYZACJA
use crate::sniper::{NewToken, SafetyConfig};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub struct SafetyChecker {
    http_client: Client,
    config: SafetyConfig,  // Zamiast rozproszonych pól
    blacklisted_creators: HashSet<Pubkey>,
    blacklisted_mints: HashSet<Pubkey>,
    blacklisted_keywords: HashSet<String>,
}

impl SafetyChecker {
    pub fn from_config(config: &SafetyConfig) -> Self {
        // Konwersja String -> Pubkey dla blacklist
        let mut blacklisted_creators = HashSet::new();
        for creator_str in &config.blacklisted_creators {
            if let Ok(pubkey) = Pubkey::from_str(creator_str) {
                blacklisted_creators.insert(pubkey);
            } else {
                warn!("Invalid creator pubkey in blacklist: {}", creator_str);
            }
        }
        
        let mut blacklisted_mints = HashSet::new();
        for mint_str in &config.blacklist_mints {
            if let Ok(pubkey) = Pubkey::from_str(mint_str) {
                blacklisted_mints.insert(pubkey);
            } else {
                warn!("Invalid mint pubkey in blacklist: {}", mint_str);
            }
        }
        
        let blacklisted_keywords: HashSet<String> = config.blacklist_keywords
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        
        Self {
            http_client: Client::new(),
            config: config.clone(),
            blacklisted_creators,
            blacklisted_mints,
            blacklisted_keywords,
        }
    }
    
    pub async fn check_token(&self, token: &NewToken) -> Result<SafetyResult> {
        // Sprawdź czy safety checks są włączone
        if !self.config.enable_safety_checks {
            return Ok(SafetyResult::Safe);
        }
        
        // Check blacklisted mints
        if self.blacklisted_mints.contains(&token.mint) {
            return Ok(SafetyResult::Unsafe("Blacklisted mint".to_string()));
        }
        
        // Check blacklisted creators
        if self.blacklisted_creators.contains(&token.creator) {
            return Ok(SafetyResult::Unsafe("Blacklisted creator".to_string()));
        }
        
        // Check suspicious keywords
        if self.has_suspicious_keywords(token) {
            return Ok(SafetyResult::Unsafe("Suspicious name/symbol".to_string()));
        }
        
        // Check minimum liquidity - użyj config zamiast hardcoded
        let min_liquidity_lamports = (self.config.min_liquidity_sol * 1_000_000_000.0) as u64;
        if token.initial_liquidity < min_liquidity_lamports {
            return Ok(SafetyResult::Unsafe(
                format!("Insufficient liquidity: {} SOL < {} SOL minimum", 
                    token.initial_liquidity as f64 / 1_000_000_000.0,
                    self.config.min_liquidity_sol
                )
            ));
        }
        
        // Check honeypot - użyj URL z config
        if let Some(api_url) = &self.config.honeypot_api {
            if let Ok(is_honeypot) = self.check_honeypot_api(&token.mint, api_url).await {
                if is_honeypot {
                    return Ok(SafetyResult::Unsafe("Honeypot detected".to_string()));
                }
            }
        }
        
        // Check token taxes - użyj URL i progów z config
        if let Some(api_url) = &self.config.rugcheck_api {
            if let Ok((buy_tax, sell_tax)) = self.check_token_taxes(&token.mint, api_url).await {
                if buy_tax > self.config.max_buy_tax_percent {
                    return Ok(SafetyResult::Unsafe(
                        format!("Buy tax too high: {}% > {}% max", 
                            buy_tax, self.config.max_buy_tax_percent)
                    ));
                }
                if sell_tax > self.config.max_sell_tax_percent {
                    return Ok(SafetyResult::Unsafe(
                        format!("Sell tax too high: {}% > {}% max", 
                            sell_tax, self.config.max_sell_tax_percent)
                    ));
                }
            }
        }
        
        info!("✅ Token passed all safety checks: {}", token.mint);
        Ok(SafetyResult::Safe)
    }
    
    // Zmodyfikuj check_honeypot_api - przyjmij api_url jako parametr
    async fn check_honeypot_api(&self, mint: &Pubkey, api_url: &str) -> Result<bool> {
        let url = format!("{}?address={}", api_url, mint);
        
        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let data: serde_json::Value = response.json().await?;
                    if let Some(is_honeypot) = data.get("IsHoneypot").and_then(|v| v.as_bool()) {
                        return Ok(is_honeypot);
                    }
                }
            }
            Err(e) => {
                debug!("Honeypot API error: {}", e);
            }
        }
        Ok(false)
    }
    
    // Zmodyfikuj check_token_taxes - przyjmij api_url jako parametr
    async fn check_token_taxes(&self, mint: &Pubkey, api_url: &str) -> Result<(f64, f64)> {
        let url = format!("{}/{}", api_url, mint);
        
        if let Ok(resp) = self.http_client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    let buy = v.get("buyTax").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let sell = v.get("sellTax").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    return Ok((buy, sell));
                }
            }
        }
        Ok((0.0, 0.0))
    }
}
```

### DoD:
- ✅ `SafetyChecker::from_config()` konstruktor używający `SafetyConfig`
- ✅ Wszystkie hardcoded wartości zastąpione przez config
- ✅ Blacklisty inicjalizowane z config
- ✅ API URLs pobierane z config
- ✅ Progi (liquidity, taxes) z config
- ✅ `enable_safety_checks` flag respektowany

---

## 📝 **Task 5: Aktualizacja config_sniper.yaml**

### Pliki do modyfikacji:
- `config_sniper.yaml`

### Implementacja:

```yaml
# config_sniper.yaml - ROZSZERZ o pełną sekcję safety
safety:
  # Podstawowe filtry
  min_liquidity_sol: 3.0           # Minimum liquidity in SOL
  max_market_cap_usd: 100000.0     # Max $100k market cap
  max_buy_tax_percent: 5.0         # Max 5% buy tax
  max_sell_tax_percent: 5.0        # Max 5% sell tax
  
  # Token age i holders
  max_token_age_minutes: 60        # Only tokens < 60 min old
  min_holders: 10                  # Minimum 10 holders
  max_dev_percentage: 30.0         # Max 30% held by single wallet
  
  # Blacklisty
  blacklist_mints:
    - "11111111111111111111111111111111"  # System program
    - "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"  # Token program
    
  blacklisted_creators:
    # Dodaj znanych scammerów
    - "ScamCreator11111111111111111111111111111111"
    
  blacklist_keywords:
    - "test"
    - "fake"
    - "scam"
    - "rug"
    - "honeypot"
    - "ponzi"
    - "pyramid"
    
  # API endpoints
  honeypot_api: "https://api.honeypot.is/v2/IsHoneypot"
  rugcheck_api: "https://api.rugcheck.xyz/v1/tokens"
  enable_safety_checks: true
  
# Risk management z timeout
risk:
  max_daily_loss_sol: 0.5
  max_positions: 3
  position_timeout_minutes: 60     # Auto-sell after 60 min
```

### DoD:
- ✅ Wszystkie pola `SafetyConfig` mają przykładowe wartości
- ✅ Komentarze wyjaśniające każde pole
- ✅ Sensowne wartości domyślne dla produkcji

---

## 📝 **Task 6: Testy jednostkowe SafetyChecker z config**

### Pliki do utworzenia/modyfikacji:
- `src/sniper/safety.rs` (dodaj testy na końcu pliku)

### Implementacja:

```rust
// src/sniper/safety.rs - DODAJ na końcu
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_safety_config() -> SafetyConfig {
        SafetyConfig {
            min_liquidity_sol: 5.0,
            max_market_cap_usd: 50_000.0,
            max_buy_tax_percent: 3.0,
            max_sell_tax_percent: 3.0,
            max_token_age_minutes: 30,
            min_holders: 20,
            max_dev_percentage: 25.0,
            blacklist_mints: vec!["BadMint11111111111111111111111111111111111".to_string()],
            blacklisted_creators: vec!["BadCreator1111111111111111111111111111111".to_string()],
            blacklist_keywords: vec!["scam".to_string(), "rug".to_string()],
            honeypot_api: None,  // Disabled for tests
            rugcheck_api: None,   // Disabled for tests
            enable_safety_checks: true,
        }
    }
    
    #[tokio::test]
    async fn test_config_loading() {
        let config = test_safety_config();
        let checker = SafetyChecker::from_config(&config);
        
        assert_eq!(checker.config.min_liquidity_sol, 5.0);
        assert_eq!(checker.config.max_buy_tax_percent, 3.0);
        assert_eq!(checker.blacklisted_keywords.len(), 2);
    }
    
    #[tokio::test]
    async fn test_liquidity_check() {
        let config = test_safety_config();
        let checker = SafetyChecker::from_config(&config);
        
        let token = NewToken {
            mint: Pubkey::new_unique(),
            creator: Pubkey::new_unique(),
            name: "TestToken".to_string(),
            symbol: "TEST".to_string(),
            initial_liquidity: 4_000_000_000, // 4 SOL - poniżej progu 5 SOL
            pool_address: Pubkey::new_unique(),
            timestamp: 0,
            market_cap_estimate: 10_000.0,
        };
        
        let result = checker.check_token(&token).await.unwrap();
        match result {
            SafetyResult::Unsafe(reason) => {
                assert!(reason.contains("Insufficient liquidity"));
            }
            _ => panic!("Expected Unsafe result for low liquidity"),
        }
    }
    
    #[tokio::test]
    async fn test_blacklist_mint() {
        let mut config = test_safety_config();
        let bad_mint = Pubkey::new_unique();
        config.blacklist_mints = vec![bad_mint.to_string()];
        
        let checker = SafetyChecker::from_config(&config);
        
        let token = NewToken {
            mint: bad_mint,
            creator: Pubkey::new_unique(),
            name: "GoodToken".to_string(),
            symbol: "GOOD".to_string(),
            initial_liquidity: 10_000_000_000,
            pool_address: Pubkey::new_unique(),
            timestamp: 0,
            market_cap_estimate: 10_000.0,
        };
        
        let result = checker.check_token(&token).await.unwrap();
        match result {
            SafetyResult::Unsafe(reason) => {
                assert_eq!(reason, "Blacklisted mint");
            }
            _ => panic!("Expected Unsafe result for blacklisted mint"),
        }
    }
    
    #[tokio::test]
    async fn test_keyword_detection() {
        let config = test_safety_config();
        let checker = SafetyChecker::from_config(&config);
        
        let token = NewToken {
            mint: Pubkey::new_unique(),
            creator: Pubkey::new_unique(),
            name: "ScamToken".to_string(), // Contains "scam"
            symbol: "SCAM".to_string(),
            initial_liquidity: 10_000_000_000,
            pool_address: Pubkey::new_unique(),
            timestamp: 0,
            market_cap_estimate: 10_000.0,
        };
        
        let result = checker.check_token(&token).await.unwrap();
        match result {
            SafetyResult::Unsafe(reason) => {
                assert_eq!(reason, "Suspicious name/symbol");
            }
            _ => panic!("Expected Unsafe result for suspicious keyword"),
        }
    }
    
    #[tokio::test]
    async fn test_safety_disabled() {
        let mut config = test_safety_config();
        config.enable_safety_checks = false; // Wyłącz safety
        
        let checker = SafetyChecker::from_config(&config);
        
        let token = NewToken {
            mint: Pubkey::new_unique(),
            creator: Pubkey::new_unique(),
            name: "ScamToken".to_string(), // Normalnie byłby odrzucony
            symbol: "SCAM".to_string(),
            initial_liquidity: 1_000_000, // Bardzo niska płynność
            pool_address: Pubkey::new_unique(),
            timestamp: 0,
            market_cap_estimate: 10_000.0,
        };
        
        let result = checker.check_token(&token).await.unwrap();
        match result {
            SafetyResult::Safe => {
                // Powinien przejść gdy safety wyłączone
            }
            _ => panic!("Expected Safe when safety checks disabled"),
        }
    }
}
```

### DoD:
- ✅ Test ładowania config do SafetyChecker
- ✅ Test sprawdzania minimalnej płynności
- ✅ Test blacklisty mintów
- ✅ Test detekcji podejrzanych słów kluczowych
- ✅ Test wyłączenia safety checks
- ✅ Wszystkie testy przechodzą: `cargo test --package solana-arbitrage-bot --lib sniper::safety`

---

## 📊 **Podsumowanie PR1**

### **Zakres zmian:**
1. ✅ Dodanie `SafetyConfig` struct z pełnymi polami konfiguracji
2. ✅ Rozszerzenie parsowania YAML w `sniper.rs`
3. ✅ Przekazanie config przez cały pipeline (binary → SniperEngine → SafetyChecker)
4. ✅ Refaktoryzacja SafetyChecker - usunięcie hardcoded wartości
5. ✅ Aktualizacja `config_sniper.yaml` z przykładowymi wartościami
6. ✅ Testy jednostkowe weryfikujące poprawność

### **Komenda testowa:**
```bash
# Uruchom testy
cargo test --package solana-arbitrage-bot --lib sniper::safety

# Sprawdź czy kompiluje się binary
cargo build --release --bin sniper

# Test z custom config
./target/release/sniper --config config_sniper.yaml --dry-run
```

### **Metryki sukcesu:**
- Zero hardcoded wartości w SafetyChecker
- 100% wartości konfigurowalnych przez YAML
- Testy jednostkowe pokrywające główne ścieżki
- Binary sniper poprawnie ładuje i stosuje config

### **Czas realizacji:** 
- Estymacja: 4-6 godzin
- Junior dev: 8-10 godzin
- Senior dev: 3-4 godziny

**Czy chcesz, żebym przygotował analogiczny breakdown dla PR2 (implementacja brakujących filtrów)?** 🚀