## [Audience/Heckler]
**Concept:** A virtual audience that reacts to the duel exchanges. It tracks "hype" and boos repetition or failures, and cheers for parries.
**Fate:** Merged
**Lesson:** Adding "game feel" through side-systems (like an audience) is a low-risk way to add personality to a strict logic game. It doesn't interfere with the core `Duel` state but enhances the experience.

## [The Dojo]
**Concept:** A single-player training mode where a user fights an AI "Sensei". The Sensei can be configured with difficulty (skill level).
**Fate:** Merged
**Lesson:** Reusing the `Duel` state machine for single-player is possible by wrapping it in a loop that auto-plays the opponent's turns. Borrow checker requires careful handling when the AI reads game state before mutating it.

## [The Parrot]
**Concept:** A hint system for players stuck on comebacks. It uses a "Parrot" character that masks the answer (e.g. "H__ a__________. Y__ f____ l___ a c__!") to help without solving it entirely.
**Fate:** Merged
**Lesson:** Additive features that help the user (like hints) reduce frustration and can be implemented purely with existing data (`InsultBank`) without modifying core game logic.
