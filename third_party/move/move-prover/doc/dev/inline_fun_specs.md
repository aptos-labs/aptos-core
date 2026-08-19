# Specifications for Inline Functions

This note documents how the Move Prover treats specifications on inline functions,
the rationale behind the current restrictions, and the design space for lifting them.
For the general treatment of function values and behavioral predicates, see
[fun_values_note.md](fun_values_note.md).

## The Rule

Inline functions fall into two classes for specification purposes:

1. **Inline functions without function-typed parameters** may carry a regular function
   spec block. In verify mode such a function is compiled to bytecode and its body is
   checked against its spec, like a regular function (`FunctionEnv::is_inline_verified`).
   If it additionally has `pragma opaque`, its calls are *retained* — not expanded — in
   the verification model, and call sites use the spec instead of the body, via the
   ordinary opaque machinery (`FunctionEnv::is_inline_opaque_retained`). This gives
   modular verification for inline functions that behave like regular functions, e.g.
   abstraction of a loop by a closed form. In normal (non-verify) compilation, calls
   are always expanded and the spec is unused.

2. **Inline functions with function-typed parameters** (higher-order inline functions)
   cannot carry a function spec block; the model builder rejects it. Their calls are
   always expanded, with lambda arguments beta-reduced into the body. Callers verify
   through the expansion. In-body `spec { .. }` blocks remain allowed in both classes;
   they are expanded into the caller and verified there. In-body spec blocks —
   in particular loop invariants — may constrain the behavior of the function
   parameters via *behavioral predicates*, which are inlined per expansion site
   (see below).

## Why Higher-Order Inline Functions Reject Specs

A useful spec of a higher-order function must describe the behavior of its function
parameter `f`, using behavioral predicates such as `ensures_of<f>(x, result)`. To
discharge such predicates at a call site, the prover needs a *specification for the
actual argument*, which for an inline function is a literal lambda.

An earlier design kept calls to opaque higher-order inline functions un-expanded and
lambda-lifted their lambda arguments into function values carrying the lambda's
attached (or inferred) spec, so behavioral predicates could be resolved against real
closures. This turned out to be a dead end: the lifted closures must capture the
lambda's free variables, and captures of *mutable references* cannot be supported —
a function type `|T|R` is opaque about whether its closure captured a `&mut`, so the
prover cannot model writes through such a capture at an opaque call site. A special
case admitting immutable-reference captures existed but did not extend to the mutable
case, which is the common one (e.g. a `for_each`-style lambda accumulating into an
enclosing local). Beta-reduction, in contrast, handles references of both kinds
natively, since the lambda body is spliced into the caller's scope — hence expansion
won, and specs on higher-order inline functions were withdrawn.

## Inlining Behavioral Predicates

Just as the function itself is inlined, occurrences of behavioral predicates in its
in-body spec blocks are inlined too: when a call is expanded, each behavioral
predicate whose target resolves to a lambda argument is replaced by the lambda's own
spec, substituted with the predicate's arguments (implemented in the inliner,
`env_pipeline/inliner.rs`):

- `requires_of<f>(x)` — conjunction of the lambda's `requires` (`true` if none);
- `aborts_of<f>(x)` — disjunction of the lambda's `aborts_if`;
- `ensures_of<f>(x, r)` — conjunction of the lambda's `ensures`; for a `&mut`
  parameter the canonical dual-argument form `ensures_of<f>(pre, post)` is required,
  with `old(param)` substituted by `pre` and plain `param` by `post`;
- `result_of<f>(x)` — the lambda's functional `ensures result == E`, or the
  beta-reduced body otherwise (also when a spec is attached but has no
  condition of that shape — the body stays authoritative for the value).

For a spec-less lambda, the spec is derived from the body by the **source-level
weakest-precondition analysis** (`move-model/src/spec_derivation.rs`): a forward
symbolic execution over the AST mirroring the semantics of the bytecode-level spec
inference. It covers imperative bodies (lets, assignments, `if`/`match`, mutation
through `&mut` parameters with the dual-form `old(p)`/`p` conventions), exact abort
conditions for primitive operations, modular summaries for calls (exact WP for
`std::vector` intrinsics — including the loop-implemented prover intrinsics
`index_of`, `swap_remove`, `append`, `remove`, `insert` — spec-function
substitution for pure callees, `result_of`/`ensures_of`/`aborts_of` behavioral
summaries otherwise), and global state effects (`exists` aborts, two-state
`publish`/`remove`/`update` conditions with memory labels). So that the
spec-function substitution is available *while* inline functions are expanded,
`$fun` companion spec functions for hereditarily pure Move functions are
derived by a dedicated env-pipeline stage before the inliner runs
(`spec_rewriter::run_pure_fun_companion_derivation`, verify mode only); the
later spec-rewriter pass reuses these companions, and they are emitted by the
backend only when a derived condition actually references them. A behavioral
summary of a callee which provably has *no* memory effects (memory-pure or
abort-only, established by a conservative transitive body scan,
`spec_derivation::fun_has_no_memory_effects`) is single-state: it introduces
no memory labels and keeps the derivation's exact modifies footprint. `requires_of` resolves to `true` for a body without
callee-precondition material (the analysis reserves a requires field for future
use); if the body calls a function carrying a `requires` condition, or applies a
function value whose `requires` is unknown, `requires_of` reports an error rather
than dishonestly claiming `true` (the callee preconditions themselves are still
checked at their call sites within the beta-reduced expansion). Derivation
fails for bodies with loops (no invariants are available), escaping lambda
values, or writes through references of the enclosing scope. In a loop
invariant, underivable `ensures_of` warns and weakens the complete invariant
condition to `true`; other predicates report the "add a spec block" error. On
any such failure the predicate is first left intact over the resolved lambda,
which keeps the expression well-typed (a constant would not be, for
`result_of` over a non-bool lambda); the enclosing inliner then performs the
diagnosed weakening where allowed.

