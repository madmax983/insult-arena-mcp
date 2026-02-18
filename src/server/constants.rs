//! Shared constants for MCP tool definitions and action parsing.
//!
//! Centralizing these strings prevents "magic string" errors and ensures consistency
//! between the tool schema definitions and the action parser.

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
