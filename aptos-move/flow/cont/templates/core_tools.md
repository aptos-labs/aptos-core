{% if once(name="core_tools") %}
## Inspecting a Move package

- Call only the tool which answers a concrete question you have. Avoid a
  routine status/manifest/query sequence and repeated unchanged read-only
  queries.
- Minimize `{{ tool(name="move_package_status") }}` calls. Skip routine setup
  and after-edit status calls when the next operation is inference,
  verification, or a candidate check;
  those operations already compile the package and report compilation
  diagnostics. Reserve package status for a standalone compiler-diagnostic
  request when no such operation is otherwise needed.
- Use `{{ tool(name="move_package_manifest") }}` only when you need to
  distinguish target source files (`source_paths`) from dependency files
  (`dep_paths`). Package status is not a file or module discovery tool.
- Use `{{ tool(name="move_package_query") }}` only when a structural query
  answers a specific unresolved question. Prefer the narrowest query:
  - `function_usage` with `function: "module::function"` for the direct and
    transitive calls and closure captures relevant to one function;
  - `module_summary` with `module: "module"` for signatures and declarations;
  - `facts` only when detailed attributes or source locations are necessary;
    set `module: "module"` whenever possible because package-wide output can
    be large;
  - `dep_graph` for module dependencies;
  - `call_graph` for package-wide calls;

All tools take `package_path`, which must name the directory containing
`Move.toml`.
{% endif %}
