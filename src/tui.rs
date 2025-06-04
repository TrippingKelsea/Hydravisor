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
use tracing::Level;
use tokio::sync::mpsc;

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

pub mod theme; // Add theme module
use theme::AppTheme; // Import AppTheme

pub mod widgets; // Added widgets submodule declaration
pub mod tracing_layer; // Add this line
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
    pub thought: Option<String>,
}

// Represents an active chat session
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub model_name: String,
    pub messages: Vec<ChatMessage>,
    pub is_streaming: bool,
}

// New struct for TUI log entries
#[derive(Clone, Debug)]
pub struct UILogEntry {
    pub timestamp: String, // Using String for simplicity, formatted in the tracing layer
    pub level: Level,
    pub target: String,
    pub message: String,
    // Consider adding: pub file: Option<String>, pub line: Option<u32>,
}

// New enum for TUI chat stream events
#[derive(Clone, Debug)]
pub enum ChatStreamEvent {
    Chunk(String),      // A piece of the response
    Error(String),      // An error occurred during streaming
    Completed,          // Streaming finished successfully
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
    log_entries: Vec<UILogEntry>,
    log_list_state: ListState,
    log_receiver: Option<mpsc::UnboundedReceiver<UILogEntry>>,

    // For Ollama chat streaming
    chat_stream_sender: mpsc::UnboundedSender<ChatStreamEvent>,
    chat_stream_receiver: Option<mpsc::UnboundedReceiver<ChatStreamEvent>>,
    chat_list_state: ListState,
    theme: Arc<AppTheme>, // Add theme field
}

impl App {
    pub fn new(
        config: Arc<Config>,
        session_manager: Arc<SessionManager>,
        policy_engine: Arc<PolicyEngine>,
        env_manager: Arc<EnvironmentManager>,
        audit_engine: Arc<AuditEngine>,
        ollama_manager: Arc<OllamaManager>,
        log_receiver: mpsc::UnboundedReceiver<UILogEntry>,
    ) -> Self {
        // Create channel for chat stream events
        let (chat_tx, chat_rx) = mpsc::unbounded_channel::<ChatStreamEvent>();

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
            log_entries: Vec::new(),
            log_list_state: ListState::default(),
            log_receiver: Some(log_receiver),
            chat_stream_sender: chat_tx,
            chat_stream_receiver: Some(chat_rx),
            chat_list_state: ListState::default(),
            theme: Arc::new(AppTheme::default()), // Initialize theme
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
            let current_selection = self.log_list_state.selected();
            let num_entries = self.log_entries.len();

            if num_entries == 0 { // No logs, no navigation
                // Allow global keys like Tab and q to be processed below
            } else {
                match key_event.code {
                    KeyCode::Up => {
                        if let Some(selected) = current_selection {
                            if selected > 0 {
                                self.log_list_state.select(Some(selected - 1));
                            }
                        } else {
                            self.log_list_state.select(Some(num_entries - 1)); // Select last if nothing selected
                        }
                        event_consumed = true;
                    }
                    KeyCode::Down => {
                        if let Some(selected) = current_selection {
                            if selected < num_entries - 1 {
                                self.log_list_state.select(Some(selected + 1));
                            }
                        } else {
                            self.log_list_state.select(Some(0)); // Select first if nothing selected
                        }
                        event_consumed = true;
                    }
                    KeyCode::PageUp => {
                        if let Some(selected) = current_selection {
                            // For simplicity, jump 10 lines or to the start
                            let new_selection = selected.saturating_sub(10);
                            self.log_list_state.select(Some(new_selection));
                        } else {
                            self.log_list_state.select(Some(0));
                        }
                        event_consumed = true;
                    }
                    KeyCode::PageDown => {
                        if let Some(selected) = current_selection {
                            // For simplicity, jump 10 lines or to the end
                            let new_selection = (selected + 10).min(num_entries - 1);
                            self.log_list_state.select(Some(new_selection));
                        } else {
                            self.log_list_state.select(Some((10 as usize).min(num_entries -1 ) )); // select 10th or last
                        }
                        event_consumed = true;
                    }
                    KeyCode::Home => {
                        self.log_list_state.select(Some(0));
                        event_consumed = true;
                    }
                    KeyCode::End => {
                        self.log_list_state.select(Some(num_entries - 1));
                        event_consumed = true;
                    }
                    _ => {} // Other keys fall through to global handling
                }
            }
        }

        if event_consumed {
            return;
        }
        
