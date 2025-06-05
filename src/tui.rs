// src/tui.rs

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc};
use tokio::sync::{mpsc, Mutex};

// New module organization
pub mod app;
pub mod events;
pub mod theme;
pub mod tracing_layer;
pub mod ui;
pub mod widgets;

// Re-export necessary components
pub use app::{App, UILogEntry};
use events::run_app_loop;

// Import necessary concrete types for the function signature
use crate::{
    audit_engine::AuditEngine,
    config::Config,
    env_manager::EnvironmentManager,
    ollama_manager::OllamaManager,
    policy::PolicyEngine,
    session_manager::SessionManager,
};

/// Main function to run the TUI.
///
/// This function initializes the terminal, creates the `App` state,
/// and enters the main event loop. It's responsible for restoring
/// the terminal state when the application exits.
pub fn run_tui(
    rt: &tokio::runtime::Handle, // Accept a handle to the existing runtime
    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
    policy_engine: Arc<PolicyEngine>,
    env_manager: Arc<Mutex<EnvironmentManager>>,
    audit_engine: Arc<AuditEngine>,
    ollama_manager: Arc<Mutex<OllamaManager>>,
    log_receiver: mpsc::UnboundedReceiver<UILogEntry>,
) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app
    let app = App::new(
        config,
        session_manager,
        policy_engine,
        env_manager,
        audit_engine,
        ollama_manager,
        log_receiver,
    );

    // run app loop
    let res = rt.block_on(run_app_loop(&mut terminal, app));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err)
    }

    Ok(())
} 