//! Agent Definitions — Stateless Functions Operating on Filesystem Slices
//!
//! Each agent is a stateless function that:
//! 1. Receives a context bundle (assembled from disk)
//! 2. Calls the Gemini API
//! 3. Writes the result back to a specific file on disk
//! 4. Returns — no state held

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

use crate::gemini::{GeminiClient, GenerateParams, ModelTier};
use crate::context::{ContextAssembler, ContextTask};

/// The agent pool — dispatches tasks to the appropriate agent.
pub struct AgentPool {
    gemini: Arc<GeminiClient>,
    context: ContextAssembler,
    project_dir: PathBuf,
}

/// Result of an agent execution.
#[derive(Debug)]
pub struct AgentResult {
    pub output_path: PathBuf,
    pub tokens_used: usize,
    pub success: bool,
}

impl AgentPool {
    pub fn new(gemini: Arc<GeminiClient>, project_dir: PathBuf) -> Self {
        let context = ContextAssembler::new(project_dir.clone());
        Self { gemini, context, project_dir }
    }



    /// Run the chapter plan agent for a specific chapter.
    pub async fn run_chapter_plan_agent(&self, chapter: usize) -> Result<AgentResult> {
        self.run_agent(
            ContextTask::PlanChapter { chapter },
            ModelTier::Pro,
            0.8,
            8192,
            &format!("plan/ch{:02}_plan.md", chapter),
        ).await
    }

    /// Run the prose generation agent for a specific chapter.
    pub async fn run_prose_agent(&self, chapter: usize) -> Result<AgentResult> {
        self.run_agent(
            ContextTask::GenerateProse { chapter },
            ModelTier::Pro,
            1.1,
            32768,
            &format!("prose/ch{:02}_prose.md", chapter),
        ).await
    }

    /// Run the review agent for a specific chapter.
    pub async fn run_review_agent(&self, chapter: usize) -> Result<AgentResult> {
        self.run_agent(
            ContextTask::ReviewChapter { chapter },
            ModelTier::Flash,
            0.3,
            4096,
            &format!("reviews/ch{:02}_review.md", chapter),
        ).await
    }

    /// Generic agent execution: assemble context → call API → write to disk.
    async fn run_agent(
        &self,
        task: ContextTask,
        tier: ModelTier,
        temperature: f32,
        max_tokens: u32,
        output_rel_path: &str,
    ) -> Result<AgentResult> {
        let output_path = self.project_dir.join(output_rel_path);

        info!("[AGENT] Starting {:?} → {}", task, output_rel_path);

        // 1. Assemble context from filesystem
        let bundle = self.context.assemble(&task)
            .context("Failed to assemble context")?;

        info!("[AGENT] Context assembled: {} files, ~{} tokens",
            bundle.source_files.len(), bundle.estimated_tokens);

        // 2. Call Gemini API
        let params = GenerateParams {
            system_prompt: bundle.system_prompt,
            user_prompt: bundle.user_prompt,
            contents: None,
            tier,
            temperature,
            max_tokens,
        };

        let output = self.gemini.generate(params).await
            .context("Gemini API call failed")?;

        // 3. Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // 4. Write result to disk
        std::fs::write(&output_path, &output)
            .with_context(|| format!("Failed to write output: {}", output_path.display()))?;

        info!("[AGENT] Output written: {} ({} chars)", output_rel_path, output.len());

        Ok(AgentResult {
            output_path,
            tokens_used: bundle.estimated_tokens + (output.len() / 4),
            success: true,
        })
    }
}
