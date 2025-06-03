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
use cli::Cli;
use config::Config;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    // Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO) // Default level, can be overridden by config or CLI arg
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default tracing subscriber failed");

    info!("Starting Hydravisor...");

    // Parse CLI arguments
    let cli_args = Cli::parse();

    // Load configuration
    // The path can be overridden by cli_args.config
    let config = match Config::load(cli_args.config.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            // Optionally, load with default values or exit
            // For now, let's try to proceed with default if user didn't specify a config
            // or exit if a specified config failed to load.
            if cli_args.config.is_some() {
                return Err(e.into());
            }
            eprintln!("Warning: Failed to load configuration, proceeding with defaults: {}", e);
            Config::default() // Assuming Config::default() is implemented
        }
    };
    
    // TODO: Adjust log level based on config here if necessary
    // Example: Override FmtSubscriber if config.logging.level is different

    // Dispatch based on CLI arguments
    // If no subcommand, and not headless, launch TUI.
    // Otherwise, handle CLI command.
    if cli_args.command.is_none() && !cli_args.headless {
        info!("No subcommand provided and not headless, launching TUI...");
        // tui::run_tui(&config)?; // TODO: Implement TUI entry point
        println!("TUI would run here with config: {:?}", config);
        todo!("Implement TUI launch");
    } else {
        info!("Handling CLI command or running headless...");
        // cli::handle_command(cli_args, &config)?; // TODO: Implement CLI command handler
        println!("CLI command would be handled here: {:?}, with config: {:?}", cli_args, config);
        todo!("Implement CLI command handling");
    }

    // Ok(())
}
