//! Filesystem Watcher — Native OS Event Detection with Debouncing
//!
//! Uses the `notify` crate to receive real-time filesystem events from the OS
//! kernel (ReadDirectoryChangesW on Windows, inotify on Linux, FSEvents on macOS).
//! Zero polling. Zero abstraction tax.
//!
//! Includes event deduplication: suppresses duplicate events on the same path
//! within a configurable window (default 500ms) to prevent double-fires.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{debug, info, trace, warn};

/// Debounce window — events on the same path within this window are suppressed.
const DEBOUNCE_MS: u64 = 500;

/// A filesystem event relevant to the Architext protocol.
#[derive(Debug, Clone)]
pub struct FsEvent {
    /// The path that changed
    pub path: PathBuf,

    /// The kind of change
    pub kind: FsEventKind,
}

/// Simplified event kinds relevant to the protocol engine.
#[derive(Debug, Clone, PartialEq)]
pub enum FsEventKind {
    /// A file was created (new plan, new chapter draft, project.yaml dropped)
    Created,

    /// A file was modified (user edited a plan, agent updated context log)
    Modified,

    /// A file was removed
    Removed,
}

/// The core filesystem watcher with built-in debouncing.
pub struct FsWatcher {
    /// The project root being watched
    root: PathBuf,

    /// Async receiver for filesystem events
    rx: mpsc::Receiver<FsEvent>,

    /// The underlying OS watcher (must be kept alive)
    _watcher: RecommendedWatcher,
}

impl FsWatcher {
    /// Create a new watcher on the given project directory.
    /// Watches recursively for all file changes.
    pub fn new(root: PathBuf) -> Result<Self> {
        let (tx, rx) = mpsc::channel(256);

        let sender = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        let kind = match event.kind {
                            EventKind::Create(_) => Some(FsEventKind::Created),
                            EventKind::Modify(_) => Some(FsEventKind::Modified),
                            EventKind::Remove(_) => Some(FsEventKind::Removed),
                            _ => None,
                        };

                        if let Some(kind) = kind {
                            for path in event.paths {
                                if is_hidden(&path) {
                                    continue;
                                }

                                if !is_relevant_file(&path) {
                                    trace!("Ignoring irrelevant file event: {}", path.display());
                                    continue;
                                }

                                let fs_event = FsEvent {
                                    path: path.clone(),
                                    kind: kind.clone(),
                                };

                                debug!("FS Event: {:?} -> {}", fs_event.kind, fs_event.path.display());

                                if sender.blocking_send(fs_event).is_err() {
                                    warn!("Event channel closed, watcher shutting down");
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Watcher error: {}", e);
                    }
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(2)),
        )
        .context("Failed to create filesystem watcher")?;

        watcher
            .watch(root.as_ref(), RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch directory: {}", root.display()))?;

        info!("Filesystem watcher attached to: {}", root.display());

        Ok(Self {
            root,
            rx,
            _watcher: watcher,
        })
    }

    /// Run the watcher loop with debouncing.
    /// Events on the same path within DEBOUNCE_MS are suppressed.
    /// The handler is now an async function.
    pub async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(FsEvent) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        info!("Watcher loop started for: {} (debounce: {}ms)", self.root.display(), DEBOUNCE_MS);

        let mut last_seen: HashMap<PathBuf, Instant> = HashMap::new();
        let debounce_duration = Duration::from_millis(DEBOUNCE_MS);

        while let Some(event) = self.rx.recv().await {
            let now = Instant::now();

            // Debounce: skip if we saw this exact path within the window
            if let Some(last) = last_seen.get(&event.path) {
                if now.duration_since(*last) < debounce_duration {
                    debug!("[DEBOUNCE] Suppressed duplicate event on: {}",
                        event.path.file_name().unwrap_or_default().to_string_lossy());
                    continue;
                }
            }

            // Record this event's timestamp
            last_seen.insert(event.path.clone(), now);

            // Periodically clean up old entries to prevent memory leak
            if last_seen.len() > 100 {
                last_seen.retain(|_, t| now.duration_since(*t) < Duration::from_secs(60));
            }

            handler(event).await;
        }

        info!("Watcher loop terminated");
        Ok(())
    }

    /// Get the project root being watched.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Check if a path is hidden (starts with `.`)
fn is_hidden(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
    })
}

/// Check if a file is relevant to the Architext protocol.
fn is_relevant_file(path: &Path) -> bool {
    if path.is_dir() {
        return false;
    }

    match path.extension().and_then(|e| e.to_str()) {
        Some("yaml" | "yml") => true,
        Some("md") => true,
        Some("json") => true,
        Some("approved") => true,
        _ => false,
    }
}
