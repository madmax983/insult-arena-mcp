//! Core domain types for the Arena.
//!
//! This module contains the foundational types used throughout the Arena system,
//! including validated inputs (`SessionId`, `PlayerInput`) and outcome types.

use std::borrow::Cow;

use crate::duel::{Duelist, InsultError};

/// Maximum length of any user input string (insults, comebacks).
///
/// This limit prevents Denial of Service (`DoS`) attacks via memory exhaustion.
pub const MAX_INPUT_LENGTH: usize = 1024;

/// Maximum length of a session ID string.
///
/// This limit prevents Denial of Service (`DoS`) attacks where malicious clients
/// send excessively long session IDs.
pub const MAX_SESSION_ID_LENGTH: usize = 128;

/// A validated session ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Returns the session ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SessionId {
    type Error = ArenaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_SESSION_ID_LENGTH {
            Err(ArenaError::SessionIdTooLong(MAX_SESSION_ID_LENGTH))
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A validated player input string (insult or comeback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInput(String);

impl PlayerInput {
    /// Returns the input as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl TryFrom<String> for PlayerInput {
    type Error = ArenaError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_INPUT_LENGTH {
            Err(ArenaError::InputTooLong(MAX_INPUT_LENGTH))
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<str> for PlayerInput {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Errors that can occur in the Arena.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArenaError {
    /// No duel is currently active.
    #[error("No duel in progress. Call start_duel first!")]
    NoDuel,
    /// User input exceeded the maximum allowed length.
    #[error("Input too long (max {0} chars)")]
    InputTooLong(usize),
    /// Session ID exceeded the maximum allowed length.
    #[error("Session ID too long (max {0} chars)")]
    SessionIdTooLong(usize),
    /// The requested role is already occupied by another session.
    #[error("{0} role is already taken!")]
    RoleTaken(String),
    /// The session tried to act out of turn.
    #[error("It is not your turn! Waiting for {0}.")]
    NotYourTurn(String),
    /// The insult is not in the [`crate::InsultBank`].
    #[error("Unknown insult: \"{0}\". Use list_insults to see valid options.")]
    UnknownInsult(String),
    /// A hint was requested but no insult is pending.
    #[error("No pending insult to hint about.")]
    NoPendingInsult,
    /// Could not find a comeback for the pending insult (should not happen with valid insults).
    #[error("Could not find comeback for this insult.")]
    ComebackNotFound,
    /// Tried to start a duel while one is already in progress.
    #[error("A duel is already in progress. Wait for it to finish!")]
    DuelInProgress,
    /// An error from the underlying duel logic.
    #[error(transparent)]
    DuelError(#[from] InsultError),
}

/// The outcome of an action in the Arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaOutcome {
    /// A new duel has started.
    DuelStarted,
    /// A session has successfully registered for a role.
    RoleRegistered {
        /// The role that was assigned.
        role: Duelist,
    },
    /// An insult was successfully thrown.
    InsultThrown {
        /// The insult that was thrown.
        insult: Cow<'static, str>,
    },
    /// An exchange (insult + comeback) was completed.
    ExchangeProcessed {
        /// The result of the exchange.
        exchange: crate::duel::Exchange,
    },
}
