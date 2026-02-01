//! MCP server implementation with tool handlers and turn notifications.

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
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use crate::duel::{Duel, DuelState, Duelist, InsultError};

/// Tracks which session is playing which role.
#[derive(Debug, Default)]
struct DuelSessions {
    challenger: Option<String>,
    defender: Option<String>,
}

/// MCP server for insult sword fighting with turn notifications.
#[derive(Clone)]
pub struct InsultServer {
    duel: Arc<Mutex<Option<Duel>>>,
    sessions: Arc<Mutex<DuelSessions>>,
    runtime: Arc<RwLock<Option<Arc<HyperRuntime>>>>,
}

impl InsultServer {
    /// Creates a new insult server.
    #[must_use]
    pub fn new() -> Self {
        Self {
            duel: Arc::new(Mutex::new(None)),
            sessions: Arc::new(Mutex::new(DuelSessions::default())),
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

/// Response from the insult server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelResponse {
    /// Whether the action succeeded.
    pub success: bool,
    /// Human-readable message about what happened.
    pub message: String,
    /// Current state of the duel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DuelStateView>,
    /// Your role in this duel (if registered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub your_role: Option<String>,
}

/// Serializable view of the duel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelStateView {
    /// Current phase of the duel.
    pub phase: String,
    /// Who should act next (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_to_act: Option<String>,
    /// The pending insult waiting for a comeback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_insult: Option<String>,
    /// Challenger's score.
    pub challenger_score: u8,
    /// Defender's score.
    pub defender_score: u8,
    /// Wins needed to win the duel.
    pub wins_needed: u8,
    /// The winner (if duel is over).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner: Option<String>,
}

impl DuelResponse {
    fn success(message: impl Into<String>, state: DuelStateView) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: None,
        }
    }

    fn success_with_role(message: impl Into<String>, state: DuelStateView, role: &str) -> Self {
        Self {
            success: true,
            message: message.into(),
            state: Some(state),
            your_role: Some(role.to_string()),
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            state: None,
            your_role: None,
        }
    }

    fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| "Error serializing response".to_string())
    }
}

