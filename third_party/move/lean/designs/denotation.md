# One denotation, one agreement proof (V5)

Status: current verification design, decided 2026-09-08. It replaces
[`historical/certifying-execution.md`](historical/certifying-execution.md)
(the frame-free row route) and
[`historical/generic-route.md`](historical/generic-route.md) (normalization,
then the native route), and executes milestone V5 of
[`historical/verification-v2.md`](historical/verification-v2.md). Progress
is measured by the check ledger of
[`test-organization.md`](test-organization.md).

## Status (checkpoint 2026-09-23)

Implemented in `leaner-ir`: builds, the `DenotePerformance` gate, and the
check ledger of [`test-organization.md`](test-organization.md) (65 of 65
files exact on 2026-09-23) are as recorded there. The agreement theorem is **assumed**, by the user's
decision, until its induction is done.

### Modules

- `LeanerIR/Proofs/Denote/`: `Types` (native carriers, rows, codecs, the
  state operations and their wp rules), `Term` (typed terms and
  `Term.denote`), `Compile` (`compileFunction`, structural on fuel),
  `Agreement` (the named axioms `compileFunction_agrees`,
  `compileFunction_least_cycle`, and `compileFunction_least_generic`, and
  the transport
  to `SatisfiesFunction`), `Close` (the closer, the `lir_denote` and
  `lir_denote_norm` simp sets, the state-fact and leaf tactics).
- `LeanerLang/Verify.lean` owns `verify` (in a module and at top level),
  which audits each target's artifacts (the denotation route, approved
  axioms only). Per target it publishes the quoted term,
  the kernel `rfl` certificate of its compilation, `typedVerified` over the
  denotation, and `verified` over the big-step meaning; per struct twin it
  generates the two bridge theorems (`erase_eq_encode`, `decode?_fields`).
  `Contract.lean` keeps the clause translator and preparation.
- Gate: `LeanerLang/Tests/DenotePerformance.{lean,exp}` records heartbeats
  and proof objects per target (straight-line 1.7–6M, loops about 20M,
  the three-variant `match` with a matching contract 35M). The e2e check
  driver caps a target at 180k verification heartbeats.

### Carried

Vectors (2026-09-10): `NTy.vector`, whose carrier is the bounded
`SpecVector` (length below 2^64) that the spec twins already use, so a
vector's length is a certificate at a leaf as an integer's range is, and
`length` is exact; element places with literal, local, or from-end
positions in a path (`Proj.index`), vector literals, `index` with its
abort, the bounds check the lowering emits before an element place,
`push`, `insert`, `remove`; a construction that would exceed the bound is
undefined, as the runtime value it produces is not representable.
Loops carry a state invariant beside the local frame: the store and
registry when the body has no global effect, the pending set when it has
no call, and the loan discipline from the entry state, so a body that
mints loans still closes; a reference local the body only writes through
keeps its loan. In a loop invariant, `old(p)` reads the parameter at the
function's start: the proof records the arguments and state as a
`FunctionStart` hypothesis, which the closer passes to the invariant when
a clause reads `old`. A mutable loan the body declares dies at a
`continue` of its own loop, as at the end of an iteration. An invariant
reads structs and enums through their typed twins, and a variant test of
an encoded enum value normalizes to the variant it holds
(`testVariants_encode_enum`, `NTy.variantName_of_encode_enum`).

An unspecified callee (no `spec` block) is inlined: the caller's proof
reasons over the callee's compiled body through the agreement theorem
(`wp_calleeMeaning_of_compiled`), never over a summary; a callee with a
contract must be verified first. Structural equality of a vector against
a spec literal is carried: the codecs are tight (`NTy.encode_eq_iff`, by
induction over the type family), so an encoding equation is a decoding
equation in both directions, and `eqb` decides equality on reference-free
types (`NTy.eqb_iff`).

