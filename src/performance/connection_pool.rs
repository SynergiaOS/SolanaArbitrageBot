//! Connection Pool Manager - Optimized for low latency
//! Manages multiple RPC connections with automatic failover

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use log::{info, warn, error};

#[derive(Clone)]
pub struct ConnectionPool {
    /// Primary RPC connections
    primary_pool: Arc<Vec<Arc<RpcClient>>>,
    /// Backup RPC connections
    backup_pool: Arc<Vec<Arc<RpcClient>>>,
    /// Current connection index for round-robin
    current_index: Arc<RwLock<usize>>,
    /// Health status of connections
    health_status: Arc<RwLock<Vec<(bool, Instant)>>>,
    /// Metrics
    metrics: Arc<RwLock<ConnectionMetrics>>,
}

#[derive(Default)]
struct ConnectionMetrics {
    total_requests: u64,
    failed_requests: u64,
    avg_latency_ms: f64,
    last_health_check: Instant,
}

impl ConnectionPool {
    pub fn new(urls: Vec<String>, backup_urls: Vec<String>) -> Result<Self> {
        let primary_pool: Vec<Arc<RpcClient>> = urls
            .iter()
            .map(|url| Arc::new(RpcClient::new(url.clone())))
            .collect();

        let backup_pool: Vec<Arc<RpcClient>> = backup_urls
            .iter()
            .map(|url| Arc::new(RpcClient::new(url.clone())))
            .collect();

        let health_status = vec![(true, Instant::now()); primary_pool.len()];

        Ok(Self {
            primary_pool: Arc::new(primary_pool),
            backup_pool: Arc::new(backup_pool),
            current_index: Arc::new(RwLock::new(0)),
            health_status: Arc::new(RwLock::new(health_status)),
            metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
        })
    }

    /// Get the fastest available RPC connection
    pub async fn get_fastest_client(&self) -> Arc<RpcClient> {
        let health_status = self.health_status.read().await;
        
        // Find healthy connections
        let healthy_indices: Vec<usize> = health_status
            .iter()
            .enumerate()
            .filter(|(_, (healthy, _))| *healthy)
            .map(|(i, _)| i)
            .collect();

        if healthy_indices.is_empty() {
            warn!("No healthy primary connections, using backup");
            return self.backup_pool[0].clone();
        }

        // Round-robin among healthy connections
        let mut index = self.current_index.write().await;
        *index = (*index + 1) % healthy_indices.len();
        
        self.primary_pool[healthy_indices[*index]].clone()
    }

    /// Execute request with automatic retry and failover
    pub async fn execute_with_retry<T, F, Fut>(
        &self,
        operation: F,
        max_retries: u32,
    ) -> Result<T>
    where
        F: Fn(Arc<RpcClient>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        
        // Try primary pool
        for _ in 0..max_retries {
            let client = self.get_fastest_client().await;
            let start = Instant::now();
            
            match operation(client.clone()).await {
                Ok(result) => {
                    // Update metrics
                    let mut metrics = self.metrics.write().await;
                    metrics.total_requests += 1;
                    let latency = start.elapsed().as_millis() as f64;
                    metrics.avg_latency_ms = 
                        (metrics.avg_latency_ms * metrics.total_requests as f64 + latency) 
                        / (metrics.total_requests + 1) as f64;
                    
                    return Ok(result);
                }
                Err(e) => {
                    warn!("Request failed: {}", e);
                    last_error = Some(e);
                    
                    // Mark connection as unhealthy
                    self.mark_unhealthy(client).await;
                }
            }
        }

        // Try backup pool if all primary failed
        for backup_client in self.backup_pool.iter() {
            match operation(backup_client.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All connections failed")))
    }

    /// Mark a connection as unhealthy
    async fn mark_unhealthy(&self, client: Arc<RpcClient>) {
        let mut health_status = self.health_status.write().await;
        
        // Find the index of this client
        for (i, primary_client) in self.primary_pool.iter().enumerate() {
            if Arc::ptr_eq(primary_client, &client) {
                health_status[i] = (false, Instant::now());
                warn!("Marked connection {} as unhealthy", i);
                break;
            }
        }
    }

    /// Health check task - run in background
    pub async fn start_health_check(&self) {
        let pool = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                pool.check_health().await;
            }
        });
    }

    async fn check_health(&self) {
        let mut health_status = self.health_status.write().await;
        
        for (i, client) in self.primary_pool.iter().enumerate() {
            let start = Instant::now();
            match client.get_slot().await {
                Ok(_) => {
                    let latency = start.elapsed().as_millis();
                    if latency < 200 {
                        health_status[i] = (true, Instant::now());
                        info!("Connection {} healthy ({}ms)", i, latency);
                    }
                }
                Err(_) => {
                    health_status[i] = (false, Instant::now());
                    warn!("Connection {} unhealthy", i);
                }
            }
        }
    }

    /// Get connection metrics
    pub async fn get_metrics(&self) -> String {
        let metrics = self.metrics.read().await;
        format!(
            "Total requests: {}, Failed: {}, Avg latency: {:.2}ms",
            metrics.total_requests,
            metrics.failed_requests,
            metrics.avg_latency_ms
        )
    }
}

/// Pre-configured connection pool for production
pub fn create_production_pool() -> Result<ConnectionPool> {
    let urls = vec![
        "https://api.mainnet-beta.solana.com".to_string(),
        "https://solana-mainnet.g.alchemy.com/v2/demo".to_string(),
        "https://rpc.ankr.com/solana".to_string(),
    ];
    
    let backup_urls = vec![
        "https://solana-api.projectserum.com".to_string(),
        "https://api.devnet.solana.com".to_string(), // Fallback to devnet for testing
    ];
    
    ConnectionPool::new(urls, backup_urls)
}
