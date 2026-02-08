use thiserror::Error;

/// Errors that can occur during a duel.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InsultError {
    /// The insult is not recognized.
    #[error("Unknown insult: {0}")]
    UnknownInsult(String),
    /// It's not time for an insult, we're waiting for a comeback.
    #[error("Waiting for a comeback, not an insult")]
    WaitingForComeback,
    /// It's not time for a comeback, we're waiting for an insult.
    #[error("Waiting for an insult, not a comeback")]
    WaitingForInsult,
    /// The duel is already over.
    #[error("The duel is over")]
    DuelOver,
    /// No active duel.
    #[error("No active duel - challenge someone first")]
    NoDuel,
}

/// Errors that can occur in the Arena.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ArenaError {
    #[error("No duel in progress. Call start_duel first!")]
    NoDuel,
    #[error("Input too long (max {0} chars)")]
    InputTooLong(usize),
    #[error("Session ID too long (max {0} chars)")]
    SessionIdTooLong(usize),
    #[error("{0} role is already taken!")]
    RoleTaken(String),
    #[error("It is not your turn! Waiting for {0}.")]
    NotYourTurn(String),
    #[error("Unknown insult: \"{0}\". Use list_insults to see valid options.")]
    UnknownInsult(String),
    #[error("No pending insult to hint about.")]
    NoPendingInsult,
    #[error("Could not find comeback for this insult.")]
    ComebackNotFound,
    #[error(transparent)]
    DuelError(#[from] InsultError),
}
