//! Reproduce `DoS` via Unbounded Allocation in Tool Parsing.
//!
//! This test verifies that `InsultServer` correctly rejects inputs that exceed
//! `MAX_INPUT_LENGTH` by sending requests through the `ServerHandler` trait.

#![allow(clippy::unwrap_used)]

use async_trait::async_trait;
use insult_arena_mcp::InsultServer;
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::auth::AuthInfo;
use rust_mcp_sdk::error::McpSdkError;
use rust_mcp_sdk::mcp_server::ServerHandler;
use rust_mcp_sdk::schema::{
    CallToolRequestParams, ClientJsonrpcRequest, ClientMessage, CustomRequest,
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

#[tokio::test]
async fn rejects_excessive_input_length_in_throw_insult() {
    let server = InsultServer::new();
    let mock_server = Arc::new(MockMcpServer {
        session_id: Some("session1".to_string()),
    });

    // Create a massive string exceeding the limit
    let limit = insult_arena_mcp::arena::MAX_INPUT_LENGTH;
    let long_string = "a".repeat(limit + 1);

    let params = CallToolRequestParams {
        name: "throw_insult".to_string(),
        arguments: Some(
            json!({ "insult": long_string })
                .as_object()
                .unwrap()
                .clone(),
        ),
        meta: None,
        task: None,
    };

    // The function should return an error
    let result = server.handle_call_tool_request(params, mock_server).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = format!("{err:?}");
    assert!(err_msg.contains("Insult too long"));
    assert!(err_msg.contains(&format!("max {limit} chars")));
}

#[tokio::test]
async fn rejects_excessive_input_length_in_respond() {
    let server = InsultServer::new();
    let mock_server = Arc::new(MockMcpServer {
        session_id: Some("session1".to_string()),
    });

    // Create a massive string exceeding the limit
    let limit = insult_arena_mcp::arena::MAX_INPUT_LENGTH;
    let long_string = "b".repeat(limit + 1);

    let params = CallToolRequestParams {
        name: "respond".to_string(),
        arguments: Some(
            json!({ "comeback": long_string })
                .as_object()
                .unwrap()
                .clone(),
        ),
        meta: None,
        task: None,
    };

    // The function should return an error
    let result = server.handle_call_tool_request(params, mock_server).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = format!("{err:?}");
    assert!(err_msg.contains("Comeback too long"));
}