Scalars and checked arithmetic, comparisons, Boolean operations, unsigned
`&`, checked shifts and casts, constants, `if`, `let`, blocks, `abort`,
`assert`, early `return`, assignment, `break`/`continue` (labeled too),
loops with `invariant` clauses (an automatic frame ties every immutable
local available at the header to its entry value; the invariant asserts
every local it reads defined, so an iteration knows their values before
the body reads them, and a local a clause binds itself is not a slot; at
the iteration the whole context is normalized and its equations
substituted once, so that every leaf inherits normal hypotheses and the
cheap deciders close what they can without rewriting the context),
direct monomorphic calls through the callee's published theorem, the Rust profile's modular
`+`/`-`/`*` (`Term.modular`, wrapping as the runtime's `modularInteger`),
tuples, structs, enums with
variant tests and payload selection (a `match` arrives as those), shared
borrows of locals, mutable references (`NTy.ref`: a loan with the native
current value; parameters, in-place mutation, typed projection paths,
local lenders, reborrowed call arguments settled from the callee's
exports), and storage over the runtime's keyed global map (`globalRead`,
`globalContains`, `globalBorrow`, `globalPublish`, `globalTake`, with a
mutable global borrow leaving the runtime's hole and its death marker
writing back by key), and the vector primitives `push`, `insert`, `remove`,
`swap`, `concat`, `slice`, `reverseSlice`, `contains`, `indexOf`, and
`destroyEmpty`. A rejection names the construct.

### Not carried

`|`, `^`, signed bitwise, `bytes` and fixed-length vectors, a cycle of
calls through a generic function other than one calling itself (D6), the
generic residuals listed under D3, unspecified pure
callees used as summaries (`plus_one`), references inside an aggregate,
and nested destructuring patterns.

### Rules the implementation settled

