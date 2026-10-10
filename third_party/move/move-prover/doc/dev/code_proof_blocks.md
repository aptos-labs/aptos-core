This document is an exploration of a design that is not implemented.

# Code Proof Blocks

Design of *proof blocks in code position*: proof annotations attached to statements, in particular to calls of higher-order functions (HOFs) whose contracts describe repeated application of a function value. Builds on the `fun_post_of` application chains of [fun_values_note.md](fun_values_note.md). Status: design; implementation pending.

## Motivation

An opaque inline HOF can declare what N applications of a mut-capturing closure do:

```move
spec fun apply_all(f: |&u64| has copy + drop, v: vector<u64>, end: u64): |&u64| has copy + drop {
    if (end == 0) f else fun_post_of<apply_all(f, v, end - 1)>(v[end - 1])
}
spec for_each_ref {
    pragma opaque;
    requires forall j in 0..len(v): !aborts_of<apply_all(f, v, j)>(v[j]);
    aborts_if false;
    ensures f == apply_all(old(f), v, len(v));
}
```

verified once against the body (loop invariant `f == apply_all(old(f), v, i)`; the induction step checks the apply discipline: one application per element, in order). But a *caller* proving `result == spec_sum(v, len(v))` for `let s = 0; for_each_ref(v, |e| s = s + *e); s` at symbolic length needs recursive lemmas over `partial_of`/`captures_of`/`write_of` plus `forall .. apply` instantiations — far more than the single loop invariant the *inlined* loop costs (see `reduce` in `tests/sources/functional/macro_verification.move`).

