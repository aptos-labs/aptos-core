---
name: move
description: Move development on Aptos
user-invocable: false
---

# Move on Aptos

Move is a safe, resource-oriented programming language for smart contracts on the
Aptos blockchain. It uses a linear type system to enforce ownership and prevent
double-spending at compile time.

Key concepts: modules, structs, resources, abilities (`key`, `store`, `copy`, `drop`),
entry functions, and the global storage model.

## Move Language

### References

- [The Move Book](https://aptos.dev/en/build/smart-contracts/book)
- [Move Specification Language](https://github.com/aptos-labs/aptos-core/blob/main/third_party/move/move-prover/doc/user/spec-lang.md)
- [Aptos Framework Reference](https://aptos.dev/en/build/smart-contracts/aptos-framework)

## Checking Move Code

Use the `{{ tool(name="move_package_status") }}` MCP tool to check for compilation errors and warnings.

- Call `{{ tool(name="move_package_status") }}` with `package_path` set to the package directory.
- The tool sets error and returns detailed error messages if the package does not compile.

Notice that like with a build system, the tool is idempotent, and does not cause recompilation 
if the compilation result and sources are up-to-date.

### Edit–Compile Cycle

When fixing compilation errors, follow this iterative loop:

1. Call `{{ tool(name="move_package_status") }}` with the package path.
2. If the package compiles cleanly, report success and stop.
3. If there are errors, read the diagnostics carefully, fix the source, and go back to step 1.

## Package Information

Use the `{{ tool(name="move_package_manifest") }}` MCP tool to discover source files and dependencies
of a Move package:

- Call `{{ tool(name="move_package_manifest") }}` with `package_path` set to the package directory.
- The result includes `source_paths` (target modules) and `dep_paths` (dependencies).

## Formal Verification

Use the `{{ tool(name="move_package_verify") }}` MCP tool to run the **Move Prover** on a package and
formally verify its specifications:

- Call `{{ tool(name="move_package_verify") }}` with `package_path` set to the package directory.
- The tool returns "verification succeeded" when all specs hold, or a diagnostic with a
  counterexample when a spec fails.

### Narrowing scope with filters

Verification of a full package can be slow. Use the `filter` parameter to restrict the prover
to only the code you are working on:

- **Single function:** set `filter` to `module_name::function_name` to verify one function.
- **Single module:** set `filter` to `module_name` to verify all functions in one module.

A good workflow when writing or editing specs is:

1. Start with a function-level filter to get fast feedback on the spec you are editing.
2. Once the function verifies, widen to the module to catch any interactions.
3. Finally, run without a filter to verify the whole package.

### Adjusting the timeout

The `timeout` parameter controls the solver timeout per verification condition (default: 40
seconds). Useful strategies:

- **Use a short timeout (5–10 s) during iterative editing** to get quick feedback. If the
  prover cannot decide within the limit it will report a timeout error; combine with a
  function-level filter for the fastest cycle.
- **Increase the timeout (60–120 s) for complex specs** that involve heavy arithmetic,
  loops, or large data structures.
- If verification times out repeatedly, consider simplifying the spec or adding helper lemmas
  rather than increasing the timeout further.

### Turning Verification Off

If a verification timeout cannot be resolved, then as a last resort, the verification
of the related function can be disabled by adding `pragma verify = false;` to the spec block.

### Edit–Verify Cycle

When writing or fixing specifications, follow this iterative loop:

1. Call `{{ tool(name="move_package_verify") }}` with the package path. Use a `filter` for the
   function or module you are working on and a short `timeout` (5–10 s) for fast feedback.
2. If verification succeeds, widen the filter (function → module → full package) and re-verify.
3. If verification fails with a counterexample, read the diagnostics, fix the spec or
   implementation, and go back to step 1.
4. If the same function times out twice in a row, add `pragma verify = false;` to its spec
   block and move on.

### Rules

- When you edit specifications, do not introduce `pragma aborts_if_is_partial`. Do not weaken 
  specifications.