There is one compositional exception to the loop restriction. An inline
`map_ref` can expose recursive `spec_map_ref` and `spec_map_ref_aborts`
summaries and use both in its loop invariants. At expansion, the inliner
specializes those recursions over the literal lambda and carries the resulting
value and abort expressions in a prover-internal `InlineCallSummary` marker.
If that expanded `map_ref` is itself inside a lambda, source derivation consumes
the marker instead of symbolically executing the inner loop. The marker is
prover-only and is removed during instrumentation; the executable expansion
also asserts that its actual result equals the carried value summary, while its
abort paths remain explicit. Thus nesting is compositional without trusting the
summary or weakening verification. See `nested_map_ref.move`.

A summarized call returning `&mut T` normally needs a place projection so
later reads or writes through the result preserve its alias. If the call is a
non-final sequence expression, however, Move discards that result immediately.
The derivation then keeps the call's abort condition and `&mut`-argument
post-values, but does not require a place for the unused result. This covers
chaining-style mutators such as `scalar_mul_assign`; consuming their returned
reference still requires a derivable projection. See
`discarded_mut_ref_result.move`.

Behavioral predicates are the *only* way to access a function value in a
specification: directly applying one (`assert f(x)` in an in-body spec block, or in
the body of a spec function) is an error.

## Anchoring Global State Effects

