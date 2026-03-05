{# Shared spec inference rules: pitfalls, dedup, scope, loop invariants #}
{% if once(name="spec_inf_rules") %}

### Common Pitfalls in AI Generated Spec Expressions

When writing spec expressions — especially loop invariants — these rules are
**hard constraints** enforced by the compiler. Violating them will cause
compilation errors and wasted iterations:

- **No `old(expr)` on locals or complex expressions.** In loop invariants,
  `old(x)` is only allowed when `x` is a simple function parameter name.
  Use locals directly — they refer to the current iteration's values.
- **No `*e` or `&e`.** Spec expressions operate on values, not references.
  Access fields directly (e.g. `v.field`, not `(*v).field`).
- **Do not forgot space after property** Write`aborts_if [inferred] !exists p` 
  with spaces separating the `[..]` property.

### Avoiding Duplicate Conditions

Before adding any condition (ensures, aborts_if, loop invariant, etc.), check
whether an equivalent condition already exists in the same spec block. Do not
add a condition that is semantically identical to one already present — even if
the WP tool produced it again.

### Respecting Filter Scope

When a `filter` restricts inference to a specific function or module, only modify
spec blocks for functions within that scope. Do not touch, add, or alter specs
of any function outside the filter. Leave all other code and spec blocks exactly
as they are.

### Synthesizing Loop Invariants

Add loop invariants for every loop in the target code which doesn't yet have one
and mark them as `[inferred]`. Remove all existing `[inferred = *]`
conditions.

Loop invariants often need **recursive spec helper functions** to express
properties about values built up across iterations (e.g. partial sums,
accumulated vectors, running products). When no existing spec function captures
the relationship, define a new `spec fun` in the same module. Typical pattern:

```
spec fun sum_up_to(n: u64): u64 {
    if (n == 0) { 0 } else { n + sum_up_to(n - 1) }
}
```

Then reference the helper in the loop invariant:

```
invariant [inferred] acc == sum_up_to(i);
```

Create as many helpers as needed to make invariants precise and verifiable.
Add a `///` doc comment to every new spec helper explaining the property it
captures. Place new spec helper functions below the Move function and spec
block that introduce them. Place axioms for a helper directly beneath that
helper's declaration.

### Referring to Behavior of other Functions

When specifying a function that calls other functions **which are not inline functions**, you 
can use **behavioral predicates** to abstract the callee's specification without inlining its 
details. These built-in predicates lift a function's spec clauses into expressions:

- `requires_of<f>(args)` — true when `f`'s `requires` clauses hold for `args`.
- `aborts_of<f>(args)` — true when `f`'s `aborts_if` clauses hold for `args`.
- `ensures_of<f>(args, result)` — true when `f`'s `ensures` clauses hold for
  `args` and the given `result` value(s). For functions returning unit, omit
  the result argument. For multiple return values, pass `result_1, result_2, ...`.
- `result_of<f>(args)` — the return value of `f` when called with `args`,
  usable in `let` bindings and expressions inside spec blocks.

The `<f>` target can be:
- A **function parameter** of function type: `ensures_of<f>(x, result)` where
  `f` is a parameter with type `|u64| u64`.
- A **named function** (same module or cross-module): `ensures_of<increment>(x, result)`
  or `ensures_of<M::increment>(x, result)`.
- A **generic function** with explicit or inferred type arguments:
  `ensures_of<identity<u64>>(x, result)` or `ensures_of<identity>(x, result)`.

**Examples:**

Specifying a higher-order function that applies a callback:

```move
fun apply(f: |u64| u64, x: u64): u64 { f(x) }
spec apply {
    aborts_if aborts_of<f>(x);
    ensures ensures_of<f>(x, result);
}
```

Using `result_of` to chain calls in a spec (e.g. `f(f(x))`):

```move
fun apply_seq(f: |u64| u64 has copy, x: u64): u64 { f(f(x)) }
spec apply_seq {
    let y = result_of<f>(x);
    requires requires_of<f>(x) && requires_of<f>(y);
    aborts_if aborts_of<f>(x) || aborts_of<f>(y);
    ensures result == result_of<f>(y);
}
```

Referring to a named function's behavior from a caller:

```move
spec bar {
    ensures ensures_of<increment>(x, result);
}
```

Using `result_of` inside loop invariants with closures:

```move
spec {
    invariant forall j in 0..i: !result_of<pred>(v[j]);
};
```

{% endif %}
