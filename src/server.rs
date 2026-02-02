//! MCP server implementation with tool handlers and turn notifications.
//!
//! # Server Architecture
//!
//! The `InsultServer` manages the state of a single duel and handles
//! interactions from multiple clients (sessions).
//!
//! ## State Management
//!
//! - **Arena**: Encapsulates game logic and state in `Arc<Mutex<Arena>>`.
//! - **Notifications**: Uses `HyperRuntime` to broadcast turn notifications to all connected clients.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::mcp_server::hyper_runtime::HyperRuntime;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, CustomNotification, ListToolsResult,
    PaginatedRequestParams, RpcError, TextContent, Tool, ToolInputSchema,
};
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::arena::{Arena, DuelResponse, DuelStateView};

/// MCP server for insult sword fighting with turn notifications.
///
/// Handles tool execution and state management for the duel.
#[derive(Clone)]
pub struct InsultServer {
    arena: Arc<Mutex<Arena>>,
    runtime: Arc<RwLock<Option<Arc<HyperRuntime>>>>,
}

impl InsultServer {
    /// Creates a new insult server instance.
    ///
    /// The server starts with no active duel.
    /// Call `start_duel` (via tool) to initialize a new game.
    #[must_use]
    pub fn new() -> Self {
        Self {
            arena: Arc::new(Mutex::new(Arena::new())),
            runtime: Arc::new(RwLock::new(None)),
        }
    }

    /// Sets the `HyperRuntime` for sending notifications.
    /// Call this after `server.start_runtime()` returns.
    pub async fn set_runtime(&self, runtime: Arc<HyperRuntime>) {
        let mut guard = self.runtime.write().await;
        *guard = Some(runtime);
    }

    /// Broadcasts a turn notification to all connected sessions.
    ///
    /// Sends a JSON-RPC notification with method `notifications/turn`.
    /// The payload contains:
    /// - `type`: "`turn_notification`"
    /// - `state`: The current `DuelStateView`
    /// - `message`: A human-readable message indicating whose turn it is.
    async fn broadcast_turn_notification(&self, state: &DuelStateView) {
        let runtime_guard: tokio::sync::RwLockReadGuard<'_, Option<Arc<HyperRuntime>>> =
            self.runtime.read().await;
        let Some(runtime) = runtime_guard.as_ref() else {
            warn!("⚠️  Cannot send notification: Runtime not initialized");
            return;
        };

        let sessions = runtime.sessions().await;
        info!("📡 Active sessions: {} connected", sessions.len());

        if sessions.is_empty() {
            info!("   No active sessions to notify");
            return;
        }

        // Build params as a Map<String, Value>
        let mut params = serde_json::Map::new();
        params.insert("type".to_string(), json!("turn_notification"));
        params.insert("state".to_string(), json!(state));
        params.insert(
            "message".to_string(),
            json!(format!(
                "It's {}'s turn!",
                state.next_to_act.as_deref().unwrap_or("unknown")
            )),
        );

        let notification = CustomNotification {
            method: "notifications/turn".to_string(),
            params: Some(params),
        };

        info!(
            "📢 Broadcasting to {} session(s): {}'s turn",
            sessions.len(),
            state.next_to_act.as_deref().unwrap_or("unknown")
        );

        for session_id in sessions {
            info!("   → Sending to session: {}", session_id);
            if let Err(e) = runtime
                .notify_custom(&session_id, notification.clone())
                .await
            {
                warn!("   ✗ Failed to send notification to {}: {}", session_id, e);
            } else {
                info!("   ✓ Notification sent to {}", session_id);
            }
        }
    }
}

impl Default for InsultServer {
    fn default() -> Self {
        Self::new()
    }
}

// Helper to create empty input schema
fn empty_input_schema() -> ToolInputSchema {
    ToolInputSchema::new(vec![], None, None)
}

// Helper to create input schema with a string parameter
fn string_param_schema(name: &str, description: &str) -> ToolInputSchema {
    let mut props = HashMap::new();
    let mut prop_map = serde_json::Map::new();
    prop_map.insert("type".to_string(), json!("string"));
    prop_map.insert("description".to_string(), json!(description));
    props.insert(name.to_string(), prop_map);

    ToolInputSchema::new(vec![name.to_string()], Some(props), None)
}