The cause: the contract exports the iteration only as a value-level *fold* (a recursive spec fun). SMT has no induction schema, so every fact extracted from a fold needs a hand-written inductive lemma. A loop with an invariant, by contrast, is the VC generator applying the *induction principle* at the meta level: the solver checks base and step; the recursion never enters the logic. The lemma path forces consumers to rebuild that eliminator by hand — plus instantiation management and well-foundedness (#20275).

The fix: export the chain's induction principle as a call-site construct. Target: a call to a known HOF pattern costs at most the one invariant the visible loop would cost — typically less, since index bookkeeping and frame conditions come from the contract, verified once.

## Surface Syntax

Both forms gated on language version 2.4 (which introduced `proof` blocks and lemmas).

**1. Statement-attached proof block** (also on a `let` right-hand side):

```move
for_each_ref(v, |e| s = s + *e) proof {
    invariant(k) s == spec_sum(v, k);
};
let total = vector::fold(v, 0, |acc, e| acc + e) proof {
    invariant(k) acc == spec_sum(v, k);
};
```

**2. Standalone proof statement**, sugar for `spec {} proof { .. }`:

```move
proof { assert x > 0; };
```

Existing proof items `assert`, `assume`, `let`, `if`, `calc`, `apply`, `forall .. apply` are available and take effect at that program point (`apply` asserts the lemma's requires, assumes its ensures). `post` and `split` are rejected in code position.

**The `invariant` item** — new, only in a proof block attached to a call:

```
invariant ( <index-binder> ) <exp> ;
```

The binder (`k: num`) counts applications performed. The expression may mention: locals of the enclosing function (captured state), the value parameters of the call's function argument by their declared names (`fold`'s `acc` — the value entering the k-th application), and the binder. `old(..)` is rejected in V1; the snapshot idiom `let s0 = s;` covers relative invariants.

## State Channels

**Which state flows through the application sequence is a property of the call site, not of the HOF.** Any iterating HOF can receive a lambda that captures and mutates enclosing state, and one call can carry several channels:

```move
let count = 0;
let total = vector::fold(v, 0, |acc, e| { count = count + 1; acc + e }) proof {
    invariant(k) count == k && acc == spec_sum(v, k);
};
```

- **Capture channel**: state behind the closure's `&mut` captures (`count`); advances the *closure value* — a `fun_post_of` chain.
- **Threading channel**: state returned by one application and passed into the next (`acc`) — a `result_of` chain.

The HOF author therefore always writes the *general* chain contract (the author cannot know the caller's lambda), and the two chains are mutually recursive: the k-th application happens at the k-th closure value *and* the k-th threaded value. Simplified per-application contracts (plain quantified `ensures_of` for `map`) are unsound as general contracts — a capturing lambda makes applications order-dependent. The simplifications reappear at call sites as *degenerations* of the chain, handled by the rule automatically.

## The Chain Induction Rule

An `invariant(k) P` must be attached to a call of an opaque function whose contract declares an application chain (patterns below). The declaration licenses induction, instrumented as ordinary assert/assume conditions around the call — the same trusted-rule status as loop instrumentation; no lemmas or recursive definitions enter the caller's VC.

Let σ = (c, t): c the closure's `&mut`-capture values, t the threaded values (absent without threading). The rule maintains as *built-in* motive the pinning of the k-th chain member to the call site's closure variant with capture state c (for a non-carrying closure: the collapse `chain(k) == old(f)`). The user's `P(k, σ)` rides on top:

| Obligation | Kind | Condition |
|---|---|---|
| BASE | assert | `P(0, σ_entry)` — captures at pre-call values, threaded values at initial arguments |
| STEP | assert | below |
| PREMISE | assume | the contract's no-abort premise over the chain |
| EXIT | assume | `P(B, σ_exit)` — captures at written-back values, threaded values at call results |

```
forall k: num, σ:
    0 <= k && k < B && P(k, σ)
==> !step_aborts(σ, elem(k))
    && (forall σ': step_ensures(σ, elem(k), σ') ==> P(k + 1, σ'))
```

- **Step relation**: the one-application contract of the function argument — the lambda's attached spec, the curried named function's spec (`|e| step(&mut s, e)` reduces to a closure over `step`), or the inferred lambda spec. Inlined by substitution.
- **Abort discharge**: the `!step_aborts` conjunct discharges the contract's abort premise pointwise, so the premise is *assumed*, justified by BASE + STEP — as the loop rule justifies assuming the invariant after the loop. This removes the prefix-monotonicity lemmas abort premises otherwise force.
- **Quantifiers are cheap**: STEP is an asserted ∀ (negated/Skolemized — no triggers). The user's vocabulary (`spec_sum`) unfolds one definitional step per obligation, in lockstep with the induction; the fold-vs-vocabulary equivalence never arises. This recovers, for the opaque call, exactly what makes the plain-loop `reduce` example lemma-free.
- **Soundness** rests on the chain ensures being *exact* — the declared sequence, order, and argument flow — which the callee-side apply-discipline check certifies (or a trusted contract asserts). Given exactness, the rule is applied by instrumentation on the same trust basis as loop instrumentation.

## General Chain Contracts for the Vector HOFs

Canonical form: a closure-value chain (`.._funs`) mutually recursive with a threaded-value chain (`.._vals`, absent without threading). Contracts below are for module-local mirrors of the `std::vector` signatures, generic and trusted (`pragma verify = false`, see Limitations). All use the *exact* aborts form: spec-less callers pay nothing; a caller proving `aborts_if false` refutes the existential via STEP.

### `for_each_ref` — capture channel only

```move
spec fun apply_all<Element>(f: |&Element| has copy + drop, v: vector<Element>, k: u64)
        : |&Element| has copy + drop {
    if (k == 0) f else fun_post_of<apply_all(f, v, k - 1)>(v[k - 1])
}
spec for_each_ref {
    pragma opaque;
    pragma verify = false;
    aborts_if exists j in 0..len(v): aborts_of<apply_all(f, v, j)>(v[j]);
    ensures f == apply_all(old(f), v, len(v));
}
```

```move
fun sum_all(v: &vector<u64>): u64 {
    let s = 0;
    for_each_ref(v, |e| s = s + *e) proof {
        invariant(k) s == spec_sum(v, k);
    };
    s
}
spec sum_all {
    requires forall j in 0..len(v): spec_sum(v, j + 1) <= MAX_U64;
    aborts_if false;
    ensures result == spec_sum(v, len(v));
}
```

Cost beyond the specification: one line. EXIT binds `s` to the value written back from the capture slot (the write-back sits between call and proof anchor).

### `fold` — both channels, mutually recursive

```move
spec fun fold_funs<A, E>(f: |A, E|A has copy + drop, init: A, v: vector<E>, k: u64)
        : |A, E|A has copy + drop {
    if (k == 0) f
    else fun_post_of<fold_funs(f, init, v, k - 1)>(fold_vals(f, init, v, k - 1), v[k - 1])
}
spec fun fold_vals<A, E>(f: |A, E|A has copy + drop, init: A, v: vector<E>, k: u64): A {
    if (k == 0) init
    else result_of<fold_funs(f, init, v, k - 1)>(fold_vals(f, init, v, k - 1), v[k - 1])
}
spec fold {
    pragma opaque;
    pragma verify = false;
    aborts_if exists j in 0..len(v):
        aborts_of<fold_funs(f, init, v, j)>(fold_vals(f, init, v, j), v[j]);
    ensures result == fold_vals(old(f), init, v, len(v));
    ensures f == fold_funs(old(f), init, v, len(v));
}
```

Pure accumulator: threading channel only (`invariant(k) acc == spec_sum(v, k)`; BASE binds `acc` to `init`, EXIT to the result). Capturing accumulator: product state, as in the State Channels example. Non-carrying closure ⇒ `fold_funs` collapses under the built-in motive; induction over the threading channel alone.

### `map` — capture channel plus per-index result facts

Results do not feed back; the result vector is described *pointwise at the chain members*:

```move
spec fun map_funs<E, N>(f: |E|N has copy + drop, v: vector<E>, k: u64): |E|N has copy + drop {
    if (k == 0) f else fun_post_of<map_funs(f, v, k - 1)>(v[k - 1])
}
spec map {
    pragma opaque;
    pragma verify = false;
    aborts_if exists j in 0..len(v): aborts_of<map_funs(f, v, j)>(v[j]);
    ensures len(result) == len(v);
    ensures forall j in 0..len(v): result[j] == result_of<map_funs(old(f), v, j)>(v[j]);
    ensures f == map_funs(old(f), v, len(v));
}
```

Pure lambda: the chain collapses, the pointwise ensures becomes the familiar quantified contract, no invariant needed. Capturing lambda:

```move
let sum = 0;
let w = vector::map(v, |e| { sum = sum + e; e + 1 }) proof {
    invariant(k) sum == spec_sum(v, k);
};
// yields: len(w) == len(v), forall j: w[j] == v[j] + 1, sum == spec_sum(v, len(v))
```

### `find` — capture channel with early exit

The chain bound is an expression over the call's results:

```move
spec find {
    pragma opaque;
    pragma verify = false;
    ensures f == apply_all(old(f), v, if (result_1) result_2 + 1 else len(v));
    ensures result_1 ==> result_2 < len(v)
        && result_of<apply_all(old(f), v, result_2)>(v[result_2]);
    ensures forall j in 0..(if (result_1) result_2 else len(v)):
        !result_of<apply_all(old(f), v, j)>(v[j]);
    ...
}
```

Rule unchanged; bound-mentioning obligations are placed after the call where result temporaries exist (sound — asserts are obligations). A capturing predicate `|e| { count = count + 1; *e > 10 }` supports `invariant(k) count == k`, yielding `count == i + 1` on a hit at `i`, `count == len(v)` on a miss; the `P(k, σ)` premise makes STEP self-limiting past the exit. `any`/`all`: thin wrappers over `find`.

### Degenerations

One general contract per HOF; the rule specializes per call site:

| Call site | Effective rule |
|---|---|
| pure lambda, no threading (`map`) | chain collapses; contract facts direct; **no invariant** |
| pure lambda, threading (`fold`) | induction over threading channel; one invariant |
| capturing lambda, no threading (`for_each_ref`) | induction over capture channel; one invariant |
| capturing + threading (`fold`) | induction over product state; one invariant |

## Implementation Architecture

1. **Parser**: `Exp_::WithProof(exp, spec_block)` in statement position (contextual lookahead `proof {`, like `spec lemma`); standalone form desugars to an inline `Exp_::Spec` with a proof-only member. New item `Proof_::Invariant(binder, exp)`.
2. **Model builder**: code spec blocks accept proof members (today rejected in `def_ana_code_spec_block`); the attached form lowers to a sequence placing the `ExpData::SpecBlock` *directly after* the call — positional association, as loop invariants attach to loops. Fun-argument parameter names (`acc`) are injected as scoped locals for invariant translation.
3. **Travel**: through `Bytecode::SpecBlock` and the file-format `on_impl` table as today; at stackless regeneration, proof items lower to `Prop` assert/assume (lemma `apply` via the function-level proof expansion code), and each `invariant` becomes an anchor `Prop` registered in a new `FunctionData.code_invariants` map — the `loop_invariants` pattern, so all prover passes maintain the expressions through existing `Prop` remapping.
4. **Instrumentation** (`spec_instrumentation.rs`): a *chain recognizer* matches the (caller-substituted) callee post conditions — a `fun_post_of` chain, optionally mutually recursive with a `result_of` chain — extracting bound, roots, element template, state channels, and the step target from the `Closure` operation. BASE/STEP before the call's requires handling, PREMISE before requires asserts and the abort branch, EXIT at the anchor after the capture write-back. Ordinary `Prop`s; no CFG surgery.
5. **Diagnostics**: every recognizer failure (no adjacent call, chain-less callee, unrecognized shape, memory-dependent step spec, unknown name in the invariant) is a hard error at the invariant's location — fail closed, never degrade to unsoundness.

## Relationship to the Lemma Layer

The rule subsumes caller-side use of `partial_of`/`captures_of`/`write_of` entirely: at a call site the closure literal is visible, state is named directly, no recognizers — which also sidesteps their type-generic restriction for callers of generic HOFs. Remaining lemma-layer roles: module-level lemmas *about* chains over symbolic fun values (power-user path); `write_of` for that path's step-generic folds; and, as a possible separate future feature, structural vocabulary over *value* captures for stored closures (dynamic dispatch) — the current recognizers cannot serve it (they require a `&mut`-capture witness, and such closures are not storable).

## Limitations and Future Work

- **Generic callee certification**: in a type-generic HOF's own VC the fun type is skolemized and `fun_post_of` degenerates to identity — chain ensures would certify vacuously. Until per-instantiation verification (#20278), generic chain contracts are trusted (`pragma verify = false`); monomorphic HOFs can be certified today.
- **Step spec restrictions** (V1): memory-independent one-application contracts; violations rejected.
- **`old()` in call invariants**: rejected in V1 (snapshot locals suffice).
- **Motive derivation**: the invariant is often the ensures with the bound generalized to the index (`result == spec_sum(v, len(v))` ⇒ `s == spec_sum(v, k)`). Derived candidates are *checked* by BASE/STEP, so derivation needs no trust — a future extension for calls without a proof block, reducing the common case to the specification alone.
- **Invariant inference**: WP over one abstract chain step (the step relation is a contract, not code) is a natural target for spec inference, in inference mode.
- **Auto-derived chain packages**: chain spec funs, chain ensures, and the certifying loop invariant are mechanically determined by the canonical iteration shape; synthesis would reduce the HOF author's cost to a pragma.
- **Well-foundedness of recursive lemma application** (#20275): open item of the lemma layer; the rule does not depend on it.
