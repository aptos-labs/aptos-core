{# One authoritative description of WP's contract and diagnostic handling. #}
{% if once(name="wp_tool") %}

### WP tool

`{{ tool(name="move_package_wp") }}` derives conditions and writes them to source.
Pass `package_path`; optionally use `filter: "module"` or
`filter: "module::function"`. Without a filter it processes the package.
`spec_output: "inline"` (default) writes contracts into source;
`"file"` writes companion `.spec.move` files. Invariants belong beside loops.

Interpret the result per function:

- **No warnings:** the generated specification is complete and correct by
  construction, including implicit arithmetic, bounds, resource, and callee
  aborts. WP does not run the prover. Verification may still time out: repair
  the proof or use an equivalent solver-friendly expression without weakening
  the contract. A compilation error or counterexample against unchanged,
  warning-free output is a tool bug.
- **Missing or inadequate loop invariant:** add an invariant that holds at
  entry and is preserved by each iteration. The warning's bounded loop-head
  observations help discover it; they are not a proof and describe only the
  displayed execution prefix. Rerun WP for that function after removing stale
  generated function clauses, preserving invariants, helpers, and user clauses.
- **Partial opaque or bodyless callee specification:** this is the only callee
  case which can make the caller legitimately partial. The caller cannot have
  total abort coverage while that boundary remains partial. Keep
  `pragma aborts_if_is_partial`, document the named callee, and do not rewrite
  the caller or remove the pragma to claim totality. The inherited-partiality
  rule below defines which such boundaries the candidate check accepts.
- **Transparent callee without a complete opaque contract:** WP cannot complete
  the caller. If the named callee is in the editable scope (for example, in the
  current module), infer and verify its opaque contract first, then rerun WP on
  the caller. If it is outside the editable scope, report the dependency as a
  corpus/package blocker: its owner must provide a complete verified opaque
  contract. Never use this case to justify `aborts_if_is_partial` on the caller.
- **Unmodeled prover intrinsic:** this is a WP tool bug. Intrinsics execute a
  prover builtin rather than their Move body; do not add a source-level spec or
  make them opaque. WP must supply the builtin value, abort, and mutation
  semantics internally.

Unexpected loss of conditions, malformed output, or any other inference
failure is a tool bug, not an invitation to weaken the specification.
{% endif %}
