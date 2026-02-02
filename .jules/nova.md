## [Audience/Heckler]
**Concept:** A virtual audience that reacts to the duel exchanges. It tracks "hype" and boos repetition or failures, and cheers for parries.
**Fate:** Merged
**Lesson:** Adding "game feel" through side-systems (like an audience) is a low-risk way to add personality to a strict logic game. It doesn't interfere with the core `Duel` state but enhances the experience.

## [The Dojo]
**Concept:** A single-player training mode where a user fights an AI "Sensei". The Sensei can be configured with difficulty (skill level).
**Fate:** Merged
**Lesson:** Reusing the `Duel` state machine for single-player is possible by wrapping it in a loop that auto-plays the opponent's turns. Borrow checker requires careful handling when the AI reads game state before mutating it.
