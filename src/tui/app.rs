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
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

#[cfg(feature = "ollama_integration")]
use ollama_rs::models::LocalModel;

#[cfg(feature = "bedrock_integration")]
use aws_sdk_bedrock::types::FoundationModelSummary;

use crate::config::Config;
use crate::session_manager::SessionManager;
use crate::policy::PolicyEngine;
use crate::libvirt_manager::{LibvirtManager, VmStatus};
use crate::audit::AuditEngine;
use crate::ollama_manager::OllamaManager;
#[cfg(feature = "bedrock_integration")]
use crate::bedrock_manager::BedrockManager;
#[cfg(feature = "bedrock_integration")]
use crate::tui::view_mode::list::ListViewMode;

use super::theme::AppTheme;

// Define different views for the TUI
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppView {
    VmList,
    OllamaModelList,
    #[cfg(feature = "bedrock_integration")]
    BedrockModelList,
    Chat,
    Logs,
}

impl AppView {
    pub fn next(&self) -> Self {
        match self {
            Self::VmList => Self::OllamaModelList,
            #[cfg(not(feature = "bedrock_integration"))]
            Self::OllamaModelList => Self::Chat,
            #[cfg(feature = "bedrock_integration")]
            Self::OllamaModelList => Self::BedrockModelList,
            #[cfg(feature = "bedrock_integration")]
            Self::BedrockModelList => Self::Chat,
            Self::Chat => Self::Logs,
            Self::Logs => Self::VmList,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Self::VmList => Self::Logs,
            Self::OllamaModelList => Self::VmList,
            #[cfg(feature = "bedrock_integration")]
            Self::BedrockModelList => Self::OllamaModelList,
            #[cfg(not(feature = "bedrock_integration"))]
            Self::Chat => Self::OllamaModelList,
            #[cfg(feature = "bedrock_integration")]
            Self::Chat => Self::BedrockModelList,
            Self::Logs => Self::Chat,
        }
    }
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
    FetchOllamaModels,
    #[cfg(feature = "bedrock_integration")]
    FetchBedrockModels,
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

    #[cfg(feature = "bedrock_integration")]
    pub bedrock_models: Vec<FoundationModelSummary>,

    pub vms: Vec<VmStatus>,
    pub vm_list_state: ListState,
    
    #[cfg(feature = "ollama_integration")]
    pub ollama_model_list_state: ListState,

    #[cfg(feature = "bedrock_integration")]
    pub bedrock_model_list_state: ListState,

    pub config: Arc<Config>,
    pub libvirt_manager: Arc<Mutex<LibvirtManager>>,
    pub ollama_manager: Arc<Mutex<OllamaManager>>,
    #[cfg(feature = "bedrock_integration")]
    pub bedrock_manager: Arc<Mutex<BedrockManager>>,

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
    pub ollama_connected: bool,
    #[cfg(feature = "bedrock_integration")]
    pub bedrock_connected: bool,

    // Channel for sending async commands from sync event handlers
    pub event_sender: mpsc::UnboundedSender<AppEvent>,
    pub event_receiver: Option<mpsc::UnboundedReceiver<AppEvent>>,

    #[cfg(feature = "bedrock_integration")]
    pub bedrock_model_view_mode: ListViewMode<FoundationModelSummary>,

    pub show_keybindings_modal: bool,

    pub menu_level: u8, // 0 = main, 1 = preferences
    pub menu_sub_state: ListState,

    pub keybinding_map: HashMap<String, (KeyCode, KeyModifiers)>,

    #[cfg(feature = "bedrock_integration")]
    pub current_bedrock_filter: String,
    #[cfg(feature = "bedrock_integration")]
    pub current_bedrock_sort: String,
}

