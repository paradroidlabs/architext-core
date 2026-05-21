use eframe::egui;
use egui_commonmark::CommonMarkCache;
use crate::theme::{Theme, SyntaxColors};
use crate::ipc::SignalBus;
use crate::logger::{EventLogger, LogFilter, LogLevel, ProjectSnapshot, FileContent, AgentStatus};
use crate::telemetry::{TelemetryEngine, TelemetryData};
use egui_extras::{TableBuilder, Column};

/// Layout mode for the central editor/preview area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutMode {
    EditorOnly,
    Split,
    PreviewOnly,
}

pub struct ArchitextApp {
    project_dir: std::path::PathBuf,
    selected_file: Option<String>,
    editor_content: String,
    last_loaded_mtime: std::time::SystemTime,
    commonmark_cache: CommonMarkCache,
    theme: Theme,
    layout_mode: LayoutMode,
    logger: EventLogger,
    show_diagnostics: bool,
    agent_status: AgentStatus,
    /// IPC signal bus — the sidecar observation layer
    signal_bus: SignalBus,
    /// Whether the left sidebar panels are visible
    show_sidebar: bool,
    /// Active log filter for the diagnostics terminal
    log_filter: LogFilter,
    /// Cached list of project files, refreshed periodically
    project_files: Vec<String>,
    /// Timestamp of last file list refresh
    last_file_scan: std::time::Instant,
    show_telemetry_modal: bool,
    telemetry_engine: TelemetryEngine,
    telemetry_data: Option<TelemetryData>,
    last_telemetry_scan: std::time::Instant,
}

impl ArchitextApp {
    pub fn new(cc: &eframe::CreationContext<'_>, logger: EventLogger, agent_status: AgentStatus, signal_bus: SignalBus) -> Self {
        // Set up the dark, high-contrast visual style
        let initial_theme = Theme::default();
        let mut style = egui::Style::default();
        style.visuals = initial_theme.visuals();
        cc.egui_ctx.set_style(style);

        // Extract the project directory from active file, or args, or use CWD
        let active_path_file = crate::ipc::active_project_path_file();
        let project_dir = if let Ok(contents) = std::fs::read_to_string(&active_path_file) {
            let path = std::path::PathBuf::from(contents.trim());
            if path.exists() && path.is_dir() {
                path
            } else {
                std::env::args()
                    .nth(1)
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            }
        } else {
            std::env::args()
                .nth(1)
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        };

        // Write/sync back the active project path
        let _ = std::fs::write(&active_path_file, project_dir.to_string_lossy().to_string());

        let mut signal_bus = signal_bus;
        if signal_bus.ipc_dir().parent() != Some(&project_dir) {
            signal_bus = SignalBus::new(&project_dir);
        }
        let _ = signal_bus.ensure_dirs();

        let telemetry_engine = TelemetryEngine::new(&project_dir);
        let telemetry_data = telemetry_engine.load_latest();

        Self { 
            project_dir,
            selected_file: None,
            editor_content: String::new(),
            last_loaded_mtime: std::time::UNIX_EPOCH,
            commonmark_cache: CommonMarkCache::default(),
            theme: initial_theme,
            layout_mode: LayoutMode::Split,
            logger,
            show_diagnostics: false,
            agent_status,
            signal_bus,
            show_sidebar: true,
            log_filter: LogFilter::All,
            project_files: Vec::new(),
            last_file_scan: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(60))
                .unwrap_or(std::time::Instant::now()),
            show_telemetry_modal: false,
            telemetry_engine,
            telemetry_data,
            last_telemetry_scan: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(60))
                .unwrap_or(std::time::Instant::now()),
        }
    }

    /// Scans the project directory structure and returns a flat list of
    /// relative file paths for the file browser. Refreshed at ~1Hz.
    fn refresh_project_files(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_file_scan) < std::time::Duration::from_secs(1) {
            return;
        }
        self.last_file_scan = now;

        let mut files: Vec<String> = Vec::new();

        // Root-level important files
        for name in &["project.yaml", "concept.md"] {
            let p = self.project_dir.join(name);
            if p.exists() {
                files.push(name.to_string());
            }
        }

        // Scanned subdirectories — ordered by workflow phase
        let scan_dirs = ["plan", "prose", "reviews", "context", "exports"];
        for dir in &scan_dirs {
            let dir_path = self.project_dir.join(dir);
            if let Ok(entries) = std::fs::read_dir(&dir_path) {
                let mut dir_files: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        // Only show .md, .yaml, .json — skip hidden files
                        if name.starts_with('.') {
                            return None;
                        }
                        let ext = std::path::Path::new(&name)
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        if matches!(ext, "md" | "yaml" | "json" | "txt") {
                            Some(format!("{}/{}", dir, name))
                        } else {
                            None
                        }
                    })
                    .collect();
                dir_files.sort();
                files.extend(dir_files);
            }
        }

        self.project_files = files;
    }
}

