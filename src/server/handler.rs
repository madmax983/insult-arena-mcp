//! Tool handler trait definition.

use async_trait::async_trait;
use rust_mcp_sdk::schema::Tool;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use serde_json::Value;

use crate::arena::SessionId;
use crate::server::{DuelResponse, InsultServer};

/// Trait for handling tool execution.
///
/// Implementors of this trait define both the schema (via `tool_def`)
/// and the execution logic (via `execute`) for a specific tool.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Returns the unique name of the tool.
    fn name(&self) -> &'static str;

    /// Returns the full tool definition (schema, description).
    fn tool_def(&self) -> Tool;

    /// Executes the tool logic.
    ///
    /// # Arguments
    ///
    /// * `server` - The server instance (provides access to Arena state).
    /// * `session_id` - The ID of the session calling the tool.
    /// * `params` - The raw JSON arguments for the tool call.
    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        params: Value,
    ) -> Result<DuelResponse, CallToolError>;
}
