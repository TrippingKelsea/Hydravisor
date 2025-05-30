# Hydravisor – Logging and Audit Specification

**Version:** 0.1.0
**File:** `./technical_design/logging_audit.md`

---

## 🎯 Purpose

This document specifies how Hydravisor handles operational, audit, and model activity logging. Logs are designed for human readability, postmortem traceability, and long-term agent accountability.

---

## 🗂 Log Categories

### 1. **System Logs**

General operational behavior:

* Startup/shutdown
* Crate initialization
* Configuration load success/failure

Location: `~/.hydravisor/logs/system.log`
Format: line-delimited, timestamped plaintext

### 2. **VM & Container Lifecycle Logs**

Logs each managed instance:

* Create/delete/snapshot
* Attach/detach model events
* Resource allocations or failures

Location: `~/.hydravisor/logs/instances/{id}/lifecycle.log`
Format: timestamped JSONL

### 3. **tmux Session Recordings**

Archived terminal I/O per agent/user VM session:

* Keystroke and model input
* Output buffer history

Location: `~/.hydravisor/logs/instances/{id}/terminal.(log|jsonl)`
Format:

* `ansi`: raw terminal escape sequences
* `jsonl`: structured log lines with user/model distinction

### 4. **MCP Activity Logs**

Every inbound/outbound MCP message:

* Validated vs rejected
* Source identity
* Affected instance or subsystem

Location: `~/.hydravisor/logs/mcp/mcp_activity.jsonl`
Format: structured JSONL with cryptographic identity tags

### 5. **Audit Ledger**

Immutable, append-only event trail:

* Role elevation or command overrides
* Policy violations or denials
* Model access attempts

Location: `~/.hydravisor/logs/audit/audit_ledger.jsonl`
Format: hash-chained JSONL (with optional Merkle root index)

---

## 🔒 Integrity Strategies

* Timestamps signed with session key (optionally GPG or ed25519)
* Ledger optionally committed to Merkle tree at shutdown
* Enforced `append_only` filesystem flag (where available)
* Checksum verification in background for archival logs

---

## 🧪 Functional Test Coverage

| Scenario                      | Expectation                                  |
| ----------------------------- | -------------------------------------------- |
| Log directory unwritable      | Startup fails fast with actionable error     |
| Model triggers denied action  | Audit log entry created with metadata        |
| MCP message missing signature | Rejected and logged with trace               |
| VM shutdown triggers autosave | tmux log + lifecycle + MCP records completed |
| User replay of session        | Restores original log faithfully             |

---

## 🧭 CLI Tools for Logs

```sh
hydravisor logs list                # List recent logs and active instances
hydravisor logs tail --vm abc123   # Follow latest log events for a VM
hydravisor logs export --format=cast --id abc123
```

---

*Document authored by Kelsea & Alethe – 2025*
