//! Insult Arena - LLM vs LLM sword fighting over MCP.
//!
//! Run with: `cargo run`
//!
//! Connect clients to: <http://localhost:3000/sse>

use std::sync::Arc;
use std::time::Duration;

use insult_arena_mcp::InsultServer;
use rust_mcp_sdk::mcp_server::{hyper_server, HyperServerOptions};
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ProtocolVersion, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::ToMcpServerHandler;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    tracing::info!("⚔️  INSULT SWORD FIGHTING ARENA");
    tracing::info!("================================");
    tracing::info!("📡 Starting SSE server on http://localhost:3000");
    tracing::info!("🔗 Connect clients to: http://localhost:3000/sse");
    tracing::info!("");
    tracing::info!("🎭 Waiting for challengers...");

    // Define server details
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "insult-arena-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("Insult Sword Fighting Arena".into()),
            description: Some("LLM vs LLM Monkey Island-style insult sword fighting!".into()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some(
            "Insult Sword Fighting Arena! Like the legendary Monkey Island game. \
             Multiple AI clients can connect and duel each other. \
             \n\nTo play:\n\
             1. register_as_challenger or register_as_defender to pick your role\n\
             2. start_duel to begin a new duel\n\
             3. Challenger uses throw_insult, Defender uses respond\n\
             4. You'll receive notifications when it's your turn!\n\
             First to 3 exchange wins takes the duel!"
                .to_string(),
        ),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    };

    // Create handler
    let handler = InsultServer::new();
    let handler_for_runtime = handler.clone();

    // Create hyper server with SSE support
    let server = hyper_server::create_server(
        server_details,
        handler.to_mcp_server_handler(),
        HyperServerOptions {
            host: "0.0.0.0".to_string(),
            port: 3000,
            sse_support: true,
            ping_interval: Duration::from_secs(30),
            ..Default::default()
        },
    );

    tracing::info!("🚀 Arena is LIVE!");

    // Start the server and get the runtime for notifications
    let runtime = server
        .start_runtime()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start server: {e:?}"))?;
    let runtime = Arc::new(runtime);

    // Inject runtime into handler for broadcasting notifications
    handler_for_runtime.set_runtime(runtime.clone()).await;

    tracing::info!("📢 Turn notifications enabled!");

    // Wait for server to finish - need to move out of Arc
    // Since await_server consumes self, we use a different approach
    // The server will run until interrupted
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");
    runtime.graceful_shutdown(Some(std::time::Duration::from_secs(5)));

    Ok(())
}
