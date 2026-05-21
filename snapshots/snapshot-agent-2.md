# Snapshot: snapshot-agent-2

**Saved:** 2026-05-21T09:34:00Z
**Project:** C:\Users\mkibb\Documents\deep-meteor\architext-core
**Status:** ACTIVE

## Objective

Decouple visual GUI and headless daemon binaries, implement file-backed IPC logs/tailing, support multi-project context hot-swapping via `~/.architext_active`, verify background runners/scripts, and fully update documentation including the new CLI companion.

## Completed Milestones

- [x] **Split Lifecycles**: Separated egui visual viewer (`architext-core` in `src/main.rs`) and headless protocol engine (`architext-daemon` in `src/bin/architext-daemon.rs`).
- [x] **File-Backed IPC & Tailing**: Implemented JSON Lines logging to `.architext/events.jsonl` in `src/logger.rs` and incremental tailing in `src/ipc.rs` and `src/ui.rs`.
- [x] **Multi-Project Switching**: Integrated global active project tracker `~/.architext_active` in both binaries to allow real-time project switching and hot-swapping watcher threads.
- [x] **Background Running**: Created `scripts/run-daemon.ps1` for Windows lifecycle control and user-level `systemd/architext-daemon.service` for Linux/WSL.
- [x] **CLI Companion**: Developed `src/bin/architext-cli.rs` allowing external agents to query status, locks, logs, and signal files.
- [x] **Project Documentation**: Updated `ROADMAP.md` (detailed Phase 4 accomplishments, structured Phase 5 plans), `README.md` (added CLI companion guide), and `task.md` brain artifact.
- [x] **Unified Folder Structure**: Reorganized snapshots directly into the project root `snapshots/` folder for easier inspection and visibility.

## Pending Next Step

**Phase 5: Paradroid Labs Ecosystem Integration** is next:

- Define database synchronization models and schemas for Notion genealogy tables.
- Implement the Notion integration client (`src/notion.rs` or external sync agent).
- Refine GUI editor workspace to support multi-modal asset previews (flowcharts, character maps, images).

## Active Constraints

- **The Filesystem is the IPC**: All state synchronization and signaling must occur through local files (`.architext/` and `~/.architext_active`). Do not introduce network port requirements or sockets.
- **Unified Snapshots Location**: The snapshot manager now uses the root `snapshots/` folder as its primary directory instead of `.agents/skills/snapshots/`.
- **Three-Block Architecture Styling**: Adhere to the established Paradroid Labs layout and design tokens.

## File References

| File                               | Purpose                           | Notes                                                     |
| ---------------------------------- | --------------------------------- | --------------------------------------------------------- |
| `src/bin/architext-daemon.rs`      | Headless daemon engine binary     | Loop processes active project events & runs state machine |
| `src/main.rs`                      | Visual GUI client entrypoint      | Boots the egui native client                              |
| `src/ui.rs`                        | egui view layout and update loop  | Tails `.architext/events.jsonl` in real time              |
| `src/bin/architext-cli.rs`         | Interactive command-line client   | Enables external agent locking, logging, signaling        |
| `scripts/run-daemon.ps1`           | PowerShell daemon manager         | Manages start/stop/restart/status on Windows              |
| `systemd/architext-daemon.service` | Systemd service definition        | Runs daemon under user-level systemd on Linux/WSL         |
| `ROADMAP.md`                       | Overarching roadmap tracking file | Updated with detailed Phase 4 and 5 items                 |
| `README.md`                        | Core repository readme            | Updated with the CLI Companion guide                      |
| `snapshots/`                       | Unified snapshots folder          | Easily browsed at the repository root                     |

## Command History

```bash
cargo check  # Verified successful compilation of all targets: daemon, core, cli, gui
```

## Open Questions

- What Notion database structure or API credentials will we use for the genealogy sync?
- What specific asset directories or formats (e.g. PNG, SVG, Mermaid markdown) should be supported in the UI preview panel?
