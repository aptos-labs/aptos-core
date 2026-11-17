# Plain values: withdrawing the certified carriers

The plan of the work that follows the certified carriers and the nominal
invariant sorts. It replaces [`one-spelling.md`](one-spelling.md) and
[`nominal-invariant-sorts.md`](nominal-invariant-sorts.md) as the order of
work, and it changes what [`native-nominal-types.md`](native-nominal-types.md)
builds on. The measurements behind the decision are in
[`transitions.md`](transitions.md), "What it cost, and the decision to
withdraw it".

> **Approved; phase 0 complete.** Three independent reviews and a Phase 0/1
> alignment. The first rejected how a condition was enforced and how the
> carrier was collapsed; the second rejected the first design note's rules for
> facts; the third accepted the second note's translation with corrections; the
> settlement/slice audit and user decisions (`U1`–`U7`) closed the remaining
> questions. Phases 1 and 2 below are that specification. The design is
> [`plain-values-semantics.md`](plain-values-semantics.md), which is the
> authority wherever an older passage of this plan differs from it.

In one paragraph: a value of a declared type goes back to being its plain
value, with no certificate in it. A data invariant is a condition, owed where
a value is made or a mutation ends and assumed where a value is received.
Every declared type becomes a nominal type, an atomic identity with a named
layout, early in the work and without an invariant in it. The code that
relates two layers of one value is deleted.

## Goal

The work is done when all four hold.

1. **Completeness as before the carriers.** Every target that verified at the
   reference verifies. The four Lean suites, the Move Prover's lean gate and
   prover-smoke pass. A recorded baseline differs from the reference's
   only where the expectation ledger names the reason.
2. **Cost as before the carriers, target by target.** No target of the
   reference costs more than it did, except where a later feature is named as
   the cause and has its own item.
3. **Less code.** The certified layer and everything that reconciles it with
   the plain layer are gone, and nominal types replace the structural layout
   instead of standing beside it.
4. **What the period found is kept.** The soundness fixes, the cost work that
   does not depend on the certified layer, the Rust work, the Given frame and
   the nominal type identity all carry over.

The reference is the benchmark of CI run 41770462997 on commit `73672a3e3c7`
(2026-11-03), and for test expectations the tree at `81ad86eb3ee`
(2026-11-05), the last one before the carriers. Its figures are under
"Gates".

## Decisions

These are settled. A change to one of them is taken to the user before any
code, written against the goal above.

**D1. Remove in place.** The work starts from the current stack, not from the
tree of 2026-11-05. The Rust work uses the certified layer in about 280 of its
25 500 added lines of `verex-ir`, each a case that keeps an existing
definition total, and `verex-rust` and `verex-move` do not use it at all.
Re-applying the Rust work to the old tree rejects 381 hunks in 28 files, loses
about 6 900 lines of solver work that came with the carriers, and forces a
rebase of the Rust-on-Move branch, which has 52 000 lines of Lean on top of
this stack.

**D2. Values are plain.** `NTy.carrier` of a struct, an enum or a nominal is
its plain type, and a mutable reference is a pair of plain values. There is
one family of carriers. `forget`, `ofDeepHolds`, `plainOf`, `certificateFree`
and the plain family have nothing left to relate and are deleted.

**D3. The protocol of the carriers stays; only the representation goes.** A
plain value with its validity is what a certified value was. Where the
carriers quantify over a certified value the plain design quantifies over a
plain value under `Holds`; where they construct one it checks. The steps that
owe an invariant, their order, and the choice of a prophecy among valid values
are kept, with one hole repaired. The older condition layer does not take
over: its markers are no step of the denotation. The design is
[`plain-values-semantics.md`](plain-values-semantics.md).

**D4. Storage keeps valid values.** One flat subtype at the storage interface
and nowhere inside a value; a read's fact is a theorem; the public theorems
assume nothing new (U1).

**D5. Every declared type is nominal, early.** Nominal type identity is built
on the plain values as soon as the carrier is plain, before the cleanup is
complete. It carries no invariant. Its library part is ported beside the
earlier phases, because it lives in new files.

**D6. The representation of invariants is not reopened before the baseline is
green.** A closed sort for invariant-bearing declarations may be tried later
as a contained experiment against a green tree at the reference cost. It is
not scheduled.

**D7. Nothing is built on a state that misses its gate.** Each phase ends at a
gate with figures against the reference. The working rules below say when to
stop.

**D8. One condition assumes the others, where a fact exists.** A verification
succeeds only if all its conditions do, so a condition is discharged by the
fact an earlier one established, in a well-founded order
([`roadmap.md`](roadmap.md), "Modular verification as a stated principle").
Every value the carriers typed as certified has such a fact, so boundaries,
joins and reads pass facts along and do not prove a declaration's clauses
again. Going further, to a mutation's end that owes only its path, waits until
after G2 (U3).

## Decisions taken on 2026-11-13

By the user, after the two reviews. They override the text below where they
differ.

**U1. Storage keeps valid values.** Option A of the design note: one flat
subtype at the storage interface, a global borrow's prophecy chosen among
valid values, no new assumption in the public theorems.

**U2. The Given route is parked until G2.** Its source stays and compiles
through the collapse, with its types adapted. A switch selects it
(`verex.givenRoute`). After G2 the route is adapted to the validity predicate
of a type parameter, switched on and completed as a step of its own with its
own gate.

Phase 0 measured both routes, and the parking cannot start yet: with the route
off, every generic function of `OrderedMap` fails on the concrete route (7 of
its 17 targets), where at the reference they verified concretely. At this tip
the Given route is what verifies generic members. So the option is **on** by
default through phase 1, and it is turned off at the switch of phase 2, where
the lent layer goes and the concrete route for generic members is restored;
`OrderedMap`'s authored proofs get their version for the concrete route then.
The tests that assert what the route does select it explicitly, so they keep
running whatever the default. Confirmed by the user.

**U3. The path-only obligation waits.** A mutation's end owing only its path,
and any analysis of which levels a death closes, are taken up after G2.

**U4. The target is the reference's cost for a stronger theorem.** At the
reference a data invariant was checked by the solver and was not part of the
proved statement. Here every check is part of it. A target that ends above its
reference figure for that reason is an exception the user signs off at G2,
with the measured cause.

