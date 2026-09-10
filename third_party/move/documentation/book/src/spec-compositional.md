# Compositional Specifications

_Since language version 2.4_

This chapter describes the constructs that allow specifications to be
*composed* from other specifications, including reasoning about higher-order
functions, the global state a function may read or write, and intermediate
states between calls.

## Behavioral Predicates

### Overview

A key challenge in specifying higher-order functions is expressing the behavior of function parameters without knowing their implementation. Behavioral predicates solve this by lifting the specification clauses of a function — its preconditions, postconditions, and abort conditions — into first-class predicates that can be referenced in the specifications of other functions.

There are four behavioral predicates:

| Predicate | Meaning |
|-----------|---------|
| `ensures_of<f>(args, result)` | The postcondition of `f` applied to `args` yielding `result` |
| `aborts_of<f>(args)` | The abort condition of `f` applied to `args` |
| `requires_of<f>(args)` | The precondition of `f` applied to `args` |
| `result_of<f>(args)` | A deterministic result selector: the value `y` such that `ensures_of<f>(args, y)` holds |

In all cases, `f` must be a name that refers to either a function parameter of function type or a concrete function.

### `ensures_of`

The `ensures_of<f>(args, result)` predicate represents the postcondition of function `f`. When used in a specification, it asserts that whatever postcondition `f` guarantees will hold for the given arguments and result.

Consider a basic higher-order function that applies a function to an argument:

```move
fun apply(f: |u64| u64, x: u64): u64 {
    f(x)
}
spec apply {
    ensures ensures_of<f>(x, result);
}
```

This specification says: whatever the postcondition of the function `f` is, it holds between the input `x` and the returned `result`.

When `apply` is **transparent** (the default — not marked `pragma opaque`), the prover inlines the function body and reasons through the actual implementation. This means closures without explicit inline specs work:

```move
fun test_add_five(x: u64): u64 {
    apply(|y| y + 5, x)
}
spec test_add_five {
    ensures result == x + 5;
}
```

