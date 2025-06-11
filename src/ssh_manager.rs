// src/ssh_manager.rs
// Manages SSH key generation, distribution, and configuration (ssh.toml)

use anyhow::{Context, Result};
use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct SshConfig {
    #[serde(default)]
    pub hosts: HashMap<String, SshHostConfigEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SshHostConfigEntry {
    pub address: String,
    pub username: String,
    pub identity_file: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    #[serde(default = "default_true")]
    pub host_key_check: bool,
    #[serde(default)]
    pub forward_agent: bool,
    pub connect_timeout: Option<u64>,
    pub session_timeout: Option<u64>,
}

fn default_ssh_port() -> u16 {
    22
}

fn default_true() -> bool {
    true
}

pub struct SshManager {
    pub config: SshConfig,
}

impl SshManager {
    pub fn load(config: &Config) -> Result<Self> {
        let ssh_config = if let Some(path) = &config.ssh_config_file_path {
            if path.exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read SSH config file at {:?}", path))?;
                toml::from_str(&content)
                    .with_context(|| format!("Failed to parse SSH config file at {:?}", path))?
            } else {
                SshConfig::default()
            }
        } else {
            SshConfig::default()
        };
        Ok(SshManager {
            config: ssh_config,
        })
    }
}

// TODO: Add tests for SshManager:
// - Loading ssh.toml (valid, missing, malformed).
// - Default values for SshHostConfigEntry.
// - `provision_vm_keys` (mock filesystem or use temp dirs, verify key file creation).
// - `get_ssh_connection_info` (path expansion, correct defaults).
// - `get_vm_ssh_details`. 