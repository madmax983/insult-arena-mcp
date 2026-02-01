# Forge's Journal

**[Reducing Tool Definition Boilerplate]**
**Learning:** The `Tool` struct in `rust_mcp_sdk` has many optional fields, leading to verbose initialization code when creating simple tools. This obscures the important parts (name, description, schema).
**Action:** Created `create_tool` and `create_tool_with_input` helper functions to abstract away the defaults and reduce noise.
