# 🚀 Szczegółowy Plan Następnych Kroków - SolanaArbitrageBot

## 📊 Status Obecny

✅ **PR1: Config Safety Integration - UKOŃCZONY**
- SafetyConfig struct zaimplementowana
- YAML parsing rozszerzony
- SafetyChecker refaktoryzowany
- Testy jednostkowe napisane
- Konfiguracja safety w pełni zintegrowana

❌ **Problem**: Błędy kompilacji związane z rust_decimal feature flags

## 🛤️ Dwie Ścieżki Rozwoju

### ŚCIEŻKA A: Naprawa Problemów Kompilacji [PRIORYTET 1]
**Cel**: Umożliwić kompilację i testowanie bota

### ŚCIEŻKA B: PR2 - Missing Safety Filters [PRIORYTET 2]  
**Cel**: Implementacja brakujących filtrów bezpieczeństwa

---

## 🔧 ŚCIEŻKA A: Naprawa Problemów Kompilacji

### A1: Analiza Problemów Kompilacji
**Czas**: 30 min  
**Opis**: Szczegółowa analiza wszystkich błędów kompilacji

**Zadania**:
- [ ] Katalogowanie błędów rust_decimal (serde-with-float)
- [ ] Identyfikacja problemów z typami Decimal vs f64
- [ ] Analiza błędów OptionSerializer
- [ ] Sprawdzenie wersji dependencies w Cargo.toml
- [ ] Dokumentacja wszystkich wymaganych zmian

**Pliki do sprawdzenia**:
- `Cargo.toml` - feature flags
- `src/lib.rs` - serde annotations
- `src/monitor.rs` - type mismatches
- `src/safety.rs` - Decimal conversions
- `src/executor.rs` - OptionSerializer issues

### A2: Naprawa Cargo.toml - Feature Flags
**Czas**: 15 min  
**Opis**: Dodanie brakujących feature flags

**Zadania**:
- [ ] Dodać `serde-with-float` do rust_decimal features
- [ ] Sprawdzić kompatybilność wersji dependencies
- [ ] Dodać missing features dla solana crates
- [ ] Test kompilacji po zmianach

**Przykład zmiany**:
```toml
[dependencies]
rust_decimal = { version = "1.37", features = ["serde-with-float"] }
```

### A3: Konwersje Typów Decimal ↔ f64
**Czas**: 45 min  
**Opis**: Implementacja konwersji między typami

**Zadania**:
- [ ] Utworzenie helper functions dla konwersji
- [ ] Naprawa `src/monitor.rs` - price types
- [ ] Naprawa `src/safety.rs` - daily loss calculations
- [ ] Naprawa `src/calculator.rs` - profit calculations
- [ ] Naprawa `src/executor.rs` - slippage calculations

**Helper functions**:
```rust
// src/utils/conversions.rs
pub fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}

pub fn f64_to_decimal(f: f64) -> Decimal {
    Decimal::from_f64(f).unwrap_or(Decimal::ZERO)
}
```

### A4: Naprawa OptionSerializer
**Czas**: 20 min  
**Opis**: Rozwiązanie problemów z solana-transaction-status

**Zadania**:
- [ ] Zamiana `unwrap_or_default()` na `unwrap_or(vec![])`
- [ ] Sprawdzenie dokumentacji solana-transaction-status
- [ ] Test transaction parsing

**Przykład naprawy**:
```rust
// Przed:
let pre_balances = meta.pre_token_balances.unwrap_or_default();

// Po:
let pre_balances = meta.pre_token_balances.unwrap_or(vec![]);
```

### A5: Test Kompilacji i Uruchomienia
**Czas**: 30 min  
**Opis**: Weryfikacja poprawności napraw

**Zadania**:
- [ ] `cargo check` - sprawdzenie kompilacji
- [ ] `cargo test` - uruchomienie testów
- [ ] `cargo run --bin sniper --help` - test CLI
- [ ] Test ładowania config_sniper.yaml
- [ ] Weryfikacja logów startowych

---

## 🛡️ ŚCIEŻKA B: PR2 - Missing Safety Filters

### B1: Market Cap Calculation
**Czas**: 60 min  
**Opis**: Implementacja wyliczania market cap

**Zadania**:
- [ ] Dodanie metody `calculate_market_cap()` do SafetyChecker
- [ ] Pobranie total supply z mint account
- [ ] Wyliczenie ceny z pool reserves
- [ ] Implementacja `MC = supply × price`
- [ ] Dodanie sprawdzenia `max_market_cap_usd` w check_token()

**Implementacja**:
```rust
// src/sniper/safety.rs
async fn calculate_market_cap(&self, token: &NewToken) -> Result<f64> {
    // 1. Pobierz mint account dla total supply
    // 2. Pobierz pool reserves dla ceny
    // 3. Wylicz MC = supply * price
}
```

