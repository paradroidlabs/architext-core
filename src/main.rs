//! Architext Core — GUI Entry Point
//!
//! Boots the native egui Dashboard, letting it connect to the active project
//! and read the filesystem state and daemon events.

use std::path::PathBuf;
use architext_core::ipc::{SignalBus, active_project_path_file};

fn main() -> eframe::Result {
    // Load environment variables from .env
    dotenvy::dotenv().ok();

    // Resolve project directory — default to active project file, CLI arg, or CWD
    let active_path_file = active_project_path_file();
    let project_dir = if let Ok(contents) = std::fs::read_to_string(&active_path_file) {
        let path = PathBuf::from(contents.trim());
        if path.exists() && path.is_dir() {
            path
        } else {
            std::env::args()
                .nth(1)
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().expect("Failed to get CWD"))
        }
    } else {
        std::env::args()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().expect("Failed to get CWD"))
    };

    // Ensure active project file is written/synchronized
    let _ = std::fs::write(&active_path_file, project_dir.to_string_lossy().to_string());

    // Initialize event logger for the UI dashboard (reads from events.jsonl)
    let logger = architext_core::logger::EventLogger::new();

    // Initialize shared agent status (lock-free UI polling)
    let agent_status = architext_core::logger::AgentStatus::new();

    // Initialize the IPC signal bus (.architext/ directory)
    let signal_bus = SignalBus::new(&project_dir);
    let _ = signal_bus.ensure_dirs();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Architext: Narrative IDE"),
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