A derived (or attached) lambda spec with global state effects is two-state: its
conditions relate the states before and after the lambda's execution. Such
conditions are consumable at a behavioral predicate occurrence when the function
parameter has a **unique application site** in the inline function's body: the
inliner places a prover-internal `SaveStateAnchor(L)` marker between the
beta-reduced application's parameter binding and the lambda body — after the
invocation's arguments, which may themselves have global state effects, are
evaluated (spec instrumentation snapshots the referenced memories there) — and
wraps the substituted conditions in `WithStateAnchor(L)`, whose `old(..)`
references resolve to the anchored state instead of function entry. Within the
wrapper, the whole-memory effect operations of derived conditions
(`update`/`publish`/`remove`) resolve their unlabeled pre-state to the anchor
as well (the spec translator relabels their pre-ranges like `old(Global)`
reads): bound to function entry instead, they would compare against the wrong
base memory once the inline body mutated the same resource — at any address —
before the application. Abort and
requires conditions, which refer to the application's pre-state, get their
memory reads `old(..)`-wrapped under the same anchor; consequently, *any*
global state read in such a condition requires the anchor — without a unique
application site the predicate is rejected (evaluating the reads at the
assertion-site state instead would let a false `aborts_of` or `requires_of`
claim verify once memory changed in between).
An application site inside a loop, or inside a forwarding lambda passed to a
non-inline callee, is not considered unique: either can execute dynamically
more than once and overwrite the same anchor snapshot. Calls through a lambda
passed to an opaque callee are therefore rejected conservatively even when a
particular implementation happens to invoke it once.
Inside *loop invariants*, where no unique site exists, the conditions are
instead resolved against the two states an invariant can speak about (see
[below](#global-state-effects-in-loop-invariants)).

A condition may read global state *indirectly*, through a **spec function**
whose body reads memory. The state policies treat such a call like a direct
read — for pre-state conditions the *call itself* is `old(..)`-wrapped, so its
whole evaluation, arguments included, resolves at the anchored pre-state.
Nested behavioral evaluators receive the same treatment. Translating
`old(behavior)` labels both the evaluator's pre- and post-memory at that state;
labeling only its pre-memory would still let a nested `result_of` observe the
later assertion state.
Since `SpecFunDecl::used_memory`/`uses_old` are only computed by the spec
rewriter, which runs after the inliner, the detection is a transitive
inline-time scan of the spec function bodies (memoized; bodiless declarations
are fixed functions of their arguments and state-independent, and the `$fun`
companions of pure Move functions are memory-free by construction). A
**two-state spec function** — one whose body (transitively) uses `old(..)`,
e.g. a delta helper `global<R>(a).v - old(global<R>(a).v)` — is anchor
material in an `ensures`: under `WithStateAnchor` the spec translator saves
the helper's old memory at the anchor label, so its `old(..)` reads the
application's pre-state rather than function entry (in a loop invariant it
reads function entry, the invariant's own `old(..)` scope). In `requires` and
`aborts_if` conditions, which describe a single state, two-state spec
functions are rejected. See
`tests/sources/functional/closures/inline/bp_specfun_state.move`,
`two_state_spec_fun.move`, `anchor_mutation_pre_state.move`.

A single spec condition can reference several such states at once — e.g.
`assert ensures_of<f>(a) && ensures_of<g>(a)` with two state-effecting lambda
parameters contains two `WithStateAnchor` wrappers with distinct labels, possibly
next to plain `old(..)` parts denoting function entry. State saves are therefore
scoped per label: the translated spec records `(resource, label)` snapshot pairs
(the same resource may be snapshotted under several labels) and groups parameter
saves by anchor label; instrumentation emits each save at the program point its
label belongs to — the matching `SaveStateAnchor` marker for anchor labels,
function entry for the rest. The snapshots must not alias: taking f's "pre-state"
at g's marker (after f ran) would let a false two-state claim about f verify (see
`tests/sources/functional/closures/inline/anchor_aliasing.move`).

This makes generic loop invariants in higher-order inline functions verifiable at
each expansion site. For example, a `for_each_mut` over a vector can state
`invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j])`, which becomes, for a
caller passing `|e| *e = *e + 1 spec { ensures e == old(e) + 1; }`, the concrete
invariant `v[j] == old(v)[j] + 1` — proven against the beta-reduced body, so a wrong
lambda spec fails at the call site (the lambda's spec is only instantiated into
asserted conditions, never assumed). See
`tests/sources/functional/closures/inline/vector_hofs_for_each.move`.

Since substituted lambda specs are written in caller scope, behavioral predicates
they contain resolve at the next expansion level when the caller is itself inline
(transitivity is structural; it terminates because the inline call graph is acyclic).
Predicates over function-typed parameters of the enclosing non-inline function, or
over closures of named functions, stay as they are and are handled by the existing
machinery described in [fun_values_note.md](fun_values_note.md).

## Global State Effects in Loop Invariants

Per-iteration state pairs are not expressible in a loop invariant — and they are
not needed: a behavioral predicate over a state-effecting lambda is resolved by
letting pre-state reads (`old(..)`) denote *function entry*, the invariant's own
`old(..)` scope, and post-state reads the *current* state. Under this resolution
the derived whole-memory effect operations (`update`/`publish`/`remove`, which
assert "the current memory is the pre-memory with exactly this one change" and
would be false after a second iteration) are **projected to point facts** over
the current state: `update<R>(a, v)` and `publish<R>(a, v)` become
`exists<R>(a) && global<R>(a) == v`, and `remove<R>(a)` becomes `!exists<R>(a)`;
the value arguments keep their `old(..)`-wrapped pre-state reads, which now
denote entry. Abort and requires conditions get their memory reads
`old(..)`-wrapped as in the anchored case, likewise denoting entry.

The frame dropped by the projection ("nothing else changed") is recovered by a
dedicated behavioral predicate, **`unchanged_of<f>(x)`**: the memory the lambda
may write when applied to arguments `x` is unchanged relative to the pre-state.
It is built from the lambda's derived modifies footprint — for each target
`global<R>(A)` the conjunct `exists<R>(A) == old(exists<R>(A)) &&
(old(exists<R>(A)) ==> global<R>(A) == old(global<R>(A)))` — and degenerates to
`true` for pure lambdas, so one generic HOF serves value and state lambdas
alike, with the canonical invariant pattern:

```move
while (i < n) {
    f(*vector::borrow(v, i));
    i = i + 1;
} spec {
    invariant i <= n;
    invariant forall j in 0..i: ensures_of<f>(v[j]);   // point facts: pre = entry, post = current
    invariant forall j in 0..i: !aborts_of<f>(v[j]);   // memory reads at entry
    invariant forall j in i..n: unchanged_of<f>(v[j]); // suffix frame
    invariant forall x: address: (forall j in 0..i: x != v[j]) ==> unchanged_of<f>(x); // outer frame
};
```

The resolution is sound for any lambda: substituted conditions are only
*asserted* against the beta-reduced body, never assumed. Disjointness of the
per-element footprints (the callers' distinctness preconditions) is what makes
the invariants *provable*; framing of the beta-reduced body itself is free,
since Boogie's memory model sees the real per-address writes. See
`tests/sources/functional/closures/inline/state_hofs.move`.

Remaining state limitations (see `bp_inline_errors.move`):

- **Intermediate states**: two dependent global effects in one lambda leave a
  labeled (non-default) memory range after projection — the state between the
  effects cannot be named in an invariant. An `ensures_of` loop invariant is
  weakened with a warning; contexts which cannot soundly drop the condition
  still error.
- **State-dependent `result_of` values outside of loop invariants**: the value
  spliced for `result_of` — whether from an attached functional
  `ensures result == E` or derived from the body — cannot reference the
  lambda's own two-state scope there, since the state anchor is a boolean
  wrapper and cannot be applied to a value; bare (un-`old`-ed) memory reads
  are equally rejected, since spliced into a plain assertion they would be
  evaluated at the assertion-site state, letting a false claim verify once
  memory changed in between. `ensures_of` with an explicit result argument
  covers these cases (see `result_of_attached_state.move`).
- **Unknown callee footprints**: `unchanged_of` requires the modifies footprint
  derived from the lambda's own body; a lambda calling a state-mutating
  function has none. The condition is weakened with a verifier-only warning.
  (The callee's `ensures_of`/`aborts_of` summaries
  themselves *do* pass through, as nested predicates with the same
  entry/current meaning — note that `aborts_of<callee>` is a single-state
  predicate and evaluates at the current state.)
- **Spec function bodies**: specialization contexts remain single-state; a
  state-effecting predicate there has no pre-state scope to resolve against.

Relatedly, user-written `old(..)` over global state (`old(global<R>(a))`,
`old(exists<R>(a))`, and selections thereof — but not over locals) is allowed in
inline spec blocks and loop invariants, resolving to function entry. This
supports hand-written frame invariants where `unchanged_of` is not derivable
(see `for_each_addr_framed` in `state_hofs.move`).

## Capture-Accumulating Lambdas: folds_of

The essential imperative accumulation `let sum = 0;
vector::for_each_ref(v, |e| sum = sum + *e);` is out of reach for the
predicates above: the lambda's effect lives in a *captured local* which the
generic HOF invariant cannot name, and the capture's value after `i`
applications is an inductive quantity no two-state formula can express. Such a
lambda *is* a fold, with the mutable-capture tuple as implicit accumulator.
The dedicated predicate **`folds_of`** exposes this in the generic loop
invariant; it is only meaningful — and only accepted — inside a loop
invariant.

Two surface forms describe how iteration `j` obtains the target's arguments:

- **element form** `folds_of<f>(v, i)` — unary `f` applied to `v[0..i]` in
  order; iteration `j`'s argument is `v[j]`;
- **general form** `folds_of<f>(g, i)` — `f` applied to `g(0), .., g(i-1)`,
  where `g` is a *literal* index lambda `|j| <argument tuple>` (enumerate
  `|j| (j, v[j])`, zip `|j| (v1[j], v2[j])`, reversed order
  `|j| v[len(v)-1-j]`).

**Resolution.** When an expansion resolves `folds_of<f>(..)` for a lambda
argument (`resolve_folds_of_occurrences` in `env_pipeline/inliner.rs`, verify
mode only), it:

1. discovers the lambda's *mutated captures* — free variables assigned or
   mutably borrowed by the body
   (`spec_derivation::collect_mutated_free_vars`), in stable symbol order;
2. derives the per-iteration effect with the captures treated as implicit
   `&mut` parameters (`spec_derivation::derive_spec_with_captures`), yielding
   the captures' exact final values over the iteration's pre-state
   (`DerivedSpec::mut_param_values`) and the abort disjuncts;
3. builds the accumulator transformer `|acc, e| E` from the derived final
   value, with the capture's pre-state reference replaced by the accumulator
   parameter; for the general form, the index lambda's argument components
   are composed into the transformer, so it stays a restatable `|acc, j|`
   literal;
4. specializes a fold recursion over the transformer through the *regular*
   literal-lambda specialization path (previous section), so caller
   restatements with spec-equivalent lambdas unify onto the same specialized
   function;
5. records the captures' base values in verifier temporaries at an
   expansion-entry `FoldsCaptureAnchor` (dereferencing `&mut` captures), and
   routes the invariant's `old(..)` reads to that anchor. These snapshots do
   not create Move values and therefore require no `copy` or `drop` ability.

The substituted invariant is the bundle

```text
(c1, .., ck) == spec_fold$N(v, old@L(c1), .., old@L(ck), i) // the equation
&& forall j in 0..i:                                        // prefix no-abort
     !ABORT[c1..ck -> spec_fold$N(v, old@L(c1), .., old@L(ck), j), e -> v[j]]
```

where `ABORT` is the disjunction of the lambda's derived abort conditions with
the captures replaced by the fold at `j` (their value at the start of
iteration `j`) and the parameters by iteration `j`'s arguments. For a **pure
lambda** (no mutated captures) the equation and the snapshots disappear and
`folds_of` degenerates to the prefix no-abort condition alone — so one
invariant set serves every lambda class, and `folds_of` *replaces* the
point-wise `forall j in 0..i: !aborts_of<f>(..)` conjunct in generic HOF
invariants (which errors for capture-writing lambdas, see below).

**The recursions and restatement.** A *single* capture specializes a generic,
surface-declared recursion — by convention `std::vector::spec_fold` for the
element form, `std::vector::spec_fold_idx` for the general form:

```move
spec fun spec_fold<Element, Acc>(f: |Acc, &Element| Acc, v: vector<Element>, init: Acc, end: u64): Acc {
    if (end == 0) init else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
}
spec fun spec_fold_idx<Acc>(t: |Acc, u64| Acc, init: Acc, end: u64): Acc {
    if (end == 0) init else result_of<t>(spec_fold_idx(t, init, end - 1), end - 1)
}
```

The declarations are resolved by well-known lookup in `std::vector` when the
vector module provides them, with a like-named declaration of the same shape
in the expansion target's module as fallback (the prover's tests declare
their own); a resolved declaration must match the expected shape. Because the
specialization goes through the ordinary literal-lambda path, everything from
the previous sections applies: a caller condition `ensures result ==
spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v))` (qualified as
`vector::spec_fold` when resolved there) *is* the loop-exit fact and verifies
without a lemma, and an inductive bridging lemma relates the fold to a user
abstraction (`tests/sources/functional/closures/inline/folds_of.move`,
`folds_of_idx.move`).

**Multiple captures** fold as a tuple. Tuples are not expressible as spec
type arguments or lambda parameters, so no generic `Acc` instance exists;
instead a bespoke recursion with per-capture `init` parameters and a tuple
return is *generated* (`generate_multi_capture_recursion`), and unified
across expansions by spec-equivalence of the transformer material — facts
proven about one expansion apply to spec-equivalent lambdas elsewhere. The
generated recursion is unnameable from source (even in lemmas), so
multi-local-capture accumulations get concrete-length exactness only
(`folds_of_multi.move`). The supported idiom for *symbolic* facts about
coupled state is the **through-`&mut` struct accumulator**: capture one
`&mut` reference to a struct and update its fields — the accumulator is the
referenced struct value, folded by the generic (restatable) `spec_fold`, with
functional field updates as the transformer (`folds_of_ref.move`). The
capture count is bounded by the Boogie tuple maximum (8).

**Capture-aware policies of the other predicates.** For a capture-writing
lambda, `ensures_of` resolves by *dropping* the derived conjuncts that
mention a capture — a sound weakening; the capture facts are `folds_of`
material (a generic point-wise `ensures_of` invariant therefore stays
applicable). `aborts_of` errors, pointing to `folds_of`: the captures' values
at a given application are not expressible point-wise. `result_of` accepts a
derived result which is independent of every mutated capture; it errors when
the result reads a mutated capture (or cannot be derived), because that value
depends on the capture's unnameable per-application pre-state. `unchanged_of`
and `requires_of` are unaffected (capture writes do not touch memory, and
requires-material is capture-independent).

**Semantic derivation boundaries** warn and weaken the complete loop-invariant
condition to `true` (see `folds_of_errors.move`):

- *Capture writes combined with global state access*: the transformer and the
  abort conditions must be pure and single-state — their per-iteration
  evaluation points are not expressible in a loop invariant.
- *Underivable per-iteration effect*: loops in the lambda body, callees
  without a modular summary, or final capture values not expressible over the
  iteration's pre-state.
- *State-reading general-form index function*: its per-iteration evaluation
  state cannot be named by the loop invariant.
- *Accumulation through a callee* (only when no post value can be routed,
  see "Consuming callee specs" below): a lambda passing the capture (or a
  `&mut` to it) to a function call whose effect on the `&mut` parameter is
  not expressible as a value — e.g. a relational-only spec, a loop body
  without a functional ensures, or aliasing `&mut` arguments; the remedy is
  to perform the update in the lambda body, call a helper returning the new
  value, or give the callee a functional `ensures` for the `&mut` parameter.
- *Unexpressible capture post-values*: aliasing or relational summaries may
  establish conditions on the final captures without yielding values usable
  as the fold transformer's next accumulator.

**Structural boundaries** remain errors:

- *Direct parameter reassignment*: mutation through an enclosing function's
  parameter (including fields of an `&mut` parameter) is supported by
  normalizing its AST temporary to a stable derivation symbol. A direct
  assignment which instead names the parameter by symbol is still rejected:
  its reads use a temporary, and the write-only symbol cannot be
  distinguished from a shadowing local. Copy it into a local and capture
  that instead.
- *Capture abilities*: capture snapshots are verifier temporaries, so values
  without `copy` or `drop` are supported.
- *More than 8 captures* (the generated recursion returns the capture tuple).
- *`&mut` lambda parameters*: the folded iteration arguments would evolve
  with the iteration; the `_mut` HOF variants keep their dual-form
  point-wise invariant sets instead.
- *General-form index function*: must be a literal lambda producing a literal
  tuple of the target's arguments and must not depend on a captured variable
  the lambda writes (the index arguments are
  re-evaluated per iteration, so a dependency on the evolving capture would
  make them state-dependent). It also cannot reference function-typed
  parameters.
- *Outside a loop invariant*: `folds_of` is rejected in any other context.
- *Non-compositional forwarding*: mixed capture-writing/forwarding lambdas
  weaken the fold invariant with a verifier warning linked to #20383. A final
  function-value target in a non-inline function remains an error.
- *Consuming iteration*: an expansion-entry parameter anchor retains the
  logical input without constructing a Move copy. A reverse-consuming loop
  can therefore use the general form
  `folds_of<f>(|j| old(v)[len(old(v)) - 1 - j], processed)` even after the
  executable vector has popped those elements.

In non-verify compilation, conditions containing unresolved behavioral
predicates — including `folds_of` — are replaced by `true` through the
existing unresolved-predicate seam, so normal builds are unaffected.

### Consuming callee specs in fold transformers

A lambda which accumulates into a capture *through a callee* — passing the
capture or a `&mut` to it into a function call — has its per-iteration
effect known only through the callee. The body derivation routes an exact
post value `E(old(p), args)` for each `&mut` argument place of a summarized
call, in this order:

1. *Intrinsic-map WPs* (`well_known::map_intrinsic_wp`): calls bound to the
   value-level add/del map roles (`map_add_no_override`,
   `map_add_override_if_exists`, `map_del_must_exist`,
   `map_del_return_key`) get exact post values (`map_spec_set` /
   `map_spec_del`), results (`map_spec_get`, the key) and abort conditions
   (the declared abort spec functions), phrased over the map type's
   declared spec functions. This is a cascade step of the call evaluation,
   parallel to the `std::vector` intrinsic WPs.
2. *Attached functional ensures*: an unconditional `ensures` conjunct
   `p == E(old(p), args)` — or a complete set of per-field conjuncts
   `p.f == E_f`, composed via field update (the `coin::merge` shape) —
   where `E` is pure, single-state, and phrased over the pre-state
   (mentions of `&mut` parameters only under `old(..)`). Conditions marked
   `[concrete]` are invisible to callers and skipped; consuming
   `[abstract]` conditions of opaque callees rides the same trust the
   prover places in opaque specifications.
3. *Body value summary* (memoized per callee and instantiation): the
   callee's own body is analyzed by the same derivation, and its exact
   `&mut`-parameter post values are substituted — the transitive closure of
   "perform the update in a helper".

Aliasing `&mut` argument places (sharing a root) conservatively fall back
to symbolic post values, which the fold transformer cannot restate. A
callee judged free of memory effects (`fun_has_no_memory_effects`) keeps
the summary single-state; that judgment accepts exact frame conjuncts
`X == old(X)` in the callee's spec (e.g. the `supply<CoinType>` frame on
`coin::merge`), treats bodiless spec functions without used memory as
state-independent. Opaque callees without `modifies` are checked against their
bodies because their call summaries conservatively havoc unframed effects.

