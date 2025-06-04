// src/env_manager.rs
// Manages VMs (libvirt/KVM) and containers (containerd)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::Config;
// use crate::errors::HydraError; // Not used yet, keep for later if specific errors are needed

#[cfg(feature = "libvirt_integration")]
use virt::connect::Connect;
#[cfg(feature = "libvirt_integration")]
use virt::domain::{Domain, DomainInfo};
#[cfg(feature = "libvirt_integration")]
use virt::sys; // Import the sys module for C constants

// Represents the type of environment
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum EnvironmentType {
    #[default]
    Vm,
    Container,
}

// Configuration for creating a new environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentConfig {
    pub instance_id: String, // Unique ID for this environment instance
    pub env_type: EnvironmentType,
    pub base_image: String, // e.g., "ubuntu-22.04" or "docker.io/library/alpine:latest"
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_gb: Option<u64>,      // Primarily for VMs
    pub network_policy: String,    // Reference to a network policy name/ID
    pub security_policy: String,   // Reference to a security policy name/ID
    pub custom_script: Option<String>, // Optional bootstrap script content or path
    pub template_name: Option<String>, // Name of the template used, if any
    pub labels: Option<HashMap<String, String>>, // For tagging/metadata
}

// Represents the runtime state of an environment
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum EnvironmentState {
    Provisioning,
    Booting,
    Running,
    Suspended,
    Terminated,
    Stopped, // Cleanly shut down, can be restarted
    Error(String),
    #[default]
    Unknown,
}

// Detailed status of a running or managed environment
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EnvironmentStatus {
    pub instance_id: String, // For VMs, this could be the libvirt UUID or name
    pub name: String, // For VMs, the libvirt domain name
    pub env_type: EnvironmentType,
    pub state: EnvironmentState,
    pub ip_address: Option<String>,
    pub ssh_port: Option<u16>,
    pub created_at: String, // ISO 8601 timestamp - for VMs, libvirt might not have this directly
    pub updated_at: String, // ISO 8601 timestamp
    pub base_image: Option<String>, // May not always be known for externally created VMs
    pub cpu_cores_used: Option<u32>, // Current vCPUs (from libvirt DomainInfo)
    pub memory_max_kb: Option<u64>,   // Max memory allocated (from libvirt DomainInfo)
    pub memory_used_kb: Option<u64>, // Current memory usage (from libvirt DomainInfo)
    // pub disk_usage_gb: Option<u64>, // Harder to get generically from libvirt
    pub error_details: Option<String>,
}

pub struct EnvironmentManager {
    // config: EnvManagerConfig, // Derived from main Config
    #[cfg(feature = "libvirt_integration")]
    libvirt_conn: Option<Connect>,
    // active_environments: Mutex<HashMap<String, EnvironmentStatus>>,
    // containerd_client: Option<ContainerdClient>, // If containerd feature enabled
}

impl EnvironmentManager {
    pub fn new(app_config: &Config) -> Result<Self> {
        #[cfg(feature = "libvirt_integration")]
        let libvirt_conn = match Connect::open(Some("qemu:///system")) {
            Ok(conn) => {
                println!("Successfully connected to libvirt daemon (qemu:///system).");
                Some(conn)
            }
            Err(e) => {
                eprintln!(
                    "Failed to connect to libvirt (qemu:///system): {}. Live VM data will not be available.",
                    e
                );
                None
            }
        };
        #[cfg(not(feature = "libvirt_integration"))]
        let libvirt_conn: Option<Connect> = None; // Ensure libvirt_conn is defined even if feature is off


        println!(
            "EnvironmentManager initialized. VM Provider support: {}. Container Provider support: TODO. Config: {:?}",
            if cfg!(feature = "libvirt_integration") { "libvirt" } else { "None" },
            app_config.providers
        );

        Ok(EnvironmentManager {
            #[cfg(feature = "libvirt_integration")]
            libvirt_conn,
        })
    }

