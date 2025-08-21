# Analiza Bezpieczeństwa i Lista Zadań - SolanaArbitrageBot

## Stan Obecny (Analiza Kodu)

### 1. SafetyGuard (src/safety.rs)
- **Inicjalizacja**: Używa `config.limits.*` (nie pasuje do struktury config.yaml)
- **Hard-coded**: `min_pool_liquidity_usd = 50_000 USD`
- **Użycie**: Ogólne limity pozycji i daily loss

### 2. SafetyChecker (src/sniper/safety.rs)
- **Min. płynność**: 5 SOL (hard-coded) vs 3.0 SOL w config.yaml
- **Podatki**: >10% (hard-coded) vs 5% w config.yaml
- **API endpointy**: Stałe URL honeypot.is i rugcheck.xyz
- **Blacklisty**: Mechanizm istnieje, ale brak wczytywania z config

### 3. TP/SL/Timeout (src/sniper/position.rs)
- **TP/SL**: Używa SniperConfig (OK)
- **Timeout**: 60 min hard-coded vs config.risk.position_timeout_minutes

### 4. Ładowanie Konfiguracji (src/bin/sniper.rs)
- **Problem**: Czyta tylko sekcję `sniper`, ignoruje `safety`
- **Brakuje**: Mapowanie pól safety do SafetyChecker

### 5. Monitoring Real-time
- **Obecny**: Tylko pre-buy via Raydium logsSubscribe
- **Brakuje**: Post-trade rug pull detection

## Kluczowe Rozbieżności

| Parametr | Config.yaml | Kod | Status |
|----------|-------------|-----|--------|
| Min. płynność | 3.0 SOL | 5 SOL | ❌ Rozbieżność |
| Podatki | 5% | 10% | ❌ Rozbieżność |
| API endpointy | Zdefiniowane | Hard-coded | ❌ Nieużywane |
| Market cap | Zdefiniowany | Brak | ❌ Nieegzekwowany |
| Min holders | Zdefiniowany | Brak | ❌ Nieegzekwowany |
| Dev percentage | Zdefiniowany | Brak | ❌ Nieegzekwowany |
| Token age | Zdefiniowany | Brak | ❌ Nieegzekwowany |

## Duża Lista Zadań

### A. Spójność Konfiguracji [WYSOKI PRIORYTET]

#### A1. Wczytywanie sekcji safety
- [ ] Dodać `SafetySection` do `RootYamlConfig` w `src/bin/sniper.rs`
- [ ] Mapować pola: `min_liquidity_sol`, `max_buy_tax_percent`, `max_sell_tax_percent`
- [ ] Mapować: `max_market_cap_usd`, `max_dev_percentage`, `min_holders`, `max_token_age_minutes`
- [ ] Mapować: `blacklist_mints`, `blacklisted_creators`, `honeypot_api`, `rugcheck_api`

#### A2. Ujednolicenie progów
- [ ] SafetyChecker używa `config.min_liquidity_sol` zamiast hard-coded 5 SOL
- [ ] SafetyChecker używa `config.max_buy_tax_percent/max_sell_tax_percent` zamiast 10%
- [ ] PositionManager używa `config.position_timeout_minutes` zamiast 60

#### A3. Endpointy API z config
- [ ] `check_honeypot_api()` używa `config.honeypot_api`
- [ ] `check_token_taxes()` używa `config.rugcheck_api`
- [ ] Fallback na domyślne URL gdy brak w config

#### A4. Blacklisty z config
- [ ] Inicjalizacja `blacklisted_creators` z config przy tworzeniu SafetyChecker
- [ ] Inicjalizacja `blacklist_mints` z config
- [ ] Ładowanie `blacklist_keywords` z config (+ domyślne)

### B. Rozszerzenie Filtrów SafetyChecker [WYSOKI PRIORYTET]

