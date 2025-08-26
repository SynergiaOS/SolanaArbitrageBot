//! 🚨 Kestra Alert Management
//! 
//! Alert configuration, event handling, and notification management

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, anyhow};
use log::{debug, info, warn, error};

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Alert system enabled
    pub enabled: bool,
    
    /// Default notification channels
    pub default_channels: Vec<NotificationChannel>,
    
    /// Alert thresholds
    pub thresholds: AlertThresholds,
    
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    
    /// Escalation rules
    pub escalation_rules: Vec<EscalationRule>,
    
    /// Alert templates
    pub templates: HashMap<String, AlertTemplate>,
}

/// Notification channels configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel ID
    pub id: String,
    
    /// Channel type
    pub channel_type: ChannelType,
    
    /// Channel configuration
    pub config: HashMap<String, Value>,
    
    /// Whether channel is enabled
    pub enabled: bool,
    
    /// Priority level for this channel
    pub priority: AlertPriority,
}

/// Types of notification channels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelType {
    Email,
    Slack,
    Discord,
    Webhook,
    SMS,
    PagerDuty,
    Console,
}

/// Alert thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Execution failure rate threshold (%)
    pub failure_rate_threshold: f64,
    
    /// Execution time threshold (seconds)
    pub execution_time_threshold: u64,
    
    /// Queue size threshold
    pub queue_size_threshold: usize,
    
    /// Memory usage threshold (%)
    pub memory_threshold: f64,
    
    /// CPU usage threshold (%)
    pub cpu_threshold: f64,
    
    /// Disk usage threshold (%)
    pub disk_threshold: f64,
}

/// Rate limiting configuration for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum alerts per time window
    pub max_alerts_per_window: u32,
    
    /// Time window duration
    pub window_duration: Duration,
    
    /// Cooldown period after rate limit
    pub cooldown_duration: Duration,
}

/// Escalation rule for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    /// Rule ID
    pub id: String,
    
    /// Alert types this rule applies to
    pub alert_types: Vec<AlertType>,
    
    /// Time to wait before escalation
    pub escalation_delay: Duration,
    
    /// Target channels for escalation
    pub escalation_channels: Vec<String>,
    
    /// Whether rule is enabled
    pub enabled: bool,
}

/// Alert template for formatting messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertTemplate {
    /// Template ID
    pub id: String,
    
    /// Template subject
    pub subject: String,
    
    /// Template body (supports placeholders)
    pub body: String,
    
    /// Template format (text, html, markdown)
    pub format: String,
}

/// Alert event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    /// Event ID
    pub id: String,
    
    /// Alert type
    pub alert_type: AlertType,
    
    /// Alert priority
    pub priority: AlertPriority,
    
    /// Event title
    pub title: String,
    
    /// Event message
    pub message: String,
    
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Source of the alert
    pub source: AlertSource,
    
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
    
    /// Related workflow/execution ID
    pub related_id: Option<String>,
    
    /// Alert status
    pub status: AlertStatus,
    
    /// Notification history
    pub notifications: Vec<NotificationRecord>,
}

/// Types of alerts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    WorkflowFailed,
    WorkflowTimeout,
    HighFailureRate,
    SystemResourceHigh,
    QueueBacklog,
    ServiceDown,
    ConfigurationError,
    SecurityAlert,
    Performance,
    Custom(String),
}

/// Alert priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertPriority {
    Low,
    Medium,
    High,
    Critical,
    Emergency,
}

/// Alert sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSource {
    Workflow,
    System,
    Monitor,
    User,
    External,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    New,
    Acknowledged,
    InProgress,
    Resolved,
    Escalated,
    Suppressed,
}

/// Notification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    /// Notification ID
    pub id: String,
    
    /// Channel used
    pub channel: String,
    
    /// Timestamp sent
    pub sent_at: SystemTime,
    
    /// Success status
    pub success: bool,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// Response details
    pub response: Option<String>,
}

