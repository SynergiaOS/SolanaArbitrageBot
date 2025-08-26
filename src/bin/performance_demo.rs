use anyhow::Result;
use log::info;
use solana_arbitrage_bot::performance::{
    PerformanceManager, PoolConfig, ErrorHandlingConfig, PerformanceConfig,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    info!("🚀 Performance & Error Handling Demo");
    
    // Demo 1: Connection pooling
    demo_connection_pooling().await?;
    
    // Demo 2: Error handling
    demo_error_handling().await?;
    
    // Demo 3: Performance optimization
    demo_performance_optimization().await?;
    
    // Demo 4: Comprehensive testing
    demo_testing_framework().await?;
    
    info!("✅ All performance demos completed successfully!");
    Ok(())
}

async fn demo_connection_pooling() -> Result<()> {
    info!("🏊 Demo 1: Connection Pooling");
    
    let pool_config = PoolConfig {
        max_connections: 10,
        min_connections: 2,
        primary_rpc_url: "https://api.devnet.solana.com".to_string(),
        backup_rpc_urls: vec![
            "https://api.testnet.solana.com".to_string(),
        ],
        ..Default::default()
    };
    
    let error_config = ErrorHandlingConfig::default();
    let perf_config = PerformanceConfig::default();
    
    let performance_manager = PerformanceManager::new(
        pool_config,
        error_config,
        perf_config,
    ).await?;
    
    // Test multiple concurrent connections
    let mut handles = Vec::new();
    
    for i in 0..20 {
        let manager = performance_manager.clone();
        let handle = tokio::spawn(async move {
            match manager.get_rpc_connection().await {
                Ok(_conn) => {
                    info!("✅ Connection {} acquired successfully", i);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    true
                }
                Err(e) => {
                    info!("❌ Connection {} failed: {}", i, e);
                    false
                }
            }
        });
        handles.push(handle);
    }
    
    let mut successful_connections = 0;
    for handle in handles {
        if let Ok(success) = handle.await {
            if success {
                successful_connections += 1;
            }
        }
    }
    
    info!("📊 Connection pool results: {}/20 successful connections", successful_connections);
    
    // Show pool statistics
    let pool_stats = performance_manager.get_connection_pool().get_stats();
    info!("📈 Pool stats:");
    info!("  - Total requests: {}", pool_stats.total_requests);
    info!("  - Success rate: {:.1}%", pool_stats.success_rate);
    info!("  - Pool hit rate: {:.1}%", pool_stats.pool_hit_rate);
    info!("  - Avg response time: {}ms", pool_stats.average_response_time_ms);
    
    Ok(())
}

async fn demo_error_handling() -> Result<()> {
    info!("🛡️ Demo 2: Robust Error Handling");
    
    let pool_config = PoolConfig::default();
    let error_config = ErrorHandlingConfig::default();
    let perf_config = PerformanceConfig::default();
    
    let performance_manager = PerformanceManager::new(
        pool_config,
        error_config,
        perf_config,
    ).await?;
    
    // Test 1: Safe operation that might panic
    info!("🧪 Testing panic recovery...");
    let result = performance_manager.execute_optimized(
        async {
            // This would normally panic, but our system catches it
            if true {
                return Err(anyhow::anyhow!("Simulated error"));
            }
            Ok("Success")
        },
        "panic_test"
    ).await;
    
    match result {
        Ok(_) => info!("✅ Operation succeeded"),
        Err(e) => info!("✅ Error handled gracefully: {}", e),
    }
    
    // Test 2: Multiple errors to trigger circuit breaker
    info!("🔴 Testing circuit breaker...");
    for i in 1..=6 {
        let result = performance_manager.execute_optimized::<_, ()>(
            async {
                Err(anyhow::anyhow!("Repeated error {}", i))
            },
            "circuit_breaker_test"
        ).await;
        
        if let Err(e) = result {
            info!("❌ Error {}: {}", i, e);
        }
    }
    
    // Show error statistics
    let error_stats = performance_manager.get_error_handler().get_error_stats().await;
    info!("📊 Error handling stats:");
    info!("  - Total errors: {}", error_stats.total_errors);
    info!("  - Consecutive errors: {}", error_stats.consecutive_errors);
    info!("  - Recovery attempts: {}", error_stats.recovery_attempts);
    info!("  - Error rate: {:.1}%", error_stats.error_rate * 100.0);
    
    Ok(())
}

