// src/libvirt_manager.rs
// Manages VMs using libvirt/KVM

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

// Configuration for creating a new VM
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VmConfig {
    pub instance_id: String, // Unique ID for this VM instance
    pub base_image: String, // e.g., "ubuntu-22.04" or a path to a source qcow2
    pub boot_iso: Option<String>,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_gb: Option<u64>,
    pub network_policy: String,    // Reference to a network policy name/ID
    pub security_policy: String,   // Reference to a security policy name/ID
    pub custom_script: Option<String>, // Optional bootstrap script content or path
    pub template_name: Option<String>, // Name of the template used, if any
    pub labels: Option<HashMap<String, String>>, // For tagging/metadata
}

// Represents the runtime state of a VM
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum VmState {
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

// Detailed status of a running or managed VM
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VmStatus {
    pub instance_id: String, // For VMs, this could be the libvirt UUID or name
    pub name: String, // For VMs, the libvirt domain name
    pub state: VmState,
    pub ip_address: Option<String>,
    pub ssh_port: Option<u16>,
    pub created_at: String, // ISO 8601 timestamp - for VMs, libvirt might not have this directly
    pub updated_at: String, // ISO 8601 timestamp
    pub base_image: Option<String>, // May not always be known for externally created VMs
    pub cpu_cores_used: Option<u32>, // Current vCPUs (from libvirt DomainInfo)
    pub memory_max_kb: Option<u64>,   // Max memory allocated (from libvirt DomainInfo)
    pub memory_used_kb: Option<u64>, // Current memory usage (from libvirt DomainInfo)
    pub error_details: Option<String>,
}

pub struct LibvirtManager {
    #[cfg(feature = "libvirt_integration")]
    libvirt_conn: Option<Connect>,
    #[cfg(feature = "libvirt_integration")]
    pub libvirt_connected: bool,
}

impl LibvirtManager {
    pub fn new(_app_config: &Config) -> Result<Self> {
        #[cfg(feature = "libvirt_integration")]
        let (libvirt_conn, libvirt_connected) = match Connect::open(Some("qemu:///system")) {
            Ok(conn) => (Some(conn), true),
            Err(_e) => (None, false),
        };
        #[cfg(not(feature = "libvirt_integration"))]
        let (_libvirt_conn, libvirt_connected): (Option<()>, bool) = (None, false);

        Ok(LibvirtManager {
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

    pub fn destroy_vm(&self, instance_id: &str) -> Result<()> {
        #[cfg(feature = "libvirt_integration")]
        {
            if let Some(conn) = &self.libvirt_conn {
                if let Ok(domain) = Domain::lookup_by_name(conn, instance_id) {
                    if domain.is_active()? {
                        domain.destroy()?;
                    }
                    domain.undefine()?;
                    // TODO: Delete the associated disk image from /var/lib/libvirt/images/
                    return Ok(());
                } else {
                    return Err(anyhow!("VM with instance_id '{}' not found.", instance_id));
                }
            }
        }
        Err(anyhow!(
            "Libvirt not available. Cannot destroy VM."
        ))
    }

    pub fn resume_vm(&self, _instance_id: &str) -> Result<()> {
        todo!("Implement VM resuming.");
    }
    
    // TODO: Add other lifecycle methods like stop, start, restart as needed.

    pub fn list_vms(&self) -> Result<Vec<VmStatus>> {
        #[cfg(feature = "libvirt_integration")]
        {
            if let Some(conn) = &self.libvirt_conn {
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
                        let hydra_state = self.map_libvirt_state_to_vm_state(state_info.state);
                        let status = VmStatus {
                            instance_id: domain.get_uuid_string().unwrap_or_else(|_| "N/A-UUID".to_string()),
                            name: name.clone(),
                            state: hydra_state,
                            memory_max_kb: Some(state_info.max_mem as u64),
                            memory_used_kb: Some(state_info.memory as u64),
                            cpu_cores_used: Some(state_info.nr_virt_cpu as u32),
                            ..Default::default()
                        };
                        vms.push(status);
                    }
                }
                return Ok(vms);
            } else {
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
    
    fn list_vms_placeholder(&self) -> Result<Vec<VmStatus>> {
        Ok(vec![
            VmStatus {
                instance_id: "vm-uuid-placeholder-001".to_string(),
                name: "dev-vm-arch-placeholder".to_string(),
                state: VmState::Running,
                ip_address: Some("192.168.122.101".to_string()),
                memory_max_kb: Some(4 * 1024 * 1024), // 4GB
                memory_used_kb: Some(1 * 1024 * 1024), // 1GB
                cpu_cores_used: Some(2),
                ..Default::default()
            },
            VmStatus {
                instance_id: "vm-uuid-placeholder-002".to_string(),
                name: "test-vm-ubuntu-placeholder".to_string(),
                state: VmState::Stopped,
                memory_max_kb: Some(2 * 1024 * 1024), // 2GB
                cpu_cores_used: Some(1),
                ..Default::default()
            },
        ])
    }
    
    #[cfg(feature = "libvirt_integration")]
    pub fn create_vm(&self, vm_config: &VmConfig) -> Result<VmStatus> {
        if let Some(conn) = &self.libvirt_conn {
            let vm_name = vm_config.instance_id.clone();
            let disk_path = format!("/var/lib/libvirt/images/{}.qcow2", vm_name);

            let xml = self.create_vm_xml(
                &vm_name,
                vm_config.cpu_cores,
                vm_config.memory_mb,
                &disk_path,
                vm_config.boot_iso.as_deref(),
            );
            
            let domain = Domain::create_xml(conn, &xml, 0)?;
            
            Ok(VmStatus {
                instance_id: domain.get_uuid_string()?,
                name: domain.get_name()?,
                state: VmState::Provisioning,
                ..Default::default()
            })
        } else {
            Err(anyhow!("Libvirt connection not available"))
        }
    }

    #[cfg(not(feature = "libvirt_integration"))]
    pub fn create_vm(&self, _vm_config: &VmConfig) -> Result<VmStatus> {
        Err(anyhow!("Cannot create VM: libvirt_integration feature is disabled."))
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
        let memory_kb = memory_mb * 1024;
        let mut iso_disk = "".to_string();
        if let Some(iso_path) = boot_iso {
            iso_disk = format!(
                r#"<disk type='file' device='cdrom'>
                      <driver name='qemu' type='raw'/>
                      <target dev='hda' bus='sata'/>
                      <source file='{}'/>
                      <readonly/>
                   </disk>"#,
                iso_path
            );
        }

        format!(
            r#"<domain type='kvm'>
                  <name>{}</name>
                  <memory unit='KiB'>{}</memory>
                  <vcpu>{}</vcpu>
                  <os>
                    <type arch='x86_64' machine='q35'>hvm</type>
                    <boot dev='hd'/>
                    {}
                  </os>
                  <devices>
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
                    <graphics type='vnc' port='-1' autoport='yes' listen='127.0.0.1'>
                      <listen type='address' address='127.0.0.1'/>
                    </graphics>
                    <video>
                        <model type="virtio" heads="1" primary="yes"/>
                    </video>
                  </devices>
                </domain>"#,
            name, memory_kb, vcpu, if boot_iso.is_some() { "<boot dev='cdrom'/>" } else { "" }, disk_path, iso_disk
        )
    }

    #[cfg(feature = "libvirt_integration")]
    fn map_libvirt_state_to_vm_state(&self, state_code: u32) -> VmState {
        match state_code {
            sys::VIR_DOMAIN_NOSTATE => VmState::Unknown,
            sys::VIR_DOMAIN_RUNNING => VmState::Running,
            sys::VIR_DOMAIN_BLOCKED => VmState::Suspended,
            sys::VIR_DOMAIN_PAUSED => VmState::Suspended,
            sys::VIR_DOMAIN_SHUTDOWN => VmState::Terminated,
            sys::VIR_DOMAIN_SHUTOFF => VmState::Stopped,
            sys::VIR_DOMAIN_CRASHED => VmState::Error("Crashed".to_string()),
            _ => {
                VmState::Unknown
            }
        }
    }
} 