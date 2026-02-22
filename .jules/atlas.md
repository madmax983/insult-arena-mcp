# Atlas's Journal

## [Decoupling MCP Tool Handlers]
**Tangle:** Adding a new tool required modifying 4 files: `constants.rs` (name), `tools.rs` (schema), `action.rs` (parsing), and `mod.rs` (execution). This "Shotgun Surgery" smell made the system brittle and hard to extend.
**Blueprint:** Introduced a `ToolHandler` trait (in `src/server/handler.rs`) that encapsulates definition, schema, and execution. Refactored `InsultServer` to use a dynamic registry of handlers (`HashMap<String, Arc<dyn ToolHandler>>`). Consolidated tool implementations into `src/server/tools.rs`. Deleted `action.rs` and `constants.rs`.
