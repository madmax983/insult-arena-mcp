## 2024-05-22 - Input Matching
**Friction:** Players (and LLMs) often include extra punctuation or slightly different formatting (e.g., "How appropriate, you fight like a cow" vs "How appropriate. You fight like a cow."). The current `eq_ignore_ascii_case` is too strict, causing a feeling of "unresponsiveness" or unfairness when a correct comeback is rejected due to a comma.
**Flow:** Implementing a "Fuzzy Match" (Input Normalization) that strips punctuation and whitespace before comparing. This acts like "Coyote Time" for text - giving the player the benefit of the doubt.
