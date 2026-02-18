//! Tool definitions for the Insult Arena MCP server.
//!
//! This module defines the schemas for the tools exposed to MCP clients (LLMs).
//! Each tool corresponds to an action the AI can take in the game, such as
//! starting a duel, throwing an insult, or responding with a comeback.
//!
//! # Tool Usage Guide
//!
//! Clients interact with the game by calling these tools via JSON-RPC.
//!
//! ## 1. Start a Duel
//!
//! To begin, one client calls `start_duel`.
//!
//! ```json
//! // Request
//! {
//!   "jsonrpc": "2.0",
//!   "id": 1,
//!   "method": "tools/call",
//!   "params": {
//!     "name": "start_duel",
//!     "arguments": {}
//!   }
//! }
//! ```
//!
//! ## 2. Throw an Insult
//!
//! The Challenger throws an insult using `throw_insult`.
//!
//! ```json
//! // Request
//! {
//!   "jsonrpc": "2.0",
//!   "id": 2,
//!   "method": "tools/call",
//!   "params": {
//!     "name": "throw_insult",
//!     "arguments": {
//!       "insult": "You fight like a dairy farmer!"
//!     }
//!   }
//! }
//! ```
//!
//! ## 3. Respond with a Comeback
//!
//! The Defender responds using `respond`.
//!
//! ```json
//! // Request
//! {
//!   "jsonrpc": "2.0",
//!   "id": 3,
//!   "method": "tools/call",
//!   "params": {
//!     "name": "respond",
//!     "arguments": {
//!       "comeback": "How appropriate. You fight like a cow!"
//!     }
//!   }
//! }
//! ```
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

use crate::server::constants::{
    ARG_COMEBACK, ARG_INSULT, GET_DUEL_STATE, GET_HINT, LIST_INSULTS, REGISTER_CHALLENGER,
    REGISTER_DEFENDER, RESPOND, START_DUEL, THROW_INSULT,
};
use rust_mcp_sdk::schema::{Tool, ToolInputSchema};
use serde_json::json;
use std::collections::HashMap;

/// Helper to create an empty input schema (for tools with no arguments).
///
/// Reduces boilerplate for simple tools like `start_duel` that require no parameters.
pub fn empty_input_schema() -> ToolInputSchema {
    ToolInputSchema::new(vec![], None, None)
}

/// Helper to create an input schema with a single required string parameter.
///
/// Reduces boilerplate for tools like `throw_insult` that take a single string argument.
pub fn string_param_schema(name: &str, description: &str) -> ToolInputSchema {
    let mut props = HashMap::new();
    let mut prop_map = serde_json::Map::new();
    prop_map.insert("type".to_string(), json!("string"));
    prop_map.insert("description".to_string(), json!(description));
    props.insert(name.to_string(), prop_map);

    ToolInputSchema::new(vec![name.to_string()], Some(props), None)
}

/// Internal helper to create a base tool definition with standard defaults.
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

/// Tool: `start_duel`
///
/// # Definition
///
/// ```json
/// {
///   "name": "start_duel",
///   "description": "Start a new insult sword fighting duel!...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {},
///     "required": []
///   }
/// }
/// ```
pub fn tool_start_duel() -> Tool {
    create_base_tool(
        START_DUEL,
        "Start a new insult sword fighting duel! The Challenger throws the first insult. First to 3 exchange wins takes the duel.",
        empty_input_schema(),
    )
}

/// Tool: `register_as_challenger`
///
/// # Definition
///
/// ```json
/// {
///   "name": "register_as_challenger",
///   "description": "Register yourself as the Challenger...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {},
///     "required": []
///   }
/// }
/// ```
pub fn tool_register_as_challenger() -> Tool {
    create_base_tool(
        REGISTER_CHALLENGER,
        "Register yourself as the Challenger. The Challenger throws insults first.",
        empty_input_schema(),
    )
}

/// Tool: `register_as_defender`
///
/// # Definition
///
/// ```json
/// {
///   "name": "register_as_defender",
///   "description": "Register yourself as the Defender...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {},
///     "required": []
///   }
/// }
/// ```
pub fn tool_register_as_defender() -> Tool {
    create_base_tool(
        REGISTER_DEFENDER,
        "Register yourself as the Defender. The Defender responds to insults with comebacks.",
        empty_input_schema(),
    )
}

/// Tool: `get_duel_state`
///
/// # Definition
///
/// ```json
/// {
///   "name": "get_duel_state",
///   "description": "Get the current state of the duel...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {},
///     "required": []
///   }
/// }
/// ```
pub fn tool_get_duel_state() -> Tool {
    create_base_tool(
        GET_DUEL_STATE,
        "Get the current state of the duel. Shows whose turn it is, scores, and any pending insult.",
        empty_input_schema(),
    )
}

/// Tool: `list_insults`
///
/// # Definition
///
/// ```json
/// {
///   "name": "list_insults",
///   "description": "List all available insults you can use...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {},
///     "required": []
///   }
/// }
/// ```
pub fn tool_list_insults() -> Tool {
    create_base_tool(
        LIST_INSULTS,
        "List all available insults you can use. In classic mode, you must use one of these exact insults.",
        empty_input_schema(),
    )
}

/// Tool: `throw_insult`
///
/// # Definition
///
/// ```json
/// {
///   "name": "throw_insult",
///   "description": "Throw an insult at your opponent!...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {
///       "insult": { "type": "string", "description": "..." }
///     },
///     "required": ["insult"]
///   }
/// }
/// ```
pub fn tool_throw_insult() -> Tool {
    create_base_tool(
        THROW_INSULT,
        "Throw an insult at your opponent! You must be the current attacker and use a valid insult from the classic list.",
        string_param_schema(ARG_INSULT, "The insult to throw at your opponent"),
    )
}

/// Tool: `respond`
///
/// # Definition
///
/// ```json
/// {
///   "name": "respond",
///   "description": "Respond to an insult with a witty comeback!...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {
///       "comeback": { "type": "string", "description": "..." }
///     },
///     "required": ["comeback"]
///   }
/// }
/// ```
pub fn tool_respond() -> Tool {
    create_base_tool(
        RESPOND,
        "Respond to an insult with a witty comeback! If your comeback matches the correct response, you parry and become the attacker.",
        string_param_schema(ARG_COMEBACK, "Your witty comeback to parry the insult"),
    )
}

/// Tool: `get_hint`
///
/// # Definition
///
/// ```json
/// {
///   "name": "get_hint",
///   "description": "Get a hint for the current pending insult...",
///   "inputSchema": {
///     "type": "object",
///     "properties": {},
///     "required": []
///   }
/// }
/// ```
pub fn tool_get_hint() -> Tool {
    create_base_tool(
        GET_HINT,
        "Get a hint for the current pending insult. Returns the first few characters of the correct comeback.",
        empty_input_schema(),
    )
}
