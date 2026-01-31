**[Unified Response Struct]**
**Learning:** Instead of returning raw JSON strings for different tool outputs, extending a single `DuelResponse` struct with optional fields (`insults`, `hint`) and using `#[serde(skip_serializing_if = "Option::is_none")]` allows for type-safe handlers while maintaining the same JSON output.
**Action:** When refactoring JSON-returning functions, prefer extending the main response struct over creating ad-hoc JSON or multiple distinct structs if the domain is cohesive.
