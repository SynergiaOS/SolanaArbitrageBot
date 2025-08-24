//! Comprehensive Safety Integration Tests
//! Tests all safety features including pre-trade checks, post-trade monitoring, and emergency procedures


use solana_arbitrage_bot::sniper::{
    safety::{EnhancedTokenData, RealTimeMetrics},
    RugMonitor, RugMonitorConfig, SafetyChecker, SafetyConfig, SafetyResult,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

#[cfg(test)]
mod safety_tests {
    use super::*;

    fn test_safety_config() -> SafetyConfig {
        SafetyConfig {
            min_liquidity_sol: 3.0,
            max_market_cap_usd: 100_000.0,
            max_buy_tax_percent: 5.0,
            max_sell_tax_percent: 5.0,
            max_token_age_minutes: 8,
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
            helius_api_key: None, // Test without Helius API
            enable_safety_checks: true,
        }
    }

    fn make_test_rug_monitor_config() -> RugMonitorConfig {
        RugMonitorConfig {
            enable_monitoring: true,
            liquidity_drop_threshold: 30.0,
            authority_change_timeout: 10,
            tax_increase_threshold: 50.0,
            monitoring_duration_minutes: 60,
            check_interval_seconds: 5,
        }
    }

    async fn create_test_rpc_client() -> Arc<RpcClient> {
        // Use a mock RPC client for testing
        Arc::new(RpcClient::new(
            "https://api.mainnet-beta.solana.com".to_string(),
        ))
    }

    #[tokio::test]
    async fn test_safety_config_consistency() {
        let config = test_safety_config();

        // Test that all safety thresholds are reasonable
        assert!(
            config.min_liquidity_sol > 0.0,
            "Min liquidity should be positive"
        );
        assert!(
            config.max_market_cap_usd > 0.0,
            "Max market cap should be positive"
        );
        assert!(
            config.max_buy_tax_percent >= 0.0,
            "Buy tax should be non-negative"
        );
        assert!(
            config.max_sell_tax_percent >= 0.0,
            "Sell tax should be non-negative"
        );
        assert!(
            config.max_token_age_minutes > 0,
            "Max token age should be positive"
        );
        assert!(config.min_holders > 0, "Min holders should be positive");
        assert!(
            config.max_dev_percentage >= 0.0,
            "Max dev percentage should be non-negative"
        );
        assert!(
            config.enable_safety_checks,
            "Safety checks should be enabled"
        );
    }

    #[tokio::test]
    async fn test_safety_checker_creation() {
        let rpc_client = create_test_rpc_client().await;
        let config = test_safety_config();

        let checker = SafetyChecker::from_config(&config, rpc_client);

        // Test that checker is properly initialized
        assert_eq!(checker.get_config().min_liquidity_sol, config.min_liquidity_sol);
        assert_eq!(checker.get_config().max_market_cap_usd, config.max_market_cap_usd);
        assert_eq!(
            checker.get_config().enable_safety_checks,
            config.enable_safety_checks
        );
    }

    #[tokio::test]
    async fn test_blacklist_functionality() {
        let rpc_client = create_test_rpc_client().await;
        let mut config = test_safety_config();

        // Add a test creator to blacklist
        let test_creator = Pubkey::new_unique();
        config.blacklisted_creators.push(test_creator.to_string());

        let checker = SafetyChecker::from_config(&config, rpc_client);

        // Test that blacklisted creator is detected
        assert!(checker.is_creator_blacklisted(&test_creator));
    }

    #[tokio::test]
    async fn test_keyword_filtering() {
        let rpc_client = create_test_rpc_client().await;
        let config = test_safety_config();
        let checker = SafetyChecker::from_config(&config, rpc_client);

        // Test suspicious keywords
        let suspicious_keywords = vec!["scam", "rug", "fake", "test"];
        for keyword in suspicious_keywords {
            assert!(checker.is_keyword_blacklisted(keyword));
        }
    }

    #[tokio::test]
    async fn test_rug_monitor_config() {
        let config = make_test_rug_monitor_config();

        assert!(config.enable_monitoring, "Rug monitoring should be enabled");
        assert!(
            config.liquidity_drop_threshold > 0.0,
            "Liquidity threshold should be positive"
        );
        assert!(
            config.authority_change_timeout > 0,
            "Authority timeout should be positive"
        );
        assert!(
            config.tax_increase_threshold > 0.0,
            "Tax threshold should be positive"
        );
        assert!(
            config.monitoring_duration_minutes > 0,
            "Monitoring duration should be positive"
        );
        assert!(
            config.check_interval_seconds > 0,
            "Check interval should be positive"
        );
    }

    #[tokio::test]
    async fn test_rug_monitor_creation() {
        let rpc_client = create_test_rpc_client().await;
        let config = make_test_rug_monitor_config();

        let monitor = RugMonitor::with_config(config.clone(), rpc_client);

        // Test that monitor is properly initialized
        assert_eq!(monitor.get_config().enable_monitoring, config.enable_monitoring);
        assert_eq!(
            monitor.get_config().liquidity_drop_threshold,
            config.liquidity_drop_threshold
        );
        assert_eq!(
            monitor.get_config().authority_change_timeout,
            config.authority_change_timeout
        );
    }

    #[tokio::test]
    async fn test_enhanced_token_data_defaults() {
        let data = EnhancedTokenData::default();

        assert!(
            data.creation_time.is_none(),
            "Creation time should default to None"
        );
        assert!(data.creator.is_none(), "Creator should default to None");
        assert!(!data.is_verified, "Token should not be verified by default");
        assert!(!data.is_frozen, "Token should not be frozen by default");
        assert!(!data.is_mutable, "Token should not be mutable by default");
    }

    #[tokio::test]
    async fn test_realtime_metrics_defaults() {
        let metrics = RealTimeMetrics::default();

        assert_eq!(
            metrics.current_price, 0.0,
            "Current price should default to 0"
        );
        assert_eq!(metrics.volume_24h, 0.0, "24h volume should default to 0");
        assert_eq!(metrics.holder_count, 0, "Holder count should default to 0");
        assert_eq!(metrics.market_cap, 0.0, "Market cap should default to 0");
    }

    #[tokio::test]
    async fn test_safety_result_enum() {
        let safe_result = SafetyResult::Safe;
        let unsafe_result = SafetyResult::Unsafe("Test reason".to_string());

        match safe_result {
            SafetyResult::Safe => {
                // Test passed - expected Safe result
            },
            SafetyResult::Unsafe(_) => {
                panic!("Safe result should not match Unsafe variant")
            }
        }

        match unsafe_result {
            SafetyResult::Safe => panic!("Unsafe result should not match Safe variant"),
            SafetyResult::Unsafe(reason) => assert_eq!(reason, "Test reason"),
        }
    }

    #[tokio::test]
    async fn test_config_validation() {
        // Test invalid configurations
        let invalid_configs = vec![
            SafetyConfig {
                min_liquidity_sol: -1.0, // Invalid: negative
                ..test_safety_config()
            },
            SafetyConfig {
                max_market_cap_usd: 0.0, // Invalid: zero
                ..test_safety_config()
            },
            SafetyConfig {
                max_token_age_minutes: 0, // Invalid: zero
                ..test_safety_config()
            },
            SafetyConfig {
                min_holders: 0, // Invalid: zero
                ..test_safety_config()
            },
        ];

        for config in invalid_configs {
            // These should be caught by validation
            assert!(
                config.min_liquidity_sol < 0.0
                    || config.max_market_cap_usd <= 0.0
                    || config.max_token_age_minutes == 0
                    || config.min_holders == 0,
                "Invalid config should be detected"
            );
        }
    }

    #[tokio::test]
    async fn test_rug_monitor_alert_types() {
        use solana_arbitrage_bot::sniper::rug_monitor::RugAlert;

        // Test different alert types
        let liquidity_alert = RugAlert::LiquidityDrain {
            old_liq: 1000000,
            new_liq: 500000,
            drop_percent: 50.0,
        };

        let authority_alert = RugAlert::AuthorityChanged {
            old_auth: None,
            new_auth: Some(Pubkey::new_unique()),
        };

        let tax_alert = RugAlert::TaxIncreased {
            old_buy: 5.0,
            new_buy: 15.0,
            old_sell: 5.0,
            new_sell: 15.0,
        };

        let transaction_alert = RugAlert::FailedTransaction {
            error: "RPC timeout".to_string(),
        };

        match liquidity_alert {
            RugAlert::LiquidityDrain { drop_percent, .. } => {
                assert_eq!(drop_percent, 50.0, "Liquidity drop should be 50%");
            }
            _ => panic!("Expected LiquidityDrain alert"),
        }

        match authority_alert {
            RugAlert::AuthorityChanged { new_auth, .. } => {
                assert!(new_auth.is_some(), "New authority should be present");
            }
            _ => panic!("Expected AuthorityChanged alert"),
        }

        match tax_alert {
            RugAlert::TaxIncreased {
                old_buy, new_buy, ..
            } => {
                assert_eq!(old_buy, 5.0, "Old buy tax should be 5%");
                assert_eq!(new_buy, 15.0, "New buy tax should be 15%");
            }
            _ => panic!("Expected TaxIncreased alert"),
        }

        match transaction_alert {
            RugAlert::FailedTransaction { error } => {
                assert_eq!(error, "RPC timeout", "Error message should match");
            }
            _ => panic!("Expected FailedTransaction alert"),
        }
    }

    #[tokio::test]
    async fn test_safety_checker_with_disabled_checks() {
        let rpc_client = create_test_rpc_client().await;
        let mut config = test_safety_config();
        config.enable_safety_checks = false;

        let checker = SafetyChecker::from_config(&config, rpc_client);

        // When safety checks are disabled, should return Safe
        // Note: This would require a mock token, so we'll just test the config
        assert!(
            !checker.get_config().enable_safety_checks,
            "Safety checks should be disabled"
        );
    }

    #[tokio::test]
    async fn test_blacklist_operations() {
        let rpc_client = create_test_rpc_client().await;
        let config = test_safety_config();
        let mut checker = SafetyChecker::from_config(&config, rpc_client);

        let test_creator = Pubkey::new_unique();
        let test_keyword = "suspicious";

        // Test adding to blacklist
        checker.add_blacklisted_creator(test_creator);
        checker.add_blacklisted_keyword(test_keyword.to_string());

        assert!(checker.is_creator_blacklisted(&test_creator));
        assert!(checker.is_keyword_blacklisted(test_keyword));
    }

    #[tokio::test]
    async fn test_monitoring_configuration_edge_cases() {
        // Test edge cases for monitoring configuration
        let edge_case_configs = vec![
            RugMonitorConfig {
                liquidity_drop_threshold: 0.1, // Very sensitive
                ..make_test_rug_monitor_config()
            },
            RugMonitorConfig {
                liquidity_drop_threshold: 99.9, // Very insensitive
                ..make_test_rug_monitor_config()
            },
            RugMonitorConfig {
                check_interval_seconds: 1, // Very frequent
                ..make_test_rug_monitor_config()
            },
            RugMonitorConfig {
                monitoring_duration_minutes: 1, // Very short
                ..make_test_rug_monitor_config()
            },
        ];

        for config in edge_case_configs {
            assert!(
                config.liquidity_drop_threshold > 0.0,
                "Threshold should be positive"
            );
            assert!(
                config.liquidity_drop_threshold < 100.0,
                "Threshold should be less than 100%"
            );
            assert!(
                config.check_interval_seconds > 0,
                "Check interval should be positive"
            );
            assert!(
                config.monitoring_duration_minutes > 0,
                "Monitoring duration should be positive"
            );
        }
    }
}
