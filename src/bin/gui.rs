//! Architext GUI — Native egui Dashboard
//!
//! The visual dashboard binary. Uses the full ArchitextApp UI from src/ui.rs
//! with editor, markdown preview, split panes, diagnostics, and telemetry.
//!
//! Run with: cargo run --bin gui
//! Or:       cargo run --bin gui -- <project_dir>

use std::path::PathBuf;

use architext_core::ipc::{SignalBus, active_project_path_file};

fn main() -> eframe::Result {
    // Load environment variables from .env
    dotenvy::dotenv().ok();

    // Resolve project directory: CLI arg first, then active project file, then CWD
    let project_dir = if let Some(arg) = std::env::args().nth(1) {
        let path = PathBuf::from(&arg);
        if path.exists() && path.is_dir() {
            path
        } else {
            eprintln!("Warning: '{}' is not a valid directory, falling back.", arg);
            fallback_project_dir()
        }
    } else {
        fallback_project_dir()
    };

    // Sync the active project file so the daemon and other tools see it
    let active_path_file = active_project_path_file();
    let _ = std::fs::write(&active_path_file, project_dir.to_string_lossy().to_string());

    // Initialize event logger for the UI dashboard
    let logger = architext_core::logger::EventLogger::new();

    // Initialize shared agent status (lock-free UI polling)
    let agent_status = architext_core::logger::AgentStatus::new();

    // Initialize the IPC signal bus (.architext/ directory)
    let signal_bus = SignalBus::new(&project_dir);
    let _ = signal_bus.ensure_dirs();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Architext — Narrative IDE"),
        ..Default::default()
    };

    eframe::run_native(
        "Architext IDE",
        options,
        Box::new(move |cc| {
            Ok(Box::new(architext_core::ui::ArchitextApp::new(cc, logger, agent_status, signal_bus)))
        }),
    )
}

/// Fallback: read from ~/.architext_active, then CWD
fn fallback_project_dir() -> PathBuf {
    let active_path_file = active_project_path_file();
    if let Ok(contents) = std::fs::read_to_string(&active_path_file) {
        let path = PathBuf::from(contents.trim());
        if path.exists() && path.is_dir() {
            return path;
        }
    }
    std::env::current_dir().expect("Failed to get CWD")
}
