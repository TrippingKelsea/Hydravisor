# Hydravisor – Interface Architecture Specification

**Version:** 0.1.1  
**File:** `./technical_design/interface.design.md`

---

## 🎯 Purpose
This document details the low-level integration plan between Hydravisor and key system components: TMUX, SSH, KVM (via libvirt), containerd, Ollama, and Amazon Bedrock. It includes detailed entity diagrams, API layers, command sequences, configuration expectations, and rationale for interface decisions. While no code is written here, this spec is optimized for guiding automatic or semi-automatic code generation.

---

## 📦 External Interfaces

### 1. TMUX
**Method:** External CLI via `tmux` binary + Rust wrapper crate (`tmux_interface`)  
**Use Cases:** Session creation, pane attach/detach, capture logs, create named buffers

#### Core Commands (Invoked via `tmux_interface`)
- `tmux new-session -s hydra-<id> -d`
- `tmux split-window -t hydra-<id>`
- `tmux send-keys -t hydra-<id>:<pane> "<command>" C-m`
- `tmux capture-pane -t <pane> -p`
- `tmux save-buffer -b <buf> ~/logs/hydra-<id>.log`

#### Output
- Raw pane stdout/stderr
- Scrollback buffer dump (for replay/archive)

#### Configuration Notes
- Should detect `$TMUX` and gracefully fallback to launching session
- Requires consistent session naming for identification

---

### 2. SSH
**Method:** TCP client using `openssh-rs` or subprocess-spawned `ssh` command

#### Command Pattern (subprocess or lib-based)
- `ssh -i ~/.ssh/hydravisor_ed25519 user@host -p 2222`
- `scp ./bootstrap.sh user@host:/tmp`

#### Features Needed
- Identity file discovery (configurable)
- Optional host key verification disable for test/dev
- Port forwarding support (for MCP agents)
- Stream reader for STDOUT/STDERR logging

#### Configuration Notes
- `~/.config/hydravisor/ssh.toml` to store **per-host SSH profile overrides**
- Defaults fall back to native `~/.ssh/config`, OpenSSH agent, and provider-level SSH settings
- Use of ControlMaster encouraged for speed-up

---

### 3. KVM
**Method:** `libvirt` via `libvirt-rs` crate or FFI bindings to `libvirtd`

#### Command Equivalents (libvirt API)
- `virConnectOpen("qemu:///system")`
- `virDomainDefineXML(domain_xml)`
- `virDomainCreate(domain_ptr)`
- `virDomainShutdown(domain_ptr)`
- `virDomainSnapshotCreateXML(snapshot_xml)`

#### Domain XML Snippet Example
```xml
<domain type='kvm'>
  <name>vm-hydra-01</name>
  <memory unit='MiB'>4096</memory>
  <vcpu>2</vcpu>
  <os><type arch='x86_64'>hvm</type></os>
  <devices>
    <emulator>/usr/bin/qemu-system-x86_64</emulator>
    <disk type='file' device='disk'>...</disk>
    <interface type='network'>...</interface>
  </devices>
</domain>
```

#### Event Hooks
- Connect to `libvirt` event stream for domain state tracking
- Use `virDomainGetInfo()` to monitor CPU/RAM usage

---

### 4. containerd
**Method:** gRPC using `containerd-client` Rust crate

#### gRPC Services
- `ContainerService.Create` — pull image & prepare container
- `TaskService.Start` — begin execution
- `TaskService.Delete` — clean removal
- `MetricsService.Get` — resource monitoring

#### Command-Line Equivalents
- `ctr image pull docker.io/library/debian:latest`
- `ctr run -d --rm docker.io/library/debian:latest mycontainer /bin/bash`

#### Socket
- Default: `/run/containerd/containerd.sock`
- May need sudo or system-level permissions

#### Namespacing
- Use `hydravisor-<id>` as container name prefix
- Capture stdout logs using `TaskService` I/O piping

---

### 5. Ollama
**Method:** HTTP over localhost or Unix domain socket (`ollama-rs` or direct HTTP)

#### API Endpoints
- `GET /api/tags` — list installed models
- `POST /api/generate` — inference request with prompt
- `POST /api/create` — load new model into memory
- `DELETE /api/delete` — unload model

#### JSON Request Example
```json
{
  "model": "llama3",
  "prompt": "What is the capital of France?",
  "stream": true
}
```

#### Notes
- Socket path configurable: `/var/run/ollama.sock`
- Each response stream tokenized for real-time UI

---

### 6. Amazon Bedrock
**Method:** AWS SDK (`aws-sdk-bedrock`) + IAM credentials

#### API Flow
- `InvokeModelWithResponseStream`
  - Request body: JSON string
  - Headers: SigV4 signed with session token

#### Endpoint Example
```json
{
  "modelId": "anthropic.claude-v2",
  "contentType": "application/json",
  "body": "{\"prompt\":\"Hello!\"}"
}
```

#### Auth Configuration
- `~/.aws/config` and `~/.aws/credentials`
- Profile auto-discovery or via env var `AWS_PROFILE`

---

## ✅ Interface Behavior Checklist

| Interface     | Auth Model                | Reconnect? | Logging               | Dependencies         |
|---------------|---------------------------|------------|------------------------|----------------------|
| TMUX          | Local binary              | Yes        | Pane & buffer logs     | `tmux_interface`     |
| SSH           | Keypair / password        | Optional   | Session & command logs | `openssh-rs`         |
| KVM/libvirt   | UNIX socket perms         | Yes        | Domain + VM stats      | `libvirt-rs`         |
| containerd    | gRPC socket auth          | Yes        | Task logs, metrics     | `containerd-client`  |
| Ollama        | Local socket              | Yes        | Token stream           | HTTP or `ollama-rs`  |
| Bedrock       | IAM (SigV4)               | Yes        | Req/resp stream        | `aws-sdk-bedrock`    |

