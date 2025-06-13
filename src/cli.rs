// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::policy::PolicyEngine;
use crate::session_manager::SessionManager;
use crate::libvirt_manager::LibvirtManager;
use crate::audit_engine::AuditEngine;

use anyhow::Result;

/// Hydravisor: AI Agent Sandbox Manager
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Optional path to the Hydravisor configuration file
    #[clap(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Suppress TUI auto-launch (e.g., for scripting or headless operation)
    #[clap(long)]
    pub headless: bool,

    /// Set log level
    #[clap(long, value_name = "LEVEL", value_enum, default_value_t = LogLevelCli::Info)]
    pub log_level: LogLevelCli,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage policies
    #[clap(subcommand)]
    Policy(PolicyCommands),

    /// Manage agents
    #[clap(subcommand)]
    Agent(AgentCommands),

    /// Manage VMs and Containers
    #[clap(subcommand)]
    Vm(VmCommands),

    /// Manage logs
    #[clap(subcommand)]
    Log(LogCommands),
    // TODO: Add `store` subcommand for encrypted disk management as per cli.design.md
}

#[derive(Subcommand, Debug)]
pub enum PolicyCommands {
    /// Validate the policy.toml file
    Validate {
        /// Optional path to the policy file
        #[clap(long, value_name = "FILE")]
        path: Option<PathBuf>,
    },
    /// Simulate an authorization decision
    Check {
        #[clap(long)]
        agent_id: String,
        #[clap(long)]
        vm_id: String,
        #[clap(long)] // TODO: Make this an enum based on defined actions
        action: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// List all configured/active agents
    List,
    /// Show status and policy bindings for an agent
    Info {
        agent_id: String
    },
    // TODO: `agent promote <id>` as per cli.design.md (future)
}

#[derive(Subcommand, Debug)]
pub enum VmCommands {
    /// List known VM sessions or configurations
    List,
    /// Show VM state, logs, and bindings
    Info {
        vm_id: String
    },
    /// Export current VM as an archive
    Snapshot {
        vm_id: String,
        #[clap(long, short, value_name = "FILE")]
        output: PathBuf,
    },
    // TODO: `vm create`, `vm delete` from config.toml.md
}

#[derive(Subcommand, Debug)]
pub enum LogCommands {
    /// Show available session logs
    List {
        #[clap(long, value_enum, default_value_t = LogType::Vm)]
        log_type: LogType,
        #[clap(long, default_value_t = 10)]
        limit: usize,
    },
    /// View logs (e.g., .log, .cast, .jsonl)
    View {
        session_id: String
    },
    /// Export logs to a target directory or convert to playback format
    Export {
        session_id: String,
        #[clap(long, short, value_enum)]
        format: LogFormat,
        #[clap(long, short, value_name = "DIR_OR_FILE")]
        output: PathBuf,
    }
    // TODO: `log replay` (future)
    // TODO: `audit verify` from config.toml.md
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogLevelCli {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevelCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevelCli::Trace => write!(f, "trace"),
            LogLevelCli::Debug => write!(f, "debug"),
            LogLevelCli::Info => write!(f, "info"),
            LogLevelCli::Warn => write!(f, "warn"),
            LogLevelCli::Error => write!(f, "error"),
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogType {
    Vm,
    Container,
    System,
    Mcp,
    Audit,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogFormat {
    Cast,
    Jsonl,
    Ansi,
}

pub async fn handle_command(
    command: Commands, // Now it's guaranteed to be Some by main.rs
    config: Arc<Config>,
    policy_engine: Arc<PolicyEngine>,
    session_manager: Arc<SessionManager>,
    libvirt_manager: Arc<Mutex<LibvirtManager>>,
    audit_engine: Arc<AuditEngine>,
) -> Result<()> {
    match command {
        Commands::Policy(policy_cmd) => handle_policy_command(policy_cmd, config, policy_engine).await?,
        Commands::Agent(agent_cmd) => handle_agent_command(agent_cmd, config, session_manager).await?,
        Commands::Vm(vm_cmd) => handle_vm_command(vm_cmd, config, libvirt_manager).await?,
        Commands::Log(log_cmd) => handle_log_command(log_cmd, config, audit_engine).await?,
    }
    Ok(())
}

async fn handle_policy_command(
    command: PolicyCommands, 
    _config: Arc<Config>, // Renamed to avoid unused warning for now
    policy_engine: Arc<PolicyEngine>
) -> Result<()> {
    match command {
        PolicyCommands::Validate { path } => {
            println!("Policy validate command: Path to validate explicitly: {:?}", path);

            // 1. Determine the policy content to validate.
            let (policy_value, policy_source_description): (serde_json::Value, String) = if let Some(p) = path {
                let policy_str = std::fs::read_to_string(&p)
                    .map_err(|e| anyhow::anyhow!("Failed to read policy file {:?}: {}", p, e))?;
                let parsed_policy_toml: toml::Value = toml::from_str(&policy_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse TOML from policy file {:?}: {}", p, e))?;
                // Convert toml::Value to serde_json::Value for jsonschema validation
                let json_value = serde_json::to_value(parsed_policy_toml)?;
                (json_value, format!("file '{}'", p.display()))
            } else if let Some(loaded_path_str) = &policy_engine.config.source_path {
                // If no path override, validate the currently loaded policy.
                // Need to re-serialize PolicyConfig to toml::Value then to serde_json::Value, 
                // or find a more direct way if PolicyConfig can be directly validated (if it derives Serialize for jsonschema)
                // For now, let's assume we need to get it as a Value.
                // This is a bit convoluted; ideally, PolicyConfig itself could be validated if its structure matches the schema directly.
                let policy_as_toml_value = toml::Value::try_from(&policy_engine.config)?;
                let json_value = serde_json::to_value(policy_as_toml_value)?;
                (json_value, format!("currently loaded policy from '{}'", loaded_path_str))
            } else {
                anyhow::bail!("No policy file specified for validation and no policy file was loaded initially.");
            };

            // 2. Load the JSON schema.
            // TODO: Consider embedding the schema using include_str! for robustness.
            let schema_path = PathBuf::from("technical_design/policy.schema.json");
            let schema_str = std::fs::read_to_string(&schema_path)
                .map_err(|e| anyhow::anyhow!("Failed to read policy schema file {:?}: {}", schema_path, e))?;
            let schema_json: serde_json::Value = serde_json::from_str(&schema_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse policy schema JSON from {:?}: {}", schema_path, e))?;
            
            let compiled_schema = jsonschema::JSONSchema::compile(&schema_json)
                .map_err(|e| anyhow::anyhow!("Failed to compile policy JSON schema: {}", e))?;

            // 3. Validate the policy content against the schema.
            match compiled_schema.validate(&policy_value) {
                Ok(_) => {
                    println!("SUCCESS: Policy from {} is valid against the JSON schema.", policy_source_description);
                    // TODO: Add internal consistency checks from PolicyEngine if needed for the validated content.
                    // For example: `PolicyEngine::validate_internal_consistency(parsed_policy_config_if_loaded_from_path)?`
                }
                Err(errors) => {
                    let error_messages: Vec<String> = errors.map(|e| format!("  - {}", e)).collect();
                    anyhow::bail!("ERROR: Policy from {} is INVALID against the JSON schema:\n{}", policy_source_description, error_messages.join("\n"));
                }
            }
            // Ok(())
            todo!("Refine validation output and add internal consistency checks via PolicyEngine method.");
        }
        PolicyCommands::Check { agent_id, vm_id, action } => {
            println!("Policy check command: Agent: {}, VM: {}, Action: {}", agent_id, vm_id, action);
            // TODO: Implement policy check simulation from policy_cli_tools.md
            // 1. Load policy.toml (already available in policy_engine.config)
            // 2. Simulate authorization using policy_engine.check_permission()
            //    - Construct an AuthRequest.
            //    - Print the AuthDecision.
            todo!("Implement policy check simulation using PolicyEngine.");
        }
    }
    // Ok(())
}

async fn handle_agent_command(
    command: AgentCommands,
    _config: Arc<Config>,
    _session_manager: Arc<SessionManager> // Added, marked unused for now
) -> Result<()> {
    match command {
        AgentCommands::List => {
            println!("List agents command");
            // TODO: Fetch from SessionManager
        }
        AgentCommands::Info { agent_id } => {
            println!("Agent info command for: {}", agent_id);
            // TODO: Fetch from SessionManager
        }
    }
    Ok(())
}

async fn handle_vm_command(
    command: VmCommands,
    _config: Arc<Config>,
    libvirt_manager: Arc<Mutex<LibvirtManager>>, // Added, marked unused for now
) -> Result<()> {
    match command {
        VmCommands::List => {
            println!("Listing all known VMs...");
            let libvirt_manager_guard = libvirt_manager.lock().await;
            let vms = libvirt_manager_guard.list_vms()?;
            if vms.is_empty() {
                println!("No VMs found.");
            } else {
                // TODO: Replace with a proper table using a crate like `prettytable-rs`
                println!("{:<38} {:<25} {:<12} {:<10}", "ID", "NAME", "STATE", "CORES");
                for vm in vms {
                    println!(
                        "{:<38} {:<25} {:<12?} {:<10}",
                        vm.instance_id,
                        vm.name,
                        vm.state,
                        vm.cpu_cores_used.map_or_else(|| "N/A".to_string(), |c| c.to_string())
                    );
                }
            }
        }
        VmCommands::Info { vm_id } => {
            println!("VM info command for: {}", vm_id);
            // TODO: Fetch from EnvManager and format output
        }
        VmCommands::Snapshot { vm_id, output } => {
            println!("VM snapshot command for: {}, Output: {:?}", vm_id, output);
            // TODO: Call EnvManager snapshot method
        }
    }
    Ok(())
}

async fn handle_log_command(
    command: LogCommands,
    _config: Arc<Config>,
    _audit_engine: Arc<AuditEngine> // Added, marked unused for now
) -> Result<()> {
    match command {
        LogCommands::List { log_type, limit } => {
            println!("Log list command: Type: {:?}, Limit: {}", log_type, limit);
            // TODO: Use _audit_engine
            todo!("Implement log list - requires AuditEngine or direct log file access logic");
        }
        LogCommands::View { session_id } => {
            println!("Log view command: SessionID: {}", session_id);
            // TODO: Use _audit_engine
            todo!("Implement log view - requires AuditEngine or direct log file access logic");
        }
        LogCommands::Export { session_id, format, output } => {
            println!("Log export command: SessionID: {}, Format: {:?}, Output: {:?}", session_id, format, output);
            // TODO: Use _audit_engine
            todo!("Implement log export - requires AuditEngine or direct log file access logic");
        }
    }
    // Ok(())
}

// TODO: Add tests for CLI parsing and command handling (mocking components)

// TODO: Add tests for CLI parsing 