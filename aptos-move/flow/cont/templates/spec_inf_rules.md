{# Shared spec inference rules: pitfalls, dedup, scope, loop invariants #}
{% if once(name="spec_inf_rules") %}

### Common Pitfalls in AI Generated Spec Expressions

Respect the `old()` usage rules and expression restrictions from the spec
language reference above — violating them causes compilation errors. Additionally:

- **Do not forget space after property.** Write `aborts_if [inferred] !exists p` 
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

### Marking Inferred Conditions

Every condition you write during inference — whether during loop invariant
synthesis or simplification — must carry the `[inferred]` property.
Conditions without
`[inferred]` are treated as user-written and will not be cleaned up on re-runs.

```
ensures [inferred] result == x + 1;
aborts_if [inferred] x + y > MAX_U64;
invariant [inferred] acc == sum_up_to(i);
```

Never write a bare `ensures`, `aborts_if`, or `invariant` during inference.

### Synthesizing Loop Invariants

Add loop invariants for every loop in the target code which doesn't yet have one.
Remove all existing `[inferred]` and `[inferred = *]`
conditions.

**`old()` in loop invariants:** `old(x)` is only allowed when `x` is a simple
function parameter name. To refer to a value from before the loop, save it into
a `let` binding before the loop and reference that local in the invariant.

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
block that introduce them. Place lemmas for a helper directly beneath that
helper's declaration.

### Data Invariants and Global Update Invariants

Data invariants (`spec Struct { invariant <expr>; }`) express properties that
must hold for every instance of a struct at all times. The prover checks them on
construction and after every mutation.

Good candidates for data invariants:
- Positivity / non-zero bounds on fields that are denominators or reserves
  (e.g., `invariant balance > 0;`). These eliminate impossible states and help the
  prover rule out division-by-zero or underflow in callers.
- Relationships between fields that hold by construction and are preserved by
  all operations (e.g., `invariant len == vector::length(data);`).

Do NOT add data invariants that are broken by normal operations. For example,
an AMM pool's exchange rate changes after every swap — a fixed-ratio invariant
like `invariant x == y;` will fail verification on swap.

Global update invariants (`spec module { invariant update ...; }`) constrain
how a resource changes between its old and new state during any modification.
They are verified once per function that modifies the resource, then assumed at
every call site — including inside loops. This makes them powerful for loop
verification: the prover gets the property at each iteration for free without
needing recursive spec helpers.

```
spec module {
    invariant update forall addr: address
        where old(exists<T>(addr)) && exists<T>(addr):
        old(global<T>(addr)).field <= global<T>(addr).field;
}
```

Good candidates for update invariants:
- Monotonicity properties: a value that only grows or only shrinks
  (e.g., total supply, sequence numbers, timestamps).
- Conservation laws: a quantity preserved across state transitions
  (e.g., `old(x) + old(y) == x + y` for token transfers).
- Product bounds: for AMM-style contracts, the constant-product property
  `old(rx) * old(ry) <= rx * ry` (non-decreasing due to integer division
  rounding). This is verified once on the swap function, then the prover uses
  it at every loop iteration to bound intermediate reserve values without
  recursive spec helpers.

Update invariants are especially valuable when loop bodies call opaque functions.
The prover cannot inline the function body but CAN use the update invariant to
constrain how the resource changed — bridging the gap between opaque call
semantics and loop invariant preservation.

**Combining data and update invariants with loops:** A data invariant like
`x > 0 && y > 0` plus an update invariant like `old(x) * old(y) <= x * y`
gives the prover both a floor on individual fields and a relationship between
them at every loop step — without any recursive spec functions or manual
unfolding. This pattern is the key to verifying iterative operations over
stateful resources.

{% endif %}
