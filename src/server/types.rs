use crate::duel::{Duel, DuelState};
use serde::{Deserialize, Serialize};

/// Response from the insult server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelResponse {
    /// Whether the action succeeded.
    pub success: bool,
    /// Human-readable message about what happened.
    pub message: String,
    /// Current state of the duel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DuelStateView>,
    /// Your role in this duel (if registered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_role: Option<String>,
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

impl DuelResponse {
    pub fn success(message: impl Into<String>, state: DuelStateView) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: None,
        }
    }

    pub fn success_with_role(message: impl Into<String>, state: DuelStateView, role: &str) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: Some(role.to_string()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            state: None,
            your_role: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "Error serializing response".to_string())
    }
}

pub fn duel_state_view(duel: &Duel) -> DuelStateView {
    let (phase, next_to_act, winner) = match duel.state() {
        DuelState::AwaitingInsult { attacker } => (
            "awaiting_insult".to_string(),
            Some(attacker.to_string()),
            None,
        ),
        DuelState::AwaitingComeback { attacker } => (
            "awaiting_comeback".to_string(),
            Some(attacker.opponent().to_string()),
            None,
        ),
        DuelState::Finished { winner } => ("finished".to_string(), None, Some(winner.to_string())),
    };

    let (challenger_score, defender_score) = duel.scores();

    DuelStateView {
        phase,
        next_to_act,
        pending_insult: duel.pending_insult().map(String::from),
        challenger_score,
        defender_score,
        wins_needed: 3,
        winner,
    }
}
