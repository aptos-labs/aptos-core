# Verification v2: shallow denotation over native data

> **Historical (moved here 2026-09-03).** This design is executed: V1–V3
> and V7 are done, and the native elaborated-Lean route its V6 describes
> was retired 2026-09-02 in favour of the frame-free row route designed
> and tracked in [`certifying-execution.md`](certifying-execution.md).
> The items it left open (V4, V5, and the deferred Rust shapes) are
> carried in that document's register. Kept for the rationale, the V2
> measurement, and the vacuity finding; no longer updated.

## Status and scope

**Status as of 2026-09-01: V1, V2, and V3 are done, and the whole v0
corpus verifies from a bare `verify`. V6 is in progress: field-focused
storage borrows, arbitrary nominal field chains, branches, function control,
returned-reference shapes, and invariant-driven structured loops
are on the native elaborated-Lean route. Rust-profile verification is tracked
separately, with trait/evidence denotations explicitly deferred. V4 and V5
are not started; V7 was performed 2026-09-02, before corpus coverage, by
the user's decision.**

| Status | Milestone | What remains before it is done |
|---|---|---|
| **DONE** | V1 | Combinator denotation, certifying lowering, agreement generation, native locations, storage/generic gates, and the original bare-`verify` fixture gate were implemented and passed before V3 work. |
| **DONE** | V2 | All ten v0 corpus functions verify with a bare `verify`; the comparison is recorded below. |
| **DONE** | V3 | Repeated nested mutable-call verification converges (`bump_twice` and the full five-function `VerificationV2` fixture), and the verification suites are green, including the previously red `VerificationAborts` and `VerificationStorage`. |
| **NOT STARTED** | V4 | Storage-key-dependent instantiation closure and monomorphized views. |
| **NOT STARTED** | V5 | Generic induction proof replacing emitted agreement proofs. |
| **IN PROGRESS** | V6 | Projected borrows, `.return_`/`.throw_`, dynamic/multiple/projected/global returned references with analysis-authored deaths, and invariant-driven structured loops are native and guarded against fallback. Value-carrying breaks and pattern assignments remain. |
| **IN PROGRESS** | Rust support | Three Rust-profile functions verify through the shared native route. Reference-field decoding remains open; trait/evidence denotations are deferred. |
| **DONE (2026-09-02)** | V7 | Retirement, performed before corpus coverage by the user's decision: the frame route is gone and every uncovered target is a negative check (see `certifying-execution.md`, "Retirement performed"). |

### V2 measurement (2026-08-31)

Ten functions were ported from the v0 acceptance corpus, contracts
unchanged, each attempted as a bare `verify` (fixtures:
`VerificationV2.lean`, retired with T3 into
[`Check/Verification/Corpus.lean`](../../leaner-e2e-tests/LeanerE2ETests/Check/Verification/Corpus.lean)).
All ten are automatic in v0.

| v0 source | Function | Bare `verify` |
|---|---|---|
| `Callees.lean` | `bump` | **automatic** (~3s) |
| `Callees.lean` | `bump_twice` | **automatic** (~30s) |
| `Callees.lean` | `take_and_bump` | **automatic** (~15s) |
| `Callees.lean` | `set_pair` | **automatic** |
| `Callees.lean` | `forward_set_pair` | **automatic** |
| `GlobalBorrows.lean` | `replace` | **automatic** |
| `GlobalBorrows.lean` | `read_whole` | **automatic** |
| `GlobalInv.lean` | `remove` | **automatic** (function contract; module invariants have no leaner surface yet) |
| `Account.lean` | `deposit` | **automatic** with V6's field-focused borrows (below) |
| `Account.lean` | `withdraw` | **automatic** with V6's field-focused borrows and the branch the denotation subset gained with them |

All ten meet the gate (threshold: eight); eight did when the table was
first measured, and the two `Account` functions joined them with V6's
field-focused borrows and branch support.  Verification times are seconds
per function but
still above v0 (sub-second); the residual cost is the call-boundary
write-back reconciliation and is measured, not structural (see the V3
notes below).

The V6 investigation sharpened the finding into a soundness hazard worth
recording: the LeanerLang frontend lowered `&mut X[a].f.g` to value-level
reference-typed `select` chains, a shape with no runtime meaning
(`evaluateDataOperation?` rejects reference operands, and only the
exchange path's `borrow(select(local))` trees are normalized to places by
validation).  Because `Satisfies` is partial correctness — its `undefined`
relation is empty for the M1 subset — a function stuck on such a shape
satisfies **any** contract vacuously: a deliberately false `ensures` was
"verified" against it.  Two conclusions: a bare-`verify` success is only
meaningful over LIR every node of which has executable semantics, and
preparation must reject the meaningless shape rather than classify
`data.select` executable unconditionally, which it now does.