// Tool definitions
fn tool_start_duel() -> Tool {
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

fn tool_register_as_challenger() -> Tool {
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

fn tool_register_as_defender() -> Tool {
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

fn tool_get_duel_state() -> Tool {
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

fn tool_list_insults() -> Tool {
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

fn tool_throw_insult() -> Tool {
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

fn tool_respond() -> Tool {
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

fn tool_get_hint() -> Tool {
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

impl InsultServer {
    async fn handle_start_duel(&self) -> String {
        info!("⚔️  NEW DUEL STARTED!");
        info!("   Challenger vs Defender - First to 3 wins!");
        info!("   Challenger attacks first...");

        let mut arena = self.arena.lock().await;
        let (msg, view) = arena.start_duel();

        // Broadcast initial state
        drop(arena);
        self.broadcast_turn_notification(&view).await;

        DuelResponse::success(msg, view).to_json()
    }

    async fn handle_register_as_challenger(&self, session_id: Option<String>) -> String {
        let mut arena = self.arena.lock().await;
        let session = session_id.unwrap_or_else(|| "unknown".to_string());

        match arena.register_challenger(session.clone()) {
            Ok((msg, state)) => {
                info!("🎭 Session {} registered as Challenger", session);
                if let Some(state) = state {
                    DuelResponse::success_with_role(msg, state, "Challenger").to_json()
                } else {
                    json!({
                       "success": true,
                       "message": msg,
                       "your_role": "Challenger"
                    })
                    .to_string()
                }
            }
            Err(e) => DuelResponse::error(e).to_json(),
        }
    }

    async fn handle_register_as_defender(&self, session_id: Option<String>) -> String {
        let mut arena = self.arena.lock().await;
        let session = session_id.unwrap_or_else(|| "unknown".to_string());

        match arena.register_defender(session.clone()) {
            Ok((msg, state)) => {
                info!("🎭 Session {} registered as Defender", session);
                if let Some(state) = state {
                    DuelResponse::success_with_role(msg, state, "Defender").to_json()
                } else {
                    json!({
                       "success": true,
                       "message": msg,
                       "your_role": "Defender"
                    })
                    .to_string()
                }
            }
            Err(e) => DuelResponse::error(e).to_json(),
        }
    }

    async fn handle_get_duel_state(&self, session_id: Option<String>) -> String {
        let arena = self.arena.lock().await;

        match arena.get_duel_state(session_id.as_deref()) {
            Ok((view, role)) => {
                if let Some(role_name) = role {
                    DuelResponse::success_with_role("Current duel state:", view, &role_name)
                        .to_json()
                } else {
                    DuelResponse::success("Current duel state:", view).to_json()
                }
            }
            Err(e) => DuelResponse::error(e).to_json(),
        }
    }

    async fn handle_list_insults(&self) -> String {
        let arena = self.arena.lock().await;
        match arena.list_insults() {
            Ok(insults) => json!({
                "success": true,
                "message": "Available insults for the duel:",
                "insults": insults
            })
            .to_string(),
            Err(e) => DuelResponse::error(e).to_json(),
        }
    }

    async fn handle_throw_insult(&self, insult: String) -> String {
        let mut arena = self.arena.lock().await;

        match arena.throw_insult(&insult) {
            Ok((msg, view)) => {
                info!("🗣️  INSULT: \"{}\"", insult);
                drop(arena); // Release lock before broadcast
                self.broadcast_turn_notification(&view).await;

                DuelResponse::success(msg, view).to_json()
            }
            Err(e) => {
                warn!("❌ Insult error: \"{}\"", e);
                DuelResponse::error(e).to_json()
            }
        }
    }

    async fn handle_respond(&self, comeback: String) -> String {
        let mut arena = self.arena.lock().await;

        info!("💬 COMEBACK ATTEMPT: {:?}", comeback);

        match arena.respond(&comeback) {
            Ok((msg, view)) => {
                info!("   Result: {}", msg);
                let is_finished = view.phase == "finished";

                // Broadcast turn notification (unless duel is over)
                if !is_finished {
                    drop(arena); // Release lock before broadcast
                    self.broadcast_turn_notification(&view).await;
                }

                DuelResponse::success(msg, view).to_json()
            }
            Err(e) => DuelResponse::error(e).to_json(),
        }
    }

    async fn handle_get_hint(&self) -> String {
        let arena = self.arena.lock().await;

        match arena.get_hint() {
            Ok((hint, insult)) => json!({
                "success": true,
                "message": "Here's a hint for the comeback:",
                "hint": hint,
                "insult": insult
            })
            .to_string(),
            Err(e) => DuelResponse::error(e).to_json(),
        }
    }
}

#[async_trait]
impl ServerHandler for InsultServer {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: vec![
                tool_start_duel(),
                tool_register_as_challenger(),
                tool_register_as_defender(),
                tool_get_duel_state(),
                tool_list_insults(),
                tool_throw_insult(),
                tool_respond(),
                tool_get_hint(),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        // Get session ID for role tracking
        let session_id = runtime.session_id();

        let result = match params.name.as_str() {
            "start_duel" => self.handle_start_duel().await,
            "register_as_challenger" => self.handle_register_as_challenger(session_id).await,
            "register_as_defender" => self.handle_register_as_defender(session_id).await,
            "get_duel_state" => self.handle_get_duel_state(session_id).await,
            "list_insults" => self.handle_list_insults().await,
            "throw_insult" => {
                let args = params.arguments.unwrap_or_default();
                let insult = args
                    .get("insult")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.handle_throw_insult(insult).await
            }
            "respond" => {
                let args = params.arguments.unwrap_or_default();
                let comeback = args
                    .get("comeback")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.handle_respond(comeback).await
            }
            "get_hint" => self.handle_get_hint().await,
            _ => {
                return Err(CallToolError::unknown_tool(&params.name));
            }
        };

        Ok(CallToolResult {
            content: vec![TextContent::new(result, None, None).into()],
            is_error: None,
            meta: None,
            structured_content: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_duel_creates_new_game() {
        let server = InsultServer::new();
        let response = server.handle_start_duel().await;
        assert!(response.contains("En garde"));
        assert!(response.contains("success"));
    }

    #[tokio::test]
    async fn get_state_without_duel_returns_error() {
        let server = InsultServer::new();
        let response = server.handle_get_duel_state(None).await;
        assert!(response.contains("No duel in progress"));
    }
}
