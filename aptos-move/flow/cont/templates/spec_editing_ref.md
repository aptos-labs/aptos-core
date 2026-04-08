{# Shared spec writing/editing guidance #}
{% if once(name="spec_editing_ref") %}

{% include "templates/spec_lang.md" %}
{% include "templates/core_tools.md" %}

## Writing and Editing Specs

When writing or editing specifications:

1. Use `{{ tool(name="move_package_manifest") }}` to discover source files.
2. Read the function body to understand its behavior and abort conditions.
3. Write `spec fun_name { ... }` blocks after each function, following the Move Specification
   Language rules above.
4. Write `spec lemma lemma_name ...` block after the function for which they are introduced.
5. Spec functions are put into a `spec fun` declarations.
6. If the project already uses `.spec.move` files, put new specs into that file instead of the 
   main Move file.

**`.spec.move` files:** A `.spec.move` file is compiled as part of the same module
as the corresponding source file. Use `spec module { }` (the keyword `module`,
not a module name) to declare module-level spec items (helper functions, lemmas).
Use `spec fun_name { }` to add conditions to functions defined in the main source.
There is no `spec <module_name> { }` syntax — `spec name { }` always targets a
function named `name`.

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
- **Keep `[inferred]` markers** on all inferred conditions — they distinguish
  inferred specs from user-written ones and are needed for WP re-runs.
  Remove `[inferred = vacuous]` and `[inferred = sathard]` conditions entirely
  (as described above), but keep plain `[inferred]` on conditions you retain.
- **Keep `pragma opaque = true;`** — never remove it. It is essential for
  verification performance, not an inference artifact. If a function with
  `pragma opaque` fails verification, add `pragma verify = false;` rather
  than removing the opaque pragma.

### Additional Rules for Editing Specs

1. **Do not change function bodies.** Only modify `spec` blocks and their contents.
2. **Preserve** any user-written (non-inferred) specifications exactly as they are.
3. **Never drop `aborts_if` conditions.** Every function that can abort must have
   `aborts_if` conditions. The WP tool infers both `ensures` and `aborts_if` —
   simplify them but never remove them just because they are complex or hard to
   verify. If an `aborts_if` needs rewriting, replace it with a semantically
   equivalent expression, do not delete it.
4. **Never remove `pragma opaque`.** The WP tool marks inferred specs as opaque
   so the prover uses the spec contract instead of inlining the function body.
   Removing it causes verification to re-analyze the implementation, leading to
   timeouts. Preserve `pragma opaque = true;` in every spec block that has it.
   If verification fails on an opaque function (e.g., the prover cannot reason
   about closure side effects), add `pragma verify = false;` to disable
   verification while keeping the spec contract intact for callers.
5. **Never duplicate conditions.** Before adding any condition to a spec block,
   check whether an equivalent condition already exists. Do not create a condition
   that is semantically identical to one already present in the same spec block.
6. **No empty spec blocks.** Never create or leave behind an empty
   `spec fun_name {}` block. If removing inferred conditions would leave a spec
   block with no conditions or pragmas, delete the entire block instead.
{% endif %}