        // Handle input mode switching and global quit
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
                        KeyCode::BackTab => {
                            self.active_view = match self.active_view {
                                TuiView::VmList => TuiView::Logs,
                                TuiView::OllamaModelList => TuiView::VmList,
                                TuiView::Chat => TuiView::OllamaModelList,
                                TuiView::Logs => TuiView::Chat,
                            };
                            self.vm_list_state.select(if self.vms.is_empty() { None } else { Some(0) });
                            #[cfg(feature = "ollama_integration")]
                            self.ollama_model_list_state.select(if self.ollama_models.is_empty() { None } else { Some(0) });
                        }
                        KeyCode::Tab => {
                            self.active_view = match self.active_view {
                                TuiView::VmList => TuiView::OllamaModelList,
                                TuiView::OllamaModelList => TuiView::Chat,
                                TuiView::Chat => TuiView::Logs,
                                TuiView::Logs => TuiView::VmList,
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
                                            timestamp: Local::now().format("%H:%M:%S").to_string(),
                                            thought: None,
                                        });

                                        let assistant_model_name = chat_session.model_name.clone();
                                        chat_session.messages.push(ChatMessage {
                                            sender: assistant_model_name.clone(),
                                            content: "".to_string(), // Placeholder
                                            timestamp: Local::now().format("%H:%M:%S").to_string(),
                                            thought: None,
                                        });
                                        chat_session.is_streaming = true;

