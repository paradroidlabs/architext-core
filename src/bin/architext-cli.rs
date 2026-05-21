use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::exit;

use architext_core::ipc::{SignalBus, SignalKind};

/// Architext CLI Companion — Filesystem-native IPC interaction
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
    /// Display the current status of the Architext IPC bus
    Status,
    
    /// Acquire the external agent lock
    Lock {
        /// Name of the external agent
        agent: String,
        /// Description of the current task
        task: String,
    },
    
    /// Release the external agent lock
    Unlock,
    
    /// Append a message to the agent log stream
    Log {
        /// The message to append
        message: String,
    },
    
    /// Send a control signal to the internal Architext daemon or active external agent
    Signal {
        /// The kind of signal to send (cancel, approve, pause)
        #[arg(value_parser = parse_signal)]
        kind: SignalKind,
    },
}

fn parse_signal(s: &str) -> Result<SignalKind, String> {
    match s.to_lowercase().as_str() {
        "cancel" => Ok(SignalKind::Cancel),
        "approve" => Ok(SignalKind::Approve),
        "pause" => Ok(SignalKind::Pause),
        _ => Err(format!("Unknown signal kind: {}. Use cancel, approve, or pause.", s)),
    }
}

fn main() {
    let cli = Cli::parse();
    
    let bus = SignalBus::new(&cli.project);
    
    // Commands that don't strictly require an initialized directory
    match cli.command {
        Commands::Status => {
            if !bus.is_initialized() {
                println!("No .architext/ directory found in {}. IPC is not initialized.", cli.project.display());
                exit(1);
            }
            
            println!("IPC Directory: {}", bus.ipc_dir().display());
            
            match bus.read_agent_lock() {
                Some(lock) => {
                    println!("\n[LOCKED] External Agent Active:");
                    println!("  Agent:   {}", lock.agent);
                    println!("  Task:    {}", lock.task);
                    if let Some(pid) = lock.pid {
                        println!("  PID:     {}", pid);
                    }
                    println!("  Started: {}", lock.started);
                }
                None => {
                    println!("\n[IDLE] No external agent lock found.");
                }
            }
            
            println!("\nActive Signals:");
            for kind in SignalKind::all() {
                if bus.check_signal(*kind) {
                    println!("  - {:?}", kind);
                }
            }
        }
        Commands::Lock { agent, task } => {
            if let Err(e) = bus.ensure_dirs() {
                eprintln!("Error creating IPC directories: {}", e);
                exit(1);
            }
            
            if let Some(lock) = bus.read_agent_lock() {
                eprintln!("Warning: Overwriting existing lock from agent '{}' (task: '{}')", lock.agent, lock.task);
            }
            
            if let Err(e) = bus.write_agent_lock(&agent, &task) {
                eprintln!("Error writing lock: {}", e);
                exit(1);
            }
            println!("Lock acquired for agent '{}', task '{}'", agent, task);
        }
        Commands::Unlock => {
            if let Err(e) = bus.clear_agent_lock() {
                eprintln!("Error clearing lock: {}", e);
                exit(1);
            }
            println!("Lock released.");
        }
        Commands::Log { message } => {
            if let Err(e) = bus.ensure_dirs() {
                eprintln!("Error creating IPC directories: {}", e);
                exit(1);
            }
            
            if let Err(e) = bus.append_agent_log(&message) {
                eprintln!("Error appending log: {}", e);
                exit(1);
            }
            // Silent success for logging to avoid terminal clutter when used in scripts
        }
        Commands::Signal { kind } => {
            if let Err(e) = bus.ensure_dirs() {
                eprintln!("Error creating IPC directories: {}", e);
                exit(1);
            }
            
            if let Err(e) = bus.write_signal(kind) {
                eprintln!("Error writing signal: {}", e);
                exit(1);
            }
            println!("Signal sent: {:?}", kind);
        }
    }
}
