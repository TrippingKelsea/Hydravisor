#![allow(clippy::all)] // TEMPORARY: To reduce noise during refactoring
// src/main.rs

mod api;
mod audit;
mod cli;
mod config;
mod libvirt_manager;
mod errors;
mod logging;
mod tui;
// Placeholders for other modules based on design
mod policy;
mod session_manager;
mod ssh_manager;
mod ollama_manager;
#[cfg(feature = "bedrock_integration")]
mod bedrock_manager;

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex; // Use tokio's Mutex
use std::fs::create_dir_all; // For creating log directory

use cli::Cli;
use config::{Config, APP_NAME}; // Import APP_NAME
use policy::PolicyEngine;
use ssh_manager::SshManager;
use audit::AuditEngine;
use libvirt_manager::LibvirtManager;
use session_manager::SessionManager;
use ollama_manager::OllamaManager;
#[cfg(feature = "bedrock_integration")]
use bedrock_manager::BedrockManager;

use tracing::{error, info, warn, debug}; // Removed Level as it's implicitly handled by EnvFilter/macros
use tracing_subscriber::{
    filter::EnvFilter,
    fmt, // For fmt::layer()
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Registry, // Explicitly using Registry as the base
};
use tracing_appender::non_blocking::WorkerGuard; // Specific import for WorkerGuard
use tracing_appender::rolling; // For file logging
use xdg::BaseDirectories; // For log path

