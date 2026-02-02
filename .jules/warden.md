# Warden's Journal

**2024-05-24 - DoS: Unbounded Input Allocation**
**Threat:** The `normalize` function in `src/insults.rs` allocates a new String based on input length. `handle_throw_insult` and `handle_respond` in `src/server.rs` pass user input directly to this function without length validation. An attacker could send a massive string (e.g., 1GB), causing memory exhaustion (OOM) or high CPU usage during normalization.
**Defense:** Implement `MAX_INPUT_LENGTH` (1024 chars) validation at the API boundary in `src/server.rs` before processing any input.

**2024-05-25 - DoS: Unbounded Session ID & Logging**
**Threat:**
1. `Arena::register_challenger` and `register_defender` blindly stored unbounded Session ID strings, allowing memory exhaustion (DoS).
2. `InsultServer::handle_respond` logged user input (`comeback`) before validation, allowing disk fill or slow I/O DoS via massive log entries.
**Defense:**
1. Enforced `MAX_SESSION_ID_LENGTH` (128 chars) in `Arena`.
2. Enforced `MAX_INPUT_LENGTH` (1024 chars) in `InsultServer` boundaries (fail fast) before logging or locking.