- A recursive `spec fun` names a `decreases` measure (a mathematical
  integer, stored as its contract's `decreases` condition) and is a Lean
  definition `f.spec` over its bundled arguments, by well-founded recursion
  on the measure: each recursive call's decrease is proved by `omega` from
  the conditions on its path (its conditionals are dependent), and
  `f.spec.unfold` unfolds it once. A recursive call is never expanded, so a
  proof about it is authored, with lemmas about `f.spec` that are items of
  the module (see module items below). A Boolean
  result is a proposition; a type-parameter argument or result is a runtime
  value.

- `compare` (`std::cmp::compare`) is the structural order of runtime values
  (`RuntimeValue.order`): primitives by their natural order, vectors,
  tuples, and fields lexicographically, enum values by their variant's
  declaration position (each validated namespace caches the unit's variant
  orders), kinds and spellings breaking the remaining ties. It is proved a
  total order for every assignment of variant positions
  (`LeanerIR/Proofs/Order.lean`), so a proof at a type parameter compares
  encodings and uses the laws. Operands are values or shared references;
  a shared reference is its value.
- A module elaborates in three passes, so nothing among its items needs to
  precede its use: its Leaner items are registered and its recursive
  specification functions defined; its `theorem` items (optionally under
  `open … in`) are elaborated in source order in the module's namespace
  (`«0x42».m`, where `f.spec` lives); its verification targets follow. A
  function with a body and a `spec` block is verified automatically unless
  its specification or its module sets `pragma verify = false`; a `verify
  f` item states a target explicitly (with `by`, an authored proof, which
  sees the module's theorems by their short names), and verifies `f` even
  under the pragma. Targets are verified callees first: a target's
  specified callees, found through the unspecified callees it inlines, are
  verified before it, whatever their source order. Theorems are the only
  Lean items: the module is parsed as one command before any item is
  elaborated, so a syntax extension (a tactic macro) declared among its
  items could not be used by them. A
  top-level `verify path` remains for a `#guard_msgs` rejection and for a
  module declared in another file; its bare name must be declared by
  exactly one registered module.
- `verify f by tactics` runs the closer in residual mode: the obligations
  it does not decide stay goals (`leaf_n`, the obligation wrapper removed)
  and the authored tactics prove them. In residual mode a leaf first meets
  the cheap deciders (those that never rewrite the context: omega and
  decision after a goal-only normal form, the goal rewritten by the
  hypotheses); only a leaf they leave is normalized once
  (`leaner_denote_prepare`: split, substituted, bounded, the goal and
  every hypothesis in normal form, a conditional whose condition omega
  decides taking its branch, an element lookup with a provable bound
  becoming the element) and handed over — the closer's multi-round
  pipeline never runs on a leaf the proof will prove. The loop's locals
  and the quantified variables keep their source names; hypotheses are
  anonymous and an authored proof selects one by its shape
  (`‹∀ i : Int, lo.val ≤ i → … › k h`), binding it with `have` so the
  shape is refined by the arguments rather than by the goal. A leaf's
  cost report (`leaner.denoteDebug`) shows what each stage spent.

- A function without a body runs as the runtime implements it: the big-step
  rule `EvalFunction.native` and the interpreter share one fixed but unknown
  function (`BigStep.nativeCall`), so determinism and interpreter
  completeness hold. A caller's theorems assume, as a hypothesis
  (`native_f`), that each native it reaches satisfies its contract — never
  an axiom — and a caller of a verified function assumes that function's
  natives too. A native returning `&mut` is carried when its contract
  states the returned reference's final value (`final(result)`); otherwise
  a caller would reason over it through a body it does not have, and the
  call is rejected.

- Freezing a mutable reference a local holds (explicitly, or by passing it
  where a shared reference is expected) denotes its current value; the loan
  stays live and is resolved where its holder dies. A loop frame releases the
  current value of a reference the body writes through, and keeps the
  global store unchanged unless the body reaches a global operation, a
  callee whose contract may modify storage, or an inlined callee whose body
  does. A specification local (a quantifier binder of a logical type) keeps
  an empty slot, and a quantifier over a bounded integer type ranges over
  that type's values.

- A specification function applied in a clause is expanded at its
  arguments' types (a generic one substitutes its type arguments into every
  type the translator consults). A derived reading rebinds an assigned
  local and drops statements without logical effect; an abort or an
  opaque specification function denotes a fixed unknown value
  (`Contract.opaqueSpec`, keyed by the qualified name and instantiation).
  A resource read through a specification function belongs to the
  contract's families as a direct read does.

- The normalization names only simp sets. Ground evaluation (literal
  arithmetic, comparisons, `Int`/`Nat` casts, string equality, decided
  `dite`) is the set `lir_denote_eval`, so the guards of an operation at
  literal arguments are decided before its result is built. Array literals
  are updated at literal positions by simprocs over `setIfInBounds`,
  `swapIfInBounds`, `extract`, and `reverseRange`; `contains` and
  `indexOf` unfold `findIndex?` over the literal's size.

- A field selection whose operand is a mutable reference has no executable
  meaning (validation rejects it for execution), so the LeanerLang lowering
  never emits one: a field read through `&mut` selects from the
  dereferenced referent, a field borrow or assignment through `&mut` is a
  place rooted at the dereferenced local, and a write through a computed
  reference (a call result, a storage borrow) rests the reference in a
  hidden `$t` holder and writes the place through it. The printer folds the
  holder back into `e.f := v`. Variant payload fields are projection steps
  (`Proj.variant`, a `Choices` selection with `Choices.update?` on write),
  so `self.value := v` on an enum verifies like a struct field.
- A twin meets the native world through two bridges per declaration. An
  enum twin gets a native view (`Twin.native`, its variants injected into
  the carrier) and `erase_eq_encode` over it; a struct twin's
  `erase_eq_encode` folds enum-typed fields through that view, and its
  `decode?_fields` literal bridge has one lemma per combination of the
  variants its enum-typed fields can hold. The closer splits twin-typed
  locals (a struct into its fields, an enum into its variants, licensed by
  the `leaner_twin` tag) once nothing else destructures, so native views
  and erasures reach constructor forms; native decoding of enum literals
  proceeds variant by variant (`NTy.decode?_enum_cons_nominal`) through
  injections declared at the folded carrier type.

- Everything `compileFunction` computes is evaluated by the kernel when a
  target's `compiled_eq` certificate is checked, so it may read only what
  the target's own body reaches. A namespace-wide fold (`siteRoles` over
  the whole expression arena, 2026-09-10) made every `&mut` target's
  certificate cost superlinear in the module: twenty minutes and 22 GB for
  `Vectors/VectorOperations`, invisible to heartbeats and surfacing as a
  stall inside the first `simp` that had to wait for the kernel. Roles are
  now collected by a traversal from the function root.
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
- The equation lemmas of `Term.denote` (some seventy cases in a mutual
  structural recursion) are realized under the package's default heartbeat
  budget, which no `set_option` in the file reaches; the `LeanerIR`
  library raises that default in `lakefile.toml`. Verification caps are
  unaffected: the check driver and the cost gate set theirs explicitly.
- Inventories are simp sets, never explicit lists (0.8M heartbeats per
  call otherwise). `HList` and `variantCarrier` are not reducible, so
  every lemma whose right-hand side builds a row value is stated at the
  row type, never at the product it unfolds to. One spelling per
  encoding: a lemma's right-hand side encodes through `τ.codec.encode`,
  never through `NTy.encode τ`, so element-wise encodings of a vector
  meet the same normal form from the callee's post and from a settle.
