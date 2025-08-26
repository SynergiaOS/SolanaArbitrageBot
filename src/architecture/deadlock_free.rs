use anyhow::{anyhow, Result};
use log::{debug, error, warn};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

/// Deadlock-free mutex manager with ordered locking
pub struct DeadlockFreeManager {
    locks: HashMap<String, Arc<Mutex<()>>>,
    lock_order: Vec<String>,
    timeout_duration: Duration,
    lock_attempts: Arc<AtomicU64>,
    lock_timeouts: Arc<AtomicU64>,
    deadlock_detections: Arc<AtomicU64>,
}

impl DeadlockFreeManager {
    pub fn new(timeout_seconds: u64) -> Self {
        // Define strict lock ordering to prevent deadlocks
        let lock_order = vec![
            "raydium_price".to_string(),
            "orca_price".to_string(),
            "trades_today".to_string(),
            "profit_today".to_string(),
            "trade_history".to_string(),
            "circuit_breaker".to_string(),
            "price_cache".to_string(),
            "last_update_times".to_string(),
        ];

        let mut locks = HashMap::new();
        for lock_name in &lock_order {
            locks.insert(lock_name.clone(), Arc::new(Mutex::new(())));
        }

        Self {
            locks,
            lock_order,
            timeout_duration: Duration::from_secs(timeout_seconds),
            lock_attempts: Arc::new(AtomicU64::new(0)),
            lock_timeouts: Arc::new(AtomicU64::new(0)),
            deadlock_detections: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Acquire multiple locks in safe order
    pub async fn acquire_locks(&self, lock_names: &[&str]) -> Result<Vec<tokio::sync::MutexGuard<'_, ()>>> {
        self.lock_attempts.fetch_add(1, Ordering::Relaxed);
        
        // Sort locks according to predefined order
        let mut ordered_locks: Vec<&str> = lock_names.to_vec();
        ordered_locks.sort_by_key(|name| {
            self.lock_order.iter().position(|x| x == *name).unwrap_or(usize::MAX)
        });

        debug!("🔒 Acquiring locks in order: {:?}", ordered_locks);

        let mut guards = Vec::new();
        let start_time = Instant::now();

        for lock_name in ordered_locks {
            if let Some(lock) = self.locks.get(lock_name) {
                match timeout(self.timeout_duration, lock.lock()).await {
                    Ok(guard) => {
                        guards.push(guard);
                        debug!("✅ Acquired lock: {}", lock_name);
                    }
                    Err(_) => {
                        self.lock_timeouts.fetch_add(1, Ordering::Relaxed);
                        error!("⏰ Lock timeout for: {}", lock_name);
                        return Err(anyhow!("Lock timeout for: {}", lock_name));
                    }
                }
            } else {
                return Err(anyhow!("Unknown lock: {}", lock_name));
            }
        }

        let acquisition_time = start_time.elapsed();
        if acquisition_time > Duration::from_millis(100) {
            warn!("🐌 Slow lock acquisition: {:?} for {:?}", acquisition_time, lock_names);
        }

        Ok(guards)
    }

    /// Try to acquire locks without blocking
    pub fn try_acquire_locks(&self, lock_names: &[&str]) -> Result<Vec<tokio::sync::MutexGuard<'_, ()>>> {
        let mut ordered_locks: Vec<&str> = lock_names.to_vec();
        ordered_locks.sort_by_key(|name| {
            self.lock_order.iter().position(|x| x == *name).unwrap_or(usize::MAX)
        });

        let mut guards = Vec::new();

        for lock_name in ordered_locks {
            if let Some(lock) = self.locks.get(lock_name) {
                match lock.try_lock() {
                    Ok(guard) => {
                        guards.push(guard);
                        debug!("✅ Try-acquired lock: {}", lock_name);
                    }
                    Err(try_lock_error) => {
                        debug!("🔒 Lock busy: {}", lock_name);
                        return Err(anyhow!("Lock busy: {}", lock_name));
                    }
                }
            } else {
                return Err(anyhow!("Unknown lock: {}", lock_name));
            }
        }

        Ok(guards)
    }