When `apply` is **opaque** (`pragma opaque`), the prover only sees the specification, not the implementation. In this case, closures must carry explicit inline specs (see [Inline Closure Specifications](#inline-closure-specifications) below).

### `aborts_of`

The `aborts_of<f>(args)` predicate represents the abort condition of function `f`. It is used in `aborts_if` clauses to propagate abort conditions from function parameters:

```move
fun apply_may_abort(f: |u64| u64, x: u64): u64 {
    f(x)
}
spec apply_may_abort {
    aborts_if aborts_of<f>(x);
    ensures ensures_of<f>(x, result);
}
```

Since `apply_may_abort` is transparent here, the prover inlines the body and resolves the closure's abort behavior directly:

```move
fun test_may_abort(x: u64): u64 {
    apply_may_abort(|y| if (y == 0) abort 1 else y, x)
}
spec test_may_abort {
    aborts_if x == 0;
    ensures result == x;
}
```

For opaque higher-order functions, the closure would need an explicit inline spec with `aborts_if` conditions.

### `requires_of`

The `requires_of<f>(args)` predicate represents the precondition of function `f`. It allows higher-order functions to place requirements on their callers based on what the passed function expects:

```move
fun apply_no_abort(f: |u64| u64, x: u64): u64 {
    f(x)
}
spec apply_no_abort {
    requires !aborts_of<f>(x);
    aborts_if false;
    ensures ensures_of<f>(x, result);
}
```

This specifies that callers must pass arguments for which `f` will not abort. If a caller violates this, the prover reports an error:

```move
fun test_fail(): u64 {
    // FAILS: passing MAX_U64 violates !aborts_of<f>(x) since the closure aborts on MAX_U64
    apply_no_abort(
        |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1; },
        MAX_U64
    )
}
```

The prover output:

```
error: precondition does not hold at this call
  ┌─ requires_of_err.move:6:9
  │
6 │         requires !aborts_of<f>(x);
  │         ^^^^^^^^^^^^^^^^^^^^^^^^^^
```

### `result_of`

The `result_of<f>(args)` predicate is a deterministic result selector. Semantically, `result_of<f>(x)` denotes the value `y` such that `ensures_of<f>(x, y)` holds. It is particularly useful for specifying sequential applications and loop invariants:

```move
fun apply_seq(f: |u64| u64 has copy, x: u64): u64 {
    f(f(x))
}
spec apply_seq {
    let y = result_of<f>(x);
    requires requires_of<f>(x) && requires_of<f>(y);
    aborts_if aborts_of<f>(x) || aborts_of<f>(y);
    ensures result == result_of<f>(y);
}
```

Here `result_of` is used to name the intermediate value `y` — the result of the first application — and then specify that the final result is `f` applied to `y`.

`result_of` can also be used with known concrete functions:

```move
fun double(x: u64): u64 { x * 2 }
spec double { ensures result == x * 2; }

fun test_known(): u64 { double(5) }
spec test_known {
    ensures result == result_of<double>(5);
}
```

The existence of `result_of<f>(args)` implies that `f` is deterministic — it denotes the unique value `y` satisfying `ensures_of<f>(args, y)`. This is why `result_of` also establishes functional behavior: if `ensures_of<f>(x, y1)` and `ensures_of<f>(x, y2)` both hold, then `y1 == y2 == result_of<f>(x)`.

`result_of<f>(args)` has the return type of `f`. For void callees, `result_of` is invalid; use `ensures_of` instead. For closures with `&mut` parameters, see [Mutable Reference Parameters](#mutable-reference-parameters).

### Inline Closure Specifications

When a closure is passed to an opaque higher-order function, the prover needs to know the closure's specification to reason about it. Closures can carry inline specifications using the `spec { ... }` syntax:

```move
fun test_guarded_apply(x: u64): u64 {
    guarded_apply(|y| {
        if (y > 500) abort 1;
        y * 2
    } spec {
        aborts_if y > 500;
        ensures result == y * 2;
    }, x)
}
```

The inline specification provides the closure's contract: its abort conditions and postconditions. The prover uses these to instantiate behavioral predicates at the call site.

When the higher-order function is transparent (not opaque), the prover can often derive the closure's behavior from its implementation, making inline specs optional. However, for opaque functions, inline specs are required since the prover relies solely on specifications.

### Opaque Higher-Order Functions

Opaque functions are verified only from their specifications, not their implementations. Behavioral predicates enable writing useful specifications for opaque higher-order functions:

```move
fun apply_opaque(f: |u64| u64, x: u64): u64 {
    f(x)
}
spec apply_opaque {
    pragma opaque = true;
    ensures ensures_of<f>(x, result);
}
```

At the call site, callers must provide closures with explicit inline specs:

```move
fun test_opaque(x: u64): u64 {
    apply_opaque(|y| y + 5 spec { ensures result == y + 5; }, x)
}
spec test_opaque {
    ensures result == x + 5;
}
```

This approach enables modular verification: the implementation of `apply_opaque` is verified once against its specification, and callers are verified against the specification without seeing the implementation.

### Mutable Reference Parameters

Behavioral predicates extend to closures with mutable reference parameters. A `&mut T` argument appears **once** in a behavioral-predicate call — the prover supplies both the pre-state and post-state of the argument to the underlying predicate automatically:

```move
fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
spec apply_mut {
    ensures result == result_of<f>(x);
}
```

`result_of<f>(x_mut)` has the same type as the procedure's `result`, so `result == result_of<f>(x_mut)` type-checks directly. The `&mut` post-state of `x` is constrained deterministically: when `f` is functional in its inputs, `x`'s post-state is uniquely determined by its pre-state.

`ensures_of<f>(x_mut, r)` is the relational form. It is equivalent to `r == result_of<f>(x_mut)` for non-void deterministic callees, and is the only choice for void callees:

```move
fun apply_void_mut(f: |&mut u64|, x: &mut u64) { f(x) }
spec apply_void_mut {
    // Void `f` has no return — `result_of` is rejected. `ensures_of`
    // relates pre- and post-state of `x`.
    ensures ensures_of<f>(x);
}
```

For multi-return closures with `&mut` parameters, `result_of<f>` returns the declared return tuple. The `&mut` post-state is still pinned automatically:

```move
fun apply_two_results(f: |&mut u64| (u64, u64), x: &mut u64): (u64, u64) {
    f(x)
}
spec apply_two_results {
    ensures (result_1, result_2) == result_of<f>(x);
}
```

### Behavioral Predicates with Loops

Behavioral predicates integrate with loop invariants, enabling specification of functions like `contains`, `index`, and `reduce` over vectors:

```move
fun contains(v: &vector<u64>, pred: |&u64| bool has copy + drop): bool {
    let i = 0;
    let len = std::vector::length(v);
    while (i < len) {
        if (pred(std::vector::borrow(v, i))) {
            return true;
        };
        i = i + 1;
    }
        spec {
            invariant i <= len;
            invariant forall j in 0..i: !result_of<pred>(v[j]);
        };
    false
}
spec contains {
    requires forall x in 0..len(v): !aborts_of<pred>(v[x]);
    aborts_if false;
    ensures result == (exists k in 0..len(v): result_of<pred>(v[k]));
}
```

Notice how `result_of<pred>` is used in both the loop invariant and the postcondition to express the predicate's behavior over vector elements.

A recursive specification function can use `result_of` to define the semantics of a fold operation:

```move
spec fun spec_reduce(reducer: |u64, u64|u64, v: vector<u64>, val: u64, end: u64): u64 {
    if (end == 0) val
    else {
        let val = spec_reduce(reducer, v, val, end - 1);
        result_of<reducer>(val, v[end - 1])
    }
}

fun reduce(vec: vector<u64>, start: u64, reducer: |u64, u64|u64 has copy + drop): u64 {
    // ... loop implementation ...
}
spec reduce {
    ensures result == spec_reduce(reducer, vec, start, len(vec));
}
```

### Inline Higher-Order Functions

The higher-order functions shown so far take *function values* as parameters. Move's `inline` functions offer an alternative: an inline function is expanded at each call site, with lambda arguments beta-reduced into the body. This is the preferred style for iteration functions over vectors (`for_each_mut`, `map`, `fold`, ...), because a lambda passed to an inline function is spliced into the caller's scope and may therefore capture references — including mutable references — from that scope, which function values cannot.

For specification purposes, inline functions fall into two classes:

- An inline function **without function-typed parameters** may carry a regular function spec block. It is verified standalone against its spec like a regular function, and if it is additionally marked `pragma opaque`, its calls are retained in the verification model and call sites use the spec instead of the expanded body. This gives modular verification for inline functions that behave like regular functions, for example the abstraction of a loop by a closed form.
- An inline function **with function-typed parameters** (a higher-order inline function) cannot carry a function spec block:

  ```
  error: function spec blocks are not supported for inline functions with
         function-typed parameters; those functions are verified at each
         application site
  ```

  Its calls are always expanded, and callers verify through the expansion, guided by in-body `spec { .. }` blocks — in particular loop invariants — which may constrain the function parameters via behavioral predicates.

#### Predicate Substitution

When a call to an inline function is expanded, each behavioral predicate whose target resolves to a lambda argument is replaced by that lambda's own specification, with the predicate's arguments substituted:

| Predicate | Substituted by |
|-----------|----------------|
| `requires_of<f>(x)` | conjunction of the lambda's `requires` clauses (`true` if none) |
| `aborts_of<f>(x)` | disjunction of the lambda's `aborts_if` clauses |
| `ensures_of<f>(x, r)` | conjunction of the lambda's `ensures` clauses |
| `result_of<f>(x)` | the lambda's functional `ensures result == E`, or the beta-reduced body otherwise |

If the lambda has no attached spec, one is *derived* from its body by a source-level weakest-precondition analysis. The analysis covers imperative bodies (`let`, assignment, `if`/`match`, mutation through `&mut` parameters), exact abort conditions of primitive operations, modular summaries for calls to other functions, and global state effects. It fails — with an error asking for an explicit spec block on the lambda — for bodies containing loops, escaping lambda values, or writes through references captured from the enclosing scope:

```
error: cannot resolve `aborts_of` for this lambda argument: add a spec block
       to the lambda (e.g. `|x| .. spec { aborts_if ..; ensures ..; }`)
```

The substituted conditions are only *asserted*, never assumed: they are proven against the beta-reduced lambda body at each expansion site, so a wrong lambda spec fails at the call site. Note also that behavioral predicates are the only way to refer to a function parameter in a specification — directly applying it, as in `assert f(x) == 0`, is an error.

For a lambda parameter of type `&mut T`, behavioral predicates must use the canonical *dual-argument* form: `ensures_of<f>(pre, post)` receives the pre-state and post-state values of the `&mut` argument explicitly (`old(param)` in the lambda's spec is substituted by `pre`, plain `param` by `post`), and `aborts_of<f>(pre)` receives the pre-state value. The single-argument form of the [Mutable Reference Parameters](#mutable-reference-parameters) section is not available here.

#### Element-Wise Loops

A generic loop invariant in a higher-order inline function constrains the function parameter with behavioral predicates, which are then instantiated per expansion site:

```move
inline fun for_each_mut<T>(v: &mut vector<T>, f: |&mut T|) {
    let i = 0;
    let n = vector::length(v);
    while (i < n) {
        f(vector::borrow_mut(v, i));
        i = i + 1;
    } spec {
        invariant i <= n;
        invariant len(v) == len(old(v));
        invariant n == len(v);
        invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
        invariant forall j in 0..i: !aborts_of<f>(old(v)[j]);
        invariant forall j in i..n: v[j] == old(v)[j];
    };
}
```

A fully symbolic caller verifies directly. In the following example the lambda has no attached spec, so the derived spec `aborts_if e == MAX_U64; ensures e == old(e) + 1` is used, and the `ensures_of` invariant instantiates to `forall j in 0..i: v[j] == old(v)[j] + 1`:

```move
fun increment_all(v: &mut vector<u64>) {
    for_each_mut(v, |e| *e = *e + 1);
}
spec increment_all {
    requires forall i in 0..len(v): v[i] < MAX_U64;
    aborts_if false;
    ensures forall i in 0..len(v): v[i] == old(v)[i] + 1;
}
```

Because the invariant is point-wise quantified, its loop-exit fact transfers to the caller's postcondition by plain first-order reasoning — no induction is involved. Symbolic callers of element-wise HOFs (`for_each_ref`, `for_each_mut`, `map`, `find`) thus need no additional proof effort.

#### Accumulating Loops: Specializing Spec Functions

A fold relates a whole chain of intermediate accumulator values, which is expressed by a *recursive spec function taking the function value as a parameter*:

```move
spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
    if (end == 0) init
    else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
}

inline fun fold<T, Acc: copy + drop>(
    v: &vector<T>,
    init: Acc,
    f: |Acc, &T| Acc has copy + drop,
): Acc {
    let acc = init;
    let i = 0;
    let n = vector::length(v);
    while (i < n) {
        acc = f(acc, vector::borrow(v, i));
        i = i + 1;
    } spec {
        invariant i <= n;
        invariant n == len(v);
        invariant acc == spec_fold(f, v, init, i);
        invariant forall j in 0..i: !aborts_of<f>(spec_fold(f, v, init, j), v[j]);
    };
    acc
}
```

After expansion, `f` is a beta-reduced lambda rather than a function value, so the `spec_fold(f, ..)` calls are resolved by *specialization*: the prover generates a copy of the spec function per lambda in which the function parameter is eliminated — behavioral predicates over it are substituted as described above, and recursive calls are redirected to the specialization. Free variables of the lambda become additional parameters of the copy.

A caller can name this specialization in its own spec by calling the spec function with a *literal lambda*. Specializations are unified by *spec-equivalence* of the lambdas — alpha-equivalence which treats reference operations as transparent and ignores the integer-type instantiation of arithmetic operators — so the literal lambda `|acc, e| acc + e` below denotes the same specialized function as the beta-reduced `|acc, e| acc + *e` from the code, and the postcondition is the loop-exit fact verbatim, verified without any further proof:

```move
fun sum(v: &vector<u64>): u64 {
    fold(v, 0, |acc, e| acc + *e)
}
spec sum {
    pragma aborts_if_is_partial;
    ensures result == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
}
```

The explicit type arguments `spec_fold<u64, u64>` are required: the type instantiation of the call must agree exactly with the code side, and spec-mode number inference would otherwise widen it to `num`. This style composes upward — callers of `sum` see a named recursion and can reason about it by bounded unfolding. The `pragma aborts_if_is_partial` leaves the abort condition unspecified here; stating it exactly requires the lemma-based bridge of the next section.

#### Bridging to a User Abstraction

Relating the fold to an independently defined recursion requires induction, which is beyond plain SMT reasoning. The gap is closed with [lemmas](spec-proofs.md#proofs-and-lemmas): an inductive bridging lemma restates the code's lambda literally — unifying with the expansion — and equates the specialized recursion with the user's abstraction. Such a lemma is stated *once* per (lambda, abstraction) pair and reused with a one-line `apply` at any caller. This gives the `sum` function above an exact abort condition and a postcondition in terms of the user's own `spec_sum`:

```move
/// The user's own abstraction: sum of the first `n` elements.
spec fun spec_sum(v: vector<u64>, n: num): num {
    if (n == 0) 0 else spec_sum(v, n - 1) + v[n - 1]
}

/// Bridging lemma: the fold with the addition lambda equals `spec_sum`.
/// Proven by induction; both sides unfold one step.
spec lemma fold_is_sum(v: vector<u64>, n: u64) {
    requires n <= len(v);
    ensures spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n) == spec_sum(v, n);
} proof {
    if (n > 0) {
        apply fold_is_sum(v, n - 1);
    }
}

/// A partial sum plus the next element is bounded by any later partial
/// sum (elements are non-negative). Needed for the abort direction.
spec lemma sum_step_bound(v: vector<u64>, i: u64, n: u64) {
    requires i < n && n <= len(v);
    ensures spec_sum(v, i) + v[i] <= spec_sum(v, n);
} proof {
    if (i + 1 < n) {
        apply sum_step_bound(v, i, n - 1);
    }
}

spec sum {
    aborts_if spec_sum(v, len(v)) > MAX_U64;
    ensures result == spec_sum(v, len(v));
} proof {
    apply fold_is_sum(v, len(v));
    forall n: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n)}
        apply fold_is_sum(v, n);
    forall i: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, i)}
        apply sum_step_bound(v, i, len(v));
}
```

The `forall .. apply` instantiations make the bridging facts available at every prefix length, which the abort direction needs: an abort can occur at any iteration, and `sum_step_bound` lifts an overflowing step to the total sum. See [Proofs and Lemmas](spec-proofs.md#proofs-and-lemmas) for `proof` blocks, `apply`, and `forall .. apply`.

#### State-Modifying Lambdas

A lambda passed to an inline higher-order function may also modify global state. Its spec (attached or derived) is then two-state: the conditions relate the states before and after the lambda's application. Behavioral predicates over such a lambda are resolved as follows:

| Context | Resolution |
|---------|------------|
| Unique application site outside a loop | `old(..)` in the substituted conditions refers to the state just before the application |
| Loop invariant | `old(..)` refers to *function entry* (the invariant's own `old(..)` scope), plain state reads to the *current* state; the lambda's whole-memory effects become per-element facts like `exists<R>(a) && R[a].v == ..` |

For loops, a fifth behavioral predicate provides the frame — the part of the two-state contract the per-element facts do not carry:

| Predicate | Meaning |
|-----------|---------|
| `unchanged_of<f>(args)` | The memory `f` may write when applied to `args` is unchanged relative to the pre-state |

`unchanged_of` is built from the lambda's derived write footprint, is `true` for a lambda that does not write global state, and is currently only supported over lambda arguments of inline functions. Together this yields a canonical invariant pattern which serves value and state lambdas alike:

```move
inline fun for_each_addr(v: &vector<address>, f: |address|) {
    let i = 0;
    let n = vector::length(v);
    while (i < n) {
        f(*vector::borrow(v, i));
        i = i + 1;
    } spec {
        invariant i <= n;
        invariant forall j in 0..i: ensures_of<f>(v[j]);   // per-element facts: pre = entry, post = current
        invariant forall j in 0..i: !aborts_of<f>(v[j]);   // abort conditions read at entry
        invariant forall j in i..n: unchanged_of<f>(v[j]); // unprocessed suffix untouched
        invariant forall x: address: (forall j in 0..i: x != v[j]) ==> unchanged_of<f>(x); // everything else untouched
    };
}
```

A caller passing a resource-updating lambda proves an exact contract, provided the addresses are distinct — each element's footprint must be touched at most once for the per-element facts to be inductive:

```move
struct R has key { v: u64 }

fun bump_all(v: &vector<address>) {
    for_each_addr(v, |a| {
        let r = &mut R[a];
        r.v = r.v + 1;
    });
}
spec bump_all {
    requires forall i in 0..len(v), j in 0..len(v): i != j ==> v[i] != v[j];
    requires forall i in 0..len(v): exists<R>(v[i]) && R[v[i]].v < MAX_U64;
    aborts_if false;
    ensures forall i in 0..len(v): R[v[i]].v == old(R[v[i]].v) + 1;
    ensures forall b: address: (forall j in 0..len(v): v[j] != b) ==>
        (exists<R>(b) == old(exists<R>(b)) && (old(exists<R>(b)) ==> R[b].v == old(R[b].v)));
}
```

The substituted conditions are asserted against the beta-reduced lambda body, never assumed, so this resolution is sound for any lambda; the distinctness precondition is what makes the invariants provable. Some restrictions remain: a lambda with two dependent global effects cannot be constrained in a loop invariant (the intermediate memory state is not expressible), `unchanged_of` requires the write footprint to be derivable from the lambda's own body (a call to a state-modifying function hides it), and behavioral predicates in the bodies of spec functions cannot be resolved over state-modifying lambdas.

#### Capture-Accumulating Lambdas: folds_of

The most common imperative iteration pattern accumulates into a variable of the enclosing scope:

```move
let sum = 0;
v.for_each_ref(|e| sum = sum + *e);
```

The predicates above cannot describe this lambda: its effect lives in the *captured* variable `sum`, which a generic invariant in the higher-order function has no way to name, and the capture's value after `i` applications is defined by recursion over all previous applications. Such a lambda is a *fold* — the captured variable is its accumulator — and a dedicated predicate exposes it in the generic loop invariant:

| Predicate | Meaning |
|-----------|---------|
| `folds_of<f>(v, i)` | The captured variables written by `f` hold the fold of `f`'s per-application effect over `v[0..i]` from their values at loop entry, and no application in the prefix aborts |
| `folds_of<f>(\|j\| (..), i)` | The same, with iteration `j`'s arguments given by a literal index lambda (enumerations, zips, reversed orders) |

`folds_of` is only meaningful inside a loop invariant. The element form applies to unary `f` iterated over a vector; the general form describes the argument tuple of iteration `j` explicitly, e.g. `folds_of<f>(|j| (j, v[j]), i)` for an enumerating loop or `folds_of<f>(|j| (v1[j], v2[j]), i)` for a zip. For a lambda that writes no captures, `folds_of` degenerates to the prefix no-abort condition alone, so it *replaces* the point-wise `forall j in 0..i: !aborts_of<f>(v[j])` conjunct in the canonical invariant pattern (which is not resolvable for capture-writing lambdas):

```move
inline fun each_ref<T>(v: &vector<T>, f: |&T|) {
    let i = 0;
    let n = vector::length(v);
    while (i < n) {
        f(vector::borrow(v, i));
        i = i + 1;
    } spec {
        invariant i <= n;
        invariant n == len(v);
        invariant folds_of<f>(v, i);
    };
}
```

At each expansion the prover derives the lambda's accumulator transformer from its body — for `|e| sum = sum + *e` the transformer `|acc, e| acc + e` — snapshots the captures at expansion entry, and specializes a generic fold recursion over the transformer. By convention this recursion is `std::vector::spec_fold`, resolved there when the vector module declares it; otherwise a declaration of the same shape in the current module serves:

```move
spec fun spec_fold<Element, Acc>(f: |Acc, &Element| Acc, v: vector<Element>, init: Acc, end: u64): Acc {
    if (end == 0) init
    else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
}
```

(The general form analogously specializes `spec_fold_idx`, resolved by the same convention, with the index lambda's arguments composed into the transformer.) This is the same specialization-and-unification machinery as for [accumulating loops](#accumulating-loops-specializing-spec-functions), so the restatement idiom carries over verbatim: a caller states the fold with a literal lambda that is spec-equivalent to the derived transformer, and the postcondition is the loop-exit fact — here through the `each_ref` function and the `spec_fold` declaration above:

```move
fun sum(v: &vector<u64>): u64 {
    let sum = 0;
    each_ref(v, |e| sum = sum + *e);
    sum
}
spec sum {
    pragma aborts_if_is_partial;
    ensures result == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
}
```

When the vector module declares the recursion, the restatement names it as `vector::spec_fold<u64, u64>(..)` instead. In restatements, annotate integer literals with their concrete type (e.g. `1u64`): unannotated literals in spec context default to the widest integer type, and the restated lambda must match the code lambda's instantiation exactly to unify.

Exact abort conditions and postconditions in terms of a user abstraction (`spec_sum`) work exactly as in [Bridging to a User Abstraction](#bridging-to-a-user-abstraction), with the bridging lemma restating the transformer: `ensures spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n) == spec_sum(v, n);`.

With a standard vector module that carries `folds_of` in the loop invariants of its iteration functions (`for_each_ref`, `zip_ref`, `enumerate_ref`) and declares `spec_fold`/`spec_fold_idx`, the pattern above applies to `v.for_each_ref(..)` directly. The `ensures_of` predicate remains applicable to capture-writing lambdas by dropping the conditions that mention a capture (a sound weakening — the capture facts are carried by `folds_of`), while `aborts_of` and `result_of` report an error pointing to `folds_of`.

A lambda may write *several* captures, or accumulate *through a captured `&mut` reference*. Multiple written captures fold as a tuple with a generated (unnameable) recursion, so exact facts are limited to concrete lengths; for symbolic-length facts about coupled state, couple it in a struct behind a single captured `&mut` reference — the accumulator is then the referenced struct value, folded by the restatable generic `spec_fold` with functional field updates as the transformer.

Boundaries: the captures must have `copy` and `drop` (their values are snapshotted at loop entry); the lambda's capture writes and abort conditions must not depend on global state (their per-iteration evaluation state is not expressible in an invariant); the per-iteration effect must be derivable from the body (no loops, no callees without a modular summary); a lambda writing a *parameter* of the enclosing function is rejected — copy the parameter into a local first; the general form's index lambda must be a literal tuple of the target's arguments, not access global state, and not depend on a written capture; and lambdas with `&mut` parameters are not supported.

#### Accumulating Through a Callee

The accumulating lambda need not perform the update itself; it may pass the capture — or a `&mut` reference to it — into a function call:

```move
let m = simple_map::new();
v.for_each_ref(|e| { m.upsert(key_of(e), val_of(e)); });
```

The per-iteration effect is then taken from the callee, in this order: intrinsic map operations (add/remove and their variants of the intrinsically modeled map types) have built-in exact effects; otherwise a *functional* `ensures` for the `&mut` parameter in the callee's spec — an unconditional conjunct `p == E(old(p), ..)`, or a complete set of per-field conjuncts — is consumed (including `[abstract]` conditions of opaque callees, on the same trust as any opaque spec); otherwise the callee's own body is analyzed, transitively. If none of these yields a value — e.g. the callee's spec is relational only, or its body has loops — `folds_of` reports an error suggesting exactly these remedies.

#### Wrapper Higher-Order Functions

An inline HOF of your own may *forward* its function parameter into a vector HOF through an adapter lambda, as data-structure containers do:

```move
public inline fun for_each_val<K, V>(self: &Table<K, V>, f: |&V|) {
    self.entries().for_each_ref(|e| f(e.value_ref()));
}
```

The vector HOF's invariants then contain predicates over the adapter, whose behavior depends on the wrapper's parameter `f`. These predicates are *deferred*: they are rewritten over `f` at the adapter's composed arguments and resolve at the wrapper's own call site, against the lambda the caller supplies — behavioral predicates are transitive through forwarding wrappers, across any number of levels. For this to work, the adapter must be a *pure forwarder*: an effect-free prelude (typically reference projections) followed by a single unconditional application of the parameter. `folds_of` defers likewise, snapshotting the caller lambda's captures at each entry into the wrapper's expansion — for a bucketed container this yields fold facts *per bucket* (a whole-table index vocabulary is a planned extension).

#### Further Restrictions

In the spec of a lambda constrained by a behavioral predicate of an inline function, `old(..)` may only be applied directly to a lambda parameter.

In loop invariants and inline `spec { .. }` blocks, user-written `old(..)` over global state — `old(global<R>(a))`, `old(exists<R>(a))`, and selections thereof, but not over local variables — is allowed and refers to the state at function entry. This is useful for writing frame invariants by hand when `unchanged_of` is not derivable.

## Access Specifiers and Frame Conditions

### The `modifies_of` and `reads_of` Declarations

When a higher-order function takes a function parameter, the prover needs to know which global resources the parameter may read or write in order to establish frame conditions (what is unchanged after the call). Without `modifies_of`/`reads_of` declarations, the function parameter is treated as **pure**: its behavioral predicates can only reason about data arguments and return values, not global state. This is correct for transparent (non-opaque) higher-order functions, where the closure body is inlined and verified directly. For opaque higher-order functions whose parameters modify global state, `modifies_of` and/or `reads_of` declarations are required to make those effects visible to the specification.

The `modifies_of` and `reads_of` declarations in a function's specification describe these resource access permissions:

```move
spec apply {
    pragma opaque;
    reads_of<f> Config;
    modifies_of<f>(a: address) Data[a];
    ensures ensures_of<f>(x, result);
    aborts_if aborts_of<f>(x);
}
```

The syntax is:

```
reads_of<param_name> Resource1, Resource2, ...;
modifies_of<param_name>(formal_params) Resource1[addr], Resource2[addr], ...;
```

`reads_of` names the resource types that the function parameter may read. It takes only type names — no address expressions or parenthesized parameters.

`modifies_of` names the resource types that the function parameter may modify, using Move-2 index syntax (e.g., `Data[a]`) to specify the address at which modification is permitted. The formal parameters are variables that can be used in the modify target expressions — for example, `Data[a]` where `a` is a formal parameter.

These declarations serve two purposes:

1. **Frame conditions**: The prover uses access declarations to determine which resources are unchanged after a call. Resources declared with `reads_of` are guaranteed unchanged everywhere. Resources declared with `modifies_of` using an address expression like `Resource[a]` are guaranteed unchanged at all addresses other than `a`.
2. **Access validation**: The compiler checks that closures passed to the function do not access resources beyond what is declared.

Functions can also declare `reads` and `modifies` directly in their spec blocks:

```move
spec my_fun {
    reads R, S;
    modifies R[addr];
}
```

Both declarations are enforced. An opaque function may omit `modifies`, but
the resulting summary is coarse: the prover warns and havocs every address of
each unframed resource type the implementation can modify. A precise
`modifies` clause preserves all other addresses. If a function declares
`reads`, the prover checks that every resource the function accesses is
covered by either the `reads` or `modifies` declaration:

```
error: function `my_fun` accesses resource `S`
       which is not covered by its `reads` or `modifies` declaration
```

If no `reads` declaration is present, no read checking is performed.

### Read Access

When a resource is declared with `reads_of`, the prover becomes aware that the function parameter's behavior depends on these resources, making it sensitive to their current values. As a secondary effect, `reads_of` resources are guaranteed unchanged after the function parameter executes, enabling frame conditions at the call site:

```move
fun apply_reads(f: |address| u64, x: address): u64 {
    f(x)
}
spec apply_reads {
    pragma opaque;
    reads_of<f> Data, Index;
    ensures result == result_of<f>(x);
    ensures ensures_of<f>(x, result);
}
```

Callers can rely on the frame condition — both `Data` and `Index` are unchanged after the call:

```move
fun test_reads(addr: address): u64 acquires Data, Index {
    apply_reads(|a| read_indexed(a) spec {
        ensures result == Data[a].value + Index[a].pos;
    }, addr)
}
spec test_reads {
    ensures result == Data[addr].value + Index[addr].pos;
    // Both resources are guaranteed unchanged since reads_of declares reads-only
    ensures Data[addr] == old(Data[addr]);
    ensures Index[addr] == old(Index[addr]);
}
```

### Write Access

When a resource is declared with `modifies_of`, the function parameter may modify it. The `modifies_of` clause includes an address expression to specify where modification is permitted. The enclosing function's `modifies` clause must also list the resource:

```move
fun apply_writes(f: |address| u64, x: address): u64 {
    f(x)
}
spec apply_writes {
    pragma opaque;
    modifies Data[x];
    modifies_of<f>(a: address) Data[a];
    ensures ensures_of<f>(x, result);
    aborts_if aborts_of<f>(x);
}
```

The `modifies_of<f>(a: address) Data[a]` declaration says that `f` may only modify `Data` at address `a` (the formal parameter of the `modifies_of` declaration). This enables the prover to establish that `Data` is unchanged at all other addresses:

```move
fun test_writes(addr: address): u64 acquires Data {
    apply_writes(|a| set_data(a, 99) spec {
        modifies Data[a];
        ensures result == 99;
        ensures Data[a].value == 99;
        aborts_if !exists<Data>(a);
    }, addr)
}
spec test_writes {
    aborts_if !exists<Data>(addr);
    ensures result == 99;
    // Data at other addresses is unchanged
    ensures forall a: address where a != addr:
        Data[a] == old(Data[a]);
}
```

### Mixed Access

Different resources can have different access modes declared separately. This is common when a function reads configuration state but writes data state:

```move
fun apply_mixed(f: |address| u64, x: address): u64 {
    f(x)
}
spec apply_mixed {
    pragma opaque;
    modifies Data[x];
    reads_of<f> Config;
    modifies_of<f>(a: address) Data[a];
    ensures ensures_of<f>(x, result);
    aborts_if aborts_of<f>(x);
}
```

Here, `Config` is guaranteed unchanged everywhere, and `Data` may only be modified at address `a`. The caller can rely on both frame conditions:

```move
spec test_mixed {
    // Config is unchanged since reads_of declares it as reads-only
    ensures Config[addr] == old(Config[addr]);
    // Data is unchanged at all addresses except addr
    ensures forall a: address where a != addr:
        Data[a] == old(Data[a]);
}
```

### Access Validation

The compiler validates that closures passed to a function do not exceed the declared access. If a closure accesses resources not listed in `reads_of` or `modifies_of`, or writes to a resource declared with `reads_of`, the compiler reports an error. When no `modifies_of`/`reads_of` declarations exist for a parameter, no access validation is performed — the parameter is treated as pure (see above).

**Too narrow (missing resource):** The `reads_of` declares only `Counter`, but the closure also reads `Config`:

```move
spec apply_narrow_read {
    reads_of<f> Counter;
    ensures ensures_of<f>(x, result);
}

fun test_narrow_read(addr: address): u64 acquires Counter, Config {
    apply_narrow_read(|a| {
        // ERROR: closure accesses Config which isn't in reads_of or modifies_of
        if (Config[a].active) { Counter[a].value } else { 0 }
    } spec { ... }, addr)
}
```

The prover reports:

```
error: function argument accesses resource `Config`
       which is not declared in `modifies_of`/`reads_of` for `f`
```

**Writes violation:** The `reads_of` declares read access but the closure modifies the resource:

```move
spec apply_reads_only {
    reads_of<f> Counter;
    ensures ensures_of<f>(x);
}

fun test_writes_violation(addr: address) acquires Counter {
    apply_reads_only(|a| write_counter(a) spec {
        // ERROR: closure writes Counter but reads_of only allows reads
        modifies Counter[a];
        ...
    }, addr);
}
```

The prover reports:

```
error: function argument writes resource `Counter`
       but only `reads_of` (not `modifies_of`) is declared for `f`
```

**Parameter forwarding:** When wrapping a higher-order function, the wrapper's access declarations must not exceed the callee's:

```move
spec apply_counter_only {
    reads_of<f> Counter;
}

fun wrapper(g: |address| u64, x: address): u64 {
    // ERROR: g may access Config (per wrapper's reads_of) but apply_counter_only only allows Counter
    apply_counter_only(g, x)
}
spec wrapper {
    reads_of<g> Counter, Config;
}
```

## State Labels

### Motivation

Behavioral predicates like `ensures_of<f>(x, result)` describe a relation between the pre-state and post-state of a function call. When a function makes a single call, there is one pre-state (the function's entry) and one post-state (the function's exit), and these are implicit. But when a function makes *multiple* state-modifying calls, intermediate states arise: the post-state of the first call becomes the pre-state of the second call. State labels make these intermediate states explicit.

### Abstract State

State labels in specifications represent *abstract* snapshots of the global memory at particular points in a function's execution. They are not tied to concrete program counters or bytecode offsets — instead, they name logical states that are connected by specification constraints.

A specification with state labels defines a system of constraints over a sequence of abstract states. Each constraint says something about the relationship between two states (or observes a single state). The verifier treats these states as symbolic: it introduces unconstrained memory variables for each labeled state and then assumes only what the specification constraints assert about them (see [Mutation Primitives](#mutation-primitives) below for the primary mechanism that constrains how states relate to each other).

For example, in a specification with label `S`:

- The *entry state* is the function's pre-state (accessible via `old()`)
- State `S` is an intermediate abstract state
- The *exit state* is the function's post-state (the default)

The constraints connect these states into a chain: entry → S → exit. Each link in the chain is established by a mutation primitive or a behavioral predicate that describes how global resources change between two states.

### The `|~` Operator

| Syntax | Meaning |
|--------|---------|
| `S1..S2 \|~ expr` | Evaluate `expr` with pre-state `S1` and post-state `S2` |
| `..S \|~ expr` | Evaluate `expr` with the function's entry as pre-state; name the post-state `S` |
| `S.. \|~ expr` | Evaluate `expr` with pre-state `S` and the function's exit as post-state |
| `S \|~ expr` | Evaluate `expr` in state `S` (single state, no transition) |

### Defining and Using State Labels

State labels appear in two roles: *defining* a label establishes a new abstract state, while *using* a label references a previously defined state. The distinction determines how the verifier introduces and constrains memory variables.

**Defining a label.** A label is defined when it appears as the post-state (after `..`) in one of these constructs:

- **Mutation primitives**: `..S |~ publish<R>(addr, val)`, `..S |~ remove<R>(addr)`, `..S |~ update<R>(addr, val)`. These are the primary state-defining operations — they specify exactly how global memory changes from the pre-state to the newly defined state S. See [Mutation Primitives](#mutation-primitives) below.
- **Behavioral predicates**: `..S |~ ensures_of<f>(x)`. This defines S as the post-state of calling `f`, constraining it by `f`'s specification.
- **Two-state spec functions**: `..S |~ counter_increased(addr)`. This defines S as the post-state of a two-state spec function evaluation.

**Using a label.** A label is used when it appears as the pre-state (before `..`) or as a single-state label:

- `S.. |~ expr` — evaluate `expr` starting from state S
- `S |~ expr` — observe `expr` in state S (single state, no transition)
- `S |~ global<R>(addr)` or equivalently `S |~ R[addr]` — read resource R at addr in state S

Every label that is defined must also be used (no orphaned labels), and every label that is used must have been defined (no dangling references). This ensures the chain of states is well-connected.

### Mutation Primitives

Mutation primitives are specification-only builtins that describe how a global resource changes between two states. They are the fundamental building blocks for constraining abstract state transitions.

| Primitive | Meaning |
|-----------|---------|
| `publish<R>(addr, value)` | Resource R is created at `addr` with `value`. Requires R did not exist before. |
| `remove<R>(addr)` | Resource R is removed from `addr`. Requires R existed before. |
| `update<R>(addr, value)` | Resource R at `addr` is replaced with `value`. Requires R existed before. |

Each primitive is a boolean-valued expression that constrains the relationship between a pre-state and a post-state. When used with a state label range, the primitive defines how memory transitions between those states:

```move
ensures ..S |~ publish<Counter>(addr, Counter{value: 0});
```

This says: transitioning from the entry state to state S, a `Counter` resource with value 0 is published at `addr`. The implicit assertion is that `Counter` did not exist at `addr` in the entry state.

**Without state labels**, mutation primitives describe the transition from the function's entry to its exit:

```move
spec create_counter(account: &signer, init_value: u64) {
    ensures publish<Counter>(signer::address_of(account), Counter{value: init_value});
}
```

**With state labels**, mutation primitives chain together to describe sequences of state changes:

```move
spec double_update(addr: address, v1: u64, v2: u64) {
    // First update: entry → S
    ensures ..S |~ update<Counter>(addr, update_field(old(Counter[addr]), value, v1));
    // Second update: S → exit
    ensures S.. |~ update<Counter>(addr, update_field(S |~ Counter[addr], value, v2));
}
```

Note how the second `update` reads `Counter[addr]` in state S (via `S |~ Counter[addr]`) to get the value after the first update, then modifies it further.

**Conditional mutations** use implications to describe path-dependent state changes:

```move
spec conditional_remove(addr: address, cond: bool) {
    ensures cond ==> remove<Counter>(addr);
}
```

**Verification semantics.** Under the hood, the verifier implements mutation primitives using a *havoc-and-assume* pattern:

1. The memory for each modified resource type is havoced (set to an unconstrained value).
2. Frame conditions constrain that unmodified resource types and unmodified addresses are unchanged.
3. The mutation primitive constraints are assumed, pinning the havoced memory to the specified values.

This approach is sound and decoupled from the implementation — the verifier reasons about state transitions purely through the specification constraints, without tracking bytecode offsets or instruction ordering.

### Examples

**Two sequential state-modifying calls.** Here `..S` *defines* state `S` as the post-state of the first call, and `S..` *uses* it as the pre-state of the second. The single-state form `S |~ expr` observes `expr` in state `S` (e.g., for abort checks):

```move
fun double_remove(addr1: address, addr2: address): (Resource, Resource) acquires Resource {
    let r1 = remove_resource(addr1);
    let r2 = remove_resource(addr2);
    (r1, r2)
}
spec double_remove {
    // First removal: entry state → S
    ensures ..S |~ result_1 == result_of<remove_resource>(addr1);
    // Second removal: S → exit state
    ensures S.. |~ result_2 == result_of<remove_resource>(addr2);
    // Abort of second call checked in state S (after first removal)
    aborts_if S |~ aborts_of<remove_resource>(addr2);
    // Abort of first call checked in entry state (implicit)
    aborts_if aborts_of<remove_resource>(addr1);
}
```

**Create then read.** The single-state form `S |~ expr` is useful for observing intermediate state:

```move
fun create_then_read(account: &signer, addr: address): u64 acquires Resource {
    move_to(account, Resource { value: 42 });
    read_resource(addr)
}
spec create_then_read {
    ensures S.. |~ result == result_of<read_resource>(addr);
    ensures S |~ exists<Resource>(signer::address_of(account));
    ensures S |~ Resource[signer::address_of(account)] == Resource{value: 42};
    aborts_if S |~ aborts_of<read_resource>(addr);
    aborts_if exists<Resource>(signer::address_of(account));
}
```

**Three or more sequential calls.** The full `S1..S2` form chains intermediate states:

```move
spec three_calls {
    ensures ..s1 |~ ensures_of<f>(x);
    ensures s1..s2 |~ ensures_of<g>(x);
    ensures s2.. |~ ensures_of<h>(x);
}
```

### Predicate Restrictions

Not all behavioral predicates can carry both pre and post labels:

- `requires_of` and `aborts_of` describe conditions in a *single state*. They cannot have post-state labels:

```move
spec apply_requires_err {
    ensures ..post |~ requires_of<f>(x); // ERROR: post-state label not allowed on requires_of
}
```

- `ensures_of` and `result_of` describe state transitions and can carry both pre and post labels.

### Validation Rules

The compiler enforces three rules on state labels:

1. **No orphaned labels**: Every post-state label defined with `..S` must be referenced by some pre-state label `S..` or `S..T` in the same spec block.

```move
spec apply_orphan_post {
    ensures ..orphan |~ ensures_of<f>(x, result); // ERROR: 'orphan' is never referenced
}
```

2. **No cycles**: State label references must form a directed acyclic graph.

```move
spec apply_cycle {
    ensures a..b |~ ensures_of<f>(x, result);
    ensures b..a |~ ensures_of<f>(x, result); // ERROR: cyclic reference a -> b -> a
}
```

3. **No self-references**: A label cannot reference itself.

```move
spec apply_self_cycle {
    ensures a..a |~ ensures_of<f>(x, result); // ERROR: self-referencing label
}
```

## Two-State Specification Functions

### Defining Two-State Spec Functions

A two-state specification function is a `spec fun` that uses `old()` to reference the pre-state while also reading the current (post) state. This allows expressing transition properties that relate state before and after a function executes:

```move
spec fun counter_increased(addr: address): bool {
    old(Counter[addr].value) < Counter[addr].value
}
```

This spec function evaluates to `true` when the `Counter` value at `addr` in the current state is strictly greater than its value in the pre-state. The prover detects the use of `old()` and automatically provides dual memory parameters (pre-state and post-state) when translating to the verification backend.

### Using Two-State Spec Functions

Two-state spec functions are used in `ensures` clauses to express transition properties:

```move
fun increment_if_active(addr: address) acquires Counter, Config {
    if (Config[addr].active) {
        Counter[addr].value = Counter[addr].value + 1;
    };
}
spec increment_if_active {
    pragma opaque;
    modifies Counter[addr];
    ensures Config[addr].active ==> counter_increased(addr);
}
```

The spec function `counter_increased` compactly expresses that the counter went up, without repeating the `old()` pattern in every specification that needs to say this.

Two-state spec functions can also be used with state labels. When used with `|~`, the `old()` references resolve to the labeled pre-state:

```move
spec two_increments {
    // First increment: entry → S
    ensures ..S |~ counter_increased(addr);
    // Second increment: S → exit
    ensures S.. |~ counter_increased(addr);
}
```

Here `counter_increased` is evaluated twice with different state pairs: first between the function's entry and state `S`, then between `S` and the function's exit.

Spec functions without `old()` can be composed with two-state spec functions:

```move
spec fun counter_is_positive(addr: address): bool {
    Counter[addr].value > 0
}

spec fun counter_ok(addr: address): bool {
    counter_is_positive(addr)  // transitive: reads Counter in current state
}
```

The prover discovers the memory footprint of spec functions transitively through the call chain, so even wrapper spec functions that don't directly reference a resource will receive the correct memory parameters.

### Two-State Spec Functions with Behavioral Predicates

Two-state spec functions work seamlessly with behavioral predicates and closures. When a closure's inline spec uses a two-state spec function, the prover correctly threads the state labels through the behavioral predicate evaluation:

```move
spec fun counter_increased(addr: address): bool {
    old(Counter[addr].value) < Counter[addr].value
}

fun apply(f: |address|, x: address) {
    f(x)
}
spec apply {
    pragma opaque;
    reads_of<f> Config;
    modifies_of<f>(a: address) Counter[a];
    ensures ensures_of<f>(x);
    aborts_if aborts_of<f>(x);
}

fun test_uses_old_in_closure(addr: address) acquires Counter, Config {
    apply(|a| increment_if_active(a) spec {
        modifies Counter[a];
        ensures Config[a].active ==> counter_increased(a);
    }, addr);
}
spec test_uses_old_in_closure {
    // Config is read-only, so it's unchanged
    ensures Config[addr] == old(Config[addr]);
}
```

The prover handles the dual-state memory parameters: `old()` inside `counter_increased` resolves to the state before the closure executed, while unqualified resource references resolve to the state after the closure executed. Combined with the `reads_of` declaration that marks `Config` as reads-only and the `modifies_of` declaration that restricts `Counter` modifications to address `a`, the prover can establish that `Config` is unchanged while `Counter` may have been modified.
