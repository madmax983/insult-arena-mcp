# Ludwig's Journal

## 2025-10-26 - Input Parser
**Friction:** The game requires exact string matches (ignoring case) for comebacks. Missing a period or adding an exclamation mark causes failure, frustrating players who "know" the answer but miss a keystroke.
**Flow:** Implemented a "Forgiving Parser" (normalization) that ignores punctuation and whitespace differences. This allows the player to focus on the *wit* rather than the *syntax*.

## 2025-10-27 - Hint System
**Friction:** Hints were just the first 20 characters of the comeback. This felt arbitrary and sometimes gave away too much or too little ("I am rubber you are..."). It felt "debuggy".
**Flow:** Changed to a "Hangman-style" mask (first letter of each word visible). This feels more like a puzzle and gives consistent, fair guidance that rewards knowing the general shape of the comeback.

## 2025-10-28 - Feedback Loop
**Friction:** Players were losing track of the score and didn't feel the tension of "Match Point". The feedback was too dry ("OOF!").
**Flow:** Added "Juice" to the feedback loop: explicit score display and a "🔥 MATCH POINT! 🔥" announcement. This creates a "Climax" moment in the gameplay loop.
