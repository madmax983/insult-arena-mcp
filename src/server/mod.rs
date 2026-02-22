//! MCP server implementation with tool handlers and turn notifications.
//!
//! # Server Architecture
//!
//! The [`InsultServer`] acts as the "Game Master", managing the lifecycle of the duel
//! and mediating between the MCP runtime and the core game logic in [`Arena`].
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
    CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError,
    TextContent,
};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::announcer::Announcer;
use crate::arena::{Arena, ArenaError, ArenaOutcome, SessionId};
use crate::duel::DuelStateView;

pub mod handler;
pub mod json_util;
pub mod notifications;
pub mod response;

use handler::ToolHandler;
use notifications::NotificationManager;
pub use response::DuelResponse;

/// MCP server for insult sword fighting with turn notifications.
///
/// Handles tool execution and state management for the duel.
///
/// # Architecture
///
/// It uses an [`Arc<Mutex<Arena>>`] to share game state safely across async tasks.
/// Notifications are sent via the `HyperRuntime` which is injected after server start.
///
/// It implements `Clone` cheaply (Arc-based), so it can be shared between the
/// Hyper service and the notification system.
///
/// # Example
///
/// ```
/// use insult_arena_mcp::InsultServer;
///
/// let server = InsultServer::new();
/// // The server is now ready to handle MCP requests.
/// ```
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
    pub(crate) arena: Arc<Mutex<Arena>>,
    pub(crate) notifications: Arc<NotificationManager>,
    tools: Arc<HashMap<String, Arc<dyn ToolHandler>>>,
}

impl InsultServer {
    /// Creates a new insult server instance.
    ///
    /// The server starts with no active duel.
    /// Call `start_duel` (via tool) to initialize a new game.
    #[must_use]
    pub fn new() -> Self {
        use tools::{
            GetDuelState, GetHint, ListInsults, RegisterChallenger, RegisterDefender, Respond,
            StartDuel, ThrowInsult,
        };

        let mut tools: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();

        let handlers: Vec<Arc<dyn ToolHandler>> = vec![
            Arc::new(StartDuel),
            Arc::new(RegisterChallenger),
            Arc::new(RegisterDefender),
            Arc::new(GetDuelState),
            Arc::new(ListInsults),
            Arc::new(ThrowInsult),
            Arc::new(Respond),
            Arc::new(GetHint),
        ];

        for handler in handlers {
            tools.insert(handler.name().to_string(), handler);
        }

        Self {
            arena: Arc::new(Mutex::new(Arena::new())),
            notifications: Arc::new(NotificationManager::new()),
            tools: Arc::new(tools),
        }
    }

    /// Sets the `HyperRuntime` for sending notifications.
    /// Call this after `server.start_runtime()` returns.
    pub async fn set_runtime(&self, runtime: Arc<HyperRuntime>) {
        self.notifications.set_runtime(runtime).await;
    }