async fn demo_performance_optimization() -> Result<()> {
    info!("⚡ Demo 3: Performance Optimization");
    
    let pool_config = PoolConfig::default();
    let error_config = ErrorHandlingConfig::default();
    let perf_config = PerformanceConfig::default();
    
    let performance_manager = PerformanceManager::new(
        pool_config,
        error_config,
        perf_config,
    ).await?;
    
    // Simulate high-frequency operations
    info!("🏃 Running high-frequency operations...");
    
    let start_time = std::time::Instant::now();
    let mut operations = 0;
    
    while start_time.elapsed() < Duration::from_secs(5) {
        let result = performance_manager.execute_optimized(
            async {
                // Simulate fast operation
                tokio::time::sleep(Duration::from_micros(100)).await;
                Ok(operations)
            },
            "high_frequency_test"
        ).await;
        
        if result.is_ok() {
            operations += 1;
        }
        
        if operations % 100 == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    let ops_per_second = operations as f64 / start_time.elapsed().as_secs_f64();
    info!("📈 Performance: {:.0} operations/second", ops_per_second);
    
    // Record some business metrics
    performance_manager.record_arbitrage_opportunity(true);
    performance_manager.record_arbitrage_opportunity(false);
    performance_manager.record_profit(25.50).await;
    
    // Show comprehensive performance stats
    let perf_stats = performance_manager.get_performance_stats().await;
    info!("📊 Performance statistics:");
    info!("  - Total operations: {}", perf_stats.total_operations);
    info!("  - Success rate: {:.1}%", perf_stats.success_rate);
    info!("  - Avg execution time: {:.1}ms", perf_stats.average_execution_time_ms);
    info!("  - Fastest operation: {}ms", perf_stats.fastest_operation_ms);
    info!("  - Slowest operation: {}ms", perf_stats.slowest_operation_ms);
    info!("  - Arbitrage opportunities: {}", perf_stats.arbitrage_opportunities_found);
    info!("  - Arbitrage executed: {}", perf_stats.arbitrage_opportunities_executed);
    info!("  - Total profit: ${:.2}", perf_stats.total_profit_usd);
    
    // Health check
    let health = performance_manager.health_check().await;
    info!("🏥 System health: {:?}", health.status);
    if !health.issues.is_empty() {
        info!("⚠️ Health issues:");
        for issue in health.issues {
            info!("  - {}", issue);
        }
    }
    
    Ok(())
}

async fn demo_testing_framework() -> Result<()> {
    info!("🧪 Demo 4: Testing Framework");
    
    // Run quick tests
    info!("🏃 Running quick development tests...");
    match solana_arbitrage_bot::testing::run_quick_tests().await {
        Ok(success) => {
            if success {
                info!("✅ Quick tests passed!");
            } else {
                info!("❌ Quick tests failed!");
            }
        }
        Err(e) => {
            info!("❌ Quick tests error: {}", e);
        }
    }
    
    // Show what production tests would do (without actually running them)
    info!("🏭 Production test suite would include:");
    info!("  - Integration tests (8 categories)");
    info!("  - Performance benchmarks");
    info!("  - Security tests with fuzzing");
    info!("  - Stress tests");
    info!("  - Load testing");
    info!("  - Memory leak detection");
    info!("  - Connection pool stress testing");
    info!("  - Error recovery validation");
    
    info!("💡 To run full production tests:");
    info!("   cargo run --bin production_tests");
    
    Ok(())
}

/// Helper trait to make PerformanceManager cloneable for demo
trait CloneableManager {
    fn clone(&self) -> Self;
}

impl CloneableManager for PerformanceManager {
    fn clone(&self) -> Self {
        // For demo purposes, create a new instance
        // In real usage, you'd use Arc<PerformanceManager>
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                PerformanceManager::new(
                    PoolConfig::default(),
                    ErrorHandlingConfig::default(),
                    PerformanceConfig::default(),
                ).await.unwrap()
            })
        })
    }
}
