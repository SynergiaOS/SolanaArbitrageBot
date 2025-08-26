use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

/// High-performance RPC connection pool with load balancing
pub struct RpcConnectionPool {
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    config: PoolConfig,
    stats: Arc<PoolStats>,
    health_checker: Arc<HealthChecker>,
}

#[derive(Clone)]
pub struct PoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub health_check_interval: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub primary_rpc_url: String,
    pub backup_rpc_urls: Vec<String>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            primary_rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            backup_rpc_urls: vec![
                "https://solana-api.projectserum.com".to_string(),
                "https://api.mainnet-beta.solana.com".to_string(),
            ],
        }
    }
}

#[derive(Clone)]
struct PooledConnection {
    client: Arc<RpcClient>,
    created_at: Instant,
    last_used: Instant,
    request_count: u64,
    error_count: u64,
    is_healthy: bool,
    endpoint_url: String,
}

impl PooledConnection {
    fn new(url: &str) -> Self {
        let client = Arc::new(RpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig::confirmed(),
        ));

        Self {
            client,
            created_at: Instant::now(),
            last_used: Instant::now(),
            request_count: 0,
            error_count: 0,
            is_healthy: true,
            endpoint_url: url.to_string(),
        }
    }

    fn is_expired(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }

    fn update_usage(&mut self, success: bool) {
        self.last_used = Instant::now();
        self.request_count += 1;
        if !success {
            self.error_count += 1;
        }
        
        // Mark as unhealthy if error rate > 50%
        if self.request_count > 10 {
            let error_rate = self.error_count as f64 / self.request_count as f64;
            self.is_healthy = error_rate < 0.5;
        }
    }
}

#[derive(Default)]
pub struct PoolStats {
    pub total_requests: AtomicU64,
    pub successful_requests: AtomicU64,
    pub failed_requests: AtomicU64,
    pub active_connections: AtomicUsize,
    pub total_connections_created: AtomicU64,
    pub average_response_time_ms: AtomicU64,
    pub pool_hits: AtomicU64,
    pub pool_misses: AtomicU64,
}

impl PoolStats {
    pub fn record_request(&self, success: bool, response_time: Duration) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        // Update average response time (simple moving average)
        let current_avg = self.average_response_time_ms.load(Ordering::Relaxed);
        let new_time = response_time.as_millis() as u64;
        let new_avg = (current_avg + new_time) / 2;
        self.average_response_time_ms.store(new_avg, Ordering::Relaxed);
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 100.0;
        }
        let successful = self.successful_requests.load(Ordering::Relaxed);
        (successful as f64 / total as f64) * 100.0
    }
}

struct HealthChecker {
    pool: Arc<RwLock<Vec<PooledConnection>>>,
    config: PoolConfig,
}

impl HealthChecker {
    fn new(pool: Arc<RwLock<Vec<PooledConnection>>>, config: PoolConfig) -> Self {
        Self { pool, config }
    }

    async fn start_health_checks(&self) {
        let mut interval = tokio::time::interval(self.config.health_check_interval);
        let pool = self.pool.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                Self::perform_health_check(&pool, &config).await;
            }
        });
    }

    async fn perform_health_check(
        pool: &Arc<RwLock<Vec<PooledConnection>>>,
        config: &PoolConfig,
    ) {
        let mut connections = pool.write().await;
        let mut healthy_count = 0;
        let mut removed_count = 0;

        // Check each connection
        for i in (0..connections.len()).rev() {
            let conn = &mut connections[i];
            
            // Remove expired connections
            if conn.is_expired(config.idle_timeout) {
                connections.remove(i);
                removed_count += 1;
                continue;
            }

            // Test connection health
            match Self::test_connection_health(&conn.client).await {
                Ok(_) => {
                    conn.is_healthy = true;
                    healthy_count += 1;
                }
                Err(_) => {
                    conn.is_healthy = false;
                    // Remove unhealthy connections
                    connections.remove(i);
                    removed_count += 1;
                }
            }
        }

        if removed_count > 0 {
            debug!("🧹 Removed {} expired/unhealthy connections", removed_count);
        }

        // Ensure minimum connections
        while connections.len() < config.min_connections {
            if let Ok(new_conn) = Self::create_new_connection(config).await {
                connections.push(new_conn);
                debug!("➕ Added new connection to maintain minimum pool size");
            } else {
                warn!("Failed to create new connection for minimum pool size");
                break;
            }
        }

        debug!("🏥 Health check: {} healthy connections", healthy_count);
    }

    async fn test_connection_health(client: &RpcClient) -> Result<()> {
        // Quick health check - get slot
        let _slot = client.get_slot()?;
        Ok(())
    }

    async fn create_new_connection(config: &PoolConfig) -> Result<PooledConnection> {
        // Try primary first, then backups
        let urls = std::iter::once(&config.primary_rpc_url)
            .chain(config.backup_rpc_urls.iter());

        for url in urls {
            match Self::test_url_connection(url).await {
                Ok(_) => {
                    debug!("✅ Created new connection to {}", url);
                    return Ok(PooledConnection::new(url));
                }
                Err(e) => {
                    debug!("❌ Failed to connect to {}: {}", url, e);
                }
            }
        }

        Err(anyhow!("Failed to create connection to any RPC endpoint"))
    }

    async fn test_url_connection(url: &str) -> Result<()> {
        let client = RpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig::confirmed(),
        );

        let _slot = client.get_slot()?;
        Ok(())
    }
}