**Known boundary — bv-typed material:** behavioral-predicate Skolems and
evaluators are emitted with non-bv Boogie types, so consuming specs whose
values are bit-vector-marked (`pragma bv`, e.g. `std::features::set` over
its features vector) produces Boogie type mismatches. Such callees keep
the resolution error; bv-aware emission (and `Operation::Behavior`
handling in the number-operation analysis) is a follow-up.

## Forwarding Wrappers and Transitivity

Data-structure modules wrap the vector HOFs in inline HOFs of their own —
e.g. `smart_table::for_each_ref` iterates its buckets and hands each entry
to its function parameter through a small adapter lambda:

```move
public inline fun for_each_ref<K, V>(self: &SmartTable<K, V>, f: |&K, &V|) {
    for (i in 0..self.num_buckets()) {
        self.borrow_buckets().borrow(i).for_each_ref(|elem| {
            let (key, value) = elem.borrow_kv();
            f(key, value)
        });
    }
}
```

When the inner vector HOF's invariants are resolved (at the expansion of the
wrapper's *body*), the adapter's behavior is unknowable: it depends on `f`, a
function-typed parameter of the enclosing inline function. Resolution is
therefore **deferred**. The body derivation does not fail on an application
of such a parameter; it records it as a *label-free* behavioral summary
(`deferred_fun_param_temps`, `DerivedSpec::deferred_applications`), and the
substituted invariant contains behavioral predicates over `f` at the
composed argument values. Since substituted conditions are written in caller
scope, these predicates re-resolve at the *next* expansion level, against
whatever lambda the outer caller supplies — behavioral predicates are
transitive through forwarding wrappers, and multi-level forwarding is
handled structurally (the inline call graph is acyclic). A function-typed
parameter of a *non-inline* function has no further expansion level: the
predicates translate directly through the function-value machinery where an
encoding exists, and `unchanged_of` — which has none — reports an error.

