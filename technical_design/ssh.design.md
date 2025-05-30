# SSH Configuration Design (`ssh.toml`)

## 📜 Purpose

The `ssh.toml` configuration file enables per-host SSH profile overrides for Hydravisor-managed virtual machines and containers. This allows the system to securely and flexibly connect to guests without relying solely on global SSH configuration files.

## 📂 File Location

Hydravisor expects this configuration at:

```text
$XDG_CONFIG_HOME/hydravisor/ssh.toml
```

## 🔐 Design Goals

- **Secure Defaults:** Strong identity separation using unique key pairs per VM.
- **Environment Awareness:** Defaults to environment variables like `$XDG_CONFIG_HOME` and `$HOME`.
- **Override Control:** Per-host control for port, timeout, and forwarding behavior.
- **Validation:** Schema-backed to ensure correctness and avoid runtime surprises.

## 🔑 Key Management Strategy

- Each VM is provisioned with a unique SSH keypair.
  - `foo-vm-client` is the user's identity key for connecting to VM `foo-vm`.
  - `foo-vm-host` is the host key preloaded into the VM.
- Keys are stored by default in:
  
  ```text
  $XDG_CONFIG_HOME/hydravisor/keys/
  ```

- In future releases, this directory may be replaced by an encrypted internal vault filesystem with CLI/API retrieval interfaces.

## 🧾 SSH Profile Format (`ssh.toml`)

```toml
[hosts.foo-vm]
address = "192.168.122.12"
port = 2222
username = "hydra"
identity_file = "$HOME/.hydravisor/keys/foo-client"
host_key_check = true
forward_agent = false
connect_timeout = 10
session_timeout = 120
```

## ⚠️ Missing Config Behavior

- If a host entry is missing:
  - Hydravisor will fall back to the default system SSH config (`~/.ssh/config`).
  - If that fails, the connection will not proceed.
  - Users will receive a descriptive error and be prompted to run a diagnostic or auto-generate the entry.

## 📑 Schema Validation

A JSON Schema is provided to validate `ssh.toml` after conversion to JSON:

File: `./schemas/ssh.schema.json`

### Required Fields per Host:
- `address`
- `username`
- `identity_file`

### Optional Fields:
- `port` (default: 22)
- `host_key_check` (default: true)
- `forward_agent` (default: false)
- `connect_timeout`
- `session_timeout`

```json
"hosts": {
  "foo-vm": {
    "address": "192.168.122.12",
    "username": "hydra",
    "identity_file": "$HOME/.hydravisor/keys/foo-client"
  }
}
```

## 🔮 Future Work

- Encrypted vault storage for keys
- UI management panel for SSH profiles
- Configurable key rotation schedules
- Boot-time VM SSH key injection with Arch-based builder

## 📎 Related Files

- [`ssh.toml`](ssh.toml)
- [`ssh.schema.json`](ssh.schema.json)

