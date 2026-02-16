# Warden's Journal

## 2025-02-25 - Deserialization Bomb Vulnerability
**Threat:** The `rust-mcp-sdk` crate (v0.8) uses `serde_json` to parse incoming JSON-RPC requests without an apparent configurable body size limit in `HyperServerOptions`. This allows a malicious client to send a massive JSON payload (e.g., 1GB string), causing the server to allocate memory until it crashes (OOM Denial of Service).
**Defense:** Attempted to configure `max_body_size` in `HyperServerOptions` but investigation revealed no such field exists in the current SDK version. The vulnerability remains unmitigated at the SDK level.
**Mitigation:** Hardened other areas (see below). Awaiting SDK update or upstream fix.

## 2025-02-25 - Unbounded Memory Growth in Audience
**Threat:** The `Audience` struct in `experimental/audience.rs` (used in Dojo mode) stored an unbounded history of insults (`Vec<String>`) to detect repetition. A long-running session or malicious actor could cause memory exhaustion by generating infinite unique exchanges.
**Defense:** Implemented `MAX_HISTORY_SIZE = 50`. The history now acts as a rolling buffer, discarding the oldest entries when full.
**Verification:** Added `test_history_limit` in `src/experimental/audience.rs` verifying that the 51st element evicts the 1st element.
