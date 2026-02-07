# Forge's Journal

**[Busy Logic in `Duel::respond`]**
**Learning:** The `Duel::respond` function mixes state validation, business logic (is the comeback correct?), score updating, and state transition logic. This makes it hard to read and test individual components.
**Action:** Extract these responsibilities into small, private helper functions: `validate_respond_turn`, `resolve_exchange`, `update_scores`, and `update_state_after_exchange`. This flattens the main function and gives clear names to each step of the process.