    /// Helper to execute a state-changing action on the arena.
    ///
    /// Handles locking, error mapping, logging, turn notifications, and response formatting.
    pub(crate) async fn execute_turn_action<F>(&self, context: &str, action: F) -> DuelResponse
    where
        F: FnOnce(&mut Arena) -> Result<(ArenaOutcome, DuelStateView), ArenaError>,
    {
        let mut arena = self.arena.lock().await;
        match action(&mut arena) {
            Ok((outcome, view)) => {
                // Specialized logging based on outcome
                if let ArenaOutcome::InsultThrown { ref insult } = outcome {
                    info!("🗣️  INSULT: {:?}", insult);
                }

                let message = Announcer::announce(&outcome, Some(&view));

                // Log result for completed exchanges
                if let ArenaOutcome::ExchangeProcessed { .. } = outcome {
                    info!("   Result: {:?}", message);
                }

                // Release lock before broadcasting to avoid holding it during network IO
                drop(arena);

                // Only notify if the game is still active
                if view.phase != "finished" {
                    self.notifications.notify_turn(&view).await;
                }

                DuelResponse::success(message, view)
            }
            Err(e) => {
                warn!("❌ {} error: {:?}", context, e);
                DuelResponse::error(e.to_string())
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

#[async_trait]
impl ServerHandler for InsultServer {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        let mut tools_list: Vec<_> = self.tools.values().map(|h| h.tool_def()).collect();
        // Sort for consistent output
        tools_list.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(ListToolsResult {
            tools: tools_list,
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
        let session_id_str = session_id_opt
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Validate session ID immediately
        let session_id = SessionId::try_from(session_id_str).map_err(|e| match e {
            ArenaError::SessionIdTooLong(limit) => CallToolError::invalid_arguments(
                &params.name,
                Some(format!("Session ID too long (max {limit} chars)")),
            ),
            _ => CallToolError::from_message("Unexpected session ID error"),
        })?;

        let tool_name = params.name.clone();
        let args_val = params
            .arguments
            .map_or(serde_json::Value::Null, serde_json::Value::Object);

        let handler = self
            .tools
            .get(&tool_name)
            .ok_or_else(|| CallToolError::unknown_tool(&tool_name))?;

        let response = handler.execute(self, session_id, args_val).await?;

        Ok(CallToolResult {
            content: vec![TextContent::new(response.to_json(), None, None).into()],
            is_error: None,
            meta: None,
            structured_content: None,
        })
    }
}

// Security tests removed from here as they are covered by log_injection_test.rs

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_duel_creates_new_game() {
        let server = InsultServer::new();
        let mock_server = Arc::new(MockMcpServer { session_id: None });
        let params = CallToolRequestParams {
            name: "start_duel".to_string(),
            arguments: None,
            meta: None,
            task: None,
        };
        let result = server
            .handle_call_tool_request(params, mock_server)
            .await
            .unwrap();

        let text = match &result.content[0] {
            rust_mcp_sdk::schema::ContentBlock::TextContent(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("En garde"));
        assert!(text.contains("true")); // success: true
    }

    #[tokio::test]
    async fn get_state_without_duel_returns_error() {
        let server = InsultServer::new();
        let mock_server = Arc::new(MockMcpServer { session_id: None });
        let params = CallToolRequestParams {
            name: "get_duel_state".to_string(),
            arguments: None,
            meta: None,
            task: None,
        };
        let result = server
            .handle_call_tool_request(params, mock_server)
            .await
            .unwrap();

        let text = match &result.content[0] {
            rust_mcp_sdk::schema::ContentBlock::TextContent(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("No duel in progress"));
        assert!(text.contains("false")); // success: false
    }

    use rust_mcp_sdk::auth::AuthInfo;
    use rust_mcp_sdk::error::McpSdkError;
    use rust_mcp_sdk::schema::{
        ClientJsonrpcRequest, ClientMessage, CustomRequest, InitializeRequestParams,
        InitializeResult, MessageFromServer, RequestId, ResultFromClient, ResultFromServer,
        ServerJsonrpcRequest, ServerMessage,
    };
    use rust_mcp_sdk::task_store::TaskStore;
    use std::sync::Arc;
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

        async fn start(self: Arc<Self>) -> Result<(), McpSdkError> {
            unimplemented!()
        }
        async fn set_client_details(&self, _: InitializeRequestParams) -> Result<(), McpSdkError> {
            unimplemented!()
        }
        fn server_info(&self) -> &InitializeResult {
            unimplemented!()
        }
        fn client_info(&self) -> Option<InitializeRequestParams> {
            unimplemented!()
        }
        async fn auth_info(&self) -> tokio::sync::RwLockReadGuard<'_, Option<AuthInfo>> {
            unimplemented!()
        }
        async fn auth_info_cloned(&self) -> Option<AuthInfo> {
            unimplemented!()
        }
        async fn update_auth_info(&self, _: Option<AuthInfo>) {
            unimplemented!()
        }
        async fn wait_for_initialization(&self) {
            unimplemented!()
        }
        fn task_store(
            &self,
        ) -> Option<Arc<dyn TaskStore<ClientJsonrpcRequest, ResultFromServer> + 'static>> {
            unimplemented!()
        }
        fn client_task_store(
            &self,
        ) -> Option<Arc<dyn TaskStore<ServerJsonrpcRequest, ResultFromClient> + 'static>> {
            unimplemented!()
        }
        async fn stderr_message(&self, _: String) -> Result<(), McpSdkError> {
            unimplemented!()
        }
        async fn send(
            &self,
            _: MessageFromServer,
            _: Option<RequestId>,
            _: Option<Duration>,
        ) -> Result<Option<ClientMessage>, McpSdkError> {
            unimplemented!()
        }
        async fn send_batch(
            &self,
            _: Vec<ServerMessage>,
            _: Option<Duration>,
        ) -> Result<Option<Vec<ClientMessage>>, McpSdkError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    #[allow(clippy::expect_used)]
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
        let err = result.expect_err("Should error on long session ID");
        assert!(format!("{err:?}").contains("Session ID too long"));
    }
}
#[cfg(test)]
mod log_injection_test;
