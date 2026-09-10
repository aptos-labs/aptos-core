{% if once(name="move_package") %}
## Move Packages

A Move package is rooted at `Move.toml`, which defines its package name,
dependencies, and named addresses. Work from that directory unless the user
selects another package explicitly.

### Named Addresses

Named addresses must resolve for compilation:

- `[addresses]` contains package bindings and may use `_` for a publish-time
  assignment.
- `[dev-addresses]` supplies development and test bindings.

For an unresolved address, first inspect the package and its dependencies for
the intended binding. For local-only code, add a unique non-framework value to
`[dev-addresses]`. Do not invent or replace a production binding merely to make
the compiler pass.

```toml
[dev-addresses]
my_package = "0x100"
```
{% endif %}
