use anyhow::{anyhow, Result};
use log::{debug, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter dla API calls i transakcji
#[derive(Clone)]
pub struct RateLimiter {
    limits: Arc<Mutex<HashMap<String, RateLimit>>>,
    config: RateLimitConfig,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub api_calls_per_minute: u32,
    pub transactions_per_minute: u32,
    pub max_daily_transactions: u32,
    pub jupiter_calls_per_minute: u32,
    pub helius_calls_per_minute: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            api_calls_per_minute: 60,      // 1 per second
            transactions_per_minute: 10,   // Conservative for safety
            max_daily_transactions: 1000,  // Daily limit
            jupiter_calls_per_minute: 120, // Jupiter limit
            helius_calls_per_minute: 100,  // Helius limit
        }
    }
}

struct RateLimit {
    calls: Vec<Instant>,
    daily_count: u32,
    last_reset: Instant,
}

impl RateLimit {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            daily_count: 0,
            last_reset: Instant::now(),
        }
    }
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limits: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Sprawdza czy można wykonać API call
    pub async fn check_api_limit(&self, endpoint: &str) -> Result<()> {
        let limit = match endpoint {
            "jupiter" => self.config.jupiter_calls_per_minute,
            "helius" => self.config.helius_calls_per_minute,
            _ => self.config.api_calls_per_minute,
        };

        self.check_rate_limit(endpoint, limit, Duration::from_secs(60)).await
    }

    /// Sprawdza czy można wykonać transakcję
    pub async fn check_transaction_limit(&self) -> Result<()> {
        // Sprawdź minutowy limit
        self.check_rate_limit(
            "transactions", 
            self.config.transactions_per_minute, 
            Duration::from_secs(60)
        ).await?;

        // Sprawdź dzienny limit
        self.check_daily_limit().await
    }

    /// Główna logika rate limiting
    async fn check_rate_limit(&self, key: &str, max_calls: u32, window: Duration) -> Result<()> {
        let mut limits = self.limits.lock().await;
        let rate_limit = limits.entry(key.to_string()).or_insert_with(RateLimit::new);

        let now = Instant::now();
        
        // Usuń stare wpisy spoza okna czasowego
        rate_limit.calls.retain(|&call_time| now.duration_since(call_time) < window);

        // Sprawdź czy przekroczono limit
        if rate_limit.calls.len() >= max_calls as usize {
            let oldest_call = rate_limit.calls[0];
            let wait_time = window - now.duration_since(oldest_call);
            
            warn!("🚫 Rate limit exceeded for {}: {}/{} calls in {:?}. Wait {:?}", 
                  key, rate_limit.calls.len(), max_calls, window, wait_time);
            
            return Err(anyhow!("Rate limit exceeded for {}. Wait {:?}", key, wait_time));
        }

        // Dodaj nowy call
        rate_limit.calls.push(now);
        debug!("✅ Rate limit OK for {}: {}/{} calls", key, rate_limit.calls.len(), max_calls);

        Ok(())
    }

    /// Sprawdza dzienny limit transakcji
    async fn check_daily_limit(&self) -> Result<()> {
        let mut limits = self.limits.lock().await;
        let rate_limit = limits.entry("daily_transactions".to_string()).or_insert_with(RateLimit::new);

        let now = Instant::now();
        
        // Reset licznika jeśli minął dzień
        if now.duration_since(rate_limit.last_reset) >= Duration::from_secs(24 * 60 * 60) {
            rate_limit.daily_count = 0;
            rate_limit.last_reset = now;
            debug!("🔄 Daily transaction counter reset");
        }

        // Sprawdź dzienny limit
        if rate_limit.daily_count >= self.config.max_daily_transactions {
            warn!("🚫 Daily transaction limit exceeded: {}/{}", 
                  rate_limit.daily_count, self.config.max_daily_transactions);
            return Err(anyhow!("Daily transaction limit exceeded"));
        }

        rate_limit.daily_count += 1;
        debug!("✅ Daily transaction limit OK: {}/{}", 
               rate_limit.daily_count, self.config.max_daily_transactions);

        Ok(())
    }

    /// Wymusza opóźnienie dla bezpieczeństwa
    pub async fn enforce_delay(&self, operation: &str) -> Result<()> {
        let delay = match operation {
            "transaction" => Duration::from_millis(100), // Min 100ms między transakcjami
            "api_call" => Duration::from_millis(50),     // Min 50ms między API calls
            "jupiter_quote" => Duration::from_millis(25), // Szybkie quotes
            _ => Duration::from_millis(100),
        };

        debug!("⏱️ Enforcing {}ms delay for {}", delay.as_millis(), operation);
        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// Pobiera statystyki rate limiting
    pub async fn get_stats(&self) -> HashMap<String, RateLimitStats> {
        let limits = self.limits.lock().await;
        let mut stats = HashMap::new();

        for (key, limit) in limits.iter() {
            let now = Instant::now();
            let recent_calls = limit.calls.iter()
                .filter(|&&call_time| now.duration_since(call_time) < Duration::from_secs(60))
                .count();

            stats.insert(key.clone(), RateLimitStats {
                calls_last_minute: recent_calls as u32,
                daily_count: limit.daily_count,
                last_call: limit.calls.last().copied(),
            });
        }

        stats
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub calls_last_minute: u32,
    pub daily_count: u32,
    pub last_call: Option<Instant>,
}

/// Circuit breaker dla dodatkowej ochrony
pub struct CircuitBreaker {
    failure_count: u32,
    max_failures: u32,
    reset_timeout: Duration,
    last_failure: Option<Instant>,
    state: CircuitState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // Normalny stan
    Open,      // Zablokowany po błędach
    HalfOpen,  // Testowanie po timeout
}

impl CircuitBreaker {
    pub fn new(max_failures: u32, reset_timeout_secs: u64) -> Self {
        Self {
            failure_count: 0,
            max_failures,
            reset_timeout: Duration::from_secs(reset_timeout_secs),
            last_failure: None,
            state: CircuitState::Closed,
        }
    }

    /// Sprawdza czy operacja może być wykonana
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure {
                    if Instant::now().duration_since(last_failure) > self.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Rejestruje sukces operacji
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }

    /// Rejestruje błąd operacji
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.max_failures {
            self.state = CircuitState::Open;
            warn!("🔴 Circuit breaker OPEN after {} failures", self.failure_count);
        }
    }

    pub fn get_state(&self) -> CircuitState {
        self.state.clone()
    }
}
