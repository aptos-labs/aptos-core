# Move Specification Language Version 2

**version 2.0**

The Move Specification Language Version 2 (MSL-2 for short) is an extension of [MSL], the specification language of the Move Prover ([PROVER]). Preserving the full set of MSL, it adds support for _compositional specification of functions_. This enables to not only reason about function values (higher-order functions), but also to compose function specifications from the specifications of other functions. This, in turn, is a key ingredient for specification inference.

MSL-2 adds the following sets of features:

- [*Behavioral predicates*](#behavioral-predicates): special specification functions which allow to deal first-class with specifications of other functions. Besides supporting modular specifications, this also allows to reason about function values (higher-order functions).
- [*Access specifiers*](#access-specifiers-and-frame-conditions): describing a frame condition, that is the range of the global state which can be read and written to by a function or function value.
- [*State labels*](#state-labels): evaluating expressions in a named state. This allows to define a system of constraints representing transitions between intermediate global states as described by behavioral predicates.
- [*Two-state specification functions*](#two-state-specification-functions): support for specification functions which can constrain a pre/post state pair.
- *Proofs and Lemmas*: TBD
- [*Specification inference*](#specification-inference): automatic derivation of specifications using weakest-precondition analysis, designed to work in combination with AI-based inference tools.

This document describes MSL-2 in more detail, by providing examples and discussing the semantics of each construct.

---

- [Behavioral Predicates](#behavioral-predicates)
    - [Overview](#overview)
    - [`ensures_of`](#ensures_of)
    - [`aborts_of`](#aborts_of)
    - [`requires_of`](#requires_of)
    - [`result_of`](#result_of)
    - [Inline Closure Specifications](#inline-closure-specifications)
    - [Opaque Higher-Order Functions](#opaque-higher-order-functions)
    - [Mutable Reference Parameters](#mutable-reference-parameters)
    - [Behavioral Predicates with Loops](#behavioral-predicates-with-loops)
- [Access Specifiers and Frame Conditions](#access-specifiers-and-frame-conditions)
    - [The `modifies_of` and `reads_of` Declarations](#the-modifies_of-and-reads_of-declarations)
    - [Read Access](#read-access)
    - [Write Access](#write-access)
    - [Mixed Access](#mixed-access)
    - [Access Validation](#access-validation)
- [State Labels](#state-labels)
    - [Motivation](#motivation)
    - [The `|~` Operator](#the--operator)
    - [Examples](#examples)
    - [Predicate Restrictions](#predicate-restrictions)
    - [Validation Rules](#validation-rules)
- [Two-State Specification Functions](#two-state-specification-functions)
    - [Defining Two-State Spec Functions](#defining-two-state-spec-functions)
    - [Using Two-State Spec Functions](#using-two-state-spec-functions)
    - [Two-State Spec Functions with Behavioral Predicates](#two-state-spec-functions-with-behavioral-predicates)
- [Specification Inference](#specification-inference)
    - [Weakest-Precondition Approach](#weakest-precondition-approach)
    - [Loops and Invariants](#loops-and-invariants)
    - [Integration with AI-Based Inference](#integration-with-ai-based-inference)

---

# Behavioral Predicates

## Overview

A key challenge in specifying higher-order functions is expressing the behavior of function parameters without knowing their implementation. Behavioral predicates solve this by lifting the specification clauses of a function — its preconditions, postconditions, and abort conditions — into first-class predicates that can be referenced in the specifications of other functions.

MSL-2 introduces four behavioral predicates:

| Predicate | Meaning |
|-----------|---------|
| `ensures_of<f>(args, result)` | The postcondition of `f` applied to `args` yielding `result` |
| `aborts_of<f>(args)` | The abort condition of `f` applied to `args` |
| `requires_of<f>(args)` | The precondition of `f` applied to `args` |
| `result_of<f>(args)` | A deterministic result selector: the value `y` such that `ensures_of<f>(args, y)` holds |

In all cases, `f` must be a name that refers to either a function parameter of function type or a concrete function.

## `ensures_of`

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

## `aborts_of`

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

## `requires_of`

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

## `result_of`

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

## Inline Closure Specifications

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

## Opaque Higher-Order Functions

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

## Mutable Reference Parameters

Behavioral predicates extend to closures with mutable reference parameters. When a function takes `&mut T`, it effectively has two outputs: the explicit return value and the modified reference. The predicates account for both:

```move
fun apply_void_mut(f: |&mut u64|, x: &mut u64) { f(x) }
spec apply_void_mut {
    // For void return with &mut param, result_of returns the modified value
    ensures x == result_of<f>(old(x));
}

fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
spec apply_mut {
    // For non-void return with &mut, ensures_of takes (input, result, modified_param)
    ensures ensures_of<f>(old(x), result, x);
}
```

When a closure both returns a value and modifies a `&mut` parameter, `result_of` returns a tuple `(explicit_result, modified_value)`:

```move
fun apply_mut_result(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
spec apply_mut_result {
    ensures (result, x) == result_of<f>(old(x));
}
```

Tuple components can be extracted with let expressions:

```move
spec apply_mut_extract {
    ensures result == {let (r, _p) = result_of<f>(old(x)); r};
    ensures x == {let (_r, p) = result_of<f>(old(x)); p};
}
```

## Behavioral Predicates with Loops

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

---

# Access Specifiers and Frame Conditions

## The `modifies_of` and `reads_of` Declarations

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

`modifies_of` names the resource types that the function parameter may modify, using Move-2 index syntax (e.g. `Data[a]`) to specify the address at which modification is permitted. The formal parameters are variables that can be used in the modify target expressions — for example, `Data[a]` where `a` is a formal parameter.

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

Both declarations are enforced. The prover checks that opaque functions have `modifies` clauses covering all resources they actually modify. If a function declares `reads`, the prover checks that every resource the function accesses is covered by either the `reads` or `modifies` declaration:

```
error: function `my_fun` accesses resource `S`
       which is not covered by its `reads` or `modifies` declaration
```

If no `reads` declaration is present, no read checking is performed — the prover only checks `modifies` for opaque functions.

## Read Access

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

## Write Access

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

## Mixed Access

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

## Access Validation

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

---

# State Labels

## Motivation

Behavioral predicates like `ensures_of<f>(x, result)` describe a relation between the pre-state and post-state of a function call. When a function makes a single call, there is one pre-state (the function's entry) and one post-state (the function's exit), and these are implicit. But when a function makes *multiple* state-modifying calls, intermediate states arise: the post-state of the first call becomes the pre-state of the second call. State labels make these intermediate states explicit.

## The `|~` Operator

| Syntax | Meaning |
|--------|---------|
| `S1..S2 \|~ expr` | Evaluate `expr` with pre-state `S1` and post-state `S2` |
| `..S \|~ expr` | Evaluate `expr` with the function's entry as pre-state; name the post-state `S` |
| `S.. \|~ expr` | Evaluate `expr` with pre-state `S` and the function's exit as post-state |
| `S \|~ expr` | Evaluate `expr` in state `S` (single state, no transition) |

## Examples

**Two sequential state-modifying calls.** Here `..S` defines state `S` as the post-state of the first call, and `S..` uses it as the pre-state of the second. The single-state form `S |~ expr` evaluates `expr` in state `S` (e.g. for abort checks):

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

## Predicate Restrictions

Not all behavioral predicates can carry both pre and post labels:

- `requires_of` and `aborts_of` describe conditions in a *single state*. They cannot have post-state labels:

```move
spec apply_requires_err {
    ensures ..post |~ requires_of<f>(x); // ERROR: post-state label not allowed on requires_of
}
```

- `ensures_of` and `result_of` describe state transitions and can carry both pre and post labels.

## Validation Rules

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

---

# Two-State Specification Functions

## Defining Two-State Spec Functions

A two-state specification function is a `spec fun` that uses `old()` to reference the pre-state while also reading the current (post) state. This allows expressing transition properties that relate state before and after a function executes:

```move
spec fun counter_increased(addr: address): bool {
    old(Counter[addr].value) < Counter[addr].value
}
```

This spec function evaluates to `true` when the `Counter` value at `addr` in the current state is strictly greater than its value in the pre-state. The prover detects the use of `old()` and automatically provides dual memory parameters (pre-state and post-state) when translating to the verification backend.

## Using Two-State Spec Functions

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

## Two-State Spec Functions with Behavioral Predicates

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


# Specification Inference

With the constructs introduced in MSL-2, the Move Prover now includes a *specification inference engine* that can automatically derive specifications for functions. This is a key ingredient for scaling formal verification: rather than requiring developers to manually annotate every function, the prover can infer many specifications automatically.

## Weakest-Precondition Approach

The inference engine uses a _weakest-precondition_ (WP) backward analysis over the function's bytecode. Starting from the function's exit point, it works backward through each instruction, accumulating the conditions that must hold for the function to satisfy its specification. For each state-changing operation (global writes, function calls, aborts), the analysis emits an appropriate specification condition:

- **Direct mutations** (e.g., `Counter[addr].value = v`) produce `ensures` conditions that relate the final state to the initial state using `update_field`.
- **Opaque function calls** produce behavioral predicate conditions (`ensures_of`, `aborts_of`) that delegate to the callee's specification.
- **Abort points** produce `aborts_if` conditions.
- **Frame conditions** (`modifies`) are inferred from the set of global resources written by the function.

When multiple state-changing operations occur in sequence, the inference engine introduces [state labels](#state-labels) to distinguish intermediate states, producing a chain of constraints (e.g., `..S |~ ensures_of<f>(a)` followed by `S.. |~ ensures_of<g>(b)`).

## Loops and Invariants

The WP analysis requires that each loop in the function body has an explicit _loop invariant_. Without invariants, the analysis cannot reason across loop iterations and will produce vacuous (trivially true) specifications that are unsound for verification.

If a function contains loops without invariants, the inference engine will still produce output, but the inferred specifications should be treated as incomplete. For correct results, loop invariants must be provided by the developer (or AI -- see below) before inference can derive meaningful specifications.

## Integration with AI-Based Inference

The WP-based inference engine is designed to complement AI-based specification inference, specifically for loop invariants, as provided by the **MoveFlow** tool. The two approaches have complementary strengths:

- **WP inference** is precise and sound for straight-line code with opaque calls and mutations, but requires human-provided loop invariants and cannot guess high-level intent.
- **AI-based inference** can suggest loop invariants, high-level properties, and specifications for functions where WP analysis alone is insufficient.

A typical workflow combines both: AI-based tools propose candidate specifications (including loop invariants), and the WP engine fills in the precise arithmetic and frame conditions. The result is then verified by the Move Prover to ensure soundness.

[PROVER]: prover-guide.md

[PROVER_USAGE]: prover-guide.md

[MSL]: spec-lang.md

[PRE_POST_REFERENCE]: https://en.wikipedia.org/wiki/Design_by_contract
