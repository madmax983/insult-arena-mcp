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
//!     assert_eq!(insult.as_str(), "You fight like a dairy farmer!");
//! }
//! ```

use crate::arena::{ArenaError, MAX_INPUT_LENGTH, PlayerInput};
use crate::server::constants::{
    GET_DUEL_STATE, GET_HINT, LIST_INSULTS, REGISTER_CHALLENGER, REGISTER_DEFENDER, RESPOND,
    START_DUEL, THROW_INSULT,
};
use rust_mcp_sdk::schema::CallToolRequestParams;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use serde::de::{DeserializeOwned, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

/// Represents a parsed and validated tool action.
///
/// This enum encapsulates the intent of a client's tool call.
/// It is constructed by parsing `CallToolRequestParams`.
///
/// # Robustness
///
/// Note that string arguments are parsed using `deserialize_lossy_string`,
/// meaning that invalid types (numbers, nulls) are silently converted to empty strings
/// rather than returning a JSON parsing error.
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

/// Helper for lossy string deserialization.
///
/// If the input is a string, it returns it.
/// If the input is anything else (or null), it returns an empty string.
///
/// # Robustness
///
/// This preserves legacy behavior where invalid types were treated as empty strings.
/// This prevents deserialization errors from crashing the request handler when
/// clients send unexpected types (e.g., numbers, nulls).
fn deserialize_lossy_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(LossyStringVisitor)
}

struct LossyStringVisitor;

impl<'de> Visitor<'de> for LossyStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or anything else (which becomes empty string)")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() > MAX_INPUT_LENGTH + 1 {
            let mut end = MAX_INPUT_LENGTH + 1;
            while !v.is_char_boundary(end) {
                end -= 1;
            }
            Ok(v[..end].to_owned())
        } else {
            Ok(v.to_owned())
        }
    }

    fn visit_string<E>(self, mut v: String) -> Result<Self::Value, E> {
        if v.len() > MAX_INPUT_LENGTH + 1 {
            let mut end = MAX_INPUT_LENGTH + 1;
            while !v.is_char_boundary(end) {
                end -= 1;
            }
            v.truncate(end);
        }
        Ok(v)
    }

    fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(String::new())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(String::new())
    }
}

/// Arguments for `throw_insult`.
#[derive(Deserialize, Default)]
struct ThrowInsultArgs {
    #[serde(rename = "insult", deserialize_with = "deserialize_lossy_string")]
    insult: String,
}

/// Arguments for `respond`.
#[derive(Deserialize, Default)]
struct RespondArgs {
    #[serde(rename = "comeback", deserialize_with = "deserialize_lossy_string")]
    comeback: String,
}

impl ToolAction {
    fn map_validation_error(e: &ArenaError, field: &str, tool: &str) -> CallToolError {
        match e {
            ArenaError::InputTooLong(limit) => CallToolError::invalid_arguments(
                tool,
                Some(format!("{field} too long (max {limit} chars)")),
            ),
            _ => CallToolError::from_message(e.to_string()),
        }
    }

    /// Helper to parse arguments or fall back to default if parsing fails.
    ///
    /// This ensures we don't crash on malformed JSON structure (e.g. array instead of object),
    /// though `serde_json::from_value` usually handles type mismatches if strict types aren't used.
    fn parse_args_or_default<T: DeserializeOwned + Default>(value: serde_json::Value) -> T {
        serde_json::from_value(value).unwrap_or_else(|_| T::default())
    }
}

impl TryFrom<CallToolRequestParams> for ToolAction {
    type Error = CallToolError;

    fn try_from(params: CallToolRequestParams) -> Result<Self, Self::Error> {
        // ⚡ Bolt Optimization: Take ownership of arguments to avoid string cloning.
        // Convert Option<Map> to Value::Object (or Null) for serde parsing.
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
                let args: ThrowInsultArgs = Self::parse_args_or_default(args_val);

                let insult = PlayerInput::try_from(args.insult)
                    .map_err(|e| Self::map_validation_error(&e, "Insult", &tool_name))?;

                Ok(Self::ThrowInsult { insult })
            }

