# Forge's Journal

**[Arena Session Management]**
**Learning:** `Arena` was manually managing session logic (registration, validation) for `DuelSessions` fields, leading to "Feature Envy".
**Action:** Encapsulated logic into `DuelSessions` impl block to keep `Arena` focused on orchestration.

**[Idiomatic Conversions]**
**Learning:** Found ad-hoc conversion function `duel_state_view` instead of standard `From` trait.
**Action:** Always prefer implementing `From`/`Into` traits for type conversions to enable idiomatic usage (`.into()`, `map(From::from)`).

**[Server Handlers Return Types]**
**Learning:** `InsultServer` handlers were returning `String` (serialized JSON), coupling business logic with serialization.
**Action:** Refactored handlers to return `DuelResponse` struct, pushing serialization to the boundary (`handle_call_tool_request`).
