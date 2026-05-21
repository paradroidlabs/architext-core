# Architext Core — Project Roadmap

This document tracks the overarching phases of the Architext IDE project, shifting from its origins as NovelizeAI into a robust, filesystem-native, AI-orchestrated environment.

## Phase 1: Foundation & Core Protocol (Completed)
- [x] Establish Rust backend architecture (`watcher.rs`, `protocol.rs`).
- [x] Define the 8-Phase Narrative AI State Machine.
- [x] Integrate Gemini API (`gemini.rs`) with custom REST client.
- [x] Build the stateless agent pool (`agents.rs`).
- [x] Initialize the UI dashboard with egui (`ui.rs`), providing real-time log tailing and Markdown preview.

## Phase 2: The Sidecar UI & IPC Layer (Current)
*Objective: Transform the UI into a passive "sidecar" observer that visualizes the filesystem state, while enabling external agents to participate via a native IPC protocol.*

- [x] **Architecture Planning**: Design `.architext/` signaling directory.
- [x] **Core IPC Module**: Implement `src/ipc.rs` with `SignalBus` and `ExternalAgentLock`.
- [x] **Agent Awareness**: Update `AgentStatus` to support external locks.
- [x] **Protocol Integration**: Wire `ProtocolEngine` to write locks and check IPC signals on dispatch.
- [x] **UI Integration**: Wire `ui.rs` to poll `.architext/agent.lock` and render external agent status/logs.
- [x] **CLI Companion**: Build `architext-cli` as a lightweight terminal entrypoint for scriptable agent participation.
- [x] **Documentation**: Formalize `AGENT_PROTOCOL.md` for external tool authors.

## Phase 3: Layout Stability & Advanced UX (Completed)
*Objective: Ensure the UI is resilient, highly responsive, and capable of displaying complex AI interactions without visual breakage.*

- [x] Resolve squashing/layout bugs in the terminal diagnostics panel (removed max_height cap, proper min_height + resizable).
- [x] Implement robust split-pane controls — sidebar collapse toggle (⊣/⊢), sidebar-aware preview panel sizing.
- [x] Enhance syntax highlighting — heading depth (H1/H2/H3 at 16/15/14px), list items, bold lines, code blocks all distinctly colored per theme.
- [x] Provide granular log filtering (`ALL | INTERNAL | EXTERNAL | ERRORS` segmented control in diagnostics terminal).
- [x] Dynamic file browser — replaces static hardcoded list with live directory scanner (plan/, prose/, reviews/, context/), grouped with section headers.
- [x] Theme accent system — `accent_color()` and `separator_color()` on `Theme` for fully consistent per-theme styling.
- [x] Functional Clear button — now properly flushes the log buffer.

## Phase 4: Headless Daemonization & System Integration (Completed)
*Objective: Allow Architext to run persistently in the background across platforms (Windows/WSL).*

- [x] **Split Lifecycles**: Decouple the visual GUI (`architext-core`) from the background orchestration engine (`architext-daemon`).
- [x] **File-Backed IPC**: Move to local state-file synchronization (`.architext/agent.lock` and `.architext/agent.log`) with zero network overhead.
- [x] **Event Streaming**: Stream daemon-level events to `.architext/events.jsonl` in JSON-Lines format, tailed incrementally by the GUI.
- [x] **Global Project Tracker**: Watch and swap active project directories in real time using the `~/.architext_active` pointer.
- [x] **Windows Runner**: Implement `scripts/run-daemon.ps1` for daemon start/stop/restart/status process management on Windows.
- [x] **Linux/WSL Integration**: Create a user-level `systemd/architext-daemon.service` definition.
- [x] **CLI Companion**: Build `architext-cli` to enable external scripts and tools to lock, log, and signal the IPC bus.

## Phase 5: Paradroid Labs Ecosystem Integration (Planned)
*Objective: Fully connect Architext to the broader Paradroid methodology and external toolchain.*

- [ ] **Notion Genealogy Database Sync**: Establish automated read/write synchronization with Notion databases for characters, relationships, and world building.
- [ ] **Multi-Modal Asset Previews**: Upgrade the visual editor workspace to render AI-generated images, story maps, and structural diagrams.
- [ ] **Autonomous Pipeline & Telemetry**: Support complete multi-chapter generation passes with automatic self-critique, revision, and high-fidelity run reports.
