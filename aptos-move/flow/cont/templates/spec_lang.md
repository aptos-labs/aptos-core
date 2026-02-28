{% if once(name="spec_lang") %}
## Move Specification Language

Move specifications use `spec` blocks to express formal properties that are checked
by the Move Prover.

### Function spec clauses

These appear in `spec fun_name { ... }` blocks. Spec blocks ALWAYS appear after the function
definition.

- `ensures <expr>`: Postcondition that must hold when the function returns normally.
  Evaluated in the **post-state**. Use `old(expr)` to refer to pre-state values.
- `aborts_if <expr>`: Condition under which the function may abort. **Evaluated in the
  pre-state** — **NEVER use `old()`** (see `old()` usage rules below). If any
  `aborts_if` conditions are present, the function must abort if and only if one of the
  conditions holds. Omitting all `aborts_if` clauses means abort behavior is *unspecified*
  (any abort is allowed). To express that a function never aborts, write `aborts_if false;`.
- `requires <expr>`: Precondition that callers must satisfy. **Evaluated in the pre-state** —
  **NEVER use `old()`** (see `old()` usage rules below).
- `modifies <resource>`: Declares which global resources the function may modify.

### Loop invariants

Loop invariants appear in a `spec` block after the loop body:

```move
while (cond) {
    // body
} spec {
    invariant <expr>;
};
```

- `invariant <expr>`: A property that holds before the first iteration and is preserved by
  each iteration. `old(x)` is only allowed on function parameters (see `old()` usage rules below.)

Loops without invariants cause the prover to *havoc* all loop-modified variables, which can
produce vacuous, incorrect, or overly weak specifications. Every loop needs an invariant —
examine the actual `while` loops in function bodies to find all loops that lack one.

A good invariant:
1. Holds before the first iteration (initial values satisfy it).
2. Is preserved by each iteration (inductive step).
3. Relates loop-modified variables to function parameters and constants
   (e.g., bounds like `i <= n`, accumulators like `sum == i * step`).

### Expressions in specs

- `old(expr)`: Value of `expr` at function entry. See `old()` usage rules below for
  where this is allowed.
- `result`: Return value. Only valid in `ensures`.
- `global<T>(addr)`: Global resource of type `T` at address `addr`.
- `exists<T>(addr)`: True if a resource of type `T` exists at address `addr`.
- Numeric type bounds: `MAX_U8`, `MAX_U16`, `MAX_U32`, `MAX_U64`, `MAX_U128`, `MAX_U256`.
- **No dereference or borrow**: `*e` and `&e` are not allowed in spec
  expressions. Spec expressions operate on values, not references — access
  fields directly (e.g. `v.field`, not `(*v).field` or `(&v).field`).

### `old()` usage rules

`old(expr)` means "value of `expr` at function entry." It is only valid in specific contexts:

**Wrong / Right examples:**

```move
// WRONG: old() in aborts_if — compilation error
aborts_if old(x) + old(y) > MAX_U64;
// RIGHT: aborts_if is pre-state, just use the variables directly
aborts_if x + y > MAX_U64;

// WRONG: old() in requires — compilation error
requires old(len(v)) > 0;
// RIGHT: requires is pre-state
requires len(v) > 0;

// WRONG: old(local) in loop invariant — compilation error
invariant old(sum) <= old(n) * MAX_U64;
// RIGHT: sum is a local — use it directly; n is a parameter — old(n) is ok
invariant sum <= old(n) * MAX_U64;

// WRONG: old(resource) in loop invariant — compilation error
invariant old(global<T>(addr)).field == 0;
// RIGHT: use resource directly
invariant global<T>(addr).field == 0;
```

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


### Proof blocks

When the SMT solver needs guidance to complete a verification, a `proof { ... }` block inside a
function spec block supplies hints that steer the solver. Proof blocks are only allowed in
function spec blocks.

```move
spec fun sum(n: u64): u64 {
    if (n == 0) { 0 } else { n + sum(n - 1) }
}

fun sum_up_to(n: u64): u64 { /* iterative impl */ }
spec sum_up_to {
    requires n <= 5;
    ensures result == sum(n);

    proof {
        unfold sum depth 3;
        assert sum(0) == 0;
    }
}
```

Available hints:

- `assert <expr>`: Ask the solver to prove an intermediate lemma, breaking a hard proof into
  smaller steps.
- `use <fun>(args)`: Shorthand for `assert <fun>(args)` — instantiates a spec helper at specific
  values.
- `assume [trusted] <expr>`: Introduce an assumption. The `[trusted]` annotation is **required**
  (omitting it is a compilation error) and the prover emits a warning for each trusted assumption.
- `unfold <spec_fun> [depth N]`: Expand the definition of a spec function. For recursive
  functions, `depth N` controls how many levels to unroll (default 1).
- `trigger forall x: T, ... with {exprs}`: Inject E-matching trigger patterns into a universally
  quantified formula. The trigger must mention all bound variables.
- `split on <expr>`: Case-split on a boolean (true/false) or enum (one sub-goal per variant).
- `induct on <var>`: Verify by induction on an integer parameter — creates a base case
  (var == 0) and step case (var > 0 with induction hypothesis).
- `witness x = val in exists x: T: body`: Provide a concrete witness for an existential
  postcondition. The prover substitutes the value and asserts the body.

**Examples:**

Splitting on a boolean for case analysis:

```move
fun abs_diff(a: u64, b: u64): u64 {
    if (a >= b) { a - b } else { b - a }
}
spec abs_diff {
    ensures result == if (a >= b) { a - b } else { b - a };
    proof { split on a >= b; }
}
```

Splitting on an enum:

```move
enum Color has drop { Red, Green, Blue }

fun color_code(c: Color): u64 {
    match (c) { Color::Red => 1, Color::Green => 2, Color::Blue => 3 }
}
spec color_code {
    ensures result >= 1 && result <= 3;
    proof { split on c; }
}
```

Providing an existential witness:

```move
fun find_answer(): u64 { 42 }
spec find_answer {
    ensures exists y: u64: y == 42;
    proof { witness y = 42 in exists y: u64: y == 42; }
}
```

### Property markers

The `[inferred]` property marks conditions that were not written by the user. Its value indicates
the origin or quality:

- `[inferred]`: Automatically inferred by weakest-precondition (WP) analysis. It may be overly complex, redundant,
  or occasionally incorrect.
- `[inferred = vacuous]`: Inferred by WP but detected as potentially vacuous (trivially true)
  due to unconstrained quantifier variables. Typically results from missing loop invariants.
- `[inferred = sathard]`: Inferred by WP but contains quantifier patterns that are hard for
  SMT solvers. Likely to cause verification timeouts — should be simplified or reformulated.


## Reference

- [Move Specification Language](https://aptos.dev/en/build/smart-contracts/prover/spec-lang)
{% endif %}
