//! Action parsing for MCP tool requests.
//!
//! This module decouples the parsing of tool arguments from the tool definitions themselves.
//! It implements validation logic (like input length checks) to prevent `DoS`.

use rust_mcp_sdk::schema::CallToolRequestParams;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use crate::arena::PlayerInput;

/// Represents a parsed and validated tool action.
///
/// This enum encapsulates the intent of a client's tool call.
/// It is constructed by parsing `CallToolRequestParams`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolAction {
    /// Start a new duel.
    StartDuel,
    /// Register as the Challenger (attacks first).
    RegisterChallenger,
    /// Register as the Defender (responds to insults).
    RegisterDefender,
    /// Check the current game state.
    GetDuelState,
    /// List all valid insults.
    ListInsults,
    /// Throw a specific insult.
    ThrowInsult {
        /// The insult string to throw.
        insult: PlayerInput,
    },
    /// Respond with a comeback.
    Respond {
        /// The comeback string to use.
        comeback: PlayerInput,
    },
    /// Get a hint for the current pending insult.
    GetHint,
}

impl ToolAction {
    fn take_player_input(
        args: &mut serde_json::Map<String, serde_json::Value>,
        key: &str,
        tool_name: &str,
    ) -> Result<PlayerInput, CallToolError> {
        let val_str = match args.remove(key) {
            Some(serde_json::Value::String(s)) => s,
            _ => String::new(),
        };

        // 🛡️ HARDENING: PlayerInput constructor enforces length limits
        PlayerInput::new(val_str).map_err(|e| {
            CallToolError::invalid_arguments(
                tool_name,
                Some(e.to_string()),
            )
        })
    }
}

impl TryFrom<CallToolRequestParams> for ToolAction {
    type Error = CallToolError;

    fn try_from(params: CallToolRequestParams) -> Result<Self, Self::Error> {
        // ⚡ Bolt Optimization: Take ownership of arguments to avoid string cloning.
        let mut args = params.arguments.unwrap_or_default();
        let tool_name = params.name;

        match tool_name.as_str() {
            "start_duel" => Ok(Self::StartDuel),
            "register_as_challenger" => Ok(Self::RegisterChallenger),
            "register_as_defender" => Ok(Self::RegisterDefender),
            "get_duel_state" => Ok(Self::GetDuelState),
            "list_insults" => Ok(Self::ListInsults),
            "throw_insult" => Ok(Self::ThrowInsult {
                insult: Self::take_player_input(&mut args, "insult", &tool_name)?,
            }),
            "respond" => Ok(Self::Respond {
                comeback: Self::take_player_input(&mut args, "comeback", &tool_name)?,
            }),
            "get_hint" => Ok(Self::GetHint),
            _ => Err(CallToolError::unknown_tool(&tool_name)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_excessive_input_length_efficiently() {
        let long_string = "a".repeat(crate::arena::MAX_INPUT_LENGTH + 1);
        let mut args = serde_json::Map::new();
        args.insert("insult".to_string(), json!(long_string));

        let params = CallToolRequestParams {
            name: "throw_insult".to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{err:?}").contains("Input too long"));
    }

    #[test]
    fn rejects_invalid_types_as_empty_string() {
        // Test that non-string arguments (number, null, missing) are treated as empty strings
        let mut args = serde_json::Map::new();
        args.insert("insult".to_string(), json!(12345)); // Number instead of string

        let params = CallToolRequestParams {
            name: "throw_insult".to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        // Should return Ok but with empty insult string (default behavior)
        let result = ToolAction::try_from(params).unwrap();
        match result {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(insult.as_str(), "", "Number should become empty string");
            }
            _ => panic!("Expected ThrowInsult"),
        }
    }
}
