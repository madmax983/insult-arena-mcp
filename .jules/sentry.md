**[Auth Logic Gap]**
**Learning:** `Arena` state machine methods (`throw_insult`, `respond`) did not validate the caller's identity, allowing any session (or none) to act on behalf of players.
**Action:** Enforce session ID as a required argument for state-modifying methods and validate against the registered role in the current turn.
