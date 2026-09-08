# One denotation, one agreement proof (V5)

Status: current verification design, decided 2026-09-08. It replaces
[`historical/certifying-execution.md`](historical/certifying-execution.md)
(the frame-free row route) and
[`historical/generic-route.md`](historical/generic-route.md) (normalization,
then the native route), and executes milestone V5 of
[`historical/verification-v2.md`](historical/verification-v2.md). Progress
is measured by the check ledger of
[`test-organization.md`](test-organization.md).

## Status (checkpoint 2026-09-08)

Implemented in `leaner-ir`, suspended at this checkpoint with the tree
consistent: builds, the `DenotePerformance` gate, and the check ledger of
[`test-organization.md`](test-organization.md) (33 of 61 files exact) are
as recorded there. The agreement theorem is **assumed**, by the user's
decision, until its induction is done.

### Modules

- `LeanerIR/Proofs/Denote/`: `Types` (native carriers, rows, codecs, the
  state operations and their wp rules), `Term` (typed terms and
  `Term.denote`), `Compile` (`compileFunction`, structural on fuel),
  `Agreement` (the named axiom `compileFunction_agrees` and the transport
  to `SatisfiesFunction`), `Close` (the closer, the `lir_denote` and
  `lir_denote_norm` simp sets, the state-fact and leaf tactics).
- `LeanerLang/Verify.lean` owns `verify`, `#leaner_verify`, and
  `#leaner_require_native[_all]`. Per target it publishes the quoted term,
  the kernel `rfl` certificate of its compilation, `typedVerified` over the
  denotation, and `verified` over the big-step meaning; per struct twin it
  generates the two bridge theorems (`erase_eq_encode`, `decode?_fields`).
  `Contract.lean` keeps the clause translator and preparation.
- Gate: `LeanerLang/Tests/DenotePerformance.{lean,exp}` records heartbeats
  and proof objects per target (straight-line 1.7–6M, loops about 20M,
  the three-variant `match` with a matching contract 35M). The e2e check
  driver caps a target at 180k verification heartbeats.

### Carried

Scalars and checked arithmetic, comparisons, Boolean operations, unsigned
`&`, checked shifts and casts, constants, `if`, `let`, blocks, `abort`,
`assert`, early `return`, assignment, `break`/`continue` (labeled too),
loops with `invariant` clauses (an automatic frame ties every immutable
local available at the header to its entry value), direct monomorphic
calls through the callee's published theorem, tuples, structs, enums with
variant tests and payload selection (a `match` arrives as those), shared
borrows of locals, mutable references (`NTy.ref`: a loan with the native
current value; parameters, in-place mutation, typed projection paths,
local lenders, reborrowed call arguments settled from the callee's
exports), and storage over the runtime's keyed global map (`globalRead`,
`globalContains`, `globalBorrow`, `globalPublish`, `globalTake`, with a
mutable global borrow leaving the runtime's hole and its death marker
writing back by key). A rejection names the construct.

### Not carried

`|`, `^`, signed bitwise, vectors (every "type of a local is not carried"
row of the ledger), generic calls, constructors, and fields, recursion
(a callee is verified before its callers; `drain`, `recursive_choose`),
unspecified pure callees used as summaries (`plus_one`), a mutable borrow
outside a binding or a call argument (`reborrow`), returned references,
loops over a live global borrow (`Language/Loops` `drain`), resource
invariants (`GlobalInv`, `CrossInv`), the Rust profile's primitives, and
nested destructuring patterns. `Verification/Typed` and `EnumRefs` still
assert names of the retired route.

### Rules the implementation settled

- The closer is a worklist: a `wp` over a marked loop takes the loop rule,
  over a callee the callee's theorem, over the recursive iteration the loop
  hypothesis; a syntactic binder, conjunction, or conditional splits; only
  a goal whose head is `wp` is renormalized; a call's post hypotheses are
  consumed before renormalizing (specialize decided implications, split
  existentials, substitute witnesses, rewrite with state equations).
