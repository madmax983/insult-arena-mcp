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

use std::sync::Arc;

use async_trait::async_trait;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::mcp_server::hyper_runtime::HyperRuntime;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, CustomNotification, ListToolsResult,
    PaginatedRequestParams, RpcError, TextContent,
};
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::announcer::Announcer;
use crate::arena::{Arena, DuelStateView};

pub mod response;
pub use response::DuelResponse;

/// MCP server for insult sword fighting with turn notifications.
///
/// Handles tool execution and state management for the duel.
///
/// # Turn Notifications
///
/// The server implements an autonomous flow where clients are notified when it is
/// their turn to act. This allows two LLM instances to play against each other
/// without human intervention.
///
/// ## Flow
///
/// 1. **Client A** calls a tool (e.g., `throw_insult`).
/// 2. **Arena** processes the action and updates state (e.g., waiting for comeback).
/// 3. **Server** broadcasts `notifications/turn` to all connected clients.
/// 4. **Client B** receives the notification, sees it is their turn, and calls `respond`.
///
/// ## Sequence Diagram
///
/// ```text
/// Challenger             Server              Defender
///     |                    |                    |
///     |--- throw_insult -->|                    |
///     |                    |-- Update State     |
///     |                    |                    |
///     |<-- Turn Notification (Defender's Turn) -|
///     |                    |                    |
///     |                    |<---- respond ------|
///     |-- Update State     |                    |
///     |                    |                    |
///     |- Turn Notification (Challenger's Turn)->|
///     |                    |                    |
/// ```
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

mod tools;
use tools::{
    tool_get_duel_state, tool_get_hint, tool_list_insults, tool_register_as_challenger,
    tool_register_as_defender, tool_respond, tool_start_duel, tool_throw_insult,
};

impl InsultServer {
    async fn handle_start_duel(&self) -> String {
        info!("⚔️  NEW DUEL STARTED!");
        info!("   Challenger vs Defender - First to 3 wins!");
        info!("   Challenger attacks first...");

        let mut arena = self.arena.lock().await;
        let (outcome, view) = arena.start_duel();

        // Broadcast initial state
        drop(arena);
        self.broadcast_turn_notification(&view).await;

        DuelResponse::success(Announcer::announce(&outcome, Some(&view)), view).to_json()
    }

    async fn handle_register_as_challenger(&self, session_id: Option<String>) -> String {
        let mut arena = self.arena.lock().await;
        let session = session_id.unwrap_or_else(|| "unknown".to_string());

        match arena.register_challenger(session.clone()) {
            Ok((outcome, state)) => {
                info!("🎭 Session {:?} registered as Challenger", session);
                state.map_or_else(
                    || {
                        json!({
                           "success": true,
                           "message": Announcer::announce(&outcome, None),
                           "your_role": "Challenger"
                        })
                        .to_string()
                    },
                    |state| {
                        DuelResponse::success_with_role(
                            Announcer::announce(&outcome, Some(&state)),
                            state,
                            "Challenger",
                        )
                        .to_json()
                    },
                )
            }
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
        }
    }

    async fn handle_register_as_defender(&self, session_id: Option<String>) -> String {
        let mut arena = self.arena.lock().await;
        let session = session_id.unwrap_or_else(|| "unknown".to_string());

        match arena.register_defender(session.clone()) {
            Ok((outcome, state)) => {
                info!("🎭 Session {:?} registered as Defender", session);
                state.map_or_else(
                    || {
                        json!({
                           "success": true,
                           "message": Announcer::announce(&outcome, None),
                           "your_role": "Defender"
                        })
                        .to_string()
                    },
                    |state| {
                        DuelResponse::success_with_role(
                            Announcer::announce(&outcome, Some(&state)),
                            state,
                            "Defender",
                        )
                        .to_json()
                    },
                )
            }
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
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
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
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
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
        }
    }

    async fn handle_throw_insult(&self, session_id: String, insult: String) -> String {
        let mut arena = self.arena.lock().await;

        match arena.throw_insult(&session_id, &insult) {
            Ok((outcome, view)) => {
                info!("🗣️  INSULT: {:?}", insult);
                drop(arena); // Release lock before broadcast
                self.broadcast_turn_notification(&view).await;

                DuelResponse::success(Announcer::announce(&outcome, Some(&view)), view).to_json()
            }
            Err(e) => {
                warn!("❌ Insult error: \"{}\"", e);
                DuelResponse::error(e.to_string()).to_json()
            }
        }
    }

