# Architext Agent Protocol

## Overview

Architext uses a **filesystem-native Inter-Process Communication (IPC)** layer to orchestrate interactions between the GUI, the internal protocol engine, and external agents (like the Gemini CLI, custom python scripts, or the Paradroid Labs Hermes agent).

The core philosophy is: **The filesystem IS the API.**

Any tool that can read/write files and handle JSON can participate as a first-class agent in the Architext ecosystem. There are no SDKs, REST servers, or sockets required.

## The `.architext/` Directory

All IPC interactions occur within the `.architext/` directory, located at the root of the active Architext project.

```text
.architext/
├── agent.lock          # JSON: Identifies the currently executing agent
├── agent.log           # Append-only log stream for UI tailing
└── signals/
    ├── cancel.signal   # Request agent abort
    ├── approve.signal  # Open a human-review gate
    └── pause.signal    # Request agent pause
```

### 1. Acquiring the Agent Lock (`agent.lock`)

To announce that an external agent is taking control (e.g., executing a generation pass or running a validation script), write a JSON object to `.architext/agent.lock`. 

The Architext UI polls this file 2 times a second. When present, the UI will shift into "External Agent" mode, turning the agent status bubble orange and displaying the agent/task name.

**Lock Schema:**
```json
{
  "agent": "gemini-cli",
  "task": "prose_generation",
  "pid": 12345,
  "started": "2026-05-17T12:00:00Z"
}
```

*Note: Architext's internal daemon uses this same lock. If the file exists, you should wait or send a `cancel.signal` if you intend to forcefully take over.*

**Clearing the Lock:**
When your external agent completes its task, simply delete `.architext/agent.lock`. The Architext UI will automatically revert to its idle state.

### 2. Streaming Logs to the UI (`agent.log`)

To provide real-time feedback to the user inside the Architext diagnostic terminal, append text to `.architext/agent.log`.

**Format:**
```text
[HH:MM:SS] Your log message here
```
*While the internal engine uses the `[HH:MM:SS]` prefix, the UI simply tails the file line-by-line, so any plain text appended here will be displayed.*

**Best Practice:**
Clear or truncate `agent.log` when you write a new `agent.lock` to ensure the UI terminal starts fresh for the new task.

### 3. Sending and Receiving Signals (`signals/`)

Signals are used to interrupt or gate execution. They are simply empty (or timestamped) files created in the `.architext/signals/` directory.

#### Supported Signals:
*   `cancel.signal`: Instructs the currently running agent to abort.
*   `approve.signal`: Used to pass a human-review gate.
*   `pause.signal`: Instructs the agent to temporarily halt execution.

#### Handling Signals as an External Agent:
If your external agent runs a long loop, it should periodically check for the existence of `.architext/signals/cancel.signal`. If found, the agent should:
1. Halt execution gracefully.
2. Delete `.architext/signals/cancel.signal` (to acknowledge receipt).
3. Delete `.architext/agent.lock` (to release control).

#### Triggering Signals from an External Agent:
To tell the internal Architext daemon to stop what it's doing, simply create `.architext/signals/cancel.signal`. The internal `ProtocolEngine` polls for this file between API calls and will abort its run when detected.

---

## Example Bash Workflow

A complete external agent integration can be as simple as a bash script:

```bash
#!/bin/bash
PROJECT_DIR="/path/to/project"
IPC_DIR="$PROJECT_DIR/.architext"

# 1. Acquire Lock
echo '{"agent": "my-script", "task": "cleanup", "started": "2026-05-17T12:00:00Z"}' > "$IPC_DIR/agent.lock"

# 2. Log Progress
echo "[12:00:01] Starting cleanup phase..." >> "$IPC_DIR/agent.log"

# 3. Check for UI cancellation
if [ -f "$IPC_DIR/signals/cancel.signal" ]; then
    echo "[12:00:02] Cancelled by user." >> "$IPC_DIR/agent.log"
    rm "$IPC_DIR/signals/cancel.signal"
    rm "$IPC_DIR/agent.lock"
    exit 1
fi

# 4. Perform Work
sleep 2
echo "[12:00:04] Cleanup complete." >> "$IPC_DIR/agent.log"

# 5. Release Lock
rm "$IPC_DIR/agent.lock"
```
