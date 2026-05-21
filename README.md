# Architext Core — Narrative IDE

Architext is a filesystem-native, AI-orchestrated environment for collaborative, long-form creative writing. By treating the filesystem itself as the authoritative state machine, Architext completely decouples background orchestration engines (the daemon) from graphical editors and CLI companions (the sidecars).

## Architecture & Lifecycles

The project consists of two primary binaries:

1. **`architext-daemon`**: A headless background orchestration loop that monitors files in the active project directory, runs narrative protocol state machine transitions, and dispatches stateless LLM agents (e.g., plan generation, prose generation, chapter review) using the Google Gemini API.
2. **`architext-core`**: A native desktop dashboard built using `egui` that tails background events, lets authors read and write files, switch active project directories, and monitor the current phase of the AI writing pipeline.

### The Filesystem is the IPC

Architext avoids socket, network, or shared-memory IPC. Instead:
- **State Synchronization**: All agent states are represented by files in the `.architext/` subdirectory.
  - `.architext/agent.lock`: A JSON file announcing which agent/task is currently executing and its PID.
  - `.architext/agent.log`: An append-only text log of the active agent's output.
  - `.architext/signals/`: Directories containing signal files (`cancel.signal`, `approve.signal`, `pause.signal`) for human-in-the-loop control.
- **Daemon Events**: The background daemon appends structured events to `.architext/events.jsonl` in JSON Lines format. The GUI tails this file to populate its diagnostics console in real-time.
- **Global Project Tracker**: A file located at `~/.architext_active` (resolved via `HOME` or `USERPROFILE`) contains the absolute path of the currently active project. The daemon polls this file once per second to hot-swap watchers and state contexts.

---

## Getting Started

### 1. Configure the Environment
Create a `.env` file in the repository root and define your Gemini API key:
```env
GEMINI_API_KEY=your-api-key-here
RUST_LOG=info
```

### 2. Build the Binaries
Compile the project in release mode:
```bash
cargo build --release
```

---

## Daemon Background Running

### Windows (PowerShell)
Manage the daemon lifecycle using the helper script:
```powershell
# Start the background daemon
.\scripts\run-daemon.ps1 -Action start

# Check running status
.\scripts\run-daemon.ps1 -Action status

# Stop the daemon
.\scripts\run-daemon.ps1 -Action stop

# Restart the daemon
.\scripts\run-daemon.ps1 -Action restart
```

### Linux / WSL (Systemd)
A user-level systemd service is provided in `systemd/architext-daemon.service`. To set it up:
```bash
# Copy service to user configuration directory
mkdir -p ~/.config/systemd/user
cp systemd/architext-daemon.service ~/.config/systemd/user/

# Reload and enable the service
systemctl --user daemon-reload
systemctl --user enable architext-daemon.service
systemctl --user start architext-daemon.service

# View daemon logs
journalctl --user -u architext-daemon.service -f
```

---

## GUI Dashboard

Start the passive observer desktop UI:
```bash
cargo run --bin architext-core
```

Inside the GUI:
- Use the **📂 Switch Project** button to pick another project directory. This updates `~/.architext_active`, signaling the background daemon to hot-swap watcher contexts.
- Write, edit, and save drafts or plans, and watch the diagnostics terminal tail background logs from the daemon.

---

## CLI Companion Tool

To allow shell scripts or external agent frameworks (like Python-based agents or the Paradroid Labs Hermes agent) to interact with the Architext IPC bus, compile and run the CLI companion:

```bash
cargo run --bin architext-cli -- <command>
```

### Commands:
- **`status`**: Queries the active IPC state, printing the currently active project path, lock owner, lock details, and any active control signals.
- **`lock <agent> <task>`**: Manually claims the IPC lock. This signals to both the GUI and the daemon that an external agent is working.
- **`unlock`**: Releases the external agent lock.
- **`log "<message>"`**: Appends a custom log entry to `.architext/agent.log`. This stream is tailed in real-time by the GUI terminal.
- **`signal <cancel|approve|pause>`**: Writes a control signal file to `.architext/signals/`. For example, sending a `cancel` signal instructs the active daemon process to gracefully abort.
