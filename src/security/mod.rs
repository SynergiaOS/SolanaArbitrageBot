//! Security module for Solana Arbitrage Bot
//! 
//! Provides comprehensive security features including:
//! - Secure wallet management with encryption
//! - Rate limiting for API calls and transactions
//! - Flash loan security guards
//! - Security auditing and monitoring

pub mod wallet_manager;
pub mod rate_limiter;
pub mod flash_loan_guard;

pub use wallet_manager::{SecureWalletManager, security_audit};
pub use rate_limiter::{RateLimiter, RateLimitConfig, CircuitBreaker, CircuitState};
pub use flash_loan_guard::{FlashLoanGuard, FlashLoanConfig, FlashLoanResult};

use anyhow::Result;
use log::{info, warn};

/// Główny security manager dla całego bota
pub struct SecurityManager {
    wallet_manager: SecureWalletManager,
    rate_limiter: RateLimiter,
    flash_loan_guard: FlashLoanGuard,
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            wallet_manager: SecureWalletManager::new(),
            rate_limiter: RateLimiter::new(RateLimitConfig::default()),
            flash_loan_guard: FlashLoanGuard::new(FlashLoanConfig::default()),
        }
    }

    /// Wykonuje pełny audit bezpieczeństwa
    pub async fn full_security_audit(&self) -> Result<SecurityAuditReport> {
        info!("🔍 Starting comprehensive security audit...");

        let mut report = SecurityAuditReport::new();

        // Audit wallet security
        match security_audit() {
            Ok(_) => report.wallet_security = SecurityStatus::Secure,
            Err(e) => {
                report.wallet_security = SecurityStatus::Warning;
                report.issues.push(format!("Wallet security: {}", e));
            }
        }

        // Check rate limiting status
        let rate_stats = self.rate_limiter.get_stats().await;
        if rate_stats.values().any(|stat| stat.calls_last_minute > 50) {
            report.rate_limiting = SecurityStatus::Warning;
            report.issues.push("High API usage detected".to_string());
        } else {
            report.rate_limiting = SecurityStatus::Secure;
        }

        // Check active flash loans
        let active_loans = self.flash_loan_guard.get_active_loans_status();
        if active_loans.len() > 2 {
            report.flash_loan_security = SecurityStatus::Warning;
            report.issues.push(format!("Too many active flash loans: {}", active_loans.len()));
        } else {
            report.flash_loan_security = SecurityStatus::Secure;
        }

        // Overall security score
        report.overall_score = report.calculate_overall_score();

        info!("✅ Security audit completed. Score: {:.1}/10", report.overall_score);
        Ok(report)
    }

    pub fn get_wallet_manager(&self) -> &SecureWalletManager {
        &self.wallet_manager
    }

    pub fn get_rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }

    pub fn get_flash_loan_guard(&mut self) -> &mut FlashLoanGuard {
        &mut self.flash_loan_guard
    }
}

#[derive(Debug, Clone)]
pub struct SecurityAuditReport {
    pub wallet_security: SecurityStatus,
    pub rate_limiting: SecurityStatus,
    pub flash_loan_security: SecurityStatus,
    pub issues: Vec<String>,
    pub overall_score: f64,
}

impl SecurityAuditReport {
    fn new() -> Self {
        Self {
            wallet_security: SecurityStatus::Unknown,
            rate_limiting: SecurityStatus::Unknown,
            flash_loan_security: SecurityStatus::Unknown,
            issues: Vec::new(),
            overall_score: 0.0,
        }
    }

    fn calculate_overall_score(&self) -> f64 {
        let mut score = 10.0;

        // Deduct points for each security area
        score -= match self.wallet_security {
            SecurityStatus::Secure => 0.0,
            SecurityStatus::Warning => 2.0,
            SecurityStatus::Critical => 5.0,
            SecurityStatus::Unknown => 1.0,
        };

        score -= match self.rate_limiting {
            SecurityStatus::Secure => 0.0,
            SecurityStatus::Warning => 1.5,
            SecurityStatus::Critical => 3.0,
            SecurityStatus::Unknown => 1.0,
        };

        score -= match self.flash_loan_security {
            SecurityStatus::Secure => 0.0,
            SecurityStatus::Warning => 1.5,
            SecurityStatus::Critical => 3.0,
            SecurityStatus::Unknown => 1.0,
        };

        // Deduct for each issue
        score -= self.issues.len() as f64 * 0.5;

        score.max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityStatus {
    Secure,
    Warning,
    Critical,
    Unknown,
}

/// Security constants and limits
pub mod constants {
    use std::time::Duration;

    pub const MAX_WALLET_FILE_SIZE: u64 = 1024 * 1024; // 1MB
    pub const MIN_WALLET_PERMISSIONS: u32 = 0o600;
    pub const MAX_DAILY_TRANSACTIONS: u32 = 1000;
    pub const MAX_FLASH_LOAN_AMOUNT_SOL: f64 = 100.0;
    pub const FLASH_LOAN_TIMEOUT: Duration = Duration::from_secs(30);
    pub const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
}

/// Security utilities
pub mod utils {
    use super::*;
    use std::path::Path;

    /// Sprawdza czy plik ma bezpieczne uprawnienia
    pub fn check_file_permissions(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        
        if !path.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(path)?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        if mode & 0o077 != 0 {
            warn!("⚠️ File {} has insecure permissions: {:o}", path.display(), mode);
            return Err(anyhow::anyhow!("Insecure file permissions"));
        }

        Ok(())
    }

    /// Generuje bezpieczny identyfikator
    pub fn generate_secure_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        
        format!("sec_{}", timestamp)
    }

    /// Sprawdza czy string zawiera wrażliwe dane
    pub fn contains_sensitive_data(text: &str) -> bool {
        let sensitive_patterns = [
            "private_key",
            "secret_key", 
            "mnemonic",
            "seed_phrase",
            "wallet_password",
            "api_secret",
        ];

        let text_lower = text.to_lowercase();
        sensitive_patterns.iter().any(|pattern| text_lower.contains(pattern))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let security_manager = SecurityManager::new();
        assert!(security_manager.get_rate_limiter().get_stats().await.is_empty());
    }

    #[test]
    fn test_security_audit_report() {
        let mut report = SecurityAuditReport::new();
        report.wallet_security = SecurityStatus::Secure;
        report.rate_limiting = SecurityStatus::Secure;
        report.flash_loan_security = SecurityStatus::Secure;
        
        let score = report.calculate_overall_score();
        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_sensitive_data_detection() {
        assert!(utils::contains_sensitive_data("my_private_key_here"));
        assert!(utils::contains_sensitive_data("SECRET_KEY=abc123"));
        assert!(!utils::contains_sensitive_data("public_key_data"));
    }
}