    async fn handle_respond(&self, session_id: String, comeback: String) -> String {
        let mut arena = self.arena.lock().await;

        info!("💬 COMEBACK ATTEMPT: {:?}", comeback);

        match arena.respond(&session_id, &comeback) {
            Ok((outcome, view)) => {
                let message = Announcer::announce(&outcome, Some(&view));
                info!("   Result: {}", message);
                let is_finished = view.phase == "finished";

                // Broadcast turn notification (unless duel is over)
                if !is_finished {
                    drop(arena); // Release lock before broadcast
                    self.broadcast_turn_notification(&view).await;
                }

                DuelResponse::success(message, view).to_json()
            }
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
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
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
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
        let session_id_opt = runtime.session_id();

        // Hardening: Validate session ID length before allocation/cloning
        if let Some(ref id) = session_id_opt {
            if id.len() > crate::arena::MAX_SESSION_ID_LENGTH {
                return Err(CallToolError::invalid_arguments(
                    &params.name,
                    Some(format!(
                        "Session ID too long (max {} chars)",
                        crate::arena::MAX_SESSION_ID_LENGTH
                    )),
                ));
            }
        }

        let session_id_str = session_id_opt
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let result = match params.name.as_str() {
            "start_duel" => self.handle_start_duel().await,
            "register_as_challenger" => {
                self.handle_register_as_challenger(session_id_opt.clone())
                    .await
            }
            "register_as_defender" => {
                self.handle_register_as_defender(session_id_opt.clone())
                    .await
            }
            "get_duel_state" => self.handle_get_duel_state(session_id_opt).await,
            "list_insults" => self.handle_list_insults().await,
            "throw_insult" => {
                let args = params.arguments.unwrap_or_default();
                let insult_str = args.get("insult").and_then(|v| v.as_str()).unwrap_or("");

                // Hardening: Validate input length before allocation
                if insult_str.len() > crate::arena::MAX_INPUT_LENGTH {
                    return Err(CallToolError::invalid_arguments(
                        &params.name,
                        Some(format!(
                            "Insult too long (max {} chars)",
                            crate::arena::MAX_INPUT_LENGTH
                        )),
                    ));
                }

                self.handle_throw_insult(session_id_str, insult_str.to_string())
                    .await
            }
            "respond" => {
                let args = params.arguments.unwrap_or_default();
                let comeback_str = args.get("comeback").and_then(|v| v.as_str()).unwrap_or("");

                // Hardening: Validate input length before allocation
                if comeback_str.len() > crate::arena::MAX_INPUT_LENGTH {
                    return Err(CallToolError::invalid_arguments(
                        &params.name,
                        Some(format!(
                            "Comeback too long (max {} chars)",
                            crate::arena::MAX_INPUT_LENGTH
                        )),
                    ));
                }

                self.handle_respond(session_id_str, comeback_str.to_string())
                    .await
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

    use rust_mcp_sdk::schema::{
        InitializeRequestParams, InitializeResult, ClientJsonrpcRequest, ServerJsonrpcRequest,
        ResultFromServer, ResultFromClient, MessageFromServer, RequestId, ClientMessage, ServerMessage,
        CustomRequest,
    };
    use rust_mcp_sdk::auth::AuthInfo;
    use rust_mcp_sdk::error::McpSdkError;
    use rust_mcp_sdk::task_store::TaskStore;
    use std::time::Duration;

    struct MockMcpServer {
        session_id: Option<String>,
    }

    #[async_trait]
    impl McpServer for MockMcpServer {
        fn session_id(&self) -> Option<String> {
            self.session_id.clone()
        }

        async fn notify_custom(&self, _notification: CustomRequest) -> Result<(), McpSdkError> {
            Ok(())
        }

        async fn start(self: Arc<Self>) -> Result<(), McpSdkError> { unimplemented!() }
        async fn set_client_details(&self, _: InitializeRequestParams) -> Result<(), McpSdkError> { unimplemented!() }
        fn server_info(&self) -> &InitializeResult { unimplemented!() }
        fn client_info(&self) -> Option<InitializeRequestParams> { unimplemented!() }
        async fn auth_info(&self) -> tokio::sync::RwLockReadGuard<'_, Option<AuthInfo>> { unimplemented!() }
        async fn auth_info_cloned(&self) -> Option<AuthInfo> { unimplemented!() }
        async fn update_auth_info(&self, _: Option<AuthInfo>) { unimplemented!() }
        async fn wait_for_initialization(&self) { unimplemented!() }
        fn task_store(&self) -> Option<Arc<dyn TaskStore<ClientJsonrpcRequest, ResultFromServer> + 'static>> { unimplemented!() }
        fn client_task_store(&self) -> Option<Arc<dyn TaskStore<ServerJsonrpcRequest, ResultFromClient> + 'static>> { unimplemented!() }
        async fn stderr_message(&self, _: String) -> Result<(), McpSdkError> { unimplemented!() }
        async fn send(&self, _: MessageFromServer, _: Option<RequestId>, _: Option<Duration>) -> Result<Option<ClientMessage>, McpSdkError> { unimplemented!() }
        async fn send_batch(&self, _: Vec<ServerMessage>, _: Option<Duration>) -> Result<Option<Vec<ClientMessage>>, McpSdkError> { unimplemented!() }
    }

    #[tokio::test]
    async fn rejects_excessive_session_id_length() {
        let server = InsultServer::new();
        let long_session_id = "a".repeat(crate::arena::MAX_SESSION_ID_LENGTH + 1);
        let mock_server = Arc::new(MockMcpServer {
            session_id: Some(long_session_id),
        });

        let params = CallToolRequestParams {
            name: "get_duel_state".to_string(),
            arguments: None,
            meta: None,
            task: None,
        };

        let result = server.handle_call_tool_request(params, mock_server).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(format!("{:?}", err).contains("Session ID too long"));
    }
}