- Lean 4.32's `simp_all` drops a hypothesis it modified in two rounds
  (its earlier form stays in the rule set and rewrites the hypothesis's
  own conjuncts to `True`); the closer uses a vendored copy with the
  bookkeeping corrected (`leaner_simp_all`,
  [`Denote/SimpAll.lean`](../leaner-ir/LeanerIR/Proofs/Denote/SimpAll.lean)).
- A callee's post `encode x = literal` is read as `decode? literal = some
  x` (a fact the leaf adds), so the literal names the value; the leaf
  runs without error recovery, since `all_goals` under recovery logs a
  failing alternative and aborts instead of letting `first` move on.
- The general leaf is one pipeline (`leaner_denote_pipeline`), each
  stage over the previous stage's goals; a round collects facts, splits
  and substitutes, then rewrites, so what a substitution exposes is
  rewritten before the next decision; state facts are retried after every
  round. A literal integer decoding is decided outright by a pre-simproc
  (`decodeIntegerLiteral`), so no conditional is left for a split.
- Rows are a mutual family `NTy`/`NRow`/`NRows` (a nested inductive cannot
  derive decidable equality); an enum carries the distinctness of its
  variant names. Aggregate arguments are introduced destructured, one goal
  per variant. `do`-notation never appears in a denotation definition.

Assumption ledger: `LeanerIR.Proofs.Denote.compileFunction_agrees` is an
axiom; every `verified` theorem names it under `#print axioms`. A member
of a cycle of calls also names `compileFunction_least_cycle`, a generic
function calling itself `compileFunction_least_generic` (D6). Nothing else
is admitted.

### Next

In order: the missing v0 fixtures (see the ledger in
[`test-organization.md`](test-organization.md)), then the retirement of the
previous routes' generators, tests, and `Performance.exp` (D4). Cost: a storage target with
two global borrows and invariants costs 100M heartbeats, three quarters of
it one `simp_all` pass per leaf over the shared context; sharing that
pass across the leaves of one target is the next cost item. Discharging
the axiom blocks nothing else and is scheduled with D4.

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
analysis ([`v0/move/Move/performance-analysis.md`](../v0/move/Move/performance-analysis.md))
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

These modules were removed with the routes that used them.

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
  concrete data. Every LIR construct is one case. `Proofs/Tree.lean`
  (since removed) was not this: it is the literal tree the current generator emits *per
  target* and its once-proved induction only connects that tree's two
  readings; the tree's relation to the LIR body is still proved per target
  from the combinator lemmas. V5 moves that per-target step into the
  definition of `denote`. Calls consume the callee's meaning; recursive
  cycles are least fixed points (D6); loops are fixed points under their
  invariant; storage and references use the typed family store and
  prophecy encoding of
  [`prophetic-references.md`](prophetic-references.md). `denote` is a
  relation, not an executable; it need not choose results.
- `denote_agrees` is proved **once**, by induction on the fuel or the tree
  against `BigStep.EvalFunction`, one case per construct. It is the LIR metatheory v2 wanted and the reason a bare
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
  on leaves. `Certify.lean` is gone; the leaf support the closer needs
  (`reportObligation`, the obligation ranges) lives in `Denote/Close.lean`.

Adding a construct then costs one `denote` case, one case of the
induction, and at most one wp lemma.

The deep-embedding tax v2 measured does not return. That tax was symbolic
execution of the interpreter inside every obligation. Unfolding `denote`
is a translation over concrete data, done once per target before any
obligation exists; the gate below measures it.

## References as prophecies (D5, decided 2026-09-23)

Decision of the user, 2026-09-23: a `&mut` in the denotation is v0's
prophecy pair, and the agreement is restated up to prophecy resolution.
This supersedes the D2 implementation, which represented a reference by a
loan id and its current value, let the lender keep its value until a
write-back at the loan's death, and passed a mutable parameter's final value
to the caller as a raw export in `RuntimeState.pending` that the caller
decoded. That departed from rule 1 of this design (codecs and loan
bookkeeping in goals) and cannot carry a returned reference: at the callee's
exit the lent parameter's value is not final, and the runtime marks it with
a hole naming the returned loan, which the caller could only resolve with a
loan registry or with hole paths in its goals.

### Model

- The carrier of `NTy.ref τ` is `τ.carrier × τ.carrier`: the current value
  and the prophecy, the value the reference will hold when it dies. There
  is no loan id.
- A borrow of a place `x.p` chooses a prophecy `π`; the reference is
  `(x.p, π)` and the lender holds `π` at `p` from then on. A borrow through
  a reference is the same step on that reference's current value.
- A mutable global borrow `&mut R[k]` is `(R[k], π)`, and the store holds
  `π` at `k`.
