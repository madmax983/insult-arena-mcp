use serde::{Deserialize, Serialize};

pub const MAX_INPUT_LENGTH: usize = 1024;
pub const MAX_SESSION_ID_LENGTH: usize = 128;

/// Identifies a duelist in the fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Duelist {
    /// The challenger who initiated the duel.
    Challenger,
    /// The defender who accepted the challenge.
    Defender,
}

impl Duelist {
    /// Returns the opponent of this duelist.
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::Challenger => Self::Defender,
            Self::Defender => Self::Challenger,
        }
    }
}

impl std::fmt::Display for Duelist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Challenger => write!(f, "Challenger"),
            Self::Defender => write!(f, "Defender"),
        }
    }
}

/// The result of a single exchange.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExchangeResult {
    /// The defender parried with a perfect comeback.
    Parried {
        /// The insult that was thrown.
        insult: String,
        /// The comeback that defeated it.
        comeback: String,
    },
    /// The defender failed to counter the insult.
    Failed {
        /// The insult that was thrown.
        insult: String,
        /// The failed comeback attempt.
        attempt: String,
        /// The correct comeback they should have used.
        correct: String,
    },
}

impl ExchangeResult {
    /// Returns true if the defender successfully parried.
    #[must_use]
    pub const fn is_parried(&self) -> bool {
        matches!(self, Self::Parried { .. })
    }
}

/// A single exchange in the duel (insult + response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exchange {
    /// Who threw the insult.
    pub attacker: Duelist,
    /// The result of the exchange.
    pub result: ExchangeResult,
    /// Who won this exchange.
    pub winner: Duelist,
}

/// The current state of a duel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuelState {
    /// Waiting for an insult to be thrown.
    AwaitingInsult {
        /// Who should throw the insult.
        attacker: Duelist,
    },
    /// Waiting for a comeback response.
    AwaitingComeback {
        /// Who threw the insult.
        attacker: Duelist,
    },
    /// The duel is over.
    Finished {
        /// Who won the duel.
        winner: Duelist,
    },
}

/// The final result of a completed duel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelResult {
    /// The winner of the duel.
    pub winner: Duelist,
    /// Final score for the challenger.
    pub challenger_score: u8,
    /// Final score for the defender.
    pub defender_score: u8,
    /// All exchanges that occurred.
    pub exchanges: Vec<Exchange>,
}

/// The outcome of an action in the Arena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArenaOutcome {
    DuelStarted,
    RoleRegistered { role: Duelist },
    InsultThrown { insult: String },
    ExchangeProcessed { exchange: Exchange },
}

/// Serializable view of the duel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelStateView {
    /// Current phase of the duel.
    pub phase: String,
    /// Who should act next (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_to_act: Option<String>,
    /// The pending insult waiting for a comeback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_insult: Option<String>,
    /// Challenger's score.
    pub challenger_score: u8,
    /// Defender's score.
    pub defender_score: u8,
    /// Wins needed to win the duel.
    pub wins_needed: u8,
    /// The winner (if duel is over).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner: Option<String>,
}
