//! Response DTOs for the MCP Server.
//!
//! This module defines the standard JSON response format returned by all
//! tools in the Insult Arena.
//!
//! # The API Contract
//!
//! Every tool returns a JSON object with at least:
//! - `success`: Boolean indicating if the action worked.
//! - `message`: Human-readable description of the result.
//!
//! If successful, it may also contain:
//! - `state`: The current `DuelStateView`.
//! - `your_role`: The role of the calling session (if applicable).

use crate::arena::DuelStateView;
use serde::{Deserialize, Serialize};

/// Standard response format for all Arena tools.
///
/// # Examples
///
/// ```
/// use insult_arena_mcp::DuelResponse;
///
/// // Create a simple error response
/// let err = DuelResponse::error("Something went wrong");
/// assert_eq!(err.success, false);
/// ```
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

impl DuelResponse {
    /// Creates a successful response with the current duel state.
    pub fn success(message: impl Into<String>, state: DuelStateView) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: None,
        }
    }

    /// Creates a successful response with state and the user's role.
    pub fn success_with_role(message: impl Into<String>, state: DuelStateView, role: &str) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: Some(role.to_string()),
        }
    }

    /// Creates an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            state: None,
            your_role: None,
        }
    }

    /// Serializes the response to a pretty-printed JSON string.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "Error serializing response".to_string())
    }
}