impl App {
    pub fn new(
        config: Arc<Config>,
        _session_manager: Arc<SessionManager>,
        _policy_engine: Arc<PolicyEngine>,
        libvirt_manager: Arc<Mutex<LibvirtManager>>,
        _audit_engine: Arc<AuditEngine>,
        ollama_manager: Arc<Mutex<OllamaManager>>,
        #[cfg(feature = "bedrock_integration")] bedrock_manager: Arc<Mutex<BedrockManager>>,
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

        #[cfg(feature = "bedrock_integration")]
        let bedrock_model_view_mode = ListViewMode::new();

        let vm_uuid = Uuid::new_v4();
        let mut app = Self {
            should_quit: false,
            show_menu: false,
            menu_state: ListState::default(),
            show_about_modal: false,
            readme_content: String::new(),
            ollama_models: Vec::new(),
            #[cfg(feature = "bedrock_integration")]
            bedrock_models: Vec::new(),
            vms: Vec::new(),
            vm_list_state: ListState::default(),
            #[cfg(feature = "ollama_integration")]
            ollama_model_list_state: ListState::default(),
            #[cfg(feature = "bedrock_integration")]
            bedrock_model_list_state: ListState::default(),
            config: Arc::clone(&config),
            libvirt_manager,
            ollama_manager,
            #[cfg(feature = "bedrock_integration")]
            bedrock_manager,
            #[cfg(feature = "bedrock_integration")]
            bedrock_model_view_mode,
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
            ollama_connected: false, // Initial state
            #[cfg(feature = "bedrock_integration")]
            bedrock_connected: false, // Initial state
            event_sender: event_tx,
            event_receiver: Some(event_rx),
            show_keybindings_modal: false,
            menu_level: 0,
            menu_sub_state: ListState::default(),
            keybinding_map: HashMap::new(),
            #[cfg(feature = "bedrock_integration")]
            current_bedrock_filter: config.providers.bedrock.filters.default.clone(),
            #[cfg(feature = "bedrock_integration")]
            current_bedrock_sort: "alphabetical".to_string(),
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

        let keybinding_map = parse_keybindings(&app.config.keybindings);
        app.keybinding_map = keybinding_map;

        app
    }

    pub fn tick(&mut self) {
        // This method can be used for periodic updates, e.g., animations
    }

    #[cfg(feature = "bedrock_integration")]
    pub async fn fetch_bedrock_models(&mut self) {
        let bm = self.bedrock_manager.lock().await;
        self.bedrock_connected = bm.is_bedrock_connected();
        if self.bedrock_connected {
            match bm.list_foundation_models().await {
                Ok(models) => {
                    self.bedrock_models = models;
                    if self.bedrock_models.is_empty() {
                        self.bedrock_model_list_state.select(None);
                    } else if self.bedrock_model_list_state.selected().is_none() {
                        self.bedrock_model_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    error!("Failed to fetch Bedrock models: {}", e);
                }
            }
        }
    }

    pub async fn fetch_ollama_models(&mut self) {
        #[cfg(feature = "ollama_integration")]
        {
            let om = self.ollama_manager.lock().await;
            self.ollama_connected = om.is_ollama_connected();
            if self.ollama_connected {
                match om.list_local_models().await {
                    Ok(models) => {
                        self.ollama_models = models;
                        if self.ollama_models.is_empty() {
                            self.ollama_model_list_state.select(None);
                        } else if self.ollama_model_list_state.selected().is_none() {
                            self.ollama_model_list_state.select(Some(0));
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch Ollama models: {}", e);
                        self.ollama_models.clear(); // Clear models on failure
                    }
                }
            }
        }
    }

    pub async fn fetch_vms(&mut self) {
        let lm = self.libvirt_manager.lock().await;
        self.libvirt_connected = lm.is_libvirt_connected();
        if self.libvirt_connected {
            match lm.list_vms() {
                Ok(vms) => {
                    self.vms = vms;
                    if self.vms.is_empty() {
                        self.vm_list_state.select(None);
                    } else if self.vm_list_state.selected().is_none() {
                        self.vm_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    error!("Failed to fetch VMs: {}", e);
                    self.vms.clear();
                    self.vm_list_state.select(None);
                }
            }
        } else {
            // If not connected, clear the list
            self.vms.clear();
            self.vm_list_state.select(None);
        }
    }

    // This gets the system prompt for a model, checking for a model-specific override
    // in our live-editing map first, then falling back to the main config.
    pub fn get_active_system_prompt(&self, model_name: &str) -> String {
        self.editable_ollama_model_prompts
            .get(model_name)
            .cloned()
            .unwrap_or_else(|| {
                self.config
                    .get_system_prompt_for_model(model_name)
                    .unwrap_or_else(|| self.config.default_system_prompt.clone().unwrap_or_default())
            })
    }

    pub fn menu_next(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => (i + 1) % 3, // 3 items in menu
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub fn menu_previous(&mut self) {
        let i = match self.menu_state.selected() {
            Some(i) => (i + 3 - 1) % 3,
            None => 0,
        };
        self.menu_state.select(Some(i));
    }

    pub fn select_next_item_in_vm_list(&mut self) {
        if self.vms.is_empty() {
            self.vm_list_state.select(None);
            return;
        }
        let i = match self.vm_list_state.selected() {
            Some(i) => {
                if i >= self.vms.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.vm_list_state.select(Some(i));
    }

    pub fn select_previous_item_in_vm_list(&mut self) {
        if self.vms.is_empty() {
            self.vm_list_state.select(None);
            return;
        }
        let i = match self.vm_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.vms.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.vm_list_state.select(Some(i));
    }

    #[cfg(feature = "ollama_integration")]
    pub fn select_next_item_in_ollama_list(&mut self) {
        if self.ollama_models.is_empty() {
            self.ollama_model_list_state.select(None);
            return;
        }
        let i = match self.ollama_model_list_state.selected() {
            Some(i) => {
                if i >= self.ollama_models.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.ollama_model_list_state.select(Some(i));
    }

    #[cfg(feature = "ollama_integration")]
    pub fn select_previous_item_in_ollama_list(&mut self) {
        if self.ollama_models.is_empty() {
            self.ollama_model_list_state.select(None);
            return;
        }
        let i = match self.ollama_model_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.ollama_models.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.ollama_model_list_state.select(Some(i));
    }

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

    #[cfg(feature = "bedrock_integration")]
    pub fn select_next_item_in_bedrock_list(&mut self) {
        if self.bedrock_models.is_empty() {
            self.bedrock_model_list_state.select(None);
            return;
        }
        let i = match self.bedrock_model_list_state.selected() {
            Some(i) => {
                if i >= self.bedrock_models.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.bedrock_model_list_state.select(Some(i));
    }

    #[cfg(feature = "bedrock_integration")]
    pub fn select_previous_item_in_bedrock_list(&mut self) {
        if self.bedrock_models.is_empty() {
            self.bedrock_model_list_state.select(None);
            return;
        }
        let i = match self.bedrock_model_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.bedrock_models.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.bedrock_model_list_state.select(Some(i));
    }
}

// Helper for parsing RAM string like "4GB" or "2048MB"
pub fn parse_ram_str(ram_str: &str) -> Result<u64> {
    let s = ram_str.trim().to_uppercase();
    if let Some(num_str) = s.strip_suffix("GB") {
        num_str
            .trim()
            .parse::<u64>()
            .map(|num| num * 1024)
            .map_err(anyhow::Error::from)
    } else if let Some(num_str) = s.strip_suffix("MB") {
        num_str.trim().parse::<u64>().map_err(anyhow::Error::from)
    } else {
        ram_str
            .trim()
            .parse::<u64>()
            .map_err(anyhow::Error::from)
    }
}

fn parse_keybindings(cfg: &crate::config::KeyBindingsConfig) -> HashMap<String, (KeyCode, KeyModifiers)> {
    let mut map = HashMap::new();
    macro_rules! insert {
        ($action:expr, $binding:expr) => {
            if let Some((code, mods)) = parse_keybinding(&$binding) {
                map.insert($action.to_string(), (code, mods));
            }
        };
    }
    insert!("quit", cfg.quit);
    insert!("help", cfg.help);
    insert!("menu", cfg.menu);
    insert!("next_tab", cfg.next_tab);
    insert!("prev_tab", cfg.prev_tab);
    insert!("new_vm", cfg.new_vm);
    insert!("destroy_vm", cfg.destroy_vm);
    insert!("edit", cfg.edit);
    insert!("enter", cfg.enter);
    insert!("up", cfg.up);
    insert!("down", cfg.down);

    map.insert("filter".to_string(), parse_keybinding(&cfg.filter).unwrap_or_else(default_parsed_filter));
    map.insert("sort".to_string(), parse_keybinding(&cfg.sort).unwrap_or_else(default_parsed_sort));
    map.insert("bedrock_filter".to_string(), parse_keybinding(&cfg.bedrock.filter).unwrap_or_else(default_parsed_bedrock_filter));
    map.insert("bedrock_sort".to_string(), parse_keybinding(&cfg.bedrock.sort).unwrap_or_else(default_parsed_bedrock_sort));

    map
}

fn parse_keybinding(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let s = s.trim();
    let mut mods = KeyModifiers::empty();
    let mut key = s;
    if let Some(stripped) = s.strip_prefix("Ctrl+") {
        mods |= KeyModifiers::CONTROL;
        key = stripped;
    }
    if let Some(stripped) = key.strip_prefix("Alt+") {
        mods |= KeyModifiers::ALT;
        key = stripped;
    }
    if let Some(stripped) = key.strip_prefix("Shift+") {
        mods |= KeyModifiers::SHIFT;
        key = stripped;
    }
    let code = match key.to_lowercase().as_str() {
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        c if c.len() == 1 => KeyCode::Char(c.chars().next().unwrap()),
        _ => return None,
    };
    Some((code, mods))
}

fn default_parsed_up() -> (KeyCode, KeyModifiers) { (KeyCode::Up, KeyModifiers::NONE) }
fn default_parsed_down() -> (KeyCode, KeyModifiers) { (KeyCode::Down, KeyModifiers::NONE) }
fn default_parsed_filter() -> (KeyCode, KeyModifiers) { (KeyCode::Char('F'), KeyModifiers::NONE) }
fn default_parsed_sort() -> (KeyCode, KeyModifiers) { (KeyCode::Char('S'), KeyModifiers::NONE) }
fn default_parsed_bedrock_filter() -> (KeyCode, KeyModifiers) { (KeyCode::Char('f'), KeyModifiers::NONE) }
fn default_parsed_bedrock_sort() -> (KeyCode, KeyModifiers) { (KeyCode::Char('s'), KeyModifiers::NONE) } 