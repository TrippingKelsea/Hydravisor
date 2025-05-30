# Hydravisor – Policy File Documentation

**Version:** 0.1.2  
**File:** `./technical_design/policy.toml.md`

This document defines the schema, usage, and example for `policy.toml`, which governs runtime security and access control within Hydravisor.

**Location**: `$XDG_CONFIG_HOME/hydravisor/policy.toml`  
**Schema**: See [`./technical_design/policy.schema.json`](./technical_design/policy.schema.json)

---

## 📖 Purpose

This document defines the structure, semantics, and enforcement rules for `policy.toml`, the central trust policy file used in Hydravisor. It describes field-level expectations, validation rules, and behavior during command execution.

---

## 🗂️ File Location & Format

* **Path**: `$XDG_CONFIG_HOME/hydravisor/policy.toml`
* **Format**: [TOML](https://toml.io/en/) 1.0
* **Schema**: Validated against `policy.schema.json`

---

## 🧱 Structure Overview

### Role-Based Configuration

```toml
[roles.trusted]
can_create = true
can_attach_terminal = true
audited = false

[roles.sandboxed]
can_create = false
can_attach_terminal = true
audited = true

[roles.audited]
can_create = true
can_attach_terminal = true
audited = true

[permissions.model:llama3]
can_create = false
can_attach_terminal = false
audited = true
```

### VM and Agent-Specific Configuration

```toml
[vm."vm-name"]
trusted = true
agents = ["agent-a", "agent-b"]

[agent."agent-a"]
role = "trusted"
allow = ["vm-name"]
deny = []

[agent."agent-b"]
role = "sandboxed"
allow = []
deny = ["vm-name"]
```

### Top-Level Sections

* `[vm."<name>"]` — Configuration for each VM
* `[agent."<id>"]` — Configuration for each agent

---

## 🔍 Field Definitions

| Field                              | Section     | Type             | Required | Description                                                          |
| ---------------------------------- | ----------- | ---------------- | -------- | -------------------------------------------------------------------- |
| `roles.<role>`                     |             | table            | yes      | Defines capabilities per role (`trusted`, `sandboxed`, `audited`)    |
| `roles.<role>.can_create`          |             | boolean          | yes      | Whether the role can create VMs or containers                        |
| `roles.<role>.can_attach_terminal` |             | boolean          | yes      | Whether the role can attach to terminal sessions                     |
| `roles.<role>.audited`             |             | boolean          | yes      | Whether all actions by this role are logged                          |
| `permissions.<agent>`              |             | table            | no       | Optional override for specific agent identity (e.g., `model:llama3`) |
| `trusted`                          | `[vm.*]`    | bool             | yes      | Declares VM as internally trusted                                    |
| `agents`                           | `[vm.*]`    | array            | yes      | List of agent IDs allowed to interact                                |
| `role`                             | `[agent.*]` | string           | yes      | Must be `trusted`, `sandboxed`, or `audited`                         |
| `allow`                            | `[agent.*]` | array            | yes      | Explicit allowlist of VM IDs                                         |
| `deny`                             | `[agent.*]` | array            | yes      | Explicit denylist of VM IDs                                          |
| `capabilities`                     | `[agent.*]` | array (optional) | no       | Future field for fine-grained permissions                            |

---

## 🔒 Enforcement Logic

* All MCP commands must pass authorization via this policy file.
* Authorization checks combine:
  * Host VM policy (e.g., `trusted = true`)
  * Agent intent (`allow` vs `deny`)
* **No implicit escalation**: Missing fields default to deny.

### Precedence Table

| Host Policy    | Agent Policy   | Result  |
| -------------- | -------------- | ------- |
| Implicit Deny  | Implicit Deny  | ❌ Deny  |
| Implicit Deny  | Explicit Allow | ✅ Allow |
| Explicit Deny  | Implicit Allow | ❌ Deny  |
| Explicit Deny  | Explicit Allow | ❌ Deny  |
| Explicit Allow | Explicit Allow | ✅ Allow |
| Implicit Allow | Explicit Allow | ✅ Allow |

---

## 🧰 Role-Based Policy Examples

### Trusted Model

```toml
[roles.trusted]
can_create = true
can_attach_terminal = true
audited = false
```

### Sandboxed Model

```toml
[roles.sandboxed]
can_create = false
can_attach_terminal = true
audited = true
```

---

## 🧮 Role vs Command Matrix (Partial, Extensible)

This is a placeholder mapping to define default command permissions per role.

| Role        | Allowed Command Types                                        |
| ----------- | ------------------------------------------------------------ |
| `trusted`   | `vm/exec`, `vm/attach`, `vm/info`, `log/query`, `model/send` |
| `sandboxed` | `vm/info`, `model/send`, `log/query`                         |
| `audited`   | `vm/exec` (with recording), `log/query`                      |

> This matrix is a starting point and is subject to future extension. Future versions may enforce this via schema or capabilities list.

---

## 🎯 Agent-Specific Overrides

You can override global role definitions for individual agents. Agent names are expected to match identifiers from MCP sessions (e.g., `model:<name>`).

```toml
[permissions.model:llama3]
can_create = false
can_attach_terminal = false
audited = true
```

---

## 🔐 Security Model

Hydravisor enforces a **deny-by-default** policy. If no role or override is specified for an agent, the action is denied.

### Default Behavior:

* **Unknown agents** are denied all actions
* **Audited actions** are logged to the session record if `audited = true`
* Role lookups cascade: agent → override → role → deny

---

## 🧪 Schema Validation

* **File**: `policy.schema.json`
* Tooling: `hydravisor policy validate`
* Validation includes:
  * Allowed roles only
  * Unique names
  * Matching references between agent and VM blocks
  * Recognition of optional `capabilities` array (no enforcement yet)

---

## 🛠 CLI Tooling

### `hydravisor policy validate`

* Validate structure against JSON Schema

### `hydravisor policy check`

* Simulate authorization decision between agent and VM

Example:

```sh
$ hydravisor policy check --agent agent-a --vm vm-name
✔ Allowed
```

---

## 🧠 Design Principles

* Immutable during runtime (no live reload)
* Manual edit with clear structure
* No policy mutations allowed via UI or MCP
* Future versioning and diff audit is external to core tool

---

## 📌 Future Enhancements

* Role inheritance support (planned post-MVP)
* Optional role capabilities (`exec`, `record`, `vm/attach`) per agent
* Policy change watcher + trigger system

---

Document maintained as part of the Hydravisor Project.  
Author: Kelsea + Alethe · 2025