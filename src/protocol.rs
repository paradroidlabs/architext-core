//! Protocol Engine — The 8-Phase Narrative AI State Machine
//!
//! Maps filesystem events to protocol phase transitions.
//! The filesystem IS the state machine — phase state is determined
//! entirely by what files exist on disk.
//! 
//! NOW WIRED: Dispatches to AgentPool for live Gemini execution.
//! IPC-AWARE: Writes agent.lock on dispatch, checks approve signals.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::{info, debug, warn, error};

use crate::agents::AgentPool;

use crate::gemini::GeminiClient;
use crate::ipc::SignalBus;
use crate::watcher::{FsEvent, FsEventKind};
use crate::logger::{EventLogger, LogLevel, AgentStatus};

/// The 7 phases of the Narrative AI Creative Writer Protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Phase 0: Waiting for project initialization (project.yaml)
    Init,
    /// Phase 1: Seed idea exists
    Seed,
    /// Phase 2: Initial setup (Concept, Characters, Plot) complete
    InitialPlanning,
    /// Phase 3: Chapter-by-chapter blueprint
    ChapterPlanning,
    /// Phase 4: Prose generation (scene-by-scene)
    ChapterWriting,
    /// Phase 5: Review & continuity check
    ChapterReviewing,
    /// Phase 6: Final assembly & export
    Assembly,
}

impl Phase {
    /// Determine the current phase by inspecting the filesystem.
    pub fn detect(project_dir: &Path) -> Self {
        // Work backwards from the most advanced phase
        if project_dir.join("output").join("final.md").exists() {
            return Phase::Assembly;
        }
        if has_files_with_prefix(project_dir, "reviews", "ch") {
            return Phase::ChapterReviewing;
        }
        if has_files_with_prefix(project_dir, "prose", "ch") {
            return Phase::ChapterWriting;
        }
        if has_files_with_prefix(project_dir, "plan", "ch") {
            return Phase::ChapterPlanning;
        }
        if project_dir.join("plan").join("00_initial_setup.md").exists() {
            return Phase::InitialPlanning;
        }
        if project_dir.join("plan").join("01_concept.md").exists() {
            return Phase::Seed;
        }

        Phase::Init
    }

    /// Get the human-readable name for this phase.
    pub fn name(&self) -> &'static str {
        match self {
            Phase::Init => "INIT",
            Phase::Seed => "SEED",
            Phase::InitialPlanning => "INITIAL_PLANNING",
            Phase::ChapterPlanning => "CHAPTER_PLANNING",
            Phase::ChapterWriting => "CHAPTER_WRITING",
            Phase::ChapterReviewing => "CHAPTER_REVIEWING",
            Phase::Assembly => "ASSEMBLY",
        }
    }

    /// Get the next phase in the protocol sequence.
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Init => Some(Phase::Seed),
            Phase::Seed => Some(Phase::InitialPlanning),
            Phase::InitialPlanning => Some(Phase::ChapterPlanning),
            Phase::ChapterPlanning => Some(Phase::ChapterWriting),
            Phase::ChapterWriting => Some(Phase::ChapterReviewing),
            Phase::ChapterReviewing => Some(Phase::Assembly),
            Phase::Assembly => None,
        }
    }
}

/// The Protocol Engine — orchestrates agent execution based on filesystem state.
pub struct ProtocolEngine {
    /// The project root directory
    project_dir: PathBuf,

    /// The agent pool for dispatching Gemini-backed tasks
    agents: AgentPool,

    /// Whether Gemini is configured (has API key)
    gemini_live: bool,

    /// Revision mode from project config
    revision_mode: String,

    /// Structured event logger for the UI dashboard
    logger: EventLogger,

    /// Shared agent lifecycle state (read by UI, written here)
    agent_status: AgentStatus,

    /// IPC signal bus for external agent coordination
    signal_bus: SignalBus,

    /// Cooldown after failed dispatch — prevents rapid-fire retries
    last_dispatch_failure: std::sync::Mutex<Option<std::time::Instant>>,
}