### Implementation handoff

V1–V3 are complete. The implemented pieces:

- The V1 combinator semantics and agreement library are in
  [`../leaner-ir/LeanerIR/Proofs/Denotation.lean`](../../leaner-ir/LeanerIR/Proofs/Denotation.lean),
  the certifying lowering is in
  [`../leaner-ir/LeanerLang/Denotation.lean`](../../leaner-ir/LeanerLang/Denotation.lean),
  and the direct WP driver is in
  [`../leaner-ir/LeanerIR/Proofs/DenotationWP.lean`](../../leaner-ir/LeanerIR/Proofs/DenotationWP.lean).
- Static parameter, local, global, field, constructor, call, and borrow
  locations are selected by lowering. Dynamic loan routing uses native
  frame/state indexes and closed initial-frame/write-back equations instead
  of rediscovering locations by lookup inside a VC.
- V3 codecs and typed contract transport are implemented in
  [`../leaner-ir/LeanerIR/Proofs/Typed.lean`](../../leaner-ir/LeanerIR/Proofs/Typed.lean).
  Per-function native `Arguments`/result types, codecs, and typed
  denotations are generated by
  [`../leaner-ir/LeanerLang/Typed.lean`](../../leaner-ir/LeanerLang/Typed.lean).
  `SpecInt`, generated nominal twins, mutable arguments, and abstract generic
  carriers cross this boundary without exposing `RuntimeValue` in the
  authored typed theorem. The public runtime theorem is transported from
  the typed theorem through codec roundtrips and denotation agreement.
- Fully parametric generic functions quantify over one abstract carrier and
  certified codec. Storage-key-dependent specialization remains V4 work;
  no finite instantiation closure or trait-evidence monomorphization is
  implemented.

The V3 stabilization (2026-08-31) was one lesson applied repeatedly:
**every value must have exactly one spelling**. The convergence and
correctness failures all reduced to two spellings of the same term
coexisting — a raw kernel projection beside its named field access, `set!`
beside `setIfInBounds`, `Array.mk` beside a literal, a push-form loan
registry beside its normalized literal, an unfolded decoder beside its
roundtrip — which silently defeats shape-keyed rewrites and `omega`'s
syntactic atom identity. The fixes: reduction folds stuck kernel
projections back to named form; the reconcile set canonicalizes array
spellings before shape lemmas run; closed frame lemmas are stated over the
canonical (literal) spellings; the profile lookup in a quantified registry
is sealed and its equations are routing facts; typed storage reads that
reach the context unrouted are rewritten through their
`FamilyRepresentation`; and marker-stripping normalization runs only
inside the all-or-nothing closing attempt, so a failing clause is still
reported at its authored range. There are no hand-written fixture proofs
in these tests; the gate is a bare `verify`.

Unsupported constructs retain the old symbolic-execution route during
migration. Agreement is still assembled by the uniform generated tactic;
the generic induction proof remains V5.

This document proposes replacing the way LIR programs reach a proof
obligation. It does not change LIR, validation, the profile registries, or
the frontends. It changes what a contract is stated about, and therefore
what a verification tactic has to do.

It supersedes no existing document, but it directly reopens a decision
recorded in [`elaboration-design.md`](../elaboration-design.md): that
verification targets `FunctionMeaning` through a calculated WP over the
big-step relation. That arrangement is implemented and works; this document
argues it is the wrong long-run shape for automation, and describes the
alternative concretely enough to cost.

### What "v0" means here

Throughout this document **v0** is the original Leaner Move stack, now
deprecated and excluded from CI but still present in the tree:

- [`../move/`](../../v0/move/) — the Leaner Move surface language, its source
  semantics, contracts, `verify`, and compiler lowering. This is the stack
  whose automation rate is the benchmark below.
  - [`../move/Move/Semantics/`](../../v0/move/Move/Semantics/) — the shallow
    semantics: [`Spec.lean`](../../v0/move/Move/Semantics/Spec.lean) (the
    relational monad), [`Global.lean`](../../v0/move/Move/Semantics/Global.lean)
    (the resource store and its laws),
    [`Reference.lean`](../../v0/move/Move/Semantics/Reference.lean) (prophecy
    references).
  - [`../move/Move/Verify/`](../../v0/move/Move/Verify/) — contracts, the
    weakest-precondition rules, and the `verify` command.
  - [`../move/Move/Tests/Verification/`](../../v0/move/Move/Tests/Verification/)
    — the acceptance corpus this proposal is measured against.
