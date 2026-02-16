//! Standardized JSON responses for game actions.
//!
//! # The "Universal Response"
//!
//! To simplify client-side logic (and LLM understanding), all tool calls return
//! the same JSON structure: [`DuelResponse`].
//!
//! ## Design Philosophy
//!
//! 1.  **Always Success**: The server prefers returning `200 OK` with `success: false`
//!     rather than JSON-RPC errors for game logic failures (like "not your turn").
//!     This allows the LLM to read the error message and correct its behavior
//!     without crashing the session.
//! 2.  **State Inclusion**: Every successful response includes the current [`DuelStateView`],
//!     so the client immediately knows the result of their action.
//! 3.  **Human Readable**: The `message` field is designed to be read by the LLM
//!     to understand "what just happened".
//!
//! # Hero's Journey (Example Usage)
//!
//! ```
//! use insult_arena_mcp::{DuelResponse, DuelStateView};
//!
//! // 1. Construct a success response
//! let view = DuelStateView {
//!     phase: "active".to_string(),
//!     challenger_score: 0,
//!     defender_score: 0,
//!     wins_needed: 3,
//!     next_to_act: None,
//!     pending_insult: None,
//!     winner: None
//! };
//!
//! let response = DuelResponse::success("Duel started!", view);
//! assert!(response.success);
//!
//! // 2. Serialize to JSON for the client
//! let json = response.to_json();
//! assert!(json.contains("Duel started!"));
//! ```

use crate::duel::DuelStateView;
use serde::{Deserialize, Serialize};

/// Response from the arena/server.
///
/// This struct is serialized to JSON and returned as the tool's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelResponse {
    /// Whether the action succeeded.
    ///
    /// If `false`, the `message` contains the error description.
    pub success: bool,
    /// Human-readable message about what happened.
    ///
    /// E.g., "You fought like a dairy farmer!" or "It is not your turn!".
    pub message: String,
    /// Current state of the duel.
    ///
    /// Present on most successful actions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DuelStateView>,
    /// Your role in this duel (if registered).
    ///
    /// Only present for registration actions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_role: Option<String>,
    /// List of available insults.
    ///
    /// Only present for `list_insults`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insults: Option<Vec<String>>,
    /// Hint for the current comeback.
    ///
    /// Only present for `get_hint`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// The pending insult (for context).
    ///
    /// Only present for `get_hint`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insult: Option<String>,
}

impl DuelResponse {
    /// Creates a success response with the current duel state.
    pub fn success(message: impl Into<String>, state: DuelStateView) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: None,
            insults: None,
            hint: None,
            insult: None,
        }
    }

    /// Creates a success response that also includes the user's role.
    pub fn success_with_role(message: impl Into<String>, state: DuelStateView, role: &str) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: Some(role.to_string()),
            insults: None,
            hint: None,
            insult: None,
        }
    }

    /// Creates an error response.
    ///
    /// The `success` field will be `false`.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            state: None,
            your_role: None,
            insults: None,
            hint: None,
            insult: None,
        }
    }

    /// Creates a response with a list of insults.
    pub fn with_insults(message: impl Into<String>, insults: Vec<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: None,
            your_role: None,
            insults: Some(insults),
            hint: None,
            insult: None,
        }
    }

    /// Creates a response with a hint.
    pub fn with_hint(message: impl Into<String>, hint: String, insult: String) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: None,
            your_role: None,
            insults: None,
            hint: Some(hint),
            insult: Some(insult),
        }
    }

    /// Serializes the response to a JSON string.
    ///
    /// Returns an error message string if serialization fails (which should be rare).
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "Error serializing response".to_string())
    }
}