**U5. Off-domain total reads in specifications are unspecified (`opaque`).**
Reading an absent resource (`nativeSlot`), a field of another variant
(`typedVariantField?`), a missing map key (`SpecMap.get`, `valueAt`, `keyAt`),
an out-of-bounds vector index (`indexVector`), or `result_of` without a
returning run yields an uninterpreted (`opaque`) plain fallback, backed by
`NTy.plainInhabitant` solely to satisfy `[Inhabited]` without axioms. It
exposes neither a `Holds` fact nor concrete structural-zero layout equations,
and `DataInvariantsSatisfiable` is removed.

**U6. All slice borrow, take, and settlement equations are checked in phase 1.**
Before phase 2, slice borrow/reborrow (`borrowArraySliceAt`,
`borrowVectorSliceAt`, `reborrowSliceAt`) checks an open dereferenced current
before equating it (`1.2`), `takeSliceAtValue` checks the computed cleared
owner (`1.2`), and slice settlement (`resolveSliceAt`, `Mutables.sliceAt`)
checks both the current elements and — via slice lender provenance — any
enclosing owner rebuilt with the settled slice (`1.2b`).

**U7. Unconditional argument and result validity live on `Contract`.**
`Contract` in `VerexIR/Proofs/Contract.lean` gains first-class `validArgs` and
`validResult` predicates defaulting to `fun _ => True`, enforced
unconditionally in `Satisfies`, `Contract.summary`, and `wp_summary` across
direct calls, generics, and closures.

## Corrections from review 1

Each is binding. The review is
`/home/wrw/shared/handoff/verex-plain-values-2026-11-13/CODEX-REVIEW-1.md`;
its source claims named here were checked again.

**C1. A producer's condition is part of the theorem.** The markers of the
condition layer (`constructedAt`, `mutationEndAt`) are `Spec.pure`, and their
rules take any predicate: they are sufficient rules for the solver, and a
proof of the function's theorem need not use them. The carriers' cut
(`certifyingAt`) is different: its rule is an equivalence, so the invariant is
owed by the denotation itself. So the condition layer does not take the
declarations back as it stands. Every producer is a step whose meaning owes
the invariant, as `certifyingAt` does today, with a plain result in place of a
certified one. What is withdrawn is the representation of the result, not the
obligation of the step. The declaration's predicate stays where the
denotation can read it.

**C2. Checks come before the equations of a resolution.** Today a loan's
effects are its resolution followed by its checks, so a prophecy equation can
remove a run before the check sees it. A death checks the current's validity,
then the owners this death closes, then states its equations. For joined
holders every candidate is checked before the shared equation.

**C3. Copy-out is a producer.** Copying or observing a value of a type with an
invariant out of an open current owes that value's invariant; the carriers do
this today (`certifyingAt` with `.copyOut`). The table below lacks the row.
Counterexample to passing a fact without it: a function that writes 101 into
`p.n`, copies `*p` into its result, and writes 0 back.

**C4. Facts have a lifetime.** A condition uses only a fact established
earlier on its path, an arrival's or a callee's discharged premise, or the
hypothesis of a stated induction. A write ends the fact of what it overwrites.
A value under a live mutable loan is open and has no whole-value fact; a
sibling under another live loan is open too. So "the end of a mutation owes
the path" is restricted: a death owes the levels it closes, uses the facts of
subvalues known closed, and assumes nothing of an open sibling. An owner's
clause that relates fields is owed when the owner closes. The cost is not
bounded by the depth of the path where a clause quantifies over a collection.

**C5. A generic call's result has no fact by substitution.** With a plain
carrier, a caller's fact about an argument does not give a fact about a
different result. The family keeps a validity predicate per type parameter:
a generic body assumes it of the parameter's values it receives and owes it of
those it returns, and an instance supplies the concrete predicate. The design
note says whether this is that preservation theorem, a contract verified at
the row, or both, and costs it.

**C6. Storage is not covered by the existing ledger entry.** `StorageEncodes`
relates a store to a typed memory; when the memory's values are plain, the
same statement no longer implies their invariants. The replacement states
valid initial storage and the validity of each permitted read, with the
conditions on live loans, prophecies and external writes, and stays an audited
assumption until preservation is proved. The design note also weighs the
alternative that needs no new assumption: memory keeps valid values as its
type, and the subtype exists only at the storage interface.

**C7. No published artifact with an undischarged premise.** One failure being
reported once is a diagnostic policy. Every required condition is still
generated and checked.

**C8. The collapse is one coordinated change.** `Carriers.lent` is a field any
family may set, and the Given frame sets it, so fixing `Carriers.instantiate`
does not make a parameter's plain type and carrier equal in general. The
change covers the family's interface, instantiation, the recursive carriers,
the reference representation and the consumers that depend on them, prepared
in a bounded scratch change. Collapsing references first is not a required
stage: it would repair the transports twice. No theorem that extracts validity
from an arbitrary plain value survives (`holds_forget` and its kind).

**C9. Matchers are classified by what they do.** Some metaprograms shaped by
the certificate carry behavior the plan keeps: the entry and finals reasoning
(`strengthenEntries`), closure contracts (`invokeSite`, `siteRequiresArm?`),
and the condition branch of `dataInvariantStep`. Each is replaced by direct
reasoning over the plain row in the same landing as the deletion of its
certified branch. A subtype of type arguments or of a numeric range is
unrelated and stays. The list of matches is a worklist, not the specification
of what to delete.

**C10. Nominal types.** The representation is chosen in N0 only after it has
shown one spelling under the solver's reductions and a recursive group under
the same interface; the table provider's carrier is a fold over earlier
entries and does not define a recursive group as it stands. The first emission
is a whole unit; mixed emission needs one representation per declaration at
every producer. Owners with an abstract layout are part of completion, and
their model is designed and estimated before N5 is called done. The snapshot's
library is a source to adapt declaration by declaration, not a set of files to
take.

**C11. Gates.** A gate compares the same target identities, source and
specification hashes and native phase definitions; the tolerance applies to
the cost of repeated runs only. The ledger also covers fixtures without a
baseline, added controls, target selection and the hypotheses of published
theorems. Phase 0 measures by route and by phase on the reference and on the
base, and names the revisions that can be built for attribution, so that a
miss at G2 does not start a week of archaeology. A G2 exception needs a
measured cause.

**C12. Process.** Work stops at once when a required condition has no
theorem-level justification, when a success depends on weaker assumptions or
skipped targets, or when a repair changes the representation the design note
froze. A lower count of red targets is not progress if assumptions, coverage
or proof paths changed. A commit that is known red is an experiment and is
labeled so; a checkpoint is green. The scope of the Given route is measured in
phase 0 and decided at the design freeze, because it shapes every repair of
generic code.

