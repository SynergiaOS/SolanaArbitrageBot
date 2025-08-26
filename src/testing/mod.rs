//! Comprehensive testing framework for Solana Arbitrage Bot
//! 
//! Provides multiple types of testing:
//! - Integration tests for end-to-end functionality
//! - Performance tests and benchmarks
//! - Security tests including fuzzing
//! - Load testing and stress testing
//! - Mock testing for development

pub mod integration_tests;
pub mod performance_tests;
pub mod security_tests;

pub use integration_tests::{IntegrationTestSuite, TestConfig, TestResults};
pub use performance_tests::{PerformanceTestSuite, BenchmarkResults};
pub use security_tests::{SecurityTestSuite, SecurityTestResults};

use anyhow::Result;
use log::{info, warn};
use std::time::{Duration, Instant};

/// Master test coordinator that runs all test suites
pub struct MasterTestSuite {
    integration_suite: IntegrationTestSuite,
    performance_suite: PerformanceTestSuite,
    security_suite: SecurityTestSuite,
    config: MasterTestConfig,
}

#[derive(Clone)]
pub struct MasterTestConfig {
    pub run_integration_tests: bool,
    pub run_performance_tests: bool,
    pub run_security_tests: bool,
    pub run_stress_tests: bool,
    pub test_timeout_seconds: u64,
    pub parallel_execution: bool,
    pub generate_reports: bool,
    pub report_output_dir: String,
}

impl Default for MasterTestConfig {
    fn default() -> Self {
        Self {
            run_integration_tests: true,
            run_performance_tests: true,
            run_security_tests: true,
            run_stress_tests: false, // Disabled by default
            test_timeout_seconds: 1800, // 30 minutes
            parallel_execution: false, // Sequential by default for safety
            generate_reports: true,
            report_output_dir: "./test_reports".to_string(),
        }
    }
}

impl MasterTestSuite {
    pub async fn new(config: MasterTestConfig) -> Result<Self> {
        info!("🧪 Initializing Master Test Suite");

        let test_config = TestConfig {
            enable_live_trading_tests: false, // Always disabled for safety
            enable_stress_tests: config.run_stress_tests,
            enable_security_tests: config.run_security_tests,
            ..Default::default()
        };

        let integration_suite = IntegrationTestSuite::new(test_config).await?;
        let performance_suite = PerformanceTestSuite::new().await?;
        let security_suite = SecurityTestSuite::new().await?;

        Ok(Self {
            integration_suite,
            performance_suite,
            security_suite,
            config,
        })
    }

    /// Run all enabled test suites
    pub async fn run_all_tests(&self) -> Result<MasterTestResults> {
        info!("🚀 Starting Master Test Suite");
        let start_time = Instant::now();

        let mut results = MasterTestResults::new();

        // Set timeout for entire test run
        let test_future = self.execute_all_tests();
        let timeout_duration = Duration::from_secs(self.config.test_timeout_seconds);

        match tokio::time::timeout(timeout_duration, test_future).await {
            Ok(test_results) => {
                results = test_results?;
            }
            Err(_) => {
                warn!("⏰ Test suite timed out after {:?}", timeout_duration);
                results.timed_out = true;
            }
        }

        results.total_duration = start_time.elapsed();
        results.overall_success = results.calculate_overall_success();

        // Generate reports if enabled
        if self.config.generate_reports {
            self.generate_test_reports(&results).await?;
        }

        self.print_master_summary(&results);
        Ok(results)
    }

    async fn execute_all_tests(&self) -> Result<MasterTestResults> {
        let mut results = MasterTestResults::new();

        if self.config.parallel_execution {
            // Run tests in parallel (faster but more resource intensive)
            results = self.run_tests_parallel().await?;
        } else {
            // Run tests sequentially (safer, easier to debug)
            results = self.run_tests_sequential().await?;
        }

        Ok(results)
    }

    async fn run_tests_sequential(&self) -> Result<MasterTestResults> {
        let mut results = MasterTestResults::new();

        // Integration tests
        if self.config.run_integration_tests {
            info!("🔧 Running Integration Tests...");
            results.integration_results = Some(self.integration_suite.run_all_tests().await?);
        }

        // Performance tests
        if self.config.run_performance_tests {
            info!("⚡ Running Performance Tests...");
            results.performance_results = Some(self.performance_suite.run_all_benchmarks().await?);
        }

        // Security tests
        if self.config.run_security_tests {
            info!("🔒 Running Security Tests...");
            results.security_results = Some(self.security_suite.run_all_tests().await?);
        }

        Ok(results)
    }

    async fn run_tests_parallel(&self) -> Result<MasterTestResults> {
        // For now, fall back to sequential execution
        warn!("⚠️ Parallel execution not fully implemented, using sequential");
        self.run_tests_sequential().await
    }

