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

**[Decoupling Dojo AI Logic]**
**Tangle:** `Dojo` (in `experimental/dojo.rs`) was a "God Struct" handling game session orchestration, audience management, AND the specific AI logic for the opponent ("Sensei"). This violated Single Responsibility Principle.
**Blueprint:** Extracted `Sensei` into `experimental/sensei.rs`. `Dojo` now delegates AI decisions (attack/defend) to `Sensei`, keeping the orchestration logic clean and allowing for potentially different AI implementations in the future.

**[Extract NotificationManager from InsultServer]**
**Tangle:** `InsultServer` (in `src/server/mod.rs`) was responsible for both request handling (tools) and the low-level mechanics of broadcasting turn notifications (SSE channels, worker loops, load shedding). This violated the Single Responsibility Principle and made the server logic cluttered.
**Blueprint:** Extracted `NotificationManager` into `src/server/notifications.rs`. `InsultServer` now delegates notification duties to `NotificationManager`, which encapsulates the `HyperRuntime`, channels, and worker logic. This separates "Game Logic Orchestration" from "Network Notification Mechanics".

**[Extracted ToolAction to server/action.rs]**
**Tangle:** `src/server/tools.rs` mixed schema definitions (API Contract) with request parsing logic (Implementation), coupling "what" with "how".
**Blueprint:** Extracted `ToolAction` and its parsing logic into `src/server/action.rs`. Now `tools.rs` is pure definition, and `action.rs` is pure parsing/validation.