// Import for the custom TUI tracing layer and its message type
use crate::tui::tracing_layer::TuiLogCollectorLayer;
use crate::tui::UILogEntry; // For the channel type
use tokio::sync::mpsc; // For the channel

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments first to decide logging strategy
    let cli_args = Cli::parse();

    // Determine if TUI is likely to run
    let tui_mode = cli_args.command.is_none() && !cli_args.headless;

    // Setup XDG directories for log path if needed
    let xdg_dirs = BaseDirectories::with_prefix(APP_NAME)?;
    let log_path = xdg_dirs.get_cache_home(); // Use cache home for logs
    create_dir_all(&log_path)?; // Ensure log directory exists

    // Configure tracing subscriber
    let log_level_str = cli_args.log_level.to_string();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level_str))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // This guard must be kept alive for the duration of the program if file logging is used.
    let mut _file_worker_guard: Option<WorkerGuard> = None;

    let subscriber_registry = Registry::default().with(env_filter);
    let tui_log_rx; // Declare receiver here to be passed to App later

    if tui_mode {
        // Create channel for sending logs to TUI
        let (tx, rx) = mpsc::unbounded_channel::<UILogEntry>();
        tui_log_rx = Some(rx); // Store receiver for TUI

        // File logging layer
        let file_appender = rolling::daily(&log_path, format!("{}.log", APP_NAME));
        let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);
        _file_worker_guard = Some(guard); // Store the guard

        let file_layer = fmt::layer()
            .with_writer(non_blocking_writer)
            .with_ansi(false) // No ANSI colors in file logs
            .json(); // Use JSON format for file logs (requires 'json' feature on tracing-subscriber)

        // Custom TUI log collector layer
        let tui_collector_layer = TuiLogCollectorLayer::new(tx);

        subscriber_registry
            .with(file_layer)
            .with(tui_collector_layer) // Add our custom TUI layer
            .init(); // Initialize the global subscriber

        // Test logs immediately after subscriber initialization
        // ... existing code ...

        // Original info log about TUI mode and log file path
        info!("TUI mode detected. Logging to file and TUI. Log file: {:?}", log_path.join(format!("{}.log", APP_NAME)));
    } else {
        // Standard FmtSubscriber for console output
        tui_log_rx = None; // No receiver in non-TUI mode
        let console_layer = fmt::layer()
            .with_writer(std::io::stderr) // Log to stderr for CLI
            .with_target(true) // Show module paths
            .with_line_number(true); // Show line numbers

        subscriber_registry.with(console_layer).init();
        info!("CLI mode detected. Logging to console.");
    }

    info!("Hydravisor initializing...");

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

    info!("Configuration loaded. Effective log level controlled by RUST_LOG, CLI (--log-level), or default.");
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
    let audit_engine = Arc::new(AuditEngine::new(&config)?);
    info!("Audit Engine initialized.");

    let libvirt_manager = match LibvirtManager::new(&config) {
        Ok(manager) => Arc::new(Mutex::new(manager)),
        Err(e) => {
            error!("Failed to initialize Libvirt Manager: {}", e);
            return Err(e.into());
        }
    };
    info!("Libvirt Manager initialized.");

    let ollama_manager_result = OllamaManager::new(&config).await;
    let ollama_manager = match ollama_manager_result {
        Ok(manager) => {
            info!("Ollama Manager initialized successfully.");
            Arc::new(Mutex::new(manager))
        }
        Err(e) => {
            #[cfg(feature = "ollama_integration")]
            {
                error!("Ollama Manager critical initialization failed (ollama_integration enabled): {}", e);
                return Err(e.into()); // Fatal if feature is on
            }
            #[cfg(not(feature = "ollama_integration"))]
            // This path should be unreachable because OllamaManager::new() for non-featured always returns Ok.
            // If it were to return Err, this would be a panic indicating a logic flaw.
            {
                unreachable!("OllamaManager::new() returned Err when ollama_integration was disabled. Error: {}", e);
            }
        }
    };

    #[cfg(feature = "bedrock_integration")]
    let bedrock_manager = {
        let aws_region = config.providers.bedrock.region.clone();
        match BedrockManager::new(Some(aws_region)).await {
            Ok(manager) => {
                info!("Bedrock Manager initialized.");
                Arc::new(Mutex::new(manager))
            },
            Err(e) => {
                error!("Bedrock Manager initialization failed: {}", e);
                // Non-fatal: the app can run without Bedrock
                // Create a non-functional default or placeholder if necessary
                // For now, we'll proceed, and the TUI will show a disconnected state.
                // A more robust solution might involve a dummy manager.
                // For simplicity, we'll let the app proceed.
                // In a real-world scenario, you might want to handle this more gracefully.
                Arc::new(Mutex::new(BedrockManager::new(None).await?)) // Simplified for now
            }
        }
    };

    let session_manager = match SessionManager::new(Arc::clone(&config), Arc::clone(&libvirt_manager), Arc::clone(&policy_engine), Arc::clone(&ssh_manager), Arc::clone(&audit_engine)) {
        Ok(manager) => Arc::new(manager),
        Err(e) => {
            error!("Failed to initialize Session Manager: {}", e);
            return Err(e.into());
        }
    };
    info!("Session Manager initialized.");

    // McpServer will be initialized and started on demand via CLI or TUI action.

    // Dispatch based on CLI arguments
    if let Some(command) = cli_args.command {
        cli::handle_command(
            command, // CliCommand enum variant
            Arc::clone(&config),
            Arc::clone(&policy_engine),
            // SshManager is not currently taken by handle_command, will add later if needed by subcommands
            // Arc::clone(&ssh_manager),
            Arc::clone(&session_manager),
            Arc::clone(&libvirt_manager),
            Arc::clone(&audit_engine),
            // OllamaManager is not currently taken by handle_command
            // Arc::clone(&ollama_manager),
        )
        .await?;
    } else if !cli_args.headless { // Use cli_args.headless
        // Launch TUI if no subcommand and not headless
        info!("No subcommand provided and not headless, launching TUI.");
        crate::tui::run_tui(
            // No longer passing the handle
            Arc::clone(&config),
            Arc::clone(&session_manager),
            Arc::clone(&policy_engine),
            Arc::clone(&libvirt_manager),
            Arc::clone(&audit_engine),
            Arc::clone(&ollama_manager),
            #[cfg(feature = "bedrock_integration")]
            Arc::clone(&bedrock_manager),
            tui_log_rx.expect("Log receiver should exist in TUI mode"), // Pass receiver
        )
        .await?; // run_tui is now async
    } else {
        info!("No subcommand provided and running in headless mode. Exiting.");
        // Optionally, print help here using clap if no command is given
        // cli::Cli::command().print_help()?;
    }

    info!("Hydravisor shutting down.");
    Ok(())
}
