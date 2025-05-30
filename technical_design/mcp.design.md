# Hydravisor – Model Context Protocol Design

**Version:** 0.1.2  
**File:** `./technical_design/mcp.design.md`

---

## 🎯 Purpose

This document defines the Model Context Protocol (MCP) used within Hydravisor to enable structured, secure message routing between local agents, AI models, and virtual machines. It serves as a schema contract for agent interaction, AI orchestration, and VM lifecycle messaging. It formalizes the semantics of session scope, command structure, routing logic, and policy enforcement.

---

## 📦 Message Contract Overview

All MCP messages are JSON-encoded, with a required `type` field and optional `meta` block. Security headers are applied externally and validated against Hydravisor's runtime policy engine.

---

## 📘 Core Message Types

| Type                 | Direction       | Description                               |
| -------------------- | --------------- | ----------------------------------------- |
| `vm/create`          | Client ➝ Server | Request to provision a new VM             |
| `vm/delete`          | Client ➝ Server | Terminate an existing VM                  |
| `vm/state`           | Client ⇄ Server | Query or broadcast VM state changes       |
| `vm/attach-terminal` | Client ➝ Server | Request tmux-attached terminal session    |
| `model/log`          | Server ➝ Client | Relay session logs or structured messages |
| `mcp/heartbeat`      | ⇄ Bidirectional | Keepalive for connection health           |
| `mcp/authorize`      | Server ➝ Client | Issue or deny credentialed access         |
| `mcp/error`          | Server ➝ Client | Standardized error response envelope      |

---

## 🧪 Example Messages

### `vm/create`

```json
{
  "type": "vm/create",
  "os": "ubuntu-22.04",
  "cpu": 2,
  "ram": "4GB",
  "model": "ollama:llama3",
  "meta": {
    "name": "llama-sandbox",
    "record_session": true
  }
}
```

### `vm/delete`

```json
{
  "type": "vm/delete",
  "instance_id": "vm-23ac1a9d"
}
```

### `vm/state`

```json
{
  "type": "vm/state",
  "instance_id": "vm-23ac1a9d",
  "query": true
}
```

### `vm/attach-terminal`

```json
{
  "type": "vm/attach-terminal",
  "instance_id": "vm-23ac1a9d",
  "role": "auditor"
}
```

### `model/log`

```json
{
  "type": "model/log",
  "source": "vm-23ac1a9d",
  "payload": [
    { "timestamp": "2025-05-29T20:15:42Z", "msg": "Agent initialized." },
    { "timestamp": "2025-05-29T20:15:50Z", "msg": "Received task input." }
  ]
}
```

---

## 🧭 Routing Architecture

### Entities

* **Agent**: A user-facing or automated process initiating requests.
* **VM**: A containerized or virtualized workload.
* **Model**: Local or remote LLM-like compute.

### Flow Model

```
Agent → MCP Dispatcher → Target VM or Model
                     ↘ Logs, Audit Queue, UI Hooks
```

Each route is evaluated for permission via `policy.toml`, and unauthorized routes are blocked before transmission.

---

## 📦 Command Structure

### Envelope Format

```json
{
  "src": "agent-a",
  "dst": "vm-foo",
  "type": "vm/exec",
  "payload": { "command": "ls -alh" }
}
```

### Common Command Types

| Type         | Payload Schema          | Description                       |
| ------------ | ----------------------- | --------------------------------- |
| `vm/exec`    | `{command: String}`     | Execute shell command             |
| `vm/attach`  | `{terminal_id: String}` | Bind model to VM terminal session |
| `vm/info`    | `{}`                    | Request current VM metadata       |
| `model/send` | `{text: String}`        | Forward message to attached model |
| `log/query`  | `{session_id: String}`  | Retrieve past session logs        |

### Payload Schema Reference

* Schema stub: `payload.schema.json`
* Validation tool: TBD (deferred to post-MVP)

---

## 🛡️ Policy Scoping & Enforcement

* All commands must pass authz check:
  * `src` must be explicitly allowed by both `agent` and `vm` config
  * `type` must not exceed scope of agent role (e.g., `sandboxed` agents blocked from `vm/exec`)
* Deny-by-default if:
  * Any field is missing
  * Command type is undefined
  * Agent or VM is unrecognized

---

## ⏳ Session Scoping

* Each MCP session is:
  * Bound to a unique agent ↔ VM relationship
  * Assigned an internal `session_id` for tracking logs, models, messages
* Agents may operate across **multiple concurrent sessions** (configurable default: 10)
* Sessions are ephemeral but loggable

---

## 🔄 Response Conventions

* Success responses may echo the `instance_id` or `ok: true`
* Failures should include a `mcp/error` type with a `code` and `message`:

```json
{
  "type": "mcp/error",
  "code": 403,
  "message": "Access denied: insufficient role permissions"
}
```

---

## 🔐 Security Considerations

Each message should be wrapped with metadata from Hydravisor's session enforcement layer:

* `agent_fingerprint`
* `role` (e.g., `trusted`, `sandboxed`, `audited`)
* Optional signature or HMAC value

Hydravisor will deny messages violating ACLs or originating from unauthorized roles.

---

## 🔄 Failure & Fallback

| Error Type             | Response Behavior            |
| ---------------------- | ---------------------------- |
| Invalid route          | Log error, respond to sender |
| Unauthorized command   | Deny and audit               |
| Unavailable target     | Respond with 503-equivalent  |
| Parsing/format error   | Respond with 400-equivalent  |
| Timeout (>10s default) | Log timeout and abort route  |

Internal messages may surface to the user via the TUI (future work), and always emit to log stream.

---

## 🔐 Security Guarantees

* No implicit trust across sessions
* Agent role is scoped and enforced per command
* Command logs include full envelope for audit
* Model access must be explicitly attached and policy-approved

---

## 🔮 Future Extensions

* Command chaining (`pipe` or `macro`) support
* Agent ↔ Agent messaging
* Rate limits or budgeted model access
* JSON Schema validation for command payloads (`payload.schema.json`)
* `mcp/heartbeat` support for long-lived agents or sessions
* Retry logic and exponential backoff for recoverable errors

---

*Document authored by Kelsea & Alethe – 2025*