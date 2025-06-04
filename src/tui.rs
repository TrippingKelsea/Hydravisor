// src/tui.rs

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::ListState,
    Terminal,
};
use std::{io::{self, Stdout}, sync::Arc, time::{Duration, Instant}};
use chrono::Local;

use crate::config::Config;
use crate::session_manager::SessionManager;
use crate::policy::PolicyEngine;
use crate::env_manager::{EnvironmentManager, EnvironmentStatus};
use crate::audit_engine::AuditEngine;
use crate::ollama_manager::OllamaManager;

#[cfg(feature = "ollama_integration")]
use ollama_rs::models::LocalModel;

#[cfg(feature = "ollama_integration")]
use futures::executor::block_on;
#[cfg(feature = "ollama_integration")]
use tokio::runtime::Handle;

// tui-logger imports
use tui_logger::{TuiWidgetState, TuiWidgetEvent};
// No separate import for tui_logger::Input; will use full path.

pub mod widgets; // Added widgets submodule declaration
use self::widgets::status_bar::StatusBarWidget; // Use the new widget
use self::widgets::input_bar::InputBarWidget;   // Use the new widget
use self::widgets::vm_list::VmListWidget;
use self::widgets::ollama_model_list::OllamaModelListWidget;
use self::widgets::chat::ChatWidget;
use self::widgets::logs::LogsWidget;

// Define different views for the TUI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiView {
    VmList,
    OllamaModelList,
    Chat,
    Logs,
}

// Define input modes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

// Represents a chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub timestamp: String,
}

// Represents an active chat session
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub model_name: String,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
}

pub struct App {
    should_quit: bool,
    
    #[cfg(feature = "ollama_integration")]
    ollama_models: Vec<LocalModel>,
    #[cfg(not(feature = "ollama_integration"))]
    ollama_models: Vec<String>,

    vms: Vec<EnvironmentStatus>,
    vm_list_state: ListState,
    
    #[cfg(feature = "ollama_integration")]
    ollama_model_list_state: ListState,

    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
    policy_engine: Arc<PolicyEngine>,
    env_manager: Arc<EnvironmentManager>,
    audit_engine: Arc<AuditEngine>,
    ollama_manager: Arc<OllamaManager>,

    active_view: TuiView,
    input_mode: InputMode,
    current_input: String,
    active_chat: Option<ChatSession>,
    log_widget_state: TuiWidgetState,
}

impl App {
    pub fn new(
        config: Arc<Config>,
        session_manager: Arc<SessionManager>,
        policy_engine: Arc<PolicyEngine>,
        env_manager: Arc<EnvironmentManager>,
        audit_engine: Arc<AuditEngine>,
        ollama_manager: Arc<OllamaManager>,
    ) -> Self {
        let log_widget_state = TuiWidgetState::new();
        
        let mut app = Self {
            should_quit: false,
            ollama_models: Vec::new(),
            vms: Vec::new(),
            vm_list_state: ListState::default(),
            #[cfg(feature = "ollama_integration")]
            ollama_model_list_state: ListState::default(),
            config,
            session_manager,
            policy_engine,
            env_manager,
            audit_engine,
            ollama_manager: Arc::clone(&ollama_manager),
            active_view: TuiView::VmList,
            input_mode: InputMode::Normal,
            current_input: String::new(),
            active_chat: None,
            log_widget_state,
        };

        #[cfg(feature = "ollama_integration")]
        {
            let ollama_manager_clone = Arc::clone(&ollama_manager);
            if let Ok(handle) = Handle::try_current() {
                 let models_future = async move { ollama_manager_clone.list_local_models().await };
                 match block_on(handle.spawn(models_future)) {
                    Ok(Ok(models)) => app.ollama_models = models,
                    Ok(Err(e)) => tracing::error!("Error fetching Ollama models on init (async task error): {}", e),
                    Err(e) => tracing::error!("Error fetching Ollama models on init (spawn error): {}", e),
                }
            } else {
                tracing::error!("Error: Not in a Tokio runtime context. Cannot fetch Ollama models for TUI.");
                 app.ollama_models = vec![LocalModel{name: "Tokio runtime error - No models loaded".to_string(), modified_at: "N/A".to_string(), size: 0 }];
            }
            if !app.ollama_models.is_empty() {
                app.ollama_model_list_state.select(Some(0));
            }
        }
        #[cfg(not(feature = "ollama_integration"))]
        {
            app.ollama_models.push("Ollama integration disabled.".to_string());
        }
        app.fetch_vms();
        if !app.vms.is_empty() {
            app.vm_list_state.select(Some(0));
        }
        app
    }

    #[cfg(feature = "ollama_integration")]
    #[allow(dead_code)]
    async fn _fetch_ollama_models_async(ollama_manager: Arc<OllamaManager>) -> Result<Vec<LocalModel>> {
        match ollama_manager.list_local_models().await {
            Ok(models) => Ok(models),
            Err(e) => {
                tracing::error!("Error fetching Ollama models: {}", e);
                Ok(Vec::new()) 
            }
        }
    }