- A leaf clears every computation hypothesis first (`contradiction` had
  reduced the whole unit through the preparation hypothesis), adds the
  bounds of every range certificate in context as separate facts (never
  rewrites the certificate a term depends on: the cast it leaves is not
  typed at the transparency simp matches at), saturates with the context,
  splits the range-check conditionals, and decides by `omega`/`decide`.
  Signed quotients and remainders use width-specific range facts, not
  magnitudes.
- Inventories are simp sets, never explicit lists (0.8M heartbeats per
  call otherwise). `HList` and `variantCarrier` are not reducible, so
  every lemma whose right-hand side builds a row value is stated at the
  row type, never at the product it unfolds to.
- Rows are a mutual family `NTy`/`NRow`/`NRows` (a nested inductive cannot
  derive decidable equality); an enum carries the distinctness of its
  variant names. Aggregate arguments are introduced destructured, one goal
  per variant. `do`-notation never appears in a denotation definition.

Assumption ledger: `LeanerIR.Proofs.Denote.compileFunction_agrees` is an
axiom; every `verified` theorem names it under `#print axioms`. Nothing
else is admitted.

### Next

In order: resource invariants and loops over live global borrows (D2),
vectors, recursion and unspecified callee summaries, returned references,
generics (D3), then the retirement of the previous routes' generators,
tests, and `Performance.exp` (D4). Discharging the axiom blocks nothing
else and is scheduled with D4.

## What does not change

- **`BigStep` is the authority.** A `verify f` theorem is a theorem about
  the big-step relation of validated LIR, connected by proof, never by
  assumption. The elaborated LeanerLang term is produced by frontends and
  a metaprogram, so a theorem only about the elaborator's output would have
  an unfalsifiable trust root; that is why v2 rejected shallow-as-authority,
  and it still holds.
- **One semantics.** The interpreter, the big-step relation, and the
  verifier run one model, including the prophecy encoding of references in
  [`prophetic-references.md`](prophetic-references.md). The interpreter
  stays the executable form and the MonoVM differential anchor; its
  soundness and fuel completeness are unchanged.
- **Vacuity.** A bare `verify` success is meaningful only over LIR every
  node of which has executable semantics. Preparation rejects the rest.
- **The three claims** in [`../CLAUDE.md`](../CLAUDE.md) stay separate:
  source verification, lowering to bytecode, and the unproved
  compiler-correctness theorem between them.

Decision (user, 2026-09-08): stop porting acceptance files onto the current
native route and build V5 from
[`historical/verification-v2.md`](historical/verification-v2.md) instead.
The proven path stays: `BigStep` is the authority and every `verify`
theorem must be a theorem about it. What changes is how that connection is
established.

## Principles that yield performance

v0 verified 225 functions automatically at sub-second cost each. Every
route since was slower, and the perf audit
([`historical/verification-perf-audit.md`](historical/verification-perf-audit.md)) and v0's own
analysis ([`../move/Move/performance-analysis.md`](../v0/move/Move/performance-analysis.md))
attribute the difference to the same handful of causes. The principles
below are the inverse of those causes. Each names its evidence and how it
is checked, and D0 fails if a principle is violated, whatever the
measured number.

1. **The goal contains the mathematics, not the machinery.** The term a
   `verify` reasons over mentions only what the source mentions: values of
   native carriers, Lean binders for locals, prophecy values for `&mut`,
   the typed family store for globals. No `RuntimeFrame`, row, loan
   registry, arena, expression id, or string occurs in it. *Evidence:*
   `deposit` was term-bound because the focused loan, the registry, and the
   keyed hole rode through every goal (audit F4, F7); v0's goals carried
   none of that. *Check:* the elaborator audits the constants of the
   unfolded term and rejects a target whose term mentions any of them.

2. **Translate once, before any obligation; never symbolically execute.**
   All computation over program data happens in the definitional unfold of
   `denote`, which finishes before the first obligation exists. An
   obligation never contains `denote`, an evaluator, or an arena lookup.
   *Evidence:* the deep-embedding tax of v2, paid per obligation, and every
   `whnf`/seal trap in the audit. *Check:* the unfolded term contains no
   `denote` or interpreter constant; `DenotePerformance.exp` reports unfold cost
   separately from closing cost.

