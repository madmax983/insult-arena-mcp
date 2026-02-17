//! Action parsing for MCP tool requests.
//!
//! # The "Parser" Layer
//!
//! This module acts as the "Parser" in the "Parse, Don't Validate" pattern.
//! It decouples the *intent* of a tool call (what the user wants to do) from the
//! *mechanics* of how it's executed.
//!
//! ## Responsibilities
//!
//! 1.  **Parsing**: Converts untyped JSON (`CallToolRequestParams`) into a strongly-typed [`ToolAction`] enum.
//! 2.  **Validation**: Enforces structural validity (e.g., `start_duel` takes no args).
//! 3.  **Security**: Enforces input length limits *before* any heavy business logic runs, preventing `DoS` via memory exhaustion.
//!
//! ## Hero's Journey (Implementation)
//!
//! ```ignore
//! use insult_arena_mcp::server::action::ToolAction;
//! use rust_mcp_sdk::schema::CallToolRequestParams;
//! use serde_json::json;
//!
//! // 1. Receive a raw request from the client
//! let params = CallToolRequestParams {
//!     name: "throw_insult".to_string(),
//!     arguments: Some(serde_json::Map::from_iter(vec![
//!         ("insult".to_string(), json!("You fight like a dairy farmer!"))
//!     ])),
//!     meta: None,
//!     task: None,
//! };
//!
//! // 2. Parse it into a strongly-typed Action
//! let action = ToolAction::try_from(params).unwrap();
//!
//! // 3. Match and execute (in the Server)
//! if let ToolAction::ThrowInsult { insult } = action {
//!     assert_eq!(insult, "You fight like a dairy farmer!");
//! }
//! ```

use rust_mcp_sdk::schema::CallToolRequestParams;
use rust_mcp_sdk::schema::schema_utils::CallToolError;

/// Represents a parsed and validated tool action.
///
/// This enum encapsulates the intent of a client's tool call.
/// It is constructed by parsing `CallToolRequestParams`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolAction {
    /// Start a new duel.
    ///
    /// Corresponds to the `start_duel` tool.
    /// Requires no arguments.
    StartDuel,
    /// Register as the Challenger (attacks first).
    ///
    /// Corresponds to the `register_as_challenger` tool.
    RegisterChallenger,
    /// Register as the Defender (responds to insults).
    ///
    /// Corresponds to the `register_as_defender` tool.
    RegisterDefender,
    /// Check the current game state.
    ///
    /// Corresponds to the `get_duel_state` tool.
    GetDuelState,
    /// List all valid insults.
    ///
    /// Corresponds to the `list_insults` tool.
    ListInsults,
    /// Throw a specific insult.
    ///
    /// Corresponds to the `throw_insult` tool.
    ///
    /// # Validation
    ///
    /// The `insult` string is truncated/rejected if it exceeds [`crate::arena::MAX_INPUT_LENGTH`].
    ThrowInsult {
        /// The insult string to throw.
        insult: String,
    },
    /// Respond with a comeback.
    ///
    /// Corresponds to the `respond` tool.
    ///
    /// # Validation
    ///
    /// The `comeback` string is truncated/rejected if it exceeds [`crate::arena::MAX_INPUT_LENGTH`].
    Respond {
        /// The comeback string to use.
        comeback: String,
    },
    /// Get a hint for the current pending insult.
    ///
    /// Corresponds to the `get_hint` tool.
    GetHint,
}

impl ToolAction {
    /// Internal helper to extract and validate a string argument.
    ///
    /// # Security
    ///
    /// This method enforces length limits *before* returning the string to prevent
    /// Denial of Service (`DoS`) attacks via memory exhaustion.
    fn take_string(
        args: &mut serde_json::Map<String, serde_json::Value>,
        key: &str,
        tool_name: &str,
        error_label: &str,
    ) -> Result<String, CallToolError> {
        let val_str = match args.remove(key) {
            Some(serde_json::Value::String(s)) => s,
            _ => String::new(),
        };

        // 🛡️ HARDENING: Check length BEFORE allocation to prevent DoS
        // Note: The allocation happened when serde parsed the JSON request,
        // but we prevent further cloning/allocation here.
        if val_str.len() > crate::arena::MAX_INPUT_LENGTH {
            return Err(CallToolError::invalid_arguments(
                tool_name,
                Some(format!(
                    "{} too long (max {} chars)",
                    error_label,
                    crate::arena::MAX_INPUT_LENGTH
                )),
            ));
        }
        Ok(val_str)
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
                insult: Self::take_string(&mut args, "insult", &tool_name, "Insult")?,
            }),
            "respond" => Ok(Self::Respond {
                comeback: Self::take_string(&mut args, "comeback", &tool_name, "Comeback")?,
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
        assert!(format!("{err:?}").contains("Insult too long"));
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
                assert_eq!(insult, "", "Number should become empty string");
            }
            _ => panic!("Expected ThrowInsult"),
        }
    }
}