    fn fetch_vms(&mut self) {
        match self.env_manager.list_vms() { 
            Ok(vms) => self.vms = vms,
            Err(e) => {
                tracing::error!("Error fetching VMs: {}", e);
                self.vms = Vec::new();
            }
        }
    }

    pub fn on_key(&mut self, key_event: KeyEvent) {
        let mut event_consumed = false;

        if self.active_view == TuiView::Logs && self.input_mode == InputMode::Normal {
            // Global keys like 'q' for quit and 'Tab' for view switching are handled first.
            match key_event.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    event_consumed = true;
                }
                KeyCode::Tab => {
                    self.active_view = TuiView::VmList; // Cycle from Logs to VmList
                    self.vm_list_state.select(if self.vms.is_empty() { None } else { Some(0) });
                    #[cfg(feature = "ollama_integration")]
                    self.ollama_model_list_state.select(if self.ollama_models.is_empty() { None } else { Some(0) });
                    event_consumed = true;
                }
                // Map keys to TuiWidgetEvent for tui-logger state transition
                KeyCode::Up => {
                    self.log_widget_state.transition(&TuiWidgetEvent::UpKey);
                    event_consumed = true;
                }
                KeyCode::Down => {
                    self.log_widget_state.transition(&TuiWidgetEvent::DownKey);
                    event_consumed = true;
                }
                KeyCode::PageUp => {
                    self.log_widget_state.transition(&TuiWidgetEvent::PrevPageKey);
                    event_consumed = true;
                }
                KeyCode::PageDown => {
                    self.log_widget_state.transition(&TuiWidgetEvent::NextPageKey);
                    event_consumed = true;
                }
                KeyCode::Left => {
                    self.log_widget_state.transition(&TuiWidgetEvent::LeftKey);
                    event_consumed = true;
                }
                KeyCode::Right => {
                    self.log_widget_state.transition(&TuiWidgetEvent::RightKey);
                    event_consumed = true;
                }
                KeyCode::Char('h') => {
                    self.log_widget_state.transition(&TuiWidgetEvent::HideKey);
                    event_consumed = true;
                }
                KeyCode::Char('f') => {
                    self.log_widget_state.transition(&TuiWidgetEvent::FocusKey);
                    event_consumed = true;
                }
                KeyCode::Char('-') => {
                    self.log_widget_state.transition(&TuiWidgetEvent::MinusKey);
                    event_consumed = true;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => { // '=' is often unshifted '+'
                    self.log_widget_state.transition(&TuiWidgetEvent::PlusKey);
                    event_consumed = true;
                }
                KeyCode::Esc => {
                     self.log_widget_state.transition(&TuiWidgetEvent::EscapeKey);
                     event_consumed = true;
                }
                KeyCode::Char(' ') => {
                    self.log_widget_state.transition(&TuiWidgetEvent::SpaceKey);
                    event_consumed = true;
                }
                // KeyCode::Char('?') for ToggleHelpKey - check if modifiers are involved
                // KeyCode::Char('d') for ToggleDisplayModeKey

                // TODO: Add mappings for ToggleHelpKey ('?') and ToggleDisplayModeKey ('d')
                // For now, unmapped keys fall through.
                _ => {
                    // event_consumed remains false, will be handled by general input logic if any
                }
            }
        }

