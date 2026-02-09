//! Tool definitions for the Insult Arena MCP server.
//!
//! This module defines the schemas for the tools exposed to MCP clients (LLMs).
//! Each tool corresponds to an action the AI can take in the game, such as
//! starting a duel, throwing an insult, or responding with a comeback.
//!
//! # Schema Examples
//!
//! The `throw_insult` tool appears to the LLM like this:
//!
//! ```json
//! {
//!   "name": "throw_insult",
//!   "description": "Throw an insult at your opponent!...",
//!   "inputSchema": {
//!     "type": "object",
//!     "properties": {
//!       "insult": {
//!         "type": "string",
//!         "description": "The insult to throw at your opponent"
//!       }
//!     },
//!     "required": ["insult"]
//!   }
//! }
//! ```

use rust_mcp_sdk::schema::{Tool, ToolInputSchema};
use serde_json::json;
use std::collections::HashMap;

/// Helper to create an empty input schema (for tools with no arguments).
///
/// # Examples
///
/// ```rust,ignore
/// let schema = empty_input_schema();
/// assert!(schema.properties.is_none());
/// ```
pub fn empty_input_schema() -> ToolInputSchema {
    ToolInputSchema::new(vec![], None, None)
}

/// Helper to create an input schema with a single required string parameter.
///
/// # Examples
///
/// ```rust,ignore
/// let schema = string_param_schema("insult", "The insult text");
/// assert!(schema.properties.unwrap().contains_key("insult"));
/// ```
pub fn string_param_schema(name: &str, description: &str) -> ToolInputSchema {
    let mut props = HashMap::new();
    let mut prop_map = serde_json::Map::new();
    prop_map.insert("type".to_string(), json!("string"));
    prop_map.insert("description".to_string(), json!(description));
    props.insert(name.to_string(), prop_map);

    ToolInputSchema::new(vec![name.to_string()], Some(props), None)
}

/// Tool: `start_duel`
///
/// Starts a new duel, resetting the state and waiting for registrations.
pub fn tool_start_duel() -> Tool {
    Tool {
        name: "start_duel".to_string(),
        description: Some("Start a new insult sword fighting duel! The Challenger throws the first insult. First to 3 exchange wins takes the duel.".to_string()),
        input_schema: empty_input_schema(),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `register_as_challenger`
///
/// Registers the calling session as the Challenger.
pub fn tool_register_as_challenger() -> Tool {
    Tool {
        name: "register_as_challenger".to_string(),
        description: Some(
            "Register yourself as the Challenger. The Challenger throws insults first.".to_string(),
        ),
        input_schema: empty_input_schema(),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `register_as_defender`
///
/// Registers the calling session as the Defender.
pub fn tool_register_as_defender() -> Tool {
    Tool {
        name: "register_as_defender".to_string(),
        description: Some(
            "Register yourself as the Defender. The Defender responds to insults with comebacks."
                .to_string(),
        ),
        input_schema: empty_input_schema(),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `get_duel_state`
///
/// Returns the full state of the current duel.
pub fn tool_get_duel_state() -> Tool {
    Tool {
        name: "get_duel_state".to_string(),
        description: Some("Get the current state of the duel. Shows whose turn it is, scores, and any pending insult.".to_string()),
        input_schema: empty_input_schema(),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `list_insults`
///
/// Returns a list of all valid insults in the bank.
pub fn tool_list_insults() -> Tool {
    Tool {
        name: "list_insults".to_string(),
        description: Some("List all available insults you can use. In classic mode, you must use one of these exact insults.".to_string()),
        input_schema: empty_input_schema(),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `throw_insult`
///
/// The action for the attacker to use an insult.
pub fn tool_throw_insult() -> Tool {
    Tool {
        name: "throw_insult".to_string(),
        description: Some("Throw an insult at your opponent! You must be the current attacker and use a valid insult from the classic list.".to_string()),
        input_schema: string_param_schema("insult", "The insult to throw at your opponent"),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `respond`
///
/// The action for the defender to reply with a comeback.
pub fn tool_respond() -> Tool {
    Tool {
        name: "respond".to_string(),
        description: Some("Respond to an insult with a witty comeback! If your comeback matches the correct response, you parry and become the attacker.".to_string()),
        input_schema: string_param_schema("comeback", "Your witty comeback to parry the insult"),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Tool: `get_hint`
///
/// Returns a masked hint for the current required comeback.
pub fn tool_get_hint() -> Tool {
    Tool {
        name: "get_hint".to_string(),
        description: Some("Get a hint for the current pending insult. Returns the first few characters of the correct comeback.".to_string()),
        input_schema: empty_input_schema(),
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
        output_schema: None,
        title: None,
    }
}
