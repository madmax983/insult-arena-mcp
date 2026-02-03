# Warden's Journal

**2024-05-24 - DoS: Unbounded Input Allocation**
**Threat:** The `normalize` function in `src/insults.rs` allocates a new String based on input length. `handle_throw_insult` and `handle_respond` in `src/server.rs` pass user input directly to this function without length validation. An attacker could send a massive string (e.g., 1GB), causing memory exhaustion (OOM) or high CPU usage during normalization.
**Defense:** Implement `MAX_INPUT_LENGTH` (1024 chars) validation at the API boundary in `src/server.rs` before processing any input.

**2024-05-24 - DoS: Unbounded Session ID Allocation**
**Threat:** The `register_challenger` and `register_defender` methods in `src/arena.rs` accepted arbitrary length strings for `session_id` and stored them in memory. An attacker could exhaust server memory by registering with massive session ID strings.
**Defense:** Defined `MAX_SESSION_ID_LENGTH` (128 chars) and enforced this limit in registration methods. Also hardened score arithmetic with `saturating_add`.