            RESPOND => {
                let args: RespondArgs = Self::parse_args_or_default(args_val);

                let comeback = PlayerInput::try_from(args.comeback)
                    .map_err(|e| Self::map_validation_error(&e, "Comeback", &tool_name))?;

                Ok(Self::Respond { comeback })
            }

            _ => Err(CallToolError::unknown_tool(&tool_name)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::server::constants::{ARG_INSULT, THROW_INSULT};
    use serde_json::json;

    #[test]
    fn visitor_truncates_long_input() {
        let limit = crate::arena::MAX_INPUT_LENGTH;
        let long_string = "a".repeat(limit + 100);

        // Test visit_str (via &str)
        let visitor = LossyStringVisitor;
        let result = visitor
            .visit_str::<serde::de::value::Error>(long_string.as_str())
            .unwrap();
        assert_eq!(
            result.len(),
            limit + 1,
            "visit_str should truncate to limit + 1"
        );

        // Test visit_string (via String)
        let visitor = LossyStringVisitor;
        let result = visitor
            .visit_string::<serde::de::value::Error>(long_string)
            .unwrap();
        assert_eq!(
            result.len(),
            limit + 1,
            "visit_string should truncate to limit + 1"
        );
    }

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
        // Test that non-string arguments (number, null, missing) are treated as empty strings
        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), json!(12345)); // Number instead of string

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
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

    #[test]
    fn handles_missing_argument_as_empty_string() {
        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(serde_json::Map::new()), // Empty args
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params).unwrap();
        match result {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(
                    insult.as_str(),
                    "",
                    "Missing argument should become empty string"
                );
            }
            _ => panic!("Expected ThrowInsult"),
        }
    }

    #[test]
    fn handles_deeply_nested_structure_gracefully() {
        // Create a deeply nested structure that would cause large allocation
        // if fully deserialized into Value.
        // e.g., [[[[[[...]]]]]]
        let mut nested = json!([]);
        for _ in 0..100 {
            nested = json!([nested]);
        }

        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), nested);

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params);
        assert!(result.is_ok(), "Should not fail on nested structure");

        match result.unwrap() {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(insult.as_str(), "", "Should return empty string for array");
            }
            _ => panic!("Expected ThrowInsult action"),
        }
    }

    #[test]
    fn handles_complex_object_gracefully() {
        // Create a large object structure
        let complex = json!({
            "foo": "bar",
            "nested": {
                "a": [1, 2, 3],
                "b": { "x": "y" }
            }
        });

        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), complex);

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params);
        assert!(result.is_ok());

        match result.unwrap() {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(insult.as_str(), "", "Should return empty string for object");
            }
            _ => panic!("Expected ThrowInsult action"),
        }
    }

    #[test]
    fn handles_null_gracefully() {
        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), serde_json::Value::Null);

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params);
        assert!(result.is_ok());

        match result.unwrap() {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(insult.as_str(), "", "Should return empty string for null");
            }
            _ => panic!("Expected ThrowInsult action"),
        }
    }

    #[test]
    fn handles_boolean_gracefully() {
        let mut args = serde_json::Map::new();
        args.insert(ARG_INSULT.to_string(), json!(true));

        let params = CallToolRequestParams {
            name: THROW_INSULT.to_string(),
            arguments: Some(args),
            meta: None,
            task: None,
        };

        let result = ToolAction::try_from(params);
        assert!(result.is_ok());

        match result.unwrap() {
            ToolAction::ThrowInsult { insult } => {
                assert_eq!(
                    insult.as_str(),
                    "",
                    "Should return empty string for boolean"
                );
            }
            _ => panic!("Expected ThrowInsult action"),
        }
    }
}