#### B1. Market Cap limit
- [ ] Implementacja wyliczania MC: `circulating_supply × price`
- [ ] Odrzucanie tokenów z MC > `safety.max_market_cap_usd`
- [ ] Logowanie powodu odrzucenia

#### B2. Minimalna liczba holderów
- [ ] Integracja z Helius Enhanced API lub indekserami
- [ ] Odrzucanie jeśli < `safety.min_holders`
- [ ] Fallback gdy brak danych

#### B3. Max dev percentage
- [ ] Analiza top holders (via Helius/indekser)
- [ ] Odrzucanie jeśli największy holder > `safety.max_dev_percentage`
- [ ] Handling "unknown" przypadków

#### B4. Maksymalny wiek tokena
- [ ] Wyznaczanie wieku (czas od inicjalizacji mint/poola)
- [ ] Porównanie do `safety.max_token_age_minutes`
- [ ] Implementacja heurystyk dla wieku

#### B5. Egzekwowanie wszystkich filtrów
- [ ] Blacklist mints/creators w `check_token()`
- [ ] Próg płynności z config
- [ ] Podatki z config

### C. Monitoring Real-time Po Zakupie [WYSOKI PRIORYTET]

#### C1. LP drain detection
- [ ] `accountSubscribe` na kontach rezerw poola (SOL/USDC vaults)
- [ ] Konfigurowalny próg spadku X% w Y sekund
- [ ] Natychmiastowy SELL przy wykryciu

#### C2. Authority changes detection
- [ ] `accountSubscribe` na mint
- [ ] Śledzenie freeze/mint authority
- [ ] SELL przy niebezpiecznych zmianach

#### C3. Dynamic taxes re-check
- [ ] Okresowe odpytywanie RugCheck (co 60-120s)
- [ ] SELL jeśli buy/sell tax > próg
- [ ] Rate limiting dla API

#### C4. Failed swaps/trading halt
- [ ] Zliczanie błędów swapów
- [ ] Monitoring skok slippage/price impact
- [ ] SELL lub wstrzymanie przy anomaliach

#### C5. Emergency SELL mechanism
- [ ] Implementacja `emergency_sell()` w PositionManager
- [ ] Discord alerty z powodem (LP drain, authority change, tax spike)
- [ ] Logowanie wszystkich emergency actions

### D. Integracja Helius [ŚREDNI PRIORYTET]

#### D1. Konfiguracja połączeń
- [ ] Dodać `helius_ws_url` z api-key do config
- [ ] Fallback na public RPC przy błędach
- [ ] Konfiguracja backup_url

#### D2. Webhooki
- [ ] Moduł rejestracji webhooków (LOGS/TRANSACTIONS)
- [ ] Prosty serwer odbierający zdarzenia
- [ ] Monitoring własnego walleta i obserwowanych mintów

#### D3. Enhanced Transactions API
- [ ] Endpoints do inspekcji ostatnich transakcji walleta
- [ ] Narzędzia debug (off-trade)
- [ ] Parsowanie dekodowanych instrukcji

#### D4. Rate limiting/Retry
- [ ] Exponential backoff dla Helius
- [ ] Limity zapytań do RugCheck/Honeypot
- [ ] Circuit breaker pattern

### E. Refaktoryzacja i Testy [ŚREDNI PRIORYTET]

#### E1. Struktury config
- [ ] Wprowadzić `SafetyConfig` i `RiskConfig`
- [ ] Serializacja z YAML
- [ ] Walidacja pól config

#### E2. Testy jednostkowe SafetyChecker
- [ ] Testy: liquidity, taxes, blacklist
- [ ] Testy: market cap, holders, token age, dev %
- [ ] Mock API responses

#### E3. Testy integracyjne
- [ ] Symulacja nowego tokena + SafetyChecker
- [ ] Weryfikacja odrzuceń i akceptacji
- [ ] End-to-end flow tests

