//! Tool implementations using the [`ToolHandler`] trait.
//!
//! This module contains the definitions and execution logic for all tools.
//! By consolidating definition and execution, we ensure high cohesion and reduce
//! the "Shotgun Surgery" smell when adding new tools.

use std::collections::HashMap;

use async_trait::async_trait;
use rust_mcp_sdk::schema::schema_utils::CallToolError;
use rust_mcp_sdk::schema::{Tool, ToolInputSchema};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tracing::info;

use crate::announcer::Announcer;
use crate::arena::{Arena, ArenaError, PlayerInput, SessionId};
use crate::duel::Duelist;
use crate::server::handler::ToolHandler;
use crate::server::json_util::deserialize_lossy_string;
use crate::server::{DuelResponse, InsultServer};

// --- Helper Functions ---

fn empty_input_schema() -> ToolInputSchema {
    ToolInputSchema::new(vec![], None, None)
}

fn string_param_schema(name: &str, description: &str) -> ToolInputSchema {
    let mut props = HashMap::new();
    let mut prop_map = serde_json::Map::new();
    prop_map.insert("type".to_string(), json!("string"));
    prop_map.insert("description".to_string(), json!(description));
    props.insert(name.to_string(), prop_map);

    ToolInputSchema::new(vec![name.to_string()], Some(props), None)
}

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

fn parse_args_or_default<T: DeserializeOwned + Default>(value: Value) -> T {
    serde_json::from_value(value).unwrap_or_else(|_| T::default())
}

fn map_validation_error(e: &ArenaError, field: &str, tool: &str) -> CallToolError {
    match e {
        ArenaError::InputTooLong(limit) => CallToolError::invalid_arguments(
            tool,
            Some(format!("{field} too long (max {limit} chars)")),
        ),
        _ => CallToolError::from_message(e.to_string()),
    }
}

// --- Tool Implementations ---

pub struct StartDuel;

#[async_trait]
impl ToolHandler for StartDuel {
    fn name(&self) -> &'static str {
        "start_duel"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Start a new insult sword fighting duel! The Challenger throws the first insult. First to 3 exchange wins takes the duel.",
            empty_input_schema(),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        _session_id: SessionId,
        _params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        info!("⚔️  NEW DUEL STARTED!");
        info!("   Challenger vs Defender - First to 3 wins!");
        info!("   Challenger attacks first...");

        Ok(server.execute_turn_action("Start duel", Arena::start_duel).await)
    }
}

pub struct RegisterChallenger;

#[async_trait]
impl ToolHandler for RegisterChallenger {
    fn name(&self) -> &'static str {
        "register_as_challenger"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Register yourself as the Challenger. The Challenger throws insults first.",
            empty_input_schema(),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        _params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        handle_register(server, Duelist::Challenger, session_id).await
    }
}

pub struct RegisterDefender;

#[async_trait]
impl ToolHandler for RegisterDefender {
    fn name(&self) -> &'static str {
        "register_as_defender"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Register yourself as the Defender. The Defender responds to insults with comebacks.",
            empty_input_schema(),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        _params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        handle_register(server, Duelist::Defender, session_id).await
    }
}

async fn handle_register(
    server: &InsultServer,
    role: Duelist,
    session_id: SessionId,
) -> Result<DuelResponse, CallToolError> {
    let mut arena = server.arena.lock().await;

    match arena.register(role, session_id.clone()) {
        Ok((outcome, state)) => {
            info!("🎭 Session {:?} registered as {}", session_id, role);
            Ok(DuelResponse::success_with_role(
                Announcer::announce(&outcome, Some(&state)),
                state,
                &role.to_string(),
            ))
        }
        Err(e) => Ok(DuelResponse::error(e.to_string())),
    }
}

pub struct GetDuelState;

