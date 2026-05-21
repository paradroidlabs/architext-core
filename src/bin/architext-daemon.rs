//! Architext Daemon — Headless Background Orchestration Engine
//!
//! Watches the active project directory for filesystem changes,
//! evaluates narrative protocol phases, and dispatches Gemini agent actions.
//!
//! State synchronization is achieved via `~/.architext_active` tracking and
//! filesystem indicators.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, error};

use architext_core::ipc::{SignalBus, active_project_path_file};
use architext_core::watcher::FsWatcher;
use architext_core::protocol::ProtocolEngine;
use architext_core::gemini::GeminiClient;
use architext_core::logger::{EventLogger, AgentStatus};

fn get_current_active_path(active_path_file: &Path) -> PathBuf {
    if let Ok(contents) = std::fs::read_to_string(active_path_file) {
        let path = PathBuf::from(contents.trim());
        if path.exists() && path.is_dir() {
            return path;
        }
    }
    // Fallback to CWD if file doesn't exist or directory is invalid
    let cwd = std::env::current_dir().expect("Failed to get CWD");
    let _ = std::fs::write(active_path_file, cwd.to_string_lossy().to_string());
    cwd
}

fn spawn_watcher(project_dir: &PathBuf, watcher_task: &mut Option<tokio::task::JoinHandle<()>>) {
    let project_dir = project_dir.clone();
    
    // Resolve Gemini API key from environment
    let gemini_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let gemini_client = GeminiClient::new(gemini_key);
    
    // Event logger backed by events.jsonl in .architext/
    let events_file = project_dir.join(".architext").join("events.jsonl");
    let logger = EventLogger::with_file_backing(events_file);
    
    // Reset log entries/backing file on new project start
    logger.clear();
    logger.log(architext_core::logger::LogLevel::Info, "Daemon attached to project directory");

    let agent_status = AgentStatus::new();
    let signal_bus = SignalBus::new(&project_dir);
    if let Err(e) = signal_bus.ensure_dirs() {
        error!("Failed to initialize signal bus directories: {:#}", e);
        return;
    }

    let protocol_engine = Arc::new(ProtocolEngine::new(
        project_dir.clone(),
        gemini_client,
        logger,
        agent_status,
        signal_bus,
    ));

    // Trigger initial check asynchronously
    let engine_for_init = Arc::clone(&protocol_engine);
    let init_dir = project_dir.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let yaml_event = architext_core::watcher::FsEvent {
            path: init_dir.join("project.yaml"),
            kind: architext_core::watcher::FsEventKind::Modified,
        };
        engine_for_init.handle_event(yaml_event).await;
    });

    let engine = Arc::clone(&protocol_engine);
    let mut watcher = match FsWatcher::new(project_dir.clone()) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create FsWatcher: {:#}", e);
            return;
        }
    };

    let task = tokio::spawn(async move {
        info!("FsWatcher loop started for {}", project_dir.display());
        let run_result = watcher.run(move |event| {
            let engine = Arc::clone(&engine);
            async move {
                engine.handle_event(event).await;
            }
        }).await;
        
        if let Err(e) = run_result {
            error!("FsWatcher run loop failed: {:#}", e);
        }
    });

    *watcher_task = Some(task);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env
    dotenvy::dotenv().ok();

    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting Architext Headless Daemon...");

    let active_path_file = active_project_path_file();
    info!("Active project tracker path: {}", active_path_file.display());

    let current_project_dir = get_current_active_path(&active_path_file);
    info!("Initial active project path: {}", current_project_dir.display());

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                let _ = shutdown_tx.send(()).await;
            }
            Err(e) => {
                error!("Failed to register Ctrl+C handler: {}. Signal handling disabled.", e);
            }
        }
    });

    let mut watcher_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut last_path = current_project_dir.clone();

    // Start watching the initial directory
    spawn_watcher(&current_project_dir, &mut watcher_task);

    // Loop indefinitely checking for path changes or SIGINT
    loop {
        tokio::select! {
            Some(_) = shutdown_rx.recv() => {
                info!("Ctrl+C / Interrupt signal received. Shutting down daemon gracefully...");
                if let Some(task) = watcher_task.take() {
                    task.abort();
                    let _ = task.await;
                }
                break;
            }
            _ = sleep(Duration::from_secs(1)) => {
                let new_path = get_current_active_path(&active_path_file);
                if new_path != last_path {
                    info!(
                        "Active project path changed from {} to {}. Restarting watcher...",
                        last_path.display(),
                        new_path.display()
                    );
                    
                    if let Some(task) = watcher_task.take() {
                        task.abort();
                        let _ = task.await;
                    }

                    spawn_watcher(&new_path, &mut watcher_task);
                    last_path = new_path;
                }
            }
        }
    }

    Ok(())
}