3. **Weakest preconditions, never unfolded relations.** One `wp` rule per
   combinator, `wp (bind a f) ↔ wp a (fun v => wp (f v))`: no existential,
   each sub-term once, linear in the body. Well-definedness is structural
   and never unfolded. *Evidence:* v0's strategy 1, which made its
   automatic `verify` wp-based, is what made it automatic. *Check:* the wp
   rules are one lemma per combinator in a named simp set, and the
   obligation count equals the exit paths plus the explicit invariant
   obligations.

4. **One traversal, one context.** The body is traversed once, split by
   exit path, not three times for `ok`, `aborts`, and `undefined`.
   Context-wide passes (`simp_all`, `subst_vars`) run only on leaf goals
   over the leaf's own context. *Evidence:* `bump_twice` was search-bound
   at 25k heartbeats per proof object because closing passes reprocessed
   the whole context per residual goal (audit F1, F1c). *Check:* heartbeats
   per proof object in `DenotePerformance.exp`, gated per target.

5. **Modularity by contract.** A call contributes its callee's contract as
   a native equation at the boundary, never the callee's body, and a
   generic body is proved once and instantiated. *Evidence:* the generic
   language checkpoint cut a caller from over 50M heartbeats to 14M by not
   re-verifying its callee. *Check:* a caller's cost is independent of its
   callee's body size.

6. **Decidable leaves, structural everything else.** Leaf goals are linear
   arithmetic over `Int` with range certificates, closed by `omega`, or
   finite enumerations, closed by `decide`. No tactic searches or
   backtracks over obligation structure, and no closer matches goal
   shapes. *Evidence:* `Certify.lean` rejecting `0 < args.item.val` as an
   unsupported shape. *Check:* the closer is the simp inventory followed by
   `omega`/`decide`/`grind`, with no route selection.

7. **Numeric identity in hot paths.** Ids, storage keys, and variant
   indices are `Nat`; every literal has one spelling. *Evidence:* audit F2
   (string identity in hot comparisons), F3 (array spelling
   multiplicity), F5 (derived `BEq` on nested inductives is `partial`).
   *Check:* no `String` and no `Array.mk`/`List.toArray` mix in the
   unfolded term.

8. **Precomputed inventories, stable keys.** Every simp inventory a proof
   uses is a registered simp set (`lir_denote`, `lir_denote_norm`),
   never a long explicit list: `simp only [list]` re-elaborates every
   entry per call, and that cost was a constant 0.8M heartbeats per
   target when the normal-form list grew. The types that key rewrite
   lemmas (`HList`, `variantCarrier`) are not reducible: a reducible key
   is reduced in goals but stuck in lemmas, and the rewrite silently
   stops matching. *Check:* the normalizers name only per-target
   constants and simprocs beyond the sets, and a lemma over a row or
   variant value is exercised by an aggregate target of the gate.

9. **Measured against v0, per target.** `DenotePerformance.exp` records
   heartbeats, proof objects, and unfold cost per target; the gate is v0's
   per-function time for the same contract, and a regeneration names only
   the targets a change was meant to move.

Principles 1, 2, 3, and 6 are architectural: they hold or fail by
construction of `denote` and the closer, and are what the previous routes
violated. Principles 4, 5, 7, 8, and 9 are engineering discipline the
routes had partly recovered and must not lose again.

## What went wrong

v2 asked for a single generic denotation of validated LIR into `Spec`,
connected to `BigStep` by one agreement theorem proved by induction (V5).
Per-target emitted agreement proofs were the interim "acceptable second
place" until V5 landed. V5 was carried, never scoped, and never attempted;
the interim became the architecture. Three route rewrites followed (script,
normalize/compose, native), and each re-did the same three things per
construct:

1. a native combinator, generated per target by a shape-matching
   metaprogram (`LeanerLang/Native*.lean`);
2. an agreement law connecting it to the frame model
   (`Proofs/*Agreement.lean`, assembled per target into
   `computationRepresents` by `LeanerLang/Computation.lean`);
3. closer support for the goal shape the new combinator produces
   (`Proofs/Certify.lean`, which rejects a goal it does not recognize —
   `0 < args.item.val`, closed by `omega` — as an unsupported shape).

Because each route's agreement library was incomplete, each route needed
the previous one for what it did not cover. That is the "fallback" work of
the last week, and removing the fallback exposed the dependency rather
than removing it: 42 of 61 Check files fail because their constructs lack
one of the three per-construct pieces. The proof-facing `Proofs/` tree is
41k lines and covers less than v0's 15k-line semantics and verifier.

## The design

Two definitions and one theorem, all written once:

```
denote : ExecutableUnit → FunctionHandle →
  Array RuntimeValue → Spec RuntimeState Failure (Array RuntimeValue)

denote_agrees : ∀ unit function arguments,
  Spec.Equiv (denote unit function arguments) (meaning unit function arguments)
```

- `denote` is a Lean function **of the validated unit itself**, recursing
  over the body's expression arena (`Validation/IndexedArena.lean`) by
  structural recursion on a fuel bounded by the arena size, the pattern of
  `Proofs/Fuel.lean`, or over a tree reified once from the arena by a
  checked function — D0 decides which. Not by well-founded recursion,
  which does not reduce, so that the kernel and `simp` can unfold it on
  concrete data. Every LIR construct is one case. `Proofs/Tree.lean` is
  not this: it is the literal tree the current generator emits *per
  target* and its once-proved induction only connects that tree's two
  readings; the tree's relation to the LIR body is still proved per target
  from the combinator lemmas. V5 moves that per-target step into the
  definition of `denote`. Calls consume the callee
  through the oracle of `Proofs/Recursion.lean`; recursive SCCs are the
  least fixed point already defined there (`fixBody`); loops are the
  `Runs`/`Fails`/`Undefined` fixed points of `Proofs/NativeLoop.lean` under
  their invariant; storage and references use the typed family store and
  prophecy encoding of
  [`prophetic-references.md`](prophetic-references.md). `denote` is a
  relation, not an executable; it need not choose results.
- `denote_agrees` is proved **once**, by induction on the fuel or the tree
  against `BigStep.EvalFunction`, reusing the per-construct agreement
  lemmas that already exist in `Proofs/Denotation.lean` as the inductive
  cases. It is the LIR metatheory v2 wanted and the reason a bare
  `verify` success is meaningful: the vacuity hazard recorded in v2 cannot
  arise from the denotation, only from preparation admitting a node without
  executable semantics, which it must keep rejecting.