### B2: Holder Count Analysis
**Czas**: 90 min  
**Opis**: Integracja z Helius Enhanced API

**Zadania**:
- [ ] Implementacja Helius Enhanced API client
- [ ] Metoda `get_token_holders()` 
- [ ] Parsing response i liczenie unique holders
- [ ] Fallback na RPC `getProgramAccounts` jeśli Helius niedostępny
- [ ] Dodanie sprawdzenia `min_holders` w check_token()

**API Endpoint**:
```
GET https://api.helius.xyz/v0/tokens/{mint}/holders?api-key={key}
```

### B3: Dev Percentage Check
**Czas**: 75 min  
**Opis**: Analiza koncentracji u top holders

**Zadania**:
- [ ] Rozszerzenie holder analysis o balances
- [ ] Identyfikacja top 10 holders
- [ ] Wyliczenie % największego holdera
- [ ] Heurystyki wykrywania dev wallets (creation time, patterns)
- [ ] Dodanie sprawdzenia `max_dev_percentage` w check_token()

**Logika**:
```rust
// Jeśli top holder ma > max_dev_percentage% supply → UNSAFE
// Uwzględnić locked/burned tokens
```

### B4: Token Age Verification
**Czas**: 45 min  
**Opis**: Wyznaczanie wieku tokena

**Zadania**:
- [ ] Implementacja `get_token_age()` 
- [ ] Pobranie mint account creation slot/timestamp
- [ ] Alternatywnie: pierwszy pool creation timestamp
- [ ] Konwersja na minuty od utworzenia
- [ ] Dodanie sprawdzenia `max_token_age_minutes` w check_token()

**Metody**:
```rust
// Opcja 1: Mint account creation (via getAccountInfo + slot)
// Opcja 2: Pool creation timestamp (z logs)
// Opcja 3: Pierwszy trade timestamp (z Helius)
```

### B5: Integracja z SafetyChecker
**Czas**: 30 min  
**Opis**: Podpięcie nowych filtrów

**Zadania**:
- [ ] Dodanie nowych checks do `check_token()`
- [ ] Respektowanie progów z SafetyConfig
- [ ] Logowanie powodów odrzucenia
- [ ] Optymalizacja kolejności checks (szybkie najpierw)
- [ ] Error handling dla API failures

**Kolejność checks**:
1. Blacklists (szybkie, lokalne)
2. Liquidity (szybkie, z NewToken)
3. Token age (średnie, RPC call)
4. Market cap (średnie, RPC calls)
5. Holder count (wolne, API call)
6. Dev percentage (wolne, API call)
7. Honeypot/taxes (wolne, external APIs)

### B6: Testy Nowych Filtrów
**Czas**: 60 min  
**Opis**: Unit testy dla wszystkich nowych funkcji

**Zadania**:
- [ ] Test market cap calculation
- [ ] Test holder count (mock Helius API)
- [ ] Test dev percentage detection
- [ ] Test token age calculation
- [ ] Test integration w check_token()
- [ ] Test error handling i fallbacks

---

## 🎯 Rekomendowana Kolejność Wykonania

### Faza 1: Stabilizacja (1-2 godziny)
1. **A1**: Analiza problemów kompilacji
2. **A2**: Naprawa Cargo.toml
3. **A3**: Konwersje typów
4. **A4**: Naprawa OptionSerializer
5. **A5**: Test kompilacji

### Faza 2: Rozszerzenie Funkcjonalności (3-4 godziny)
1. **B4**: Token age (najprostsze)
2. **B1**: Market cap calculation
3. **B2**: Holder count analysis
4. **B3**: Dev percentage check
5. **B5**: Integracja z SafetyChecker
6. **B6**: Testy nowych filtrów

## 🔍 Kryteria Sukcesu

### Po Fazie 1:
- [ ] `cargo check` przechodzi bez błędów
- [ ] `cargo test` uruchamia testy SafetyChecker
- [ ] Bot startuje i ładuje config_sniper.yaml
- [ ] Podstawowe safety checks działają

### Po Fazie 2:
- [ ] Wszystkie pola SafetyConfig są egzekwowane
- [ ] Market cap, holders, dev %, token age sprawdzane
- [ ] Testy pokrywają nowe funkcje
- [ ] Bot odrzuca tokeny według wszystkich kryteriów

## 🚨 Potencjalne Problemy

1. **Helius API Rate Limits**: Implementować cache i exponential backoff
2. **RPC Timeouts**: Fallback strategies dla każdego API call
3. **False Positives**: Tunowanie progów na podstawie testów
4. **Performance**: Optymalizacja kolejności i równoległości checks

## 📈 Następne PR po B6

- **PR3**: Post-trade rug monitoring (LP drain, authority changes)
- **PR4**: Enhanced Helius integration (webhooks, WS)
- **PR5**: Performance optimizations i telemetria
