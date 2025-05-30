# Hydravisor – System Interface Architecture

**Version:** 0.1.2  
**File:** `./technical_design/interface.design.md`

---

## 🎯 Purpose

This document defines the low-level system interfaces used by Hydravisor to control, monitor, and manipulate virtual machines, terminal multiplexers, external model runtimes, and runtime environments. It details the integration plan between Hydravisor and key system components: TMUX, SSH, KVM (via libvirt), containerd, Ollama, and Amazon Bedrock. It includes detailed entity diagrams, API layers, command sequences, configuration expectations, and rationale for interface decisions. While no code is written here, this spec is optimized for guiding automatic or semi-automatic code generation.

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
- All sessions prefixed with `hydravisor-` namespace

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
- `~/.config/hydravisor/ssh.toml` stores **per-host SSH profile overrides**
- If no matching configuration is found, Hydravisor will fall back to standard OpenSSH configurations: `~/.ssh/config`, SSH agent, and provider-specific configs
- If fallback is unsuccessful or invalid, a user prompt will be triggered to select or define an appropriate connection strategy
- Use of `ControlMaster` recommended for speed-up and connection reuse
- Hydravisor will generate **per-VM SSH keypairs** by default:
  - Host keypair: `foo-host` && `foo-host.pub`
  - Client keypair: `foo-client` && `foo-client.pub`
- Keypairs are stored locally within the Hydravisor-managed application directory using an **encrypted virtual filesystem volume**
- Keys are retrievable via Hydravisor CLI and API for authorized users
- This provides isolation per-VM, reduces blast radius in case of key compromise, and avoids reliance on shared global secrets

#### Future Considerations
- Investigate custom Arch Linux image with pre-trusted host keys during bootstrap

---

### 🔮 Future Work: Arch Image Customization
We plan to explore using a base Arch Linux image preconfigured with Hydravisor-specific trust anchors and bootstrap tooling:

- Inject VM host/client keypairs at image build time
- Pre-authorize the client keys in the guest's `~/.ssh/authorized_keys`
- Embed initialization hooks to register MCP agents automatically post-boot
- Maintain template image via reproducible build process

This would significantly simplify secure deployments and key distribution, while ensuring every VM instance starts in a known-good, trusted state.

---

### 3. KVM
**Method:** `libvirt` via `libvirt-rs` crate or FFI bindings to `libvirtd`

#### Command Equivalents (libvirt API)
- `virConnectOpen("qemu:///system")`
- `virDomainDefineXML(domain_xml)`
- `virDomainCreate(domain_ptr)`
- `virDomainShutdown(domain_ptr)`
- `virDomainSnapshotCreateXML(snapshot_xml)`

#### Direct QEMU Integration (default backend)
* **Launch**: Spawned via `qemu-system-*` commands
* **Networking**: Default user-mode NAT; future support for TAP bridged
* **Volume Mounts**: `-drive` flags with snapshot or persistent disk images
* **Control**: STDIO or QMP socket (planned)

```sh
qemu-system-x86_64 \
  -name vm-foo \
  -m 2048 \
  -smp 2 \
  -hda /var/lib/hydravisor/vms/vm-foo.img \
  -nographic -serial mon:stdio
```

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

#### podman (future integration)
* **Use case**: run containers as lightweight VMs or sandboxes
* Accessed via CLI tool: `podman run ...`
* No OCI runtime assumed, requires presence of `podman`

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
- Accessed via REST API at `http://localhost:11434`
- Commands sent as JSON to `/api/generate` or `/api/chat`
- Hydravisor routes MCP `model/send` commands into this endpoint
- Future: context streaming and JSONL response parsing

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
- Model access controlled by `policy.toml` bindings
- Session requests signed via SigV4 and routed through `bedrock-runtime:InvokeModel`

#### Key Concepts
* Models are treated as remote agents
* Responses streamed and logged per session

---

## 🧩 OS-Level Integration

* **Key Generation**: `ssh-keygen -t ed25519`, or Rust-native
* **Filesystem**: Uses `std::fs`, `tempfile`, and path-safe handling
* **Syscalls**: Subprocesses managed via `std::process::Command`
* **UID/GID enforcement**: Future sandboxing (non-root)
* **Entropy**: From `/dev/urandom`

---

## 🔒 Security Controls

* All subprocess input is sanitized and quoted
* Privilege escalation is not supported
* Planned integration: `seccomp`, `capsh`, and sandboxed shells
* No external API receives raw terminal data without policy check

---

## 🔁 Logging Interfaces

* **Terminal (tmux)**: Captured to `.cast` and `.jsonl`
* **stdout**: For structured tracing logs
* **stderr**: Reserved for panic/fault reporting
* **AI Model Output**: Logged as streaming JSONL sessions

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

---

## 📌 Future Work

| Area              | Detail                                     |
| ----------------- | ------------------------------------------ |
| libvirt support   | Abstracted VM definition support           |
| podman bridging   | Sandboxed container VMs                    |
| seccomp/capsh     | Runtime syscall filtering                  |
| Model fusion      | Mix ollama + Bedrock results in one stream |
| Network isolation | Integrate with `firejail`, namespaces      |

---