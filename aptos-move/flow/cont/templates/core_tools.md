{% if once(name="core_tools") %}
## Inspecting a Move package

- Use `{{ tool(name="move_package_status") }}` for current compiler errors and
  warnings. Re-run it after edits; cached results make unchanged checks cheap.
- Use `{{ tool(name="move_package_manifest") }}` to distinguish target sources
  (`source_paths`) from dependency sources (`dep_paths`).
- Use `{{ tool(name="move_package_query") }}` instead of reading an entire
  package when a structural query answers the question:
  - `module_summary` for signatures and declarations;
  - `facts` for detailed declarations, attributes, and source locations;
  - `dep_graph` for module dependencies;
  - `call_graph` for package-wide calls;
  - `function_usage` with `function: "module::function"` for the direct and
    transitive calls and closure captures relevant to one function.

All tools take `package_path`, which must name the directory containing
`Move.toml`.
{% endif %}
