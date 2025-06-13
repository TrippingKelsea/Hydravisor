// src/config.rs
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use xdg::BaseDirectories;

pub const APP_NAME: &str = "hydravisor";
pub const DEFAULT_CONFIG_FILENAME: &str = "config.toml";
pub const DEFAULT_POLICY_FILENAME: &str = "policy.toml";
pub const DEFAULT_SSH_CONFIG_FILENAME: &str = "ssh.toml";

// Main configuration structure, mapping to config.toml
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub interface: InterfaceConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub tmux: TmuxConfig,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub keybindings: KeyBindingsConfig,
    // Paths to other config files, not part of config.toml itself
    // but resolved during Config::load
    #[serde(skip)]
    pub policy_file_path: Option<PathBuf>,
    #[serde(skip)]
    pub ssh_config_file_path: Option<PathBuf>,
    pub ollama_host: Option<String>,
    pub ollama_port: Option<u16>,
    #[serde(default = "default_global_system_prompt")]
    pub default_system_prompt: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InterfaceConfig {
    #[serde(default = "default_interface_mode")]
    pub mode: String, // "session" or "modal"
    #[serde(default = "default_modal_key")]
    pub modal_key: String,
    #[serde(default = "default_refresh_interval_ms")]
    pub refresh_interval_ms: u64,
    #[serde(default = "default_about_modal_readme_lines")]
    pub about_modal_readme_lines: usize,
}

fn default_interface_mode() -> String {
    "session".to_string()
}
fn default_modal_key() -> String {
    "9".to_string()
}
fn default_refresh_interval_ms() -> u64 {
    500
}

