//! API definitions for the Insult Arena MCP server.
//!
//! This module defines the "Contract" between the server and the MCP clients.
//! It consolidates:
//! 1. **Constants**: Tool names and argument keys.
//! 2. **Schema**: The `Tool` definitions exposed to LLMs.
//! 3. **Parsing**: The `ToolAction` enum and logic to parse `CallToolRequestParams`.

use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{CallToolRequestParams, Tool, ToolInputSchema};
use serde::{Deserialize, Deserializer};
use serde_json::json;
use std::collections::HashMap;

// --- CONSTANTS ---

/// Tool Name: Start a new duel.
pub const START_DUEL: &str = "start_duel";

/// Tool Name: Register as the Challenger.
pub const REGISTER_CHALLENGER: &str = "register_as_challenger";

/// Tool Name: Register as the Defender.
pub const REGISTER_DEFENDER: &str = "register_as_defender";

/// Tool Name: Get the current duel state.
pub const GET_DUEL_STATE: &str = "get_duel_state";

/// Tool Name: List available insults.
pub const LIST_INSULTS: &str = "list_insults";

/// Tool Name: Throw an insult.
pub const THROW_INSULT: &str = "throw_insult";

/// Tool Name: Respond with a comeback.
pub const RESPOND: &str = "respond";

/// Tool Name: Get a hint for the pending insult.
pub const GET_HINT: &str = "get_hint";

/// Argument Key: The insult string.
pub const ARG_INSULT: &str = "insult";

/// Argument Key: The comeback string.
pub const ARG_COMEBACK: &str = "comeback";

// --- ACTION PARSING ---

/// Represents a parsed and validated tool action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolAction {
    /// Start a new duel.
    StartDuel,
    /// Register as the Challenger.
    RegisterChallenger,
    /// Register as the Defender.
    RegisterDefender,
    /// Check the current game state.
    GetDuelState,
    /// List all valid insults.
    ListInsults,
    /// Throw a specific insult.
    ThrowInsult {
        /// The insult string to throw.
        insult: String,
    },
    /// Respond with a comeback.
    Respond {
        /// The comeback string to use.
        comeback: String,
    },
    /// Get a hint for the current pending insult.
    GetHint,
}

/// Helper for lossy string deserialization.
fn deserialize_lossy_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match v {
        serde_json::Value::String(s) => Ok(s),
        _ => Ok(String::new()),
    }
}

/// Arguments for `throw_insult`.
#[derive(Deserialize)]
struct ThrowInsultArgs {
    #[serde(rename = "insult", deserialize_with = "deserialize_lossy_string")]
    insult: String,
}

/// Arguments for `respond`.
#[derive(Deserialize)]
struct RespondArgs {
    #[serde(rename = "comeback", deserialize_with = "deserialize_lossy_string")]
    comeback: String,
}

impl ToolAction {
    /// Helper to validate string length to prevent `DoS`.
    fn validate_length(
        s: &str,
        field_name: &str,
        tool_name: &str,
        limit: usize,
    ) -> Result<(), CallToolError> {
        if s.len() > limit {
            Err(CallToolError::invalid_arguments(
                tool_name,
                Some(format!("{field_name} too long (max {limit} chars)")),
            ))
        } else {
            Ok(())
        }
    }
}

impl TryFrom<CallToolRequestParams> for ToolAction {
    type Error = CallToolError;

