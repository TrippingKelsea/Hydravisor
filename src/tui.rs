// src/tui.rs

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, ListState, Paragraph},
    Terminal,
};
use std::{io::{self, Stdout}, sync::Arc, time::{Duration, Instant}};
use chrono::Local;
use tracing::{Level, error};
use tokio::sync::mpsc;
use uuid::Uuid;

#[cfg(feature = "ollama_integration")]
use futures::StreamExt; // Added StreamExt for .next() method on streams

use crate::config::Config;
use crate::session_manager::SessionManager;
use crate::policy::PolicyEngine;
use crate::env_manager::{EnvironmentManager, EnvironmentStatus, EnvironmentConfig, EnvironmentType};
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
use self::widgets::new_vm_popup::NewVmPopupWidget;

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
    VmWizard,
    ConfirmingDestroy,
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
    Error(String),      // An error occurred during streaming - changed from ollama_rs::error::OllamaError
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

    // For the New VM Popup
    show_new_vm_popup: bool,
    new_vm_name: String,
    new_vm_use_iso: bool,
    new_vm_iso_path: String,
    new_vm_source_image_path: String,
    new_vm_disk_path: String,
    new_vm_cpu: String,
    new_vm_ram_mb: String,
    new_vm_disk_gb: String,
    active_new_vm_input_idx: usize,

    // For VM Destruction confirmation
    vm_to_destroy: Option<String>,

    // For editing system prompts
    editing_system_prompt_for_model: Option<String>, // Name of the model whose system prompt is being edited
    // This map will hold live edits to system prompts before saving to config
    // It's initialized from app.config and is the source for OllamaModelListWidget display
    editable_ollama_model_prompts: std::collections::HashMap<String, String>,
    input_bar_scroll: u16, // Scroll offset for the input bar
    input_bar_last_wrapped_line_count: usize, // For clamping scroll
    input_bar_visible_height: u16,          // For scroll calculation
    input_bar_cursor_needs_to_be_visible: bool, // Flag for auto-scroll logic
    input_cursor_char_idx: usize, // Character index of the cursor in current_input
    last_input_text_area_width: u16, // Cache for Up/Down arrow navigation
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

        // Initialize editable_ollama_model_prompts from config
        let mut initial_editable_prompts = std::collections::HashMap::new();
        if let Some(model_prompts) = &config.providers.ollama.model_system_prompts {
            initial_editable_prompts = model_prompts.clone();
        }

        let vm_uuid = Uuid::new_v4();
        let mut app = Self {
            should_quit: false,
            ollama_models: Vec::new(),
            vms: Vec::new(),
            vm_list_state: ListState::default(),
            #[cfg(feature = "ollama_integration")]
            ollama_model_list_state: ListState::default(),
            config: Arc::clone(&config),
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
            show_new_vm_popup: false,
            new_vm_name: format!("{}-{}", &config.defaults.default_vm_image, vm_uuid.simple()),
            new_vm_use_iso: true,
            new_vm_iso_path: config.defaults.default_vm_iso.clone(),
            new_vm_source_image_path: config.defaults.default_source_image.clone().unwrap_or_default(),
            new_vm_disk_path: String::new(),
            new_vm_cpu: config.defaults.default_cpu.to_string(),
            new_vm_ram_mb: config.defaults.default_ram.clone(),
            new_vm_disk_gb: config.defaults.default_disk_gb.to_string(),
            active_new_vm_input_idx: 0,
            vm_to_destroy: None,
            editing_system_prompt_for_model: None,
            editable_ollama_model_prompts: initial_editable_prompts,
            input_bar_scroll: 0, // Initialize scroll offset
            input_bar_last_wrapped_line_count: 0,
            input_bar_visible_height: 1, // Default to 1, will be updated by render
            input_bar_cursor_needs_to_be_visible: true,
            input_cursor_char_idx: 0, // Initialize cursor position
            last_input_text_area_width: 1, // Default, will be updated by render
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
        if let Ok(xdg_dirs) = xdg::BaseDirectories::with_prefix("hydravisor") {
            if let Ok(path) = xdg_dirs.place_data_file(format!("images/{}.qcow2", &app.new_vm_name)) {
                app.new_vm_disk_path = path.to_str().unwrap_or("").to_string()
            }
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

    // Helper to get the active system prompt for a model, considering edits
    fn get_active_system_prompt(&self, model_name: &str) -> String {
        self.editable_ollama_model_prompts
            .get(model_name)
            .cloned()
            .or_else(|| self.config.default_system_prompt.clone())
            .unwrap_or_else(|| "You are a helpful AI assistant.".to_string()) // Fallback if even default is None
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

        // Add chat view scrolling logic here
        if self.active_view == TuiView::Chat && self.input_mode == InputMode::Normal {
            if let Some(chat_session) = &self.active_chat {
                if !chat_session.messages.is_empty() {
                    let current_selection = self.chat_list_state.selected().unwrap_or(0);
                    let num_messages = chat_session.messages.len();
                    let view_height = 10; // Approximate height for PageUp/PageDown, can be refined

                    match key_event.code {
                        KeyCode::Up => {
                            let next_selection = if current_selection > 0 { current_selection - 1 } else { 0 };
                            self.chat_list_state.select(Some(next_selection));
                            event_consumed = true;
                        }
                        KeyCode::Down => {
                            let next_selection = if current_selection < num_messages - 1 { current_selection + 1 } else { num_messages - 1 };
                            self.chat_list_state.select(Some(next_selection));
                            event_consumed = true;
                        }
                        KeyCode::PageUp => {
                            let next_selection = current_selection.saturating_sub(view_height);
                            self.chat_list_state.select(Some(next_selection));
                            event_consumed = true;
                        }
                        KeyCode::PageDown => {
                            let next_selection = (current_selection + view_height).min(num_messages - 1);
                            self.chat_list_state.select(Some(next_selection));
                            event_consumed = true;
                        }
                        KeyCode::Home => {
                            self.chat_list_state.select(Some(0));
                            event_consumed = true;
                        }
                        KeyCode::End => {
                            self.chat_list_state.select(Some(num_messages - 1));
                            event_consumed = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        if event_consumed {
            return;
        }
        
        // Global quit, already handled if input_mode is Normal and key is 'q'
        // Need to ensure 'q' doesn't quit if editing_system_prompt_for_model is Some

        if self.editing_system_prompt_for_model.is_some() {
            match key_event {
                KeyEvent { code: KeyCode::Enter, .. } => {
                    if let Some(model_name) = self.editing_system_prompt_for_model.take() {
                        self.editable_ollama_model_prompts.insert(model_name.clone(), self.current_input.clone());
                        let mut config_to_save = (*self.config).clone();
                        if config_to_save.providers.ollama.model_system_prompts.is_none() {
                            config_to_save.providers.ollama.model_system_prompts = Some(std::collections::HashMap::new());
                        }
                        config_to_save.providers.ollama.model_system_prompts = Some(self.editable_ollama_model_prompts.clone());
                        match config_to_save.save() {
                            Ok(_) => {
                                self.config = Arc::new(config_to_save);
                                tracing::info!("System prompt for {} updated and config saved.", model_name);
                            }
                            Err(e) => {
                                tracing::error!("Failed to save config after updating system prompt for {}: {}", model_name, e);
                            }
                        }
                        self.current_input.clear();
                        self.input_mode = InputMode::Normal;
                        self.input_bar_cursor_needs_to_be_visible = true;
                        self.input_bar_scroll = 0;
                        self.input_cursor_char_idx = 0; // Reset cursor position
                    }
                }
                KeyEvent { code: KeyCode::Char(c), modifiers, .. } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
                    let mut chars: Vec<char> = self.current_input.chars().collect();
                    if self.input_cursor_char_idx <= chars.len() {
                        chars.insert(self.input_cursor_char_idx, c);
                        self.current_input = chars.into_iter().collect();
                        self.input_cursor_char_idx += 1;
                    }
                    self.input_bar_cursor_needs_to_be_visible = true;
                }
                KeyEvent { code: KeyCode::Backspace, modifiers: KeyModifiers::NONE, .. } => { 
                    if self.input_cursor_char_idx > 0 {
                        let mut chars: Vec<char> = self.current_input.chars().collect();
                        if self.input_cursor_char_idx <= chars.len() { // Ensure cursor is within actual char bounds
                            chars.remove(self.input_cursor_char_idx - 1);
                            self.current_input = chars.into_iter().collect();
                            self.input_cursor_char_idx -= 1;
                        }
                    }
                    self.input_bar_cursor_needs_to_be_visible = true;
                }
                KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE, .. } => {
                    if self.input_cursor_char_idx > 0 {
                        self.input_cursor_char_idx -= 1;
                    }
                    self.input_bar_cursor_needs_to_be_visible = true; // Always true
                }
                KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE, .. } => {
                    let char_count = self.current_input.chars().count();
                    if self.input_cursor_char_idx < char_count {
                        self.input_cursor_char_idx += 1;
                    }
                    self.input_bar_cursor_needs_to_be_visible = true; // Always true
                }
                KeyEvent { code: KeyCode::Esc, .. } => {
                    self.editing_system_prompt_for_model = None;
                    self.current_input.clear();
                    self.input_mode = InputMode::Normal;
                    self.input_bar_cursor_needs_to_be_visible = true;
                    self.input_bar_scroll = 0;
                    self.input_cursor_char_idx = 0; // Reset cursor position
                }
                KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, .. } => {
                    let _original_cursor_idx = self.input_cursor_char_idx;
                    if self.last_input_text_area_width > 0 && !self.current_input.is_empty() {
                        let wrapped_lines: Vec<String> = textwrap::wrap(&self.current_input, self.last_input_text_area_width as usize).iter().map(|s| s.to_string()).collect();
                        if wrapped_lines.is_empty() { return; }

                        let mut current_char_idx_abs = 0;
                        let mut current_wrapped_line_idx = 0;
                        let mut current_char_col_on_line = 0;

                        for (idx, line_str) in wrapped_lines.iter().enumerate() {
                            let line_char_len = line_str.chars().count();
                            if self.input_cursor_char_idx <= current_char_idx_abs + line_char_len {
                                current_wrapped_line_idx = idx;
                                current_char_col_on_line = self.input_cursor_char_idx - current_char_idx_abs;
                                break;
                            }
                            current_char_idx_abs += line_char_len;
                        }
                         // Handle case where cursor might be at the very end of text, effectively on the last line at its end column.
                        if self.input_cursor_char_idx == self.current_input.chars().count() && !wrapped_lines.is_empty() {
                             current_wrapped_line_idx = wrapped_lines.len() - 1;
                             current_char_col_on_line = wrapped_lines.last().unwrap().chars().count();
                        }

                        if current_wrapped_line_idx > 0 {
                            let target_wrapped_line_idx = current_wrapped_line_idx - 1;
                            let mut new_char_idx_abs = 0;
                            for i in 0..target_wrapped_line_idx {
                                new_char_idx_abs += wrapped_lines[i].chars().count();
                            }
                            let target_line_actual_len = wrapped_lines[target_wrapped_line_idx].chars().count();
                            new_char_idx_abs += current_char_col_on_line.min(target_line_actual_len);
                            self.input_cursor_char_idx = new_char_idx_abs;
                        }
                    }
                    // Always set to true after Up arrow, assuming cursor might have moved or view needs check
                    self.input_bar_cursor_needs_to_be_visible = true; 
                }
                KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, .. } => {
                    let _original_cursor_idx = self.input_cursor_char_idx;
                    if self.last_input_text_area_width > 0 && !self.current_input.is_empty() {
                        let wrapped_lines: Vec<String> = textwrap::wrap(&self.current_input, self.last_input_text_area_width as usize).iter().map(|s| s.to_string()).collect();
                        if wrapped_lines.is_empty() || wrapped_lines.len() == 1 { return; }

                        let mut current_char_idx_abs = 0;
                        let mut current_wrapped_line_idx = 0;
                        let mut current_char_col_on_line = 0;

                        for (idx, line_str) in wrapped_lines.iter().enumerate() {
                            let line_char_len = line_str.chars().count();
                            if self.input_cursor_char_idx <= current_char_idx_abs + line_char_len {
                                current_wrapped_line_idx = idx;
                                current_char_col_on_line = self.input_cursor_char_idx - current_char_idx_abs;
                                break;
                            }
                            current_char_idx_abs += line_char_len;
                        }
                        if self.input_cursor_char_idx == self.current_input.chars().count() && !wrapped_lines.is_empty() {
                             current_wrapped_line_idx = wrapped_lines.len() - 1;
                             current_char_col_on_line = wrapped_lines.last().unwrap().chars().count();
                        }

                        if current_wrapped_line_idx < wrapped_lines.len() - 1 {
                            let target_wrapped_line_idx = current_wrapped_line_idx + 1;
                            let mut new_char_idx_abs = 0;
                            for i in 0..target_wrapped_line_idx {
                                new_char_idx_abs += wrapped_lines[i].chars().count();
                            }
                            let target_line_actual_len = wrapped_lines[target_wrapped_line_idx].chars().count();
                            new_char_idx_abs += current_char_col_on_line.min(target_line_actual_len);
                            self.input_cursor_char_idx = new_char_idx_abs;
                        }
                    }
                    // Always set to true after Down arrow
                    self.input_bar_cursor_needs_to_be_visible = true;
                }
                _ => {} // Other keys are currently ignored here
            }
            return; 
        }

        // --- Start: Normal Mode and Chat Editing Mode Logic ---
        match self.input_mode {
            InputMode::Normal => {
                match key_event.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('i') => { // Changed from 'e' to 'i'
                        if self.active_view == TuiView::OllamaModelList {
                            #[cfg(feature = "ollama_integration")]
                            if let Some(selected_idx) = self.ollama_model_list_state.selected() {
                                if let Some(model) = self.ollama_models.get(selected_idx) {
                                    self.editing_system_prompt_for_model = Some(model.name.clone());
                                    self.current_input = self.get_active_system_prompt(&model.name);
                                    self.input_cursor_char_idx = self.current_input.chars().count(); // Set cursor to end
                                    self.input_mode = InputMode::Editing;
                                    self.input_bar_cursor_needs_to_be_visible = true; // Ensure view updates
                                }
                            }
                        } else if self.active_view == TuiView::Chat || self.editing_system_prompt_for_model.is_none() { // General case for chat or if not system prompt
                             // This is for entering chat input mode, 'i' was already used here.
                             // Ensure editing_system_prompt_for_model is None (implicit from flow or check above)
                            self.input_mode = InputMode::Editing;
                            // Set cursor to end of current input, which might be empty or contain partially typed message
                            self.input_cursor_char_idx = self.current_input.chars().count();
                            self.input_bar_cursor_needs_to_be_visible = true; // Ensure scroll visibility updates
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
                    KeyCode::Enter => { // Select Ollama model to start chat
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
                                    self.current_input.clear(); 
                                }
                            }
                        }
                    }
                    KeyCode::Char('j') => self.select_next_item_in_vm_list(),
                    KeyCode::Char('k') => self.select_previous_item_in_vm_list(),
                    KeyCode::Char('n') if key_event.modifiers == KeyModifiers::CONTROL => {
                        self.input_mode = InputMode::VmWizard;
                        self.active_new_vm_input_idx = 0;
                    }
                    KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => {
                        if let Some(selected_idx) = self.vm_list_state.selected() {
                            if let Some(vm) = self.vms.get(selected_idx) {
                                self.vm_to_destroy = Some(vm.instance_id.clone());
                                self.input_mode = InputMode::ConfirmingDestroy;
                            }
                        }
                    }
                    _ => {}
                }
            }
            InputMode::Editing => {
                // This is for CHAT INPUT only
                match key_event {
                    KeyEvent { code: KeyCode::Enter, modifiers: KeyModifiers::NONE, .. } => { // Added NONE modifier
                        if self.active_view == TuiView::Chat {
                            let prompt_text = self.current_input.clone(); // Clone before drain
                            self.current_input.clear(); // Clear for next input
                            self.input_cursor_char_idx = 0; // Reset cursor for next input

                            if !prompt_text.is_empty() {
                                if let Some(chat_session) = self.active_chat.as_mut() {
                                    chat_session.messages.push(ChatMessage {
                                        sender: "user".to_string(),
                                        content: prompt_text.clone(),
                                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                                        thought: None,
                                    });

                                    let assistant_model_name = chat_session.model_name.clone();
                                    chat_session.messages.push(ChatMessage {
                                        sender: assistant_model_name.clone(),
                                        content: String::new(), 
                                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                                        thought: None, 
                                    });
                                    chat_session.is_streaming = true;

                                    let ollama_manager_clone = Arc::clone(&self.ollama_manager);
                                    let model_name = assistant_model_name.clone();
                                    // Pass the cloned prompt_text to the stream
                                    let messages_for_stream = chat_session.messages.clone(); 
                                    let stream_tx = self.chat_stream_sender.clone();
                                    let system_prompt = self.get_active_system_prompt(&model_name);

                                    tokio::spawn(async move {
                                        match ollama_manager_clone
                                            .generate_response_stream(model_name.clone(), messages_for_stream, Some(system_prompt))
                                            .await
                                        {
                                            Ok(mut stream) => {
                                                while let Some(item_result) = stream.next().await {
                                                    match item_result {
                                                        Ok(chunk) => {
                                                            if stream_tx.send(ChatStreamEvent::Chunk(chunk)).is_err() {
                                                                error!("Failed to send chat chunk to TUI");
                                                                break;
                                                            }
                                                        }
                                                        Err(e_str) => { 
                                                            error!("Chat stream item error: {}", e_str);
                                                            if stream_tx.send(ChatStreamEvent::Error(e_str)).is_err() { 
                                                                error!("Failed to send chat stream error to TUI");
                                                            }
                                                            break; 
                                                        }
                                                    }
                                                }
                                                if stream_tx.send(ChatStreamEvent::Completed).is_err() {
                                                    error!("Failed to send chat stream completion to TUI");
                                                }
                                            }
                                            Err(e) => { 
                                                error!("Failed to start chat stream: {:?}", e);
                                                if stream_tx.send(ChatStreamEvent::Error(e.to_string())).is_err() { 
                                                    error!("Failed to send initial chat stream error to TUI");
                                                }
                                            }
                                        }
                                    });
                                    
                                    if let Some(chat_session) = &self.active_chat {
                                        if !chat_session.messages.is_empty() {
                                            self.chat_list_state.select(Some(chat_session.messages.len() - 1));
                                        }
                                    }
                                    self.input_bar_cursor_needs_to_be_visible = true; 
                                    self.input_bar_scroll = 0; 
                                }
                            }
                        }
                    }
                    KeyEvent { code: KeyCode::Char(c), modifiers, .. } if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT => {
                        let mut chars: Vec<char> = self.current_input.chars().collect();
                        if self.input_cursor_char_idx <= chars.len() {
                             chars.insert(self.input_cursor_char_idx, c);
                             self.current_input = chars.into_iter().collect();
                             self.input_cursor_char_idx += 1;
                        }
                        self.input_bar_cursor_needs_to_be_visible = true;
                    }
                    KeyEvent { code: KeyCode::Backspace, modifiers: KeyModifiers::NONE, .. } => { 
                        if self.input_cursor_char_idx > 0 {
                            let mut chars: Vec<char> = self.current_input.chars().collect();
                            if self.input_cursor_char_idx <= chars.len() {
                                chars.remove(self.input_cursor_char_idx - 1);
                                self.current_input = chars.into_iter().collect();
                                self.input_cursor_char_idx -= 1;
                            }
                        }
                        self.input_bar_cursor_needs_to_be_visible = true;
                    }
                    KeyEvent { code: KeyCode::Left, modifiers: KeyModifiers::NONE, .. } => {
                        if self.input_cursor_char_idx > 0 {
                            self.input_cursor_char_idx -= 1;
                        }
                        self.input_bar_cursor_needs_to_be_visible = true; // Always true
                    }
                    KeyEvent { code: KeyCode::Right, modifiers: KeyModifiers::NONE, .. } => {
                        let char_count = self.current_input.chars().count();
                        if self.input_cursor_char_idx < char_count {
                            self.input_cursor_char_idx += 1;
                        }
                        self.input_bar_cursor_needs_to_be_visible = true; // Always true
                    }
                    KeyEvent { code: KeyCode::Esc, .. } => {
                        self.input_mode = InputMode::Normal;
                        // self.current_input.clear(); // Optionally clear
                        // self.input_cursor_char_idx = 0; // Optionally reset
                        self.input_bar_cursor_needs_to_be_visible = true; 
                        self.input_bar_scroll = 0; // Reset scroll when exiting edit mode for chat
                    }
                    _ => {} // Other keys are currently ignored here
                }
            }
            InputMode::VmWizard => self.handle_vm_wizard_mode_key(key_event),
            InputMode::ConfirmingDestroy => self.handle_confirm_destroy_mode_key(key_event),
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
            let mut was_at_bottom_chat = false;
            let mut new_chat_content_added = false;

            if self.active_view == TuiView::Chat {
                if let Some(chat_session) = &self.active_chat {
                    if !chat_session.messages.is_empty() {
                        if let Some(selected_idx) = self.chat_list_state.selected() {
                            if selected_idx == chat_session.messages.len() - 1 {
                                was_at_bottom_chat = true;
                            }
                        } else {
                            was_at_bottom_chat = true; // If nothing selected, new content should scroll to bottom
                        }
                    }
                }
            }

            while let Ok(event) = chat_rx.try_recv() {
                new_chat_content_added = true; // Any event implies potential change or need to check scroll
                if let Some(chat_session) = &mut self.active_chat {
                    match event {
                        ChatStreamEvent::Chunk(chunk) => {
                            if chat_session.is_streaming {
                                if let Some(last_message) = chat_session.messages.last_mut() {
                                    // Sanitize the chunk before appending
                                    let mut sanitized_chunk = strip_ansi_escapes::strip_str(&chunk);
                                    // Explicitly remove form feed characters
                                    sanitized_chunk = sanitized_chunk.replace('\u{000C}', ""); 
                                    last_message.content.push_str(&sanitized_chunk);
                                }
                            }
                        }
                        ChatStreamEvent::Error(error_msg) => {
                            if let Some(last_message) = chat_session.messages.last_mut() {
                                let mut sanitized_error = strip_ansi_escapes::strip_str(&error_msg);
                                sanitized_error = sanitized_error.replace('\u{000C}', "");
                                if last_message.content.is_empty() || chat_session.is_streaming {
                                    last_message.content = format!("[Error] {}", sanitized_error);
                                } else {
                                    last_message.content.push_str(&format!("\n[Error] {}", sanitized_error));
                                }
                            }
                            chat_session.is_streaming = false;
                        }
                        ChatStreamEvent::Completed => {
                            chat_session.is_streaming = false;
                            if let Some(last_message) = chat_session.messages.last_mut() {
                                if last_message.sender != "user" { // Only process assistant messages
                                    let mut new_content = last_message.content.clone(); // Already sanitized for ANSI and form feed by chunks
                                    let mut thought_text: Option<String> = None;

                                    if let Some(start_think) = new_content.find("<think>") {
                                        if let Some(end_think) = new_content.rfind("</think>") {
                                            if start_think < end_think && new_content.starts_with("<think>") {
                                                let raw_thought = new_content[start_think + "<think>".len()..end_think].to_string();
                                                let mut sanitized_thought = strip_ansi_escapes::strip_str(&raw_thought);
                                                sanitized_thought = sanitized_thought.replace('\u{000C}', "");
                                                thought_text = Some(sanitized_thought);
                                                new_content = new_content[end_think + "</think>".len()..].trim_start().to_string();
                                            }
                                        }
                                    }
                                    last_message.thought = thought_text;
                                    last_message.content = new_content; 
                                }
                            }
                        }
                    }
                }
            }
            
            if new_chat_content_added && self.active_view == TuiView::Chat {
                if let Some(chat_session) = &self.active_chat {
                    if !chat_session.messages.is_empty() && was_at_bottom_chat {
                        self.chat_list_state.select(Some(chat_session.messages.len() - 1));
                    }
                }
            } else if self.active_view == TuiView::Chat && self.active_chat.as_ref().map_or(true, |cs| cs.messages.is_empty()) {
                self.chat_list_state.select(None); // Clear selection if no messages or no active chat
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

    // New method to handle mouse events
    pub fn on_mouse_event(&mut self, mouse_event: MouseEvent) {
        let is_editing_input_bar = self.input_mode == InputMode::Editing || self.editing_system_prompt_for_model.is_some();

        if is_editing_input_bar {
            match mouse_event.kind {
                MouseEventKind::ScrollUp => {
                    self.input_bar_scroll = self.input_bar_scroll.saturating_sub(1);
                    self.input_bar_cursor_needs_to_be_visible = false;
                    return; // Consume event for input bar
                }
                MouseEventKind::ScrollDown => {
                    self.input_bar_scroll = self.input_bar_scroll.saturating_add(1);
                    self.input_bar_cursor_needs_to_be_visible = false;
                    return; // Consume event for input bar
                }
                _ => {} // Other mouse events for input bar (e.g. click) not handled yet
            }
        }

        // Existing mouse scroll logic for chat view (ensure it doesn't conflict if chat is active view AND editing input bar)
        // The return statements above should prevent conflict for scroll events.
        if self.active_view == TuiView::Chat {
            if let Some(chat_session) = &self.active_chat {
                if !chat_session.messages.is_empty() {
                    let num_messages = chat_session.messages.len();
                    let current_selection = self.chat_list_state.selected().unwrap_or(if num_messages > 0 { num_messages -1 } else { 0 } ); // Default to bottom or 0

                    match mouse_event.kind {
                        MouseEventKind::ScrollUp => {
                            let next_selection = if current_selection > 0 { current_selection - 1 } else { 0 };
                            self.chat_list_state.select(Some(next_selection));
                        }
                        MouseEventKind::ScrollDown => {
                            let next_selection = if current_selection < num_messages - 1 { current_selection + 1 } else { num_messages - 1 };
                            self.chat_list_state.select(Some(next_selection));
                        }
                        _ => {} // Other mouse events like Move, Drag, Down, Up are ignored for now
                    }
                }
            }
        }
    }

    fn handle_vm_wizard_mode_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Tab => {
                self.active_new_vm_input_idx = (self.active_new_vm_input_idx + 1) % 8;
            }
            KeyCode::BackTab => {
                self.active_new_vm_input_idx = (self.active_new_vm_input_idx + 8 - 1) % 8;
            }
            KeyCode::Char(' ') => {
                if self.active_new_vm_input_idx == 6 { // Corresponds to the ISO checkbox
                    self.new_vm_use_iso = !self.new_vm_use_iso;
                }
            }
            KeyCode::Char(c) => match self.active_new_vm_input_idx {
                0 => self.new_vm_name.push(c),
                1 => self.new_vm_source_image_path.push(c),
                2 => self.new_vm_disk_path.push(c),
                3 => self.new_vm_cpu.push(c),
                4 => self.new_vm_ram_mb.push(c),
                5 => self.new_vm_disk_gb.push(c),
                7 => self.new_vm_iso_path.push(c),
                _ => {}
            },
            KeyCode::Backspace => match self.active_new_vm_input_idx {
                0 => { self.new_vm_name.pop(); }
                1 => { self.new_vm_source_image_path.pop(); }
                2 => { self.new_vm_disk_path.pop(); }
                3 => { self.new_vm_cpu.pop(); }
                4 => { self.new_vm_ram_mb.pop(); }
                5 => { self.new_vm_disk_gb.pop(); }
                7 => { self.new_vm_iso_path.pop(); }
                _ => {}
            },
            KeyCode::Enter => {
                let boot_iso = if self.new_vm_use_iso { Some(self.new_vm_iso_path.clone()) } else { None };

                let vm_config = EnvironmentConfig {
                    instance_id: self.new_vm_name.clone(),
                    env_type: EnvironmentType::Vm,
                    base_image: self.new_vm_source_image_path.clone(),
                    boot_iso,
                    cpu_cores: self.new_vm_cpu.parse().unwrap_or(self.config.defaults.default_cpu),
                    memory_mb: parse_ram_str(&self.new_vm_ram_mb).unwrap_or(4096),
                    disk_gb: Some(self.new_vm_disk_gb.parse().unwrap_or(self.config.defaults.default_disk_gb)),
                    network_policy: "default".to_string(),
                    security_policy: "default".to_string(),
                    custom_script: None,
                    template_name: None,
                    labels: None,
                };
                
                // TODO: The underlying create_environment is currently stubbed out
                // and will panic due to the todo! macro. We will proceed with the call
                // to demonstrate the full UI flow.
                match self.env_manager.create_environment(&vm_config) {
                    Ok(status) => {
                        tracing::info!("Successfully created new VM: {:?}", status);
                        self.fetch_vms();
                    }
                    Err(e) => {
                        tracing::error!("Failed to create VM: {}", e);
                    }
                }
                
                self.input_mode = InputMode::Normal;
                // maybe clear the fields
            }
            _ => {}
        }
    }

    fn handle_confirm_destroy_mode_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(instance_id) = self.vm_to_destroy.take() {
                    match self.env_manager.destroy_environment(&instance_id) {
                        Ok(_) => tracing::info!("Successfully initiated destruction of VM: {}", instance_id),
                        Err(e) => tracing::error!("Failed to destroy VM {}: {}", instance_id, e),
                    }
                    self.fetch_vms();
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.vm_to_destroy = None;
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    /// Resets the cursor position to the end of the input string.
    fn reset_cursor_position(&mut self) {
        self.input_cursor_char_idx = self.current_input.len();
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
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

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
            match event::read()? {
                CrosstermEvent::Key(key_event) => {
                    app.on_key(key_event);
                }
                CrosstermEvent::Mouse(mouse_event) => { // Handle Mouse Events
                    app.on_mouse_event(mouse_event);
                }
                CrosstermEvent::Resize(_, _) => { // Handle Resize if needed in the future
                    // For now, redraw will handle it. May need to recalculate layouts.
                }
                _ => {} // Other events like FocusGained/Lost, Paste
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
            Constraint::Length(4), // For Input Bar - Increased from 3 to 4 for 2 lines of text
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

    // New VM Popup
    if app.input_mode == InputMode::VmWizard {
        NewVmPopupWidget::render(f, app, f.size());
    }

    // Destroy Confirmation Popup
    if app.input_mode == InputMode::ConfirmingDestroy {
        if let Some(vm_name) = &app.vm_to_destroy {
            let text = format!("Are you sure you want to destroy VM '{}'? (y/n)", vm_name);
            let p = Paragraph::new(text)
                .block(Block::default().title("Confirm Destruction").borders(Borders::ALL))
                .alignment(ratatui::layout::Alignment::Center);

            let area = centered_rect(60, 20, f.size());
            f.render_widget(ratatui::widgets::Clear, area); //this clears the background
            f.render_widget(p, area);
        }
    }
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

fn parse_ram_str(ram_str: &str) -> Result<u64> {
    let mut s = ram_str.trim().to_uppercase();
    if s.ends_with("GB") {
        s.pop();
        s.pop();
        let val: u64 = s.parse()?;
        Ok(val * 1024)
    } else if s.ends_with("MB") {
        s.pop();
        s.pop();
        let val: u64 = s.parse()?;
        Ok(val)
    } else {
        // Assume MB if no unit
        let val: u64 = s.parse()?;
        Ok(val)
    }
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