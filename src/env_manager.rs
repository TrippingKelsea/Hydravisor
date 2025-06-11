// src/env_manager.rs
// Manages VMs (libvirt/KVM) and containers (containerd)

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub base_image: String, // e.g., "ubuntu-22.04" or a path to a source qcow2
    pub boot_iso: Option<String>,
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
    #[cfg(feature = "libvirt_integration")]
    pub libvirt_connected: bool,
    // active_environments: Mutex<HashMap<String, EnvironmentStatus>>,
    // containerd_client: Option<ContainerdClient>, // If containerd feature enabled
}

impl EnvironmentManager {
    pub fn new(app_config: &Config) -> Result<Self> {
        #[cfg(feature = "libvirt_integration")]
        let (libvirt_conn, libvirt_connected) = match Connect::open(Some("qemu:///system")) {
            Ok(conn) => (Some(conn), true),
            Err(_e) => (None, false),
        };
        #[cfg(not(feature = "libvirt_integration"))]
        let (libvirt_conn, libvirt_connected): (Option<Connect>, bool) = (None, false); // Ensure libvirt_conn is defined even if feature is off

        Ok(EnvironmentManager {
            #[cfg(feature = "libvirt_integration")]
            libvirt_conn,
            #[cfg(feature = "libvirt_integration")]
            libvirt_connected,
        })
    }

    #[cfg(feature = "libvirt_integration")]
    pub fn is_libvirt_connected(&self) -> bool {
        self.libvirt_connected
    }

    #[cfg(not(feature = "libvirt_integration"))]
    pub fn is_libvirt_connected(&self) -> bool {
        false
    }

    pub fn create_environment(&self, env_config: &EnvironmentConfig) -> Result<EnvironmentStatus> {
        match env_config.env_type {
            EnvironmentType::Vm => self.create_vm(env_config),
            EnvironmentType::Container => {
                todo!("Implement container creation using containerd/podman")
            }
        }
    }

    pub fn destroy_environment(&self, instance_id: &str) -> Result<()> {
        #[cfg(feature = "libvirt_integration")]
        {
            if let Some(conn) = &self.libvirt_conn {
                if let Ok(domain) = Domain::lookup_by_name(conn, instance_id) {
                    // If the VM is running, destroy it (forced shutdown)
                    if domain.is_active()? {
                        domain.destroy()?;
                    }
                    // Undefine the VM (removes its configuration)
                    domain.undefine()?;

                    // TODO: Delete the associated disk image from /var/lib/libvirt/images/
                    return Ok(());
                } else {
                    return Err(anyhow!("VM with instance_id '{}' not found.", instance_id));
                }
            }
        }
        // If libvirt is not enabled or no connection, we can't do anything
        Err(anyhow!(
            "Libvirt not available. Cannot destroy environment."
        ))
    }

    pub fn list_environments(&self) -> Result<Vec<EnvironmentStatus>> {
        // For now, "environments" are just VMs. This can be expanded later.
        self.list_vms()
    }

