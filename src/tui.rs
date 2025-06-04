// src/tui.rs

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{io::{self, Stdout}, sync::Arc, time::{Duration, Instant}};
use chrono::Local;

use crate::config::Config;
use crate::session_manager::SessionManager;
use crate::policy::PolicyEngine;
use crate::env_manager::{EnvironmentManager, EnvironmentStatus};
use crate::audit_engine::AuditEngine;
// use crate::errors::HydraError; // Not used yet

// App state
pub struct App {
    should_quit: bool,
    #[allow(dead_code)] // ollama_models will be replaced or put in a tab soon
    ollama_models: Vec<String>,
    vms: Vec<EnvironmentStatus>,
    vm_list_state: ListState,
    // Shared core components
    #[allow(dead_code)] // Will be used soon
    config: Arc<Config>,
    #[allow(dead_code)] // Will be used soon
    session_manager: Arc<SessionManager>,
    #[allow(dead_code)] // Will be used soon
    policy_engine: Arc<PolicyEngine>,
    #[allow(dead_code)] // Will be used soon
    env_manager: Arc<EnvironmentManager>,
    #[allow(dead_code)] // Will be used soon
    audit_engine: Arc<AuditEngine>,
    // Add other TUI specific state here, e.g., current view, selected item, list states etc.
    // ollama_models_list_state: ListState, // Example for if selection is added
}

impl App {
    pub fn new(
        config: Arc<Config>,
        session_manager: Arc<SessionManager>,
        policy_engine: Arc<PolicyEngine>,
        env_manager: Arc<EnvironmentManager>,
        audit_engine: Arc<AuditEngine>,
    ) -> Self {
        let mut app = Self {
            should_quit: false,
            ollama_models: Vec::new(),
            vms: Vec::new(),
            vm_list_state: ListState::default(),
            config,
            session_manager,
            policy_engine,
            env_manager,
            audit_engine,
            // ollama_models_list_state: ListState::default(), // Init state if used
        };
        app.fetch_ollama_models(); // Load models on init
        app.fetch_vms(); // Load VMs on init
        if !app.vms.is_empty() {
            app.vm_list_state.select(Some(0)); // Select first VM if list is not empty
        }
        app
    }

    fn fetch_ollama_models(&mut self) {
        // Placeholder: In the future, this would interact with a component
        // that can list Ollama models (e.g., via config.providers.ollama)
        self.ollama_models = vec![
            "llama3:latest".to_string(),
            "mistral:7b-instruct-v0.2-q5_K_M".to_string(),
            "codegemma:7b-instruct".to_string(),
            "qwen:7b-chat-q5_K_S".to_string(),
            "starcoder2:3b".to_string(),
        ];
        // TODO: Potentially sort or filter based on config.providers.ollama.models if it's a preferred list
    }

    fn fetch_vms(&mut self) {
        match self.env_manager.list_vms_placeholder() {
            Ok(vms) => self.vms = vms,
            Err(e) => {
                // Log error or set a TUI status message
                eprintln!("Error fetching VMs: {}", e);
                self.vms = Vec::new(); // Clear VMs on error
            }
        }
    }

    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Down => {
                if !self.vms.is_empty() {
                    let i = match self.vm_list_state.selected() {
                        Some(i) => if i >= self.vms.len() - 1 { 0 } else { i + 1 },
                        None => 0,
                    };
                    self.vm_list_state.select(Some(i));
                }
            }
            KeyCode::Up => {
                if !self.vms.is_empty() {
                    let i = match self.vm_list_state.selected() {
                        Some(i) => if i == 0 { self.vms.len() - 1 } else { i - 1 },
                        None => 0,
                    };
                    self.vm_list_state.select(Some(i));
                }
            }
            // TODO: Handle other key presses for navigation (e.g., up/down in lists), actions, etc.
            // KeyCode::Up => self.ollama_models_list_state.select_previous(),
            // KeyCode::Down => self.ollama_models_list_state.select_next(),
            _ => {}
        }
    }

    // Placeholder for periodic updates
    #[allow(dead_code)] // Will be used when background tasks/polling is added
    pub fn on_tick(&mut self) {
        // Update app state based on time, e.g., refresh data
        // self.fetch_ollama_models(); // Example: refresh models periodically if they can change
    }
}

// Main function to run the TUI
pub fn run_tui(
    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
    policy_engine: Arc<PolicyEngine>,
    env_manager: Arc<EnvironmentManager>,
    audit_engine: Arc<AuditEngine>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run the main loop
    let app = App::new(config, session_manager, policy_engine, env_manager, audit_engine);
    let res = run_app_loop(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        // It's useful to print the error to stderr after restoring the terminal,
        // so it's visible if the TUI crashes.
        eprintln!("TUI Error: {}\nCaused by: {:#?}", err, err.root_cause());
        return Err(err);
    }

    Ok(())
}