- Reads and writes through a reference act on its current value.
- A loan's death (its `endLoan` marker) resolves the holder: execution
  continues only where `current = prophecy`. A loan consumed by a call
  argument resolves inside the callee.
- A `&mut` parameter arrives as `(c, φ)`, the caller's current value and
  prophecy. Every exit resolves each `&mut` parameter the function still
  holds. Moving a reference-typed local clears its slot, so a parameter
  moved into the result is resolved by the caller, not at the exit.
- A call passes values. A reborrowed argument `&mut p` is a borrow as
  above, so after the call the lender holds the argument's prophecy, which
  the callee has resolved. There is no write-back and no export step.
- A returned reference is an ordinary value. `reborrow(slot)` returns
  `(c, π)` with `slot = (π, φ)`; the exit resolves `π = φ`, so the caller
  receives `(c, φ)` while its lender already holds `φ`, and the result's
  death in the caller resolves `φ`. A dynamic choice (`choose_reborrow`),
  a pair, and projected or global lenders need no rule of their own. This
  is v0's transfer: the returned mutation carries the lender's future.

Choice and resolution are `Spec.choose` (any value, state unchanged) and
`Spec.assume` (continue only where a proposition holds); `wp` reads them as
a universally quantified value and a hypothesis. The unfolded term keeps
rule 1: no loan id, frame, registry, `pending`, or codec at a reference.

### Agreement, restated

`propheticMeaning unit handle f : HList f.params → Comp f.result.carrier`
is defined from `functionSpec`, the big-step meaning, by a loan assignment
and prophecy resolution:

- `ok initial r final` holds when there are distinct loan ids for the
  argument references and a start state `s₀` that agrees with `initial` on
  `globals`, keeps those loans below `nextLoan`, has an empty `pending` and
  fresh global loan ids, such that BigStep runs the arguments' current
  values with those loans from `s₀` to results `rs` and state `s₁`; `r`
  carries `rs` with each returned reference's prophecy; the export of every
  argument loan in `s₁.pending`, with each returned loan's hole replaced by
  that reference's prophecy, encodes the argument's prophecy; and `final`
  is `s₁` with `globalLoans`, `nextLoan`, and `pending` of `initial`.
- `aborts initial e` holds when some such start state and loans abort
  with `e`.
- `undefined initial` holds when some such run returns a value the codecs
  do not decode.

`compileFunction_agrees` becomes
`Spec.Equiv (f.denote unit args) (propheticMeaning unit handle f args)`
and stays the only admission.

The public theorem keeps its meaning. `satisfies_prophetic`
transports a contract over the denotation to `SatisfiesFunction` of its
runtime form, which reads a mutable parameter's exit as its export with
every returned loan's hole replaced by that reference's current value, the
value view below. The transport is proved, not assumed.

### Contracts

- The contract of `verify f` is stated over `HList f.params` and
  `f.result.carrier` directly, with no row equation and no codec pullback.
  In `ensures`, `old(p)` of a `&mut` parameter is its current value and `p`
  its prophecy.