impl eframe::App for ArchitextApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Since the filesystem IS the state machine, we can just poll it on frame update.
        // The background daemon handles the actual event debouncing and AI dispatch.
        let current_phase = crate::protocol::Phase::detect(&self.project_dir);

        // --- IPC Sidecar Polling ---
        // Check for external agent claims (e.g. from hermes or gemini-cli)
        if let Some(lock) = self.signal_bus.read_agent_lock() {
            self.agent_status.set_external(&lock.agent, &lock.task);
        } else {
            self.agent_status.clear_external();
        }

        // Tail external agent logs and interleave them into our diagnostic terminal
        let new_logs = self.signal_bus.read_agent_log_tail();
        for log_line in new_logs {
            self.logger.log(LogLevel::ExternalAgent, log_line);
        }

        // Tail daemon internal events and populate our diagnostic terminal
        let new_events = self.signal_bus.read_daemon_events_tail();
        for event in new_events {
            self.logger.push_entry(event);
        }

        // Refresh the project file list periodically
        self.refresh_project_files();

        let now = std::time::Instant::now();
        if now.duration_since(self.last_telemetry_scan) > std::time::Duration::from_secs(5) {
            self.last_telemetry_scan = now;
            if let Ok(data) = self.telemetry_engine.update_and_save(160000, 20) {
                self.telemetry_data = Some(data);
            }
        }

        // Responsive layout: auto-collapse the preview panel when the window is too
        // narrow to show both editor and preview comfortably side-by-side.
        // Left panels ~400px + editor needs ~400px + preview ~400px = ~1200px ideal.
        // Below 900px total, Split mode forces to EditorOnly for this frame.
        let window_width = ctx.screen_rect().width();
        let effective_layout = if self.layout_mode == LayoutMode::Split
            && (window_width <= 900.0 || self.selected_file.is_none())
        {
            LayoutMode::EditorOnly
        } else {
            self.layout_mode
        };

        // Live Reload Logic
        if let Some(file_path) = &self.selected_file {
            let full_path = self.project_dir.join(file_path);
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                if let Ok(mtime) = metadata.modified() {
                    if mtime > self.last_loaded_mtime {
                        if let Ok(content) = std::fs::read_to_string(&full_path) {
                            self.editor_content = content;
                            self.last_loaded_mtime = mtime;
                        }
                    }
                }
            }
        }

        let accent = self.theme.accent_color();

        // Top Panel - App Header (2-row layout)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // === Row 1: Sidebar toggle + Branding + Agent Status + Settings ===
            ui.horizontal(|ui| {

                // Sidebar collapse/expand toggle
                let sidebar_icon = if self.show_sidebar { "⊣" } else { "⊢" };
                let sidebar_tooltip = if self.show_sidebar { "Collapse sidebar" } else { "Expand sidebar" };
                if ui.button(egui::RichText::new(sidebar_icon).strong())
                    .on_hover_text(sidebar_tooltip)
                    .clicked()
                {
                    self.show_sidebar = !self.show_sidebar;
                }

                ui.separator();

                ui.heading(egui::RichText::new("ARCHITEXT").color(accent));
                ui.label("v0.1.0 — Paradroid Labs");

                ui.separator();

                // === AGENT STATUS INDICATOR ===
                let running = self.agent_status.is_running();
                if running {
                    let t = ctx.input(|i| i.time);
                    let pulse = ((t * 3.0).sin() * 0.5 + 0.5) as f32;
                    let alpha = (128.0 + 127.0 * pulse) as u8;
                    let dot_color = if self.agent_status.is_external() {
                        egui::Color32::from_rgba_unmultiplied(255, 150, 0, alpha) // Orange for external
                    } else {
                        egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha)
                    };

                    ui.label(egui::RichText::new("●").color(dot_color).size(18.0));

                    let task_name = self.agent_status.current_task_name();
                    let prefix = if let Some(agent) = self.agent_status.external_agent_name() {
                        format!("{} ACTIVE:", agent.to_uppercase())
                    } else {
                        "AGENT ACTIVE:".to_string()
                    };

                    ui.label(
                        egui::RichText::new(format!("{} {}", prefix, task_name))
                            .color(if self.agent_status.is_external() { egui::Color32::from_rgb(255, 150, 0) } else { accent })
                            .strong()
                    );
                } else {
                    ui.label(egui::RichText::new("○").color(egui::Color32::GRAY).size(18.0));
                    ui.label(egui::RichText::new("AGENT IDLE").color(egui::Color32::GRAY));
                }

                // Right-aligned controls: cancel button, diagnostics, theme
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut theme_changed = false;
                    egui::ComboBox::from_id_salt("theme_selector")
                        .selected_text(self.theme.name())
                        .show_ui(ui, |ui| {
                            for t in Theme::all() {
                                if ui.selectable_value(&mut self.theme, *t, t.name()).changed() {
                                    theme_changed = true;
                                }
                            }
                        });
                    
                    if theme_changed {
                        let mut style = (*ctx.style()).clone();
                        style.visuals = self.theme.visuals();
                        ctx.set_style(style);
                    }

                    ui.add_space(10.0);
                    if ui.selectable_label(self.show_telemetry_modal, "📊 TELEMETRY").clicked() {
                        self.show_telemetry_modal = !self.show_telemetry_modal;
                    }
                    ui.add_space(10.0);
                    if ui.selectable_label(self.show_diagnostics, "🔍 DIAGNOSTICS").clicked() {
                        self.show_diagnostics = !self.show_diagnostics;
                    }
                    ui.add_space(10.0);

                    if running {
                        if ui.button(egui::RichText::new("⏹ CANCEL").color(egui::Color32::from_rgb(255, 80, 80)).strong()).clicked() {
                            self.agent_status.request_cancel();
                            let _ = self.signal_bus.write_signal(crate::ipc::SignalKind::Cancel);
                            self.logger.log(LogLevel::Warning, "Cancel requested by user.");
                        }
                        ui.add_space(10.0);
                    }
                });
            });

            ui.separator();

            // === Row 2: File Name + Layout Controls + Force Save ===
            ui.horizontal(|ui| {
                // File name (or project path if no file selected)
                if let Some(file_path) = &self.selected_file {
                    ui.label(egui::RichText::new(format!("📄 {}", file_path)).strong());
                    
                    if file_path.ends_with(".md") {
                        let word_count = self.editor_content.split_whitespace().count();
                        ui.label(egui::RichText::new(format!("({} words)", word_count)).weak());
                    }
                } else {
                    ui.label(egui::RichText::new("No file selected").color(egui::Color32::GRAY).italics());
                }

                // Right-aligned: layout mode + force save + project path
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Switch Project
                    if ui.button(egui::RichText::new("📂 Switch Project").small()).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            let active_path_file = crate::ipc::active_project_path_file();
                            if std::fs::write(&active_path_file, path.to_string_lossy().to_string()).is_ok() {
                                self.project_dir = path;
                                self.selected_file = None;
                                self.editor_content.clear();
                                self.last_loaded_mtime = std::time::SystemTime::UNIX_EPOCH;
                                self.logger.clear();
                                // Reset tailing line counters on the signal bus
                                self.signal_bus = SignalBus::new(&self.project_dir);
                                let _ = self.signal_bus.ensure_dirs();
                                // Force immediately scanning the folder
                                self.last_file_scan = std::time::Instant::now()
                                    .checked_sub(std::time::Duration::from_secs(60))
                                    .unwrap_or(std::time::Instant::now());
                                self.refresh_project_files();
                            }
                        }
                    }

                    ui.separator();

                    // Project path (far right, truncated)
                    let path_text = self.project_dir.display().to_string();
                    ui.add(egui::Label::new(
                        egui::RichText::new(path_text).color(egui::Color32::GRAY).small()
                    ).truncate());

                    ui.separator();

                    // Force Save
                    if self.selected_file.is_some() {
                        if ui.button("Force Save").clicked() {
                            if let Some(fp) = &self.selected_file {
                                let full_path = self.project_dir.join(fp);
                                let _ = std::fs::write(&full_path, &self.editor_content);
                                if let Ok(metadata) = std::fs::metadata(&full_path) {
                                    if let Ok(mtime) = metadata.modified() {
                                        self.last_loaded_mtime = mtime;
                                    }
                                }
                            }
                        }
                    }

                    ui.separator();

                    // Layout mode segmented control: [ 📝 ] [ 📝|👁 ] [ 👁 ]
                    // Reversed order because we're laying out right-to-left.
                    let modes = [
                        (LayoutMode::PreviewOnly, "👁", "Preview Only"),
                        (LayoutMode::Split,       "📝|👁", "Split View"),
                        (LayoutMode::EditorOnly,  "📝", "Editor Only"),
                    ];
                    for (mode, icon, tooltip) in modes {
                        let is_active = self.layout_mode == mode;
                        let label = if is_active {
                            egui::RichText::new(icon).color(accent).strong()
                        } else {
                            egui::RichText::new(icon).weak()
                        };
                        if ui.selectable_label(is_active, label)
                            .on_hover_text(tooltip)
                            .clicked()
                        {
                            self.layout_mode = mode;
                        }
                    }
                });
            });
        });

        // ─── Diagnostics Terminal Panel ──────────────────────────────────────────
        // Fixed: uses min_height + fully resizable with no max_height cap.
        // This was the source of the squashing bug — max_height(250) prevented the
        // panel from growing, causing log content to get crushed at larger window sizes.
        if self.show_diagnostics {
            egui::TopBottomPanel::bottom("diagnostic_panel")
                .resizable(true)
                .min_height(120.0)
                .default_height(200.0)
                .show(ctx, |ui| {
                    self.draw_diagnostics_ui(ui);
                });
        }

        if self.show_sidebar {
            // Left Panel - Protocol Phases
            egui::SidePanel::left("left_panel")
                .resizable(true)
                .min_width(80.0)
                .max_width(250.0)
                .default_width(150.0)
                .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.add_space(5.0);
                ui.label(egui::RichText::new("PROTOCOL PHASES").color(ui.visuals().weak_text_color()).small());
                ui.separator();
                ui.add_space(5.0);
                
                let phases = [
                    (crate::protocol::Phase::Init, "0: INIT"),
                    (crate::protocol::Phase::Seed, "1: SEED"),
                    (crate::protocol::Phase::InitialPlanning, "2: INITIAL PLANNING"),
                    (crate::protocol::Phase::ChapterPlanning, "3: CHAPTER PLANNING"),
                    (crate::protocol::Phase::ChapterWriting, "4: CHAPTER WRITING"),
                    (crate::protocol::Phase::ChapterReviewing, "5: CHAPTER REVIEWING"),
                    (crate::protocol::Phase::Assembly, "6: ASSEMBLY"),
                ];

                for (phase, label) in phases.iter() {
                    let is_current = current_phase == *phase;
                    let text = if is_current {
                        egui::RichText::new(format!("▶ {}", label)).color(accent).strong()
                    } else {
                        egui::RichText::new(format!("  {}", label)).color(ui.visuals().weak_text_color())
                    };
                    ui.label(text);
                    ui.add_space(2.0);
                }
            });

            // Dashboard Panel — dynamic file browser + controls
            egui::SidePanel::left("dashboard_panel")
                .resizable(true)
                .min_width(50.0)
                .max_width(400.0)
                .default_width(250.0)
                .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

                ui.add_space(5.0);
                ui.label(egui::RichText::new("DAEMON STATUS").color(ui.visuals().weak_text_color()).small());
                ui.separator();
                ui.add_space(3.0);
                
                // Phase display — truncates naturally
                ui.add(egui::Label::new(
                    egui::RichText::new(format!("Phase: {}", current_phase.name()))
                        .color(accent).strong()
                ).truncate());
                
                ui.add_space(10.0);
                
                ui.label(egui::RichText::new("FILES").color(ui.visuals().weak_text_color()).small());
                ui.separator();
                ui.add_space(3.0);

                // ── Dynamic file browser ────────────────────────────────────────
                // Scan results are cached and refreshed at ~1Hz. Grouped by directory.
                egui::ScrollArea::vertical()
                    .id_salt("file_browser_scroll")
                    .max_height(300.0)
                    .show(ui, |ui| {
                    let files = self.project_files.clone();
                    let mut current_dir = "";

                    for file_path in &files {
                        // Determine the directory prefix for grouping headers
                        let dir = if let Some(slash) = file_path.find('/') {
                            &file_path[..slash]
                        } else {
                            "."
                        };

                        // Print a section header when the directory changes
                        if dir != current_dir {
                            if current_dir != "" {
                                ui.add_space(4.0);
                            }
                            let header = if dir == "." { "ROOT" } else { &dir.to_uppercase() };
                            ui.label(
                                egui::RichText::new(header)
                                    .color(ui.visuals().weak_text_color())
                                    .small()
                            );
                            current_dir = dir;
                        }

                        let full_path = self.project_dir.join(file_path);
                        let exists = full_path.exists();
                        let icon = if exists { "◉ " } else { "◯ " };
                        let is_selected = self.selected_file.as_deref() == Some(file_path.as_str());
                        let text_color = if exists { ui.visuals().text_color() } else { ui.visuals().weak_text_color() };
                        
                        // Just show the filename portion (not the full path) for readability
                        let display_name = file_path.rsplit('/').next().unwrap_or(file_path.as_str());
                        let label = egui::RichText::new(format!("{}{}", icon, display_name)).color(text_color);
                        if ui.add(egui::SelectableLabel::new(is_selected, label))
                            .on_hover_text(file_path.as_str())
                            .clicked()
                        {
                            if exists {
                                self.selected_file = Some(file_path.clone());
                                self.last_loaded_mtime = std::time::UNIX_EPOCH;
                            }
                        }
                    }
                });
                
                ui.add_space(12.0);
                
                ui.label(egui::RichText::new("CONTROLS").color(ui.visuals().weak_text_color()).small());
                ui.separator();
                ui.add_space(3.0);
                
                // Buttons use full available width — they shrink with the panel
                let btn_width = ui.available_width();
                
                if current_phase == crate::protocol::Phase::Init {
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Init Project")).clicked() {
                        let _ = std::fs::create_dir_all(self.project_dir.join("plan"));
                        let concept_text = "# Concept\n\nA rogue AI tries to understand human creativity by writing a novel.\n\n## Constraints\n- Genre: Sci-Fi\n- Target Word Count: 8000";
                        if let Err(e) = std::fs::write(self.project_dir.join("plan").join("01_concept.md"), concept_text) {
                            self.logger.log(LogLevel::Error, format!("Failed to write 01_concept.md: {}", e));
                        } else {
                            self.logger.log(LogLevel::Success, "01_concept.md created. Markdown-Native architecture initialized.");
                        }
                        let _ = std::fs::write(
                            self.project_dir.join("concept.md"), 
                            "A rogue AI tries to understand human creativity by writing a novel."
                        );
                    }
                } else {
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Reset to INIT")).clicked() {
                        // Only remove generated output, not config
                        for dir in &["plan", "prose", "reviews", "context"] {
                            let _ = std::fs::remove_dir_all(self.project_dir.join(dir));
                        }
                        let _ = std::fs::remove_file(self.project_dir.join("concept.md"));
                        self.selected_file = None;
                        self.logger.log(LogLevel::Warning, "Project reset to INIT phase.");
                    }
                }

                // Stacked vertically — no horizontal forcing
                if ui.add_sized([btn_width, 0.0], egui::Button::new("📥 Export")).clicked() {
                    self.export_project_state();
                }
                if ui.add_sized([btn_width, 0.0], egui::Button::new("📤 Import")).clicked() {
                    self.import_project_state();
                }
            });
        }

        // Right Panel — Resizable Markdown Preview (only in Split mode)
        // The max_width is dynamically capped so the editor always gets at least 250px.
        // Left panels consume ~300px (when sidebar visible), so: max_preview = window - 300 (left) - 250 (editor min).
        if effective_layout == LayoutMode::Split {
            let left_offset = if self.show_sidebar { 550.0 } else { 50.0 };
            let max_preview = (window_width - left_offset).clamp(200.0, 800.0);
            egui::SidePanel::right("preview_panel")
                .resizable(true)
                .min_width(200.0)
                .max_width(max_preview)
                .default_width(max_preview.min(400.0))
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("PREVIEW").color(egui::Color32::GRAY).small());
                    ui.separator();
                    self.draw_preview_pane(ui);
                });
        }

        // Central Panel — Editor/Preview content (always fills remaining space)
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_file.is_some() {
                match effective_layout {
                    LayoutMode::EditorOnly => {
                        self.draw_editor_pane(ui);
                    }
                    LayoutMode::PreviewOnly => {
                        self.draw_preview_pane(ui);
                    }
                    LayoutMode::Split => {
                        // In split mode, editor fills central panel;
                        // preview is in the right SidePanel (rendered above).
                        self.draw_editor_pane(ui);
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("Select a file from the dashboard to edit.").color(ui.visuals().weak_text_color()));
                });
            }
        });
        
        // Dashboard / Telemetry Modal Overlay
        if self.show_telemetry_modal {
            egui::Window::new("Project Telemetry Dashboard")
                .collapsible(false)
                .resizable(true)
                .default_width(800.0)
                .default_height(600.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some(data) = &self.telemetry_data {
                        ui.heading("Vespers Telemetry");
                        ui.separator();
                        
                        let total_target = data.target_words.max(1);
                        let progress = (data.total_words as f32 / total_target as f32).min(1.0);
                        
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Total Words: {}", data.total_words)).size(24.0).strong().color(accent));
                            ui.label(egui::RichText::new(format!("/ {} Target", data.target_words)).size(16.0));
                        });
                        
                        ui.add(egui::ProgressBar::new(progress).text(format!("{:.1}%", progress * 100.0)));
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("Active Continuity Flags: {}", data.active_flags)).color(egui::Color32::from_rgb(255, 100, 100)).strong());
                        });
                        
                        ui.separator();
                        
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::auto()) // Chapter Name
                            .column(Column::initial(100.0).at_least(80.0)) // Word Count
                            .column(Column::initial(100.0).at_least(80.0)) // Plan Time
                            .column(Column::initial(100.0).at_least(80.0)) // Prose Time
                            .column(Column::initial(100.0).at_least(80.0)) // Review Time
                            .column(Column::remainder()) // Status
                            .header(20.0, |mut header| {
                                header.col(|ui| { ui.strong("Chapter"); });
                                header.col(|ui| { ui.strong("Words"); });
                                header.col(|ui| { ui.strong("Plan Time"); });
                                header.col(|ui| { ui.strong("Prose Time"); });
                                header.col(|ui| { ui.strong("Review Time"); });
                                header.col(|ui| { ui.strong("Status"); });
                            })
                            .body(|mut body| {
                                for ch in &data.chapter_stats {
                                    body.row(24.0, |mut row| {
                                        row.col(|ui| { ui.label(&ch.name); });
                                        row.col(|ui| { ui.label(ch.word_count.to_string()); });
                                        row.col(|ui| { ui.label(format!("{}s", ch.plan_duration_secs)); });
                                        row.col(|ui| { ui.label(format!("{}s", ch.prose_duration_secs)); });
                                        row.col(|ui| { ui.label(format!("{}s", ch.review_duration_secs)); });
                                        row.col(|ui| { 
                                            if ch.is_revised {
                                                ui.label(egui::RichText::new("Revised").color(egui::Color32::from_rgb(100, 255, 100)));
                                            } else {
                                                ui.label(egui::RichText::new("Draft").color(egui::Color32::GRAY));
                                            }
                                        });
                                    });
                                }
                            });
                    } else {
                        ui.label("Calculating telemetry...");
                    }
                    
                    ui.add_space(20.0);
                    if ui.button("Close Dashboard").clicked() {
                        self.show_telemetry_modal = false;
                    }
                });
        }

        // Polling the filesystem at ~2Hz (every 500ms). 
        // This keeps the UI completely stateless and synced with the daemon.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