**C13. Estimates.** Phase 1 and phase 2 are estimated again after the design
note. The reviewer's provisional figures are 10 to 15 days and 8 to 12 days,
and three to five weeks for the nominal track without recursion.

## Corrections from review 2

The second review
(`/home/wrw/shared/handoff/verex-plain-values-2026-11-13/CODEX-REVIEW-2.md`)
is of the design note. Its corrections are in the note's second revision,
[`plain-values-semantics.md`](plain-values-semantics.md), which overrides this
plan where they differ. For this plan they mean:

**R1. The plain design is the carriers' design under one bijection.** Where
the carriers quantify over a certified value the plain design quantifies over
a plain value under `Holds`, and where they construct one it checks. The
protocol of checks, their order and the choice of prophecies among valid
values stays what it is, with two holes repaired: the death of a loan through
a reference, and the resolution of a slice reference. No new rule about when
a value is valid is introduced.

**R2. "Modular conditions" is reduced.** A boundary discharging a clause by a
fact stands. A mutation's end owing only its path, and the notion of a level
closed by a death, are withdrawn from the collapse: the borrow analysis does
not give exact closure. They are an optimization for after G2, with a review
of their own (the note's section 10).

**R3. Storage: option A**, recommended by the review and the note: memory
keeps valid values, one flat subtype at the storage interface, and a global
borrow's prophecy is chosen among valid values. The user decides. Routing the
denotation and the memory solvers through the plain read and write is
implementation work, not a deletion.

**R4. The switch and what precedes it.** The representation switches in one
coordinated change; the two repairs, the controls with captured loan facts,
the inventory of former certificate receipts, the folded boundary clauses and
the interfaces that can stand beside the old ones land before it, each on its
own (the note's section 8).

## Data invariants as conditions

The design is [`plain-values-semantics.md`](plain-values-semantics.md). In
brief:

- **Checked** where the carriers certify today: a pack; a loan's death, of the
  current and of the owner rebuilt with it (erased at direct-root `.deref`),
  before the settlement's equations; slice borrow, take and settlement
  equations, enclosing owners included; a `&mut` hand-over; a returned
  reference and a `&mut` parameter's exit; a copy out of an open current; a
  write of storage. Repairs in phase 1 close every settlement and slice
  equation that today assumes a certified side unchecked, in the resolver, the
  slice helpers, and the compiler's combined settlement.
- **A fact** `Holds` for every value the carriers typed as certified: from a
  check, from the argument domain of the theorem, from a callee's result,
  whose validity is unconditional, from a prophecy's choice among valid
  values, from a storage read, from the owner by opening, from an equation.
- **Open**, with no fact until checked: the current of a mutable reference.
- **A join or a loop head** takes the validity of the values it abstracts as a
  mandatory premise, proved by each arrival at its own arguments.
- **A type parameter** has one validity predicate in the family, so a generic
  body proves preservation with its contract.
- **Not the image of the translation**, and audited before the switch: a
  default that was certified, and the removal of the satisfiability
  hypothesis.

### Controls

Every negative below is rejected with a named obligation, by the automatic
route and against the authored proof that once exploited it, and each positive
beside it verifies. They run at every commit of phases 1 to 3, each with the
validated loan facts recorded beside its verdict. The design note's section 9
lists the controls to add.

- `VerexSL/Tests/SoundnessCounterexamples.lean`: the cases of the two
  soundness reviews.
- `Check/Storage/MutationEnds` and `SourceVerify/data_invariant_reads`,
  `data_invariant_global_mut`, `data_invariant_call_during_loan`.
- The loan-death set, ported from the stopped migration: a write of an invalid
  value through `&mut pair.left` of a `&mut Pair`, through a box, through a
  nested path, and one that escapes to a callee. On the current stack the
  first of these verifies with an authored proof; closing it is part of
  phase 1.
- The hand-over case the Move Prover accepts: a `&mut` opened by a reborrow
  that died, then passed to an opaque callee.

## Nominal types

### What they are for

A declared type at its arguments is one small term: a constant applied to the
arguments' types. Type terms stop carrying a declaration's fields at every
mention, an instantiation is computed once and named, and a recursive
declaration becomes expressible. On the framework modules the structural
layout's type terms are 52 to 68% of the reported unfoldings, a count and not
a time share. Nominal types do not change the cost of quotation, which comes
from primitives, or of local-variable environments, which is a separate
track.

### Requirements

- **N1. Identity.** `NTy.nominal source arguments productive` is the only
  spelling of a declared type. Substitution visits the arguments only.
- **N2. One spelling of a value's parts.** A field read, a construction and a
  case analysis are each one named operation with one equation, or the Lean
  projection, constructor and `match` of a generated type. The design note
  chooses one and every producer follows it.
- **N3. Setup proportional to use, and small.** A unit's setup generates names
  and no proofs per declaration. The laws of a declared type are library
  theorems, applied where a proof needs them and cached per declaration and
  argument row. The gate is the certification phase of the benchmark at the
  reference.
- **N4. No invariant.** The functor of a nominal type has no field that
  mentions an invariant. A declaration's invariant is a predicate of its plain
  value, stated beside the type.
- **N5. Ready for recursion.** The carrier and the operations are defined so
  that a recursive group needs no second representation.

### What exists

- On main: the `NTy.nominal` constructor, with a carrier that is a runtime
  value under a typing condition. Nothing emits it.
- On the current stack: the functor basis (`NominalFunctor`, held as one field
  of the family, `NominalCarriers`, the arms of the codec, equality and order).
  It is stated over the plain type of a nominal. Four of its fields and one of
  `NominalCarriers` state the invariant and go (`nominalHolds` with its three
  laws, `nominalOwn`). Nothing emits a nominal yet: stage 1 of six in
  [`native-nominal-types.md`](native-nominal-types.md).
- On the current stack: named descriptors per declaration for the structural
  layout, with their cache (`NominalQuote.lean`). Independent of the
  certified layer.
- In the snapshot of the stopped migration (`wrwg/verex-nominal-sorts-snapshot`):
  a template library for finite declarations (about 5 000 lines: layout,
  codec, equality, order, substitution, instantiation), the declaration plan,
  and the generator's skeleton. About 1 400 lines of it use no part of the
  invariant layer; the rest reaches it through one file, `NativeModels.lean`,
  which states the model of a declaration over the certified carrier.
- In the provider prototype of the stopped migration (`NativeTable.lean`,
  `NativeTableCoverage.lean`): a library construction that replaces the
  per-declaration generated provider by one fold over a named table. Measured
  on one unit: 16.5M heartbeats and 202 constants down to 1.0M and 20. A law
  proved per declaration by evaluation through the fold costs more than
  today's, so the laws have to be library theorems. The generator that writes
  a unit's table was not written.

### What is written new

`ntyOf` giving a nominal for a declaration; the per-unit instance without an
invariant; the table generator; the owner-indexed operations of `Term`, `Proj`
and `Pat` with `Compile` and quotation; the solver's observers and the proof
language's steps over a nominal value; the removal of `supports`, `covers` and
the structural layout.

## What carries over, and what goes

From the carriers' change, by a sampled inventory of its 21 500 added source
lines: about 10 400 are the certified layer and go; about 11 100 stay.

| Kept | Where it is |
|---|---|
| The obligation where a loan on global memory dies, and one obligation per dying loan | `Compile.lean` |
| Lender identity and `Lender.resolve`; liveness in the borrow analysis; the index-write-under-loan refusal | `Compile.lean`, `Term.lean`, `Validation/Borrowing.lean`, `Validation/Validated.lean` |
| The hand-over obligation and the entry assumption of a `&mut` parameter | `Term.handedOver`, `Term.assumeCurrent`; their statements are re-expressed without `certificateFree` |
| Mutation conditions keyed on the loan's holder; `declaresDataInvariants` | `VerexSL/Verify/Contracts.lean`, `VerexSL/Verify/Members.lean` |
| Site-tagged construction and field terms | `Term.lean`; a pack owes through `constructedAt` instead of `certifyingAt` |
| The cost work: Boolean decisions as propositions, determined variants in shared preparation, value bounds and pinned terms, leaf instances and congruence atoms, the spec-definition evaluator, the scalar normal forms, un-named simproc patterns | `Solve/Decide.lean`, `Solve/Base.lean`, `VerexSL/Verify/Steps.lean` |
| The denotation over an arbitrary state; the Given frame; relational transfer | `Term.lean`, `Interpretation.lean`, `VerexSL/Verify/Given*.lean`, `RelationalTransfer.lean` |
| The entry and finals form of a contract | `Behavior.lean`, restated over the one family |
| The benchmark tool's parallel runs and history, the hooks, the failure reports | `scripts/`, `VerexSL/Verify/Diagnostics.lean` |
| The soundness counterexamples and the mutation-end fixtures | tests |

From the stack above the Rust work: the row route of a generic callee, the
fold of an invariant into one named fact, the one element default, the
closure's behavior at its target, and the nominal basis without its invariant
fields.

From the stopped migration: the loan-death repair of `Lender.resolve` and its
controls, re-expressed as a condition; the nominal library; and these
findings, each to be applied where its producer lives: the subject-head test
before the context scan of `nativeModelSelection`; no required definitional
check in `ensureSpecialization`; frame certificates once per unit; prophecy
destructuring at the consuming binder.

| Goes | Where it is |
|---|---|
| The plain family and the second tower: `PlainFamily`, `plainOf`, `HListPlainOf`, `variantPlainOf`, the plain codecs, orders and equalities, `deepHolds`, `ofDeepHolds`, `forget`, `certificateFree` | `Types.lean` |
| The respellings and the heterogeneous transports | `FamilyTransport.lean` |
| `plainReframe` and its lemmas, `OwnAtArguments`, the gating on `CarriersNonempty` | `Resources.lean` |
| `CarriersNonempty`, `Inhabitants`, `DataInvariantsSatisfiable` | about 200 sites |
| The `forgotten*`, `certified*` and `plain*Skolem*` simprocs | `Solve/Normal.lean` |
| `assertCertifiedClauses`, `certifiedEntries?`, `certifiedCaptures?`, `PlainSpelling`, the certified twins of the case and range routines | `Solve/Base.lean`, `Solve/Decide.lean`, `Solve/Invoke.lean` |
| The `*Certified?` selectors and their lemmas; `certifyingAt` | `Term.lean` |
| `Carriers.own`, `dataOwn`, the carrier instance's generation | `VerexSL/Contract.lean`, `VerexSL/Verify/FunctionValues.lean`, `VerexSL/Verify/Members.lean` |
| The tests of the carrier machinery | about 1 400 lines |
| Closed and open sorts, readings, observers as a closed vocabulary, the closing rules | the snapshot only; never ported |

Three generator files mix the two and need work declaration by declaration:
`VerexSL/Verify/Members.lean`, `VerexSL/Verify/FunctionValues.lean` and
`VerexSL/Verify/Contracts.lean`. In `Term.lean` the retyping to the plain
layer and the move to an arbitrary state are in the same hunks.

## Order of work

Each phase starts with a short design note: the declarations it changes, the
statement forms, the order of edits. The note is reviewed by a second model
before implementation. Each phase ends with its gate's figures against the
reference. The estimates are working days and are estimates.

In sum, in working days of effort: phase 0, 2 to 3; phase 1, 15 to 21,
of which up to three items run at once; phase 2, 10 to 15. That is five and a
half to seven and a half weeks of effort to G2a, and four to six weeks by the
calendar with phase 1 run in parallel. The first cost signal is phase 0's, in
the first three days. The nominal track is another three to five weeks without
recursion, its library part beside the earlier phases. The order is

```
phase 0 -> phase 1 (1.1 .. 1.6, by file) -> G1 -> phase 2 -> G2a -> G2b
        -> 3a -> N2 N3 N4 -> 3b -> N5 N6
N0, N1 (new files only) beside phases 0 to 2
```

### Phase 0. Base, reference, controls (2 days)

No behavior changes.

1. **Base.** The current stack (`wrwg/verex-nominal`) restacked on main. The
   older partial migration in the worktree is not part of it: its source
   changes are reverted when the plan is approved, and the document changes of
   this plan are kept. A copy of the whole uncommitted state is in the stop
   directory of the stopped migration.
2. **Reference.** The reference benchmark's results are stored in the tree as
   data, with the per-target table under "Gates". Every later table has the
   reference column.
3. **Expectation ledger.** For every fixture that exists at `81ad86eb3ee`, its
   baseline there is the expectation. A script lists the differences of the
   current baselines, and [`../status/`](../status/) holds the list with a
   reason per entry: a soundness fix, a Rust-era change, an intended message
   change. An entry without a reason is a regression.
4. **Controls.** The soundness controls above run as one target. The
   loan-death set is ported from the extracted patch's tests and recorded as
   it stands.
5. **The base's own figures** on the benchmark tool, the cost tests and the
   four suites, through CI on a draft PR. By route and by phase: the generic
   targets on the Given route and on the concrete route, and setup,
   verification and certification apart, on the reference and on the base. A
   switch for the Given route is added for this and stays (U2). The revisions
   that can be built for attribution are named here.
6. **One experiment, one day: made, and dropped.** A temporary option under
   which `dataOwnTerms` carries no declaration is not a sound switch: on
   `OrderedMap` it gives type mismatches at the module's own command, between
   a parameter's inhabitant and a concrete carrier, and the scenario targets
   never elaborate. It was not repaired and is not in the tree. One signal
   came from it: with the Given route off and nothing carried, four generic
   functions verify at about half their present cost, near their reference
   figures.

Gate: the base builds, its state is recorded against the reference, the
controls run, the experiment's table exists.

### Phase 0b. The semantic design note (done)

[`plain-values-semantics.md`](plain-values-semantics.md), in its fourth
revision, incorporating three reviews, the settlement/slice audit, and `U1`–`U7`.
It is the design of phases 1 and 2.

### Phase 1. Preparations on the present representation (15 to 21 days of work)

Nothing here changes the representation. Each item is a change of its own,
checked on the tree as it is, and the items touch different files, so up to
three can proceed at once. The design note's section 8 lists them; the
specification is:

**1.1 The settlement repair** (2 to 3 days). Every settlement checks the plain
side of each certified equation before assuming it: in `Lender.resolve` at
`.place` (referent then rebuilt owner when `path.hasDeref`, with direct-root
`.deref rest` erased by `Lender.resolve_place_deref`) and `.candidate` (rebuilt
owner whenever present, erased at `.deref rest` by
`Lender.resolve_candidate_deref`), and in `holderResolves` in `Compile.lean`
(one unconditional `.referent` before all `.candidate`s and `.equation`). With
`certifyingAt`. Its controls land with it: the loan-death set, `.sliceIndex`
and stored-reference negatives, and combined settlements whose candidates all
dereference.

**1.2 The controls, slice-current, slice-borrow and slice-take checks** (2 to 3
days). The soundness counterexamples and the loan-death set as one target, each
run recording the validated loan facts beside its verdict. The explicit check
of a slice reference's current reading before its equality at `Mutables.resolve`
and `resolveSliceAtValue` (with `site` added to `Term.resolveSliceAt` and
`CarrierDeaths.lean`); the check of an extracted current under `path.hasDeref`
before the existential assumption in `borrowArraySliceAt`,
`borrowVectorSliceAt`, and `reborrowSliceAt`; and the concrete rebuild and
check of the cleared owner in `takeSliceAtValue` (`U6`), each with its control.

**1.2b Slice lender provenance** (2 to 3 days). Carry lender provenance for
slice loans to `Term.resolveSliceAt` and `Mutables.sliceAt` so that ending a
slice loan checks any enclosing local or candidate owner rebuilt with the
settled slice before assuming `reference.current = reference.final` (`U6`),
with controls for cross-element owner invariants (split across the
`VerexIRTests` modules `LoanDeathSupport`, `LoanDeathSoundness`,
`SliceLeafSupport`, `SliceProvenanceDecls`, `SliceProvenancePositives`,
`SliceProvenanceNegatives`, and `SliceSemanticControls`). Result at the
pre-switch representation: a mutable slice borrow of a struct's certified array
(and slice mutation through an open reference) is not verified automatically
yet, leaving two leaf shapes in `SliceProvenancePositives.lean` and
`SliceProvenanceNegatives.lean` (`SliceLeafSupport.lean`): (1) slice
birth/reborrow leaves carrying an unprojected `NTy.forget (.sliceRef _) v =
SpecSliceRef.ofArrays current final` equality in `LCtx` together with
`NTy.forget_array_values` lacking `no_index` on its type arguments (`↑v.extent
≤ 1 ⊢ False` and enclosing `Holder` relation obligations), and (2) open-reference
slice/array mutation leaves requiring `SpecArray.size_eq` and element membership
instantiation across `Array.setIfInBounds`. Under the `250,000` default
`verex.verifyHeartbeats` budget, six targets in `SliceProvenanceDecls.lean`
carry measured `pragma heartbeats = N` raises (`slice_holder_either_ok`:
measured `277,897` -> `350`; `slice_holder_diff_len_ok`: `318,187` -> `400`;
`slice_slot_reborrow_ok`: `548,353` -> `690`; `slice_holder_either_bad`:
`313,023` -> `390`; `slice_holder_diff_len_bad`: `362,057` -> `450`;
`slice_slot_reborrow_bad`: `559,597` -> `700`), with the three negatives
confirmed to fail at the semantic invariant check (`Percent` / `Holder`) rather
than the budget.

**1.3 The inventory, audit of defaults, and opaque spec-read fallbacks** (3 to
4 days). Every place a certificate is received, classified as extraction or
preservation, checked production, contractual receipt, or raw value. Every
formerly certified default audited; controls per row of the design note's table
of total specification reads; and the shared `opaque` plain fallback helper
prepared for `nativeSlot`, `typedVariantField?`, `SpecMap` total reads,
`indexVector`, and `ResultOf` (`U5`). **The audit gates phase 2**: an
observable default left unresolved stops the switch.

**1.4 Folded boundary clauses** (3 to 4 days). `slotDataInvariantTerms` and
the loop's clauses state one `Holds` per value, with the opening rule and the
source range of each declared clause kept for authored proofs and reports.
Checked against the authored proofs of the benchmark and the framework before
it lands.

**1.5 Storage through its interface** (4 to 6 days). The denotation's storage
steps, `Meanings.interpreterOps` and the solver's memory steps go through the
plain read and write (`Skolems.plainAt`, `Skolems.writePlain`) and a folded
rule for a successful write, while memory still holds certified values. After
this the switch changes only what is inside the two operations. The cost is
measured here, on `deposit` and the storage targets, and again after the
switch.

**1.6 Interfaces and library lemmas beside the old ones** (3 to 4 days).
First-class `Contract.validArgs` and `Contract.validResult` domain predicates
(defaulting to `fun _ => True`) in `VerexIR/Proofs/Contract.lean` with
`Satisfies`, `Contract.summary`, and `wp_summary` updated (`U7`); collection
`Holds`-preservation lemmas introducing `Holds` of vectors, arrays, slices, and
maps from their elements (`push`, `insert`, `remove`, `swap`, `concat`,
`slice`, `reverseSlice`, `arrayOf`, `repeatVector`, `SpecMap.ofEntries`); the
`Holds`-preservation laws of the frame's transports; and the mandatory premise
bundle in a shared continuation's node and its arrivals, discharged for now by
the certificates the values carry.

Gate G1:
- every soundness control passes, and every negative is rejected by the check
  it tests (not by a budget stop or an unrelated error), the settlement and
  slice negatives rejected at the repaired checks;
- the four suites and the ledger are no worse than at the base, or diagnosed
  and assigned to G2a (user decision, 2026-11-14: defects in code that survives
  the switch are repaired now; losses caused by the pre-switch bridge layer or
  postponed obligations rewritten by parts 2b/2c are diagnosed by cause,
  recorded in the baselines and ledger as postponed at G1, and due at G2a; no
  loss enters phase 2 undiagnosed);
- the audit of defaults and its total-read controls are complete;
- cost is recorded per item across all benchmark and `DenotePerformance`
  targets so G2b has the baseline; small cost increases at G1 do not hold this
  stage as correctness has priority (user decision, 2026-11-14).

Rows diagnosed at G1 and postponed to G2a:
- `SourceVerify/library_iter` and `SourceVerify/library_iter_errors` (`last_or`, `invariant_2_step`): dual carrier/plain spelling at the loop step (`2a` + `2d`);
- `SourceVerify/library_iter_adapters` (`find_entry`, `invariant_1_step`, `ensures_2`; paired with `SourceVerify/library_iter_adapters_errors` `find_entry` at `pragma heartbeats = 730`): `Var.selectCertified`/`NTy.forget` round-trip on `NTy.slice Entry` and `Option.map Entry.forget` element read (`2a` + `2d`), while `invariant_3_step` compounds that same `P` round-trip over an inherited `O` range-step split (`0 <= k < pos + 1`, already open on `u32` in `SourceVerify/library_iter_enumerate::position`; listed separately, rechecked at G2a, and neither promised to stay open nor exempted from G2a's reference counts);
- `functional/enum.move` (`test_enum_vector`, `assert_2`): element `NTy.Holds` recovery across vector indexing (`2a` + `2b` + `2d`);
- `functional/closures/stored_fun_values.move` (`config_aware_increment`, `aborts_if_1`): dual `nameSomeReads` witness at a global read (`2a` + `2c` + `2d`);
- `functional/closures/amm_example.move` (`constant_product_with_fee`, `ensures_1`, `ensures_2`): dual `nameSomeReads` witness at a global read (`2a` + `2c` + `2d`);
- `functional/closures/behavioral_congruence.move` (`field_equal`, `param_equal`, `aborts_if_1`): behavioral entry congruence over the plain row (`2a` + `2d`).

A simplification pass follows each item (working rule 9).

### Phase 2. The switch (10 to 15 days of work, one implementer)

One coordinated change, developed on a dedicated switch branch starting from the
signed-off Phase 1 commit, with one checkpoint commit per part (`2a`–`2d`,
building or labeled intermediate while the tree is mid-switch, committed only on
the user's word) so work never sits only in a working tree across days and every
gate runs on a fixed tree; it lands on the main stack whole after G2a, as the
design note's section 8 lays it out. Item 1.3 only prepares `plainFallback` /
`plainFallbackValue` and its controls (`PlainDefaults`, `TotalReadSoundness`);
switching the five off-domain total specification reads (`nativeSlot`,
`typedVariantField?`, `SpecMap` `get`/`valueAt`/`keyAt`, `indexVector`,
`ResultOf`) to `plainFallbackValue` happens in part 2d (`U5`), after naming any
baseline expectation changes for the user. `LoanDeathSoundness` is already split
into its standalone `VerexIRTests` modules (`LoanDeathSupport`,
`LoanDeathSoundness`, `SliceLeafSupport`, `SliceProvenanceDecls`,
`SliceProvenancePositives`, `SliceProvenanceNegatives`, `SliceSemanticControls`).

It is estimated in four parts, each verified by its targeted control suite
before advancing to the next part and before running the full gate:

| Part | Content | Per-part controls (before full gate) | Days |
|---|---|---|---:|
| 2a. Family and towers | `Carriers` with `holds` in place of `lent`; one carrier, one row type, one variant type; `deepHolds` over the carrier; `Carriers.instantiate`, `Carriers.runtime`, `Skolems` and its one pair of transports; the parked Given frame's types adapted so that it compiles | `lake build VerexIR` (`Types`, `FamilyTransport`, `Given*` compile cleanly) | 4 to 5 |
| 2b. Steps and quantified values | `checkingAt`; the denotation's arms (using 1.6's collection preservation lemmas); a prophecy's choice with its assumption; `Contract.validArgs` and `Contract.validResult` wired to `HList.Holds` and `ResultShape.Holds` across `Satisfies`, `Contract.summary` and `Meanings`; the mandatory premise bundle of joins and loop frames; the plain selectors | `VerexIR.Tests.SharedContinuations`, `VerexSL.Tests.ContinuationJoins`, `VerexIR.Tests.CollectionAndTransportHolds`, `VerexSL.Tests.SoundnessCounterexamples` | 3 to 4 |
| 2c. Storage and the bridge | the flat stored value inside the two operations; the global borrow; the encoding, the transports, collection resources, allocation and unnamed-resource agreement, loan correspondence and returned storage justified again | `VerexIR` agreement/bridge build, `Check/Storage/MutationEnds`, `Check/Generics/GenericStorage`, `DenotePerformance` storage targets | 2 to 3 |
| 2d. Consumers, inhabitants, generation | the consumers that carry kept behavior replaced (whatever opens or observes `Holds` identifies a fact by its whole proposition — `carriers`, type, and value — carrying over `VerexIR.Tests.HoldsWorklistIdentity`); those that only recover a certificate deleted with `forget` and `ofDeepHolds` (including `SliceLeafSupport.lean`, the 6 `pragma heartbeats` lines in `SliceProvenanceDecls.lean`, `reduceProjection?` in `Solve/Decide.lean` `decideInstance`, and the Phase 1 storage/slice bridge lemmas and simprocs in `Solve/Normal.lean`); inhabitants by productivity; off-domain total spec reads switched to `opaque` plain fallbacks (`U5`); `verex.givenRoute` defaulted to `false` (`U2`); the carrier instance, the `own` predicate and the frames regenerated; authored proofs repaired | split `LoanDeathSoundness` / `SliceProvenance*` / `SliceSemanticControls` modules (with `SliceLeafSupport.lean` and the 6 `pragma heartbeats` lines deleted), `VerexIR.Tests.HoldsWorklistIdentity`, `PlainDefaults`, `TotalReadSoundness`, `SharedStateLabels`, `CertifiedFieldProjections`, `bench/lean/OrderedMap.lean`, then full G2a and G2b gates | 3 to 4 |

Then two gates, kept apart because they answer different questions.

**G2a, semantic acceptance.** The tree is plain and correct:
- the four suites, prover-smoke and the Move Prover's lean gate pass, with
  the ledger;
- every row postponed at G1 is back, or goes to the user with its cause — in
  particular, on the `find_entry` pair (`SourceVerify/library_iter_adapters` and
  `SourceVerify/library_iter_adapters_errors`) the positive verifies `ensures_2`
  and `invariant_1_step` while the negative fails at `spec.lean:7` (`ensures_2`,
  `spec.lean:6` before adding the pragma) with no other failure beyond those the
  positive shares (demonstrating `.value` vs `.key` discrimination) and its
  `pragma heartbeats` is re-measured and lowered or removed — and with explicit
  controls for the postponed global-read and behavioral-congruence recoveries
  (`stored_fun_values`, `amm_example`, `behavioral_congruence`): two namings of
  one read (`nameSomeReads`) produce two plain variables that `subst` unifies,
  and `strengthenEntries` over the plain row does not depend on running before
  or after substitution;
- the controls pass, including `SliceLeafSupport.lean` and the six `pragma
  heartbeats` lines in `SliceProvenanceDecls.lean` deleted and the eight
  positive slice controls in `SliceProvenancePositives.lean` verifying by bare
  `verify` within the `250,000` default budget (or any remaining slice-borrow
  completeness gap reported explicitly to the user);
- `OrderedMap` 17 of 17, the benchmark's problems and the framework sample at
  or above the reference's counts, the same target identities;
- the names deleted by the switch have no occurrence.

**G2b, cost.** Each target of the reference at or under its figure, and
certification at or under the reference's. If it is missed the difference is
attributed before anything else is done: per solver step against the reference
profile, then by the revisions named in phase 0 for the changes that are not
the certified layer. Recovery is boxed at five days, after which the state is
taken to the user with the measured causes; an exception for the stronger
theorem is the user's (U4).

Nothing lands on main before G2a. The nominal emission waits for G2b or for
the user's decision on its exceptions.

### Phase 3a. Delete the dead consumers (1 to 2 days)

In the files the nominal work is about to enter (`Solve/Normal.lean`,
`Solve/Base.lean`, `VerexSL/Contract/Values.lean`, `Term.lean`): what phase 2
left as identities and dead code, namely the certified twins of the solver's
routines, the `*Certified?` selectors and `certifyingAt`. Otherwise the same
metaprograms are edited twice. Mechanical; the gate is G2 again and the count
of the deleted names at zero.

### Nominal track

**N0. Design note and prototype** (beside phases 1 and 2; 2 days of design).
The carrier of a nominal value (a generated Lean type per declaration, or the
library's table functor), the one spelling of N2, the instantiation cache, and
how a recursive group will fit. Decided on three units of the cost tests with
the setup and two targets measured, and reviewed, before integration.

**N1. Library** (beside phases 0 to 2; new files only). The template files
that use no part of the invariant layer are taken as they are.
`NativeModels.lean` is restated over the plain nominal type, and the files
below it follow with three renamings and their validity clauses removed. The
table provider is ported. Gate: the library builds in the package and its own
tests pass; nothing uses it.

**N2. A unit's instance** (after G2; 3 to 4 days). The models of a struct and
an enum over the plain nominal type, which the committed tree does not have,
the table generator, and the per-unit instance without an invariant, whose
`supports` holds of the declarations it carries. Gate: a hand-written
two-declaration fixture and a generated one agree; a unit's setup meets N3.

**N3. Operations** (5 to 8 days). The owner-indexed operations, `Compile`,
quotation, the solver's observers, the proof language's steps. This is the
largest step of the track: the structural layout is opened by pattern in about
160 places (101 in `Types.lean`, 22 in `Term.lean`) and named by quotation in
about 34 places of the metaprograms, and each gets a nominal arm or goes
through the model. Gate: the fixtures of N2 verify with nominal values;
nothing is emitted for other units.

**N4. Emission, in stages** (4 to 6 days). `ntyOf` has no nominal path today,
and the coverage law of a unit's instance is stated against `ntyOf` itself, so
emitting a nominal and proving coverage are one obligation per stage. One
stage is one commit with the gates run: a unit that opts in; every non-generic
declaration; generic declarations; Rust units. Gate at each stage: G2 holds,
and no target is above its reference figure. Whether a unit may hold both
representations across its declarations, or switches whole, is decided in N0.

**N5. One representation.** `supports`, `covers` and the descriptor constants
are removed, and the structural layout with them, except for owners with an
abstract layout: their model is an open design question, and the structural
layout stays for them until it is settled. Gate: G2; the structural
constructors have no other producer.

**N6. Recursive declarations**, as stage 6 of
[`native-nominal-types.md`](native-nominal-types.md).

### Phase 3b. Delete the rest (3 to 5 days, after N4)

The second tower and the plain family, the respellings and transports,
`plainReframe`, the inhabitant hypotheses and their generation, the tests of
the carrier machinery. The frame's second pair of transports
(`plainToRuntime`, `plainOfRuntime` and their laws) are fields of `Skolems`,
so removing them changes every frame literal, the generated ones included.
What came with the carriers' change is reverted to its earlier text; what came
later is budgeted separately. Gate G3:
- the names listed under "Goes" have no occurrence;
- `verex-ir` without tests is smaller than the base by at least the certified
  layer's share, and `Types.lean` is back near its size before the carriers
  plus the Rust constructors;
- G2 holds.

### Phase 4. What remains

The attribution items left from G2; the failures the ledger lists as open; the
design documents rewritten to the state reached. Then, and only with a
measured class of targets that needs it, the question of D6.

## Gates

The reference, from CI run 41770462997. Heartbeats in millions, native, the
benchmark tool's own run. The data is in the tree:
`bench/reference/2026-11-03-reference.json`, the run with the carriers beside
it (`2026-11-10-carriers.json`: 10.01G verification, 1.44G certification,
`OrderedMap` timed out), and the framework sample of the same commit
(`2026-11-04-framework-sample.json`).

| `OrderedMap` target | Reference |
|---|---:|
| `empty` | 3.2 |
| `length` | 1.9 |
| `borrow_key_at` | 5.2 |
| `lower_bound` | 8.8 |
| `lower_bound_loop` | 35.4 |
| `contains` | 19.3 |
| `borrow` | 29.1 |
| `add` | 54.6 |
| `remove` | 43.1 |
| `missing_scenario` | 18.0 |
| `absent_lookup_scenario` | 40.2 |
| `duplicate_scenario` | 39.2 |
| `bool_key_scenario` | 58.0 |
| `ordering_scenario` | 78.4 |
| `lookup_scenario` | 84.6 |
| `remove_scenario` | 108.6 |
| `remove_edges_scenario` | 128.9 |
| Sum of the targets | 756.6 |
| Verification phase | 768.1 |
| Certification | 82.9 |

| Measure | Reference |
|---|---|
| Benchmark problems verified | 30 of 31; `amm` failed and is an expected rejection since |
| Benchmark, all 31 problems | 4.97G verification, 0.98G certification, 6.39G in total |
| Framework sample, 24 modules, 588 targets | 382 verified, 156 budget stops, 48 failed, 2 interrupted |
| `primary_fungible_store::deposit` | verifies within 300M and bounded wall time, the gate the carriers set and did not show |
| Fixture expectations | the baselines at `81ad86eb3ee`, with the ledger |

A figure counts as met within 5%, the spread of repeated native runs on one
tree. The benchmark is run by CI on a draft PR, so that the instrument is the
one that produced the reference.

## Working rules

1. **One line of work, committed.** Every step ends in a commit on the stack.
   A scratch copy is for building; nothing lives only in a scratch copy for
   more than a day.
2. **The reference column.** Every cost or completeness table carries it. No
   figure is reported against another state without it.
3. **Stop rules.** Work stops and the state is taken to the user when a phase
   passes one and a half times its estimate; when the gate's count of red
   items has not gone down over two working days; or when a repair needs a
   new representation, a new layer, or a rule that reconciles two spellings.
4. **Repair the producer.** A second spelling is fixed where it is emitted.
5. **Read before running.** A diagnosis is made from the source; a probe
   answers one stated question.
6. **Parallel only where the files are disjoint.** Phase 1's items are
   independent and partitioned by file, and up to three proceed at once, each
   landing whole and green. The switch of phase 2 and every repair that
   converges on one leaf pipeline has one implementer. Read-only
   classification may run beside either, and the nominal library is ported
   beside them because it is in new files. Design, diagnosis and review are
   done by the strongest model; an implementation follows the phase's
   specification and reports a disagreement instead of deviating.
7. **No budget, guard, baseline or specification is edited to pass, and no
   admission is added.**
8. **The controls run at every commit** of phases 1 to 3.
9. **Simplify what was changed, after its gate.** Once a phase's change is
   green at its gate, its own diff gets one simplification pass for reuse,
   duplication and dead code, and the gate is run again. No such pass is made
   on code that has no recorded baseline, on code a later phase deletes, or
   before phase 0 has recorded the base. After phase 3b the pass covers the
   files that shrank most. Code that is dead at the base, such as
   `DataOwn.behavioural`, which is written and never read, is deleted after
   phase 0 and not before.

## Risks

| Risk | What shows it | What is done |
|---|---|---|
| Joins or loops lose a `Holds` fact that the carrier kept by typing | fixtures that change verdict after the switch | item 1.6 and part 2b: the mandatory premise bundle of `Node` in `SharedContinuations` and loop frames, proved by each arrival from its facts |
| A settlement or slice equation assumes a certified side before checking its plain side | the loan-death and slice controls | items 1.1, 1.2 and 1.2b; any negative control that verifies stops the phase |
| Flat valid storage (`Memory`, Option A) adds transport or normalization cost | `deposit` and the storage cost targets | item 1.5 routes the denotation and solver through `Skolems.plainAt` and `Skolems.writePlain` with a folded write rule, measured before and after the switch; Option C is the fallback if dependent packaging dominates |
| Off-domain total specification reads leak structural zero layouts or lose relied-on facts | the total-read controls and fixture baselines | item 1.3 and part 2d (`U5`): `opaque` plain fallbacks keep off-domain reads unspecified; specifications state the presence/bound premise where a fact is needed |
| The tree is plain and still above the reference | G2b | attribution as described under G2b, before any further phase |
| A metaprogram that matches the subtype's shape stops firing without an error | timeouts and moved baselines after the collapse | phase 2 replaces or deletes each such declaration in the switch's own change (C9), from the inventory made in phase 1 |
| The collapse cannot be staged finer than one coordinated change across `Types.lean` and `Term.lean` | `Carriers.lent` and the family transports (C8) | items 1.1–1.6 land all representation-independent pieces first; phase 2 executes one coordinated switch on a dedicated switch branch with per-part checkpoint commits |
| The nominal operations are larger than estimated: about 160 pattern sites and 34 quotation sites open the structural layout | N3's progress against its list | the list is made in N0; N3 stops by the stop rules, with the structural layout still working |
| Nominal generation costs what it did | N0's prototype, N2's gate | N3 of the requirements: no proofs per declaration; the table provider |
| Nominal values change every leaf's spelling at once | N4 | emission in stages, the structural layout kept until N5 |
| The Rust-on-Move branch conflicts with the removal | restack at each gate | it touches 120 lines of `verex-ir` and none of the certified layer |
| The parked Given route rots while it is off after phase 2 | its explicit tests, and when it is switched on after G2 | `verex.givenRoute` stays on in phase 1, compiles at every commit after it flips off in phase 2, and its explicit tests keep running (`U2`) |

## Documents

| Document | State |
|---|---|
| [`../designs/data-invariants.md`](../designs/data-invariants.md) | describes the source until phase 2; rewritten in phase 4 from "Data invariants as conditions" |
| [`../designs/given-parameters.md`](../designs/given-parameters.md) | stands, except "ADTs keep their certificates" and the plain and certified roles |
| [`native-nominal-types.md`](native-nominal-types.md) | identity, plain basis, generation, operations and stages stand as revised by "Nominal types" here; "Certification" is withdrawn |
| [`one-spelling.md`](one-spelling.md) | superseded as the order of work; its principles and its target normal form stand |
| [`nominal-invariant-sorts.md`](nominal-invariant-sorts.md), [`../status/nominal-invariant-sorts.md`](../status/nominal-invariant-sorts.md) | closed |
| [`data-and-global-invariants.md`](data-and-global-invariants.md) | the global-invariant scheme, the audit of native models and the behavioral clauses stand; the problems of the certified layer close with it |
