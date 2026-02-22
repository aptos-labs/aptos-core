---
name: move-prove
description: Run the Move Prover to formally verify specifications
---

Use the `{{ tool(name="move_package_verify") }}` MCP tool to formally verify the current Move
package.

- Call `{{ tool(name="move_package_verify") }}` with `package_path` set to the Move package directory
  (the directory containing `Move.toml`).
- If a specific module or function is being worked on, pass a `filter` to narrow the scope
  (`module_name` or `module_name::function_name`). This gives faster feedback.
- Use a short `timeout` (e.g. 10) during iterative editing for quick feedback; increase it
  for complex specs.
- If the tool returns "verification succeeded", confirm to the user that all specs hold.
- If the tool returns a verification error, report the diagnostic and the counterexample to
  the user and suggest fixes to the spec or implementation.
- If verification times out, try narrowing the scope with a filter, lowering the timeout, or
  simplifying the spec. After 2 failed timeout attempts on the same function, suggest adding
  `pragma verify = false;` to the spec block and move on.
