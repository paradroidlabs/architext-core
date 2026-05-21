use std::path::PathBuf;
use anyhow::Result;
use tracing::{debug, info};

use crate::prompts::{
    self, ChapterContext, OutputPhase, ReviewContext, DEFAULT_WORD_COUNT
};

/// The Context Assembler — reads filesystem slices to build agent prompts.
pub struct ContextAssembler {
    project_dir: PathBuf,
}

/// A context bundle assembled for a specific agent task.
#[derive(Debug, Clone)]
pub struct ContextBundle {
    pub system_prompt: String,
    pub user_prompt: String,
    pub source_files: Vec<PathBuf>,
    pub estimated_tokens: usize,
}

/// What kind of context assembly is needed.
#[derive(Debug, Clone)]
pub enum ContextTask {
    PlanChapter { chapter: usize },
    GenerateProse { chapter: usize },
    ReviewChapter { chapter: usize },
    Assemble,
}

impl ContextAssembler {
    pub fn new(project_dir: PathBuf) -> Self {
        Self { project_dir }
    }

    pub fn assemble(&self, task: &ContextTask) -> Result<ContextBundle> {
        match task {
            ContextTask::PlanChapter { chapter } => self.assemble_chapter_plan(*chapter),
            ContextTask::GenerateProse { chapter } => self.assemble_prose(*chapter),
            ContextTask::ReviewChapter { chapter } => self.assemble_review(*chapter),
            ContextTask::Assemble => self.assemble_final(),
        }
    }

    fn read_file(&self, rel: &str) -> (String, Option<PathBuf>) {
        let full = self.project_dir.join(rel);
        match std::fs::read_to_string(&full) {
            Ok(c) => { debug!("Read: {} ({} chars)", rel, c.len()); (c, Some(full)) }
            Err(_) => { debug!("Not found: {}", rel); (String::new(), None) }
        }
    }

    fn bundle(&self, sys: String, user: String, sources: Vec<PathBuf>) -> ContextBundle {
        let est = (sys.len() + user.len()) / 4;
        info!("[CONTEXT] {} files, ~{} tokens", sources.len(), est);
        ContextBundle { system_prompt: sys, user_prompt: user, source_files: sources, estimated_tokens: est }
    }

    /// Helper to inject standard 5-doc workflow
    fn inject_markdown_docs(&self, s: &mut Vec<PathBuf>) {
        let docs = [
            "plan/01_concept.md",
            "plan/02_universe.md",
            "plan/03_plot.md",
            "plan/04_blueprint.md",
            "plan/05_cowork.md",
        ];
        for d in docs {
            let (_, p) = self.read_file(d);
            if let Some(p) = p { s.push(p); }
        }
    }

    fn find_agent_file(&self, agent_name: &str) -> Option<PathBuf> {
        let mut current = self.project_dir.canonicalize().unwrap_or_else(|_| self.project_dir.clone());
        loop {
            let plugin_path = current
                .join(".agents")
                .join("plugins")
                .join(agent_name)
                .join("agents")
                .join(format!("{}.md", agent_name));
            if plugin_path.exists() {
                return Some(plugin_path);
            }

            let root_path = current
                .join(".agents")
                .join(format!("{}.md", agent_name));
            if root_path.exists() {
                return Some(root_path);
            }

            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        None
    }

    pub fn load_custom_agent_prompt(&self, agent_name: &str) -> Option<String> {
        // Try to load from the filesystem first
        if let Some(path) = self.find_agent_file(agent_name) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.starts_with("---") {
                    let lines: Vec<&str> = content.lines().collect();
                    if lines.len() > 1 {
                        let mut second_dash_idx = None;
                        for (i, line) in lines.iter().enumerate().skip(1) {
                            if line.trim() == "---" {
                                second_dash_idx = Some(i);
                                break;
                            }
                        }
                        if let Some(idx) = second_dash_idx {
                            let remaining_lines = &lines[idx + 1..];
                            return Some(remaining_lines.join("\n").trim().to_string());
                        }
                    }
                }
                return Some(content.trim().to_string());
            }
        }

