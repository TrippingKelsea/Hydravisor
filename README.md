# Hydravisor

**Hydravisor** is a terminal-based management interface for orchestrating virtual machines and containerized environments designed for local and hybrid AI workflows.

Powered by Rust and built on the `ratatui` crate, Hydravisor integrates seamlessly with KVM (via libvirt) and containerd to provide an intuitive TUI (terminal user interface) for managing isolated execution environments. It includes native support for launching AI workloads locally via [Ollama](https://ollama.com) and remotely via [Amazon Bedrock](https://aws.amazon.com/bedrock/?trk=0798126e-84b4-4be2-a791-d3c5a4d7000d&sc_channel=el).

---

## ✨ Features

* **Unified TUI for KVM + containerd**
* **Ollama integration**: Run large language models locally in secure, containerized spaces
* **Amazon Bedrock integration**: Spin up remote model inference tasks from within your TUI
* **tmux integration**: Launch and manage isolated sessions per container/VM, share context between human and model
* **Dialog tabs**: Real-time model chat sessions per instance, co-habited with interactive shell access
* **Client/Server hooks**: Easily tie Hydravisor into your distributed compute workflows

---

## 🧠 Use Cases

* Hosting local AI development environments with agent visibility control
* Running multiple isolated AI agents in parallel (e.g., Ollama + Bedrock hybrid agents)
* Creating collaborative tmux environments shared between human user and LLM
* Lightweight MLOps for bare metal developers
* Building offline-first workflows with privacy-focused AI models

---

## 🚀 Example CLI

```sh
hydravisor tui
hydravisor launch --vm ubuntu
hydravisor launch --container ollama:llama3
hydravisor dialog --attach agent1
hydravisor tmux --new-session agent1
```

---

## 🔧 Technologies

* Rust + Ratatui (TUI engine)
* Libvirt (KVM integration)
* containerd (via gRPC)
* tmux control interface
* Ollama (local models)
* AWS Bedrock SDK (remote models)

---

## 📜 License

Hydravisor is licensed under the GNU General Public License v3.0 (GPL-3.0). See `LICENSE` for more information.

---

## 🧪 Status

🚧 Currently under early development. Expect sharp edges and missing features. Contributions welcome!

---

## 👾 Author

Built by [Kelsea Blackwell](https://yourlinkhere.dev), a reliability engineer, AI explorer, and open systems advocate.

---

## 📬 Want to Help?

If you’re interested in contributing to Hydravisor, open an issue or drop in a PR. Let’s build a better interface for local AI control together.

