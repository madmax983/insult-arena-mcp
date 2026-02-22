#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use std::sync::{Arc, Mutex};
use rust_mcp_sdk::McpServer;
use rust_mcp_sdk::schema::{CallToolRequestParams, CustomRequest};
use rust_mcp_sdk::auth::AuthInfo;
use rust_mcp_sdk::error::McpSdkError;
use rust_mcp_sdk::schema::{
    ClientJsonrpcRequest, ClientMessage, InitializeRequestParams,
    InitializeResult, MessageFromServer, RequestId, ResultFromClient, ResultFromServer,
    ServerJsonrpcRequest, ServerMessage,
};
use rust_mcp_sdk::task_store::TaskStore;
use std::time::Duration;
use async_trait::async_trait;

struct LogWriter(Arc<Mutex<String>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.0.lock().expect("mutex lock").push_str(&s);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

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

#[test]
fn test_log_injection_throw_insult() {
    let buffer = Arc::new(Mutex::new(String::new()));
    let buffer_clone = buffer.clone();

    // Set up a subscriber that writes to our buffer
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || LogWriter(buffer_clone.clone()))
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime build");

        rt.block_on(async {
            let server = InsultServer::new();
            let mock_server = Arc::new(MockMcpServer { session_id: Some("session".to_string()) });

            // Start duel first
            let start_params = CallToolRequestParams {
                name: "start_duel".to_string(),
                arguments: None,
                meta: None,
                task: None,
            };
            let _ = server.handle_call_tool_request(start_params, mock_server.clone()).await;

            // Register challenger
            let register_params = CallToolRequestParams {
                name: "register_as_challenger".to_string(),
                arguments: None,
                meta: None,
                task: None,
            };
            let _ = server.handle_call_tool_request(register_params, mock_server.clone()).await;

            // We attempt to throw an insult.
            // If the insult is unknown, it returns an error containing the input.
            // This will trigger the error path in `ThrowInsult::execute`.
            let malicious_input = "malicious\nINJECTED_LOG";

            let mut args = serde_json::Map::new();
            args.insert("insult".to_string(), serde_json::Value::String(malicious_input.to_string()));

            let throw_params = CallToolRequestParams {
                name: "throw_insult".to_string(),
                arguments: Some(args),
                meta: None,
                task: None,
            };

            tracing::warn!("TEST LOG");
            let _ = server.handle_call_tool_request(throw_params, mock_server.clone()).await;
        });
    });

    let output = buffer.lock().expect("mutex lock").clone();

    // Verify that we actually captured something
    assert!(
        output.contains("TEST LOG"),
        "Failed to capture logs! Output is empty or missing expected log."
    );

    // Check if the output contains the unescaped newline.
    // If vulnerable, it will look like: ... "malicious\nINJECTED_LOG" ...
    // If fixed (Debug), it will look like: ... "malicious\\nINJECTED_LOG" ...

    // We assert that the raw newline followed by INJECTED_LOG is NOT present.
    // This assertion should FAIL if the code is vulnerable.
    assert!(
        !output.contains("malicious\nINJECTED_LOG"),
        "Log injection detected! Newline was not escaped. Output: {output}"
    );
}
