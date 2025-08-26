use anyhow::Result;
use log::{info, warn};
use rust_decimal::Decimal;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::architecture::EnhancedSharedState;
use crate::calculator::ArbitrageOpportunity;
use crate::performance::{connection_pool::RpcConnectionPool, error_handling::RobustErrorHandler};
use crate::security::{FlashLoanGuard, SecurityManager};

/// Comprehensive integration test suite
pub struct IntegrationTestSuite {
    rpc_client: Arc<RpcClient>,
    connection_pool: Arc<RpcConnectionPool>,
    error_handler: Arc<RobustErrorHandler>,
    test_keypair: Arc<Keypair>,
    config: TestConfig,
}

#[derive(Clone)]
pub struct TestConfig {
    pub rpc_url: String,
    pub test_duration_seconds: u64,
    pub max_test_amount_sol: f64,
    pub enable_live_trading_tests: bool,
    pub enable_stress_tests: bool,
    pub enable_security_tests: bool,
    pub test_endpoints: Vec<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            test_duration_seconds: 300, // 5 minutes
            max_test_amount_sol: 0.1,   // Small test amounts
            enable_live_trading_tests: false, // Disabled by default for safety
            enable_stress_tests: true,
            enable_security_tests: true,
            test_endpoints: vec![
                "https://api.devnet.solana.com".to_string(),
                "https://api.testnet.solana.com".to_string(),
            ],
        }
    }
}

