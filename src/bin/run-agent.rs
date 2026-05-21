use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};
use tracing::{info, error};

use architext_core::gemini::GeminiClient;
use architext_core::agents::AgentPool;

/// Architext Run-Agent — Manually dispatch an agent task, bypassing the daemon
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the Architext project root
    #[arg(short, long, default_value = ".")]
    project: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate the initial setup (Concept, Characters, Plot)
    Setup,
    
    /// Generate a chapter plan
    Plan {
        /// The chapter number
        chapter: usize,
    },
    
    /// Generate prose for a chapter
    Prose {
        /// The chapter number
        chapter: usize,
    },
    
    /// Review a chapter's prose
    Review {
        /// The chapter number
        chapter: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    
    let gemini_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set in .env");
    let gemini_client = Arc::new(GeminiClient::new(gemini_key));
    let agents = AgentPool::new(gemini_client, cli.project.clone());

    info!("Starting manual agent dispatch for project: {}", cli.project.display());

    let result = match cli.command {
        Commands::Setup => {
            anyhow::bail!("Setup is now handled manually via Markdown-Native architecture (01_concept.md, etc.)");
        }
        Commands::Plan { chapter } => {
            agents.run_chapter_plan_agent(chapter).await
        }
        Commands::Prose { chapter } => {
            agents.run_prose_agent(chapter).await
        }
        Commands::Review { chapter } => {
            agents.run_review_agent(chapter).await
        }
    };

    match result {
        Ok(agent_result) => {
            info!("╔═══════════════════════════════════════════╗");
            info!("║  AGENT COMPLETE                           ║");
            info!("║  Output: {}", agent_result.output_path.display());
            info!("║  Tokens: ~{}", agent_result.tokens_used);
            info!("╚═══════════════════════════════════════════╝");
        }
        Err(e) => {
            error!("╔═══════════════════════════════════════════╗");
            error!("║  AGENT FAILED                             ║");
            error!("║  Error: {}", e);
            for cause in e.chain().skip(1) {
                error!("║  Caused by: {}", cause);
            }
            error!("╚═══════════════════════════════════════════╝");
            std::process::exit(1);
        }
    }

    Ok(())
}
