// src/audit_engine.rs

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs;
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
    // Configuration for log paths, formats, retention, etc.
    // config: AuditEngineConfig, (derived from main Config)
    // Handle for writing to audit logs (e.g., file, network stream)
    // writer: Mutex<Option<Box<dyn Write + Send>>>
    log_path: PathBuf, // Path to the main audit ledger file
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
            log_path: log_file_path,
        })
    }

    pub fn record_event(&self, event: AuditEvent) -> Result<()> {
        // TODO: Implement event recording logic:
        // 1. Serialize the event (e.g., to JSONL).
        // 2. Write to the appropriate log file(s) based on event_type and risk_level.
        //    - System Logs: `~/.hydravisor/logs/system.log` (plaintext)
        //    - VM & Container Lifecycle Logs: `~/.hydravisor/logs/instances/{id}/lifecycle.log` (JSONL)
        //    - tmux Session Recordings: Handled by session_manager, but audit event logged here.
        //    - MCP Activity Logs: `~/.hydravisor/logs/mcp/mcp_activity.jsonl` (JSONL)
        //    - Audit Ledger: `~/.hydravisor/logs/audit/audit_ledger.jsonl` (hash-chained JSONL)
        // 3. Implement integrity strategies (hash-chaining for audit_ledger).
        // 4. Handle potential I/O errors.
        println!("Recording audit event: {:?}", event);
        todo!("Implement actual event recording to different log files/formats based on event type and config.");
        // Ok(())
    }

    // TODO: Add methods for log verification, export, etc., if handled by this engine.
    // Or these could be CLI-specific functions that use the AuditEngine for data access.
}

// TODO: Add tests for AuditEngine, including:
// - Event serialization.
// - Writing to different log types based on event.
// - Log rotation and retention (if applicable and testable here).
// - Integrity checks for the audit ledger (mocked or with temp files). 