    pub fn create_environment(&self, env_config: &EnvironmentConfig) -> Result<EnvironmentStatus> {
        // TODO: Implement environment creation logic:
        // 1. Validate env_config against policies (resource limits, allowed images etc. - coordination with PolicyEngine).
        // 2. Based on env_config.env_type:
        //    - If Vm: Use libvirt (or QEMU direct commands as per interface.design.md) to define and start the VM.
        //      - Handle image download/management if not present.
        //      - Configure networking, disk, CPU, memory.
        //      - Inject SSH keys (coordination with SshManager).
        //      - Run custom_script if provided.
        //    - If Container: Use containerd client (or podman CLI as per interface.design.md) to pull image and run container.
        //      - Apply resource limits, network config.
        //      - Mount necessary volumes (e.g., for workspace, agent communication).
        // 3. Update internal state of active_environments.
        // 4. Return initial EnvironmentStatus (likely Provisioning or Booting).
        // 5. Log the creation event via AuditEngine.
        println!("Creating environment: {:?}", env_config);
        match env_config.env_type {
            EnvironmentType::Vm => todo!("Implement VM creation using libvirt/QEMU"),
            EnvironmentType::Container => todo!("Implement container creation using containerd/podman"),
        }
        // Ok(initial_status)
    }

    pub fn destroy_environment(&self, instance_id: &str) -> Result<()> {
        // TODO: Implement environment destruction:
        // 1. Find the environment by instance_id.
        // 2. Based on type:
        //    - VM: Stop/undefine the VM via libvirt/QEMU.
        //    - Container: Stop/remove the container via containerd/podman.
        // 3. Clean up associated resources (disks, network interfaces if not shared).
        // 4. Remove from active_environments.
        // 5. Log destruction event via AuditEngine.
        println!("Destroying environment: {}", instance_id);
        todo!("Implement environment destruction for VMs and containers.");
        // Ok(())
    }

    pub fn get_environment_status(&self, instance_id: &str) -> Result<Option<EnvironmentStatus>> {
        // TODO: Retrieve and return the current status of the specified environment.
        // - For VMs, query libvirt/QEMU for state, IP, resource usage.
        // - For containers, query containerd/podman.
        // - Update internal cache if necessary.
        println!("Getting status for environment: {}", instance_id);
        todo!("Implement environment status retrieval.");
        // Ok(None) // Or Some(status)
    }

    pub fn list_environments(&self) -> Result<Vec<EnvironmentStatus>> {
        // TODO: List all managed environments and their current statuses.
        println!("Listing all environments.");
        todo!("Implement listing of all environments.");
        // Ok(Vec::new())
    }

    pub fn snapshot_environment(&self, instance_id: &str, snapshot_name: &str, output_path: Option<PathBuf>) -> Result<String> {
        // TODO: Implement snapshotting for VMs (primarily).
        // - Use libvirt/QEMU snapshot capabilities.
        // - Store snapshot metadata.
        // - Optionally export to output_path (as per CLI design for `vm snapshot`).
        // - Log event.
        println!("Snapshotting environment: {} to name: {}, output: {:?}", instance_id, snapshot_name, output_path);
        todo!("Implement VM snapshotting.");
        // Ok("snapshot_id_or_path".to_string())
    }

    pub fn pause_environment(&self, instance_id: &str) -> Result<()> {
        // TODO: Implement pausing for VMs/containers that support it.
        println!("Pausing environment: {}", instance_id);
        todo!("Implement environment pausing.");
        // Ok(())
    }

    pub fn resume_environment(&self, instance_id: &str) -> Result<()> {
        // TODO: Implement resuming for paused VMs/containers.
        println!("Resuming environment: {}", instance_id);
        todo!("Implement environment resuming.");
        // Ok(())
    }
    
    // TODO: Add other lifecycle methods like stop, start, restart as needed.

