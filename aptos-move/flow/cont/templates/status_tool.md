{% if once(name="status_tool") %}
## Checking Move Code

Use the `{{ tool(name="move_package_status") }}` MCP tool to check for compilation errors and warnings.

- Call `{{ tool(name="move_package_status") }}` with `package_path` set to the package directory.
- The tool sets error and returns detailed error messages if the package does not compile.

Notice that like with a build system, the tool is idempotent, and does not cause recompilation
if the compilation result and sources are up-to-date.


## Package Information

Use the `{{ tool(name="move_package_manifest") }}` MCP tool to discover source files and dependencies
of a Move package:

- Call `{{ tool(name="move_package_manifest") }}` with `package_path` set to the package directory.
- The result includes `source_paths` (target modules) and `dep_paths` (dependencies).
{% endif %}
