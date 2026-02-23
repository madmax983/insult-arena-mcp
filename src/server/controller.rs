//! Game controller logic for Insult Arena.
//!
//! This module separates the core application logic (Game Controller) from the
//! MCP protocol handling (Server Handler).
//!
//! # Responsibilities
//!
//! - **State Management**: Holds the `Arena` and `NotificationManager`.
//! - **Action Execution**: Executes game actions (`start_duel`, `throw_insult`) and handles side effects.
//! - **Response Formatting**: Converts `ArenaOutcome` into `DuelResponse`.
//! - **Notifications**: Broadcasts turn updates to clients.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use rust_mcp_sdk::mcp_server::hyper_runtime::HyperRuntime;

use crate::announcer::Announcer;
use crate::arena::{Arena, ArenaError, ArenaOutcome, PlayerInput, SessionId};
use crate::duel::{DuelStateView, Duelist};
use crate::server::notifications::NotificationManager;
use crate::server::response::DuelResponse;

/// The Game Controller manages the flow of the game.
///
/// It wraps the `Arena` (state) and `NotificationManager` (broadcasting)
/// and exposes high-level actions that return API-ready responses.
#[derive(Clone)]
pub struct GameController {
    arena: Arc<Mutex<Arena>>,
    notifications: Arc<NotificationManager>,
}

impl GameController {
    /// Creates a new Game Controller.
    #[must_use]
    pub fn new() -> Self {
        Self {
            arena: Arc::new(Mutex::new(Arena::new())),
            notifications: Arc::new(NotificationManager::new()),
        }
    }

    /// Sets the `HyperRuntime` for sending notifications.
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

    /// Starts a new duel.
    pub async fn start_duel(&self) -> DuelResponse {
        info!("⚔️  NEW DUEL STARTED!");
        info!("   Challenger vs Defender - First to 3 wins!");
        info!("   Challenger attacks first...");

        self.execute_turn_action("Start duel", Arena::start_duel)
            .await
    }

    /// Internal helper for registration.
    async fn register(&self, role: Duelist, session_id: SessionId) -> DuelResponse {
        let mut arena = self.arena.lock().await;

        match arena.register(role, session_id.clone()) {
            Ok((outcome, state)) => {
                info!("🎭 Session {:?} registered as {}", session_id, role);
                DuelResponse::success_with_role(
                    Announcer::announce(&outcome, Some(&state)),
                    state,
                    &role.to_string(),
                )
            }
            Err(e) => DuelResponse::error(e.to_string()),
        }
    }

    /// Registers a session as the Challenger.
    pub async fn register_challenger(&self, session_id: SessionId) -> DuelResponse {
        self.register(Duelist::Challenger, session_id).await
    }

    /// Registers a session as the Defender.
    pub async fn register_defender(&self, session_id: SessionId) -> DuelResponse {
        self.register(Duelist::Defender, session_id).await
    }

    /// Gets the current duel state.
    pub async fn get_duel_state(&self, session_id: Option<SessionId>) -> DuelResponse {
        let arena = self.arena.lock().await;

        match arena.get_duel_state(session_id.as_ref()) {
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

    /// Lists available insults.
    pub async fn list_insults(&self) -> DuelResponse {
        let arena = self.arena.lock().await;
        match arena.list_insults() {
            Ok(insults) => DuelResponse::with_insults(
                "Available insults for the duel:",
                insults.into_iter().map(String::from).collect(),
            ),
            Err(e) => DuelResponse::error(e.to_string()),
        }
    }

    /// Throws an insult.
    pub async fn throw_insult(&self, session_id: SessionId, insult: PlayerInput) -> DuelResponse {
        // ⚡ Bolt Optimization: Pass ownership of 'insult' to Arena to avoid allocation.
        self.execute_turn_action("Insult", |arena| arena.throw_insult(&session_id, insult))
            .await
    }

    /// Responds with a comeback.
    pub async fn respond(&self, session_id: SessionId, comeback: PlayerInput) -> DuelResponse {
        info!("💬 COMEBACK ATTEMPT: {:?}", comeback);

        // ⚡ Bolt Optimization: Pass ownership of 'comeback' to Arena to avoid allocation.
        self.execute_turn_action("Respond", |arena| arena.respond(&session_id, comeback))
            .await
    }

    /// Gets a hint for the pending insult.
    pub async fn get_hint(&self, session_id: SessionId) -> DuelResponse {
        let arena = self.arena.lock().await;

        match arena.get_hint(&session_id) {
            Ok((hint, insult)) => {
                DuelResponse::with_hint("Here's a hint for the comeback:", hint, insult)
            }
            Err(e) => DuelResponse::error(e.to_string()),
        }
    }
}

impl Default for GameController {
    fn default() -> Self {
        Self::new()
    }
}