fn duel_state_view(duel: &Duel) -> DuelStateView {
    let (phase, next_to_act, winner) = match duel.state() {
        DuelState::AwaitingInsult { attacker } => (
            "awaiting_insult".to_string(),
            Some(attacker.to_string()),
            None,
        ),
        DuelState::AwaitingComeback { attacker } => (
            "awaiting_comeback".to_string(),
            Some(attacker.opponent().to_string()),
            None,
        ),
        DuelState::Finished { winner } => ("finished".to_string(), None, Some(winner.to_string())),
    };

    let (challenger_score, defender_score) = duel.scores();

    DuelStateView {
        phase,
        next_to_act,
        pending_insult: duel.pending_insult().map(String::from),
        challenger_score,
        defender_score,
        wins_needed: 3,
        winner,
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

        let mut duel_guard = self.duel.lock().await;
        let duel = Duel::new();
        let view = duel_state_view(&duel);
        *duel_guard = Some(duel);

        // Clear session registrations for new duel
        let mut sessions = self.sessions.lock().await;
        *sessions = DuelSessions::default();

        // Broadcast initial state
        self.broadcast_turn_notification(&view).await;

        DuelResponse::success(
            "En garde! A new duel begins. Challenger throws the first insult!",
            view,
        )
        .to_json()
    }

    async fn handle_register_as_challenger(&self, session_id: Option<String>) -> String {
        let mut sessions = self.sessions.lock().await;

        if sessions.challenger.is_some() {
            return DuelResponse::error("Challenger role is already taken!").to_json();
        }

        let session = session_id.unwrap_or_else(|| "unknown".to_string());
        sessions.challenger = Some(session.clone());
        info!("🎭 Session {} registered as Challenger", session);

        let duel_guard = self.duel.lock().await;
        let state = duel_guard.as_ref().map(duel_state_view);

        state.map_or_else(
            || {
                json!({
                    "success": true,
                    "message": "Registered as Challenger. Call start_duel to begin!",
                    "your_role": "Challenger"
                })
                .to_string()
            },
            |state| {
                DuelResponse::success_with_role(
                    "You are now the Challenger! Throw the first insult when ready.",
                    state,
                    "Challenger",
                )
                .to_json()
            },
        )
    }

    async fn handle_register_as_defender(&self, session_id: Option<String>) -> String {
        let mut sessions = self.sessions.lock().await;

        if sessions.defender.is_some() {
            return DuelResponse::error("Defender role is already taken!").to_json();
        }

        let session = session_id.unwrap_or_else(|| "unknown".to_string());
        sessions.defender = Some(session.clone());
        info!("🎭 Session {} registered as Defender", session);

        let duel_guard = self.duel.lock().await;
        let state = duel_guard.as_ref().map(duel_state_view);

        state.map_or_else(
            || {
                json!({
                    "success": true,
                    "message": "Registered as Defender. Wait for Challenger to start_duel!",
                    "your_role": "Defender"
                })
                .to_string()
            },
            |state| {
                DuelResponse::success_with_role(
                    "You are now the Defender! Wait for an insult, then respond with a comeback.",
                    state,
                    "Defender",
                )
                .to_json()
            },
        )
    }

    async fn get_role_for_session(&self, session_id: Option<&String>) -> Option<Duelist> {
        let sessions = self.sessions.lock().await;
        let session = session_id?;

        if sessions.challenger.as_ref() == Some(session) {
            Some(Duelist::Challenger)
        } else if sessions.defender.as_ref() == Some(session) {
            Some(Duelist::Defender)
        } else {
            None
        }
    }

    async fn handle_get_duel_state(&self, session_id: Option<String>) -> String {
        let duel_guard = self.duel.lock().await;
        let Some(duel) = duel_guard.as_ref() else {
            return DuelResponse::error("No duel in progress. Call start_duel first!").to_json();
        };

        let view = duel_state_view(duel);
        let role = self.get_role_for_session(session_id.as_ref()).await;

        if let Some(role) = role {
            DuelResponse::success_with_role("Current duel state:", view, &role.to_string())
                .to_json()
        } else {
            DuelResponse::success("Current duel state:", view).to_json()
        }
    }

    async fn handle_list_insults(&self) -> String {
        let duel_guard = self.duel.lock().await;
        let Some(duel) = duel_guard.as_ref() else {
            return DuelResponse::error("No duel in progress. Call start_duel first!").to_json();
        };

        let insults: Vec<&str> = duel
            .insult_bank()
            .all_pairs()
            .iter()
            .map(|p| p.insult)
            .collect();

        json!({
            "success": true,
            "message": "Available insults for the duel:",
            "insults": insults
        })
        .to_string()
    }

    async fn handle_throw_insult(&self, insult: String) -> String {
        let mut duel_guard = self.duel.lock().await;
        let Some(duel) = duel_guard.as_mut() else {
            return DuelResponse::error("No duel in progress. Call start_duel first!").to_json();
        };

        match duel.throw_insult(insult.clone()) {
            Ok(()) => {
                let view = duel_state_view(duel);
                info!("🗣️  INSULT: \"{}\"", insult);

                // Broadcast turn notification
                drop(duel_guard); // Release lock before broadcast
                self.broadcast_turn_notification(&view).await;

                DuelResponse::success(
                    format!("You hurl the insult: \"{insult}\" - awaiting comeback!"),
                    view,
                )
                .to_json()
            }
            Err(InsultError::UnknownInsult(insult)) => {
                warn!("❌ Unknown insult attempted: \"{}\"", insult);
                DuelResponse::error(format!(
                    "Unknown insult: \"{insult}\". Use list_insults to see valid options."
                ))
                .to_json()
            }
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
        }
    }

    async fn handle_respond(&self, comeback: String) -> String {
        let mut duel_guard = self.duel.lock().await;
        let Some(duel) = duel_guard.as_mut() else {
            return DuelResponse::error("No duel in progress. Call start_duel first!").to_json();
        };

        // Log the exact comeback for debugging
        info!("💬 COMEBACK ATTEMPT: {:?}", comeback);
        if let Some(insult) = duel.pending_insult() {
            info!("   For insult: {:?}", insult);
            if let Some(correct) = duel.insult_bank().find_comeback(insult) {
                info!("   Expected: {:?}", correct);
                info!("   Match: {}", comeback.eq_ignore_ascii_case(correct));
            }
        }

        match duel.respond(comeback.clone()) {
            Ok(exchange) => {
                let view = duel_state_view(duel);
                let (challenger, defender) = duel.scores();
                let is_finished = duel.is_finished();

                info!("💬 COMEBACK: \"{}\"", comeback);

                let message = if exchange.result.is_parried() {
                    info!("   ✨ PARRIED! {} wins the exchange!", exchange.winner);
                    if is_finished {
                        info!(
                            "🏆 DUEL OVER! {} WINS! (Score: {}-{})",
                            exchange.winner, challenger, defender
                        );
                        format!("TOUCHÉ! Perfect parry! {} wins the duel!", exchange.winner)
                    } else {
                        info!(
                            "   Score: Challenger {} - {} Defender",
                            challenger, defender
                        );
                        format!(
                            "TOUCHÉ! Perfect parry! {} wins the exchange and attacks next!",
                            exchange.winner
                        )
                    }
                } else {
                    info!("   ❌ FAILED! {} wins the exchange!", exchange.winner);

                    // Get expected comeback for better error message
                    let expected =
                        if let crate::duel::ExchangeResult::Failed { ref correct, .. } =
                            exchange.result
                        {
                            correct.clone()
                        } else {
                            String::new()
                        };

                    if is_finished {
                        info!(
                            "🏆 DUEL OVER! {} WINS! (Score: {}-{})",
                            exchange.winner, challenger, defender
                        );
                        format!(
                            "You failed to parry! {} wins the duel!\n\nExpected comeback: \"{}\"",
                            exchange.winner, expected
                        )
                    } else {
                        info!(
                            "   Score: Challenger {} - {} Defender",
                            challenger, defender
                        );
                        format!(
                            "You failed to parry! {} wins the exchange and attacks again!\n\nExpected comeback: \"{}\"",
                            exchange.winner, expected
                        )
                    }
                };

                // Broadcast turn notification (unless duel is over)
                if !is_finished {
                    drop(duel_guard); // Release lock before broadcast
                    self.broadcast_turn_notification(&view).await;
                }

                DuelResponse::success(message, view).to_json()
            }
            Err(e) => DuelResponse::error(e.to_string()).to_json(),
        }
    }

    async fn handle_get_hint(&self) -> String {
        let duel_guard = self.duel.lock().await;
        let Some(duel) = duel_guard.as_ref() else {
            return DuelResponse::error("No duel in progress. Call start_duel first!").to_json();
        };

        let Some(insult) = duel.pending_insult() else {
            return DuelResponse::error("No pending insult to hint about.").to_json();
        };

        let Some(comeback) = duel.insult_bank().find_comeback(insult) else {
            return DuelResponse::error("Could not find comeback for this insult.").to_json();
        };

        // Give first 20 characters as hint
        let hint: String = comeback.chars().take(20).collect();

        json!({
            "success": true,
            "message": "Here's a hint for the comeback:",
            "hint": format!("{}...", hint),
            "insult": insult
        })
        .to_string()
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

    #[tokio::test]
    async fn list_insults_returns_all_insults() {
        let server = InsultServer::new();
        server.handle_start_duel().await;
        let response = server.handle_list_insults().await;
        assert!(response.contains("dairy farmer"));
    }

    #[tokio::test]
    async fn full_exchange_flow() {
        let server = InsultServer::new();
        server.handle_start_duel().await;

        // Throw insult
        let throw_response = server
            .handle_throw_insult("You fight like a dairy farmer!".to_string())
            .await;
        assert!(throw_response.contains("awaiting comeback"));

        // Correct comeback
        let respond_response = server
            .handle_respond("How appropriate. You fight like a cow!".to_string())
            .await;
        assert!(respond_response.contains("TOUCHÉ"));
        assert!(respond_response.contains("Defender"));
    }

    #[tokio::test]
    async fn register_roles() {
        let server = InsultServer::new();

        let response = server
            .handle_register_as_challenger(Some("session1".to_string()))
            .await;
        assert!(response.contains("Challenger"));

        let response = server
            .handle_register_as_defender(Some("session2".to_string()))
            .await;
        assert!(response.contains("Defender"));

        // Can't register twice
        let response = server
            .handle_register_as_challenger(Some("session3".to_string()))
            .await;
        assert!(response.contains("already taken"));
    }

    #[tokio::test]
    async fn beggar_manners_insult_exchange_works() {
        let server = InsultServer::new();
        server.handle_start_duel().await;

        // Throw "beggar manners" insult
        let throw_response = server
            .handle_throw_insult("You have the manners of a beggar.".to_string())
            .await;
        assert!(
            throw_response.contains("awaiting comeback"),
            "Should be waiting for comeback"
        );

        // Respond with correct comeback
        let respond_response = server
            .handle_respond("I wanted to make sure you'd feel comfortable with me.".to_string())
            .await;
        assert!(
            respond_response.contains("TOUCHÉ"),
            "Should parry successfully"
        );
        assert!(respond_response.contains("Defender"), "Defender should win");
    }

    #[tokio::test]
    async fn throw_insult_without_duel_returns_error() {
        let server = InsultServer::new();
        let response = server.handle_throw_insult("foo".to_string()).await;
        assert!(response.contains("No duel in progress"));
    }

    #[tokio::test]
    async fn respond_without_duel_returns_error() {
        let server = InsultServer::new();
        let response = server.handle_respond("bar".to_string()).await;
        assert!(response.contains("No duel in progress"));
    }

    #[tokio::test]
    async fn state_mismatch_errors_are_reported() {
        let server = InsultServer::new();
        server.handle_start_duel().await;

        // 1. Throw insult -> OK
        server
            .handle_throw_insult("You fight like a dairy farmer!".to_string())
            .await;

        // 2. Throw insult AGAIN -> Error (Waiting for comeback)
        let response = server
            .handle_throw_insult("You fight like a dairy farmer!".to_string())
            .await;
        assert!(
            response.contains("Waiting for a comeback"),
            "Should error when throwing insult while awaiting comeback"
        );

        // 3. Respond -> OK (Parried, Defender becomes attacker)
        server
            .handle_respond("How appropriate. You fight like a cow!".to_string())
            .await;

        // 4. Respond AGAIN -> Error (Waiting for insult)
        let response = server.handle_respond("Too late".to_string()).await;
        assert!(
            response.contains("Waiting for an insult"),
            "Should error when responding while awaiting insult"
        );
    }

    #[tokio::test]
    async fn actions_after_duel_finished_return_error() {
        let server = InsultServer::new();
        server.handle_start_duel().await;

        // Win the duel (Challenger wins 3 times)
        for _ in 0..3 {
            server
                .handle_throw_insult("You fight like a dairy farmer!".to_string())
                .await;
            server.handle_respond("wrong".to_string()).await;
        }

        // Duel should be finished
        let state = server.handle_get_duel_state(None).await;
        assert!(state.contains("finished"));

        // Throw insult -> Error
        let response = server
            .handle_throw_insult("You fight like a dairy farmer!".to_string())
            .await;
        assert!(response.contains("duel is over"));

        // Respond -> Error
        let response = server.handle_respond("wrong".to_string()).await;
        assert!(response.contains("duel is over"));
    }
}
