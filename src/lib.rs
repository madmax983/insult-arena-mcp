//! # Insult Arena MCP
//!
//! LLM vs LLM Monkey Island-style insult sword fighting over MCP.
//!
//! This crate provides an MCP (Model Context Protocol) server that lets
//! AI models engage in the classic insult sword fighting from Monkey Island.
//!
//! ## Features
//!
//! - 16 classic Monkey Island insults and comebacks
//! - Turn-based dueling (first to 3 wins)
//! - SSE transport for multi-client support
//! - Works with Claude, GPT, and other MCP-compatible clients
//!
//! ## Quick Start
//!
//! ```bash
//! # Start the arena server
//! cargo run
//!
//! # Connect with mcp-remote
//! npx mcp-remote http://localhost:3000/sse
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use insult_arena_mcp::InsultServer;
//!
//! let server = InsultServer::new();
//! // Server implements MCP's ServerHandler trait
//! ```

pub mod announcer;
pub mod arena;
pub mod error;
pub mod model;

mod duel;
mod insults;
mod server;

pub use announcer::Announcer;
pub use arena::Arena;
pub use duel::Duel;
pub use error::{ArenaError, InsultError};
pub use insults::InsultBank;
pub use model::{
    ArenaOutcome, DuelResult, DuelState, DuelStateView, Duelist, Exchange, ExchangeResult,
    InsultPair,
};
pub use server::{DuelResponse, InsultServer};

#[cfg(feature = "nova")]
pub mod experimental;
