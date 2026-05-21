# Snapshot: phase3-layout-ux
**Saved:** 2026-05-21T09:00:00Z
**Project:** `C:\Users\mkibb\Documents\deep-meteor\architext-core`
**Status:** COMPLETE

## Objective
Implement Phase 3 of the Architext Core roadmap — Layout Stability & Advanced UX — fixing the diagnostics panel squashing bug, adding a sidebar collapse toggle, replacing the static file browser with a dynamic directory scanner, wiring the Clear button, and adding a 4-way log filter to the diagnostics terminal.

## Completed Milestones
- [x] **`logger.rs`** — Added `LogFilter` enum (`All`, `InternalOnly`, `ExternalOnly`, `ErrorsOnly`), `get_filtered_entries(filter)` method, and functional `clear()` method (was previously a no-op)
- [x] **`theme.rs`** — Added `accent_color() -> Color32` and `separator_color() -> Color32` per-theme semantic color accessors
- [x] **`ui.rs`** — Full Phase 3 rewrite:
  - Fixed diagnostics squash bug: replaced `max_height(250.0)` with `min_height(120.0)` + fully resizable, no cap
  - Added `show_sidebar: bool` field + `⊣/⊢` toggle button in top bar header
  - Added `log_filter: LogFilter` field + segmented control (`ALL | INTERNAL | EXTERNAL | ERRORS`)
  - Replaced static 6-file hardcoded list with live `refresh_project_files()` scanner (~1Hz cache), grouped by directory with section headers
  - Clear button now calls `self.logger.clear()`
  - Heading depth H1/H2/H3 now render at 16/15/14px respectively; list items and bold lines distinctly colored
  - Split-pane `left_offset` now respects `show_sidebar` state (550px sidebar visible, 50px collapsed)
  - All accent uses moved to `self.theme.accent_color()` instead of hardcoded `Color32::from_rgb(0, 255, 100)`
  - Reset button now removes entire `plan/`, `prose/`, `reviews/`, `context/` directories
- [x] **`ROADMAP.md`** — Phase 3 marked `(Completed)` with expanded notes
- [x] **Build verified** — `cargo build` → `Finished dev profile in 42.91s`, zero errors

## Pending Next Step
**Phase 4: Headless Daemonization & System Integration** is next per `ROADMAP.md`:
- Separate the UI and core daemon into distinct binary lifecycles (GUI viewer vs. headless engine)
- Implement systemd/WSL service definitions
- Enable multi-project context watching and switching

First concrete step: Add a second binary target in `Cargo.toml` (`architext-daemon`) that runs the protocol engine headlessly (no eframe/egui dependency), while the existing `architext-core` binary becomes a pure GUI viewer.

## Active Constraints
- **Filesystem IS the state machine** — phase detection is always done by `Phase::detect(&project_dir)` via filesystem inspection, never stored in memory
- **Stateless UI** — the UI polls every 500ms, never maintains authoritative state. The daemon writes, the UI reads
- **Egui version: 0.31** — `eframe = "0.31"`, `egui_extras = "0.31.0"`, `egui_commonmark = "0.20"` — do not bump
- **Edition 2024** — `Cargo.toml` uses `edition = "2024"`
- **No max_height on panels** — the squashing bug was caused by `max_height`. Use `min_height` + `resizable(true)` only
- **Theme accent via `accent_color()`** — never hardcode `Color32::from_rgb(0, 255, 100)` anywhere in ui.rs
- **Gemini API keys** — loaded from `.env` via `dotenvy`, never hardcoded; key var is `GEMINI_API_KEY`

## File References
| File | Purpose | Key Lines |
|------|---------|-----------|
| `src/ui.rs` | Main egui UI — Phase 3 rewrite | Full file (700 lines) |
| `src/logger.rs` | LogFilter enum + EventLogger | L1-16 (LogFilter), L56-82 (get_filtered/clear) |
| `src/theme.rs` | Theme system + accent/separator colors | L94-120 (new methods) |
| `src/protocol.rs` | Phase state machine + event dispatch | L24-80 (Phase enum + detect()) |
| `src/main.rs` | Daemon boot + eframe init | Full file |
| `Cargo.toml` | Dependencies — egui 0.31, eframe 0.31, tokio full | Full file |
| `ROADMAP.md` | Phase tracking | Phase 4 is now "Next" |
| `project.yaml` | Test project config (Science Fiction / Cyberpunk) | Full file |

## Command History (Relevant)
```bash
cargo build  # → Finished dev profile [unoptimized + debuginfo] in 42.91s — zero errors
```

## Open Questions
- Should the Phase 4 daemon binary share the same `Cargo.toml` or be extracted into a workspace?
- WSL vs native Windows service for the headless daemon — user on Windows (Oxford, MI), uses PowerShell
- Multi-project switching: file-picker dialog or CLI arg on relaunch?

## Raw Context Notes
- The session started fresh (user said "my bad, go ahead and agentically get started") — no prior conversation context was available; derived plan from ROADMAP.md + full source read
- PowerShell stderr quirk: `cargo build 2>&1` exits with code 1 when cargo prints to stderr even on success — build was clean
- The `.antigravitycli/2b477d16-bd93-4cc2-8a35-6517efb7ab15.json` workspace session file exists but contains only project path metadata, no prior task state
