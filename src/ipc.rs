//! IPC Signal Bus — Filesystem-Native Inter-Process Communication
//!
//! The `.architext/` directory serves as the IPC layer between the Architext UI,
//! the internal ProtocolEngine, and any external CLI agents (Gemini CLI, Hermes,
//! custom scripts, etc.).
//!
//! Philosophy: The filesystem IS the protocol. Any tool that can `touch` a file
//! or `echo >>` a log can participate. No SDK, no client library, no daemon coupling.
//!
//! Directory Layout:
//! ```text
//! .architext/
//! ├── agent.lock          # JSON: who is currently running
//! ├── agent.log           # Append-only log stream
//! └── signals/
//!     ├── cancel.signal   # Request agent abort
//!     ├── approve.signal  # Open a human-review gate
//!     └── pause.signal    # Request agent pause
//! ```

use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn};
use crate::logger::LogEntry;

/// Get the path to the global active project path tracker file.
pub fn active_project_path_file() -> PathBuf {
    let base_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    Path::new(&base_dir).join(".architext_active")
}

/// The IPC signal directory name (relative to project root).
const IPC_DIR: &str = ".architext";
const SIGNALS_DIR: &str = "signals";
const AGENT_LOCK_FILE: &str = "agent.lock";
const AGENT_LOG_FILE: &str = "agent.log";

// ─── Signal Kinds ────────────────────────────────────────────────────

/// The types of signals that can be sent between the UI and external agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    /// Request agent cancellation
    Cancel,
    /// Approve a human-review gate (advances protocol)
    Approve,
    /// Request agent to pause (advisory, not enforced)
    Pause,
}

impl SignalKind {
    /// The filename for this signal in the signals directory.
    pub fn filename(&self) -> &'static str {
        match self {
            SignalKind::Cancel => "cancel.signal",
            SignalKind::Approve => "approve.signal",
            SignalKind::Pause => "pause.signal",
        }
    }

    /// All known signal kinds.
    pub fn all() -> &'static [SignalKind] {
        &[SignalKind::Cancel, SignalKind::Approve, SignalKind::Pause]
    }
}

// ─── External Agent Lock ─────────────────────────────────────────────

/// Represents an external agent's claim on the project.
/// Written to `.architext/agent.lock` by external agents to announce their presence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAgentLock {
    /// The name of the agent (e.g., "gemini-cli", "hermes", "user-script")
    pub agent: String,

    /// The task being performed (e.g., "prose_ch02", "review_ch01")
    pub task: String,

    /// The process ID of the external agent (optional, for diagnostics)
    #[serde(default)]
    pub pid: Option<u32>,

    /// When the agent started this task
    #[serde(default = "Utc::now")]
    pub started: DateTime<Utc>,
}

// ─── Signal Bus ──────────────────────────────────────────────────────

/// The filesystem-native IPC bus.
///
/// Manages the `.architext/` directory and provides read/write operations
/// for agent locks, logs, and signals. All operations are synchronous
/// filesystem reads/writes — no sockets, no daemons.
#[derive(Clone)]
pub struct SignalBus {
    /// The `.architext/` directory path
    ipc_dir: PathBuf,

    /// Tracks the last line number read from agent.log for incremental tailing
    last_log_line: std::sync::Arc<std::sync::Mutex<usize>>,

    /// Tracks the last line number read from events.jsonl for incremental tailing
    last_event_line: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl SignalBus {
    /// Create a new SignalBus rooted at the given project directory.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            ipc_dir: project_dir.join(IPC_DIR),
            last_log_line: std::sync::Arc::new(std::sync::Mutex::new(0)),
            last_event_line: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }

    /// Ensure the `.architext/` directory structure exists.
    /// Called during application startup.
    pub fn ensure_dirs(&self) -> Result<()> {
        let signals_dir = self.ipc_dir.join(SIGNALS_DIR);
        std::fs::create_dir_all(&signals_dir)
            .with_context(|| format!("Failed to create IPC directory: {}", signals_dir.display()))?;

        info!("[IPC] Signal bus initialized: {}", self.ipc_dir.display());
        Ok(())
    }

    // ─── Agent Lock ──────────────────────────────────────────────

