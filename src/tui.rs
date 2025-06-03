// src/tui.rs

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::errors::HydraError;

// Main TUI application state struct
pub struct App {
    // TODO: Add TUI state fields, e.g., current view, selected items, etc.
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        App { should_quit: false }
    }

    // TODO: Methods to handle input, update state, and draw UI components
    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {
        // TODO: Handle periodic updates, e.g., refreshing data
    }
}

pub fn run_tui(config: &Config) -> Result<()> {
    enable_raw_mode().map_err(|e| HydraError::TuiError(format!("Failed to enable raw mode: {}", e)))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| HydraError::TuiError(format!("Failed to enter alternate screen: {}", e)))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app_loop(&mut terminal, &mut app, config);

    // Restore terminal
    disable_raw_mode().map_err(|e| HydraError::TuiError(format!("Failed to disable raw mode: {}", e)))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)
        .map_err(|e| HydraError::TuiError(format!("Failed to leave alternate screen: {}", e)))?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("TUI Error: {}", err);
        // Convert HydraError to anyhow::Error for the main function's signature
        return Err(err.into()); 
    }

    Ok(())
}

fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    _config: &Config, // Pass config if App needs it
) -> Result<(), HydraError> {
    let tick_rate = Duration::from_millis(250); // TODO: Make configurable from app_config.interface.refresh_interval_ms
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, app)).map_err(|e| HydraError::TuiError(format!("Failed to draw: {}",e)))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout).map_err(|e| HydraError::TuiError(format!("Event poll error: {}",e)))? {
            if let CEvent::Key(key_event) = event::read().map_err(|e| HydraError::TuiError(format!("Event read error: {}",e)))? {
                if key_event.kind == event::KeyEventKind::Press {
                    app.on_key(key_event.code);
                }
            }
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

// This is a placeholder UI drawing function
fn ui(f: &mut ratatui::Frame, app: &App) {
    // Based on tui.design.md, we need a layout with:
    // 1. Status Bar (top)
    // 2. Main Area (VM/Container List | Detail View Panel)
    // 3. Dialog Interface (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status Bar
            Constraint::Min(0),    // Main Content Area
            Constraint::Length(5), // Dialog Interface
        ])
        .split(f.size());

    let status_bar = Paragraph::new(format!("Status Bar - Mode: Session | Connected Model: None | Time: {} | Notifications: 0. Press 'q' to quit.", chrono::Local::now().format("%H:%M:%S")))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status_bar, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // VM/Container List
            Constraint::Percentage(70), // Detail View Panel
        ])
        .split(chunks[1]);

    let vm_list = Paragraph::new("VM/Container List Pane\nItem 1\nItem 2")
        .block(Block::default().borders(Borders::ALL).title("Instances"));
    f.render_widget(vm_list, main_chunks[0]);

    let detail_view = Paragraph::new(format!("Detail View Panel\nCurrently showing: Info Summary\nApp should_quit: {}", app.should_quit))
        .block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(detail_view, main_chunks[1]);

    let dialog_interface = Paragraph::new("Dialog Interface with Model\n[User Input Area]")
        .block(Block::default().borders(Borders::ALL).title("Model Chat"));
    f.render_widget(dialog_interface, chunks[2]);
    
    // TODO: Implement actual UI components, state handling, and views as per tui.design.md
}

// TODO: Add tests for TUI components and state transitions (might require a TUI testing library or careful mocking) 