#### E4. Testy monitoringu post-trade
- [ ] Mock account notifications
- [ ] Test LP reserve spadek → SELL
- [ ] Test zmiana authority → SELL

#### E5. Testy ładowania config
- [ ] Assercje poprawnego wczytywania
- [ ] Testy nadpisywania przez CLI
- [ ] Walidacja błędnych konfiguracji

### F. Stabilność i Obserwowalność [ŚREDNI PRIORYTET]

#### F1. Telemetria
- [ ] Metryki Prometheus: odrzucenia per powód
- [ ] Metryki: emergency sells, czas reakcji
- [ ] Dashboard Grafana

#### F2. Logging
- [ ] Ujednolicenie poziomów logowania
- [ ] Dodanie kontekstu (mint, pool, progi)
- [ ] Structured logging (JSON)

#### F3. Feature flags
- [ ] Włączniki dla poszczególnych checks
- [ ] `enable_post_trade_rug_monitoring`
- [ ] Runtime configuration changes

### G. Wydajność i Niezawodność [NIŻSZY PRIORYTET]

#### G1. Cache wyników
- [ ] Czasowe cache RugCheck/Honeypot (5-10 min per mint)
- [ ] LRU cache implementation
- [ ] Cache invalidation strategies

#### G2. Batch queries
- [ ] Batch odpytywanie wielu mintów
- [ ] Rate limiting dla batch operations
- [ ] Parallel processing optimization

#### G3. Failover RPC
- [ ] Automatyczny przełącznik Helius → public RPC
- [ ] Health checks dla RPC endpoints
- [ ] Load balancing

### H. Dokumentacja i Operacje [NIŻSZY PRIORYTET]

#### H1. Dokumentacja konfiguracji
- [ ] Opis pól safety w README
- [ ] Przykłady konfiguracji
- [ ] Best practices guide

#### H2. Runbook
- [ ] Diagnozowanie alertów
- [ ] Wyłączanie poszczególnych checks
- [ ] Analiza Helius logs

#### H3. Security
- [ ] Przegląd przechowywania API keys
- [ ] Maskowanie w logach
- [ ] Security audit checklist

## Proponowany Podział na PR-y

### PR1: Config Safety Integration
**Pliki**: `src/bin/sniper.rs`, `src/sniper/safety.rs`, `src/sniper/position.rs`
- Wczytywanie config.safety
- Podpięcie progów do SafetyChecker
- Endpointy API z config
- Blacklisty z config

### PR2: Missing Safety Filters
**Pliki**: `src/sniper/safety.rs`, nowe moduły dla MC/holders
- Market cap calculation
- Holder count analysis
- Dev percentage check
- Token age verification

### PR3: Post-Trade Rug Monitoring
**Pliki**: `src/sniper/position.rs`, nowy moduł `rug_monitor.rs`
- LP drain detection
- Authority changes monitoring
- Emergency SELL mechanism
- Discord alerty

### PR4: Enhanced Helius Integration
**Pliki**: `src/sniper/monitor.rs`, nowy moduł `helius.rs`
- WS URL configuration
- Webhooks implementation
- Enhanced API utils
- Rate limiting

### PR5: Testing & Observability
**Pliki**: `tests/`, `src/telemetry.rs`, dokumentacja
- Unit tests
- Integration tests
- Telemetria
- Dokumentacja

## Kryteria Akceptacji (DoD)

### Dla każdego PR:
- [ ] Wszystkie testy przechodzą
- [ ] Code coverage > 80%
- [ ] Dokumentacja zaktualizowana
- [ ] Backward compatibility zachowana
- [ ] Performance impact zmierzony
- [ ] Security review przeprowadzony

### Dla całego projektu:
- [ ] Config.yaml w pełni respektowany
- [ ] Wszystkie safety checks działają
- [ ] Post-trade monitoring aktywny
- [ ] Helius w pełni wykorzystany
- [ ] Monitoring i alerty działają
- [ ] Dokumentacja kompletna