        if !event_consumed {
            // General key handling for other views or modes
            match self.input_mode {
                InputMode::Normal => {
                    match key_event.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('i') => {
                            if self.active_view == TuiView::Chat && self.active_chat.is_some() {
                                self.input_mode = InputMode::Editing;
                            }
                        }
                        KeyCode::Tab => {
                            self.active_view = match self.active_view {
                                TuiView::VmList => TuiView::OllamaModelList,
                                TuiView::OllamaModelList => TuiView::Chat,
                                TuiView::Chat => TuiView::Logs,
                                TuiView::Logs => TuiView::VmList, // Should have been handled above
                            };
                            self.vm_list_state.select(if self.vms.is_empty() { None } else { Some(0) });
                            #[cfg(feature = "ollama_integration")]
                            self.ollama_model_list_state.select(if self.ollama_models.is_empty() { None } else { Some(0) });
                        }
                        KeyCode::Down => match self.active_view {
                            TuiView::VmList => self.select_next_item_in_vm_list(),
                            TuiView::OllamaModelList => self.select_next_item_in_ollama_list(),
                            _ => {} 
                        },
                        KeyCode::Up => match self.active_view {
                            TuiView::VmList => self.select_previous_item_in_vm_list(),
                            TuiView::OllamaModelList => self.select_previous_item_in_ollama_list(),
                            _ => {} 
                        },
                        KeyCode::Enter => {
                            if self.active_view == TuiView::OllamaModelList {
                                #[cfg(feature = "ollama_integration")]
                                if let Some(selected_idx) = self.ollama_model_list_state.selected() {
                                    if let Some(model) = self.ollama_models.get(selected_idx) {
                                        self.active_chat = Some(ChatSession {
                                            model_name: model.name.clone(),
                                            messages: Vec::new(),
                                            is_streaming: false,
                                        });
                                        self.active_view = TuiView::Chat;
                                        self.input_mode = InputMode::Editing;
                                        self.current_input.clear();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                InputMode::Editing => {
                    match key_event.code {
                        KeyCode::Enter => {
                            if self.active_view == TuiView::Chat && self.active_chat.is_some() {
                                let prompt_text = self.current_input.drain(..).collect::<String>();
                                if !prompt_text.is_empty() {
                                    if let Some(chat_session) = &mut self.active_chat {
                                        chat_session.messages.push(ChatMessage {
                                            sender: "user".to_string(),
                                            content: prompt_text.clone(),
                                            timestamp: Local::now().to_rfc3339(),
                                        });
                                        chat_session.is_streaming = true;
                                        let model_name_clone = chat_session.model_name.clone();
                                        chat_session.messages.push(ChatMessage {
                                            sender: model_name_clone,
                                            content: format!("Echo (TODO: actual Ollama call): {}", prompt_text),
                                            timestamp: Local::now().to_rfc3339(),
                                        });
                                        chat_session.is_streaming = false;
                                    }
                                }
                            }
                        }
                        KeyCode::Char(c) => self.current_input.push(c),
                        KeyCode::Backspace => { self.current_input.pop(); }
                        KeyCode::Esc => {
                            self.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    fn select_next_item_in_vm_list(&mut self) {
        if !self.vms.is_empty() {
            let i = match self.vm_list_state.selected() {
                Some(i) => if i >= self.vms.len() - 1 { 0 } else { i + 1 },
                None => 0,
            };
            self.vm_list_state.select(Some(i));
        }
    }

    fn select_previous_item_in_vm_list(&mut self) {
        if !self.vms.is_empty() {
            let i = match self.vm_list_state.selected() {
                Some(i) => if i == 0 { self.vms.len() - 1 } else { i - 1 },
                None => 0,
            };
            self.vm_list_state.select(Some(i));
        }
    }

    #[cfg(feature = "ollama_integration")]
    fn select_next_item_in_ollama_list(&mut self) {
        if !self.ollama_models.is_empty() {
            let i = match self.ollama_model_list_state.selected() {
                Some(i) => if i >= self.ollama_models.len() - 1 { 0 } else { i + 1 },
                None => 0,
            };
            self.ollama_model_list_state.select(Some(i));
        }
    }

    #[cfg(feature = "ollama_integration")]
    fn select_previous_item_in_ollama_list(&mut self) {
        if !self.ollama_models.is_empty() {
            let i = match self.ollama_model_list_state.selected() {
                Some(i) => if i == 0 { self.ollama_models.len() - 1 } else { i - 1 },
                None => 0,
            };
            self.ollama_model_list_state.select(Some(i));
        }
    }
    #[cfg(not(feature = "ollama_integration"))]
    fn select_next_item_in_ollama_list(&mut self) { }
    #[cfg(not(feature = "ollama_integration"))]
    fn select_previous_item_in_ollama_list(&mut self) { }

    pub fn on_tick(&mut self) {
    }
}

pub fn run_tui(
    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
    policy_engine: Arc<PolicyEngine>,
    env_manager: Arc<EnvironmentManager>,
    audit_engine: Arc<AuditEngine>,
    ollama_manager: Arc<OllamaManager>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(config, session_manager, policy_engine, env_manager, audit_engine, ollama_manager);
    let res = run_app_loop(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        tracing::error!("TUI Error: {}\nCaused by: {:#?}", err, err.root_cause());
        return Err(err);
    }
    Ok(())
}

fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut app: App,
) -> Result<()> {
    let tick_rate = Duration::from_millis(app.config.interface.refresh_interval_ms);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let CrosstermEvent::Key(key_event) = event::read()? {
                app.on_key(key_event);
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

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let main_layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // For Status Bar
            Constraint::Min(0),    // For Main Content Area
            Constraint::Length(3), // For Input Bar
        ].as_ref())
        .split(f.size());

    StatusBarWidget::render(f, app, main_layout_chunks[0]);
    
    let main_content_area = main_layout_chunks[1];

    match app.active_view {
        TuiView::VmList => {
            VmListWidget::render(f, app, main_content_area);
        }
        TuiView::OllamaModelList => {
            OllamaModelListWidget::render(f, app, main_content_area);
        }
        TuiView::Chat => {
            ChatWidget::render(f, app, main_content_area);
        }
        TuiView::Logs => {
            LogsWidget::render(f, app, main_content_area);
        }
    }

    InputBarWidget::render(f, app, main_layout_chunks[2]);
}

#[allow(dead_code)]
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

// TODO:
// - Implement actual async call to ollama_manager.generate_response_stream in on_key for Enter.
//   - This will involve spawning a tokio task.
//   - The task needs to send message chunks back to the TUI event loop (e.g., via mpsc channel).
//   - App::on_tick or a new event type would handle these chunks and update active_chat.messages.
// - Implement scrolling for VM list, Ollama model list, and chat message list.
// - Add more robust error handling and display for TUI (e.g., a status message area).
// - Refine UI layout and styling.
// - Complete other TODOs throughout the file. 