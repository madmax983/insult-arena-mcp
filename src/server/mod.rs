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
use crate::arena::{Arena, ArenaError, ArenaOutcome};
use crate::duel::{DuelStateView, Duelist};

pub mod action;
pub mod constants;
pub mod notifications;
pub mod response;

use action::ToolAction;
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
    arena: Arc<Mutex<Arena>>,
    notifications: Arc<NotificationManager>,
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
            notifications: Arc::new(NotificationManager::new()),
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
    async fn execute_turn_action<F>(&self, context: &str, action: F) -> DuelResponse
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
use tools::{
    tool_get_duel_state, tool_get_hint, tool_list_insults, tool_register_as_challenger,
    tool_register_as_defender, tool_respond, tool_start_duel, tool_throw_insult,
};

impl InsultServer {
    async fn handle_start_duel(&self) -> DuelResponse {
        info!("⚔️  NEW DUEL STARTED!");
        info!("   Challenger vs Defender - First to 3 wins!");
        info!("   Challenger attacks first...");

        self.execute_turn_action("Start duel", Arena::start_duel)
            .await
    }

    async fn handle_register(&self, role: Duelist, session_id: Option<String>) -> DuelResponse {
        let mut arena = self.arena.lock().await;
        let session = session_id.unwrap_or_else(|| "unknown".to_string());

        match arena.register(role, session.clone()) {
            Ok((outcome, state)) => {
                info!("🎭 Session {:?} registered as {}", session, role);
                DuelResponse::success_with_role(
                    Announcer::announce(&outcome, Some(&state)),
                    state,
                    &role.to_string(),
                )
            }
            Err(e) => DuelResponse::error(e.to_string()),
        }
    }

    async fn handle_register_as_challenger(&self, session_id: Option<String>) -> DuelResponse {
        self.handle_register(Duelist::Challenger, session_id).await
    }

    async fn handle_register_as_defender(&self, session_id: Option<String>) -> DuelResponse {
        self.handle_register(Duelist::Defender, session_id).await
    }

    async fn handle_get_duel_state(&self, session_id: Option<String>) -> DuelResponse {
        let arena = self.arena.lock().await;

        match arena.get_duel_state(session_id.as_deref()) {
            Ok((view, role)) => {
                if let Some(role_name) = role {
                    DuelResponse::success_with_role("Current duel state:", view, &role_name)
                } else {
                    DuelResponse::success("Current duel state:", view)
                }
            }
            Err(e) => DuelResponse::error(e.to_string()),
        }
    }

    async fn handle_list_insults(&self) -> DuelResponse {
        let arena = self.arena.lock().await;
        match arena.list_insults() {
            Ok(insults) => DuelResponse::with_insults(
                "Available insults for the duel:",
                insults.into_iter().map(String::from).collect(),
            ),
            Err(e) => DuelResponse::error(e.to_string()),
        }
    }

    async fn handle_throw_insult(&self, session_id: String, insult: String) -> DuelResponse {
        // ⚡ Bolt Optimization: Pass ownership of 'insult' to Arena to avoid allocation.
        self.execute_turn_action("Insult", |arena| arena.throw_insult(&session_id, insult))
            .await
    }

    async fn handle_respond(&self, session_id: String, comeback: String) -> DuelResponse {
        info!("💬 COMEBACK ATTEMPT: {:?}", comeback);

        // ⚡ Bolt Optimization: Pass ownership of 'comeback' to Arena to avoid allocation.
        self.execute_turn_action("Respond", |arena| arena.respond(&session_id, comeback))
            .await
    }

