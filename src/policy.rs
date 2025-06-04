// src/policy.rs
// Policy Engine: Loads, interprets, and enforces security policies from policy.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::config::Config as AppConfig; // Renamed to avoid conflict with PolicyConfig
use crate::errors::HydraError;

// Main structure for the parsed policy.toml file
// Maps to the structure defined in policy.schema.json and policy.toml.md
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(default)]
    pub roles: HashMap<String, RoleDefinition>,
    #[serde(default)]
    pub permissions: HashMap<String, AgentPermissionOverride>, // Key: e.g., "agent::b312a9f8"
    #[serde(default)]
    pub audit: AuditPolicySettings,
    #[serde(default)]
    pub defaults: DefaultVmSettings,
    #[serde(default)]
    pub recording: SessionRecordingPolicy,
    
    // Not part of the TOML file itself, but where it was loaded from
    #[serde(skip_serializing)]
    pub source_path: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RoleDefinition {
    pub can_create: bool,
    pub can_attach_terminal: bool,
    pub audited: bool,
    // TODO: Add more granular capabilities here as per future design, e.g.:
    // pub allowed_commands: Option<Vec<String>>,
    // pub network_access: Option<String>, // e.g., "full", "restricted", "none"
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AgentPermissionOverride {
    pub role: String, // Reference to a key in [roles]
    pub override_settings: Option<OverrideSettings>, // Field name adjusted for clarity
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OverrideSettings { // Corresponds to `override` in TOML
    pub can_create: Option<bool>,
    pub can_attach_terminal: Option<bool>,
    pub audited: Option<bool>, // Allow overriding audited status too
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AuditPolicySettings {
    #[serde(default = "default_true")] // log_denied defaults to true
    pub log_denied: bool,
    #[serde(default)]
    pub log_approved_for_roles: Vec<String>,
    #[serde(default = "default_audit_log_path")]
    pub log_path: String,
}

fn default_audit_log_path() -> String { "hydravisor/logs/audit.jsonl".to_string() }

impl Default for AuditPolicySettings {
    fn default() -> Self {
        AuditPolicySettings {
            log_denied: true,
            log_approved_for_roles: vec!["audited".to_string()],
            log_path: default_audit_log_path(),
        }
    }
}


#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DefaultVmSettings {
    #[serde(default)]
    pub vm: VmResourceLimits, // Corresponds to [defaults.vm] in TOML
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VmResourceLimits {
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: u32,
    #[serde(default = "default_ram_limit")]
    pub ram_limit: String,
    #[serde(default = "default_disk_limit")]
    pub disk_limit: String,
    #[serde(default)] // networking defaults to false
    pub networking: bool,
}

fn default_cpu_limit() -> u32 { 2 }
fn default_ram_limit() -> String { "4GB".to_string() }
fn default_disk_limit() -> String { "16GB".to_string() }
fn default_true() -> bool { true }

impl Default for VmResourceLimits {
    fn default() -> Self {
        VmResourceLimits {
            cpu_limit: default_cpu_limit(),
            ram_limit: default_ram_limit(),
            disk_limit: default_disk_limit(),
            networking: false,
        }
    }
}

impl Default for DefaultVmSettings { // For the outer [defaults] table
    fn default() -> Self {
        DefaultVmSettings { vm: VmResourceLimits::default() }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SessionRecordingPolicy {
    #[serde(default = "default_record_for_roles")]
    pub record_for_roles: Vec<String>,
    #[serde(default = "default_true")] // include_model_dialog defaults to true
    pub include_model_dialog: bool,
    #[serde(default = "default_recording_log_dir")]
    pub log_dir: String,
    #[serde(default)]
    pub redact_patterns: Vec<String>,
}

fn default_record_for_roles() -> Vec<String> { vec!["sandboxed".to_string(), "audited".to_string()] }
fn default_recording_log_dir() -> String { "hydravisor/logs/sessions".to_string() }

impl Default for SessionRecordingPolicy {
    fn default() -> Self {
        SessionRecordingPolicy {
            record_for_roles: default_record_for_roles(),
            include_model_dialog: true,
            log_dir: default_recording_log_dir(),
            redact_patterns: Vec::new(),
        }
    }
}


pub struct PolicyEngine {
    pub config: PolicyConfig,
    // TODO: Potentially a reference to AuditEngine for logging policy decisions/violations
    // pub audit_engine: Arc<AuditEngine>,
}

// Represents a request to check permissions for a specific action
#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub agent_id: String,
    pub agent_role_hint: Option<String>, // Optional: If the agent already declared a role
    pub action: ActionType,
    pub resource_id: Option<String>, // e.g., VM ID, container ID, or specific API endpoint
    pub vm_policy_context: Option<VmPolicyContext>, // Policy attributes of the target VM (e.g. `trusted` flag from policy.toml)
}

// Simplified context from a VM's policy definition in policy.toml
#[derive(Debug, Clone)]
pub struct VmPolicyContext {
    pub is_trusted_vm: bool,
    pub allowed_agents_for_vm: Option<Vec<String>>,
    // Add other VM-specific policy attributes as needed from [vm."<name>"] section
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionType {
    CreateVm,
    CreateContainer,
    DeleteVm,
    DeleteContainer,
    AttachTerminal(String), // role for attachment
    ExecuteCommandInVm(String), // command string
    AccessMcpEndpoint(String), // MCP method name e.g. "vm/create"
    ViewModelLogs,
    // Add more actions as defined by interactions between components
    Generic(String), // For extensibility
}

#[derive(Debug, Clone)]
pub struct AuthDecision {
    pub allowed: bool,
    pub reason: String,
    pub effective_role: Option<String>,
    pub should_audit: bool,
}

impl PolicyEngine {
    pub fn load(app_config: &AppConfig) -> Result<Self> {
        let policy_path = match &app_config.policy_file_path {
            Some(path) => path.clone(),
            None => {
                // If not found via main config, try default XDG path explicitly again
                let xdg_dirs = xdg::BaseDirectories::with_prefix(super::config::APP_NAME)?;
                xdg_dirs.find_config_file(super::config::DEFAULT_POLICY_FILENAME)
                    .unwrap_or_else(|| {
                        warn!("Policy file not specified and not found in XDG directory. Using empty default policy.");
                        // Return a path that won't exist to load default, or handle more gracefully
                        xdg_dirs.get_config_home().join("nonexistent_policy.toml")
                    })
            }
        };

        info!("Attempting to load policy configuration from: {:?}", policy_path);

        let mut loaded_policy_config: PolicyConfig = if policy_path.exists() {
            let policy_str = std::fs::read_to_string(&policy_path)
                .with_context(|| format!("Failed to read policy file: {:?}", policy_path))?;
            toml::from_str(&policy_str)
                .with_context(|| format!("Failed to parse TOML from policy file: {:?}", policy_path))?
        } else {
            warn!("Policy file not found at {:?}. Using empty default policy configuration.", policy_path);
            PolicyConfig::default()
        };
        loaded_policy_config.source_path = policy_path.to_str().map(String::from);

        // TODO: Validate policy (e.g. roles in overrides must exist in [roles])
        // This could be a separate `validate(&self) -> Result<()>` method.
        debug!("Policy loaded: {:?}", loaded_policy_config);

        Ok(PolicyEngine {
            config: loaded_policy_config,
        })
    }

    pub fn check_permission(&self, request: &AuthRequest) -> Result<AuthDecision> {
        // TODO: Implement comprehensive permission checking based on policy.toml.md and agent-flow.design.md:
        // 1. Determine the agent's effective role:
        //    - Check agent-specific permissions in `[permissions."agent::{agent_id}"]`.
        //    - If an override exists, use its role and specific permission settings.
        //    - Fallback to the `agent_role_hint` or a default role (e.g., "sandboxed") if no specific config.
        //    - If role still unknown, deny.
        // 2. Get the `RoleDefinition` for the effective role.
        // 3. Evaluate based on `ActionType`:
        //    - `CreateVm`/`CreateContainer`: Check `can_create`.
        //    - `AttachTerminal`: Check `can_attach_terminal`.
        //    - Other actions: Need more granular capabilities in `RoleDefinition` or a mapping from ActionType to specific booleans.
        //      (This is where `policy.toml.md`'s "Role vs Command Matrix" comes in, needs formalizing in struct)
        // 4. Consider VM-specific context (`request.vm_policy_context`):
        //    - If `vm.trusted = false` in policy, it might impose stricter rules or deny certain agent roles.
        //    - If `vm.agents` list exists, is the requesting agent_id in it?
        //    - The precedence table from `policy.toml.md` / `agent-flow.design.md` for host vs agent policy needs to be applied.
        // 5. Determine `should_audit` based on `role.audited` or `audit.log_denied` / `audit.log_approved_for_roles`.
        // 6. Construct and return `AuthDecision`.

        println!("Checking permission for request: {:?}", request);
        println!("Using policy config: {:?}", self.config);

        // Placeholder logic:
        let effective_role_name = self.determine_effective_role_and_settings(request)?.0;
        let (role_def, _overridden_settings) = self.get_role_definition_and_overrides(&effective_role_name, &request.agent_id)?;
        
        let mut allowed = false;
        let mut reason = "Default deny".to_string();

        match request.action {
            ActionType::CreateVm | ActionType::CreateContainer => {
                allowed = role_def.can_create;
                reason = if allowed { "Agent role allows creation".to_string() } else { "Agent role does not allow creation".to_string() };
            }
            ActionType::AttachTerminal(_) => {
                allowed = role_def.can_attach_terminal;
                reason = if allowed { "Agent role allows terminal attach".to_string() } else { "Agent role does not allow terminal attach".to_string() };
            }
            _ => {
                reason = "Action type not yet specifically handled by policy engine placeholder".to_string();
                // allowed remains false
            }
        }
        
        // TODO: Implement VM policy context checks and precedence (agent-flow.design.md matrix)
        // This is a very simplified check and needs to incorporate the host-side policy for the VM.

        let should_audit = role_def.audited || (!allowed && self.config.audit.log_denied) || (allowed && self.config.audit.log_approved_for_roles.contains(&effective_role_name));

        Ok(AuthDecision {
            allowed,
            reason,
            effective_role: Some(effective_role_name),
            should_audit,
        })
    }

    // Helper to get the actual RoleDefinition and any specific agent overrides
    fn get_role_definition_and_overrides(&self, role_name: &str, agent_id: &str) -> Result<(&RoleDefinition, Option<OverrideSettings>)> {
        let agent_permission_key = format!("agent::{}", agent_id);
        let agent_override = self.config.permissions.get(&agent_permission_key);

        let final_role_name = agent_override.map_or(role_name, |ovr| &ovr.role);
        let specific_settings_override = agent_override.and_then(|ovr| ovr.override_settings.clone());

        let role_def = self.config.roles.get(final_role_name)
            .ok_or_else(|| HydraError::PolicyError(format!("Role '{}' not found in policy configuration for agent {}", final_role_name, agent_id)))?;
        
        Ok((role_def, specific_settings_override))
    }
    
    // Helper to determine effective role and its settings, considering overrides
    fn determine_effective_role_and_settings(&self, request: &AuthRequest) -> Result<(String, RoleDefinition)> {
        let agent_permission_key = format!("agent::{}", request.agent_id);
        
        if let Some(agent_perm_override) = self.config.permissions.get(&agent_permission_key) {
            let base_role_name = &agent_perm_override.role;
            let base_role_def = self.config.roles.get(base_role_name)
                .ok_or_else(|| HydraError::PolicyError(format!("Base role '{}' for agent override '{}' not found.", base_role_name, request.agent_id)))?;
            
            let mut effective_settings = base_role_def.clone();
            if let Some(overrides) = &agent_perm_override.override_settings {
                if let Some(val) = overrides.can_create { effective_settings.can_create = val; }
                if let Some(val) = overrides.can_attach_terminal { effective_settings.can_attach_terminal = val; }
                if let Some(val) = overrides.audited { effective_settings.audited = val; }
            }
            return Ok((base_role_name.clone(), effective_settings)); // Return the name of the *base* role for audit logging clarity
        }

        if let Some(hinted_role) = &request.agent_role_hint {
            if let Some(role_def) = self.config.roles.get(hinted_role) {
                return Ok((hinted_role.clone(), role_def.clone()));
            }
        }
        
        // Fallback to a default role if defined, or deny. For now, let's assume a "default_sandboxed" role or similar might exist.
        // Or, more strictly, if no role is found, it's an error or implicit deny.
        // For this placeholder, we'll require a role to be determinable.
        Err(HydraError::PolicyError(format!("Could not determine effective role for agent '{}'. No specific override and hint '{:?}' not found or invalid.", request.agent_id, request.agent_role_hint)).into())
    }

    pub fn get_default_vm_limits(&self) -> &VmResourceLimits {
        &self.config.defaults.vm
    }

    pub fn should_record_session(&self, effective_role: &str) -> bool {
        self.config.recording.record_for_roles.contains(&effective_role.to_string())
    }
    
    pub fn get_session_recording_config(&self) -> &SessionRecordingPolicy {
        &self.config.recording
    }
    
    pub fn get_audit_settings(&self) -> &AuditPolicySettings {
        &self.config.audit
    }

    // TODO: Add a `validate_policy_config(config: &PolicyConfig) -> Result<()>` function for use by CLI `policy validate`.
    // This would check for internal consistency (e.g., roles in overrides exist).
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