impl ProtocolEngine {
    pub fn new(project_dir: PathBuf, gemini: GeminiClient, logger: EventLogger, agent_status: AgentStatus, signal_bus: SignalBus) -> Self {
        let gemini_live = gemini.is_configured();
        let gemini = Arc::new(gemini);
        let agents = AgentPool::new(gemini, project_dir.clone());

        let revision_mode = "human".to_string();


        Self {
            project_dir,
            agents,
            gemini_live,
            revision_mode,
            logger,
            agent_status,
            signal_bus,
            last_dispatch_failure: std::sync::Mutex::new(None),
        }
    }

    /// Handle a filesystem event by evaluating the protocol state and dispatching.
    /// This is now async to support agent execution.
    pub async fn handle_event(&self, event: FsEvent) {
        let relative = event.path
            .strip_prefix(&self.project_dir)
            .unwrap_or(&event.path);

        info!(
            "[EVENT] {:?} -> {}",
            event.kind,
            relative.display()
        );

        self.logger.log(
            LogLevel::Info, 
            format!("[FS] {:?}: {}", event.kind, relative.display())
        );

        let filename = event.path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");

        let parent_dir = event.path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|f| f.to_str())
            .unwrap_or("");

        match event.kind {
            FsEventKind::Created | FsEventKind::Modified => {
                self.dispatch_on_change(filename, parent_dir).await;
            }
            FsEventKind::Removed => {
                debug!("File removed: {} — no action taken", relative.display());
            }
        }
    }

    /// Core dispatch logic — maps file changes to protocol actions.
    async fn dispatch_on_change(&self, filename: &str, parent_dir: &str) {
        match (filename, parent_dir) {
            // Concept seed was dropped into plan — trigger concept expansion
            ("01_concept.md", "plan") => {
                info!("[PROTOCOL] 01_concept.md detected — evaluating project state");
                self.evaluate_and_advance().await;
            }

            // Planning artifacts updated
            (f, "plan") if f.ends_with(".md") => {
                info!("[PROTOCOL] Plan artifact updated: {}", f);
                self.evaluate_and_advance().await;
            }

            // Prose artifacts
            (f, "prose") if f.ends_with(".md") => {
                info!("[PROTOCOL] Prose artifact updated: {}", f);
                self.evaluate_and_advance().await;
            }

            // Review artifacts
            (f, "reviews") if f.ends_with(".md") => {
                info!("[PROTOCOL] Review artifact updated: {}", f);
                self.evaluate_and_advance().await;
            }

            // Context log updated — no phase transition needed
            ("context_log.md", "context") => {
                debug!("[PROTOCOL] Context log updated — no phase transition");
            }

            // Approval signal files
            (f, _) if f.ends_with(".approved") => {
                info!("[PROTOCOL] Approval signal received: {}", f);
                self.evaluate_and_advance().await;
            }

            _ => {
                debug!("Unhandled file change: {}/{}", parent_dir, filename);
            }
        }
    }

    /// Evaluate the current protocol phase and dispatch the next agent if possible.
    async fn evaluate_and_advance(&self) {
        let current = Phase::detect(&self.project_dir);
        info!("[PHASE] Current phase: {} ({})", current.name(), Self::phase_description(current));

        // Determine what the NEXT phase should be and whether we should auto-dispatch
        match current {
            Phase::Init => {
                // project.yaml exists but no concept yet. Check for concept.md seed.
                if self.project_dir.join("concept.md").exists() {
                    self.log_phase_transition(Phase::Init, Phase::Seed);
                    // Next evaluate pass or manual trigger will move from Seed to InitialPlanning
                } else {
                    info!("[PHASE] Waiting for concept.md seed to be dropped...");
                }
            }

            Phase::Seed => {
                // Concept seed exists. Trigger initial setup.
                if !self.project_dir.join("plan").join("00_initial_setup.md").exists() {
                    self.log_phase_transition(Phase::Seed, Phase::InitialPlanning);
                    self.dispatch_agent("initial_setup").await;
                }
            }

            Phase::InitialPlanning => {
                if !self.project_dir.join("plan").join("ch01_plan.md").exists() {
                    self.log_phase_transition(Phase::InitialPlanning, Phase::ChapterPlanning);
                    self.dispatch_agent("chapter_plan_ch01").await;
                }
            }

            Phase::ChapterPlanning => {
                // Blueprint ready — start prose generation for chapter 1
                // Only auto-dispatch if revision_mode is "auto"
                if self.revision_mode == "auto" {
                    if !self.project_dir.join("prose").join("ch01_prose.md").exists() {
                        self.log_phase_transition(Phase::ChapterPlanning, Phase::ChapterWriting);
                        self.dispatch_agent("prose_ch01").await;
                    }
                } else {
                    info!("[PHASE] Chapter plan complete. Revision mode is 'human'.");
                    info!("[PHASE] Drop a 'prose.approved' file to begin prose generation.");
                }
            }

            Phase::ChapterWriting => {
                info!("[PHASE] Prose generation in progress. Checking for review...");
                // Check if Chapter 1 prose is finished but not reviewed
                if self.project_dir.join("prose").join("ch01_prose.md").exists() 
                   && !self.project_dir.join("reviews").join("ch01_review.md").exists() {
                    self.log_phase_transition(Phase::ChapterWriting, Phase::ChapterReviewing);
                    self.dispatch_agent("review_ch01").await;
                }
            }

            Phase::ChapterReviewing => {
                info!("[PHASE] Review phase active.");
            }

            Phase::Assembly => {
                info!("[PHASE] Protocol complete — novel assembled.");
            }
        }
    }

    /// Dispatch an agent task. This is the bridge between protocol logic and Gemini execution.
    /// Now IPC-aware: writes agent.lock for external visibility, logs to agent.log,
    /// and checks for IPC cancel signals.
    async fn dispatch_agent(&self, task_name: &str) {
        if !self.gemini_live {
            warn!("[AGENT] Gemini API key not configured — skipping agent dispatch for '{}'", task_name);
            warn!("[AGENT] Set GEMINI_API_KEY in .env to enable autonomous execution.");
            return;
        }

        // Check if cancellation was requested (internal OR IPC signal)
        if self.agent_status.is_cancel_requested() || self.signal_bus.check_signal(crate::ipc::SignalKind::Cancel) {
            self.logger.log(LogLevel::Warning, format!("Agent '{}' skipped — cancel was requested.", task_name));
            self.signal_bus.clear_signal(crate::ipc::SignalKind::Cancel).ok();
            return;
        }

        // Signal the UI that an agent is now active
        self.agent_status.set_running(task_name);

        // Write IPC agent.lock so external tools can see we're busy
        if let Err(e) = self.signal_bus.write_agent_lock("architext-engine", task_name) {
            warn!("[IPC] Failed to write agent.lock: {}", e);
        }

        // Log to IPC agent.log for external observers
        self.signal_bus.append_agent_log(&format!("DISPATCH: {}", task_name)).ok();

        // Check cooldown — don't re-dispatch within 30s of a failure
        {
            let guard = self.last_dispatch_failure.lock().unwrap();
            if let Some(last_fail) = *guard {
                if last_fail.elapsed() < std::time::Duration::from_secs(30) {
                    self.logger.log(LogLevel::Warning, format!("Agent '{}' skipped — cooldown after previous failure ({}s remaining).", task_name, 30 - last_fail.elapsed().as_secs()));
                    self.agent_status.set_idle();
                    self.signal_bus.clear_agent_lock().ok();
                    return;
                }
            }
        }

        self.logger.log(LogLevel::AgentCall, format!("Dispatching agent: {}", task_name));
        info!("[DISPATCH] Launching agent: {}", task_name);

        let result = match task_name {

            "chapter_plan_ch01" => self.agents.run_chapter_plan_agent(1).await,
            "prose_ch01" => self.agents.run_prose_agent(1).await,
            "review_ch01" => self.agents.run_review_agent(1).await,
            _ => {
                error!("[DISPATCH] Unknown agent task: {}", task_name);
                self.agent_status.set_idle();
                self.signal_bus.clear_agent_lock().ok();
                return;
            }
        };

        // Check if cancel was requested during execution (internal OR IPC)
        if self.agent_status.is_cancel_requested() || self.signal_bus.check_signal(crate::ipc::SignalKind::Cancel) {
            self.logger.log(LogLevel::Warning, format!("Agent '{}' completed but cancel was requested — result may be discarded.", task_name));
            self.signal_bus.clear_signal(crate::ipc::SignalKind::Cancel).ok();
        }

        match result {
            Ok(agent_result) => {
                info!("╔═══════════════════════════════════════════╗");
                info!("║  AGENT COMPLETE                           ║");
                info!("║  Task: {}", task_name);
                info!("║  Output: {}", agent_result.output_path.display());
                info!("║  Tokens: ~{}", agent_result.tokens_used);
                info!("╚═══════════════════════════════════════════╝");

                self.logger.log(
                    LogLevel::Success,
                    format!("Agent '{}' completed. Output: {} (~{} tokens)", task_name, agent_result.output_path.display(), agent_result.tokens_used)
                );

                self.signal_bus.append_agent_log(&format!("COMPLETE: {} → {} (~{} tokens)", task_name, agent_result.output_path.display(), agent_result.tokens_used)).ok();
            }
            Err(e) => {
                error!("╔═══════════════════════════════════════════╗");
                error!("║  AGENT FAILED                             ║");
                error!("║  Task: {}", task_name);
                error!("║  Error: {}", e);
                for cause in e.chain().skip(1) {
                    error!("║  Caused by: {}", cause);
                }
                error!("╚═══════════════════════════════════════════╝");

                self.logger.log(
                    LogLevel::Error,
                    format!("Agent '{}' failed: {:#}", task_name, e)
                );

                self.signal_bus.append_agent_log(&format!("FAILED: {} — {}", task_name, e)).ok();

                // Set cooldown
                *self.last_dispatch_failure.lock().unwrap() = Some(std::time::Instant::now());
            }
        }

        // Always signal idle and clear IPC lock when done
        self.agent_status.set_idle();
        self.signal_bus.clear_agent_lock().ok();
    }

    fn phase_description(phase: Phase) -> &'static str {
        match phase {
            Phase::Init => "Waiting for concept seed",
            Phase::Seed => "Concept seed received, awaiting initial setup",
            Phase::InitialPlanning => "Initial setup complete, awaiting chapter plan",
            Phase::ChapterPlanning => "Chapter plan ready, awaiting prose generation",
            Phase::ChapterWriting => "Prose in progress, awaiting review",
            Phase::ChapterReviewing => "Review complete, awaiting assembly",
            Phase::Assembly => "Final manuscript assembled",
        }
    }

    fn log_phase_transition(&self, from: Phase, to: Phase) {
        info!("╔═══════════════════════════════════════════╗");
        info!("║  PHASE TRANSITION                         ║");
        info!("║  {} → {}",
            from.name(),
            to.name()
        );
        info!("╚═══════════════════════════════════════════╝");

        self.logger.log(
            LogLevel::Info,
            format!("[PROTOCOL] Phase Transition: {} -> {}", from.name(), to.name())
        );
    }
}

/// Helper: check if a directory contains files with a given prefix
fn has_files_with_prefix(root: &Path, subdir: &str, prefix: &str) -> bool {
    let dir = root.join(subdir);
    if !dir.exists() {
        return false;
    }
    match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .any(|e| {
                e.file_name()
                    .to_str()
                    .map(|name| name.starts_with(prefix))
                    .unwrap_or(false)
            }),
        Err(_) => false,
    }
}