    /// Read the current agent lock, if any external agent has claimed the project.
    /// Returns `None` if no lock file exists or it's invalid.
    pub fn read_agent_lock(&self) -> Option<ExternalAgentLock> {
        let lock_path = self.ipc_dir.join(AGENT_LOCK_FILE);
        if !lock_path.exists() {
            return None;
        }

        match std::fs::read_to_string(&lock_path) {
            Ok(contents) => {
                match serde_json::from_str::<ExternalAgentLock>(&contents) {
                    Ok(lock) => {
                        debug!("[IPC] External agent lock: {} (task: {})", lock.agent, lock.task);
                        Some(lock)
                    }
                    Err(e) => {
                        warn!("[IPC] Malformed agent.lock (ignoring): {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                // File might be mid-write by the external agent — not an error
                debug!("[IPC] Could not read agent.lock: {}", e);
                None
            }
        }
    }

    /// Write an agent lock (used by the internal ProtocolEngine when dispatching).
    pub fn write_agent_lock(&self, agent: &str, task: &str) -> Result<()> {
        let lock = ExternalAgentLock {
            agent: agent.to_string(),
            task: task.to_string(),
            pid: Some(std::process::id()),
            started: Utc::now(),
        };

        let lock_path = self.ipc_dir.join(AGENT_LOCK_FILE);
        let json = serde_json::to_string_pretty(&lock)
            .context("Failed to serialize agent lock")?;
        std::fs::write(&lock_path, json)
            .with_context(|| format!("Failed to write agent lock: {}", lock_path.display()))?;

        debug!("[IPC] Agent lock written: {} -> {}", agent, task);
        Ok(())
    }

    /// Remove the agent lock (called when an agent finishes).
    pub fn clear_agent_lock(&self) -> Result<()> {
        let lock_path = self.ipc_dir.join(AGENT_LOCK_FILE);
        if lock_path.exists() {
            std::fs::remove_file(&lock_path)
                .with_context(|| format!("Failed to remove agent lock: {}", lock_path.display()))?;
            debug!("[IPC] Agent lock cleared");
        }
        Ok(())
    }

    // ─── Agent Log ───────────────────────────────────────────────

    /// Read new lines from agent.log since the last poll.
    /// Returns the new lines and updates the internal cursor.
    /// This is designed to be called every UI frame (500ms).
    pub fn read_agent_log_tail(&self) -> Vec<String> {
        let log_path = self.ipc_dir.join(AGENT_LOG_FILE);
        if !log_path.exists() {
            if let Ok(mut last_line) = self.last_log_line.lock() {
                *last_line = 0;
            }
            return Vec::new();
        }

        let file = match std::fs::File::open(&log_path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    lines.push(line);
                }
            }
        }

        let lines_count = lines.len();
        let mut last_line = self.last_log_line.lock().unwrap_or_else(|e| e.into_inner());

        if lines_count < *last_line {
            *last_line = 0;
        }

        let mut new_lines = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i >= *last_line {
                new_lines.push(line.clone());
            }
        }

        *last_line = lines_count;
        new_lines
    }

    /// Read new lines from events.jsonl since the last poll.
    /// Returns the new log entries and updates the internal cursor.
    /// This is designed to be called every UI frame (500ms).
    pub fn read_daemon_events_tail(&self) -> Vec<LogEntry> {
        let events_path = self.ipc_dir.join("events.jsonl");
        if !events_path.exists() {
            if let Ok(mut last_line) = self.last_event_line.lock() {
                *last_line = 0;
            }
            return Vec::new();
        }

        let file = match std::fs::File::open(&events_path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let reader = BufReader::new(file);
        let mut lines = Vec::new();
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.trim().is_empty() {
                    lines.push(line);
                }
            }
        }

        let lines_count = lines.len();
        let mut last_line = self.last_event_line.lock().unwrap_or_else(|e| e.into_inner());

        if lines_count < *last_line {
            *last_line = 0;
        }

        let mut new_entries = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i >= *last_line {
                if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
                    new_entries.push(entry);
                }
            }
        }

        *last_line = lines_count;
        new_entries
    }

    /// Append a line to agent.log (used by internal engine and CLI tool).
    pub fn append_agent_log(&self, message: &str) -> Result<()> {
        let log_path = self.ipc_dir.join(AGENT_LOG_FILE);
        use std::io::Write;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("Failed to open agent.log: {}", log_path.display()))?;

        let timestamp = Utc::now().format("%H:%M:%S");
        writeln!(file, "[{}] {}", timestamp, message)?;

        Ok(())
    }

    // ─── Signals ─────────────────────────────────────────────────

    /// Check if a signal file exists.
    pub fn check_signal(&self, kind: SignalKind) -> bool {
        self.ipc_dir.join(SIGNALS_DIR).join(kind.filename()).exists()
    }

    /// Write a signal file. The content is the ISO timestamp of when it was sent.
    pub fn write_signal(&self, kind: SignalKind) -> Result<()> {
        let signal_path = self.ipc_dir.join(SIGNALS_DIR).join(kind.filename());
        let timestamp = Utc::now().to_rfc3339();
        std::fs::write(&signal_path, &timestamp)
            .with_context(|| format!("Failed to write signal: {}", signal_path.display()))?;

        info!("[IPC] Signal written: {:?}", kind);
        Ok(())
    }

    /// Clear (consume) a signal file.
    pub fn clear_signal(&self, kind: SignalKind) -> Result<()> {
        let signal_path = self.ipc_dir.join(SIGNALS_DIR).join(kind.filename());
        if signal_path.exists() {
            std::fs::remove_file(&signal_path)
                .with_context(|| format!("Failed to clear signal: {}", signal_path.display()))?;
            debug!("[IPC] Signal consumed: {:?}", kind);
        }
        Ok(())
    }

    /// Clear all signal files.
    pub fn clear_all_signals(&self) -> Result<()> {
        for kind in SignalKind::all() {
            self.clear_signal(*kind)?;
        }
        Ok(())
    }

    // ─── Status Query ────────────────────────────────────────────

    /// Get the path to the IPC directory (for display/diagnostics).
    pub fn ipc_dir(&self) -> &Path {
        &self.ipc_dir
    }

    /// Check if the IPC directory has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.ipc_dir.join(SIGNALS_DIR).exists()
    }

    /// Get the path to the agent.log file.
    pub fn agent_log_path(&self) -> PathBuf {
        self.ipc_dir.join(AGENT_LOG_FILE)
    }
}