                                        #[cfg(feature = "ollama_integration")]
                                        {
                                            // Clone necessary data for the async task
                                            let ollama_manager_clone = Arc::clone(&self.ollama_manager);
                                            let chat_stream_sender_clone = self.chat_stream_sender.clone();
                                            
                                            if let Ok(handle) = Handle::try_current() { // Handle is in scope due to feature gate on this block
                                                handle.spawn(async move {
                                                    match ollama_manager_clone.generate_response_stream(assistant_model_name.clone(), prompt_text).await {
                                                        Ok(mut stream) => {
                                                            // The stream from the ollama_integration feature IS StreamExt
                                                            while let Some(result_chunk) = futures::StreamExt::next(&mut stream).await {
                                                                match result_chunk {
                                                                    Ok(chunk) => {
                                                                        if chat_stream_sender_clone.send(ChatStreamEvent::Chunk(chunk)).is_err() {
                                                                            break; // Receiver dropped
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        let error_msg = format!("Ollama stream error: {}", e);
                                                                        let _ = chat_stream_sender_clone.send(ChatStreamEvent::Error(error_msg));
                                                                        break; // Stop on stream error
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let error_msg = format!("Failed to start Ollama stream: {}", e);
                                                            let _ = chat_stream_sender_clone.send(ChatStreamEvent::Error(error_msg));
                                                        }
                                                    }
                                                    let _ = chat_stream_sender_clone.send(ChatStreamEvent::Completed);
                                                });
                                            } else {
                                                // This else block is under ollama_integration feature gate
                                                let error_msg = "Failed to spawn Ollama stream task: No Tokio runtime (ollama_integration active).".to_string();
                                                if let Some(active_chat_session) = &mut self.active_chat {
                                                    if let Some(last_message) = active_chat_session.messages.last_mut() {
                                                        last_message.content = error_msg.clone();
                                                    }
                                                    active_chat_session.is_streaming = false;
                                                }
                                                let _ = self.chat_stream_sender.send(ChatStreamEvent::Error(error_msg));
                                                let _ = self.chat_stream_sender.send(ChatStreamEvent::Completed);
                                            }
                                        }

                                        #[cfg(not(feature = "ollama_integration"))]
                                        {
                                            // Ollama integration is disabled, provide a canned response
                                            if let Some(active_chat_session) = &mut self.active_chat {
                                                if let Some(last_message) = active_chat_session.messages.last_mut() {
                                                    last_message.content = "Ollama integration is disabled.".to_string();
                                                }
                                                active_chat_session.is_streaming = false;
                                            }
                                            // We can still send Completed via channel if desired, or just update UI directly
                                            let _ = self.chat_stream_sender.send(ChatStreamEvent::Completed); 
                                        }
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
        if let Some(rx) = &mut self.log_receiver {
            let mut new_logs_added_count = 0;
            let mut was_at_bottom = false;
            let old_len = self.log_entries.len();

            if self.active_view == TuiView::Logs && !self.log_entries.is_empty() {
                if let Some(selected_idx) = self.log_list_state.selected() {
                    if selected_idx == old_len - 1 {
                        was_at_bottom = true;
                    }
                } else {
                    // If nothing was selected, and logs arrive, we should scroll to bottom
                    was_at_bottom = true; 
                }
            }

            while let Ok(log_entry) = rx.try_recv() {
                self.log_entries.push(log_entry);
                new_logs_added_count += 1;
                // Optional: Cap the number of log entries
                const MAX_LOG_ENTRIES: usize = 2000; // Example cap
                if self.log_entries.len() > MAX_LOG_ENTRIES {
                    self.log_entries.drain(0..self.log_entries.len() - MAX_LOG_ENTRIES);
                    // Adjust selection if draining affected it, though auto-scroll logic might handle it
                    if let Some(selected_idx) = self.log_list_state.selected(){
                        if selected_idx < (self.log_entries.len() - MAX_LOG_ENTRIES) {
                             // selection was in the drained part, might need to clear or reset
                             self.log_list_state.select(None); 
                        }
                    }
                }
            }

            if new_logs_added_count > 0 && self.active_view == TuiView::Logs {
                if was_at_bottom {
                    self.log_list_state.select(Some(self.log_entries.len() - 1));
                } else {
                    // If user had scrolled up, and then logs were drained, ensure selection is still valid.
                    // This might be complex if draining causes current selection to be out of bounds.
                    // For now, if not at bottom, we don't auto-scroll. User has to scroll down to see new ones.
                    // But if draining happened, the selection index might need adjustment if it pointed
                    // to an entry that no longer exists due to the drain from the beginning.
                    // The current drain logic selects None if selection was in drained part.
                    // If logs were added and not at bottom, the selection index remains, but relative position shifts.
                }
            } else if self.active_view == TuiView::Logs && self.log_entries.is_empty() {
                 self.log_list_state.select(None); // Clear selection if no logs
            }
        }

        // Poll for chat stream events
        if let Some(chat_rx) = &mut self.chat_stream_receiver {
            while let Ok(event) = chat_rx.try_recv() {
                if let Some(chat_session) = &mut self.active_chat {
                    match event {
                        ChatStreamEvent::Chunk(chunk) => {
                            if chat_session.is_streaming {
                                if let Some(last_message) = chat_session.messages.last_mut() {
                                    last_message.content.push_str(&chunk);
                                }
                            }
                        }
                        ChatStreamEvent::Error(error_msg) => {
                            if let Some(last_message) = chat_session.messages.last_mut() {
                                if last_message.content.is_empty() || chat_session.is_streaming {
                                    last_message.content = format!("[Error] {}", error_msg);
                                } else {
                                    last_message.content.push_str(&format!("\n[Error] {}", error_msg));
                                }
                            }
                            chat_session.is_streaming = false;
                        }
                        ChatStreamEvent::Completed => {
                            chat_session.is_streaming = false;
                            if let Some(last_message) = chat_session.messages.last_mut() {
                                if last_message.sender != "user" { // Only process assistant messages
                                    let mut new_content = last_message.content.clone();
                                    let mut thought_text: Option<String> = None;

                                    if let Some(start_think) = new_content.find("<think>") {
                                        if let Some(end_think) = new_content.rfind("</think>") {
                                            // Ensure <think> is before </think> and it's somewhat structured like a block
                                            if start_think < end_think && new_content.starts_with("<think>") {
                                                // Extract thought
                                                thought_text = Some(new_content[start_think + "<think>".len()..end_think].to_string());
                                                // Get content after </think>
                                                new_content = new_content[end_think + "</think>".len()..].trim_start().to_string();
                                            }
                                        }
                                    }
                                    last_message.thought = thought_text;
                                    last_message.content = new_content;
                                    
                                    // If content became empty after extracting thought, add a small note or leave as is.
                                    // if last_message.content.is_empty() && last_message.thought.is_some() {
                                    //     last_message.content = "(Thought processed)".to_string();
                                    // }
                                }
                            }
                        }
                    }
                }
            }
            // Auto-scroll chat view if active and new content might have arrived
            if self.active_view == TuiView::Chat {
                if let Some(chat_session) = &self.active_chat {
                    if !chat_session.messages.is_empty() {
                        // Simplified: always scroll to bottom if chat view is active during tick with events.
                        // Similar to logs, can be made smarter to respect user scroll position.
                        self.chat_list_state.select(Some(chat_session.messages.len() - 1));
                    }
                }
            }
        }

        // Existing on_tick logic for active_chat.is_streaming (currently empty)
        if let Some(chat_session) = &mut self.active_chat {
            if chat_session.is_streaming {
                // This flag is now primarily managed by ChatStreamEvents.
                // Can be used for UI elements like a spinner.
            }
        }
    }
}

pub fn run_tui(
    config: Arc<Config>,
    session_manager: Arc<SessionManager>,
    policy_engine: Arc<PolicyEngine>,
    env_manager: Arc<EnvironmentManager>,
    audit_engine: Arc<AuditEngine>,
    ollama_manager: Arc<OllamaManager>,
    log_receiver: mpsc::UnboundedReceiver<UILogEntry>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(
        config,
        session_manager,
        policy_engine,
        env_manager,
        audit_engine,
        ollama_manager,
        log_receiver,
    );
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