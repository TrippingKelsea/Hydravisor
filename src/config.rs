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

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BedrockConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_bedrock_region")]
    pub region: String,
    #[serde(default = "default_bedrock_profile")]
    pub profile: String,
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
    14
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
    "hydravisor-".to_string()
}
fn default_record_all_sessions() -> bool {
    true
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
    "/tmp/hydravisor.sock".to_string()
}
fn default_mcp_timeout_ms() -> u64 {
    3000
}
fn default_mcp_heartbeat_interval() -> u64 {
    15
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
            policy_file_path: None,
            ssh_config_file_path: None,
            ollama_host: Some("http://localhost".to_string()),
            ollama_port: Some(11434),
            default_system_prompt: default_global_system_prompt(),
        }
    }
}

impl Config {
    pub fn load(config_path_override: Option<&Path>) -> Result<Self> {
        let xdg_dirs = BaseDirectories::with_prefix(APP_NAME)?;
        let config_path = match config_path_override {
            Some(path) => path.to_path_buf(),
            None => xdg_dirs
                .find_config_file(DEFAULT_CONFIG_FILENAME)
                .unwrap_or_else(|| xdg_dirs.get_config_home().join(DEFAULT_CONFIG_FILENAME)),
        };

        info!("Attempting to load configuration from: {:?}", config_path);

        let mut loaded_config: Config = if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
            toml::from_str(&config_str)
                .with_context(|| format!("Failed to parse TOML from config file: {:?}", config_path))?
        } else {
            warn!("Config file not found at {:?}. Using default configuration.", config_path);
            Config::default()
        };

        // Resolve paths for other config files (policy.toml, ssh.toml)
        // These are expected to be in the same directory as config.toml or XDG config dir
        let config_dir: PathBuf = config_path.parent().map(Path::to_path_buf).unwrap_or_else(|| xdg_dirs.get_config_home());

        let policy_path = config_dir.join(DEFAULT_POLICY_FILENAME);
        if policy_path.exists() {
            debug!("Found policy file at: {:?}", policy_path);
            loaded_config.policy_file_path = Some(policy_path);
        } else {
            warn!("Policy file ({}) not found in config directory {:?}. Policy features may be limited.", DEFAULT_POLICY_FILENAME, config_dir);
        }

        let ssh_config_path = config_dir.join(DEFAULT_SSH_CONFIG_FILENAME);
        if ssh_config_path.exists() {
            debug!("Found SSH config file at: {:?}", ssh_config_path);
            loaded_config.ssh_config_file_path = Some(ssh_config_path);
        } else {
            // This might be fine, as SSH can fall back to system/user ~/.ssh/config
            info!("Optional SSH config file ({}) not found in config directory {:?}. Will use system SSH config.", DEFAULT_SSH_CONFIG_FILENAME, config_dir);
        }
        
        // TODO: Expand paths like ~/ in log_dir, etc.
        // Example: loaded_config.logging.log_dir = shellexpand::tilde(&loaded_config.logging.log_dir).into_owned();

        Ok(loaded_config)
    }

    pub fn save(&self) -> Result<()> {
        let xdg_dirs = BaseDirectories::with_prefix(APP_NAME)
            .context("Failed to get XDG base directories")?;
        
        // Determine config directory and ensure it exists
        let config_home = xdg_dirs.get_config_home();
        if !config_home.exists() {
            std::fs::create_dir_all(&config_home).with_context(|| 
                format!("Failed to create config directory at {:?}", config_home)
            )?;
            info!("Created config directory at {:?}", config_home);
        }

        let config_path = config_home.join(DEFAULT_CONFIG_FILENAME);
        info!("Attempting to save configuration to: {:?}", config_path);

        // Create a clone of self to modify for serialization if needed,
        // especially to ensure skipped fields in `load` are not attempted to be saved
        // if they were part of the struct. In our case, policy_file_path and ssh_config_file_path
        // are already marked with #[serde(skip)] for serialization, so a direct clone is fine.
        let config_to_save = self.clone();

        let toml_string = toml::to_string_pretty(&config_to_save)
            .context("Failed to serialize config to TOML string")?;
        
        std::fs::write(&config_path, toml_string)
            .with_context(|| format!("Failed to write config to file: {:?}", config_path))?;
        
        info!("Configuration saved successfully to {:?}", config_path);
        Ok(())
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