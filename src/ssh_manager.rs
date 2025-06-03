// src/ssh_manager.rs
// Manages SSH key generation, distribution, and configuration (ssh.toml)

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt; // Added for Permissions::from_mode
use tracing::{debug, info, warn};

use crate::config::Config as AppConfig; // Renamed to avoid conflict with SshConfig below
use crate::errors::HydraError;

// Structure for the parsed ssh.toml file
#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct SshConfig {
    #[serde(default)]
    pub hosts: HashMap<String, SshHostConfigEntry>, // Key is host alias, e.g., "foo-vm"
    #[serde(default, rename = "vm")]
    pub vm_specific_configs: HashMap<String, VmSshDetails>, // Key is vm-name

    #[serde(skip)]
    pub source_path: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SshHostConfigEntry {
    pub address: String,
    pub port: Option<u16>,
    pub username: String,
    pub identity_file: String,
    #[serde(default = "default_true")]
    pub host_key_check: bool,
    #[serde(default = "default_false")]
    pub forward_agent: bool,
    pub connect_timeout: Option<u32>,
    pub session_timeout: Option<u32>,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct VmSshDetails {
    #[serde(default)]
    pub trusted_agents: Vec<String>,
    pub custom_keys: Option<CustomKeys>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CustomKeys {
    pub host: String,
    pub client: String,
}

pub struct SshManager {
    pub config: SshConfig,
}

#[derive(Debug, Clone)]
pub struct SshConnectionInfo {
    pub host_alias: String,
    pub address: String,
    pub port: u16,
    pub username: String,
    pub identity_file_path: PathBuf,
    pub host_key_check: bool,
    pub forward_agent: bool,
    pub connect_timeout_sec: Option<u32>,
}

impl SshManager {
    pub fn load(app_config: &AppConfig) -> Result<Self> {
        let ssh_config_path = match &app_config.ssh_config_file_path {
            Some(path) => path.clone(),
            None => {
                let xdg_dirs = xdg::BaseDirectories::with_prefix(super::config::APP_NAME)?;
                xdg_dirs.find_config_file(super::config::DEFAULT_SSH_CONFIG_FILENAME)
                    .unwrap_or_else(|| {
                        info!("Optional ssh.toml not specified and not found in XDG directory. SSH features relying on it might be limited.");
                        xdg_dirs.get_config_home().join("nonexistent_ssh.toml")
                    })
            }
        };

        info!("Attempting to load SSH configuration from: {:?}", ssh_config_path);

        let mut loaded_ssh_config: SshConfig = if ssh_config_path.exists() {
            let ssh_config_str = std::fs::read_to_string(&ssh_config_path)
                .with_context(|| format!("Failed to read ssh.toml file: {:?}", ssh_config_path))?;
            toml::from_str(&ssh_config_str)
                .with_context(|| format!("Failed to parse TOML from ssh.toml file: {:?}", ssh_config_path))?
        } else {
            info!("ssh.toml not found at {:?}. Using default empty SSH configuration.", ssh_config_path);
            SshConfig::default()
        };
        loaded_ssh_config.source_path = ssh_config_path.to_str().map(String::from);

        debug!("SSH Config loaded: {:?}", loaded_ssh_config);

        Ok(SshManager { config: loaded_ssh_config })
    }

    pub fn provision_vm_keys(&self, vm_name: &str, xdg_config_home: &Path) -> Result<(PathBuf, PathBuf)> {
        let keys_base_dir = xdg_config_home.join(super::config::APP_NAME).join("keys.d");
        let vm_key_dir = keys_base_dir.join(vm_name);
        std::fs::create_dir_all(&vm_key_dir)
            .with_context(|| format!("Failed to create directory for VM keys: {:?}", vm_key_dir))?;

        let client_key_path = vm_key_dir.join(format!("id_ed25519_{}_client", vm_name));
        let host_key_path = vm_key_dir.join(format!("id_ed25519_{}_host", vm_name));

        if !client_key_path.exists() {
            std::fs::write(&client_key_path, "placeholder_client_private_key_content")
                .with_context(|| format!("Failed to write placeholder client key: {:?}", client_key_path))?;
            std::fs::set_permissions(&client_key_path, std::fs::Permissions::from_mode(0o600))?;
        }
        if !host_key_path.exists() {
            std::fs::write(&host_key_path, "placeholder_host_private_key_content")
                .with_context(|| format!("Failed to write placeholder host key: {:?}", host_key_path))?;
             std::fs::set_permissions(&host_key_path, std::fs::Permissions::from_mode(0o600))?;
        }

        info!("Provisioned/Ensured keys for VM '{}' at {:?}", vm_name, vm_key_dir);
        todo!("Implement actual ED25519 key generation for client and host pairs instead of placeholders. Also generate .pub files.");
        // Ok((client_key_path, host_key_path))
    }

    pub fn get_ssh_connection_info(&self, host_alias: &str) -> Result<Option<SshConnectionInfo>> {
        if let Some(host_entry) = self.config.hosts.get(host_alias) {
            let identity_file_expanded = shellexpand::tilde(&host_entry.identity_file).into_owned();
            let identity_file_path = PathBuf::from(identity_file_expanded);

            if !identity_file_path.exists() {
                warn!("Identity file for host '{}' not found at resolved path: {:?}. SSH might fail or rely on agent.", host_alias, identity_file_path);
            }

            Ok(Some(SshConnectionInfo {
                host_alias: host_alias.to_string(),
                address: host_entry.address.clone(),
                port: host_entry.port.unwrap_or(22),
                username: host_entry.username.clone(),
                identity_file_path,
                host_key_check: host_entry.host_key_check,
                forward_agent: host_entry.forward_agent,
                connect_timeout_sec: host_entry.connect_timeout,
            }))
        } else {
            info!("No specific SSH config found in ssh.toml for host_alias: '{}'. SSH will use system defaults / agent.", host_alias);
            Ok(None)
        }
    }

    pub fn get_vm_ssh_details(&self, vm_name: &str) -> Option<&VmSshDetails> {
        self.config.vm_specific_configs.get(vm_name)
    }

    // TODO: Add functions for:
    // - Key rotation (as per ssh.design.md - future feature)
    // - Key revocation
    // - Managing the encrypted key store (future feature)
}

// TODO: Add tests for SshManager:
// - Loading ssh.toml (valid, missing, malformed).
// - Default values for SshHostConfigEntry.
// - `provision_vm_keys` (mock filesystem or use temp dirs, verify key file creation).
// - `get_ssh_connection_info` (path expansion, correct defaults).
// - `get_vm_ssh_details`. 