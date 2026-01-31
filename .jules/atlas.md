**[Refactoring Server Monolith]
**Tangle:** `src/server.rs` was becoming a "Blob", mixing MCP tool schemas, JSON DTOs, and server logic in a single file, making it harder to navigate and reason about.
**Blueprint:** Refactored into a `server/` module with `tools.rs` (schemas), `types.rs` (DTOs), and `mod.rs` (logic) to enforce separation of concerns and high cohesion.