- [`../move-model/`](../../v0/move-model/) — the logical model of Move stackless
  bytecode.
- [`../transpiler/`](../../v0/transpiler/) — the original Move exchange frontend.

They are reference-only: do not add functionality or tests to them, and do
not link them from current packages. Their value here is as an oracle — a
working implementation of the design this document proposes to recover.

## The problem this addresses

Verification today symbolically executes an interpreter over a quoted unit.
A function's body is data in arenas; obtaining a proof obligation means
reducing arena lookups, name resolutions, and evaluator applications inside
the tactic. That is the *deep embedding tax*, and it is paid per obligation,
forever.

The measured consequence, as of 2026-08-29:

- The leaner stack verifies **five** storage contracts (reads, `move_from`,
  a whole-resource mutable update). Each takes seconds of tactic work, and
  reaching them consumed most of a session diagnosing reduction behaviour —
  whnf not descending into projections, well-founded definitions opaque to
  reduction, unrecoverable recursion-depth exceptions aborting tactic
  sweeps, simp unable to rewrite matcher discriminants.
- The frozen v0 stack ([`../move/`](../../v0/move/)) verifies **225 functions fully
  automatically** out of 330 `verify` items — a bare `verify f` with no
  proof body. Its global-storage suites are automatic outright:
  [`Account.lean`](../../v0/move/Move/Tests/Verification/Account.lean) 2/2,
  [`GlobalBorrows.lean`](../../v0/move/Move/Tests/Verification/GlobalBorrows.lean)
  6/6, [`GlobalInv.lean`](../../v0/move/Move/Tests/Verification/GlobalInv.lean)
  5/5,
  [`GenericStorage.lean`](../../v0/move/Move/Tests/Verification/GenericStorage.lean)
  7/7 including generic resources. Manual proofs cluster only where real
  mathematical work lives
  ([`OrderedMap.lean`](../../v0/move/Move/Tests/Verification/OrderedMap.lean),
  [`Quicksort.lean`](../../v0/move/Move/Tests/Verification/Quicksort.lean),
  [`ReturnedMutRefs.lean`](../../v0/move/Move/Tests/Verification/ReturnedMutRefs.lean)).

Those tests still exist in the tree; the deprecated packages were removed
from CI, not deleted.
[`../move/Move/Tests/Verification/`](../../v0/move/Move/Tests/Verification/) is the
acceptance corpus this proposal should be measured against.

The gap is not a fixture-by-fixture deficit to be closed with more tactic
engineering. Every trap listed above is an artifact of the embedding, not of
Move semantics. v0 did not have better tactics; it had goals that did not
need them.

## Why v0 could be shallow, and what that cost it

In v0 the shallow form *was* the semantics. A Move function elaborated
directly into a Lean function over a `Spec` monad, so there was no second
model to agree with and no agreement theorem to prove. That is the whole
source of its automation advantage.

It paid for this in two places, both of which the leaner stack exists to
fix:

1. **No metatheory.** Statements quantifying over all programs — validation
   as a trust boundary, no-stuck, preservation, determinism — have no home
   in a shallow encoding, because each program is a different Lean term.
2. **An unproved gap to execution.** v0's own tree states the separation
   (see [`../CLAUDE.md`](../../CLAUDE.md), "Keep these claims separate"):
   `verify f` proves a theorem about generated source semantics; the
   compiler lowers to bytecode; *a compiler-correctness theorem connecting
   them is future work.* Additionally its store was axiomatic — a
   `ResourceStore` typeclass whose four laws were fields, with no instance
   anywhere, plus one `IndependentResourceStores` assumption per ordered
   pair of resource types.

So the naive move — make the shallow form the semantics — would recover the
automation by re-acquiring both defects, in the stack built to remove them.
It would also be worse than v0 on trust: v0's shallow term was *authored*
and readable as the program, whereas ours would be derived from data by a
large metaprogram nobody inspects.

To be precise about the criticism: it is not shallowness that costs, it is
shallowness *as the authority*. v2 also states contracts over a native
form — but through `denote_agrees` every such theorem is a theorem about
the big-step relation, which the interpreter provably implements. In v0 a
`verify f` theorem was about the shallow term and nothing else; an error in
the shallow elaboration was unfalsifiable from inside the system. The one
gap common to both stacks — deep semantics to production bytecode — is
unchanged and remains an explicit non-goal below.

## The proposal

Keep the deep semantics as the reference. Keep the interpreter as the
executable form. Add a **shallow denotation** that verification targets,
connected to the reference by a proof rather than by assumption.

Two orthogonal axes, composable and independently useful:

