//! Presentation layer - CLI interface and command handling
//!
//! This layer contains:
//! - CLI argument parsing
//! - Command handlers
//! - Interactive REPL mode
//! - Output formatting

pub mod cli;
pub mod commands;
pub mod interactive;

pub use cli::*;
pub use commands::*;
pub use interactive::*;
