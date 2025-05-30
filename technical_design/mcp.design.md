# MCP Message Design for Hydravisor

**Version:** 0.1.0
**File:** `./mcp.design.md`

---

## 🎯 Purpose

This document specifies the structure, semantics, and message flow design of the Model Context Protocol (MCP) as used within Hydravisor. It serves as a schema contract for agent interaction, AI orchestration, and VM lifecycle messaging.

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

## 🔐 Security Considerations

Each message should be wrapped with metadata from Hydravisor’s session enforcement layer:

* `agent_fingerprint`
* `role` (e.g., `trusted`, `sandboxed`, `audited`)
* Optional signature or HMAC value

Hydravisor will deny messages violating ACLs or originating from unauthorized roles.

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

*Document authored by Kelsea & Alethe – 2025*
