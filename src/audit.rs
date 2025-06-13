// src/audit_engine.rs

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
use std::sync::{Arc, Mutex};
use std::io::Write;
// use chrono::{DateTime, Utc}; // For timestamps

use crate::config::Config;

// Represents a single auditable event
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditEvent {
    // pub timestamp: DateTime<Utc>,
    pub timestamp_str: String, // Placeholder for simplicity for now
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub event_type: AuditEventType,
    pub details: serde_json::Value, // Flexible field for event-specific data
    pub risk_level: Option<RiskLevel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuditEventType {
    // System Events
    SystemStart,
    SystemShutdown,
    ConfigLoaded,
    PolicyLoaded,

    // VM/Container Lifecycle Events (from logging_audit.md)
    InstanceCreated { instance_id: String, instance_type: String }, // instance_type: "VM" or "Container"
    InstanceDeleted { instance_id: String },
    InstanceSnapshot { instance_id: String, snapshot_id: String },
    ModelAttached { instance_id: String, model_id: String },
    ModelDetached { instance_id: String, model_id: String },
    ResourceAllocation { instance_id: String, resource: String, value: String, success: bool },

    // Session Events
    SessionStart { session_id: String },
    SessionEnd { session_id: String },
    TerminalSessionRecorded { session_id: String, recording_path: PathBuf, format: String },

    // MCP Events (from logging_audit.md)
    McpMessageInbound { source: String, dest: String, message_type: String, success: bool, error: Option<String> },
    McpMessageOutbound { source: String, dest: String, message_type: String, success: bool, error: Option<String> },

    // Policy Events (from TECHNICAL_DESIGN.md & security.md)
    PolicyViolation { rule_id: String, agent_id: Option<String>, action: String, resource: Option<String> },
    PolicyDecision { agent_id: Option<String>, action: String, resource: Option<String>, allowed: bool, reason: Option<String> },
    RoleOverrideUsed { agent_id: String, role: String, original_role: String },

    // Agent Activity Events (from security.md)
    SshSessionEstablished { session_id: String, agent_id: String, source_ip: Option<String> },
    SshSessionTerminated { session_id: String, agent_id: String },
    CommandExecuted { session_id: String, agent_id: Option<String>, command: String, exit_code: Option<i32>, output_summary: Option<String> },
    FileSystemOperation { session_id: String, agent_id: Option<String>, operation: String, path: PathBuf, success: bool }, // Op: create, modify, delete, move
    NetworkConnection { session_id: String, agent_id: Option<String>, destination: String, protocol: String, allowed: bool },
    ProcessCreated { session_id: String, agent_id: Option<String>, process_name: String, pid: u32 },
    
    // Security Events (from security.md)
    AuthFailure { user_or_agent_id: String, reason: String },
    ResourceLimitViolation { instance_id: String, resource: String, limit: String, actual: String },
    AnomalyDetected { description: String, severity: RiskLevel },
    KeyOperation { operation: String, key_id: Option<String>, success: bool }, // Op: generate, rotate, revoke

    // CLI Commands
    CliCommandExecuted { command: String, args: Vec<String>, success: bool },

    // General / Unknown
    GenericMessage { message: String, level: RiskLevel },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
    Info, // For non-risky informational events
}

pub struct AuditEngine {
    // log_path: PathBuf, // This field is not read
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl AuditEngine {
    pub fn new(app_config: &Config) -> Result<Self> {
        let log_dir_str = shellexpand::tilde(&app_config.logging.log_dir).into_owned();
        let mut audit_log_dir = PathBuf::from(log_dir_str);
        
        // Determine the specific subdirectory for audit ledger if needed, or use log_dir directly
        // For now, let's assume audit ledger goes into a subdirectory "audit" within the main log_dir
        // as suggested by the comment in record_event: "~/.hydravisor/logs/audit/audit_ledger.jsonl"
        audit_log_dir.push("audit"); // e.g. ~/.hydravisor/logs/audit/

        if !audit_log_dir.exists() {
            fs::create_dir_all(&audit_log_dir)
                .map_err(|e| anyhow::anyhow!("Failed to create audit log directory {:?}: {}", audit_log_dir, e))?;
        }

        let log_file_path = audit_log_dir.join("audit_ledger.jsonl");

        println!(
            "AuditEngine initialized. Audit ledger will be at: {:?}",
            log_file_path
        );
        
        Ok(AuditEngine {
            writer: Arc::new(Mutex::new(Box::new(std::fs::File::create(log_file_path)?))),
        })
    }

    // This method is never used
    // pub fn record_event(&self, event: AuditEvent) -> Result<()> {
    //     let mut writer = self.writer.lock().unwrap();
    //     let mut json_string = serde_json::to_string(&event)?;
    //     json_string.push('\n');
    //     writer.write_all(json_string.as_bytes())?;
    //     writer.flush()?;
    //     Ok(())
    // }

    // TODO: Add methods for log verification, export, etc., if handled by this engine.
    // Or these could be CLI-specific functions that use the AuditEngine for data access.
}

// TODO: Add tests for AuditEngine, including:
// - Event serialization.
// - Writing to different log types based on event.
// - Log rotation and retention (if applicable and testable here).
// - Integrity checks for the audit ledger (mocked or with temp files). 