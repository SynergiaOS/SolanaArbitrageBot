//! 🔄 Kestra Workflows Management
//! 
//! Workflow definition, execution, and management for Kestra orchestration

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, anyhow};
use log::{debug, info, warn, error};

/// Workflow definition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique workflow identifier
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Workflow description
    pub description: String,
    
    /// YAML content of the workflow
    pub yaml_content: String,
    
    /// Workflow variables
    pub variables: HashMap<String, Value>,
    
    /// Whether workflow is enabled
    pub enabled: bool,
    
    /// Tags for categorization
    pub tags: Vec<String>,
    
    /// Workflow version
    pub version: String,
    
    /// Creation timestamp
    pub created_at: SystemTime,
    
    /// Last update timestamp
    pub updated_at: SystemTime,
    
    /// Namespace
    pub namespace: String,
    
    /// Trigger configurations
    pub triggers: Vec<WorkflowTrigger>,
    
    /// Task definitions
    pub tasks: Vec<WorkflowTask>,
    
    /// Flow dependencies
    pub dependencies: Vec<String>,
}

/// Workflow trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    /// Trigger ID
    pub id: String,
    
    /// Trigger type (schedule, webhook, etc.)
    pub trigger_type: TriggerType,
    
    /// Trigger configuration
    pub config: HashMap<String, Value>,
    
    /// Whether trigger is enabled
    pub enabled: bool,
}

/// Types of workflow triggers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TriggerType {
    Schedule,
    Webhook,
    FileWatcher,
    Manual,
    Dependency,
}

/// Workflow task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    /// Task ID
    pub id: String,
    
    /// Task type
    pub task_type: String,
    
    /// Task configuration
    pub config: HashMap<String, Value>,
    
    /// Dependencies on other tasks
    pub depends_on: Vec<String>,
    
    /// Task timeout
    pub timeout: Option<Duration>,
    
    /// Retry configuration
    pub retry_config: Option<TaskRetryConfig>,
}

/// Task retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    
    /// Delay between retries
    pub delay: Duration,
    
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    /// Execution ID
    pub id: String,
    
    /// Workflow ID
    pub workflow_id: String,
    
    /// Execution status
    pub status: ExecutionStatus,
    
    /// Start time
    pub started_at: SystemTime,
    
    /// Completion time
    pub completed_at: Option<SystemTime>,
    
    /// Execution duration
    pub duration: Option<Duration>,
    
    /// Input parameters
    pub inputs: HashMap<String, Value>,
    
    /// Output results
    pub outputs: HashMap<String, Value>,
    
    /// Error message if failed
    pub error_message: Option<String>,
    
    /// Task executions
    pub task_executions: Vec<TaskExecution>,
    
    /// Execution logs
    pub logs: Vec<ExecutionLog>,
    
    /// Execution metadata
    pub metadata: HashMap<String, Value>,
    
    /// Trigger information
    pub trigger_info: Option<TriggerInfo>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Created,
    Running,
    Success,
    Failed,
    Killed,
    Warning,
    Paused,
}

/// Individual task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    /// Task execution ID
    pub id: String,
    
    /// Task ID from workflow definition
    pub task_id: String,
    
    /// Task status
    pub status: ExecutionStatus,
    
    /// Start time
    pub started_at: SystemTime,
    
    /// End time
    pub ended_at: Option<SystemTime>,
    
    /// Duration
    pub duration: Option<Duration>,
    
    /// Attempt number
    pub attempt: u32,
    
    /// Task outputs
    pub outputs: HashMap<String, Value>,
    
    /// Error information
    pub error: Option<String>,
}

/// Execution log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    /// Log timestamp
    pub timestamp: SystemTime,
    
    /// Log level
    pub level: LogLevel,
    
    /// Log message
    pub message: String,
    
    /// Task ID (if from a specific task)
    pub task_id: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Log levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Trigger information for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    /// Trigger ID
    pub trigger_id: String,
    
    /// Trigger type
    pub trigger_type: TriggerType,
    
    /// Trigger data
    pub data: HashMap<String, Value>,
    
    /// Trigger timestamp
    pub timestamp: SystemTime,
}

/// Workflow execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStats {
    /// Total executions
    pub total_executions: u64,
    
    /// Successful executions
    pub successful_executions: u64,
    
    /// Failed executions
    pub failed_executions: u64,
    
    /// Average execution time
    pub average_execution_time: Duration,
    
    /// Last execution time
    pub last_execution: Option<SystemTime>,
    
    /// Success rate percentage
    pub success_rate: f64,
}

/// Workflow manager for orchestrating complex trading workflows
#[derive(Debug)] // Removing Clone from derive as there's manual implementation
pub struct WorkflowManager {
    workflows: Arc<RwLock<HashMap<String, WorkflowDefinition>>>,
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    scheduler: Arc<RwLock<Option<cron::Schedule>>>,
    config: WorkflowConfig,
    rpc_client: Option<Arc<RpcClient>>,
    metrics: Arc<RwLock<WorkflowMetrics>>,
}