fn highlight_markdown(text: &str, colors: &SyntaxColors) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut in_code_block = false;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.code,
                ..Default::default()
            });
            continue;
        }

        if in_code_block {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.code,
                ..Default::default()
            });
            continue;
        }

        // Headings — differentiated by depth
        if trimmed.starts_with("# ") {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(16.0),
                color: colors.header,
                ..Default::default()
            });
        } else if trimmed.starts_with("## ") {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(15.0),
                color: colors.header,
                ..Default::default()
            });
        } else if trimmed.starts_with("### ") {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.header,
                italics: true,
                ..Default::default()
            });
        } else if trimmed.starts_with("> ") {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.quote,
                italics: true,
                ..Default::default()
            });
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            // List items get a slightly muted color to distinguish structure from prose
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.italic,
                ..Default::default()
            });
        } else if trimmed.starts_with("**") || trimmed.starts_with("__") {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.bold,
                ..Default::default()
            });
        } else {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: colors.text,
                ..Default::default()
            });
        }
    }
    job
}

impl ArchitextApp {
    /// Renders the raw text editor pane (used in Editor and Split modes).
    fn draw_editor_pane(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("editor_scroll").show(ui, |ui| {
            let mut content = self.editor_content.clone();
            let theme_colors = self.theme.syntax_colors();

            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job = highlight_markdown(string, &theme_colors);
                layout_job.wrap.max_width = wrap_width;
                ui.fonts(|f| f.layout_job(layout_job))
            };

            let response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut content)
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .lock_focus(true)
                    .layouter(&mut layouter)
            );

