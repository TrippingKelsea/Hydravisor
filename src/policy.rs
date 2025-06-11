// src/policy.rs
// Policy Engine: Loads, interprets, and enforces security policies from policy.toml

use anyhow::Result;
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::Config;
use serde_yaml;

// Main structure for the parsed policy.toml file
// Maps to the structure defined in policy.schema.json and policy.toml.md
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(skip)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub roles: HashMap<String, RoleDefinition>,
    #[serde(default)]
    pub permissions: HashMap<String, PermissionOverride>,
    #[serde(default)]
    pub recording: SessionRecordingPolicy,
    #[serde(default)]
    pub audit: AuditPolicySettings,
    #[serde(default, rename = "session_type")]
    pub session_type_policies: HashMap<String, SessionTypePolicy>,
    pub default_network_access_policy: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Defaults {
    #[serde(default = "default_vm_resource_limits")]
    pub vm: VmResourceLimits,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            vm: default_vm_resource_limits(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VmResourceLimits {
    pub default_cpus: u32,
    pub max_cpus: u32,
    pub default_mem_mb: u64,
    pub max_mem_mb: u64,
}

fn default_vm_resource_limits() -> VmResourceLimits {
    VmResourceLimits {
        default_cpus: 1,
        max_cpus: 4,
        default_mem_mb: 2048,
        max_mem_mb: 8192,
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoleDefinition {
    pub can_create: bool,
    pub can_destroy: bool,
    pub can_attach_terminal: bool,
    pub audited: bool,
    pub session_recording: Option<bool>,
    pub allowed_commands: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PermissionOverride {
    pub role: String,
    #[serde(rename = "override")]
    pub override_settings: Option<OverrideSettings>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OverrideSettings {
    pub can_create: Option<bool>,
    pub can_attach_terminal: Option<bool>,
    pub audited: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionRecordingPolicy {
    pub record_by_default: bool,
    pub record_for_roles: Vec<String>,
}

impl Default for SessionRecordingPolicy {
    fn default() -> Self {
        Self {
            record_by_default: false,
            record_for_roles: vec!["admin".to_string(), "audited_user".to_string()],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AuditPolicySettings {
    pub log_denied: bool,
    pub log_approved_for_roles: Vec<String>,
}

impl Default for AuditPolicySettings {
    fn default() -> Self {
        Self {
            log_denied: true,
            log_approved_for_roles: vec!["admin".to_string()],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SessionTypePolicy {
    pub allow_all_network: Option<bool>,
    pub network_access: Option<Vec<NetworkRule>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NetworkRule {
    pub allow: Option<bool>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub protocol: Option<String>,
}

pub struct PolicyEngine {
    pub config: PolicyConfig,
}

impl PolicyEngine {
    pub fn load(config: &Config) -> Result<Self> {
        debug!("Loading policy from config");

        let policy_config = if let Some(policy_path) = &config.policy_file_path {
            let path = Path::new(policy_path);
            if path.exists() {
                let policy_str = std::fs::read_to_string(path)?;
                serde_yaml::from_str(&policy_str)?
            } else {
                warn!("Policy file path specified but not found at {:?}. Using default policy.", policy_path);
                PolicyConfig::default()
            }
        } else {
            warn!("Policy file not specified. Using default policy.");
            PolicyConfig::default()
        };
        
        debug!("Final policy loaded: {:?}", policy_config);
        
        Ok(PolicyEngine {
            config: policy_config,
        })
    }

    // The following methods are not used and will be removed.

    // pub fn check_permission(&self, request: &AuthRequest) -> Result<AuthDecision> { ... }
    // fn get_role_definition_and_overrides(...) -> Result<(...)> { ... }
    // fn determine_effective_role_and_settings(...) -> Result<(...)> { ... }
    // pub fn get_default_vm_limits(&self) -> &VmResourceLimits { ... }
    // pub fn should_record_session(&self, effective_role: &str) -> bool { ... }
    // pub fn get_session_recording_config(&self) -> &SessionRecordingPolicy { ... }
    // pub fn get_audit_settings(&self) -> &AuditPolicySettings { ... }
    // fn evaluate_network_policy(...) -> Result<()> { ... }
}

// TODO: Add tests for PolicyEngine:
// - Loading policy.toml (valid, missing, malformed).
// - Default values being applied correctly.
// - `check_permission` for various ActionTypes and agent roles/overrides.
//   - Test trusted, sandboxed, audited roles.
//   - Test agent-specific overrides.
//   - Test interaction with VmPolicyContext and precedence rules.
// - `determine_effective_role_and_settings` logic.
// - `get_default_vm_limits` and `should_record_session`.
