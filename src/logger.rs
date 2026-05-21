use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

/// Which log streams to display in the diagnostics terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    /// Show all log entries.
    All,
    /// Show only logs from the internal ProtocolEngine agents.
    InternalOnly,
    /// Show only logs from external CLI agents (via .architext/agent.log).
    ExternalOnly,
    /// Show only Warning and Error entries across all sources.
    ErrorsOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    AgentCall,
    /// Log entries from external CLI agents (via .architext/agent.log)
    ExternalAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub data: Option<String>, // For JSON dumps of agent calls/responses
}

use std::path::PathBuf;

pub struct EventLogger {
    entries: Arc<Mutex<Vec<LogEntry>>>,
    file_path: Arc<Mutex<Option<PathBuf>>>,
}

impl EventLogger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            file_path: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_file_backing(path: PathBuf) -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            file_path: Arc::new(Mutex::new(Some(path))),
        }
    }

    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        self.log_with_data(level, message, None);
    }

    pub fn log_with_data(&self, level: LogLevel, message: impl Into<String>, data: Option<String>) {
        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            data,
        };
        
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry.clone());
            // Keep the buffer reasonable (last 500 entries)
            if entries.len() > 500 {
                entries.remove(0);
            }
        }

        // If file backing is configured, write as JSON Line
        if let Ok(file_path_guard) = self.file_path.lock() {
            if let Some(path) = &*file_path_guard {
                if let Ok(json) = serde_json::to_string(&entry) {
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                    {
                        use std::io::Write;
                        let _ = writeln!(file, "{}", json);
                    }
                }
            }
        }
    }

    pub fn push_entry(&self, entry: LogEntry) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(entry);
            if entries.len() > 500 {
                entries.remove(0);
            }
        }
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Returns entries filtered by the given `LogFilter`.
    pub fn get_filtered_entries(&self, filter: LogFilter) -> Vec<LogEntry> {
        let all = self.entries.lock().unwrap_or_else(|e| e.into_inner()).clone();
        match filter {
            LogFilter::All => all,
            LogFilter::InternalOnly => all.into_iter().filter(|e| {
                !matches!(e.level, LogLevel::ExternalAgent)
            }).collect(),
            LogFilter::ExternalOnly => all.into_iter().filter(|e| {
                matches!(e.level, LogLevel::ExternalAgent)
            }).collect(),
            LogFilter::ErrorsOnly => all.into_iter().filter(|e| {
                matches!(e.level, LogLevel::Warning | LogLevel::Error)
            }).collect(),
        }
    }

    /// Clears all log entries from the buffer.
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
        // Truncate the file backing if present
        if let Ok(file_path_guard) = self.file_path.lock() {
            if let Some(path) = &*file_path_guard {
                let _ = std::fs::write(path, "");
            }
        }
    }
}

impl Clone for EventLogger {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            file_path: self.file_path.clone(),
        }
    }
}

/// Shared agent lifecycle state — read by the UI, written by the ProtocolEngine.
/// Uses atomics for the hot path (is_running check every frame) to avoid lock contention.
///
/// Supports both internal agents (dispatched by ProtocolEngine) and external agents
/// (detected via `.architext/agent.lock` polling).
pub struct AgentStatus {
    is_running: Arc<std::sync::atomic::AtomicBool>,
    cancel_requested: Arc<std::sync::atomic::AtomicBool>,
    current_task: Arc<Mutex<String>>,
    /// Whether the currently active agent is external (CLI/terminal)
    is_external: Arc<std::sync::atomic::AtomicBool>,
    /// Name of the external agent (e.g., "gemini-cli", "hermes")
    external_agent_name: Arc<Mutex<String>>,
}

impl AgentStatus {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cancel_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            current_task: Arc::new(Mutex::new(String::new())),
            is_external: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            external_agent_name: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Called by the ProtocolEngine before dispatching an agent.
    pub fn set_running(&self, task_name: &str) {
        self.cancel_requested.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut name) = self.current_task.lock() {
            *name = task_name.to_string();
        }
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Called by the ProtocolEngine when an agent finishes (success or failure).
    pub fn set_idle(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut name) = self.current_task.lock() {
            name.clear();
        }
    }

    /// Called by the UI to request cancellation.
    pub fn request_cancel(&self) {
        self.cancel_requested.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Checked by the ProtocolEngine before expensive work.
    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_requested.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Polled by the UI every frame.
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Polled by the UI for the status label.
    pub fn current_task_name(&self) -> String {
        self.current_task.lock()
            .map(|n| n.clone())
            .unwrap_or_default()
    }

    // ─── External Agent Awareness ────────────────────────────────

    /// Called by the UI when an external agent.lock is detected.
    pub fn set_external(&self, agent_name: &str, task: &str) {
        self.is_external.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut name) = self.external_agent_name.lock() {
            *name = agent_name.to_string();
        }
        if let Ok(mut t) = self.current_task.lock() {
            *t = task.to_string();
        }
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Called by the UI when the external agent.lock disappears.
    pub fn clear_external(&self) {
        self.is_external.store(false, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut name) = self.external_agent_name.lock() {
            name.clear();
        }
        // Only clear running state if no internal agent is active
        // (internal agents manage is_running via set_running/set_idle)
        if self.is_external.load(std::sync::atomic::Ordering::Relaxed) {
            self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
            if let Ok(mut t) = self.current_task.lock() {
                t.clear();
            }
        }
    }

    /// Whether the currently active agent is external (vs internal ProtocolEngine agent).
    pub fn is_external(&self) -> bool {
        self.is_external.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the external agent's name (e.g., "gemini-cli").
    pub fn external_agent_name(&self) -> Option<String> {
        if self.is_external() {
            self.external_agent_name.lock()
                .ok()
                .filter(|n| !n.is_empty())
                .map(|n| n.clone())
        } else {
            None
        }
    }
}

impl Clone for AgentStatus {
    fn clone(&self) -> Self {
        Self {
            is_running: self.is_running.clone(),
            cancel_requested: self.cancel_requested.clone(),
            current_task: self.current_task.clone(),
            is_external: self.is_external.clone(),
            external_agent_name: self.external_agent_name.clone(),
        }
    }
}

/// A full snapshot of the project for JSON export/import
#[derive(Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub title: String,
    pub timestamp: DateTime<Utc>,
    pub config: String, // project.yaml content
    pub files: Vec<FileContent>,
    pub logs: Vec<LogEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
}