        // Native fallback: compiled-in agent prompts
        match agent_name {
            "jonathan" => {
                info!("Using compiled-in fallback prompt for agent '{}'", agent_name);
                Some(prompts::JONATHAN_SYSTEM_PROMPT.to_string())
            }
            _ => None,
        }
    }

    fn assemble_chapter_plan(&self, ch: usize) -> Result<ContextBundle> {
        let mut s = vec![];
        
        self.inject_markdown_docs(&mut s);
        
        let (plot, _) = self.read_file("plan/03_plot.md");
        let (blueprint, _) = self.read_file("plan/04_blueprint.md");
        let overall_plot_outline = format!("{}\n\n{}", plot, blueprint);

        let (cowork, _) = self.read_file("plan/05_cowork.md");

        let prev_review = if ch > 1 {
            let (r, p) = self.read_file(&format!("reviews/ch{:02}_review.md", ch - 1));
            if let Some(p) = p { s.push(p); }
            Some(r).filter(|x| !x.is_empty())
        } else {
            None
        };

        let ctx = ChapterContext {
            overall_plot_outline: &overall_plot_outline,
            evolving_story_context_log: &cowork,
            previous_chapter_review_analysis: prev_review.as_deref(),
        };

        let mut sys = format!(
            "{}\n{}",
            prompts::system_prompt_goal(DEFAULT_WORD_COUNT),
            prompts::novel_structure_and_style(DEFAULT_WORD_COUNT)
        );
        
        if let Some(custom_prompt) = self.load_custom_agent_prompt("jonathan") {
            sys = format!("{}\n\n{}", custom_prompt, sys);
        }

        let user = format!(
            "{}\n\n{}",
            prompts::planning_rules_chapter_n(ch, DEFAULT_WORD_COUNT, &ctx),
            prompts::output_instructions(OutputPhase::PlanCurrentChapter, DEFAULT_WORD_COUNT, Some(ch), false)
        );

        Ok(self.bundle(sys, user, s))
    }

    fn assemble_prose(&self, ch: usize) -> Result<ContextBundle> {
        let mut s = vec![];
        
        self.inject_markdown_docs(&mut s);

        let plan_file = format!("plan/ch{:02}_plan.md", ch);
        let (plan, p) = self.read_file(&plan_file);
        if let Some(p) = p { s.push(p); }

        if plan.is_empty() {
            anyhow::bail!("Cannot generate prose for Chapter {}: {} is missing or empty.", ch, plan_file);
        }

        let mut sys = format!(
            "{}\n{}",
            prompts::system_prompt_goal(DEFAULT_WORD_COUNT),
            prompts::novel_structure_and_style(DEFAULT_WORD_COUNT)
        );
        
        if let Some(custom_prompt) = self.load_custom_agent_prompt("jonathan") {
            sys = format!("{}\n\n{}", custom_prompt, sys);
        }

        let user = format!(
            "Execute Phase 5: Prose Generation.\n\n### Chapter {} Hyper-Detailed Plan\n{}\n\n{}",
            ch,
            plan,
            prompts::output_instructions(OutputPhase::GenerateChapter, DEFAULT_WORD_COUNT, Some(ch), false)
        );

        Ok(self.bundle(sys, user, s))
    }

    fn assemble_review(&self, ch: usize) -> Result<ContextBundle> {
        let mut s = vec![];
        
        self.inject_markdown_docs(&mut s);

        let prose_file = format!("prose/ch{:02}_prose.md", ch);
        let plan_file = format!("plan/ch{:02}_plan.md", ch);
        
        let (prose, p) = self.read_file(&prose_file);
        if let Some(p) = p { s.push(p); }

        let (plan, p) = self.read_file(&plan_file);
        if let Some(p) = p { s.push(p); }

        if prose.is_empty() {
            anyhow::bail!("Cannot review Chapter {}: {} is missing.", ch, prose_file);
        }

        let ctx = ReviewContext {
            generated_prose: &prose,
            chapter_plan: &plan,
            is_final_chapter: false, // For now
        };

        let mut sys = prompts::system_prompt_goal(DEFAULT_WORD_COUNT);
        if let Some(custom_prompt) = self.load_custom_agent_prompt("jonathan") {
            sys = format!("{}\n\n{}", custom_prompt, sys);
        }

        let user = format!(
            "{}\n\n### Chapter {} Plan\n{}\n\n### Chapter {} Generated Prose\n{}\n\n{}",
            prompts::chapter_review_analysis(ch, DEFAULT_WORD_COUNT, &ctx),
            ch, plan, ch, prose,
            prompts::output_instructions(OutputPhase::ReviewChapterOnly, DEFAULT_WORD_COUNT, Some(ch), false)
        );

        Ok(self.bundle(sys, user, s))
    }

    fn assemble_final(&self) -> Result<ContextBundle> {
        let prose_dir = self.project_dir.join("prose");
        let mut chapters = vec![];
        let mut sources = vec![];
        if prose_dir.exists() {
            let mut entries: Vec<_> = std::fs::read_dir(&prose_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("ch") && n.ends_with("_prose.md")))
                .collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                chapters.push(std::fs::read_to_string(e.path())?);
                sources.push(e.path());
            }
        }
        let sys = "Assemble chapters into a final manuscript with proper formatting.".to_string();
        let user = format!("Assemble {} chapters:\n\n{}", chapters.len(), chapters.join("\n\n---\n\n"));
        Ok(self.bundle(sys, user, sources))
    }
}