    pub fn resume_environment(&self, instance_id: &str) -> Result<()> {
        // TODO: Implement resuming for paused VMs/containers.
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
                        let hydra_state = self.map_libvirt_state_to_hydra(state_info.state as u32);
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
                return Ok(vms); // Return live data
            } else {
                // Libvirt connection failed or was None initially
                #[cfg(feature = "dummy_env_data")]
                {
                    return self.list_vms_placeholder();
                }
                #[cfg(not(feature = "dummy_env_data"))]
                {
                    return Ok(Vec::new()); // Return empty list if dummy data is not enabled
                }
            }
        }
        
        #[cfg(not(feature = "libvirt_integration"))]
        {
            #[cfg(feature = "dummy_env_data")]
            {
                return self.list_vms_placeholder();
            }
            #[cfg(not(feature = "dummy_env_data"))]
            {
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

    #[cfg(feature = "libvirt_integration")]
    fn create_vm(&self, env_config: &EnvironmentConfig) -> Result<EnvironmentStatus> {
        if let Some(_conn) = &self.libvirt_conn {
            let vm_name = env_config.instance_id.clone();
            let disk_size_gb = env_config.disk_gb.unwrap_or(20); // Default to 20GB
            let disk_path = format!("/var/lib/libvirt/images/{}.qcow2", vm_name);

            // TODO: Implement actual disk creation logic
            // For now, we assume a disk image can be created at `disk_path`.
            // In a real implementation, you would use `qemu-img` or a library.

            let _domain_xml = self.create_vm_xml(
                &vm_name,
                env_config.cpu_cores,
                env_config.memory_mb,
                &disk_path,
                env_config.boot_iso.as_deref(),
            );

            // The 'virt' crate does not seem to support domain creation from XML.
            // This is a known issue blocking VM creation. Consider switching to the 'libvirt-rs' crate.
            todo!("Blocked: The 'virt' crate does not provide a method to define a VM from XML.");

        } else {
            Err(anyhow!(
                "Libvirt connection not available. Cannot create VM."
            ))
        }
    }

    #[cfg(feature = "libvirt_integration")]
    fn create_vm_xml(
        &self,
        name: &str,
        vcpu: u32,
        memory_mb: u64,
        disk_path: &str,
        boot_iso: Option<&str>,
    ) -> String {

        let boot_order_xml = if boot_iso.is_some() {
            "    <boot dev='cdrom'/>\n    <boot dev='hd'/>"
        } else {
            "    <boot dev='hd'/>"
        };

        let iso_disk_xml = if let Some(iso_path) = boot_iso {
            format!(r#"
    <disk type='file' device='cdrom'>
      <driver name='qemu' type='raw'/>
      <source file='{}'/>
      <target dev='sda' bus='sata'/>
      <readonly/>
    </disk>"#, iso_path)
        } else {
            String::new()
        };

        format!(
            r#"
<domain type='kvm'>
  <name>{}</name>
  <memory unit='MiB'>{}</memory>
  <vcpu placement='static'>{}</vcpu>
  <os>
    <type arch='x86_64' machine='pc-q35-8.0'>hvm</type>
{}
  </os>
  <features>
    <acpi/>
    <apic/>
    <vmport state='off'/>
  </features>
  <cpu mode='host-passthrough' check='none' migratable='on'/>
  <clock offset='utc'>
    <timer name='rtc' tickpolicy='catchup'/>
    <timer name='pit' tickpolicy='delay'/>
    <timer name='hpet' present='no'/>
  </clock>
  <devices>
    <emulator>/usr/bin/qemu-system-x86_64</emulator>
    <disk type='file' device='disk'>
      <driver name='qemu' type='qcow2'/>
      <source file='{}'/>
      <target dev='vda' bus='virtio'/>
    </disk>
{}
    <interface type='network'>
      <source network='default'/>
      <model type='virtio'/>
    </interface>
    <input type='tablet' bus='usb'/>
    <graphics type='vnc' port='-1' autoport='yes' listen='127.0.0.1'>
      <listen type='address' address='127.0.0.1'/>
    </graphics>
    <video>
      <model type='virtio' heads='1' primary='yes'/>
    </video>
  </devices>
</domain>
"#,
            name, memory_mb, vcpu, boot_order_xml, disk_path, iso_disk_xml
        )
    }
    
    // TODO: Add other lifecycle methods like stop, start, restart as needed.

    #[cfg(feature = "libvirt_integration")]
    fn map_libvirt_state_to_hydra(&self, state_code: u32) -> EnvironmentState {
        match state_code {
            sys::VIR_DOMAIN_NOSTATE => EnvironmentState::Unknown,
            sys::VIR_DOMAIN_RUNNING => EnvironmentState::Running,
            sys::VIR_DOMAIN_BLOCKED => EnvironmentState::Suspended, // Blocked on resource
            sys::VIR_DOMAIN_PAUSED => EnvironmentState::Suspended,
            sys::VIR_DOMAIN_SHUTDOWN => EnvironmentState::Terminated, // Being shut down
            sys::VIR_DOMAIN_SHUTOFF => EnvironmentState::Stopped,   // Is off
            sys::VIR_DOMAIN_CRASHED => EnvironmentState::Error("Crashed".to_string()),
            sys::VIR_DOMAIN_PMSUSPENDED => EnvironmentState::Suspended,
            other_state => {
                EnvironmentState::Unknown
            }
        }
    }
}

// TODO: Add tests for EnvironmentManager:
// - Mocking libvirt/containerd interactions or using test backends if available.
// - Testing lifecycle transitions (create -> running -> destroy).
// - Resource allocation checks (ensure configs are passed correctly).
// - Error handling for provider failures. 