            if response.changed() {
                self.editor_content = content;
                // Auto-save to disk
                if let Some(file_path) = &self.selected_file {
                    let full_path = self.project_dir.join(file_path);
                    let _ = std::fs::write(&full_path, &self.editor_content);
                    if let Ok(metadata) = std::fs::metadata(&full_path) {
                        if let Ok(mtime) = metadata.modified() {
                            self.last_loaded_mtime = mtime;
                        }
                    }
                }
            }
        });
    }

    /// Renders the markdown preview pane (used in Preview and Split modes).
    fn draw_preview_pane(&mut self, ui: &mut egui::Ui) {
        // Force standard Light/Dark visuals for readability
        let mut style = (**ui.style()).clone();
        if self.theme.is_light() {
            style.visuals = egui::Visuals::light();
        } else {
            style.visuals = egui::Visuals::dark();
        }
        ui.set_style(style);

        egui::ScrollArea::vertical().id_salt("preview_scroll").show(ui, |ui| {
            if let Some(file_path) = &self.selected_file {
                if file_path.ends_with(".md") {
                    egui_commonmark::CommonMarkViewer::new()
                        .show(ui, &mut self.commonmark_cache, &self.editor_content);
                } else {
                    // Non-markdown files (like yaml) should render as code blocks
                    let mut content = self.editor_content.clone();
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut content)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false)
                    );
                }
            }
        });
    }

    /// Renders the diagnostics terminal with log filtering controls.
    fn draw_diagnostics_ui(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.accent_color();

        // ── Header row: title + filter controls + clear button ──────────────────
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("TERMINAL").small().color(egui::Color32::GRAY));

            ui.separator();

            // Log filter segmented control
            let filters = [
                (LogFilter::All,          "ALL"),
                (LogFilter::InternalOnly, "INTERNAL"),
                (LogFilter::ExternalOnly, "EXTERNAL"),
                (LogFilter::ErrorsOnly,   "ERRORS"),
            ];
            for (filter, label) in &filters {
                let is_active = self.log_filter == *filter;
                let text = if is_active {
                    egui::RichText::new(*label).color(accent).strong().small()
                } else {
                    egui::RichText::new(*label).color(egui::Color32::GRAY).small()
                };
                if ui.selectable_label(is_active, text)
                    .on_hover_text(match filter {
                        LogFilter::All          => "Show all log streams",
                        LogFilter::InternalOnly => "Show only internal agent logs",
                        LogFilter::ExternalOnly => "Show only external CLI agent logs",
                        LogFilter::ErrorsOnly   => "Show only warnings and errors",
                    })
                    .clicked()
                {
                    self.log_filter = *filter;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Clear").on_hover_text("Clear the log buffer").clicked() {
                    self.logger.clear();
                }
            });
        });

        ui.separator();

        // ── Log entries scroll area ───────────────────────────────────────────
        egui::ScrollArea::vertical()
            .id_salt("log_scroll")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let entries = self.logger.get_filtered_entries(self.log_filter);
                
                if entries.is_empty() {
                    ui.label(
                        egui::RichText::new("No log entries for this filter.")
                            .color(egui::Color32::DARK_GRAY)
                            .italics()
                            .small()
                    );
                    return;
                }

                for entry in &entries {
                    ui.horizontal_top(|ui| {
                        let time_str = entry.timestamp.format("%H:%M:%S").to_string();
                        ui.label(egui::RichText::new(time_str).color(egui::Color32::GRAY).monospace().small());
                        
                        let (level_tag, color) = match entry.level {
                            LogLevel::Info          => ("INFO ", egui::Color32::from_rgb(100, 150, 255)),
                            LogLevel::Success       => ("OK   ", egui::Color32::from_rgb(0, 220, 90)),
                            LogLevel::Warning       => ("WARN ", egui::Color32::from_rgb(255, 200, 0)),
                            LogLevel::Error         => ("ERR  ", egui::Color32::from_rgb(255, 60, 60)),
                            LogLevel::AgentCall     => ("AGENT", egui::Color32::from_rgb(180, 100, 255)),
                            LogLevel::ExternalAgent => ("EXT  ", egui::Color32::from_rgb(255, 150, 0)),
                        };

                        ui.label(egui::RichText::new(level_tag).color(color).monospace().small().strong());
                        ui.label(egui::RichText::new(&entry.message).color(color).monospace().small());
                    });
                }
            });
    }

    fn export_project_state(&self) {
        let export_path = rfd::FileDialog::new()
            .set_title("Export Project Snapshot")
            .add_filter("JSON", &["json"])
            .set_file_name(&format!("snapshot_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S")))
            .save_file();

        let Some(target_path) = export_path else {
            return; // Cancelled
        };

        let mut files = Vec::new();
        let dirs = ["plan", "prose", "reviews", "context", "."];
        
        for dir in dirs {
            let path = self.project_dir.join(dir);
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        let file_path = entry.path();
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            let rel_path = file_path.strip_prefix(&self.project_dir)
                                .unwrap_or(&file_path)
                                .to_string_lossy()
                                .into_owned();
                            files.push(FileContent { path: rel_path, content });
                        }
                    }
                }
            }
        }

        let config_str = std::fs::read_to_string(self.project_dir.join("project.yaml"))
            .unwrap_or_default();

        let snapshot = ProjectSnapshot {
            title: "Architext Export".to_string(),
            timestamp: chrono::Utc::now(),
            config: config_str,
            files,
            logs: self.logger.get_entries(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
            if let Err(e) = std::fs::write(&target_path, json) {
                self.logger.log(LogLevel::Error, format!("Export failed: {}", e));
            } else {
                self.logger.log(LogLevel::Success, format!("Project snapshot exported to {}", target_path.display()));
            }
        }
    }

    fn import_project_state(&mut self) {
        let import_path = rfd::FileDialog::new()
            .set_title("Import Project Snapshot")
            .add_filter("JSON", &["json"])
            .pick_file();

        let Some(target_path) = import_path else {
            return; // Cancelled
        };

        if let Ok(json) = std::fs::read_to_string(&target_path) {
            if let Ok(snapshot) = serde_json::from_str::<ProjectSnapshot>(&json) {
                // Reconstruct files
                for file in snapshot.files {
                    let full_path = self.project_dir.join(&file.path);
                    if let Some(parent) = full_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(full_path, file.content);
                }
                // Write config
                let _ = std::fs::write(self.project_dir.join("project.yaml"), snapshot.config);
                
                self.logger.log(LogLevel::Success, format!("Project state IMPORTED from {}.", target_path.display()));
                self.selected_file = None; // Reset UI
            } else {
                self.logger.log(LogLevel::Error, "Import failed: Invalid JSON schema.");
            }
        } else {
            self.logger.log(LogLevel::Error, "Import failed: Could not read file.");
        }
    }
}