    async fn generate_test_reports(&self, results: &MasterTestResults) -> Result<()> {
        info!("📊 Generating test reports...");

        // Create output directory
        tokio::fs::create_dir_all(&self.config.report_output_dir).await?;

        // Generate JSON report
        let json_report = serde_json::to_string_pretty(results)?;
        let json_path = format!("{}/test_results.json", self.config.report_output_dir);
        tokio::fs::write(json_path, json_report).await?;

        // Generate HTML report
        let html_report = self.generate_html_report(results);
        let html_path = format!("{}/test_results.html", self.config.report_output_dir);
        tokio::fs::write(html_path, html_report).await?;

        // Generate CSV summary
        let csv_report = self.generate_csv_summary(results);
        let csv_path = format!("{}/test_summary.csv", self.config.report_output_dir);
        tokio::fs::write(csv_path, csv_report).await?;

        info!("✅ Test reports generated in {}", self.config.report_output_dir);
        Ok(())
    }

    fn generate_html_report(&self, results: &MasterTestResults) -> String {
        let status_icon = if results.overall_success { "✅" } else { "❌" };
        let status_color = if results.overall_success { "green" } else { "red" };

        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>Solana Arbitrage Bot - Test Results</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .status {{ color: {}; font-weight: bold; }}
        .section {{ margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }}
        .success {{ background: #e8f5e8; }}
        .failure {{ background: #ffe8e8; }}
        .metric {{ margin: 5px 0; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{} Solana Arbitrage Bot Test Results</h1>
        <p class="status">Overall Status: {}</p>
        <p>Total Duration: {:?}</p>
        <p>Generated: {}</p>
    </div>

    <div class="section">
        <h2>Test Summary</h2>
        <table>
            <tr><th>Test Suite</th><th>Status</th><th>Duration</th><th>Details</th></tr>
            {}
        </table>
    </div>

    <div class="section">
        <h2>Performance Metrics</h2>
        {}
    </div>

    <div class="section">
        <h2>Security Analysis</h2>
        {}
    </div>
</body>
</html>
            "#,
            status_color,
            status_icon,
            if results.overall_success { "PASS" } else { "FAIL" },
            results.total_duration,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.generate_test_summary_table(results),
            self.generate_performance_metrics_html(results),
            self.generate_security_analysis_html(results)
        )
    }

    fn generate_test_summary_table(&self, results: &MasterTestResults) -> String {
        let mut rows = Vec::new();

        if let Some(ref integration) = results.integration_results {
            let status = if integration.overall_success { "✅ PASS" } else { "❌ FAIL" };
            rows.push(format!(
                "<tr><td>Integration</td><td>{}</td><td>{:?}</td><td>8 test categories</td></tr>",
                status, integration.total_duration
            ));
        }

        if let Some(ref performance) = results.performance_results {
            let status = if performance.overall_success { "✅ PASS" } else { "❌ FAIL" };
            rows.push(format!(
                "<tr><td>Performance</td><td>{}</td><td>{:?}</td><td>{} benchmarks</td></tr>",
                status, performance.total_duration, performance.benchmark_count
            ));
        }

        if let Some(ref security) = results.security_results {
            let status = if security.overall_success { "✅ PASS" } else { "❌ FAIL" };
            rows.push(format!(
                "<tr><td>Security</td><td>{}</td><td>{:?}</td><td>{} vulnerabilities found</td></tr>",
                status, security.total_duration, security.vulnerabilities_found
            ));
        }

        rows.join("\n")
    }

    fn generate_performance_metrics_html(&self, results: &MasterTestResults) -> String {
        if let Some(ref perf) = results.performance_results {
            format!(
                "<p>Average Response Time: {:.1}ms</p>
                 <p>Throughput: {:.0} ops/sec</p>
                 <p>Memory Usage: {:.1}MB</p>
                 <p>Success Rate: {:.1}%</p>",
                perf.average_response_time_ms,
                perf.operations_per_second,
                perf.memory_usage_mb,
                perf.success_rate
            )
        } else {
            "<p>Performance tests not run</p>".to_string()
        }
    }

    fn generate_security_analysis_html(&self, results: &MasterTestResults) -> String {
        if let Some(ref sec) = results.security_results {
            format!(
                "<p>Vulnerabilities Found: {}</p>
                 <p>Security Score: {:.1}/10</p>
                 <p>Critical Issues: {}</p>
                 <p>Warnings: {}</p>",
                sec.vulnerabilities_found,
                sec.security_score,
                sec.critical_issues,
                sec.warnings
            )
        } else {
            "<p>Security tests not run</p>".to_string()
        }
    }

    fn generate_csv_summary(&self, results: &MasterTestResults) -> String {
        let mut csv = "Test Suite,Status,Duration (ms),Success Rate,Details\n".to_string();

        if let Some(ref integration) = results.integration_results {
            csv.push_str(&format!(
                "Integration,{},{},{:.1}%,8 categories\n",
                if integration.overall_success { "PASS" } else { "FAIL" },
                integration.total_duration.as_millis(),
                if integration.overall_success { 100.0 } else { 0.0 }
            ));
        }

        if let Some(ref performance) = results.performance_results {
            csv.push_str(&format!(
                "Performance,{},{},{:.1}%,{} benchmarks\n",
                if performance.overall_success { "PASS" } else { "FAIL" },
                performance.total_duration.as_millis(),
                performance.success_rate,
                performance.benchmark_count
            ));
        }

        if let Some(ref security) = results.security_results {
            csv.push_str(&format!(
                "Security,{},{},{:.1}%,{} vulnerabilities\n",
                if security.overall_success { "PASS" } else { "FAIL" },
                security.total_duration.as_millis(),
                if security.vulnerabilities_found == 0 { 100.0 } else { 0.0 },
                security.vulnerabilities_found
            ));
        }

        csv
    }

    fn print_master_summary(&self, results: &MasterTestResults) {
        info!("🎯 MASTER TEST RESULTS");
        info!("======================");
        info!("Overall Success: {}", if results.overall_success { "✅ PASS" } else { "❌ FAIL" });
        info!("Total Duration: {:?}", results.total_duration);
        info!("Timed Out: {}", if results.timed_out { "⏰ YES" } else { "✅ NO" });
        info!("");

        if let Some(ref integration) = results.integration_results {
            info!("Integration Tests: {}", if integration.overall_success { "✅ PASS" } else { "❌ FAIL" });
        }

        if let Some(ref performance) = results.performance_results {
            info!("Performance Tests: {}", if performance.overall_success { "✅ PASS" } else { "❌ FAIL" });
            info!("  - Avg Response: {:.1}ms", performance.average_response_time_ms);
            info!("  - Throughput: {:.0} ops/sec", performance.operations_per_second);
        }

        if let Some(ref security) = results.security_results {
            info!("Security Tests: {}", if security.overall_success { "✅ PASS" } else { "❌ FAIL" });
            info!("  - Vulnerabilities: {}", security.vulnerabilities_found);
            info!("  - Security Score: {:.1}/10", security.security_score);
        }

        if self.config.generate_reports {
            info!("📊 Reports generated in: {}", self.config.report_output_dir);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MasterTestResults {
    pub integration_results: Option<TestResults>,
    pub performance_results: Option<BenchmarkResults>,
    pub security_results: Option<SecurityTestResults>,
    pub total_duration: Duration,
    pub overall_success: bool,
    pub timed_out: bool,
}

impl MasterTestResults {
    fn new() -> Self {
        Self {
            integration_results: None,
            performance_results: None,
            security_results: None,
            total_duration: Duration::default(),
            overall_success: false,
            timed_out: false,
        }
    }

    fn calculate_overall_success(&self) -> bool {
        if self.timed_out {
            return false;
        }

        let integration_ok = self.integration_results.as_ref().map_or(true, |r| r.overall_success);
        let performance_ok = self.performance_results.as_ref().map_or(true, |r| r.overall_success);
        let security_ok = self.security_results.as_ref().map_or(true, |r| r.overall_success);

        integration_ok && performance_ok && security_ok
    }
}

/// Quick test runner for development
pub async fn run_quick_tests() -> Result<bool> {
    info!("🏃 Running quick development tests...");

    let config = MasterTestConfig {
        run_integration_tests: true,
        run_performance_tests: false, // Skip for speed
        run_security_tests: false,    // Skip for speed
        run_stress_tests: false,
        test_timeout_seconds: 300,    // 5 minutes
        parallel_execution: false,
        generate_reports: false,      // Skip for speed
        ..Default::default()
    };

    let suite = MasterTestSuite::new(config).await?;
    let results = suite.run_all_tests().await?;

    Ok(results.overall_success)
}

/// Production test runner with full test suite
pub async fn run_production_tests() -> Result<bool> {
    info!("🏭 Running full production test suite...");

    let config = MasterTestConfig {
        run_integration_tests: true,
        run_performance_tests: true,
        run_security_tests: true,
        run_stress_tests: true,
        test_timeout_seconds: 1800,   // 30 minutes
        parallel_execution: false,    // Sequential for reliability
        generate_reports: true,
        report_output_dir: "./production_test_reports".to_string(),
    };

    let suite = MasterTestSuite::new(config).await?;
    let results = suite.run_all_tests().await?;

    Ok(results.overall_success)
}
