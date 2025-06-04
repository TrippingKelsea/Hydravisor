// src/main.rs

mod cli;
mod config;
mod errors;
mod tui;
// Placeholders for other modules based on design
mod audit_engine;
mod env_manager;
mod mcp;
mod policy;
mod session_manager;
mod ssh_manager;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

use cli::{Cli, Commands as CliCommands};
use config::Config;
use policy::PolicyEngine;
use ssh_manager::SshManager;
use audit_engine::AuditEngine;
use env_manager::EnvironmentManager;
use session_manager::SessionManager;
// use mcp::McpServer; // Will be needed for MCP server
use tui::run_tui; // If TUI is launched from here directly

use tracing::{error, info, Level, warn, debug};
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber with environment filter
    // RUST_LOG=hydravisor=trace,warn (sets hydravisor to trace, others to warn)
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")); // Default to info if RUST_LOG not set

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_max_level(Level::TRACE) // Allow TRACE level if filter permits
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default tracing subscriber failed");

    info!("Starting Hydravisor...");

    // Parse CLI arguments
    let cli_args = Cli::parse();

    // Load configuration
    let config = match Config::load(cli_args.config.as_deref()) {
        Ok(cfg) => Arc::new(cfg), // Wrap in Arc for sharing
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            if cli_args.config.is_some() {
                return Err(e.into());
            }
            warn!("Proceeding with default configuration due to error: {}", e);
            Arc::new(Config::default()) 
        }
    };

    // Override log level from CLI if specified (after config load, CLI takes precedence for this)
    // This needs to re-initialize the global subscriber or adjust its filter.
    // For simplicity now, we set it once. A more dynamic setup could be added.
    // Based on cli_args.log_level and config.logging.level.
    // Current setup: EnvFilter from RUST_LOG, then default 'info'. CLI arg could modify EnvFilter upon init.

    info!("Configuration loaded. Effective log level controlled by RUST_LOG or default.");
    debug!("Loaded app config: {:?}", config);

    // Initialize core components (Order might matter due to dependencies)
    let policy_engine = match PolicyEngine::load(&config) {
        Ok(engine) => Arc::new(engine),
        Err(e) => {
            error!("Failed to initialize Policy Engine: {}", e);
            // Depending on severity, might want to exit or run with restricted functionality
            return Err(e.into()); 
        }
    };
    info!("Policy Engine initialized.");
    debug!("Loaded policy config: {:?}", policy_engine.config);

    let ssh_manager = match SshManager::load(&config) {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            error!("Failed to initialize SSH Manager: {}", e);
            return Err(e.into());
        }
    };
    info!("SSH Manager initialized.");
    debug!("Loaded SSH config: {:?}", ssh_manager.config);

    // AuditEngine might depend on config.logging.log_dir for its paths
    let audit_engine = match AuditEngine::new(&config) {
        Ok(engine) => Arc::new(engine),
        Err(e) => {
            error!("Failed to initialize Audit Engine: {}", e);
            return Err(e.into());
        }
    };
    info!("Audit Engine initialized.");

    let env_manager = match EnvironmentManager::new(&config) {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            error!("Failed to initialize Environment Manager: {}", e);
            return Err(e.into());
        }
    };
    info!("Environment Manager initialized.");

    let session_manager = match SessionManager::new(Arc::clone(&config), Arc::clone(&env_manager), Arc::clone(&policy_engine), Arc::clone(&ssh_manager), Arc::clone(&audit_engine)) {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            error!("Failed to initialize Session Manager: {}", e);
            return Err(e.into());
        }
    };
    info!("Session Manager initialized.");

    // TODO: Initialize MCP Server (will require tokio tasks)
    // let mcp_server = McpServer::start(Arc::clone(&config), /* dispatcher channel */).await?;
    // info!("MCP Server started.");

    // Dispatch based on CLI arguments
    if let Some(command) = cli_args.command {
        info!("Handling CLI command...");
        cli::handle_command(
            command, 
            Arc::clone(&config),
            Arc::clone(&policy_engine),
            // Add other managers as needed by CLI commands
            Arc::clone(&session_manager),
            Arc::clone(&env_manager),
            Arc::clone(&audit_engine),
        ).await?;
    } else if !cli_args.headless {
        info!("No subcommand provided and not headless, launching TUI...");
        // Ensure TUI runs in a blocking manner or main awaits it if TUI itself is async.
        run_tui(
            Arc::clone(&config),
            Arc::clone(&session_manager), 
            Arc::clone(&policy_engine),
            Arc::clone(&env_manager),
            Arc::clone(&audit_engine)
            // Pass other components as needed by the TUI
        )?;
    } else {
        info!("Headless mode, no command. Hydravisor will idle or perform background tasks.");
        // TODO: Implement headless background tasks if any (e.g. MCP server listening)
        // For now, just exits.
        // If MCP server was started, main would need to await its termination signal or run indefinitely.
        println!("Hydravisor running in headless mode. No command given. Exiting.");
    }

    info!("Hydravisor shutting down.");
    Ok(())
}