/// Alert manager for handling all alert operations
pub struct AlertManager {
    /// Configuration
    config: AlertConfig,
    
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, AlertEvent>>>,
    
    /// Alert history
    alert_history: Arc<RwLock<Vec<AlertEvent>>>,
    
    /// Rate limiting tracker
    rate_limiter: Arc<RwLock<HashMap<String, Vec<SystemTime>>>>,
    
    /// HTTP client for webhooks
    http_client: reqwest::Client,
    
    /// Channel handlers
    channel_handlers: HashMap<ChannelType, Box<dyn ChannelHandler + Send + Sync>>,
}

/// Trait for notification channel handlers
#[async_trait::async_trait]
pub trait ChannelHandler {
    /// Send notification through this channel
    async fn send_notification(
        &self,
        channel: &NotificationChannel,
        alert: &AlertEvent,
        template: Option<&AlertTemplate>,
    ) -> Result<NotificationRecord>;
}

impl AlertManager {
    /// Create new alert manager
    pub fn new(config: AlertConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        let mut manager = Self {
            config,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            rate_limiter: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            channel_handlers: HashMap::new(),
        };
        
        // Initialize channel handlers
        manager.initialize_channel_handlers();
        
        manager
    }
    
    /// Create default alert manager
    pub fn default() -> Self {
        Self::new(AlertConfig::default())
    }
    
    /// Initialize channel handlers
    fn initialize_channel_handlers(&mut self) {
        // Add built-in channel handlers
        // In a real implementation, these would be proper handlers
    }
    
