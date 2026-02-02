# Ludwig's Journal

## 2025-10-26 - Input Parser
**Friction:** The game requires exact string matches (ignoring case) for comebacks. Missing a period or adding an exclamation mark causes failure, frustrating players who "know" the answer but miss a keystroke.
**Flow:** Implemented a "Forgiving Parser" (normalization) that ignores punctuation and whitespace differences. This allows the player to focus on the *wit* rather than the *syntax*.

## 2025-10-26 - Hint System & Juice
**Friction:** The original hint system (truncating the first 20 chars) gave away too much information or cut off context awkwardly, making it feel "cheap" rather than helpful. Plain text responses lacked impact.
**Flow:** Implemented a "Masked Hint" (Hangman-style) that reveals only the first letter of each word. This turns the hint into a mini-puzzle, keeping the player engaged. Also added "Juice" (emojis and pirate slang) to feedback messages to make success/failure feel more significant.