#[async_trait]
impl ToolHandler for GetDuelState {
    fn name(&self) -> &'static str {
        "get_duel_state"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Get the current state of the duel. Shows whose turn it is, scores, and any pending insult.",
            empty_input_schema(),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        _params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        let arena = server.arena.lock().await;

        match arena.get_duel_state(Some(&session_id)) {
            Ok((view, role)) => {
                if let Some(role_name) = role {
                    Ok(DuelResponse::success_with_role("Current duel state:", view, &role_name))
                } else {
                    Ok(DuelResponse::success("Current duel state:", view))
                }
            }
            Err(e) => Ok(DuelResponse::error(e.to_string())),
        }
    }
}

pub struct ListInsults;

#[async_trait]
impl ToolHandler for ListInsults {
    fn name(&self) -> &'static str {
        "list_insults"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "List all available insults you can use. In classic mode, you must use one of these exact insults.",
            empty_input_schema(),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        _session_id: SessionId,
        _params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        let arena = server.arena.lock().await;
        match arena.list_insults() {
            Ok(insults) => Ok(DuelResponse::with_insults(
                "Available insults for the duel:",
                insults.into_iter().map(String::from).collect(),
            )),
            Err(e) => Ok(DuelResponse::error(e.to_string())),
        }
    }
}

pub struct ThrowInsult;

#[derive(Deserialize, Default)]
struct ThrowInsultArgs {
    #[serde(rename = "insult", deserialize_with = "deserialize_lossy_string")]
    insult: String,
}

#[async_trait]
impl ToolHandler for ThrowInsult {
    fn name(&self) -> &'static str {
        "throw_insult"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Throw an insult at your opponent! You must be the current attacker and use a valid insult from the classic list.",
            string_param_schema("insult", "The insult to throw at your opponent"),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        let args: ThrowInsultArgs = parse_args_or_default(params);
        let insult = PlayerInput::try_from(args.insult)
            .map_err(|e| map_validation_error(&e, "Insult", self.name()))?;

        // ⚡ Bolt Optimization: Pass ownership of 'insult' to Arena to avoid allocation.
        Ok(server.execute_turn_action("Insult", |arena| arena.throw_insult(&session_id, insult)).await)
    }
}

pub struct Respond;

#[derive(Deserialize, Default)]
struct RespondArgs {
    #[serde(rename = "comeback", deserialize_with = "deserialize_lossy_string")]
    comeback: String,
}

#[async_trait]
impl ToolHandler for Respond {
    fn name(&self) -> &'static str {
        "respond"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Respond to an insult with a witty comeback! If your comeback matches the correct response, you parry and become the attacker.",
            string_param_schema("comeback", "Your witty comeback to parry the insult"),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        let args: RespondArgs = parse_args_or_default(params);
        let comeback = PlayerInput::try_from(args.comeback)
            .map_err(|e| map_validation_error(&e, "Comeback", self.name()))?;

        info!("💬 COMEBACK ATTEMPT: {:?}", comeback);

        // ⚡ Bolt Optimization: Pass ownership of 'comeback' to Arena to avoid allocation.
        Ok(server.execute_turn_action("Respond", |arena| arena.respond(&session_id, comeback)).await)
    }
}

pub struct GetHint;

#[async_trait]
impl ToolHandler for GetHint {
    fn name(&self) -> &'static str {
        "get_hint"
    }

    fn tool_def(&self) -> Tool {
        create_base_tool(
            self.name(),
            "Get a hint for the current pending insult. Returns the first few characters of the correct comeback.",
            empty_input_schema(),
        )
    }

    async fn execute(
        &self,
        server: &InsultServer,
        session_id: SessionId,
        _params: Value,
    ) -> Result<DuelResponse, CallToolError> {
        let arena = server.arena.lock().await;

        match arena.get_hint(&session_id) {
            Ok((hint, insult)) => {
                Ok(DuelResponse::with_hint("Here's a hint for the comeback:", hint, insult))
            }
            Err(e) => Ok(DuelResponse::error(e.to_string())),
        }
    }
}
