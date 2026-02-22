---
name: move-check
description: Check a Move package for compilation errors
---

Use the `{{ tool(name="move_package_status") }}` MCP tool to check the current Move package for
compilation errors and warnings.

- Call `{{ tool(name="move_package_status") }}` with `package_path` set to the Move package directory
  (the directory containing `Move.toml`).
- If the tool returns errors, report the diagnostic messages to the user.
- If the tool returns "no errors or warnings", confirm the code compiles successfully.
