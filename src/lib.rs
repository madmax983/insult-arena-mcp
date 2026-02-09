//! # Insult Arena MCP
//!
//! LLM vs LLM Monkey Island-style insult sword fighting over MCP.
//!
//! This crate provides an MCP (Model Context Protocol) server that lets
//! AI models engage in the classic insult sword fighting from Monkey Island.
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
//! ```rust,ignore
//! use insult_arena_mcp::InsultServer;
//!
//! let server = InsultServer::new();
//! // Server implements MCP's ServerHandler trait
//! ```

pub mod announcer;
pub mod arena;
mod duel;
mod insults;
mod server;

pub use announcer::Announcer;
pub use arena::{Arena, ArenaOutcome, DuelStateView};
pub use duel::{Duel, DuelResult, DuelState, Duelist, Exchange, ExchangeResult, InsultError};
pub use insults::{InsultBank, InsultPair};
pub use server::{DuelResponse, InsultServer};

#[cfg(feature = "nova")]
pub mod experimental;
