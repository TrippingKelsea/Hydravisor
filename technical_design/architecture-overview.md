# Hydravisor Architecture

## Overview

Hydravisor is designed as a modular, extensible system for managing AI agent sandboxes. The architecture prioritizes security isolation, comprehensive auditing, and seamless agent integration while maintaining flexibility for diverse use cases.

## Core Principles

1. **Security by Design**: All components operate under least-privilege with configurable policy enforcement
2. **Agent-Native**: Every interface is designed for programmatic agent access via MCP protocol
3. **Comprehensive Auditing**: All actions are logged, tracked, and exportable for compliance
4. **Local-First**: Prioritize local execution for privacy and performance
5. **Extensible**: Plugin architecture for custom policies, audit handlers, and environment types

## System Architecture

```mermaid
graph TD
    %% Top Level
    subgraph HydravisorCore["Hydravisor Core"]

        %% Interface Layer
        TUI["TUI Interface"]
        MCP["MCP Server (Agents)"]
        CLI["CLI Tools (Human)"]

        %% Control Layer
        subgraph ControlLayer["Control Layer"]
            SessionMgr["Session Manager"]
            PolicyEngine["Policy Engine"]
            AuditEngine["Audit Engine"]
            EnvMgr["Environment Manager"]
            KeyMgr["Key Manager"]
            TemplateMgr["Template Manager"]
        end

        %% Infrastructure Layer
        subgraph InfraLayer["Infrastructure Layer"]
            Libvirt["libvirt<br/>(KVM/VMs)"]
            Containerd["containerd<br/>(Containers)"]
            Tmux["tmux<br/>(Sessions)"]
            Ollama["Ollama<br/>(Local AI)"]
            Bedrock["Bedrock<br/>(Remote AI)"]
            Filesystem["Filesystem Monitor"]
        end

        %% Wiring Interface to Control
        TUI --> SessionMgr
        MCP --> PolicyEngine
        CLI --> AuditEngine

        %% Shared Control Access
        SessionMgr --> EnvMgr
        PolicyEngine --> KeyMgr
        AuditEngine --> TemplateMgr

        %% Control to Infrastructure
        EnvMgr --> Libvirt
        EnvMgr --> Containerd
        SessionMgr --> Tmux
        KeyMgr --> Ollama
        TemplateMgr --> Bedrock
        AuditEngine --> Filesystem
    end
```

## Component Details

### Control Layer

#### Session Manager
**Purpose**: Manages agent workspaces and environment lifecycles

**Responsibilities**:
- Create/destroy environments based on templates
- Manage tmux sessions per agent workspace
- Track active environments and resource usage
- Handle environment snapshotting and rollback
- Coordinate with audit engine for session logging

**Key APIs**:
```rust
trait SessionManager {
    async fn create_session(&self, template: &Template, policy: &Policy) -> Result<SessionId>;
    async fn destroy_session(&self, session_id: SessionId) -> Result<()>;
    async fn snapshot_session(&self, session_id: SessionId) -> Result<SnapshotId>;
    async fn attach_agent(&self, session_id: SessionId, agent_id: AgentId) -> Result<SSHKey>;
    async fn get_session_status(&self, session_id: SessionId) -> Result<SessionStatus>;
}
```

#### Policy Engine
**Purpose**: Enforces security policies and access controls

**Responsibilities**:
- Evaluate policy rules against agent requests
- Manage network isolation and resource limits
- Control privilege escalation and capability assignment
- Handle policy inheritance and composition
- Integrate with external policy sources (OPA, etc.)

**Policy Types**:
- **Resource Policies**: CPU, memory, storage, network bandwidth limits
- **Access Policies**: File system access, network connectivity, sudo privileges
- **Behavioral Policies**: Process monitoring, syscall filtering, time-based restrictions
- **Audit Policies**: Logging levels, retention, export requirements

**Example Policy**:
```yaml
policy:
  name: "restricted-coding"
  resource_limits:
    max_cpu_cores: 2
    max_memory_mb: 4096
    max_storage_gb: 10
    max_network_mbps: 10
  access_controls:
    allow_internet: false
    allow_sudo: false
    writable_paths: ["/workspace", "/tmp"]
    blocked_commands: ["curl", "wget", "nc"]
  audit:
    log_level: "detailed"
    monitor_files: true
    monitor_network: true
    retention_days: 30
```