The deferral is per predicate kind:

- **Point-wise predicates** (`ensures_of`, `aborts_of`, `result_of`,
  `requires_of`): the adapter must decompose as a **pure forwarder**
  (`derive_forwarded_application`): an effect-free prelude — typically
  reference projections such as `borrow_kv`, covered by the memoized
  body-summary place projections — followed by exactly one *unconditional*
  application of the parameter with exact pure argument values. The
  predicate is rewritten over `f` at the composed arguments; the prelude's
  own abort disjuncts join the deferred `aborts_of`. Mixed shapes — a
  conditional application, prelude effects, or an abort disjunct mixing
  prelude material with predicates over the parameter (result-dependent
  aborts) — do not decompose and keep the predicate's precise error.
- **`unchanged_of`**: the adapter's own memory footprint must be exact; the
  application's footprint is *delegated* as a nested `unchanged_of<f>` over
  the composed arguments, resolved (or erroring) at the outer level.
- **`folds_of`**: the occurrence is rewritten to the general form
  `folds_of<f>(|j| (A1(..), ..), i)` targeting the wrapper's parameter,
  with iteration `j`'s arguments composed through the forwarder and the
  forwarder's prelude aborts carried along (`FoldsOfDeferred`). The composed
  index components reference the wrapper's parameters as temporaries, so the
  next level's normalization splices its actual arguments through them —
  which is what makes the eventually derived transformer unify with caller
  restatements.