// Main application loop
fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut app: App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(app.config.interface.refresh_interval_ms); // Use config for tick rate
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let CrosstermEvent::Key(key_event) = event::read()? {
                app.on_key(key_event.code);
            }
            // TODO: Handle mouse events, resize events etc.
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

// Renders the UI
fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let main_layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status Bar
            Constraint::Min(0),    // Main Content Area
            Constraint::Length(3), // Dialog Interface
        ].as_ref())
        .split(f.size());

    // 1. Status Bar
    let status_text = format!(
        "Hydravisor | Mode: {} | Quit: 'q' | VMs: {} | Selected: {:?} | Time: {}",
        app.config.interface.mode,
        app.vms.len(),
        app.vm_list_state.selected(),
        Local::now().format("%H:%M:%S")
    );
    let status_bar = Paragraph::new(status_text)
        .style(Style::default().fg(Color::DarkGray)) // Basic styling
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(status_bar, main_layout_chunks[0]);

    // 2. Main Content Area
    let content_area_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Left Pane (VM List)
            Constraint::Percentage(60), // Right Pane (Details)
        ].as_ref())
        .split(main_layout_chunks[1]);

    // Left Pane: VMs List
    let vm_items: Vec<ListItem> = app.vms
        .iter()
        .map(|vm| {
            let state_str = format!("{:?}", vm.state);
            // Display more info if needed, e.g., vm.ip_address
            ListItem::new(format!("{} ({}) - {}", vm.name, vm.instance_id, state_str))
        })
        .collect();

    let vm_list_widget = List::new(vm_items)
        .block(Block::default().borders(Borders::ALL).title("Virtual Machines (UP/DOWN to select)"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
        .highlight_symbol(">> ");
    
    f.render_stateful_widget(vm_list_widget, content_area_chunks[0], &mut app.vm_list_state);

    // Right Pane: Details of selected VM (Placeholder for now)
    let right_pane_title = "VM Details";
    let mut detail_text = "Select a VM to see details.".to_string();
    if let Some(selected_index) = app.vm_list_state.selected() {
        if let Some(selected_vm) = app.vms.get(selected_index) {
            // right_pane_title = format!("Details for: {}", selected_vm.name);
            detail_text = format!(
                "ID: {}\nName: {}\nState: {:?}\nIP: {}\nCPU Cores: {}\nMax Memory: {} KB\nUsed Memory: {} KB",
                selected_vm.instance_id,
                selected_vm.name,
                selected_vm.state,
                selected_vm.ip_address.as_deref().unwrap_or("N/A"),
                selected_vm.cpu_cores_used.map_or("N/A".to_string(), |c| c.to_string()),
                selected_vm.memory_max_kb.map_or("N/A".to_string(), |m| m.to_string()),
                selected_vm.memory_used_kb.map_or("N/A".to_string(), |m| m.to_string())
            );
        }
    }

    let right_pane_content = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(right_pane_title))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(right_pane_content, content_area_chunks[1]);

    // 3. Dialog Interface (Placeholder for now)
    let dialog_text = if app.should_quit {
        "Quitting Hydravisor..."
    } else {
        "Model Interaction Area / Command Input (Future)"
    };
    let dialog_area = Paragraph::new(dialog_text)
        .block(Block::default().borders(Borders::ALL).title("LLM Dialog / Input"));
    f.render_widget(dialog_area, main_layout_chunks[2]);

    // Example: A popup if quitting (could be a modal later)
    if app.should_quit {
        let area = centered_rect(60, 20, f.size()); // Helper for popup area
        let popup_block = Block::default().title("Confirm Quit").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow));
        let popup_paragraph = Paragraph::new("Quitting Hydravisor... (Press 'q' again to confirm - this is a placeholder)")
            .block(popup_block.clone())
            .wrap(ratatui::widgets::Wrap { trim: true });
        // f.render_widget(Clear, area); // Clear the area for the popup
        f.render_widget(popup_paragraph, area);
    }
}

/// Helper function to create a centered rectangle for popups
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// TODO: Add more UI components (views for sessions, agents, VMs, logs, policy editor)
// TODO: Implement state management for different views and lists (e.g., using ListState)
// TODO: Implement interactions (creating sessions, attaching to VMs, etc.)
// TODO: Focus management between panes/widgets 