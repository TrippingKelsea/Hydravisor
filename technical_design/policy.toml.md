# Hydravisor Runtime Policy File

This document defines the schema, usage, and example for `policy.toml`, which governs runtime security and access control within Hydravisor.

**Location**: `.$XDG_HOME/hydravisor/policy.toml`  
**Schema**: See [`./technical_design/policy.schema.json`](./technical_design/policy.schema.json)

---

## 🧱 Structure Overview

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

---

## 🔍 Field Definitions

| Field                              | Type    | Required | Description                                                          |
| ---------------------------------- | ------- | -------- | -------------------------------------------------------------------- |
| `roles.<role>`                     | table   | yes      | Defines capabilities per role (`trusted`, `sandboxed`, `audited`)    |
| `roles.<role>.can_create`          | boolean | yes      | Whether the role can create VMs or containers                        |
| `roles.<role>.can_attach_terminal` | boolean | yes      | Whether the role can attach to terminal sessions                     |
| `roles.<role>.audited`             | boolean | yes      | Whether all actions by this role are logged                          |
| `permissions.<agent>`              | table   | no       | Optional override for specific agent identity (e.g., `model:llama3`) |

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

## 🧪 Testing

To validate a `policy.toml` file, use the JSON schema available at:

```
./technical_design/policy.schema.json
```

You can use tools such as:

```bash
toml2json policy.toml | ajv validate -s policy.schema.json -d -
```

This ensures the structure is correct **before launch time.**

---

Document maintained as part of the Hydravisor Project.
Author: Kelsea + Alethe · 2025