impl WorkflowManager {
    /// Create new workflow manager
    pub fn new(config: super::KestraConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .ok();
        
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            config: Some(config),
        }
    }
    
    /// Create default workflow manager
    pub fn default() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            http_client: None,
            config: None,
        }
    }
    
    /// Register a workflow definition
    pub async fn register_workflow(&self, id: &str, workflow: WorkflowDefinition) -> Result<()> {
        info!("Registering workflow: {}", id);
        
        let mut workflows = self.workflows.write().await;
        workflows.insert(id.to_string(), workflow);
        
        debug!("Workflow {} registered successfully", id);
        Ok(())
    }
    
    /// Get workflow definition
    pub async fn get_workflow(&self, id: &str) -> Option<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        workflows.get(id).cloned()
    }
    
    /// List all workflows
    pub async fn list_workflows(&self) -> Vec<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        workflows.values().cloned().collect()
    }
    
    /// Execute workflow
    pub async fn execute_workflow(&self, workflow_id: &str, inputs: Option<HashMap<String, Value>>) -> Result<String> {
        info!("Executing workflow: {}", workflow_id);
        
        let workflow = self.get_workflow(workflow_id).await
            .ok_or_else(|| anyhow!("Workflow not found: {}", workflow_id))?;
        
        if !workflow.enabled {
            return Err(anyhow!("Workflow {} is disabled", workflow_id));
        }
        
        // Create execution
        let execution_id = self.generate_execution_id();
        let execution = WorkflowExecution {
            id: execution_id.clone(),
            workflow_id: workflow_id.to_string(),
            status: ExecutionStatus::Created,
            started_at: SystemTime::now(),
            completed_at: None,
            duration: None,
            inputs: inputs.unwrap_or_default(),
            outputs: HashMap::new(),
            error_message: None,
            task_executions: Vec::new(),
            logs: Vec::new(),
            metadata: HashMap::new(),
            trigger_info: None,
        };
        
        // Store execution
        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), execution);
        
        // Start execution asynchronously
        let manager = self.clone();
        let exec_id = execution_id.clone();
        tokio::spawn(async move {
            if let Err(e) = manager.run_workflow_execution(&exec_id).await {
                error!("Workflow execution failed: {}", e);
            }
        });
        
        Ok(execution_id)
    }
    
    /// Run workflow execution
    async fn run_workflow_execution(&self, execution_id: &str) -> Result<()> {
        info!("Running workflow execution: {}", execution_id);
        
        // Update status to running
        {
            let mut executions = self.executions.write().await;
            if let Some(execution) = executions.get_mut(execution_id) {
                execution.status = ExecutionStatus::Running;
                execution.logs.push(ExecutionLog {
                    timestamp: SystemTime::now(),
                    level: LogLevel::Info,
                    message: "Workflow execution started".to_string(),
                    task_id: None,
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Simulate workflow execution (in real implementation, this would execute tasks)
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Update status to success
        {
            let mut executions = self.executions.write().await;
            if let Some(execution) = executions.get_mut(execution_id) {
                execution.status = ExecutionStatus::Success;
                execution.completed_at = Some(SystemTime::now());
                execution.duration = execution.completed_at
                    .and_then(|end| end.duration_since(execution.started_at).ok());
                execution.logs.push(ExecutionLog {
                    timestamp: SystemTime::now(),
                    level: LogLevel::Info,
                    message: "Workflow execution completed successfully".to_string(),
                    task_id: None,
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Move to history
        self.archive_execution(execution_id).await?;
        
        Ok(())
    }
    
    /// Archive completed execution
    async fn archive_execution(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.remove(execution_id) {
            let mut history = self.execution_history.write().await;
            history.push(execution);
            
            // Keep only last 1000 executions
            if history.len() > 1000 {
                history.remove(0);
            }
        }
        Ok(())
    }
    
    /// Get execution status
    pub async fn get_execution(&self, execution_id: &str) -> Option<WorkflowExecution> {
        // Check active executions first
        {
            let executions = self.executions.read().await;
            if let Some(execution) = executions.get(execution_id) {
                return Some(execution.clone());
            }
        }
        
        // Check history
        let history = self.execution_history.read().await;
        history.iter()
            .find(|exec| exec.id == execution_id)
            .cloned()
    }
    
    /// List active executions
    pub async fn list_active_executions(&self) -> Vec<WorkflowExecution> {
        let executions = self.executions.read().await;
        executions.values().cloned().collect()
    }
    
    /// List execution history
    pub async fn list_execution_history(&self, limit: Option<usize>) -> Vec<WorkflowExecution> {
        let history = self.execution_history.read().await;
        match limit {
            Some(n) => history.iter().rev().take(n).cloned().collect(),
            None => history.clone(),
        }
    }
    
    /// Cancel execution
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Killed;
            execution.completed_at = Some(SystemTime::now());
            execution.duration = execution.completed_at
                .and_then(|end| end.duration_since(execution.started_at).ok());
            execution.logs.push(ExecutionLog {
                timestamp: SystemTime::now(),
                level: LogLevel::Warn,
                message: "Workflow execution cancelled".to_string(),
                task_id: None,
                metadata: HashMap::new(),
            });
            info!("Cancelled execution: {}", execution_id);
            Ok(())
        } else {
            Err(anyhow!("Execution not found: {}", execution_id))
        }
    }
    
    /// Update workflow statistics
    pub async fn update_workflow_stats(&self, workflow_id: &str, execution: &WorkflowExecution) {
        let mut stats = self.stats.write().await;
        let workflow_stats = stats.entry(workflow_id.to_string()).or_insert_with(|| WorkflowStats {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time: Duration::ZERO,
            last_execution: None,
            success_rate: 0.0,
        });
        
        workflow_stats.total_executions += 1;
        workflow_stats.last_execution = Some(execution.started_at);
        
        match execution.status {
            ExecutionStatus::Success => {
                workflow_stats.successful_executions += 1;
            }
            ExecutionStatus::Failed | ExecutionStatus::Killed => {
                workflow_stats.failed_executions += 1;
            }
            _ => {}
        }
        
        // Update success rate
        workflow_stats.success_rate = 
            (workflow_stats.successful_executions as f64 / workflow_stats.total_executions as f64) * 100.0;
        
        // Update average execution time
        if let Some(duration) = execution.duration {
            let total_time = workflow_stats.average_execution_time.as_secs_f64() * (workflow_stats.total_executions - 1) as f64;
            let new_average = (total_time + duration.as_secs_f64()) / workflow_stats.total_executions as f64;
            workflow_stats.average_execution_time = Duration::from_secs_f64(new_average);
        }
    }
    
    /// Get workflow statistics
    pub async fn get_workflow_stats(&self, workflow_id: &str) -> Option<WorkflowStats> {
        let stats = self.stats.read().await;
        stats.get(workflow_id).cloned()
    }
    
    /// Generate unique execution ID
    fn generate_execution_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("exec_{}", timestamp)
    }
    
    /// Enable workflow
    pub async fn enable_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.enabled = true;
            workflow.updated_at = SystemTime::now();
            info!("Enabled workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(anyhow!("Workflow not found: {}", workflow_id))
        }
    }
    
    /// Disable workflow
    pub async fn disable_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.enabled = false;
            workflow.updated_at = SystemTime::now();
            info!("Disabled workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(anyhow!("Workflow not found: {}", workflow_id))
        }
    }
    
    /// Delete workflow
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        if workflows.remove(workflow_id).is_some() {
            info!("Deleted workflow: {}", workflow_id);
            Ok(())
        } else {
            Err(anyhow!("Workflow not found: {}", workflow_id))
        }
    }
    
    /// Get all workflow statistics
    pub async fn get_all_stats(&self) -> HashMap<String, WorkflowStats> {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Clear execution history
    pub async fn clear_execution_history(&self) {
        let mut history = self.execution_history.write().await;
        history.clear();
        info!("Cleared execution history");
    }
    
    /// Export workflows to JSON
    pub async fn export_workflows(&self) -> Result<String> {
        let workflows = self.workflows.read().await;
        serde_json::to_string_pretty(&*workflows)
            .map_err(|e| anyhow!("Failed to export workflows: {}", e))
    }
    
    /// Import workflows from JSON
    pub async fn import_workflows(&self, json_data: &str) -> Result<()> {
        let imported_workflows: HashMap<String, WorkflowDefinition> = 
            serde_json::from_str(json_data)
                .map_err(|e| anyhow!("Failed to parse workflows JSON: {}", e))?;
        
        let mut workflows = self.workflows.write().await;
        for (id, workflow) in imported_workflows {
            workflows.insert(id.clone(), workflow);
            info!("Imported workflow: {}", id);
        }
        
        Ok(())
    }
}

// Clone implementation for async usage
impl Clone for WorkflowManager {
    fn clone(&self) -> Self {
        Self {
            workflows: self.workflows.clone(),
            executions: self.executions.clone(),
            execution_history: self.execution_history.clone(),
            stats: self.stats.clone(),
            http_client: self.http_client.clone(),
            config: self.config.clone(),
        }
    }
}

impl Default for WorkflowDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            yaml_content: String::new(),
            variables: HashMap::new(),
            enabled: true,
            tags: Vec::new(),
            version: "1.0.0".to_string(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            namespace: "default".to_string(),
            triggers: Vec::new(),
            tasks: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

impl Default for WorkflowExecution {
    fn default() -> Self {
        Self {
            id: String::new(),
            workflow_id: String::new(),
            status: ExecutionStatus::Created,
            started_at: SystemTime::now(),
            completed_at: None,
            duration: None,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            error_message: None,
            task_executions: Vec::new(),
            logs: Vec::new(),
            metadata: HashMap::new(),
            trigger_info: None,
        }
    }
}