**Anchor markers.** The fold equation needs the captures' base values *at
the wrapper expansion's entry* — e.g. per bucket-loop iteration — but at
deferral time neither the captures nor their snapshots exist yet. The
deferred occurrence carries an **anchor label** in its `MemoryRange`, and
the expansion prepends a spec-transparent
`Operation::FoldsCaptureAnchor(label)` marker statement (the
`SaveStateAnchor` pattern) at the point where the snapshots belong. When an
outer expansion finally resolves the anchored occurrence, its generated
invariant reads the base values through `WithStateAnchor(label, old(c))`.
Spec instrumentation then records the referenced captures in verifier
temporaries at the matching marker, so each entry into the marked region
re-records them; a body post-pass erases only stale markers. Labels are
freshened per expansion of a body carrying anchors, so the same wrapper
expanded twice in one function cannot alias. In non-verify compilation the
seam is unchanged: unresolved deferred predicates are replaced by `true`.

**Per-bucket strength.** For a bucketed container, the deferred fold
resolves once per inner loop: the caller obtains the fold equation and
no-abort facts *per bucket*, with base values snapshotted at each
bucket-loop entry. A whole-table vocabulary — a flat index across buckets
with a locate lemma relating it to `(bucket, offset)` positions — is a
named follow-up.

**Boundaries**: mixed capture-writing forwarders weaken the condition with a
warning (see above). A `folds_of` target which resolves, after all expansion
levels, to a non-lambda function value and bv-typed material remain precise
errors. Relatedly, when restating a specialized fold in a caller condition,
annotate integer literals with their concrete type (e.g. `1u64`): in spec
contexts unannotated literals default to the widest type, and specialization
unification requires the instantiations of the restatement and the code
lambda to agree exactly.