- A returned reference is read in the value view (v0's `ResolveReturn`):
  `ensures` holds under the premise that every returned reference's
  prophecy equals its current value, so clauses mean what they meant. The
  generated transfer relations (hole equations, lender foci, loan
  freshness) are deleted.
- A contract may read a returned reference's final value, `final(result)`
  (its prophecy): `ensures result == old(cell).value && cell.value ==
  final(result)` relates later writes through the reference to its lender.
  Such a contract drops the value-view premise, and callers use it through
  the callee's theorem like any other contract. A callee returning `&mut`
  whose contract does not read `final` keeps the value view, which cannot
  relate a later write to the lender, so a caller reasons over its
  denotation through the agreement instead.
- Loan facts (`FreshGlobalLoanIds`, `LoanDiscipline`, `nextLoan`) leave
  the contracts and the closer.

### Removed

`mintLoan`, `Exports`, `paramExports`, `exportThen`, `pushExports`,
`Args.settle`, `writeBack`, `publishBack`, the lenders of loan roles,
`pendingEquation?` and the lender analyses of `Contract.lean`, and the loan
tactics of `Close.lean`.

### Scope

References at the top level of a parameter or result, and components of a
result tuple. A reference inside an aggregate stays not carried and is
rejected by name.

### Milestones

| | Milestone | Gate |
|---|---|---|
| D5a | **DONE 2026-09-23.** Carriers, choice and resolution, borrows, deaths, parameters and exit resolution, moves of references, calls; agreement restated and the transport proved; contracts over native arguments. | Every check that passes today passes; `DenotePerformance` re-baselined, the diff reviewed. |
| D5b | **DONE 2026-09-23.** Returned references, the value view, callers through the callee's denotation. | `References/References` and `References/ReturnedMutRefErrors` pass. |
| D5c | **DONE 2026-09-23.** Removal of the loan machinery (the state-fact tactic, the loan lemmas, `mintLoan`, the contract generator's lender and pending plumbing); the rule-1 audit rejects the loan predicates in verified artifacts. | The audit holds for every check. |

What the implementation settled:

- A loan's death resolves each of its holders (from the borrow
  certificate) that still holds a reference. Using a reference-typed local
  as a value moves it out of its slot, so a holder it moved from resolves
  nothing, and a holder without references (an erased shared reborrow)
  observes the loan and resolves nothing.
- A loop that only writes through a reference local keeps its prophecy;
  the loop frame states that instead of a loan.
- The closer detaches a proof argument that mentions a local only through
  an unreduced projection before substituting it, and injects any equation
  between two constructor applications (a resolved enum reference is an
  equation between variant injections).
- Callees are collected through every inlined body, since a callee
  returning a reference is inlined and brings its own callees.
- The rule-1 audit forbids the loan predicates (`FreshGlobalLoanIds`,
  `LoanDiscipline`, `globalLoanKeyIn?`, `removeGlobalLoan`), not the loan
  fields of `RuntimeState` or the `borrow`/`loanHole` constructors: a store
  update is a record update that names every field, and a case split on a
  runtime value names every constructor.

## Recursion (D6, decided 2026-09-23)

**Status: DONE 2026-09-23; cycles of calls 2026-09-24.** `recursive_choose`,
`drain`, the caller `call_drain`, and the mutually recursive
`mutual_return_left` and `mutual_return_right` verify; a cycle through an
unspecified function is rejected with a diagnostic.

Decision of the user, 2026-09-23: recursion is carried by restating the
assumed agreement for an open body. A self-call denotes the callee's closed
prophetic meaning, and a contract proved by assuming it at the self-calls
is sound only because that meaning is the least fixed point of the body:
big-step derivations are finite.

- The denotation takes the meaning of calls as a parameter
  (`CalleeMeaning`). The closed denotation passes `closedMeaning unit`, the
  prophetic meaning of each callee; the body of a member of a cycle of
  calls passes `cycleMeaning unit members self`, which answers each
  member's handle with its `self` (`routeMeaning`) and every other handle
  closed. A function calling itself is a cycle of one. The routing casts
  the arguments along an equality of signatures that reduces on concrete
  ones.
- Assumed, beside the closed agreement: `compileFunction_least_cycle`, the
  closed prophetic meanings of a cycle's compiled members refine the least
  fixed point (`Spec.fixFamily`, indexed by `CycleIndex members`) of their
  denotations with the calls to members routed to the argument.
  Refinement, not equivalence: partial correctness needs only that every
  outcome of the closed meaning is a finite unfolding. The statement stays
  at the prophetic level; a statement over arbitrary runtime oracles could
  be false for oracles that read loan bookkeeping.
- Proved: `satisfies_cycle`, contracts that hold of the members' bodies
  under the hypothesis that they hold of the calls to members hold of the
  closed meanings, by induction on the unfolding depth and the refinement.
- The cycle of a function is every function it reaches by calls that
  reaches it again. `verify` of any member verifies all of them together:
  each member's typed theorem is stated over an arbitrary meaning per
  member satisfying that member's contract, and the closer consumes a call
  to a member through that hypothesis as it consumes a verified callee.
  The members' theorems then follow from `satisfies_cycle`, and they
  succeed or fail together. Every member is used through its contract, so
  each must be specified and not set `pragma verify = false`, and all must
  be in one module; a member returning `&mut` without `final` is used
  through its value view, which suffices when its callers return the
  reference.
- A generic function calling itself (2026-09-24): its calls to itself carry
  type arguments, so `self` is a family — its meaning at every skolem
  family and instantiation (`SelfFamily`) — and `recursiveGeneric` answers
  the own handle at the family and frame instantiation the call induces
  with `self` there, every other generic call closed. Assumed beside the
  monomorphic statement: `compileFunction_least_generic`, the prophetic
  meaning at every family and instantiation refines `Spec.fixFamily` of
  the body over all of them. `satisfies_recursive_generic` proves a
  contract from the body at every family under the hypothesis at every
  family; the typed theorem takes `selfVerified` quantified over families,
  which the closer instantiates as it does a generic callee's theorem.
  `Examples/Quicksort`'s `quick_sort_range` is the first such target.
- The semantics and public theorems are elaborated synchronously, as the
  typed one is: an error in an asynchronously elaborated proof is logged
  after the error count is taken and leaves a `sorry` that only the
  artifact audit then reports.
- A cycle of calls through a generic function is carried only when the
  function calls itself alone.
- Open: deriving the closed agreement from the fixed point needs a
  continuity proof over the denotation (a meaning's outcomes use finitely
  many calls); until then both statements are assumed.

## Generics (D3, decided 2026-09-23)

**Status: D3a DONE 2026-09-23.** `Generics/Generics`,
`Generics/GenericFunctions`, and `GenericScalarCalls` pass. How it landed:

- `Skolems` also requires the codec to be tight. The ground family
  (`Skolems.ground`, no values) types literals; the runtime family
  (`Skolems.runtime`, loan-free runtime values) is the public theorem's.
  A function without type parameters is proved at the runtime family: a
  term with a free family is never kept by the normalizer's whnf cache,
  which cost 12–84% per target when every proof ran over a free family.
- Terms carry calls with type arguments as `Term.callGeneric` (the
  callee's own signature, the arguments `θ` in the caller's types, the LIR
  type uses). `Term.denote` takes `Meanings`: the closed meaning of a call,
  and of a generic call at `Skolems.instantiate θ`, whose frame
  instantiation is `callTypeInstantiation` of the caller's.
- The closer instantiates a callee theorem's family and instantiation by
  unification, evaluates a substitution on a literal row, and states an
  equation between values of an instantiated parameter as one of their
  encodings at the argument's type, so a caller meets the runtime shape
  its own clauses use.
- A generic function calling itself at its own type parameters is carried
  by D6 (`Examples/Quicksort`'s `quick_sort_range`).

**Status: D3b DONE 2026-09-23.** `GenericStorage` passes. How it landed:

- A frame's generic families key through its instantiation
  (`Meanings.typeInstantiation`, `Family.instantiate`), which the empty
  instantiation reduces away. A generic contract takes the instantiation;
  the public one is at `#[]`.
- A skolem carrier is inhabited: a clause reading an absent resource reads
  a default. A call's type arguments are `TypeArgs`, a row certified
  inhabited by the compiler, and an induced family defaults to its
  argument's inhabitant, which is a twin's default for a scalar.
- A generic struct twin bridges to its native encoding at the skolem
  family and at every concrete spelling a family uses; its decoder unfolds.
- A generic call at the empty instantiation instantiates its callee's frame
  by a kernel-checked literal (`frameInstantiation_k`), which the closer
  rewrites wherever a body brings the call into a goal.
- The skolem theorem is proved for every instantiation, without the
  paper's inequality premises: where a contract distinguishes the
  instantiations of one resource, the proof fails rather than assuming.
  The aliasing instances are the completeness extension for that case and
  are not yet carried.
- Not yet carried: a generic caller of a callee whose storage depends on a
  type parameter (the callee's frame instantiation stays symbolic), and
  generic storage instantiated at a struct (the callee's contract carries
  the native view, the caller's the twin).

Decision of the user, 2026-09-23: generics follow the Move Prover's
monomorphization (TACAS'22, `move-prover/doc/paper21/design.tex`,
§Monomorphization), with the fresh type carried as a universally
quantified carrier. A generic function is proved once at a skolem instance
plus the finitely many storage-aliasing instances; callers use its
contract and never prove an instance of their own. This answers open
question 2.

- **Skolem family.** `class Skolems` gives, per type parameter index, a
  carrier, a codec to `RuntimeValue` whose encodings are `Plain`, and
  decidable equality. `NTy.param i` denotes `Skolems.carrier i`. Carriers,
  codecs, lending, the meanings, `Term.denote`, and contracts are stated
  under `[Skolems]`. `Term`, `compileFunction`, and the published
  artifacts do not mention it: a literal holds a value of the ground
  carrier, where a skolem has no values, and the denotation embeds it.
- **Type arguments.** The compiler takes a substitution for the function's
  type parameters: the skolem instance maps parameter `i` to `.param i`, an
  aliasing instance maps it to a concrete type (Rust: a trait-bounded
  parameter's impl). Every type of the body is translated under it.
- **Runtime instantiation.** `functionSpec` and `propheticMeaning` take the
  frame's type instantiation instead of `#[]`; `Term.denote` takes the
  caller's and a call passes `callTypeInstantiation`. Typed theorems
  quantify over it; `SatisfiesFunction` stays at `#[]`. Storage families
  key through `instantiatedTypeId`.
- **Calls.** A generic call denotes the callee's meaning at the callee's
  own signature under the family its type arguments induce (`θ.skolems Θ`:
  carrier `i` is `(θ i).carrier Θ` with its codec), with arguments and
  result transported between `(σs.subst θ).carrier Θ` and
  `σs.carrier (θ.skolems Θ)`; on concrete rows the transports are
  identities the normalizer removes. The callee's theorem, over every
  family, applies by unification. A non-generic call is unchanged.
- **Verify.** `verify f` of a generic `f` proves the skolem instance over
  every family and instantiation whose skolem type ids avoid the aliasing
  types, and each aliasing instance: a modified memory unifying with a
  modified or accessed one (code, specifications, callee contracts,
  invariants), closed under combining unifiers as in the paper. The
  theorem for every instantiation is a proved case split over them, not
  the paper's meta-argument. The public theorem instantiates the family at
  `RuntimeValue` with the identity codec.
- **Agreement.** `compileFunction_agrees` and `compileFunction_least_cycle` are
  restated over every family and instantiation: a skolem value is only
  moved, compared by its encoding (injective by the codec's left inverse),
  stored, and lent as a `Plain` encoding.
- **Not carried.** A type argument that is a reference (Rust) is not
  `Plain`: such an instance is specialized. Type-dependent natives
  (`type_info`) are not carried.

Milestones: **D3a** skolem instances whose storage does not depend on a
type parameter (`Generics/Generics`, `Generics/GenericFunctions`,
`GenericScalarCalls`); **D3b** generic storage with aliasing instances and
the injectivity of instantiated type ids (`GenericStorage`). The fixtures'
assertions on retired artifacts (`rawContract`, `pureVerified`) are
restated over the current ones.

## Milestones

| | Status | Milestone | Gate |
|---|---|---|---|
| D0 | **DONE 2026-09-08**, agreement assumed (named axiom, user decision) | `denote` and `denote_agrees` for the straight-line subset `Denotation.lean` already covers (values, locals, checked arithmetic, assignment, return, monomorphic calls). | Theorem closed without `sorry`; `Scalars/Arithmetic` and `Scalars/Integers` verify through definitional unfolding; per-target unfold and closing cost recorded in `Performance.exp` and compared against v0's per-function times, with the unfold reported separately. |
| D1 | **DONE 2026-09-08**; recursion 2026-09-23 (D6) | Control: branches, enum matches with payloads, structured loops with invariants, recursion. | `Control/LoopVerification`, `LoopInvariants`, `Calls`, `Callees`, the recursive `Corpus` targets, `Control/Loops`, `Enums`, `EnumPatterns`, `ControlForms` pass. |
| D2 | **DONE 2026-09-23**: references, storage, resource invariants, loops over live borrows, and vectors carried (`Account`, `Storage`, `GlobalInv` at the driver cap, `GlobalBorrows`, `Loans`, `LoopInvariants` pass); returned references by D5, recursion by D6 | Storage and references: typed global family, scoped borrows, prophecies, returned references. | `Account`, `GlobalBorrows`, `GlobalInv`, `References`, `Loans`, `Prophecies`, `Storage` pass. |
| D3 | **DONE 2026-09-23** for Move generics (D3a, D3b); aliasing instances and the Rust profile open | Generics (V4, carried) and the Rust profile denotations. | `Generics/Generics`, `Generics/GenericFunctions`, `GenericScalarCalls`, Rust-profile fixtures pass. |
| D4 | **DONE 2026-09-21** except the axiom | Retirement: the per-target `computationRepresents` generation, the `LeanerLang/Native*` generators, the `Proofs/*Agreement.lean` modules, and `Certify.lean` are deleted (78 modules, 45 test roots, 43k lines); `Contract.lean` keeps only the clause translator, preparation, and the cost commands. | Full Check audit at the unchanged caps; `DenotePerformance.exp` unchanged; the `leaner-ir` suite is red only on `Frontend` (`replace`, a mutable enum payload place) and `CompositionPerformance` (its own 50k cap). |

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
2. How generic parameters are carried: decided 2026-09-23, see Generics
   (D3).
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
