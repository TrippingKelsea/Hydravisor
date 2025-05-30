# Hydravisor – TUI Design Document

**Version:** 0.1.1  
**File:** `./technical_design/tui.design.md`

---

## 🎯 Purpose
This document outlines the visual and interactive behavior of the Hydravisor terminal interface, built using the `ratatui` crate. It describes UI structure, navigation patterns, state management, and keybinding philosophy for both session and modal modes.

---

## 📐 Layout Overview

### Root Application Panes
```text
+--------------------------------------------------------------+
| [Status Bar: Mode | Connected Model | Clock | Notifications] |
+---------------------+----------------------------------------+
| [VM/Container List] | [Detail View Panel]                    |
|                     |                                        |
+---------------------+----------------------------------------+
| [Dialog Interface (Chat with Model)]                         |
+--------------------------------------------------------------+
```

### Pane Descriptions
- **Status Bar:** Persistent header showing current mode, connected model, clock, and notifications
- **VM/Container List:** Navigable list of running and available instances
- **Detail View Panel:** Flip-through sub-pane (controlled by Tab/Shift-Tab or dedicated key) that cycles through:
  - Info Panel
  - Logs
  - MCP Connection List
  - MCP Connection Details
  - Network Connections
  - Running Agents
  - Agent Details / Logs
- **Dialog Interface:** Dedicated pane for current model interaction (chat/message-based)

Note: The entire UI exists in the native application window. Modal overlays are excluded from this scope and will be considered in future revisions.

---

## 🔀 Application State Diagram

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> DialogEntry: 'd' key
    Idle --> TerminalAttach: 't' key
    Idle --> CreateForm: 'n' key
    CreateForm --> Launching
    Launching --> Running
    DialogEntry --> Running
    TerminalAttach --> Running
    Running --> [*]
```

---

## 🎚 Input & Navigation

### Navigation Keys (Session Mode)
- `Tab`: Cycle forward through detail view sub-panes
- `Shift-Tab`: Cycle backward through sub-panes
- `↑/↓`: Scroll list views or text
- `Enter`: Select focused item

### Action Keys
- `n`: New VM/container form
- `t`: Attach terminal to instance
- `a`: Open dialog with model ("attach model")
- `q`: Close current pane or dialog

---

## 🔑 Modal Mode Support

When configured for modal behavior (`mode = "modal"`), Hydravisor reacts only to a special tmux keychain:

### Example Chain:
- `C-b` → `C-9` → `n`: Trigger VM create form
- `C-b` → `C-9` → `a`: Attach model dialog

This avoids interfering with default tmux or vim bindings.

---

## 🔁 Async Event Flow
Hydravisor uses a central async runtime (`tokio`) to handle non-blocking tasks:

- VM/container lifecycle operations
- Model inference interactions (Ollama/Bedrock)
- MCP socket I/O
- Logging and audit writes

### Message Queue
All user actions are enqueued as events:
```rust
enum UiEvent {
  KeyPress(Key),
  CreateVM(FormData),
  AttachTerminal(String),
  AttachDialog(String, Model),
  FlipDetailView,
  Tick,
}
```

Event dispatcher routes messages to subsystem handlers.

---

## 🧪 Functional UX Tests

| Feature               | Test                                       |
|----------------------|--------------------------------------------|
| Pane navigation       | All views switchable with `Tab`            |
| Detail pane cycling   | Sub-views rotate predictably               |
| Modal keychain        | Commands fire only after full sequence     |
| Async dialog attach   | Model output shown within 1s               |
| Error fallback        | Missing instance shows toast error panel   |
| Resize-aware layout   | No overflow or clipping at 80x24 or above  |

---

*Document authored by Kelsea & Alethe – 2025*
