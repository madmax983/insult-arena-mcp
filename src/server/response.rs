use crate::model::DuelStateView;
use serde::{Deserialize, Serialize};

/// Response from the arena/server.
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

    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "Error serializing response".to_string())
    }
}
