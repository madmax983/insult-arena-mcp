//! # Insult Arena MCP
//!
//! LLM vs LLM Monkey Island-style insult sword fighting over MCP.
//!
//! This crate provides an MCP (Model Context Protocol) server that lets
//! AI models engage in the classic insult sword fighting from Monkey Island.
//!
//! ## Architecture
//!
//! The system is layered as follows:
//!
//! 1. **[`InsultServer`]**: The entry point. Handles MCP protocol, tool parsing, and turn notifications.
//! 2. **[`Arena`]**: The game manager. Handles session management (who is Challenger/Defender), timeouts, and input validation.
//! 3. **[`Duel`]**: The core game logic state machine. Purely functional logic for the insult sword fighting rules.
//! 4. **[`Announcer`]**: A stateless formatter that turns game events into flavor text.
//!
//! ## The Duel Protocol
//!
//! The core loop revolves around the [`Exchange`]:
//!
//! 1. **Attacker** throws an insult.
//! 2. **Defender** responds with a comeback.
//! 3. The result is an [`ExchangeResult`]:
//!    - [`ExchangeResult::Parried`]: Defender wins, becomes Attacker.
//!    - [`ExchangeResult::Failed`]: Defender loses, Attacker attacks again.
//!
//! ## Features
//!
//! - **Classic Insults**: 16 pairs from the original game via [`InsultBank`].
//! - **Game Logic**: Turn-based dueling engine via [`Duel`] and [`Arena`].
//! - **Commentary**: Dynamic flavor text generation via [`Announcer`].
//! - **MCP Server**: Full implementation with turn notifications via [`InsultServer`].
//! - **Multi-Client**: SSE transport supports multiple connected AI agents.
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
//! ```rust
//! use insult_arena_mcp::InsultServer;
//!
//! // Create a new server instance (starts with no active duel)
//! let server = InsultServer::new();
//! ```

pub mod announcer;
pub mod arena;
mod duel;
mod insults;
mod server;

pub use announcer::Announcer;
pub use arena::{Arena, ArenaOutcome};
pub use duel::{
    Duel, DuelResult, DuelState, DuelStateView, Duelist, Exchange, ExchangeResult, InsultError,
};
pub use insults::{InsultBank, InsultPair};
pub use server::{DuelResponse, InsultServer};

#[cfg(feature = "nova")]
pub mod experimental;