impl RpcConnectionPool {
    pub async fn new(config: PoolConfig) -> Result<Self> {
        let connections = Arc::new(RwLock::new(Vec::new()));
        let semaphore = Arc::new(Semaphore::new(config.max_connections));
        let stats = Arc::new(PoolStats::default());
        let health_checker = Arc::new(HealthChecker::new(connections.clone(), config.clone()));

        let pool = Self {
            connections: connections.clone(),
            semaphore,
            config: config.clone(),
            stats,
            health_checker,
        };

        // Initialize minimum connections
        pool.initialize_connections().await?;

        // Start health checking
        pool.health_checker.start_health_checks().await;

        info!("🏊 RPC Connection Pool initialized with {} connections", config.min_connections);
        Ok(pool)
    }

    async fn initialize_connections(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        for _ in 0..self.config.min_connections {
            match HealthChecker::create_new_connection(&self.config).await {
                Ok(conn) => {
                    connections.push(conn);
                    self.stats.total_connections_created.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => {
                    warn!("Failed to create initial connection: {}", e);
                }
            }
        }

        if connections.is_empty() {
            return Err(anyhow!("Failed to create any initial connections"));
        }

        Ok(())
    }

    /// Get a connection from the pool with load balancing
    pub async fn get_connection(&self) -> Result<PooledRpcClient> {
        // Acquire semaphore permit
        let _permit = self.semaphore.acquire().await?;

        let start_time = Instant::now();
        
        // Try to get existing healthy connection
        if let Some(conn) = self.get_healthy_connection().await {
            self.stats.pool_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(PooledRpcClient::new(conn, self.stats.clone()));
        }

        // No healthy connection available, create new one
        self.stats.pool_misses.fetch_add(1, Ordering::Relaxed);
        
        let new_conn = HealthChecker::create_new_connection(&self.config).await?;
        self.stats.total_connections_created.fetch_add(1, Ordering::Relaxed);
        
        // Add to pool if there's space
        let mut connections = self.connections.write().await;
        if connections.len() < self.config.max_connections {
            connections.push(new_conn.clone());
        }

        debug!("🆕 Created new connection in {}ms", start_time.elapsed().as_millis());
        Ok(PooledRpcClient::new(Arc::new(new_conn), self.stats.clone()))
    }

    async fn get_healthy_connection(&self) -> Option<Arc<PooledConnection>> {
        let connections = self.connections.read().await;
        
        // Find the least used healthy connection
        connections
            .iter()
            .filter(|conn| conn.is_healthy)
            .min_by_key(|conn| conn.request_count)
            .cloned()
            .map(|conn| Arc::new(conn))
    }

    /// Execute operation with automatic retry and failover
    pub async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn(Arc<RpcClient>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send + Sync,
        T: Send,
    {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            let start_time = Instant::now();
            
            match self.get_connection().await {
                Ok(pooled_client) => {
                    match operation(pooled_client.client.clone()).await {
                        Ok(result) => {
                            self.stats.record_request(true, start_time.elapsed());
                            return Ok(result);
                        }
                        Err(e) => {
                            self.stats.record_request(false, start_time.elapsed());
                            last_error = Some(e);
                            
                            if attempt < self.config.max_retries {
                                let delay = self.config.retry_delay * attempt;
                                debug!("🔄 Retry {}/{} in {:?}", attempt, self.config.max_retries, delay);
                                tokio::time::sleep(delay).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStatsSnapshot {
        PoolStatsSnapshot {
            total_requests: self.stats.total_requests.load(Ordering::Relaxed),
            successful_requests: self.stats.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.stats.failed_requests.load(Ordering::Relaxed),
            success_rate: self.stats.get_success_rate(),
            active_connections: self.stats.active_connections.load(Ordering::Relaxed),
            total_connections_created: self.stats.total_connections_created.load(Ordering::Relaxed),
            average_response_time_ms: self.stats.average_response_time_ms.load(Ordering::Relaxed),
            pool_hit_rate: {
                let hits = self.stats.pool_hits.load(Ordering::Relaxed);
                let misses = self.stats.pool_misses.load(Ordering::Relaxed);
                if hits + misses > 0 {
                    (hits as f64 / (hits + misses) as f64) * 100.0
                } else {
                    0.0
                }
            },
        }
    }
}

/// Wrapper for pooled RPC client with automatic stats tracking
pub struct PooledRpcClient {
    pub client: Arc<RpcClient>,
    stats: Arc<PoolStats>,
}

impl PooledRpcClient {
    fn new(connection: Arc<PooledConnection>, stats: Arc<PoolStats>) -> Self {
        stats.active_connections.fetch_add(1, Ordering::Relaxed);
        
        Self {
            client: connection.client.clone(),
            stats,
        }
    }
}

impl Drop for PooledRpcClient {
    fn drop(&mut self) {
        self.stats.active_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct PoolStatsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub success_rate: f64,
    pub active_connections: usize,
    pub total_connections_created: u64,
    pub average_response_time_ms: u64,
    pub pool_hit_rate: f64,
}
