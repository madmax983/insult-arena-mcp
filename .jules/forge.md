**[Duplicate Role Logic]**
**Learning:** Registration logic for different roles (Challenger/Defender) was nearly identical in both `Arena` and `InsultServer`, leading to code duplication and drift risk.
**Action:** Extract generic helper methods (e.g., `assign_role`, `handle_register`) that take the role as an enum parameter to unify the logic.

**[Manual JSON Construction]**
**Learning:** Constructing `serde_json::Map` manually for notifications is verbose, untyped, and error-prone.
**Action:** Define dedicated structs with `#[derive(Serialize)]` for JSON payloads to enforce type safety and improve readability.

**[Logic vs Presentation Mixing]**
**Learning:** The `respond` method in `Arena` was becoming a God Function by mixing game state logic with complex string formatting for user messages.
**Action:** Extract pure presentation logic (message formatting) into private helper functions (e.g., `format_exchange_message`) to keep the core logic clean and testable.