    pub fn list_vms(&self) -> Result<Vec<EnvironmentStatus>> {
        #[cfg(feature = "libvirt_integration")]
        {
            if let Some(conn) = &self.libvirt_conn {
                // Attempt to fetch live data
                let mut vms = Vec::new();
                let mut domain_names = Vec::new();
                if let Ok(active_domain_ids) = conn.list_domains() {
                    for id in active_domain_ids {
                        if let Ok(domain) = Domain::lookup_by_id(&conn, id) {
                            if let Ok(name) = domain.get_name() {
                                domain_names.push(name);
                            }
                        }
                    }
                }
                if let Ok(defined_domain_names) = conn.list_defined_domains() {
                     domain_names.extend(defined_domain_names);
                }
                domain_names.sort_unstable();
                domain_names.dedup();

                for name in domain_names {
                    if let Ok(domain) = Domain::lookup_by_name(&conn, &name) {
                        let state_info: DomainInfo = domain.get_info()?;
                        let hydra_state = match state_info.state as u32 {
                            sys::VIR_DOMAIN_NOSTATE => EnvironmentState::Unknown,
                            sys::VIR_DOMAIN_RUNNING => EnvironmentState::Running,
                            sys::VIR_DOMAIN_BLOCKED => EnvironmentState::Suspended,
                            sys::VIR_DOMAIN_PAUSED => EnvironmentState::Suspended,
                            sys::VIR_DOMAIN_SHUTDOWN => EnvironmentState::Terminated,
                            sys::VIR_DOMAIN_SHUTOFF => EnvironmentState::Stopped,
                            sys::VIR_DOMAIN_CRASHED => EnvironmentState::Error("Crashed".to_string()),
                            sys::VIR_DOMAIN_PMSUSPENDED => EnvironmentState::Suspended,
                            other_state => {
                                eprintln!("Unknown libvirt domain state encountered: {}", other_state);
                                EnvironmentState::Unknown
                            }
                        };
                        let status = EnvironmentStatus {
                            instance_id: domain.get_uuid_string().unwrap_or_else(|_| "N/A-UUID".to_string()),
                            name: name.clone(),
                            env_type: EnvironmentType::Vm,
                            state: hydra_state,
                            memory_max_kb: Some(state_info.max_mem as u64),
                            memory_used_kb: Some(state_info.memory as u64),
                            cpu_cores_used: Some(state_info.nr_virt_cpu as u32),
                            ..Default::default()
                        };
                        vms.push(status);
                    }
                }
                println!("Successfully fetched {} VMs from libvirt.", vms.len());
                return Ok(vms); // Return live data
            } else {
                // Libvirt connection failed or was None initially
                eprintln!("Libvirt connection not available for fetching VMs.");
                #[cfg(feature = "dummy_env_data")]
                {
                    println!("Falling back to dummy VM data because 'dummy_env_data' feature is enabled.");
                    return self.list_vms_placeholder();
                }
                #[cfg(not(feature = "dummy_env_data"))]
                {
                    println!("Returning empty VM list because 'dummy_env_data' feature is not enabled.");
                    return Ok(Vec::new()); // Return empty list if dummy data is not enabled
                }
            }
        }
        
        #[cfg(not(feature = "libvirt_integration"))]
        {
            println!("Libvirt integration feature not enabled.");
            #[cfg(feature = "dummy_env_data")]
            {
                println!("Falling back to dummy VM data because 'dummy_env_data' feature is enabled.");
                return self.list_vms_placeholder();
            }
            #[cfg(not(feature = "dummy_env_data"))]
            {
                println!("Returning empty VM list because 'dummy_env_data' feature is not enabled and libvirt is off.");
                return Ok(Vec::new());
            }
        }
    }
    
    fn list_vms_placeholder(&self) -> Result<Vec<EnvironmentStatus>> {
        Ok(vec![
            EnvironmentStatus {
                instance_id: "vm-uuid-placeholder-001".to_string(),
                name: "dev-vm-arch-placeholder".to_string(),
                env_type: EnvironmentType::Vm,
                state: EnvironmentState::Running,
                ip_address: Some("192.168.122.101".to_string()),
                memory_max_kb: Some(4 * 1024 * 1024), // 4GB
                memory_used_kb: Some(1 * 1024 * 1024), // 1GB
                cpu_cores_used: Some(2),
                ..Default::default()
            },
            EnvironmentStatus {
                instance_id: "vm-uuid-placeholder-002".to_string(),
                name: "test-vm-ubuntu-placeholder".to_string(),
                env_type: EnvironmentType::Vm,
                state: EnvironmentState::Stopped,
                memory_max_kb: Some(2 * 1024 * 1024), // 2GB
                cpu_cores_used: Some(1),
                ..Default::default()
            },
        ])
    }
}

// TODO: Add tests for EnvironmentManager:
// - Mocking libvirt/containerd interactions or using test backends if available.
// - Testing lifecycle transitions (create -> running -> destroy).
// - Resource allocation checks (ensure configs are passed correctly).
// - Error handling for provider failures. 