fn default_about_modal_readme_lines() -> usize {
    10
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        InterfaceConfig {
            mode: default_interface_mode(),
            modal_key: default_modal_key(),
            refresh_interval_ms: default_refresh_interval_ms(),
            about_modal_readme_lines: default_about_modal_readme_lines(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DefaultsConfig {
    #[serde(default = "default_vm_image")]
    pub default_vm_image: String,
    #[serde(default = "default_vm_iso")]
    pub default_vm_iso: String,
    #[serde(default)]
    pub default_source_image: Option<String>,
    #[serde(default = "default_container_image")]
    pub default_container_image: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_cpu")]
    pub default_cpu: u32,
    #[serde(default = "default_ram")]
    pub default_ram: String, // e.g., "4GB"
    #[serde(default = "default_disk_gb")]
    pub default_disk_gb: u64,
}

fn default_vm_image() -> String {
    "archlinux-2025.04.01".to_string()
}
fn default_vm_iso() -> String {
    "/mnt/DiskImages/archlinux-2025.04.01-x86_64.iso".to_string()
}
fn default_container_image() -> String {
    "ghcr.io/hydravisor/agent:latest".to_string()
}
fn default_model() -> String {
    "ollama:qwen3".to_string()
}
fn default_cpu() -> u32 {
    2
}
fn default_ram() -> String {
    "4GB".to_string()
}
fn default_disk_gb() -> u64 {
    20
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        DefaultsConfig {
            default_vm_image: default_vm_image(),
            default_vm_iso: default_vm_iso(),
            default_source_image: None,
            default_container_image: default_container_image(),
            default_model: default_model(),
            default_cpu: default_cpu(),
            default_ram: default_ram(),
            default_disk_gb: default_disk_gb(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub bedrock: BedrockConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OllamaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ollama_path")]
    pub path: String, // Path to ollama binary
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub model_system_prompts: Option<HashMap<String, String>>,
}

fn default_ollama_path() -> String {
    "/usr/local/bin/ollama".to_string() // A common default, might need adjustment
}

impl Default for OllamaConfig {
    fn default() -> Self {
        OllamaConfig {
            enabled: true, // Often true by default for local-first approach
            path: default_ollama_path(),
            models: vec!["llama3".to_string(), "mistral".to_string()],
            model_system_prompts: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct BedrockFiltersConfig {
    #[serde(default = "default_bedrock_filter_name")] pub default: String,
    #[serde(default)] pub available_to_request_access: Option<BedrockFilterDefinition>,
    #[serde(default)] pub available_to_use: Option<BedrockFilterDefinition>,
}

fn default_bedrock_filter_name() -> String { "available_to_use".to_string() }

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct BedrockFilterDefinition {
    pub description: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BedrockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_bedrock_region")]
    pub region: String,
    #[serde(default = "default_bedrock_profile")]
    pub profile: String,
    #[serde(default)]
    pub filters: BedrockFiltersConfig,
}

fn default_bedrock_region() -> String {
    "us-west-2".to_string()
}
fn default_bedrock_profile() -> String {
    "default".to_string()
}

impl Default for BedrockConfig {
    fn default() -> Self {
        BedrockConfig {
            enabled: false, // Typically opt-in
            region: default_bedrock_region(),
            profile: default_bedrock_profile(),
            filters: BedrockFiltersConfig {
                default: default_bedrock_filter_name(),
                available_to_request_access: Some(BedrockFilterDefinition {
                    description: "Models you can request access to".to_string(),
                }),
                available_to_use: Some(BedrockFilterDefinition {
                    description: "Models you can use now".to_string(),
                }),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String, // "debug", "info", "warn", "error"
    #[serde(default = "default_log_dir")]
    pub log_dir: String, // Path, can use ~
    #[serde(default = "default_rotate_daily")]
    pub rotate_daily: bool,
    #[serde(default = "default_retain_days")]
    pub retain_days: u32,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_dir() -> String {
    "~/.hydravisor/logs".to_string() // Using ~ for home dir, will need expansion
}
fn default_rotate_daily() -> bool {
    true
}
fn default_retain_days() -> u32 {
    7
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: default_log_level(),
            log_dir: default_log_dir(),
            rotate_daily: default_rotate_daily(),
            retain_days: default_retain_days(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TmuxConfig {
    #[serde(default = "default_session_prefix")]
    pub session_prefix: String,
    #[serde(default = "default_record_all_sessions")]
    pub record_all_sessions: bool,
    #[serde(default = "default_record_format")]
    pub record_format: String, // "ansi" or "jsonl"
    #[serde(default = "default_autosave_on_exit")]
    pub autosave_on_exit: bool,
}

fn default_session_prefix() -> String {
    "hydra-".to_string()
}
fn default_record_all_sessions() -> bool {
    false
}
fn default_record_format() -> String {
    "ansi".to_string()
}
fn default_autosave_on_exit() -> bool {
    true
}

impl Default for TmuxConfig {
    fn default() -> Self {
        TmuxConfig {
            session_prefix: default_session_prefix(),
            record_all_sessions: default_record_all_sessions(),
            record_format: default_record_format(),
            autosave_on_exit: default_autosave_on_exit(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct McpConfig {
    #[serde(default = "default_mcp_socket_path")]
    pub socket_path: String,
    #[serde(default = "default_mcp_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_mcp_heartbeat_interval")]
    pub heartbeat_interval: u64, // in seconds
}

fn default_mcp_socket_path() -> String {
    "/tmp/hydravisor_mcp.sock".to_string()
}
fn default_mcp_timeout_ms() -> u64 {
    2000
}
fn default_mcp_heartbeat_interval() -> u64 {
    30
}

impl Default for McpConfig {
    fn default() -> Self {
        McpConfig {
            socket_path: default_mcp_socket_path(),
            timeout_ms: default_mcp_timeout_ms(),
            heartbeat_interval: default_mcp_heartbeat_interval(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct KeyBindingsConfig {
    #[serde(default = "default_quit")] pub quit: String,
    #[serde(default = "default_help")] pub help: String,
    #[serde(default = "default_menu")] pub menu: String,
    #[serde(default = "default_next_tab")] pub next_tab: String,
    #[serde(default = "default_prev_tab")] pub prev_tab: String,
    #[serde(default = "default_new_vm")] pub new_vm: String,
    #[serde(default = "default_destroy_vm")] pub destroy_vm: String,
    #[serde(default = "default_edit")] pub edit: String,
    #[serde(default = "default_enter")] pub enter: String,
    #[serde(default = "default_up")] pub up: String,
    #[serde(default = "default_down")] pub down: String,
    #[serde(default = "default_filter")] pub filter: String,
    #[serde(default = "default_sort")] pub sort: String,
    #[serde(default)]
    pub bedrock: BedrockKeyBindings,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BedrockKeyBindings {
    #[serde(default = "default_bedrock_filter")]
    pub filter: String,
    #[serde(default = "default_bedrock_sort")]
    pub sort: String,
}

fn default_bedrock_filter() -> String { "f".to_string() }
fn default_bedrock_sort() -> String { "s".to_string() }

impl Default for BedrockKeyBindings {
    fn default() -> Self {
        Self {
            filter: default_bedrock_filter(),
            sort: default_bedrock_sort(),
        }
    }
}

fn default_quit() -> String { "q".to_string() }
fn default_help() -> String { "?".to_string() }
fn default_menu() -> String { "Ctrl+h".to_string() }
fn default_next_tab() -> String { "Tab".to_string() }
fn default_prev_tab() -> String { "BackTab".to_string() }
fn default_new_vm() -> String { "n".to_string() }
fn default_destroy_vm() -> String { "d".to_string() }
fn default_edit() -> String { "e".to_string() }
fn default_enter() -> String { "Enter".to_string() }
fn default_up() -> String { "Up".to_string() }
fn default_down() -> String { "Down".to_string() }
fn default_filter() -> String { "F".to_string() }
fn default_sort() -> String { "S".to_string() }

impl Default for KeyBindingsConfig {
    fn default() -> Self {
        Self {
            quit: default_quit(),
            help: default_help(),
            menu: default_menu(),
            next_tab: default_next_tab(),
            prev_tab: default_prev_tab(),
            new_vm: default_new_vm(),
            destroy_vm: default_destroy_vm(),
            edit: default_edit(),
            enter: default_enter(),
            up: default_up(),
            down: default_down(),
            filter: default_filter(),
            sort: default_sort(),
            bedrock: BedrockKeyBindings::default(),
        }
    }
}

fn default_global_system_prompt() -> Option<String> {
    Some("You are a helpful AI assistant.".to_string())
}

impl Default for Config {
    fn default() -> Self {
        Config {
            interface: InterfaceConfig::default(),
            defaults: DefaultsConfig::default(),
            providers: ProvidersConfig::default(),
            logging: LoggingConfig::default(),
            tmux: TmuxConfig::default(),
            mcp: McpConfig::default(),
            keybindings: KeyBindingsConfig::default(),
            policy_file_path: None,
            ssh_config_file_path: None,
            ollama_host: None,
            ollama_port: None,
            default_system_prompt: default_global_system_prompt(),
        }
    }
}

impl Config {
    pub fn load(config_path_override: Option<&Path>) -> Result<Self> {
        let xdg_dirs = BaseDirectories::with_prefix(APP_NAME)?;
        let config_path = match config_path_override {
            Some(path) => {
                debug!("Using provided config path override: {}", path.display());
                path.to_path_buf()
            }
            None => xdg_dirs
                .find_config_file(DEFAULT_CONFIG_FILENAME)
                .with_context(|| {
                    format!(
                        "Could not find default config file '{}'",
                        DEFAULT_CONFIG_FILENAME
                    )
                })?,
        };

        info!("Loading configuration from {}", config_path.display());
        let config_str = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {}", config_path.display()))?;

        let mut config: Config = toml::from_str(&config_str)
            .with_context(|| "Failed to parse TOML configuration")?;

        // Resolve paths for other config files relative to the main config file's directory
        let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        config.policy_file_path = xdg_dirs
            .find_config_file(DEFAULT_POLICY_FILENAME)
            .or_else(|| Some(config_dir.join(DEFAULT_POLICY_FILENAME)));
        config.ssh_config_file_path = xdg_dirs
            .find_config_file(DEFAULT_SSH_CONFIG_FILENAME)
            .or_else(|| Some(config_dir.join(DEFAULT_SSH_CONFIG_FILENAME)));

        Ok(config)
    }

    pub fn get_system_prompt_for_model(&self, model_name: &str) -> Option<String> {
        self.providers
            .ollama
            .model_system_prompts
            .as_ref()
            .and_then(|prompts| prompts.get(model_name).cloned())
    }
}

// TODO: Add tests for config loading, default values, and overrides.
// Test cases:
// 1. No config file exists -> all defaults.
// 2. Config file exists with partial overrides -> defaults + overrides.
// 3. Config file exists with all values specified.
// 4. Config file path override from CLI.
// 5. Malformed config file -> error.
// 6. Correct resolution of policy_file_path and ssh_config_file_path. 