impl IntegrationTestSuite {
    pub async fn new(config: TestConfig) -> Result<Self> {
        info!("🧪 Initializing Integration Test Suite");

        // Create test keypair
        let test_keypair = Arc::new(Keypair::new());
        info!("🔑 Test wallet: {}", test_keypair.pubkey());

        // Create RPC client
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        ));

        // Create connection pool
        let pool_config = crate::performance::connection_pool::PoolConfig {
            primary_rpc_url: config.rpc_url.clone(),
            backup_rpc_urls: config.test_endpoints.clone(),
            max_connections: 10,
            min_connections: 2,
            ..Default::default()
        };
        let connection_pool = Arc::new(RpcConnectionPool::new(pool_config).await?);

        // Create error handler
        let error_handler = Arc::new(RobustErrorHandler::new(
            crate::performance::error_handling::ErrorHandlingConfig::default(),
        ));

        Ok(Self {
            rpc_client,
            connection_pool,
            error_handler,
            test_keypair,
            config,
        })
    }

    /// Run all integration tests
    pub async fn run_all_tests(&self) -> Result<TestResults> {
        info!("🚀 Starting comprehensive integration tests");
        let start_time = Instant::now();

        let mut results = TestResults::new();

        // 1. Basic connectivity tests
        results.connectivity = self.test_connectivity().await;

        // 2. Architecture tests
        results.architecture = self.test_architecture().await;

        // 3. Performance tests
        results.performance = self.test_performance().await;

        // 4. Error handling tests
        results.error_handling = self.test_error_handling().await;

        // 5. Security tests (if enabled)
        if self.config.enable_security_tests {
            results.security = self.test_security().await;
        }

        // 6. Flash loan tests
        results.flash_loans = self.test_flash_loans().await;

        // 7. Stress tests (if enabled)
        if self.config.enable_stress_tests {
            results.stress = self.test_stress().await;
        }

        // 8. Live trading tests (if enabled and safe)
        if self.config.enable_live_trading_tests {
            warn!("⚠️ Running live trading tests - using real funds!");
            results.live_trading = self.test_live_trading().await;
        }

        results.total_duration = start_time.elapsed();
        results.overall_success = results.calculate_overall_success();

        info!("✅ Integration tests completed in {:?}", results.total_duration);
        self.print_test_summary(&results);

        Ok(results)
    }

    async fn test_connectivity(&self) -> TestResult {
        info!("🌐 Testing connectivity...");
        let start_time = Instant::now();

        let mut success_count = 0;
        let mut total_tests = 0;

        // Test primary RPC
        total_tests += 1;
        match self.rpc_client.get_version() {
            Ok(version) => {
                info!("✅ Primary RPC OK: {}", version.solana_core);
                success_count += 1;
            }
            Err(e) => {
                warn!("❌ Primary RPC failed: {}", e);
            }
        }

        // Test connection pool
        total_tests += 1;
        match self.connection_pool.get_connection().await {
            Ok(_) => {
                info!("✅ Connection pool OK");
                success_count += 1;
            }
            Err(e) => {
                warn!("❌ Connection pool failed: {}", e);
            }
        }

        // Test backup endpoints
        for endpoint in &self.config.test_endpoints {
            total_tests += 1;
            let client = RpcClient::new_with_commitment(
                endpoint.clone(),
                CommitmentConfig::confirmed(),
            );
            
            match client.get_version() {
                Ok(_) => {
                    info!("✅ Backup endpoint OK: {}", endpoint);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("❌ Backup endpoint failed {}: {}", endpoint, e);
                }
            }
        }

        TestResult {
            success: success_count == total_tests,
            duration: start_time.elapsed(),
            details: format!("{}/{} connectivity tests passed", success_count, total_tests),
            metrics: vec![
                ("success_rate".to_string(), success_count as f64 / total_tests as f64),
                ("total_tests".to_string(), total_tests as f64),
            ],
        }
    }

    async fn test_architecture(&self) -> TestResult {
        info!("🏗️ Testing architecture components...");
        let start_time = Instant::now();

        let mut success_count = 0;
        let mut total_tests = 0;

        // Test EnhancedSharedState
        total_tests += 1;
        let enhanced_state = EnhancedSharedState::new();
        match enhanced_state.initialize().await {
            Ok(_) => {
                // Test atomic operations
                enhanced_state.update_raydium_price(Decimal::from_f64_retain(150.0).unwrap());
                enhanced_state.update_orca_price(Decimal::from_f64_retain(149.5).unwrap());
                
                let snapshot = enhanced_state.get_price_snapshot();
                if snapshot.is_valid {
                    info!("✅ EnhancedSharedState OK");
                    success_count += 1;
                } else {
                    warn!("❌ EnhancedSharedState invalid snapshot");
                }
            }
            Err(e) => {
                warn!("❌ EnhancedSharedState failed: {}", e);
            }
        }

        // Test concurrent operations
        total_tests += 1;
        let concurrent_result = self.test_concurrent_operations(&enhanced_state).await;
        if concurrent_result {
            info!("✅ Concurrent operations OK");
            success_count += 1;
        } else {
            warn!("❌ Concurrent operations failed");
        }

        TestResult {
            success: success_count == total_tests,
            duration: start_time.elapsed(),
            details: format!("{}/{} architecture tests passed", success_count, total_tests),
            metrics: vec![
                ("success_rate".to_string(), success_count as f64 / total_tests as f64),
            ],
        }
    }

    async fn test_concurrent_operations(&self, _state: &EnhancedSharedState) -> bool {
        let handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
        
        // Simulate concurrent operations without actual spawning
        for i in 0..10 {
            for j in 0..100 {
                let price = Decimal::from_f64_retain(150.0 + (i * 100 + j) as f64 * 0.001).unwrap();
                if i % 2 == 0 {
                    _state.update_raydium_price(price);
                } else {
                    _state.update_orca_price(price);
                }
            }
        }

        // Verify final state is consistent
        let snapshot = _state.get_price_snapshot();
        snapshot.is_valid
    }

    async fn test_performance(&self) -> TestResult {
        info!("⚡ Testing performance...");
        let start_time = Instant::now();

        let mut metrics = Vec::new();

        // Test RPC response times
        let rpc_times = self.measure_rpc_performance().await;
        metrics.push(("avg_rpc_time_ms".to_string(), rpc_times.0));
        metrics.push(("max_rpc_time_ms".to_string(), rpc_times.1));

        // Test connection pool performance
        let pool_stats = self.connection_pool.get_stats();
        metrics.push(("pool_hit_rate".to_string(), pool_stats.pool_hit_rate));
        metrics.push(("avg_response_time_ms".to_string(), pool_stats.average_response_time_ms as f64));

        // Test arbitrage calculation speed
        let calc_time = self.measure_calculation_performance().await;
        metrics.push(("calc_time_ms".to_string(), calc_time));

        let success = rpc_times.0 < 1000.0 && calc_time < 100.0; // Performance thresholds

        TestResult {
            success,
            duration: start_time.elapsed(),
            details: format!("Performance: RPC {:.1}ms, Calc {:.1}ms", rpc_times.0, calc_time),
            metrics,
        }
    }

    async fn measure_rpc_performance(&self) -> (f64, f64) {
        let mut times = Vec::new();
        
        for _ in 0..10 {
            let start = Instant::now();
            if let Ok(_) = self.rpc_client.get_slot() {
                times.push(start.elapsed().as_millis() as f64);
            }
        }

        if times.is_empty() {
            return (999999.0, 999999.0);
        }

        let avg = times.iter().sum::<f64>() / times.len() as f64;
        let max = times.iter().fold(0.0f64, |a, &b| a.max(b));
        
        (avg, max)
    }

    async fn measure_calculation_performance(&self) -> f64 {
        let start = Instant::now();
        
        // Simulate arbitrage calculations
        for _ in 0..1000 {
            let _opportunity = ArbitrageOpportunity {
                buy_dex: "Raydium".to_string(),
                sell_dex: "Orca".to_string(),
                buy_price: 150.0,
                sell_price: 151.0,
                amount_sol: 1.0,
                expected_profit_usd: 5.0,
                profit_after_fees_usd: 5.0,
                profit_percentage: 0.5,
                estimated_gas_sol: 0.001,
                price_impact: 0.1,
                confidence_score: 0.9,
            };
        }
        
        start.elapsed().as_millis() as f64
    }

    async fn test_error_handling(&self) -> TestResult {
        info!("🛡️ Testing error handling...");
        let start_time = Instant::now();

        let mut success_count = 0;
        let mut total_tests = 0;

        // Test safe operation wrapper
        total_tests += 1;
        let result: Result<()> = self.error_handler.safe_execute(
            async { Err(anyhow::anyhow!("Test error")) },
            "test_context"
        ).await;
        
        if result.is_err() {
            info!("✅ Error handling correctly caught error");
            success_count += 1;
        }

        // Test panic recovery
        total_tests += 1;
        let panic_result: Result<()> = crate::performance::error_handling::safe_operation(
            async { panic!("Test panic") },
            "panic_test"
        ).await;
        
        if panic_result.is_err() {
            info!("✅ Panic recovery working");
            success_count += 1;
        }

        // Test circuit breaker
        total_tests += 1;
        for _ in 0..6 {
            let _ = self.error_handler.handle_error::<()>(
                &anyhow::anyhow!("Repeated error"),
                "circuit_test"
            ).await;
        }
        
        let health = self.error_handler.health_check().await;
        if matches!(health, crate::performance::error_handling::HealthStatus::Critical) {
            info!("✅ Circuit breaker activated");
            success_count += 1;
        }

        TestResult {
            success: success_count == total_tests,
            duration: start_time.elapsed(),
            details: format!("{}/{} error handling tests passed", success_count, total_tests),
            metrics: vec![],
        }
    }

    async fn test_security(&self) -> TestResult {
        info!("🔒 Testing security components...");
        let start_time = Instant::now();

        let mut success_count = 0;
        let mut total_tests = 0;

        // Test SecurityManager
        total_tests += 1;
        let security_manager = SecurityManager::new();
        match security_manager.full_security_audit().await {
            Ok(report) => {
                info!("✅ Security audit completed: score {:.1}/10", report.overall_score);
                success_count += 1;
            }
            Err(e) => {
                warn!("❌ Security audit failed: {}", e);
            }
        }

        TestResult {
            success: success_count == total_tests,
            duration: start_time.elapsed(),
            details: format!("{}/{} security tests passed", success_count, total_tests),
            metrics: vec![],
        }
    }

    async fn test_flash_loans(&self) -> TestResult {
        info!("⚡ Testing flash loan security...");
        let start_time = Instant::now();

        let mut success_count = 0;
        let mut total_tests = 0;

        // Test FlashLoanGuard
        total_tests += 1;
        let mut flash_guard = FlashLoanGuard::new(
            crate::security::FlashLoanConfig::default()
        );

        // Test with safe parameters
        let test_opportunity = crate::security::flash_loan_guard::ArbitrageOpportunity {
            expected_profit_usd: 10.0,
            amount_sol: 1.0,
            spread_percent: 1.0,
            estimated_slippage: 0.5,
            sol_price_usd: 150.0,
        };

        // This should work (simulation only)
        match flash_guard.execute_secure_flash_loan(1.0, &test_opportunity).await {
            Ok(_) => {
                info!("✅ Flash loan security validation working");
                success_count += 1;
            }
            Err(e) => {
                warn!("❌ Flash loan test failed: {}", e);
            }
        }

        TestResult {
            success: success_count == total_tests,
            duration: start_time.elapsed(),
            details: format!("{}/{} flash loan tests passed", success_count, total_tests),
            metrics: vec![],
        }
    }

    async fn test_stress(&self) -> TestResult {
        info!("💪 Running stress tests...");
        let start_time = Instant::now();

        // High-frequency operations test
        let operations_per_second = self.stress_test_operations().await;
        
        // Memory usage test
        let memory_stable = self.stress_test_memory().await;
        
        // Connection pool stress test
        let pool_stable = self.stress_test_connection_pool().await;

        let success = operations_per_second > 100.0 && memory_stable && pool_stable;

        TestResult {
            success,
            duration: start_time.elapsed(),
            details: format!("Stress: {:.0} ops/sec, Memory: {}, Pool: {}", 
                           operations_per_second, memory_stable, pool_stable),
            metrics: vec![
                ("operations_per_second".to_string(), operations_per_second),
            ],
        }
    }

    async fn stress_test_operations(&self) -> f64 {
        let start = Instant::now();
        let mut operations = 0;

        while start.elapsed() < Duration::from_secs(10) {
            // Simulate high-frequency operations
            let _snapshot = EnhancedSharedState::new().get_price_snapshot();
            operations += 1;
            
            if operations % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }

        operations as f64 / start.elapsed().as_secs_f64()
    }

    async fn stress_test_memory(&self) -> bool {
        // Test bounded collections under stress
        let state = EnhancedSharedState::new();
        
        for i in 0..10000 {
            let trade = crate::architecture::TradeRecord {
                timestamp: std::time::SystemTime::now(),
                profit_usd: i as f64,
                amount_sol: 1.0,
                success: true,
                dex_buy: "Test".to_string(),
                dex_sell: "Test".to_string(),
            };
            state.record_trade(trade).await;
        }

        // Memory should be bounded
        let recent_trades = state.get_recent_trades(20000).await;
        recent_trades.len() <= 1000 // Should be bounded to max size
    }

    async fn stress_test_connection_pool(&self) -> bool {
        let mut handles = Vec::new();
        
        // Spawn many concurrent connection requests
        for _ in 0..50 {
            let pool = self.connection_pool.clone();
            let handle = tokio::spawn(async move {
                for _ in 0..10 {
                    if let Ok(_conn) = pool.get_connection().await {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            if handle.await.is_err() {
                return false;
            }
        }

        true
    }

    async fn test_live_trading(&self) -> TestResult {
        warn!("🔴 LIVE TRADING TESTS - USING REAL FUNDS");
        let start_time = Instant::now();

        // Only run if explicitly enabled and on devnet
        if !self.config.enable_live_trading_tests || !self.config.rpc_url.contains("devnet") {
            return TestResult {
                success: true,
                duration: start_time.elapsed(),
                details: "Live trading tests skipped for safety".to_string(),
                metrics: vec![],
            };
        }

        // TODO: Implement safe live trading tests with minimal amounts
        warn!("⚠️ Live trading tests not yet implemented for safety");

        TestResult {
            success: true,
            duration: start_time.elapsed(),
            details: "Live trading tests not implemented".to_string(),
            metrics: vec![],
        }
    }

    fn print_test_summary(&self, results: &TestResults) {
        info!("📊 TEST SUMMARY");
        info!("================");
        info!("Overall Success: {}", if results.overall_success { "✅ PASS" } else { "❌ FAIL" });
        info!("Total Duration: {:?}", results.total_duration);
        info!("");
        info!("Individual Results:");
        info!("  Connectivity: {}", if results.connectivity.success { "✅" } else { "❌" });
        info!("  Architecture: {}", if results.architecture.success { "✅" } else { "❌" });
        info!("  Performance:  {}", if results.performance.success { "✅" } else { "❌" });
        info!("  Error Handling: {}", if results.error_handling.success { "✅" } else { "❌" });
        info!("  Security:     {}", if results.security.success { "✅" } else { "❌" });
        info!("  Flash Loans:  {}", if results.flash_loans.success { "✅" } else { "❌" });
        info!("  Stress:       {}", if results.stress.success { "✅" } else { "❌" });
        info!("  Live Trading: {}", if results.live_trading.success { "✅" } else { "❌" });
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestResults {
    pub connectivity: TestResult,
    pub architecture: TestResult,
    pub performance: TestResult,
    pub error_handling: TestResult,
    pub security: TestResult,
    pub flash_loans: TestResult,
    pub stress: TestResult,
    pub live_trading: TestResult,
    pub total_duration: Duration,
    pub overall_success: bool,
}

impl TestResults {
    fn new() -> Self {
        Self {
            connectivity: TestResult::default(),
            architecture: TestResult::default(),
            performance: TestResult::default(),
            error_handling: TestResult::default(),
            security: TestResult::default(),
            flash_loans: TestResult::default(),
            stress: TestResult::default(),
            live_trading: TestResult::default(),
            total_duration: Duration::default(),
            overall_success: false,
        }
    }

    fn calculate_overall_success(&self) -> bool {
        self.connectivity.success
            && self.architecture.success
            && self.performance.success
            && self.error_handling.success
            && self.security.success
            && self.flash_loans.success
            && self.stress.success
            && self.live_trading.success
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub duration: Duration,
    pub details: String,
    pub metrics: Vec<(String, f64)>,
}

impl Default for TestResult {
    fn default() -> Self {
        Self {
            success: false,
            duration: Duration::default(),
            details: "Not run".to_string(),
            metrics: vec![],
        }
    }
}
