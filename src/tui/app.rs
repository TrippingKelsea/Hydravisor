// src/tui/app.rs

use anyhow::Result;
use ratatui::{
    widgets::{ListState},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{Level, error};
use tokio::sync::mpsc;
use uuid::Uuid;

#[cfg(feature = "ollama_integration")]
use ollama_rs::models::LocalModel;

use crate::config::Config;
use crate::session_manager::SessionManager;
use crate::policy::PolicyEngine;
use crate::env_manager::{EnvironmentManager, EnvironmentStatus};
use crate::audit_engine::AuditEngine;
use crate::ollama_manager::OllamaManager;

use super::theme::AppTheme;

// Define different views for the TUI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppView {
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

// New enum for app-level events to handle async operations
#[derive(Clone)]
pub enum AppEvent {
    FetchVms,
    #[cfg(feature = "ollama_integration")]
    FetchOllamaModels,
    DestroyVm(String),
    ResumeVm(String),
}


pub struct App {
    pub should_quit: bool,
    pub show_menu: bool,
    pub menu_state: ListState,
    pub show_about_modal: bool,
    pub readme_content: String,
    
    #[cfg(feature = "ollama_integration")]
    pub ollama_models: Vec<LocalModel>,
    #[cfg(not(feature = "ollama_integration"))]
    pub ollama_models: Vec<String>,

    pub vms: Vec<EnvironmentStatus>,
    pub vm_list_state: ListState,
    
    #[cfg(feature = "ollama_integration")]
    pub ollama_model_list_state: ListState,

    pub config: Arc<Config>,
    pub session_manager: Arc<SessionManager>,
    pub policy_engine: Arc<PolicyEngine>,
    pub env_manager: Arc<Mutex<EnvironmentManager>>,
    pub audit_engine: Arc<AuditEngine>,
    pub ollama_manager: Arc<Mutex<OllamaManager>>,

    pub active_view: AppView,
    pub input_mode: InputMode,
    pub current_input: String,
    pub active_chat: Option<ChatSession>,
    pub log_entries: Vec<UILogEntry>,
    pub log_list_state: ListState,
    pub log_receiver: Option<mpsc::UnboundedReceiver<UILogEntry>>,

    // For Ollama chat streaming
    pub chat_stream_sender: mpsc::UnboundedSender<ChatStreamEvent>,
    pub chat_stream_receiver: Option<mpsc::UnboundedReceiver<ChatStreamEvent>>,
    pub chat_list_state: ListState,
    pub theme: Arc<AppTheme>, // Add theme field

    // For the New VM Popup
    pub show_new_vm_popup: bool,
    pub new_vm_name: String,
    pub new_vm_use_iso: bool,
    pub new_vm_iso_path: String,
    pub new_vm_source_image_path: String,
    pub new_vm_disk_path: String,
    pub new_vm_cpu: String,
    pub new_vm_ram_mb: String,
    pub new_vm_disk_gb: String,
    pub active_new_vm_input_idx: usize,

    // For VM Destruction confirmation
    pub vm_to_destroy: Option<String>,

    // For editing system prompts
    pub editing_system_prompt_for_model: Option<String>, // Name of the model whose system prompt is being edited
    // This map will hold live edits to system prompts before saving to config
    // It's initialized from app.config and is the source for OllamaModelListWidget display
    pub editable_ollama_model_prompts: std::collections::HashMap<String, String>,
    pub input_bar_scroll: u16, // Scroll offset for the input bar
    pub input_bar_last_wrapped_line_count: usize, // For clamping scroll
    pub input_bar_visible_height: u16,          // For scroll calculation
    pub input_bar_cursor_needs_to_be_visible: bool, // Flag for auto-scroll logic
    pub input_cursor_char_idx: usize, // Character index of the cursor in current_input
    pub last_input_text_area_width: u16, // Cache for Up/Down arrow navigation

    // State for status bar
    pub libvirt_connected: bool,

    // Channel for sending async commands from sync event handlers
    pub event_sender: mpsc::UnboundedSender<AppEvent>,
    pub event_receiver: Option<mpsc::UnboundedReceiver<AppEvent>>,
}

impl App {
    pub fn new(
        config: Arc<Config>,
        session_manager: Arc<SessionManager>,
        policy_engine: Arc<PolicyEngine>,
        env_manager: Arc<Mutex<EnvironmentManager>>,
        audit_engine: Arc<AuditEngine>,
        ollama_manager: Arc<Mutex<OllamaManager>>,
        log_receiver: mpsc::UnboundedReceiver<UILogEntry>,
    ) -> Self {
        // Create channel for chat stream events
        let (chat_tx, chat_rx) = mpsc::unbounded_channel::<ChatStreamEvent>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<AppEvent>();

        // Initialize editable_ollama_model_prompts from config
        let mut initial_editable_prompts = std::collections::HashMap::new();
        if let Some(model_prompts) = &config.providers.ollama.model_system_prompts {
            initial_editable_prompts = model_prompts.clone();
        }

        let vm_uuid = Uuid::new_v4();
        let mut app = Self {
            should_quit: false,
            show_menu: false,
            menu_state: ListState::default(),
            show_about_modal: false,
            readme_content: String::new(),
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
            ollama_manager,
            active_view: AppView::VmList,
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
            libvirt_connected: false, // Initial state
            event_sender: event_tx,
            event_receiver: Some(event_rx),
        };
        
        // Read README.md for the about modal
        let readme_path = "README.md";
        let readme_lines = app.config.interface.about_modal_readme_lines;
        match std::fs::read_to_string(readme_path) {
            Ok(content) => {
                app.readme_content = content.lines().take(readme_lines).collect::<Vec<&str>>().join("\n");
            }
            Err(e) => {
                app.readme_content = format!("Could not read README.md: {}", e);
            }
        }

        app
    }

    // Fetches Ollama models asynchronously and updates the app state.
    #[cfg(feature = "ollama_integration")]
    pub async fn fetch_ollama_models(&mut self) {
        match self.ollama_manager.lock().await.list_local_models().await {
            Ok(models) => self.ollama_models = models,
            Err(e) => {
                error!("Failed to fetch Ollama models: {}", e);
            }
        }
         if self.ollama_model_list_state.selected().is_none() && !self.ollama_models.is_empty() {
            self.ollama_model_list_state.select(Some(0));
        }
    }
    
    pub async fn fetch_vms(&mut self) {
        let env_manager = self.env_manager.lock().await;
        self.libvirt_connected = env_manager.is_libvirt_connected();
        match env_manager.list_environments() {
            Ok(vms) => self.vms = vms,
            Err(e) => {
                error!("Failed to fetch VMs: {}", e);
                self.vms.clear(); // Clear VMs on error to reflect the failure state
            }
        }
        if self.vm_list_state.selected().is_none() && !self.vms.is_empty() {
            self.vm_list_state.select(Some(0));
        }
    }

    // Helper function to get the system prompt for a model, checking the live-edit map first
    pub fn get_active_system_prompt(&self, model_name: &str) -> String {
        self.editable_ollama_model_prompts
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| self.config.default_system_prompt.clone().unwrap_or_default())
    }

    pub fn menu_next(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => if i >= 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub fn menu_previous(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => if i == 0 { 1 } else { i - 1 },
            None => 0,
        };
        self.menu_state.select(Some(i));
    }
    
    pub fn select_next_item_in_vm_list(&mut self) {
            let i = match self.vm_list_state.selected() {
                Some(i) => if i >= self.vms.len() - 1 { 0 } else { i + 1 },
                None => 0,
            };
            self.vm_list_state.select(Some(i));
    }

    pub fn select_previous_item_in_vm_list(&mut self) {
            let i = match self.vm_list_state.selected() {
                Some(i) => if i == 0 { self.vms.len() - 1 } else { i - 1 },
                None => 0,
            };
            self.vm_list_state.select(Some(i));
    }

    #[cfg(feature = "ollama_integration")]
    pub fn select_next_item_in_ollama_list(&mut self) {
            let i = match self.ollama_model_list_state.selected() {
                Some(i) => if i >= self.ollama_models.len() - 1 { 0 } else { i + 1 },
                None => 0,
            };
            self.ollama_model_list_state.select(Some(i));
    }

    #[cfg(feature = "ollama_integration")]
    pub fn select_previous_item_in_ollama_list(&mut self) {
            let i = match self.ollama_model_list_state.selected() {
                Some(i) => if i == 0 { self.ollama_models.len() - 1 } else { i - 1 },
                None => 0,
            };
            self.ollama_model_list_state.select(Some(i));
        }

    #[cfg(not(feature = "ollama_integration"))]
    pub fn select_next_item_in_ollama_list(&mut self) { }

    #[cfg(not(feature = "ollama_integration"))]
    pub fn select_previous_item_in_ollama_list(&mut self) { }

    pub fn scroll_chat_up(&mut self) {
        if let Some(_) = &self.active_chat {
            let current_selection = self.chat_list_state.selected().unwrap_or(0);
            if current_selection > 0 {
                self.chat_list_state.select(Some(current_selection - 1));
            }
        }
    }
    
    pub fn scroll_chat_down(&mut self) {
        if let Some(session) = &self.active_chat {
            if session.messages.is_empty() { return; }
            let max_index = session.messages.len() - 1;
            let current_selection = self.chat_list_state.selected().unwrap_or(0);
            if current_selection < max_index {
                self.chat_list_state.select(Some(current_selection + 1));
            }
        }
    }
    
    pub fn scroll_logs_up(&mut self) {
        let current_selection = self.log_list_state.selected().unwrap_or(0);
        if current_selection > 0 {
            self.log_list_state.select(Some(current_selection - 1));
        }
    }
    
    pub fn scroll_logs_down(&mut self) {
        if self.log_entries.is_empty() { return; }
        let max_index = self.log_entries.len() - 1;
        let current_selection = self.log_list_state.selected().unwrap_or(0);
        if current_selection < max_index {
            self.log_list_state.select(Some(current_selection + 1));
        }
    }

    pub fn reset_cursor_position(&mut self) {
        self.input_cursor_char_idx = self.current_input.chars().count();
        self.input_bar_cursor_needs_to_be_visible = true;
    }

}


// Helper for parsing RAM string like "4GB" or "2048MB"
pub fn parse_ram_str(ram_str: &str) -> Result<u64> {
    let s = ram_str.trim().to_lowercase();
    if let Some(num_str) = s.strip_suffix("gb") {
        num_str
            .trim()
            .parse::<u64>()
            .map(|num| num * 1024)
            .map_err(anyhow::Error::from)
    } else if let Some(num_str) = s.strip_suffix("mb") {
        num_str.trim().parse::<u64>().map_err(anyhow::Error::from)
    } else {
        ram_str.trim().parse::<u64>().map_err(anyhow::Error::from)
    }
} 