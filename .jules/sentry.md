# Sentry's Journal

## [Arena] Hardcoded Game Rules
**Learning:** The `Arena` module was hardcoding `wins_needed: 3` in `DuelStateView`, ignoring the actual configuration in `Duel`. This meant that if `Duel` defaults changed, the API response would be incorrect.
**Action:** Always expose configuration/rules via getters in the Model (`Duel`) so the View/Controller (`Arena`) can source of truth it. Added `Duel::wins_needed()` and updated `Arena`.

## [Testing] Announcer Message Specificity
**Learning:** The `Announcer` has distinct victory messages for "Winning by Parry" (Victory!) vs "Winning by Opponent Failure" (OOF!). My integration test initially failed because I expected "VICTORY" in all winning cases.
**Action:** When testing UI/Message outputs, verify the *specific* condition (e.g., check for "wins the duel" generic phrase if exact flavor text varies).

## [InsultBank] Search Robustness
**Learning:** `InsultBank::search_insults` returns ALL items for an empty query. While currently safe (16 items), this is a pattern to watch if data grows. The `MAX_SEARCH_QUERY_LENGTH` limit effectively prevents allocation DoS.
**Action:** Keep explicit limits on all user-supplied query strings before allocation.

## [Dojo] Turn Loop Verification
**Learning:** Testing the `Dojo` autonomous turn loop required simulating a sequence of moves. The state transitions happen in a loop, so `turn()` might advance the state multiple steps (e.g. Player -> Sensei Parry -> Sensei Attack -> Player).
**Action:** When testing autonomous agents/loops, verify the *final* state after the function returns, not just the immediate next state.