    async fn handle_get_hint(&self, session_id: String) -> DuelResponse {
        let arena = self.arena.lock().await;

        match arena.get_hint(&session_id) {
            Ok((hint, insult)) => {
                DuelResponse::with_hint("Here's a hint for the comeback:", hint, insult)
            }
            Err(e) => DuelResponse::error(e.to_string()),
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
            // Apply stricter validation if needed, but for now just length check
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

        // Parse and validate the action
        let action = ToolAction::try_from(params)?;

        // Execute the action
        let response = match action {
            ToolAction::StartDuel => self.handle_start_duel().await,
            ToolAction::RegisterChallenger => {
                self.handle_register_as_challenger(session_id_opt.clone())
                    .await
            }
            ToolAction::RegisterDefender => {
                self.handle_register_as_defender(session_id_opt.clone())
                    .await
            }
            ToolAction::GetDuelState => self.handle_get_duel_state(session_id_opt).await,
            ToolAction::ListInsults => self.handle_list_insults().await,
            ToolAction::ThrowInsult { insult } => {
                self.handle_throw_insult(session_id_str, insult).await
            }
            ToolAction::Respond { comeback } => self.handle_respond(session_id_str, comeback).await,
            ToolAction::GetHint => self.handle_get_hint(session_id_str).await,
        };

        Ok(CallToolResult {
            content: vec![TextContent::new(response.to_json(), None, None).into()],
            is_error: None,
            meta: None,
            structured_content: None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod security_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct LogBuffer(Arc<Mutex<Vec<String>>>);

    impl std::io::Write for LogBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let s = String::from_utf8_lossy(buf).to_string();
            self.0.lock().unwrap().push(s);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn log_injection_prevention() {
        let buffer = LogBuffer::default();
        let buffer_clone = buffer.clone();

        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || buffer_clone.clone())
            .with_ansi(false)
            .with_level(false)
            .with_target(false)
            .without_time() // Simplify output for checking
            .finish();

        let _guard = tracing::subscriber::set_default(subscriber);

        let server = InsultServer::new();
        server.handle_start_duel().await;

        // Malicious input with newline injection
        // This will fail validation (unknown insult) but be logged in the error path
        let malicious_insult = "Invalid Insult\nINJECTED_LOG: FAKE_ENTRY";

        let _ = server
            .handle_throw_insult("attacker".to_string(), malicious_insult.to_string())
            .await;

        let logs = buffer.0.lock().unwrap().join("");

        // If vulnerable, the log will contain the literal newline character followed by INJECTED_LOG
        // The format is: ❌ Insult error: "{}"
        // So we expect: ❌ Insult error: "Invalid Insult\nINJECTED_LOG: FAKE_ENTRY"

        // We assert that the log does NOT contain the raw newline inside the message
        // However, since we want to FAIL first if it IS vulnerable, we check for what we DON'T want.

        // Wait, standard practice: Test fails if vulnerability exists?
        // No, standard practice: Test asserts correct behavior. If code is wrong, test fails.
        // Correct behavior: Newline is escaped.
        // So we assert that logs do NOT contain "\nINJECTED_LOG".

        // If vulnerable: logs contain "Invalid Insult\nINJECTED_LOG"
        // If secure (Debug): logs contain "Invalid Insult\\nINJECTED_LOG" (escaped)

        // Let's assert that we see the escaped version, or at least that we DON'T see the raw version acting as a newline.

        println!("Captured logs:\n{logs}");

        // In the vulnerable version, the log line will be split.
        // But since we capture all output into a string, we just look for the sequence.

        // We want to ensure it is ESCAPED.
        // The Debug format {:?} will produce "Invalid Insult\nINJECTED_LOG" -> "Invalid Insult\\nINJECTED_LOG"

        // The error message format is: ❌ Insult error: "{}"
        // If fixed, it becomes: ❌ Insult error: "{:?}" -> ❌ Insult error: "..."
        // Or if I just change {} to {:?}, it becomes: ❌ Insult error: "Error("...")"

        // Wait, e is `ArenaError::UnknownInsult(String)`.
        // ArenaError's Display: "Unknown insult: \"{0}\". Use list_insults to see valid options."
        // So e.to_string() contains the raw insult string inside quotes.

        // Vulnerable: warn!("❌ Insult error: \"{}\"", e);
        // e.to_string() -> Unknown insult: "Invalid Insult\nINJECTED_LOG: FAKE_ENTRY". ...
        // Log output -> ❌ Insult error: "Unknown insult: "Invalid Insult
        // INJECTED_LOG: FAKE_ENTRY". ..."

        // Secure: warn!("❌ Insult error: {:?}", e);
        // e matches ArenaError::UnknownInsult
        // Debug output -> UnknownInsult("Invalid Insult\nINJECTED_LOG: FAKE_ENTRY")
        // Which escapes the inner string.

        // So if secure, we should NOT find "Invalid Insult\nINJECTED_LOG".

        assert!(
            !logs.contains("Invalid Insult\nINJECTED_LOG"),
            "Log injection detected! Newline passed through unescaped."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_duel_creates_new_game() {
        let server = InsultServer::new();
        let response = server.handle_start_duel().await;
        assert!(response.message.contains("En garde"));
        assert!(response.success);
    }

    #[tokio::test]
    async fn get_state_without_duel_returns_error() {
        let server = InsultServer::new();
        let response = server.handle_get_duel_state(None).await;
        assert!(!response.success);
        assert!(response.message.contains("No duel in progress"));
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
