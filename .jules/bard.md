## 2024-05-22 - [Forgiving Matching]
**Confusion:** Users might think they need to type the comeback exactly as it appears in the list, including punctuation and case.
**Clarification:** The `InsultBank` uses a `normalize` function that strips non-alphanumeric characters and converts to lowercase. So "how appropriate you fight like a cow" matches "How appropriate. You fight like a cow!".