    /// Start alert manager
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Alert manager is disabled");
            return Ok(());
        }
        
        info!("🚨 Starting alert manager...");
        
        // Start background tasks
        self.start_cleanup_task().await;
        self.start_escalation_task().await;
        
        info!("✅ Alert manager started successfully");
        Ok(())
    }
    
    /// Start cleanup task for old alerts
    async fn start_cleanup_task(&self) {
        let active_alerts = self.active_alerts.clone();
        let alert_history = self.alert_history.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_hours(1));
            loop {
                interval.tick().await;
                
                // Clean up resolved alerts older than 24 hours
                let cutoff = SystemTime::now() - Duration::from_secs(24 * 3600);
                let mut to_remove = Vec::new();
                
                {
                    let alerts = active_alerts.read().await;
                    for (id, alert) in alerts.iter() {
                        if matches!(alert.status, AlertStatus::Resolved | AlertStatus::Suppressed) 
                           && alert.timestamp < cutoff {
                            to_remove.push((id.clone(), alert.clone()));
                        }
                    }
                }
                
                if !to_remove.is_empty() {
                    let mut alerts = active_alerts.write().await;
                    let mut history = alert_history.write().await;
                    
                    for (id, alert) in to_remove {
                        alerts.remove(&id);
                        history.push(alert);
                    }
                    
                    // Keep only last 10000 alerts in history
                    if history.len() > 10000 {
                        let excess = history.len() - 10000;
                        history.drain(0..excess);
                    }
                }
            }
        });
    }
    
    /// Start escalation task
    async fn start_escalation_task(&self) {
        let active_alerts = self.active_alerts.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_minutes(1));
            loop {
                interval.tick().await;
                
                // Check for alerts needing escalation
                let alerts = active_alerts.read().await;
                for alert in alerts.values() {
                    if alert.status == AlertStatus::New || alert.status == AlertStatus::Acknowledged {
                        for rule in &config.escalation_rules {
                            if rule.enabled && rule.alert_types.contains(&alert.alert_type) {
                                let elapsed = SystemTime::now()
                                    .duration_since(alert.timestamp)
                                    .unwrap_or_default();
                                
                                if elapsed >= rule.escalation_delay {
                                    // Trigger escalation (simplified)
                                    debug!("Alert {} needs escalation", alert.id);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// Create and send alert
    pub async fn send_alert(
        &self,
        alert_type: AlertType,
        priority: AlertPriority,
        title: String,
        message: String,
        source: AlertSource,
        metadata: Option<HashMap<String, Value>>,
        related_id: Option<String>,
    ) -> Result<String> {
        if !self.config.enabled {
            return Ok("alert_disabled".to_string());
        }
        
        // Check rate limiting
        if self.is_rate_limited(&alert_type).await {
            warn!("Alert rate limited: {:?}", alert_type);
            return Err(anyhow!("Alert rate limited"));
        }
        
        // Create alert event
        let alert_id = self.generate_alert_id();
        let alert = AlertEvent {
            id: alert_id.clone(),
            alert_type: alert_type.clone(),
            priority,
            title,
            message,
            timestamp: SystemTime::now(),
            source,
            metadata: metadata.unwrap_or_default(),
            related_id,
            status: AlertStatus::New,
            notifications: Vec::new(),
        };
        
        // Store active alert
        {
            let mut active_alerts = self.active_alerts.write().await;
            active_alerts.insert(alert_id.clone(), alert.clone());
        }
        
        // Send notifications
        self.send_notifications(&alert).await?;
        
        // Update rate limiter
        self.update_rate_limiter(&alert_type).await;
        
        info!("🚨 Alert sent: {} - {}", alert_type_to_string(&alert_type), alert.title);
        
        Ok(alert_id)
    }
    
    /// Send notifications for alert
    async fn send_notifications(&self, alert: &AlertEvent) -> Result<()> {
        let channels = self.get_channels_for_alert(alert);
        let template = self.get_template_for_alert(alert);
        
        for channel in channels {
            if let Err(e) = self.send_channel_notification(&channel, alert, template.as_ref()).await {
                error!("Failed to send notification to {}: {}", channel.id, e);
            }
        }
        
        Ok(())
    }
    
    /// Send notification to specific channel
    async fn send_channel_notification(
        &self,
        channel: &NotificationChannel,
        alert: &AlertEvent,
        template: Option<&AlertTemplate>,
    ) -> Result<()> {
        // Simulate notification sending
        debug!("Sending notification to {} channel: {}", channel.channel_type, channel.id);
        
        // In real implementation, this would use the appropriate channel handler
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Record notification
        let notification = NotificationRecord {
            id: format!("notif_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()),
            channel: channel.id.clone(),
            sent_at: SystemTime::now(),
            success: true,
            error: None,
            response: Some("OK".to_string()),
        };
        
        // Update alert with notification record
        {
            let mut active_alerts = self.active_alerts.write().await;
            if let Some(alert_mut) = active_alerts.get_mut(&alert.id) {
                alert_mut.notifications.push(notification);
            }
        }
        
        Ok(())
    }
    
    /// Get channels for alert based on priority and type
    fn get_channels_for_alert(&self, alert: &AlertEvent) -> Vec<NotificationChannel> {
        self.config.default_channels.iter()
            .filter(|ch| ch.enabled && ch.priority <= alert.priority)
            .cloned()
            .collect()
    }
    
    /// Get template for alert
    fn get_template_for_alert(&self, alert: &AlertEvent) -> Option<AlertTemplate> {
        let template_key = alert_type_to_string(&alert.alert_type).to_lowercase();
        self.config.templates.get(&template_key).cloned()
            .or_else(|| self.config.templates.get("default").cloned())
    }
    
    /// Check if alert type is rate limited
    async fn is_rate_limited(&self, alert_type: &AlertType) -> bool {
        let key = alert_type_to_string(alert_type);
        let now = SystemTime::now();
        let window_start = now - self.config.rate_limiting.window_duration;
        
        let mut rate_limiter = self.rate_limiter.write().await;
        let timestamps = rate_limiter.entry(key).or_insert_with(Vec::new);
        
        // Remove old timestamps
        timestamps.retain(|&ts| ts >= window_start);
        
        // Check if rate limited
        timestamps.len() >= self.config.rate_limiting.max_alerts_per_window as usize
    }
    
    /// Update rate limiter
    async fn update_rate_limiter(&self, alert_type: &AlertType) {
        let key = alert_type_to_string(alert_type);
        let now = SystemTime::now();
        
        let mut rate_limiter = self.rate_limiter.write().await;
        let timestamps = rate_limiter.entry(key).or_insert_with(Vec::new);
        timestamps.push(now);
    }
    
    /// Generate unique alert ID
    fn generate_alert_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("alert_{}", timestamp)
    }
    
    /// Get alert by ID
    pub async fn get_alert(&self, alert_id: &str) -> Option<AlertEvent> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.get(alert_id).cloned()
    }
    
    /// List active alerts
    pub async fn list_active_alerts(&self) -> Vec<AlertEvent> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values().cloned().collect()
    }
    
    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Acknowledged;
            info!("Acknowledged alert: {}", alert_id);
            Ok(())
        } else {
            Err(anyhow!("Alert not found: {}", alert_id))
        }
    }
    
    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Resolved;
            info!("Resolved alert: {}", alert_id);
            Ok(())
        } else {
            Err(anyhow!("Alert not found: {}", alert_id))
        }
    }
    
    /// Suppress alert
    pub async fn suppress_alert(&self, alert_id: &str) -> Result<()> {
        let mut active_alerts = self.active_alerts.write().await;
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Suppressed;
            info!("Suppressed alert: {}", alert_id);
            Ok(())
        } else {
            Err(anyhow!("Alert not found: {}", alert_id))
        }
    }
    
    /// Get alert statistics
    pub async fn get_alert_stats(&self) -> AlertStats {
        let active_alerts = self.active_alerts.read().await;
        let history = self.alert_history.read().await;
        
        let total_active = active_alerts.len();
        let critical_active = active_alerts.values()
            .filter(|a| matches!(a.priority, AlertPriority::Critical | AlertPriority::Emergency))
            .count();
        
        let total_resolved = history.len();
        
        AlertStats {
            total_active,
            critical_active,
            total_resolved,
            last_24h: active_alerts.values()
                .filter(|a| {
                    SystemTime::now().duration_since(a.timestamp).unwrap_or_default() < Duration::from_secs(24 * 3600)
                })
                .count(),
        }
    }
}