#### Audit Engine
**Purpose**: Comprehensive logging and monitoring of all agent activities

**Responsibilities**:
- Track file system changes with inotify/fanotify
- Log command executions and outputs
- Monitor network connections and data flows
- Record tmux session interactions
- Generate compliance reports and alerts
- Export audit trails in multiple formats

**Audit Events**:
```rust
#[derive(Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub session_id: SessionId,
    pub agent_id: Option<AgentId>,
    pub event_type: AuditEventType,
    pub details: serde_json::Value,
    pub risk_level: RiskLevel,
}

pub enum AuditEventType {
    FileCreate { path: PathBuf, size: u64 },
    FileModify { path: PathBuf, changes: FileDiff },
    FileDelete { path: PathBuf },
    CommandExecute { command: String, exit_code: i32, output: String },
    NetworkConnect { destination: SocketAddr, protocol: Protocol },
    PrivilegeEscalation { from_uid: u32, to_uid: u32 },
    PolicyViolation { rule: String, action: PolicyAction },
}
```

#### Environment Manager
**Purpose**: Orchestrates VM and container lifecycle

**Responsibilities**:
- Interface with libvirt for VM management
- Interface with containerd for container management
- Handle environment templating and customization
- Manage networking and storage allocation
- Coordinate with session manager for workspace setup

#### Key Manager
**Purpose**: Secure distribution and rotation of SSH keys for agent access

**Responsibilities**:
- Generate ephemeral SSH key pairs per session
- Distribute public keys to environments
- Rotate keys on configurable schedules
- Revoke access on session termination
- Audit key usage and access patterns

#### Template Manager
**Purpose**: Manages environment templates and configurations

**Responsibilities**:
- Load and validate environment templates
- Handle template inheritance and composition
- Manage template versioning and updates
- Provide template discovery and metadata

### Interface Layer

#### TUI Interface
**Purpose**: Human-friendly terminal interface for management and monitoring

**Features**:
- Real-time environment status dashboard
- Interactive audit log viewer with filtering
- Policy management and testing interface
- Session attachment and detachment
- Resource usage monitoring and alerts

**Layout**:
```
┌─────────────────┬──────────────────┬─────────────────┐
│   Environments  │  Active Agents   │  System Status  │
│                 │                  │                 │
│ ● coding-env-1  │ 🤖 coding-bot    │ CPU: 45%        │
│ ● research-vm   │ 🔬 research-ai   │ Memory: 2.1GB   │
│ ○ test-container│ 🛡️ safety-check  │ Disk: 15GB      │
├─────────────────┼──────────────────┼─────────────────┤
│           Audit Log (Live)         │   Quick Actions │
│                                    │                 │
│ [15:32:01] coding-bot: mkdir /app  │ [ Launch Env ]  │
│ [15:32:05] research-ai: curl ...   │ [ Attach Agent] │
│ [15:32:12] ALERT: Policy violation │ [ View Audit ]  │
└────────────────────────────────────┴─────────────────┘
```

#### MCP Server
**Purpose**: Agent-native API for programmatic environment access

**MCP Protocol Integration**:
```typescript
// Agent request example
{
  "method": "environment/create",
  "params": {
    "template": "python-dev",
    "name": "coding-session-1",
    "policy": "restricted",
    "description": "Working on web scraper project"
  }
}

// Hydravisor response
{
  "result": {
    "session_id": "sess_abc123",
    "ssh_endpoint": "127.0.0.1:2222",
    "ssh_key": "-----BEGIN PRIVATE KEY-----...",
    "expires_at": "2024-01-01T16:30:00Z",
    "workspace_path": "/workspace"
  }
}
```

**Supported MCP Operations**:
- `environment/create` - Launch new sandboxed environment
- `environment/destroy` - Clean up environment
- `environment/attach` - Get SSH access to existing environment
- `environment/snapshot` - Create environment backup
- `environment/list` - Query available environments
- `audit/query` - Search audit logs
- `policy/validate` - Test policy against intended actions

### Infrastructure Layer

#### libvirt Integration
**Purpose**: KVM virtual machine management

