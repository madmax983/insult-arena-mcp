//! Experimental features and mechanics.
//!
//! This module contains features that are either in development,
//! behind feature flags (like `nova`), or add extra "juice" to the game
//! beyond the core mechanics.
//!
//! # Hero's Journey (Dojo + Weather)
//!
//! ```
//! use insult_arena_mcp::experimental::dojo::Dojo;
//! use insult_arena_mcp::experimental::weather::WeatherSystem;
//!
//! // 1. Enter the Dojo with a weather system
//! let mut dojo = Dojo::new(0.5);
//! let mut weather = WeatherSystem::new();
//!
//! // 2. Randomize conditions
//! weather.randomize();
//! println!("Weather: {}", weather.description());
//!
//! // 3. Duel the Sensei!
//! let events = dojo.turn("You fight like a dairy farmer!").unwrap();
//! assert!(!events.is_empty());
//! ```
//!
//! # Modules
//!
//! - **Achievements**: Post-match analysis to award badges like "Clutch Master".
//! - **Audience**: Tracks "hype" and crowd reactions (cheers, boos) to exchanges.
//! - **Dojo**: A single-player training mode against an AI "Sensei".
//! - **Parrot**: A helper system that provides masked hints for comebacks.
//! - **Reporter**: Generates a narrative chronicle of the duel.
//! - **Sensei**: AI Logic for the Dojo opponent.
//! - **Weather**: Simulates environmental conditions (Fog, Storm) that affect gameplay.
//!
//! # Feature Flags
//!
//! Many of these features are gated behind the `nova` feature flag in `Cargo.toml`.

pub mod achievements;
pub mod audience;
pub mod dojo;
pub mod parrot;
pub mod reporter;
pub mod sensei;
pub mod weather;