/// Alert statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    pub total_active: usize,
    pub critical_active: usize,
    pub total_resolved: usize,
    pub last_24h: usize,
}

/// Convert alert type to string
fn alert_type_to_string(alert_type: &AlertType) -> &str {
    match alert_type {
        AlertType::WorkflowFailed => "workflow_failed",
        AlertType::WorkflowTimeout => "workflow_timeout",
        AlertType::HighFailureRate => "high_failure_rate",
        AlertType::SystemResourceHigh => "system_resource_high",
        AlertType::QueueBacklog => "queue_backlog",
        AlertType::ServiceDown => "service_down",
        AlertType::ConfigurationError => "configuration_error",
        AlertType::SecurityAlert => "security_alert",
        AlertType::Performance => "performance",
        AlertType::Custom(name) => name,
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_channels: vec![
                NotificationChannel {
                    id: "console".to_string(),
                    channel_type: ChannelType::Console,
                    config: HashMap::new(),
                    enabled: true,
                    priority: AlertPriority::Low,
                }
            ],
            thresholds: AlertThresholds::default(),
            rate_limiting: RateLimitConfig::default(),
            escalation_rules: Vec::new(),
            templates: HashMap::new(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            failure_rate_threshold: 10.0,
            execution_time_threshold: 300,
            queue_size_threshold: 100,
            memory_threshold: 80.0,
            cpu_threshold: 90.0,
            disk_threshold: 85.0,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_alerts_per_window: 10,
            window_duration: Duration::from_minutes(5),
            cooldown_duration: Duration::from_minutes(1),
        }
    }
}