**Capabilities**:
- VM creation from templates and cloud images
- Dynamic resource allocation and limits
- Network isolation with custom bridge configurations
- Snapshot and cloning support
- Live migration for load balancing

#### containerd Integration
**Purpose**: Container runtime management

**Capabilities**:
- OCI-compliant container execution
- Custom network namespaces per container
- Resource cgroup enforcement
- Image management and registry integration
- Runtime security with gVisor/Kata containers

#### tmux Integration
**Purpose**: Session persistence and logging

**Capabilities**:
- Persistent agent sessions across disconnections
- Session recording and playback
- Multi-pane environments for complex workflows
- Custom tmux configurations per environment type
- Integration with audit engine for command logging

## Data Flow

### Environment Creation Flow
```mermaid
flowchart TD
    A["Agent Request"]
    B["MCP Server"]
    C["Policy Engine"]
    D["Session Manager"]
    E["Template Manager"]
    F["Environment Manager"]
    G["libvirt / containerd"]
    H["Infrastructure"]
    I["SSH Key Generation"]
    J["Key Manager"]
    K["Session Setup"]
    L["tmux"]
    M["Audit Engine"]

    A --> B
    B --> C
    C --> D
    C --> E
    E --> F
    F --> G
    G --> H
    H --> I
    I --> J
    J --> K
    K --> L
    K --> M
```

### Audit Flow
```mermaid
flowchart TD
    A["Agent Action"]
    B["Environment"]
    C["Audit Monitors"]
    D["Audit Engine"]
    E["Event Processing"]
    F["Policy Evaluation"]
    G["Storage + Alerts"]
    H["Dashboard / Reports"]

    A --> B
    B --> C
    C --> D
    D --> E
    E --> F
    F --> G
    G --> H
```

## Security Model

### Isolation Boundaries
1. **Process Isolation**: Each environment runs in separate VM/container
2. **Network Isolation**: Custom networks with firewall rules
3. **Filesystem Isolation**: Separate filesystems with mount restrictions
4. **User Isolation**: Separate user namespaces and privilege controls

### Trust Boundaries
```mermaid
flowchart LR
    subgraph Trusted
        Core["Hydravisor<br/>Core"]
        Host["Host System"]
        Core <---> Host
    end

    Core -->|"MCP Protocol<br/>(Authenticated)"| Agents

    subgraph Untrusted
        Agents["AI Agents"]
        Envs["Environments<br/>(Sandboxed)"]
        Agents <--->|"SSH / Limited"| Envs
    end
```

### Policy Enforcement Points
- **MCP Server**: Authentication and request validation
- **Session Manager**: Resource allocation and limits
- **Environment Manager**: Network and filesystem restrictions
- **Infrastructure Layer**: Hardware-level isolation (VM/containers)

## Extensibility

### Plugin Architecture
```rust
pub trait PolicyPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(&self, request: &PolicyRequest) -> PolicyResult;
    fn on_violation(&self, event: &AuditEvent) -> Option<PolicyAction>;
}

pub trait AuditPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn handle_event(&self, event: &AuditEvent) -> Result<()>;
    fn export_format(&self) -> Vec<String>;
}

pub trait TemplatePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn template_types(&self) -> Vec<String>;
    fn create_environment(&self, template: &Template) -> Result<EnvironmentSpec>;
}
```

### Configuration Extensibility
- **Custom Policies**: YAML/JSON policy definitions with inheritance
- **Template System**: Parameterized environment templates
- **Audit Handlers**: Pluggable export formats and alerting systems
- **Integration Points**: Webhooks, message queues, external monitoring

## Performance Considerations

### Resource Management
- **Environment Pooling**: Pre-warmed environments for faster agent startup
- **Resource Overcommit**: Intelligent memory and CPU oversubscription
- **Garbage Collection**: Automatic cleanup of idle environments
- **Audit Optimization**: Efficient event batching and storage

### Scalability
- **Horizontal Scaling**: Multi-node deployment with environment distribution
- **Load Balancing**: Intelligent agent-to-node assignment
- **Storage Optimization**: Shared base images and copy-on-write filesystems
- **Network Optimization**: Overlay networks for multi-node communication

This architecture provides a solid foundation for the AI agent sandbox use case while maintaining the flexibility to support broader virtualization management scenarios.