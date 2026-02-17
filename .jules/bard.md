## 2024-05-22 - [Forgiving Matching]
**Confusion:** Users might think they need to type the comeback exactly as it appears in the list, including punctuation and case.
**Clarification:** The `InsultBank` uses a `normalize` function that strips non-alphanumeric characters and converts to lowercase. So "how appropriate you fight like a cow" matches "How appropriate. You fight like a cow!".

## 2024-11-26 - [Internal Doctests]
**Confusion:** Writing executable doctests for internal types (like `ToolAction`) fails because they are not exported from the crate root.
**Clarification:** Use ```ignore` on the code block for internal types to prevent compilation errors while still showing the example code. Alternatively, move the type to a public module or use `#[cfg(test)]` modules for internal testing.