### Axis 1 — control: denotation of the body

A Lean function `denote` maps a validated unit and a function handle to a
Lean function in the specification monad:

```
denote : ValidatedUnit → FunctionHandle →
  Array RuntimeValue → Spec RuntimeState Failure (Array RuntimeValue)
```

built from combinators — sequencing, conditionals, loops with invariants,
checked arithmetic, the keyed storage operations, scoped mutable borrows,
and one profile-owned combinator per registered intrinsic carrying its
registered semantics. The name is *denotation*, in the
denotational-semantics sense: data to meaning. (Not *reification*, which
conventionally points the other way.) `Spec` here is relational, as in v0:
it assigns meanings, it does not execute — execution stays with the
interpreter.

The connection is proved once, by induction over LIR, at the level of the
relations themselves:

```
theorem denote_agrees : ∀ args, denote unit f args ≃ functionSpec unit f args
```

where `≃` is component-wise equivalence of the normal, abort, and
obligation relations. It must be two-sided: `aborts_if` clauses state exact
abort conditions, so a one-directional refinement would not carry them. The
`Satisfies` biconditional for every contract is then a corollary. After it,
a contract stated over `denote unit f` is a statement about the big-step
semantics, and the weakest-precondition rules fire **per combinator**
rather than per LIR node excavated from an arena. That is v0's `wp_norm`,
recovered without v0's gap.

Two design constraints on `denote` itself:

- **Definitional transparency.** The per-function cost model below rests on
  `denote unit f` reducing to its combinator term by kernel defeq. That
  rules out well-founded recursion anywhere in `denote`'s definition —
  well-founded bodies are opaque to whnf and defeq, a fact this tree has
  measured repeatedly. `denote` must be structurally recursive over the
  validated body, with anything else routed through explicit fixpoint
  combinators whose unfolding is definitional.
- **Mutual recursion.** A call inside one strongly connected component of
  the call graph cannot unfold into the callee's denotation. SCCs denote
  through a mutual fixpoint combinator over the component's function
  family; v0's `fixFamily` in
  [`Spec.lean`](../../v0/move/Move/Semantics/Spec.lean) is the reference shape.
  Cross-SCC calls unfold into the callee's denotation directly (or into its
  contract, once modular verification lands).

### Axis 2 — data: native representation

A Move `struct` becomes a Lean structure whose fields carry certified
representations — integers as range-certified subtypes, nested structs as
their own twins — rather than a `RuntimeValue`. **This axis is already
built** (2026-08-29, `LeanerLang/SpecTypes.lean`,
`LeanerIR/Proofs/Representation.lean`): twins are generated per struct from
the validated unit with an `erase`/`decode?` pair and proved roundtrips, and
global storage is a typed map per resource family tied to runtime memory by
`FamilyRepresentation`.

Its store laws are proved on the keyed map, and cross-family disjointness is
a theorem of key disequality — both improvements on v0, which assumed them.

### Composition

The axes meet in a typed per-function wrapper:

```
deposit : Address → SpecInt 64 false → Spec TypedState Failure Unit
```

related to `denote unit f` through the twins' `erase`/`decode?` roundtrips
applied to arguments and results — the same idea as `FamilyRepresentation`,
one level up from storage. This step is per function but cheap, because the
codecs, roundtrips, and typed store already exist.

Without axis 2, a shallow denotation is shallow in control flow and still
untyped in values: it removes the arena-reduction problem but not the
`RuntimeValue`-inversion problem. Without axis 1, typed values still have to
be dug out by symbolic execution — which is exactly today's situation, and
why five contracts cost a session. **Both are needed; neither subsumes the
other.**

### Generic type parameters

