//! MCP server implementation with tool handlers and turn notifications.

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

use crate::duel::{Duel, Duelist, InsultError};

pub mod tools;
pub mod types;

use tools::{
    tool_get_duel_state, tool_get_hint, tool_list_insults, tool_register_as_challenger,
    tool_register_as_defender, tool_respond, tool_start_duel, tool_throw_insult,
};
use types::{DuelResponse, DuelStateView, duel_state_view};

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

impl Default for InsultServer {
    fn default() -> Self {
        Self::new()
    }
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
            return;
        };

        let sessions = runtime.sessions().await;

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

        for session_id in sessions {
            if let Err(e) = runtime
                .notify_custom(&session_id, notification.clone())
                .await
            {
                warn!("Failed to send notification to {}: {}", session_id, e);
            }
        }

        info!(
            "📢 Broadcasted turn notification: {}'s turn",
            state.next_to_act.as_deref().unwrap_or("unknown")
        );
    }

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
                    if is_finished {
                        info!(
                            "🏆 DUEL OVER! {} WINS! (Score: {}-{})",
                            exchange.winner, challenger, defender
                        );
                        format!("You failed to parry! {} wins the duel!", exchange.winner)
                    } else {
                        info!(
                            "   Score: Challenger {} - {} Defender",
                            challenger, defender
                        );
                        format!(
                            "You failed to parry! {} wins the exchange and attacks again!",
                            exchange.winner
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
            .handle_respond("How appropriate. You fight like a cow.".to_string())
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
}
