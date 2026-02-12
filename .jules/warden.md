# Warden's Journal

**2024-05-24 - DoS: Unbounded Input Allocation**
**Threat:** The `normalize` function in `src/insults.rs` allocates a new String based on input length. `handle_throw_insult` and `handle_respond` in `src/server.rs` pass user input directly to this function without length validation. An attacker could send a massive string (e.g., 1GB), causing memory exhaustion (OOM) or high CPU usage during normalization.
**Defense:** Implement `MAX_INPUT_LENGTH` (1024 chars) validation at the API boundary in `src/server.rs` before processing any input.

**2024-11-25 - Integer Overflow: Score Calculation**
**Threat:** The scores in `src/duel.rs` were incremented using `+= 1` on `u8` fields. While standard game logic limits scores to `wins_needed` (default 3), a custom game configuration or logic bug could theoretically cause an overflow/panic in debug mode or wrapping in release mode.
**Defense:** Switched to `saturating_add(1)` for score updates to ensure safe arithmetic regardless of game state.

**2024-11-25 - DoS: Unbounded Session ID Allocation**
**Threat:** Session IDs were stored in `DuelSessions` without length validation. An attacker could provide a massive session ID string (e.g. 100MB), consuming server memory per session.
**Defense:** Introduced `MAX_SESSION_ID_LENGTH` (128 chars) constant in `src/arena.rs` and enforced it in `Arena::register_*` and `InsultServer` tool handlers.

**2024-11-25 - DoS: Input Allocation Optimization**
**Threat:** Although `Arena` checks `MAX_INPUT_LENGTH`, the `InsultServer` was converting the input `serde_json::Value` to `String` *before* calling `Arena`. An attacker sending a large JSON string could cause a large heap allocation before the check rejected it.
**Defense:** Added pre-allocation validation in `src/server/mod.rs` to check the length of the string slice (`&str`) from the JSON value before calling `to_string()`.

**2024-11-26 - Log Injection**
**Threat:** `InsultServer` logged user-controlled `insult` and `session_id` strings using `Display` (`{}`), which does not escape control characters. An attacker could inject newlines to forge log entries.
**Defense:** Switched to `Debug` (`{:?}`) formatting for all user inputs in logs.

**2024-11-26 - DoS: Unbounded Search Query**
**Threat:** `InsultBank::search_insults` (a public API) allocated memory proportional to input length (`Vec<char>`). A massive query string could cause memory exhaustion.
**Defense:** Enforced `MAX_SEARCH_QUERY_LENGTH` (128 chars) limit and truncation before allocation.

**2024-11-26 - DoS: Session ID Unbounded Allocation**
**Threat:** `InsultServer::handle_call_tool_request` cloned the session ID string from `runtime.session_id()` *before* checking its length. An attacker could supply a massive session ID (via the `McpServer` implementation), causing a large heap allocation before the validation logic could reject it.
**Defense:** Implemented a length check on the `Option<&String>` reference before cloning. If the session ID exceeds `MAX_SESSION_ID_LENGTH`, it returns an error immediately without allocation. Added a regression test `rejects_excessive_session_id_length`.

**2025-05-24 - DoS: Unbounded Allocation in Tool Parsing**
**Threat:** The `throw_insult` and `respond` tools in `src/server/tools.rs` converted input `serde_json::Value`s to `String` using `.to_string()` *before* checking `MAX_INPUT_LENGTH`. An attacker could cause memory exhaustion by sending a massive JSON string, which would be allocated on the heap before rejection.
**Defense:** Moved the length check to inspect the `&str` slice (via `as_str()`) *before* allocation. Added unit test `rejects_excessive_input_length_efficiently` to verify.

**2025-05-24 - DoS: Unbounded Notification Concurrency**
**Threat:** `InsultServer::broadcast_turn_notification` spawned a `tokio::task` for every connected session in an unbounded loop. An attacker controlling many connections could trigger thousands of concurrent tasks, leading to resource exhaustion (OOM/CPU).
**Defense:** Implemented a bounded `mpsc` channel (capacity 100) and a background worker with a `Semaphore` (limit 50) to process notifications. Notifications are dropped (load shedding) if the channel is full.
