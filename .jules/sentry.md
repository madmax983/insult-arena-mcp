**[Auth Logic Gap]**
**Learning:** `Arena` state machine methods (`throw_insult`, `respond`) did not validate the caller's identity, allowing any session (or none) to act on behalf of players.
**Action:** Enforce session ID as a required argument for state-modifying methods and validate against the registered role in the current turn.

**[Duel Logic Edge Case]**
**Learning:** `Duel::with_wins_needed(0)` allowed a game state where the first player (Challenger) would win immediately upon any score update, even if they lost the exchange, because `0 >= 0` satisfied the victory condition prematurely.
**Action:** Always clamp numeric configuration values (like `wins_needed`) to sane minimums (e.g., `max(1, value)`) in constructors to prevent logical inconsistencies.