## Expansion-Entry Anchoring of `old(..)` over Parameters

In the spec conditions of an inline function body, `old(p)` of a parameter
denotes the parameter's value at the *inline function's* entry — the analog
of function-entry `old(..)` in a regular function's spec. This is what the
dual-form `_mut` HOF invariants (`length(self) == length(old(self))`,
`ensures_of<f>(old(self)[j], self[j])`, the suffix frame) mean: the
enclosing function's entry state does not in general contain the receiver's
value (a field projection `&mut s.v` or a `borrow_global_mut(..)` result is
constructed at the call site). The inliner therefore places a verifier-state
anchor inside the expansion's parameter-binding block and redirects
`old(p)` through the matching `WithStateAnchor` wrapper
(`anchor_param_old_at_expansion_entry`). Spec instrumentation records the
bound parameter in a verifier temporary at that point. No Move value is
constructed, so owned and referenced parameters need neither `copy` nor
`drop`; the same mechanism works through generic forwarding expansions.

For a consuming fold, an anchored parameter read can appear in the generated
accumulator transformer. Since `old(..)` cannot remain in the recursive
single-state spec-function body, the specializer abstracts the anchored read
into a context parameter and supplies its concrete verifier-state value at
the loop invariant call. `old(..)` over anything other than a plain parameter
(state reads, body locals, and parameters of an attached lambda spec) keeps
its function-entry anchoring.

## Verification-Stage Boundaries

- **Move-value congruence of uninterpreted spec functions.** Under a
  non-extensional vector theory, two representations can be Move-equal
  (`$IsEqual`: length + pointwise) without being raw-(SMT-)equal — e.g. a
  havocked loop-state element constrained by the suffix-frame invariant
  against its snapshot — and an uninterpreted function is congruent only
  over raw equality. Fold equations and dual-form invariants routed through
  uninterpreted spec funs (`spec_apply_patch`, `spec_scalar_mul_internal`)
  would fail their induction step exactly when the argument terms differ.
  The backend therefore emits a congruence axiom per uninterpreted spec
  function with a parameter type lacking native equality:
  `$IsEqual(a, a') && .. ==> $IsEqual(f(a, ..), f(a', ..))` (see
  `generate_uninterpreted_congruence_axiom`;
  `tests/sources/functional/uninterpreted_spec_fun_congruence.move`).
  Ghost-bearing parameter types are excluded: Move equality ignores ghost
  fields, while an uninterpreted spec function can observe them, so congruence
  over `$IsEqual` would be unsound. The
  same hazard theoretically exists for *recursive* spec functions (their
  defining axioms also bind via raw argument terms) and for generated
  behavioral-predicate functions; extend the axiom there if a site
  surfaces.
- **`result_of` over natives without functional specs.** A lambda calling a
  native whose spec has no `ensures result == ..` (e.g. the ristretto255
  "mockup" specs) derives fine, but its behavioral result function is
  uninterpreted and the native's procedure result unconstrained, so
  `map`-style invariants (`result[j] == result_of<f>(self[j])`) fail the
  induction case. Natives backed by prelude `$-spec` functions
  (`std::vector`, `bcs`, `hash`, `signer`, `from_bcs`) delegate to them and
  are exact; the backend restricts the delegation to exactly that set — for
  any other native the result function stays uninterpreted rather than
  referencing an undefined Boogie function. Giving the native an
  uninterpreted-spec-fun functional ensures (the `spec_scalar_*` pattern in
  `ristretto255.spec.move`) makes such sites provable: the derivation
  routes exact result values from a callee's attached functional ensures or
  its memoized body value summary (`callee_result_value`, the result analog
  of the `&mut` post-value routing), so the pattern also carries through
  non-opaque wrappers of such natives (e.g. `ristretto255::point_compress`
  packing `point_compress_internal`).

## Specializing Spec Functions over Lambdas

Accumulation (a `fold` closed form) relates a chain of intermediate accumulator
values, which is expressed by a *recursive spec function taking the function value as
a parameter*:

```move
spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
    if (end == 0) init
    else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
}
// in fold's loop: invariant acc == spec_fold(f, v, init, i);
```

After expansion `f` is a beta-reduced lambda, not a function value, so such a call is
resolved by *specialization*: the inliner generates a monomorphic copy of the spec
function per call site in which the function parameter is eliminated — behavioral
predicates over it are substituted as described above, and calls passing it along
(including recursive calls) are redirected to their
specializations. Free variables of the lambda material become additional parameters
of the copy, named by the same symbols — freshened if that would collide with a
retained parameter or a binder in the body — and supplied at every call site.
Callers then prove facts about concrete vectors by plain unfolding of the
specialized recursion, exactly as with the function-value machinery for non-inline
functions. See `tests/sources/functional/closures/inline/vector_hofs_fold.move`.

