# Hydravisor – CLI Design Specification

**Version:** 0.1.2
**File:** `./technical_design/cli.design.md`

---

## 🎯 Purpose

This document specifies the command-line interface (CLI) design for `hydravisor`. It defines command groupings, arguments, output formats, and the interactive philosophy behind terminal usage. All CLI commands should be scriptable, secure, and log-friendly.

---

## 🔧 CLI Philosophy

* **Declarative**: All CLI actions must reflect system state changes or queries.
* **Non-interactive by default**: All commands must succeed or fail without prompting.
* **Composable**: Designed to work in shell scripts, pipelines, and CI.
* **Explicit**: No command should have implicit defaults that can break security posture.

---

## 📜 Root Command

```bash
hydravisor [SUBCOMMAND] [OPTIONS]
```

### Global Flags

| Flag                | Description                           |
| ------------------- | ------------------------------------- |
| `--config <file>`   | Override config location              |
| `--log-level <lvl>` | Set log level: `trace`, `debug`, etc. |
| `--headless`        | Suppress UI auto-launch               |
| `--version`         | Print version and exit                |

---

## 📁 Command Groups

### `policy`

```bash
hydravisor policy validate
hydravisor policy check --agent agent-a --vm vm-foo
```

| Command    | Description                                 |
| ---------- | ------------------------------------------- |
| `validate` | Check `policy.toml` against schema          |
| `check`    | Simulate authorization between agent and VM |

**Example Output:**

```bash
$ hydravisor policy check --agent agent-a --vm vm-foo
✔ Allowed
```

---

### `agent`

```bash
hydravisor agent list
hydravisor agent info <agent-id>
```

| Command | Description                       |
| ------- | --------------------------------- |
| `list`  | Show all configured/active agents |
| `info`  | Show status and policy bindings   |

**Example Output:**

```bash
agent-id: agent-a
role: sandboxed
bound VMs: ["vm-foo"]
status: active
```

---

### `vm`

```bash
hydravisor vm list
hydravisor vm info <vm-id>
hydravisor vm snapshot <vm-id> --output /path/file.tar.gz
```

| Command    | Description                       |
| ---------- | --------------------------------- |
| `list`     | List known VM sessions or configs |
| `info`     | Show VM state, logs, and bindings |
| `snapshot` | Export current VM as archive      |

---

### `log`

```bash
hydravisor log list
hydravisor log view <session-id>
```

| Command | Description                           |
| ------- | ------------------------------------- |
| `list`  | Show available session logs           |
| `view`  | View logs (`.log`, `.cast`, `.jsonl`) |

**Note**: `log replay` for `.cast` files is future work.

---

### `store` (Future: Encrypted Disk Management)

```bash
hydravisor store unlock
```

| Command  | Description                         |
| -------- | ----------------------------------- |
| `unlock` | Unlock encrypted store (if enabled) |

**Environment Variables:**

* `SSH_STORE_PASSPHRASE`: Optional override to suppress prompt

---

## 📌 Future Commands (Planned)

| Command              | Purpose                          |
| -------------------- | -------------------------------- |
| `agent promote <id>` | Elevate trust (if policy allows) |
| `mcp route`          | Debug a route resolution         |
| `session replay`     | View `.cast` sessions via TUI    |

---

