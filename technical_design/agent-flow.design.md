# Agent Behavior & Policy Flow Design for Hydravisor

## 🧠 Purpose
This document defines how AI agents interact with Hydravisor via the Model Context Protocol (MCP), focusing on session scoping, access policy evaluation, and agent role management.

---

## 🧱 Session Isolation
- **Per-Agent Isolation:** Each agent (local or remote) is restricted to the VM/container it was attached to.
- **Terminal Scope:** Agents cannot access other terminal panes or sessions not explicitly assigned in policy.
- **No Shared State:** Agents do not share memory, file systems, or network scopes unless explicitly permitted.

---

## 🔐 Declarative Role and Policy Evaluation

All access control and permission decisions are made via a declarative policy system defined in `policy.toml`. Agents cannot request changes to their own permissions. 

### Role Types
- `trusted`: Full access to assigned VM + interactive tools.
- `sandboxed`: Highly restricted, can only access specific terminal commands.
- `audited`: Allowed to act but all actions are logged and reviewed.

### Policy Scoping Hierarchy
- Policies are evaluated at **two levels**: the VM/container (host scope) and the agent (identity scope).
- The **effective permission** is computed by intersecting the permissions from both scopes using the following logic:

### 🔀 Permission Evaluation Matrix
| Host Policy | Agent Policy | Result  |
|-------------|--------------|---------|
| implicit deny | implicit deny | deny    |
| implicit deny | explicit accept | accept |
| implicit deny | explicit deny   | deny   |
| explicit deny | explicit accept | deny   |
| explicit accept | implicit deny | accept |
| explicit accept | explicit accept | accept |

### Glossary
- **Implicit**: No policy defined (defaults to deny)
- **Explicit**: Policy value present in configuration
- **Accept/Deny**: Final decision for that action/permission

---

## 📜 Example Use Case
A local `ollama` model is classified as `sandboxed` in `policy.toml`, and assigned to a VM with an `audited` access profile.

- Policy file explicitly denies MCP `vm/delete` from `sandboxed` agents
- Host profile allows `vm/delete`
- Final decision: **denied** (explicit deny takes precedence)

---

## 🚫 No Runtime Promotion/Demotion
- **Static Role Binding:** An agent's role is bound at launch and cannot be elevated at runtime.
- **Manual Configuration Required:** Any change to agent permissions or trust level must be made via editing the policy file and restarting/rebinding the agent.

---

## 🔄 Future Work
- Define a `hydravisor policy validate` CLI command
- Introduce a `policy-check` endpoint for runtime simulation/debugging
- TUI panel for visualizing effective permission matrix for each agent