A specialization may stem from a **generic enclosing context** — a generic
function expanding the inline HOF, a generic lemma restating its lambda: the
instantiation and the lambda material then reference the context's type
parameters. The specialized declaration is made parametric over exactly the
*mentioned* parameters, remapped to a compact index space, and call sites pass
the mentioned parameters as type arguments; the backend then emits it per
instantiation like any generic spec function (one copy for the generic
verification target, one per concrete instantiation reached through callers).
The mention set is fully determined by the instantiation and the lambda
material — the same data the unifier compares — so spec-equivalent requests
from different generic contexts share one parametric specialization and
instantiate it with their own parameters; restatements must mention the
context's type parameters at the same *indices* as the expansion they unify
with. The bespoke multi-capture recursion gets the same treatment. See
`tests/sources/functional/closures/inline/specialize_generic_caller.move`.

Specialization can fail: the target may have no body to specialize (a native or
uninterpreted spec function), or the body may use the function parameter in a way
that cannot be resolved. An error is reported and the call is left intact, with
the parameter resolved to its literal lambda, so the result stays well-typed.
Failure propagates: a specialization whose body forwards the eliminated parameter
to a call that cannot be specialized is invalid as a whole — it is registered
before its body is rewritten (to serve recursive calls), so on failure its
unifier entry is poisoned and its cache entry dropped, making every
spec-equivalent request take the same leave-intact path without a repeated
attempt or error. No registered specialization ever retains a reference to an
eliminated parameter.

## Naming the Specialization: Literal Lambdas and Unification

The specialized function is generated and thus unnameable from source, which would
make the facts established by the expansion (e.g. `result == spec_fold$N(v, 0,
len(v))` at loop exit) inexpressible in the caller's own spec: relating them to a
user-defined recursion (`spec_sum`) requires induction, which is out of SMT scope.
Two mechanisms close this gap:

1. **Literal lambda arguments**: a spec function taking a function parameter can be
   called with a *literal lambda* in any specification (a condition, a lemma, a
   proof block trigger). Such calls are specialized like lambda-bound ones.
2. **Unification**: specializations are globally unified per inliner run — the same
   spec function, instantiation, and *spec-equivalent* lambdas at the same argument
   positions resolve to the same specialized function
   (`ExpData::is_spec_equivalent`: alpha-equivalence which treats reference
   operations as transparent and disregards the instantiation of
   arithmetic/relational operators, since one side stems from code with concrete
   integer types and the other from a spec context widened to `num`). Free
   variables and enclosing-function parameters must agree in symbol resp.
   index *and* in reference-stripped type: they become the typed context
   parameters of the shared specialization, so like-named captures of
   different types (e.g. `bool` vs `u64`) denote different specializations
   (see `specialize_ctx_arg_types.move`). Attached
   lambda specs are part of the lambda's identity — they must both be absent, or
   correspond condition by condition (same kinds, spec-equivalent expressions) —
   since behavioral predicates resolve to the attached spec when present and to
   the body-derived spec otherwise: were specs ignored, the specialization's
   definition would depend on which occurrence is processed first.
   Type instantiations of the *call* must agree exactly; since spec-mode number
   inference widens to `num`, the spec side pins them with explicit type arguments,
   e.g. `spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n)`.

## What Symbolic Callers Can Verify

The proof effort a *symbolic* caller needs depends only on the shape of its spec,
in three levels:

1. **Element-wise specs: nothing extra.** HOFs whose invariants are point-wise
   quantified (`for_each_ref`, `for_each_mut`, `map`, `find`) transfer their facts
   to caller conditions of the same shape (`ensures forall i in 0..len(v): v[i] ==
   old(v)[i] + 1`) by plain first-order reasoning — no recursion is involved, so no
   induction. See `increment_all_inferred`/`clamp_all_inferred` in
   `tests/sources/functional/closures/inline/vector_hofs_for_each.move`, which
   take fully symbolic vectors.

2. **Accumulator specs phrased in `spec_fold` terms: nothing extra.** A caller
   condition `ensures result == spec_fold<u64, u64>(|acc, e| acc + e, v, 0,
   len(v))` unifies with the loop invariant's specialization and *is* the loop-exit
   fact; it verifies without any lemma. This composes upward: the caller's callers
   see a named recursion and can reason by bounded unfolding (concrete prefixes,
   single-step incremental facts).

3. **Bridging to a user abstraction: one inductive lemma per bridging equation.**
   Relating the fold to an independently defined recursion (`spec_sum`), or proving
   an inherently inductive derived property (the exact iff abort condition needs
   prefix monotonicity), is out of SMT scope and uses the prover's lemmas: an
   inductive `spec lemma` restates the code's lambda literally (unifying with the
   expansion) and equates the specialized recursion with the abstraction. Such a
   lemma is stated *once* per (lambda, abstraction) pair — not per application
   site — and reused with a one-line `apply` wherever needed; note that an
   aggregate claim about an element-wise HOF (e.g. "the sum is unchanged") is
   recursive again and belongs to this level. See
   `tests/sources/functional/closures/inline/fold_symbolic.move` for a symbolic
   `sum` with an exact abort condition and an exact postcondition in terms of the
   user's `spec_sum`, bridged by two such lemmas.

## Outlook

The restriction in (2) — no function-level spec blocks on higher-order inline
functions — could be lifted by injecting the conditions at the expansion site
(`requires` as asserts at entry, `ensures` as asserts after the body) with the same
behavioral predicate inlining applied. The expansion-entry anchoring of `old(..)`
over parameters (see above) provides the anchoring such injected asserts need for
parameter data; anchoring injected two-state *memory* conditions would additionally
need expansion-entry state labels. The injection itself is not currently
implemented.
