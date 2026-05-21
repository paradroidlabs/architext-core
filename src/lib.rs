//! # Architext Core
//!
//! The headless daemon for the Architext: Narrative IDE.
//!
//! ## Architecture
//! - `watcher`: Filesystem event detection via native OS APIs (notify crate)
//! - `protocol`: The 8-phase Narrative AI Protocol state machine
//! - `context`: Smart context assembly from the filesystem
//! - `gemini`: Bespoke REST client for the Gemini 3.1 Pro/Flash API
//! - `agents`: Stateless agent definitions (Planner, Writer, Review)
//! - `ipc`: Filesystem-native IPC signal bus (.architext/ directory)

pub mod watcher;
pub mod protocol;
pub mod context;
pub mod gemini;
pub mod agents;
pub mod ui;
pub mod theme;
pub mod prompts;
pub mod logger;
pub mod ipc;
pub mod telemetry;
