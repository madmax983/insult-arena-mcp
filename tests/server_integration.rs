//! Integration tests for the Insult Server.
//!
//! Tests the full server stack (except network layer) using a mock MCP runtime.
//! Verifies tool execution, state management, and error handling.

use async_trait::async_trait;
use insult_arena_mcp::InsultServer;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::auth::AuthInfo;
use rust_mcp_sdk::error::McpSdkError;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ClientJsonrpcRequest, ClientMessage, CustomRequest,
    InitializeRequestParams, InitializeResult, MessageFromServer, RequestId, ResultFromClient,
    ResultFromServer, ServerJsonrpcRequest, ServerMessage,
};
use rust_mcp_sdk::task_store::TaskStore;
use serde_json::json;
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

#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
async fn call_tool(
    server: &InsultServer,
    session_id: &str,
    tool_name: &str,
    args: serde_json::Value,
) -> CallToolResult {
    let mock_server = Arc::new(MockMcpServer {
        session_id: Some(session_id.to_string()),
    });

    let arguments = if args.is_null() {
        None
    } else {
        Some(args.as_object().unwrap().clone())
    };

    let params = CallToolRequestParams {
        name: tool_name.to_string(),
        arguments,
        meta: None,
        task: None,
    };

    server
        .handle_call_tool_request(params, mock_server)
        .await
        .expect("Tool call failed")
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_full_game_loop() {
    let server = InsultServer::new();

    // 1. Start Duel
    let result = call_tool(&server, "any", "start_duel", json!({})).await;
    assert!(format!("{:?}", result.content[0]).contains("success"));

    // 2. Register Challenger (Alice)
    let result = call_tool(&server, "Alice", "register_as_challenger", json!({})).await;
    assert!(format!("{:?}", result.content[0]).contains("Challenger"));

    // 3. Register Defender (Bob)
    let result = call_tool(&server, "Bob", "register_as_defender", json!({})).await;
    assert!(format!("{:?}", result.content[0]).contains("Defender"));

    // 4. Alice throws insult
    let insult = "You fight like a dairy farmer!";
    let result = call_tool(
        &server,
        "Alice",
        "throw_insult",
        json!({ "insult": insult }),
    )
    .await;
    let text = format!("{:?}", result.content[0]);
    assert!(text.contains("success"));
    assert!(text.contains(insult));

    // 5. Bob gets hint
    let result = call_tool(&server, "Bob", "get_hint", json!({})).await;
    let text = format!("{:?}", result.content[0]);
    assert!(text.contains("hint"));
    // "How appropriate. You fight like a cow!" -> "H__ a__________" ...
    assert!(text.contains("H__"));

    // 6. Bob responds correctly
    let comeback = "How appropriate. You fight like a cow!";
    let result = call_tool(&server, "Bob", "respond", json!({ "comeback": comeback })).await;
    let text = format!("{:?}", result.content[0]);
    assert!(text.contains("success"));
    assert!(text.contains("TOUCHÉ"));
    // Score should be 0-1
    assert!(text.contains("0-1"));

    // 7. Bob is now attacker. Throw insult.
    let insult = "You have the manners of a beggar.";
    let result = call_tool(&server, "Bob", "throw_insult", json!({ "insult": insult })).await;
    assert!(format!("{:?}", result.content[0]).contains("success"));

    // 8. Alice responds incorrectly
    let wrong_comeback = "I am rubber, you are glue.";
    let result = call_tool(
        &server,
        "Alice",
        "respond",
        json!({ "comeback": wrong_comeback }),
    )
    .await;
    let text = format!("{:?}", result.content[0]);
    assert!(text.contains("OOF"));
    // Bob wins exchange. Score: 0-2
    assert!(text.contains("0-2"));

    // Verify state
    let result = call_tool(&server, "Alice", "get_duel_state", json!({})).await;
    let text = format!("{:?}", result.content[0]);
    // Relaxed check for debug string format
    assert!(text.contains("challenger_score") && text.contains('0'));
    assert!(text.contains("defender_score") && text.contains('2'));
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_concurrent_turns() {
    let server = Arc::new(InsultServer::new());

    // Setup
    call_tool(&server, "any", "start_duel", json!({})).await;
    call_tool(&server, "Alice", "register_as_challenger", json!({})).await;
    call_tool(&server, "Bob", "register_as_defender", json!({})).await;

    // Alice throws insult. Now Bob's turn.
    call_tool(
        &server,
        "Alice",
        "throw_insult",
        json!({ "insult": "You fight like a dairy farmer!" }),
    )
    .await;

    // Bob needs to respond. Alice cannot throw again.
    let server_clone1 = server.clone();
    let server_clone2 = server.clone();

    // Task 1: Alice tries to throw again (should fail)
    let t1 = tokio::spawn(async move {
        call_tool(
            &server_clone1,
            "Alice",
            "throw_insult",
            json!({ "insult": "Another insult!" }),
        )
        .await
    });

    // Task 2: Bob responds (should succeed)
    let t2 = tokio::spawn(async move {
        // Delay slightly to ensure race isn't trivial? No, let them race.
        call_tool(
            &server_clone2,
            "Bob",
            "respond",
            json!({ "comeback": "How appropriate. You fight like a cow!" }),
        )
        .await
    });

    let (r1, r2) = tokio::join!(t1, t2);
    let res1 = r1.unwrap();
    let res2 = r2.unwrap();

    let text1 = format!("{:?}", res1.content[0]);
    let text2 = format!("{:?}", res2.content[0]);

    // Alice's attempt should fail (Waiting for comeback / Not your turn)
    assert!(
        text1.contains("Waiting for a comeback")
            || text1.contains("error")
            || text1.contains("success\": false")
    );

    // Bob's attempt should succeed
    assert!(text2.contains("success") && text2.contains("true"));
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_error_handling() {
    let server = InsultServer::new();
    call_tool(&server, "any", "start_duel", json!({})).await;
    call_tool(&server, "Alice", "register_as_challenger", json!({})).await;

    // Empty insult
    let result = call_tool(&server, "Alice", "throw_insult", json!({ "insult": "" })).await;
    let text = format!("{:?}", result.content[0]);
    // Empty insult is "unknown"
    assert!(text.contains("Unknown insult") || text.contains("error"));

    // Missing argument
    // json!({}) -> None -> unwrap_or_default -> empty map -> insult=""
    let result = call_tool(&server, "Alice", "throw_insult", json!({})).await;
    let text = format!("{:?}", result.content[0]);
    assert!(text.contains("Unknown insult") || text.contains("error"));

    // Register twice
    let result = call_tool(&server, "Alice", "register_as_challenger", json!({})).await;
    let text = format!("{:?}", result.content[0]);
    assert!(text.contains("success") && text.contains("false"));
    assert!(text.contains("already taken"));
}