    /// Get lock statistics
    pub fn get_stats(&self) -> LockStats {
        LockStats {
            total_attempts: self.lock_attempts.load(Ordering::Relaxed),
            timeouts: self.lock_timeouts.load(Ordering::Relaxed),
            deadlock_detections: self.deadlock_detections.load(Ordering::Relaxed),
            success_rate: {
                let attempts = self.lock_attempts.load(Ordering::Relaxed);
                let timeouts = self.lock_timeouts.load(Ordering::Relaxed);
                if attempts > 0 {
                    ((attempts - timeouts) as f64 / attempts as f64) * 100.0
                } else {
                    100.0
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockStats {
    pub total_attempts: u64,
    pub timeouts: u64,
    pub deadlock_detections: u64,
    pub success_rate: f64,
}

/// Lock-free shared state using atomic operations and RwLock
pub struct LockFreeSharedState {
    // Atomic counters
    trades_today: Arc<AtomicU64>,
    
    // RwLock for infrequent writes, frequent reads
    profit_today: Arc<RwLock<f64>>,
    
    // Configuration that changes rarely
    config: Arc<RwLock<StateConfig>>,
    
    // Statistics
    read_operations: Arc<AtomicU64>,
    write_operations: Arc<AtomicU64>,
}

impl LockFreeSharedState {
    pub fn new() -> Self {
        Self {
            trades_today: Arc::new(AtomicU64::new(0)),
            profit_today: Arc::new(RwLock::new(0.0)),
            config: Arc::new(RwLock::new(StateConfig::default())),
            read_operations: Arc::new(AtomicU64::new(0)),
            write_operations: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment trades counter atomically
    pub fn increment_trades(&self) -> u64 {
        self.write_operations.fetch_add(1, Ordering::Relaxed);
        self.trades_today.fetch_add(1, Ordering::Relaxed)
    }

    /// Get trades count atomically
    pub fn get_trades_today(&self) -> u64 {
        self.read_operations.fetch_add(1, Ordering::Relaxed);
        self.trades_today.load(Ordering::Acquire)
    }

    /// Update profit with minimal locking
    pub async fn add_profit(&self, profit: f64) -> Result<()> {
        self.write_operations.fetch_add(1, Ordering::Relaxed);
        
        // Use timeout to prevent hanging
        match timeout(Duration::from_millis(100), self.profit_today.write()).await {
            Ok(mut profit_guard) => {
                *profit_guard += profit;
                Ok(())
            }
            Err(_) => Err(anyhow!("Timeout updating profit")),
        }
    }

    /// Get current profit with minimal locking
    pub async fn get_profit_today(&self) -> Result<f64> {
        self.read_operations.fetch_add(1, Ordering::Relaxed);
        
        match timeout(Duration::from_millis(50), self.profit_today.read()).await {
            Ok(profit_guard) => Ok(*profit_guard),
            Err(_) => Err(anyhow!("Timeout reading profit")),
        }
    }

    /// Reset daily counters
    pub async fn reset_daily(&self) -> Result<()> {
        self.write_operations.fetch_add(1, Ordering::Relaxed);
        
        // Reset atomic counter
        self.trades_today.store(0, Ordering::Release);
        
        // Reset profit with timeout
        match timeout(Duration::from_millis(100), self.profit_today.write()).await {
            Ok(mut profit_guard) => {
                *profit_guard = 0.0;
                Ok(())
            }
            Err(_) => Err(anyhow!("Timeout resetting profit")),
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        let reads = self.read_operations.load(Ordering::Relaxed);
        let writes = self.write_operations.load(Ordering::Relaxed);
        
        PerformanceStats {
            total_reads: reads,
            total_writes: writes,
            read_write_ratio: if writes > 0 { reads as f64 / writes as f64 } else { 0.0 },
        }
    }
}

#[derive(Debug, Clone)]
pub struct StateConfig {
    pub max_daily_trades: u64,
    pub max_daily_loss: f64,
    pub enable_trading: bool,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            max_daily_trades: 100,
            max_daily_loss: 1000.0,
            enable_trading: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_reads: u64,
    pub total_writes: u64,
    pub read_write_ratio: f64,
}

/// Deadlock detection and recovery system
pub struct DeadlockDetector {
    active_operations: Arc<RwLock<HashMap<String, OperationInfo>>>,
    detection_interval: Duration,
    max_operation_time: Duration,
}

impl DeadlockDetector {
    pub fn new(detection_interval_ms: u64, max_operation_time_ms: u64) -> Self {
        Self {
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            detection_interval: Duration::from_millis(detection_interval_ms),
            max_operation_time: Duration::from_millis(max_operation_time_ms),
        }
    }

    /// Register operation start
    pub async fn register_operation(&self, operation_id: String, description: String) {
        let mut ops = self.active_operations.write().await;
        ops.insert(operation_id, OperationInfo {
            description,
            start_time: Instant::now(),
            thread_id: std::thread::current().id(),
        });
    }

    /// Unregister operation completion
    pub async fn unregister_operation(&self, operation_id: &str) {
        let mut ops = self.active_operations.write().await;
        ops.remove(operation_id);
    }

    /// Start deadlock detection loop
    pub async fn start_detection(&self) {
        let mut interval = tokio::time::interval(self.detection_interval);
        let operations = self.active_operations.clone();
        let max_time = self.max_operation_time;

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                
                let ops = operations.read().await;
                let now = Instant::now();
                
                for (op_id, op_info) in ops.iter() {
                    if now.duration_since(op_info.start_time) > max_time {
                        error!("🚨 POTENTIAL DEADLOCK DETECTED:");
                        error!("   Operation: {} ({})", op_id, op_info.description);
                        error!("   Duration: {:?}", now.duration_since(op_info.start_time));
                        error!("   Thread: {:?}", op_info.thread_id);
                        
                        // TODO: Implement recovery mechanism
                        // - Force unlock stuck mutexes
                        // - Restart stuck operations
                        // - Emergency shutdown if needed
                    }
                }
            }
        });
    }
}

#[derive(Debug, Clone)]
struct OperationInfo {
    description: String,
    start_time: Instant,
    thread_id: std::thread::ThreadId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ordered_locking() {
        let manager = DeadlockFreeManager::new(5);
        
        // Test acquiring locks in different orders - should always work
        let _guards1 = manager.acquire_locks(&["orca_price", "raydium_price"]).await.unwrap();
        drop(_guards1);
        
        let _guards2 = manager.acquire_locks(&["raydium_price", "orca_price"]).await.unwrap();
        drop(_guards2);
        
        // Both should succeed because locks are acquired in consistent order
    }

    #[tokio::test]
    async fn test_lock_free_state() {
        let state = LockFreeSharedState::new();
        
        // Test atomic operations
        assert_eq!(state.get_trades_today(), 0);
        state.increment_trades();
        assert_eq!(state.get_trades_today(), 1);
        
        // Test profit operations
        state.add_profit(100.0).await.unwrap();
        assert_eq!(state.get_profit_today().await.unwrap(), 100.0);
    }
}