    fn try_from(params: CallToolRequestParams) -> Result<Self, Self::Error> {
        let args_val = params
            .arguments
            .map_or(serde_json::Value::Null, serde_json::Value::Object);
        let tool_name = params.name;

        match tool_name.as_str() {
            START_DUEL => Ok(Self::StartDuel),
            REGISTER_CHALLENGER => Ok(Self::RegisterChallenger),
            REGISTER_DEFENDER => Ok(Self::RegisterDefender),
            GET_DUEL_STATE => Ok(Self::GetDuelState),
            LIST_INSULTS => Ok(Self::ListInsults),
            GET_HINT => Ok(Self::GetHint),

            THROW_INSULT => {
                let args: ThrowInsultArgs =
                    serde_json::from_value(args_val).unwrap_or_else(|_| ThrowInsultArgs {
                        insult: String::new(),
                    });

                Self::validate_length(
                    &args.insult,
                    "Insult",
                    &tool_name,
                    crate::arena::MAX_INPUT_LENGTH,
                )?;

                Ok(Self::ThrowInsult {
                    insult: args.insult,
                })
            }

            RESPOND => {
                let args: RespondArgs =
                    serde_json::from_value(args_val).unwrap_or_else(|_| RespondArgs {
                        comeback: String::new(),
                    });

                Self::validate_length(
                    &args.comeback,
                    "Comeback",
                    &tool_name,
                    crate::arena::MAX_INPUT_LENGTH,
                )?;

                Ok(Self::Respond {
                    comeback: args.comeback,
                })
            }

            _ => Err(CallToolError::unknown_tool(&tool_name)),
        }
    }
}

// --- TOOL DEFINITIONS ---

/// Helper to create an empty input schema.
pub fn empty_input_schema() -> ToolInputSchema {
    ToolInputSchema::new(vec![], None, None)
}

/// Helper to create an input schema with a single required string parameter.
pub fn string_param_schema(name: &str, description: &str) -> ToolInputSchema {
    let mut props = HashMap::new();
    let mut prop_map = serde_json::Map::new();
    prop_map.insert("type".to_string(), json!("string"));
    prop_map.insert("description".to_string(), json!(description));
    props.insert(name.to_string(), prop_map);

    ToolInputSchema::new(vec![name.to_string()], Some(props), None)
}

/// Internal helper to create a base tool definition.
fn create_base_tool(name: &str, description: &str, input_schema: ToolInputSchema) -> Tool {
    Tool {
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema,
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

pub fn tool_start_duel() -> Tool {
    create_base_tool(
        START_DUEL,
        "Start a new insult sword fighting duel! The Challenger throws the first insult. First to 3 exchange wins takes the duel.",
        empty_input_schema(),
    )
}

pub fn tool_register_as_challenger() -> Tool {
    create_base_tool(
        REGISTER_CHALLENGER,
        "Register yourself as the Challenger. The Challenger throws insults first.",
        empty_input_schema(),
    )
}

pub fn tool_register_as_defender() -> Tool {
    create_base_tool(
        REGISTER_DEFENDER,
        "Register yourself as the Defender. The Defender responds to insults with comebacks.",
        empty_input_schema(),
    )
}

pub fn tool_get_duel_state() -> Tool {
    create_base_tool(
        GET_DUEL_STATE,
        "Get the current state of the duel. Shows whose turn it is, scores, and any pending insult.",
        empty_input_schema(),
    )
}

pub fn tool_list_insults() -> Tool {
    create_base_tool(
        LIST_INSULTS,
        "List all available insults you can use. In classic mode, you must use one of these exact insults.",
        empty_input_schema(),
    )
}

pub fn tool_throw_insult() -> Tool {
    create_base_tool(
        THROW_INSULT,
        "Throw an insult at your opponent! You must be the current attacker and use a valid insult from the classic list.",
        string_param_schema(ARG_INSULT, "The insult to throw at your opponent"),
    )
}

pub fn tool_respond() -> Tool {
    create_base_tool(
        RESPOND,
        "Respond to an insult with a witty comeback! If your comeback matches the correct response, you parry and become the attacker.",
        string_param_schema(ARG_COMEBACK, "Your witty comeback to parry the insult"),
    )
}

pub fn tool_get_hint() -> Tool {
    create_base_tool(
        GET_HINT,
        "Get a hint for the current pending insult. Returns the first few characters of the correct comeback.",
        empty_input_schema(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rejects_excessive_input_length_efficiently() {
        let long_string = "a".repeat(crate::arena::MAX_INPUT_LENGTH + 1);
        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), json!(long_string));

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
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
        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), json!(12345));

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params).unwrap();
        match result {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(insult, "");
            }
            _ => panic!("Expected ThrowInsult"),
        }
    }

    #[test]
    fn handles_missing_argument_as_empty_string() {
        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(serde_json::Map::new()),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params).unwrap();
        match result {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(insult, "");
            }
            _ => panic!("Expected ThrowInsult"),
        }
    }
}
