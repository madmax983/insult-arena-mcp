# Forge's Journal ⚒️

## Refactoring MCP Tool Definitions
**Learning:** Defining MCP tools manually with `Tool { ... }` creates excessive boilerplate and noise, especially when only `name` and `description` vary.
**Action:** Use helper functions like `create_tool(name, desc)` and `create_tool_with_input(...)` to make the tool list declarative and readable.

## Flattening Game State Logic
**Learning:** Core game loop methods (like `respond` in a duel) often mix validation, logic evaluation, and state mutation, leading to long, hard-to-read functions.
**Action:** Extract distinct phases into private helpers: `evaluate_action()`, `apply_effects()`, and `transition_state()`. This makes the main method read like a story.
