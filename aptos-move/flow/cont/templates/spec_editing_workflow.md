{# Shared spec writing/editing guidance #}
{% if once(name="spec_editing_workflow") %}

{% include "templates/spec_lang.md" %}
{% include "templates/status_tool.md" %}

## Writing and Editing Specs

When writing or editing specifications:

1. Use `{{ tool(name="move_package_manifest") }}` to discover source files.
2. Read the function body to understand its behavior and abort conditions.
3. Write `spec fun_name { ... }` blocks after each function, following the Move Specification
   Language rules above.
4. Spec functions are put into a `spec fun` declarations
5. Axioms are in `spec module { axiom P; }` blocks.
6. If the project already uses `.spec.move` files, but new specs into that file instead of the 
   main Move file.
7. Spec modules (as in `spec <module_name> { items }`) share the same 
   namespace as `<module_name>`

### Simplifying Specifications

Work through the following in order when cleaning up inferred or hand-written specs.

**Remove vacuous conditions.** Delete every condition marked `[inferred = vacuous]`.
These arise from havoced loop variables without sufficient invariants and are
semantically meaningless (e.g.
`ensures [inferred = vacuous] forall x: u64: result == x`).

**Eliminate quantifiers.** Conditions with quantifiers over unbounded types
(`forall x: u64`, `exists x: u64`, `forall x: address`) cause SMT solver timeouts.
They are often marked `[inferred = sathard]` but not always. Replace each with an
equivalent **non-quantified** expression:

- `exists x: u64: x < n && f(x)` — replace with a concrete bound or closed-form
  expression derived from the loop logic.
- `forall x: address: x != a ==> g(x)` — this expresses a frame condition ("nothing
  else changed"). Replace with an explicit `modifies` clause or enumerate the affected
  addresses.

**Ensure quantifiers have triggers.** Quantifiers without triggers must be avoided. Move 
supports lists of triggers as in `Q x: T, y: R {p1, .., pn}..{q1, .., qn}: e`, where each outer 
list is an alternative where all inner patterns must match. Notice that only triggers over 
uninterpreted functions are allowed, not over builtin operators.

**Simplify `update_field` expressions.** The WP engine uses
`update_field(s, field, val)` for struct mutations. Rewrite to direct struct
construction when all fields are determined, e.g.:

- `update_field(old(global<T>(addr)), value, v)` becomes
  `T { value: v, ..old(global<T>(addr)) }`, or when the struct has a single field,
  simply `T { value: v }`.
- Nested `update_field(update_field(old(p), x, a), y, b)` becomes
  `Point { x: a, y: b }` when all fields are covered.

**Consolidate unrolled specs.** When `pragma unroll` is used, the WP produces one
condition per unrolling step (e.g. `n == 0 ==> ...`, `n == 1 ==> ...`, ...,
`k < n ==> ...`). If there is a closed-form generalization, replace the case list
with a single condition. Remove the `pragma unroll` once the closed-form is in place.

**General cleanup:**

- Fix `old()` usage: `old()` in `aborts_if` or `requires` is invalid — those
  clauses are already evaluated in the pre-state. Remove `old()` wrappers.
- Remove redundant conditions implied by others or by language guarantees (e.g. an
  `aborts_if` subsumed by a stronger one).
- Simplify arithmetic. The WP engine mirrors the computation steps, producing
  expressions that can be algebraically reduced:
  - Combine terms: `(n - 1) * n / 2 + n` simplifies to `n * (n + 1) / 2`.
  - Flatten nested offsets: `old(v) + 1 + 1` becomes `old(v) + 2`.
  - Simplify overflow bounds: `v + (n - 1) > MAX_U64 - 1` becomes `v + n > MAX_U64`.
  - Specs use mathematical (unbounded) integers, so unlike Move code there is no
    risk of underflow in spec expressions — reorder freely for clarity.
- Remove `[inferred]` and `[inferred = sathard]` markers from conditions you keep.

### Additional Rules for Editing Specs

1. **Do NOT change function bodies.** Only modify `spec` blocks and their contents.
2. **Preserve** any user-written (non-inferred) specifications exactly as they are.
3. **Never duplicate conditions.** Before adding any condition to a spec block,
   check whether an equivalent condition already exists. Do not create a condition
   that is semantically identical to one already present in the same spec block.
4. **No empty spec blocks.** Never create or leave behind an empty
   `spec fun_name {}` block. If removing inferred conditions would leave a spec
   block with no conditions or pragmas, delete the entire block instead.
{% endif %}
