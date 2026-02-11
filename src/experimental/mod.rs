//! Experimental features and mechanics.
//!
//! This module contains features that are either in development,
//! behind feature flags (like `nova`), or add extra "juice" to the game
//! beyond the core mechanics.
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
