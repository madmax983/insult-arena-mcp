use rust_mcp_sdk::mcp_server::HyperServerOptions;

fn main() {
    let opts = HyperServerOptions {
        non_existent_field: 0,
        ..Default::default()
    };
}
