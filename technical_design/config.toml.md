# Hydravisor – Configuration File Spec

**Version:** 0.1.0
**File:** `./technical_design/config.toml.md`

---

## 🎯 Purpose

This document defines the format and behavior of Hydravisor’s primary configuration file, `config.toml`. This file governs runtime behavior, UI preferences, default providers, and system-wide defaults.

Location: `$XDG_CONFIG_HOME/hydravisor/config.toml`

---

## 🔧 Configuration Fields

### `[interface]`

```toml
[interface]
mode = "session"      # Options: "session" or "modal"
modal_key = "9"        # Key used after tmux-prefix to trigger modal commands
refresh_interval_ms = 500   # How often UI refreshes (in ms)
```

### `[defaults]`

```toml
[defaults]
default_vm_image = "ubuntu-22.04"
default_container_image = "ghcr.io/hydravisor/agent:latest"
default_model = "ollama:llama3"
default_cpu = 2
default_ram = "4GB"
```

### `[providers.ollama]`

```toml
[providers.ollama]
enabled = true
path = "/usr/local/bin/ollama"
models = ["llama3", "mistral", "codellama"]
```

### `[providers.bedrock]`

```toml
[providers.bedrock]
enabled = true
region = "us-west-2"
profile = "default"
```

### `[logging]`

```toml
[logging]
level = "info"           # Options: "debug", "info", "warn", "error"
log_dir = "~/.hydravisor/logs"
rotate_daily = true
retain_days = 14
```

### `[tmux]`

```toml
[tmux]
session_prefix = "hydravisor-"
record_all_sessions = true
record_format = "ansi"     # Options: "ansi", "jsonl"
autosave_on_exit = true
```

### `[mcp]`

```toml
[mcp]
socket_path = "/tmp/hydravisor.sock"
timeout_ms = 3000
heartbeat_interval = 15
```

---

## 🛡 Validation Rules

* `mode` must be one of: `"session"`, `"modal"`
* `modal_key` should be a single digit or letter (tmux-compatible)
* RAM must be parseable as a quantity (e.g., `4GB`, `512MB`)
* `ollama.path` must be executable if `enabled = true`
* `bedrock.profile` must match a valid AWS profile in `~/.aws/config`
* Log paths must be writable; fail fast if not

---

## ✅ Example Configuration

```toml
[interface]
mode = "modal"
modal_key = "9"

[defaults]
default_vm_image = "ubuntu-22.04"
default_container_image = "ghcr.io/hydravisor/agent:latest"
default_model = "ollama:llama3"
default_cpu = 4
default_ram = "8GB"

[providers.ollama]
enabled = true
path = "/usr/bin/ollama"
models = ["llama3", "mistral"]

[providers.bedrock]
enabled = false
region = "us-east-1"
profile = "default"

[logging]
level = "debug"
log_dir = "~/.hydravisor/logs"
rotate_daily = true
retain_days = 7

[tmux]
session_prefix = "hydravisor-"
record_all_sessions = true
record_format = "jsonl"
autosave_on_exit = true

[mcp]
socket_path = "/tmp/hydravisor.sock"
timeout_ms = 2000
heartbeat_interval = 10
```

---

*Document authored by Kelsea & Alethe – 2025*
