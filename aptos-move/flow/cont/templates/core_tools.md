{% if once(name="core_tools") %}
## Checking Move Code

Use the `{{ tool(name="move_package_status") }}` MCP tool to check for compilation errors and warnings.

- Call `{{ tool(name="move_package_status") }}` with `package_path` set to the package directory.
- The tool sets error and returns detailed error messages if the package does not compile.

Notice that like with a build system, the tool is idempotent, and does not cause recompilation
if the compilation result and sources are up-to-date.


## Package Manifest

Use the `{{ tool(name="move_package_manifest") }}` MCP tool to discover source files and dependencies
of a Move package:

- Call `{{ tool(name="move_package_manifest") }}` with `package_path` set to the package directory.
- The result includes `source_paths` (target modules) and `dep_paths` (dependencies).


## Querying Package Structure

Use the `{{ tool(name="move_package_query") }}` MCP tool to inspect the structure of a Move package.

Parameters:

- **`package_path`** (required) — path to the Move package directory.
- **`query`** (required) — one of the query types below.
- **`function`** (required for `function_usage`) — function name in the form `module_name::function_name`.

### Query Types

- **`dep_graph`** — returns a map from each module to the modules it depends on.
  Useful for understanding module layering and import structure.
- **`module_summary`** — returns a summary of each module's constants, structs,
  and functions. Useful for getting an overview without reading all source files.
- **`call_graph`** — returns a function-level call graph as a map from each
  function to the functions it calls.
- **`function_usage`** — returns direct and transitive calls/uses for a given
  function. "called" = direct calls; "used" = direct calls + closure captures.
  Requires the `function` parameter.
{% endif %}
