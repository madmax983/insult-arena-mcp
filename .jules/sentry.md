# Sentry's Journal

**[Server Test Strategy]**
**Learning:** Decoupled `handle_*` methods in `InsultServer` allow for direct unit testing of the request/response cycle without mocking the full MCP runtime or Hyper layer. Using `serde_json` to deserialize responses into structs enables robust assertion on state rather than fragile string matching.
**Action:** Always favor testing server handlers directly via their internal methods when possible, and deserialize JSON responses for precise state validation.
