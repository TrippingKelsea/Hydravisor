// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::config::Config;
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
    #[clap(long, value_name = "LEVEL", value_enum, default_value_t = LogLevelCli::Info)] pub log_level: LogLevelCli,

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
        path: Option<PathBuf> 
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
        log_type: LogType, // As per config.toml.md example `hydravisor logs list --type=vm`
        #[clap(long, default_value_t = 10)]
        limit: usize,
    },
    /// View logs (e.g., .log, .cast, .jsonl)
    View { 
        session_id: String // As per config.toml.md example
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
    // TODO: `audit verify` from config.toml.md (should this be under `log audit verify` or `policy audit verify` or just `audit verify`? -> leaning towards a top-level `audit` command)
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogLevelCli {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogType {
    Vm,
    Container,
    System, // For general system logs
    Mcp,    // For MCP activity
    Audit,  // For the audit ledger
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum LogFormat {
    Cast,
    Jsonl,
    Ansi, // Raw ANSI
}

pub fn handle_command(cli_args: Cli, config: &Config) -> Result<()> {
    if let Some(command) = cli_args.command {
        match command {
            Commands::Policy(policy_cmd) => handle_policy_command(policy_cmd, config)?,
            Commands::Agent(agent_cmd) => handle_agent_command(agent_cmd, config)?,
            Commands::Vm(vm_cmd) => handle_vm_command(vm_cmd, config)?,
            Commands::Log(log_cmd) => handle_log_command(log_cmd, config)?,
        }
    } else if cli_args.headless {
        // Headless mode, no TUI, no specific command. What to do?
        // Perhaps run a default background service if that's a use case?
        // Or simply print help.
        println!("Running in headless mode with no command. Hydravisor might perform background tasks or await MCP connections if configured.");
        // For now, just indicate it's headless.
        todo!("Define behavior for headless mode without specific command.")
    }
    // If command is None and not headless, main.rs handles TUI launch.
    Ok(())
}

fn handle_policy_command(command: PolicyCommands, config: &Config) -> Result<()> {
    match command {
        PolicyCommands::Validate { path } => {
            println!("Policy validate command: Path: {:?}, Config: {:?}", path, config);
            // TODO: Implement policy validation logic from policy_cli_tools.md
            // 1. Determine policy file path (use `path` or default from config/XDG)
            // 2. Load policy.toml
            // 3. Load policy.schema.json (how to bundle/locate this?)
            // 4. Validate using a JSON schema validator crate (e.g. jsonschema)
            todo!("Implement policy validation");
        }
        PolicyCommands::Check { agent_id, vm_id, action } => {
            println!("Policy check command: Agent: {}, VM: {}, Action: {}, Config: {:?}", agent_id, vm_id, action, config);
            // TODO: Implement policy check simulation from policy_cli_tools.md
            // 1. Load policy.toml
            // 2. Simulate authorization
            todo!("Implement policy check");
        }
    }
    // Ok(())
}

fn handle_agent_command(command: AgentCommands, config: &Config) -> Result<()> {
    match command {
        AgentCommands::List => {
            println!("Agent list command, Config: {:?}", config);
            todo!("Implement agent list");
        }
        AgentCommands::Info { agent_id } => {
            println!("Agent info command: AgentID: {}, Config: {:?}", agent_id, config);
            todo!("Implement agent info");
        }
    }
    // Ok(())
}

fn handle_vm_command(command: VmCommands, config: &Config) -> Result<()> {
    match command {
        VmCommands::List => {
            println!("VM list command, Config: {:?}", config);
            todo!("Implement VM list");
        }
        VmCommands::Info { vm_id } => {
            println!("VM info command: VM_ID: {}, Config: {:?}", vm_id, config);
            todo!("Implement VM info");
        }
        VmCommands::Snapshot { vm_id, output } => {
            println!("VM snapshot command: VM_ID: {}, Output: {:?}, Config: {:?}", vm_id, output, config);
            todo!("Implement VM snapshot");
        }
    }
    // Ok(())
}

fn handle_log_command(command: LogCommands, config: &Config) -> Result<()> {
    match command {
        LogCommands::List { log_type, limit } => {
            println!("Log list command: Type: {:?}, Limit: {}, Config: {:?}", log_type, limit, config);
            todo!("Implement log list");
        }
        LogCommands::View { session_id } => {
            println!("Log view command: SessionID: {}, Config: {:?}", session_id, config);
            todo!("Implement log view");
        }
        LogCommands::Export { session_id, format, output } => {
            println!("Log export command: SessionID: {}, Format: {:?}, Output: {:?}, Config: {:?}", session_id, format, output, config);
            todo!("Implement log export");
        }
    }
    // Ok(())
}

// TODO: Add tests for CLI parsing 