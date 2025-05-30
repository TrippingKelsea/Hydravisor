# policy_cli_tools.md

## 🛡️ Policy Validation and Simulation Tools

Hydravisor includes a suite of command-line tools under the `hydravisor policy` subcommand to validate, test, and simulate authorization flows without launching VMs or containers.

---

### ✅ `hydravisor policy validate`

**Purpose**: Validates the syntax and structure of the `policy.toml` file using the associated JSON schema.

**Usage**:
```bash
hydravisor policy validate [--path ./technical_design/policy.toml]
```

**Options**:
- `--path`: Optional. Path to the policy file (defaults to `$XDG_CONFIG_HOME/hydravisor/policy.toml`).

**Behavior**:
- Loads the TOML file.
- Validates against `policy.schema.json`.
- Prints detailed diagnostics for any invalid sections.
- Returns non-zero exit code on error.

**Example Output**:
```
✔ policy.toml is valid.
```

---

### 🧮 `hydravisor policy check`

**Purpose**: Simulates a policy decision for a given agent, VM, and requested action.

**Usage**:
```bash
hydravisor policy check \
    --agent-id ollama-llama3 \
    --vm-id test-vm \
    --action attach_terminal
```

**Options**:
- `--agent-id`: The identifier for the requesting agent (must match a policy entry).
- `--vm-id`: The virtual machine context (used to apply host-specific rules).
- `--action`: The permission being evaluated (e.g., `create_vm`, `attach_terminal`).

**Behavior**:
- Loads and parses the `policy.toml`.
- Computes the intersection of the host and agent policies.
- Evaluates rules based on precedence:
  - `explicit-deny` > `explicit-allow`
  - Implicit `deny` unless explicitly `allow`
- Prints decision outcome and rule source.

**Example Output**:
```
Result: DENY
Reason: agent 'ollama-llama3' is explicitly denied 'attach_terminal' in host policy for 'test-vm'
```

**Use Cases**:
- CI/CD validation.
- Pre-deployment simulation of agent permissions.
- Debugging agent failures or unexpected denials.

---

## 📁 Schema Reference
- `policy.toml` schema: `./technical_design/policy.schema.json`
- Default policy path: `$XDG_CONFIG_HOME/hydravisor/policy.toml`

---

*Documented: 2025-05-29 — Kelsea & Alethe*