The authoritative LIR stays generic: `RawUnit` and `ValidatedUnit` keep one
body with type, const, lifetime, and evidence binders. Anything
instantiation-shaped happens only while deriving the proof-facing
denotation, and a specialized denotation is always a view of the generic
body with an agreement proof — never a cloned body that becomes a second
semantic authority. (This is the central monomorphization lesson of
[Dill et al., TACAS 2022](https://arxiv.org/abs/2110.08362), and it
preserves the specialization boundary in
[`rust-mir-design.md`](../rust-mir-design.md).)

**First iteration: storage-parametric parameters.** A Move type parameter
remains parametric when it occurs in arguments, results, locals, aggregate
payloads, operations, or generic calls. It requires specialization only when
it contributes to the resource-family component of a global-storage key
(`contains`, borrow, take, or publish), including when that dependence is
passed transitively through direct calls. This is not *phantomness*:
`carry<T>(value : T) : T` uses `T` throughout and is nevertheless
parametric.

A storage-parametric parameter is treated as a given abstract, inhabited
type. The generated theorem quantifies over that carrier, so it transfers to
every concrete instantiation — which is also why substituting a convenient
concrete type without such a theorem would be unsound. Preparation rejects a
storage-key-dependent use with an explicit diagnostic rather than silently
using one symbolic resource family for all instantiations.

**Deferred (V4): storage-key-dependent parameters.** Such an instantiation
gets a specialized denotation keyed by the tuple of resource-key-relevant
type arguments. Rust trait implementation evidence later adds another
semantic specialization axis: trait selection is instance-dependent by
default even when no global resource key is involved. Preparation computes
the reachable instance graph from entry points, closes it under calls, and
proves the finite family together; it specializes reachable tuples, not a
Cartesian product. A configuration whose instance closure is not finite
(polymorphic recursion, open entry points) must retain a parametric theorem
or an explicit finite coverage certificate — never a silent generalization
from the instances that happened to be seen. Merging evidence combinations
is a later optimization, licensed only by a checked semantic equivalence.

### One semantics still holds

[`prophetic-references.md`](../prophetic-references.md) settles that the
interpreter, the big-step relation, and the verifier run one model. This
proposal does not add a second: the denotation is a *derived view* with a
proved connection, exactly as the typed store is a derived view of global
memory. What changes is which view carries the automation. The interpreter
and the big-step relation keep their current roles — execution (including
the MonoVM differential link) and reference semantics respectively.

### Coexistence during migration

Until V7 both routes were live: a construct the denotation did not yet
carry verified through the earlier symbolic execution. Since 2026-09-02
only the native route exists; a construct it does not carry is a negative
check whose expectation names it.

## Cost, and where it lands

Three ways to obtain the connection, by where the cost falls:

| Route | One-time | Per function / instance class | Fragility |
|---|---|---|---|
| Status quo (symbolic execution) | — | per *obligation* tactic search | high, unpredictable — 10-minute hangs observed |
| Emitted proof terms | certifying generator | linear proof check | low; no search |
| Verified denotation | induction proof | one `rfl` | lowest |

The third is the endpoint. Per function (or per instance class, under V4)
the cost collapses to a single kernel defeq check — the pattern the tree
already uses successfully for `semantics_eq` on the quoted prepared unit —
and no tactic searches for anything, ever. That check is only available
because of the definitional-transparency constraint on `denote` above; give
that up and the third row silently degrades into the first. The one-time
cost is the induction proof, and that is the honest price: weeks, not days.

The first row is the current cost and it does not amortize: it recurs for
every contract, every loop invariant, every arithmetic side condition.

## Milestones

Staged so the premise is tested before the largest investment is made.

- **DONE — V1: Combinator library and denotation for a subset.** The `Spec`
  combinators (v0's [`Spec.lean`](../../v0/move/Move/Semantics/Spec.lean),
  [`Global.lean`](../../v0/move/Move/Semantics/Global.lean), and
  [`Reference.lean`](../../v0/move/Move/Semantics/Reference.lean) are the
  reference shapes), and `denote` over straight-line bodies, storage
  operations, calls (cross-SCC unfolding only), and the **whole-resource
  scoped mutable borrow** (`withBorrowMutSpec` shape) — required because
  the gate fixtures use it. Generic parameters are accepted as abstract
  inhabited carriers unless they flow into global resource keys; ordinary
  generic calls remain parametric. Agreement is proved by a uniform tactic,
  not yet by induction. *Gate:* the storage contracts in
  `LeanerLang/Tests/VerificationStorage.lean` — the `&mut` deposit
  included — verify through the denotation with the generated script
  reduced to combinator-level wp rules. One parametric fixture reached at
  two concrete type arguments and through a generic forwarding call shares
  one denotation proof, while a storage-key-dependent fixture is rejected
  for V4 specialization.
- **DONE — V2: Automation measurement.** Ten functions from
  [`../move/Move/Tests/Verification/`](../../v0/move/Move/Tests/Verification/)
  (`Account`, `GlobalBorrows`, `GlobalInv`, `Callees`) — all ten automatic
  in v0 — are ported, and all ten verify with a bare `verify f`. The gate
  was a written comparison against v0 on the same functions, recorded in
  the measurement table above; it could have refuted the proposal at fewer
  than eight, and did not.
- **DONE — V3: Typed wrappers.** Typed per-function signatures related to
  the denotation through the twins' roundtrips. A function outside the
  denotation subset has no typed wrapper and verifies on the generic
  route; that is a subset question, tracked under V6, not a gap here.
- **NOT STARTED — V4: Instantiation closure and monomorphized views.** The deferred
  second step for generics: classify storage-key-dependent parameters,
  compute the finite reachable tuples, and generate one mutually connected
  denotation family per instance graph. *Gate:* a parametric body used at
  two concrete types still has one proof; a storage-key-dependent body
  produces exactly the reachable specialization tuples; an uncovered or
  infinite configuration is rejected unless accompanied by a parametric
  theorem or explicit coverage certificate.
- **NOT STARTED — V5: Verified denotation.** Replace generated agreement proofs with the
  generic induction proof; each function/instance-class obligation becomes
  `rfl`.
- **IN PROGRESS — V6: References beyond whole resources and loops.**
  Field-focused borrows came first. State as of 2026-09-01, done items
  before remaining ones:
  - **DONE — Canonical lowering.** `&mut X[a].f.g` lowers to the shape the
    runtime place machinery executes: the whole-resource global borrow is
    bound to a synthesized holder local (`pushTemporaryLocal`; body
    temporaries index past every declared local and are appended by the
    function driver), and the focus is a place borrow through
    `deref/field…` projections of the holder.  Value-level reference-typed
    select chains are no longer emitted for the mutable case; the shared
    case keeps them, which is sound because preparation erases shared
    references to their referents.
  - **DONE — Exit reconciliation.** A loan with no recorded death
    reconciles at function exit.  `exportFrameLoans` settles in-frame
    holes before exporting (`settleFrameLoans`): each round moves one
    resting current into its in-frame hole, so a holder that escapes
    through an outer loan carries the focused mutations.  Previously an
    escaping holder exported its stale hole into global memory.
  - **DONE — Focused mutation certificate.**
    `updateBorrowValue?_focusedBorrowPair` closes the mutate step for the
    holder/focus frame shape and is registered with the drive.
  - **DONE — Focused export certificate.**
    `exportFrameLoans_focusedGlobalNominal` closes finalization for the
    settled holder/focus shape; `*_focusedBorrowPairSaved` and
    `*_focusedGlobalNominalSaved` are its variants for a body that saves
    the read in a local before mutating.
  - **DONE — Closing over quantified and existential obligations.** Two
    generated-script gaps surfaced with the focused shape and are fixed
    for every function: a frame clause is universally quantified over the
    keys it does not modify, and the closing had no way to open a
    quantifier, so it never reached the keyed-map laws; and the
    obligation leaves a clause reads sit under the contract's argument-row
    existential, which the normalization passes run before witness
    instantiation cannot enter, so the closing now normalizes once more
    after instantiation.  The raw script also gained the module decoders
    the typed one already carried.
  - **DONE — `deposit`.** `&mut Balance[addr].balance.value` followed by a
    checked add through the reference verifies from a bare `verify` with
    `requires`/`ensures`/`aborts_if`/`modifies`, and the deliberately
    false-`ensures` twin correctly fails.
  - **DONE — Execution gate.** Preparation rejects a `data.select` or
    `data.selectVariants` whose operand type is a mutable reference: the
    evaluator reads a nominal value, while a mutable reference is a live
    loan whose referent may hold a focused hole.  A frontend that wants
    the field must reborrow through a place.  Typing still accepts the
    shape, because a specification reads it.
  - **DONE — Loan reconciliation order.** Several loans can end at one
    program point, and the certificate lists them in minting order.
    Reconciling them in that order writes a holder back while it still
    carries the hole of a reborrow into it, so the hole escapes into global
    memory and the focused write lands in `pending` instead of the
    resource.  `endLoans?` now reconciles in decreasing instance order: a
    reborrow is minted after the loan it projects, so that order settles a
    hole before the value holding it is written back.  An execution test
    over the shape found this; no proof did, because none of them reaches a
    `move_from` after a focused write.
  - **DONE — Branches in the shallow denotation.** `nativeBranch` runs the
    condition once and lets the boolean it produced select the arm, with a
    unit result where there is no else; `wpExpr_nativeBranch` transforms
    the condition and its own postcondition chooses the arm, so no
    evaluator equation survives for the driver to reconstruct.  A branch in
    statement position is now on the shallow route (`Verification.raise`).
  - **DONE — Native function control.** `nativeReturn` and `nativeThrow`
    evaluate their native value rows and raise the corresponding control;
    their WP and agreement theorems keep branches that return or abort on
    the shallow route. `withdraw` now verifies natively through its guarded
    abort rather than retaining generic symbolic-execution coverage.
  - **DONE — Generated decoders are simp-owned.** Symbolic execution that
    unfolded a twin's `decode?` split the range test of every certified
    integer it reached, and the branch assuming the test failed could not
    be refuted afterwards — a `SpecInt` nested inside a twin never becomes
    a range hypothesis.  The decoders are sealed after their round-trips
    are proved, so they stay folded until the closing executes them against
    the range facts the contract carries.  The benchmark shows no cost.
  - **DONE — Corpus.** `deposit` and `withdraw` are in the V2 fixture and
    the measurement table reads ten of ten; `withdraw` is also a benchmark
    target, as the branch-over-focused-borrow cost class. All ten V2
    functions carry `#leaner_require_native` gates, so generic verification
    cannot silently satisfy the fixture.
  - **DONE — Projected borrows and frame materialization.** The native
    reborrow descriptor now carries an arbitrary row of nominal field
    steps, and agreement uses a proof-only `DerefLocalFieldPath`; the
    executable descriptor retains only the local and resolved field row.
    Stable `initialLocals` plus closed four- and five-local frame equations
    enter the account bodies without exposing dependent array indexes.
    `deposit` and `withdraw` therefore use the native descriptor end to end.
  - **DONE — Scalar returned reborrow across calls.** Reference-typed
    results have native codecs, callee finalization exports
    `(outerLoan, loanHole returnedLoan)`, and caller reconciliation obtains
    the transferred identity from that hole. The contract ties the source
    parameter's visible call-boundary value to the returned current; a
    negative regression proves an unrelated existential exit value cannot
    establish a false clause. `VerificationReferences.lean` covers the
    returning callee, forwarding the reference through a second function
    boundary, mutation through both the directly returned and forwarded
    reference, and resuming the original parameter after the returned loan
    dies. Every function is guarded by `#leaner_require_native`.
  - **DONE — Path-free prophecy and explicit death.** A
    runtime mutable reference remains exactly `borrow loan current`; it has
    no owner root or projection path. The `loanHole` in the exported value
    transfers the dynamic identity across a call. Borrow analysis records
    `LoanDeath` values at normal fallthrough, explicit return, and throw, and
    semantic preparation materializes them as explicit `endLoan` operations.
    Loans carried by a normal result are deliberately excluded from the
    callee's deaths so the call boundary can transfer them. The native WP has
    closed death certificates rather than unfolding the generic loan fold.
    `RuntimeFrame.loanLocations` is a
    validated local execution cache with semantic scan fallback, and
    `RuntimeState.globalLoans` is a keyed global write-back registry. Neither
    is part of the reference value. Local returned-loan transfer derives its
    identity solely from prophecy holes; a returned global reborrow separately
    transfers the storage key in `globalLoans` from the completed outer loan
    to the returned loan. No reference stores an owner root or projection path.
  - **DONE — Native-only performance gate.** The performance suite measures
    `reborrow`, `forward_reborrow`, `set_then_read`, and
    `set_through_forward`, including returned-reference transfer across two
    call boundaries. Returned-reference equations are dispatched only for
    matching shapes rather than added to every generic reconciliation pass;
    ordinary repeated calls remain within 1% of their prior baseline. The
    explicit lifetime model deliberately resets the storage baseline:
    `replace` is 161.7M typed heartbeats for one global death, `deposit` is
    503.0M for paired projected deaths, and `withdraw` is 915.6M across its
    normal and abort paths (respectively 35%, 38%, and 24% over the pre-death
    baseline). The driver refuses to unfold the generic loan fold when a
    closed native death certificate does not match, so these measured costs
    cannot hide symbolic fallback. Future changes are gated against this
    correctness baseline.
  - **DONE — Returned-reference breadth.** The native reference fixture now
    covers two mutable inputs with a dynamically selected result, a pair of
    mutable results and mutation through both, a projected field result and
    its caller, and a field reference returned from global storage and its
    caller. Together with scalar forwarding this is twelve positive functions,
    all guarded by `#leaner_require_native`, plus the negative prophecy test.
    Transfer continues to use prophecy holes and analysis-authored deaths;
    global ownership metadata is rekeyed in the separate registry rather than
    encoded as a path in the reference.
  - **DONE — Invariant-driven structured loops.** `nativeLoop` is the least
    finite inductive relation for repeat, `continue`, `break`, return, and
    throw. Its agreement theorem is bidirectional with the big-step loop, and
    its WP theorem proves a generated invariant initially and after one native
    body step, then inducts over the finite relation. It does not symbolically
    unroll an expression id and carries no verification fuel. The expression
    id is only a static proof tag selecting the generated predicate; runtime
    evaluation is the elaborated `NativeLoop body` relation and performs no
    arena lookup. Contract generation translates each authored loop invariant
    into a native predicate over the entry/current frame and state, records the
    complete local row, anchors immutable locals to their entry slots, and
    preserves reference loan identities in the encoded local shape. The native
    subset includes local assignment, valueless `break`, `continue`, embedded
    invariant specs, and branches. `VerificationLoops.lean` proves both normal
    repetition and explicit `continue` versions of `count_to`, including
    `result == limit`, with native-only gates.
  - **NOT STARTED — Remaining loop forms.** Value-carrying breaks and pattern
    assignments remain outside the native loop subset.
- **IN PROGRESS — Rust support.** Rust-profile verification uses the same
  shared LIR and native proof route as Move.
  - **DONE — Native verification fixture.** `VerificationRust.lean` proves
    `replace`, `sum`, and `increment` with bare `verify`; the mutable-reference
    and scalar functions share the ordinary native verifier.
  - **DONE — Target pointer width.** Generated width facts compare the closed
    supported spelling instead of asking the kernel to reduce
    `String.toNat?`, so Rust units elaborate without a profile-specific proof
    escape hatch.
  - **DONE — Profile-correct arithmetic.** Rust `u32` addition verifies with
    modular contracts and no abort. A Move-shaped checked-arithmetic abort
    contract is correctly unprovable for the same Rust operation.
  - **NOT STARTED — Reference-field decoding.** Reading an integer field from
    a struct passed by reference still strands a decoder range decision. The
    drive must keep a certified twin decoder sealed rather than split a
    `Decidable.rec`; adding the nested range as a high-priority simp fact did
    not close the goal and increased `deposit` search by 36%.
  - **DEFERRED — Traits and evidence.** Trait-call denotations, implementation
    evidence, and their specialization axis have not started and remain
    intentionally outside the current Rust native-verification subset.
- **DONE (2026-09-02) — V7: Retirement.** Performed before the corpus is
  carried, by the user's decision; what was removed is recorded in
  `certifying-execution.md` under "Retirement performed".

## What this makes redundant, and what it keeps

**Redundant on success:** most of `Proofs/WP.lean`'s symbolic-execution
machinery — `deepWhnf` with its seal inventories, `leaner_drive`,
`leaner_storage`'s stepping integration, `leaner_flatten`.

**Kept:** the fueled interpreter and its soundness proof — it remains the
executable form and the MonoVM differential-testing anchor; validation and
its certificates (unchanged, and the borrow certificate becomes *more*
load-bearing — it licenses the scoped-borrow recovery); the big-step
relation as reference semantics; all LIR metatheory; the typed data
representation from 2026-08-29 in full; `buildContract`, whose spec-side
translation is already shallow and already typed.

## Non-goals

- Replacing LIR, validation, or the frontend interchange format.
- Removing the big-step relation or the interpreter. Deleting either would
  re-acquire v0's gaps, which are the reason not to take the naive route.
- A bytecode-correctness theorem. Still out of scope, still unproved, still
  must not be described as delivered.

## Open questions

1. **Profile parameterization.** The combinator library must serve the Rust
   profile too, not just Move; intrinsic combinators are the profile-owned
   part. Which combinators are core and which are profile-owned mirrors the
   existing core/profile split in [`lir-design.md`](../lir-design.md).
2. **Specialization equivalence classes.** V4 starts conservatively with one
   key per semantically distinct argument/evidence tuple. Which tuples can be
   merged by representation equality, implementation-contract equality, or a
   relational parametricity theorem without making proof search harder?
3. **Whether V5 is reachable.** The induction proof over LIR is the largest
   single item here and has not been scoped. V1–V4 are useful even if V5
   proves impractical, because emitted proof terms are an acceptable second
   place.

## Testing requirements

- Every milestone gate is a count of functions verifying with a bare
  `verify f`, compared against the v0 rate on the same functions. Automation
  rate is the metric this proposal exists to move; tactic-time
  improvements alone do not satisfy a gate.
- [`../move/Move/Tests/Verification/`](../../v0/move/Move/Tests/Verification/) is
  the acceptance corpus. Port, do not
  rewrite: a ported test that needs a manual proof where v0 needed none is a
  finding to record, not a test to weaken.
- Generic tests record which parameters are storage-parametric and, from V4
  on, the computed resource-key monomorphization keys and the number of
  generated agreement proofs. Baselines must distinguish one shared
  parametric proof, the exact finite set of storage-key-dependent
  combinations, and an explicit rejection for uncovered configurations; an
  accidental Cartesian explosion is a regression even when every generated
  proof succeeds.
- The four-suite matrix (`leaner-ir`, `leaner-move`, `leaner-rust`,
  `leaner-e2e-tests`) stays green throughout; no milestone may land red.