- Per target, `verify f` states the contract over `denote unit f`. The
  elaborator unfolds `denote` on the concrete unit to a closed shallow term
  `t` and certifies `denote unit f = t` **definitionally** (`rfl`, or
  `simp only` with `denote`'s equation lemmas), then reasons about `t`.
  Nothing is proved per target about agreement.
- **`denote` is typed and frame-free.** It is indexed by the validated
  type, `Spec RuntimeState Failure ⟦τ⟧` over native carriers, with locals
  bound by continuations and references as prophecy values; no
  `RuntimeFrame`, row, or loan registry occurs in the unfolded term. This
  is a requirement, not an option: v0 was sub-second because its goals had
  this shape, and the row route was seconds per function because its goals
  carried frames and call-boundary write-backs. The codecs of
  `Proofs/Representation.lean` appear once, in the agreement induction,
  never in a goal.
- The closer becomes general. Goals over `t` have v0's shape: a `Spec.wp`
  rule per combinator in a simp inventory, then `omega`/`decide`/`grind`
  on leaves. `Certify.lean` keeps its range, vector, and bound lemmas as
  leaf support; its shape routing goes.

Adding a construct then costs one `denote` case, one case of the
induction, and at most one wp lemma.

The deep-embedding tax v2 measured does not return. That tax was symbolic
execution of the interpreter inside every obligation. Unfolding `denote`
is a translation over concrete data, done once per target before any
obligation exists; the gate below measures it.

## Milestones

| | Status | Milestone | Gate |
|---|---|---|---|
| D0 | **DONE 2026-09-08**, agreement assumed (named axiom, user decision) | `denote` and `denote_agrees` for the straight-line subset `Denotation.lean` already covers (values, locals, checked arithmetic, assignment, return, monomorphic calls). | Theorem closed without `sorry`; `Language/Arithmetic` and `Language/Integers` verify through definitional unfolding; per-target unfold and closing cost recorded in `Performance.exp` and compared against v0's per-function times, with the unfold reported separately. |
| D1 | **DONE 2026-09-08** except recursion (`drain`, `recursive_choose`) | Control: branches, enum matches with payloads, structured loops with invariants, recursion. | `Verification/Loops`, `LoopInvariants`, `Calls`, `Callees`, the recursive `Corpus` targets, `Language/Loops`, `Enums`, `EnumPatterns`, `ControlForms` pass. |
| D2 | **IN PROGRESS** (checkpoint 2026-09-08): references and storage carried (`Account`, `Storage`, `Read`, `Prophecies`, `Corpus`, `Normalized` pass); open: resource invariants, loops over a live global borrow, returned references, vector loans | Storage and references: typed global family, scoped borrows, prophecies, returned references. | `Account`, `GlobalBorrows`, `GlobalInv`, `References`, `Loans`, `Prophecies`, `Storage` pass. |
| D3 | open | Generics (V4, carried) and the Rust profile denotations. | `Language/Generics`, `Verification/Generics`, `GenericScalarCalls`, Rust-profile fixtures pass. |
| D4 | open | Retirement: delete per-target `computationRepresents` generation, the `LeanerLang/Native*` generators, the `Proofs/*Agreement.lean` modules not consumed by `denote_agrees`, and `Certify.lean`'s shape routing. | Full Check audit at the unchanged caps; `Performance.exp` at or below the v0-parity targets; Move and Rust suites green. |

D0 is a feasibility gate as much as a milestone: if the induction for the
straight-line subset cannot be closed, or definitional unfolding costs
more than the agreement proofs it replaces, stop and report before D1.

## Working discipline

- No new `Proofs/*Agreement.lean` module, no new `LeanerLang/Native*`
  generator, and no new `Certify.lean` shape case. A construct `denote`
  does not carry is a negative check naming it, as today.
- No further ports onto the retired routes; a fixture is promoted only by
  the check ledger at the unchanged caps.
- No `sorry` in `denote_agrees`; the one named axiom stating it is the
  user-decided interim of 2026-09-08 and is removed by proving it. A case
  that cannot be closed is a construct `denote` does not carry.
- Cost is gated per target by `DenotePerformance.exp`; do not raise the caps.

## Open questions

1. Whether kernel `rfl` or `simp only [denote]` is the cheaper certificate
   for the per-target unfolding, and whether fuel over the arena or a
   reified tree unfolds cheaper; measure in D0.
2. How generic parameters are carried: a carrier family indexed by the
   type parameter with its codec, as the current generic route does, or
   monomorphized views (V4). D0 uses carrier families.
3. Which of the existing agreement lemmas survive as induction cases versus
   are re-proved directly; the induction decides, not the modules.

## Testing requirements

- Every `denote` case ships with its induction case; a construct whose
  case is open is not in `denote`.
- Positive checks are the existing Check fixtures named in the gates,
  unchanged in contract text. A fixture is promoted only when it passes
  at the unchanged caps; a passing pilot promotes nothing.
- Negative checks: for every construct `denote` does not carry, the
  expected diagnostic names it. A false `ensures` on every new construct
  must fail, as v2's vacuity finding requires.
- `Performance.exp` records per-target unfold and closing cost; a
  regeneration names only the targets the change was meant to move.
