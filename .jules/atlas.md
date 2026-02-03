# Atlas's Journal

**[Refactoring InsultServer and DuelResponse]**
**Tangle:**
1. **The Blob:** `src/server.rs` contained both the request handling logic (`InsultServer`) and a long list of `tool_*` definition functions, making it cluttered and hard to navigate.
2. **The Leak/Cohesion Violation:** `DuelResponse` (an API Response DTO) was defined in `src/arena.rs` (Domain Layer) but only used in `src/server.rs` (API Layer). This coupled the domain to the API presentation.

**Blueprint:**
1. **Module Extraction:** Refactored `src/server.rs` into `src/server/mod.rs` and extracted tool definitions into `src/server/tools.rs`. This separates "What the API can do" (tools) from "How it handles requests" (handlers).
2. **Relocation:** Moved `DuelResponse` to `src/server/mod.rs`. Now the Domain (`Arena`) returns pure results/views, and the Server wraps them in the API response format.

**[Standardizing Arena Logic]**
**Tangle:** `Arena` methods returned `Result<(String, View), String>`, mixing game logic with presentation strings and making error handling fragile (Stringly Typed).
**Blueprint:** Introduced `ArenaError` (using `thiserror`) and `ArenaOutcome` enums in `src/arena.rs`. `Arena` now returns structured data, and `Display` implementations handle the text generation, strictly separating logic from presentation.
