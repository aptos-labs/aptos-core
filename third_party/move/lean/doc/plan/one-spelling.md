# One spelling: from certified carriers to native values

The plan of the work that follows the certified carriers
([`../designs/data-invariants.md`](../designs/data-invariants.md)). It orders
what is open in [`data-and-global-invariants.md`](data-and-global-invariants.md)
and what [`native-nominal-types.md`](native-nominal-types.md) designs, and it
takes precedence over the order either of them suggests.

> **Superseded as the order of work by [`plain-values.md`](plain-values.md)
> (2026-11-13).** The certified carriers this plan follows are withdrawn.
>
> - The goal, the principles and "Target: the leaf's normal form" stand and
>   are that plan's too. With a plain carrier the Layer axis of the diagnosis
>   has no second spelling left.
> - D2 (a generic callee at the caller's row), D3 (a narrow family literal)
>   and the one element default of D4 are landed and kept.
> - D1 keeps the subtype as the semantic carrier and is withdrawn. Its leaf
>   form, a plain value with one folded fact, is the form of an invariant in
>   the new plan.
> - S3 and S7 do not apply. S4, S5, S9 and S10 are the nominal track of the
>   new plan, without certification. S6's single producers remain open items.
> - [`nominal-invariant-sorts.md`](nominal-invariant-sorts.md), which took
>   precedence over D1 and S4–S5 for a time, is closed.

## Goal

1. Every suite passes: the four Lean suites, the Move Prover's lean gate, the
   CLI and framework Lean tests, the benchmark. A recorded baseline holds an
   error only where the fixture marks it as intended; a test that landed red
   is repaired, not kept.
2. Verification costs no more than before the carriers, target by target
   (`DenotePerformance`, `CompositionPerformance`, `OrderedMap`, the framework
   benchmark), and less where a term became smaller.

## Principles

- **One spelling.** A type, a value and an operation have one spelling in a
  leaf, by construction of what produces them. No rule reconciles two.
- **Small terms, short walks.** A term names what it uses. Nothing a
  reduction can reach is spelled inline: a definition a leaf can unfold is a
  cost paid at every mention and at every pass that walks the result.
- **An instantiation is computed once.** A generic thing at given arguments is
  a named constant with a fixed meaning; a mention does not repeat the
  substitution. This holds for a function's body at a row and for a
  declaration at its type arguments alike: the layout of a struct or an enum
  at an argument row, and each operation's equation at that layout, are
  computed once per row and reused, not derived where a consumer opens the
  declaration.
- **Repair the producer.** A wrong or second spelling is fixed where it is
  emitted. A rewrite at the consumer is refused, and the existing ones are
  retired as their producers are fixed.
- **Analysis before experiment.** A rebuild after an edit of `Types.lean` or
  the solver costs minutes and a probe interprets the verifier. A decision is
  taken from the source; a probe answers one stated question.
- **Authored proofs and the proof language follow the leaf.** Where the leaf's
  form changes, authored proofs are rewritten and the proof language is
  extended; neither is a reason to keep a form.

## Diagnosis

The failures the carriers introduced and the cost they added have one shape:
a leaf holds one thing at two spellings, and no equation relates them. There
are four axes of it, and a few defects of single producers beside them.

| Axis | The two spellings | What it causes | Fixed by |
|---|---|---|---|
| Declaration | a declaration's constant (`<decl>.eqb`, `.ofDeepHolds`, `.readPlain`) against its structural row (`Sum.inr …`, `x.val.fst`, `Which.project?`) | NOMINAL (41 headlines, 16 tests), variant projection (7) | native nominal values (S4, S5) |
| Layer | a value at its certified carrier against its plain layer (`ofDeepHolds`, `forget`, `plainOf toPlain τ` against `τ.carrier`); an element default at the certified layer, at the plain layer, and through an `Inhabited` instance of a parameter's carrier | ELEM (55, 7 tests), SUB (40, 11 tests), the selected-clause duplication, probably `no_violation` | one element default (D4), the leaf form of a certified value (D1) |
| Family | a generic callee's values at the induced family against the caller's (`Carriers.lent i`, induced ascriptions, erased transports) | `OrderedMap` at 2–4× its cost, seven targets over their caps; the consumer repairs `canonicalFamilyResult`, `forgottenParameterEquation`, `forgottenVectorEquation`, `selectedForgottenValue`, `plainParameterField` | instantiation at the caller's row (D2) |
| Frame | a family's constructor literal, which a reduction exposes, at the width of what it spells inline | the cost of every pass that walks a wide literal (measured: one `grind` step nine times dearer with twenty flat fields) | a narrow literal: a group of operations is one field naming a bundle (D3) |

Single producers: a closure's behavior stated at two sites (46 headlines, 10
tests); loan-death facts missing on an aborting path, which leaves loop
invariants uninstantiated (23, 2 tests); a lender that reads its index
operands where the loan ends (the index-write refusal); a returned reborrow of
a field the invariant reads; a holder that owes an ancestor loan's invariant.

The remainder are behind these (MASKED, 25) or are budgets that a smaller term
relieves (COST, 13). The five tests once tagged FRAME are not a second
spelling: their leaves hold one spelling on both sides, and what is missing is
the boundary instance of a quantified invariant and the budget to find it.
They belong with the loop-invariant instances and with COST.

## Target: the leaf's normal form

A leaf is written in native Lean. Type codes (`NTy`) and families are what the
denotation reads; they do not reach a leaf except where a value meets storage
or an opaque specification function.

| Thing | Its one spelling in a leaf |
|---|---|
| A declared type at arguments | the generated Lean type applied to the arguments' types, `S α β`; a ground instantiation the verifier uses is a named constant |
| A value of a declared type | a value of that Lean type; a field is its projection, a variant its constructor, a case analysis a `match` |
| A scalar, a vector, a table, a tuple | the native type at native element types |
| A certified value | its plain value and one fact that its invariant holds, the fact folded behind the declaration's named predicate |
| A declaration's invariant | a generated predicate of the native value: its own clauses and its fields' predicates; opened once per value, by one equation |
| An element default | the type's one named inhabitant |
| A generic callee at arguments | the callee's meaning at the caller's row, a named constant per function and row |
| A unit's family, a frame | a name; no literal |
| A closure's behavior | the target function at its full argument row, whatever the site captured |

## Decisions

Each was settled by reading the source. Its inventory of producers, statement
forms and edits is written into the design it belongs to with the step that
implements it.

**D1. What a leaf sees of a certified value.** The semantic carrier stays the
subtype, so a read owes nothing and nothing is looked up per step. A binder of
a verification condition is the plain value and one fact that its invariant
holds, folded behind a name per declaration instance. A position that needs a
certified value owes that fact.

- It is one equation and its companions, not a rule set: a carrier is in
  bijection with its plain values that hold (`forget`, `ofDeepHolds` and their
  round trips), so every plain-layer rule is the certified rule composed with
  `NTy.forall_carrier` and the row, environment and result forms of it. No
  `Term` arm changes.
- The walk's environment keeps one certified spelling, `ofDeepHolds τ x h`,
  and each walk-time selection (`selectCertified`, `projectCertified?`,
  `getCertified?`) has one equation at it, so a leaf is plain.
- Two operations take a certified term and outlive the walk: a memory write
  and an element default. A write gets a plain operation at the frame
  (`Skolems.plainAt`, `writePlain`) whose read-over-write equation takes the
  fact as a side condition; the default is the plain one. An opaque
  specification function's result of a declared type is a plain value with no
  invariant, as it is today: giving it one would be a new assumption.
- The fold is a one-field structure, `NTy.Holds τ x`, which no transparency
  setting opens; a declaration's opening is one generated theorem. The
  `deepHolds` equations leave the normal form and the hypothesis split does
  not open a certificate, in the same step that introduces the fold, so a
  leaf never holds two names for one fact. A fact is opened by one pass at
  the leaf's boundary, keyed by the plain value. Facts exist per binder and
  never per projection, so two certificates of one value are one hypothesis.
- Statement forms: `Contract.atPlain` with its `satisfies` and summary rules,
  and one `EnsuresOfPlain` in place of `EnsuresOfEntries`, `EnsuresOfSettled`
  and the inline existential over finals; it is equivalent to `EnsuresOf`
  because a row's settled entries and finals determine it.
- A clause's binder over a declared type, certified today, becomes the plain
  value guarded by the fact. Parameters of lemmas and of recursive
  specification functions stay plain without a fact, as today.
- It is independent of the layout, so it lands on the structural layout before
  the nominal work and deletes the layer repairs that work would otherwise
  port.

**D2. A generic callee at the caller's row.** The walk inlines a generic
callee's body compiled at the call's row, and no leaf is typed at an induced
family.

- The body at a row, its named constants per function and row, the call at a
  row and the proof that relates it to the generic meaning at the run exist,
  and are used today only where a type argument holds a mutable reference.
  Two theorems make it the rule of every inlined call:
  `propheticMeaningAt_eq_specialized` and `wp_specialized_of_compiledAt`. No
  substitution theorem over `Term` is needed, and the call node, the compiler
  and both axioms are unchanged.
- The identity row, a generic caller passing its own parameters, is the
  callee's existing compiled constant.
- In-body conditions (loop invariants, assertions, callee preconditions) and
  derived summaries are built at the row and keyed by site and row.
- A contract-routed callee's contract is built at the row from the source and
  proved equivalent to the generic one, once per contract and row; a proof
  that does not close is an error, not a kept spelling.
- The transports and `Skolems.instantiate` leave the walk and stay in the
  model: a generic function's own theorem, call cycles, and the equivalence
  proofs.
- Trusted base: no axiom's statement changes. `compileFunction_agrees` already
  quantifies over a substitution; every inlined generic call then rests on
  that half of it, where today only calls at reference-holding arguments do.
  `assumptions.md` and `denotation.md` say so.

**D3. A narrow family literal.** A family stays reducible, and what its
constructor literal spells inline is kept narrow: a group of operations a
family carries is one field that names a definition which is not reducible
(`NominalFunctor`, a unit's `NominalCarriers`), so a pass that walks the
literal meets a name. Making the family, the frame and the interpretation
themselves names was measured and is rejected
([`transitions.md`](transitions.md), "Levers tried and rejected"): the normal
form relies on their reduction, and no subset of them could be named with the
tests green.

**D4. Native sorts.** A binder's type is the native Lean type that
`NTy.carrier` already reduces to at every code but a parameter, a function
type and a declaration; no new cache is needed. A declaration's native type is
the nominal work. The one element default is `NTy.plainInhabitantOf`: the
`Inhabited` instances of a parameter's lent carrier are removed, so a default
cannot be found a second way.

## Order of work

Each step lands with the suites building, its gate met, and the baselines it
moves re-recorded and reviewed. A step does not start before its decisions are
written.

**S0. Ground truth.** Our basis commit adds no diagnostic and no cost to its
parent: the fixtures that differ from their baselines are run once on the
parent and attributed. The cause table of the certified carriers is the
inventory of what is red.

**S1. One element default** (D4). Gate: the `verify_vector` leaves show one
default; then the simproc `selectedVectorPlainRead` is retired.

**S2. A generic callee at the caller's row** (D2): the two theorems; the
collection of rows and the conditions at a row; the walk's rule and the
summaries of instances; the contract at a row; then the deletions. Gate:
`borrow_key_at` and `missing_scenario` at or under their cost before the
carriers with no transport in a leaf, then every `OrderedMap` target within
its cap; the cost benchmarks unchanged or lower.

**S3. The leaf form of a certified value** (D1), on the structural layout.
Authored proofs that name a row's components are rewritten, and the proof
language gains the steps that open and apply a value's invariant. Gate: the
SUB and ELEM groups and the selected-clause targets closed, a group that stays
red re-diagnosed from its leaf; `bitwise_table::add_fail_exists` and `mut_ref`
at or under their present cost; `Check/Storage/TableAllocation`'s `duplicate`
and `verify_table`'s `add_fail_exists` verified (one leaf, decidable and over
its cap because two spellings of a map's entries are each prepared).

**S4. Model and generation of nominal declarations**
([`native-nominal-types.md`](native-nominal-types.md), stages 2 and 3): the
native type, its codec at the storage boundary, its predicate over the native
value (the own clauses no longer read an encoding), the one-layer model, and
the unit's instance as a named definition. Gate: a hand-written fixture, then
generated declarations, with nothing emitted and no baseline moved.

**S5. Nominal operations and emission** (stages 4 and 5). Field, variant and
construction operations denote through the model to native projections and
constructors; every generated declaration is a nominal. The descriptor
constants, the `supports` premise and each repair whose producer is gone are
deleted. Gate: the Declaration row of the diagnosis closed; cost at or under
the parent's on every benchmark target; `NominalCertificateFrames` verified:
its value is read from storage at a generic resource type, and its own clauses
decode an encoding at the frame the type arguments induce while the value
stands at the function's frame, which a predicate of the native value does
not do.

**S6. The single producers**, each with its own fixtures, in this order: a
closure's behavior at its target and full row; loan-death facts on the
aborting path, with the boundary instances of loop invariants; a lender that
carries its operands as evaluated at the borrow, which removes the index-write
refusal; the returned reborrow; a holder that owes what it stores. They are
independent of S2–S5 and of each other, and one of them proceeds beside
another step where its files are disjoint.

**S7. One obligation system.** Behavioural clauses of a data invariant go into
the carrier; the stored-invariant layer and the markers that check nothing are
removed.

**S8. The remainder.** The masked failures are rerun. A target still over a
budget is profiled by step and its term reduced; no budget, guard or
specification is edited to pass. Three items are known:

- The Rust branch raised verification cost by 10–17% on the composition
  targets and re-recorded the cost baselines; `StateLabelCallObservations` and
  `CertifiedFieldProjections` exceed their budgets by that margin. The
  increase is attributed by step and reversed, and the baselines re-recorded
  downward.
- A leaf that is decidable and reported as not established because it is
  prepared once per attempt: the preparation is done once.
- Two failures the Rust branch introduced that are no second spelling of ours:
  `GivenExecution` states a term form that changed (the test is repaired),
  and `NominalNormal` shows a substitution whose parameter arm removes a mark
  while the normal form's replacement of a parameter does not (the place
  where a mark is removed is decided once, then both follow it).

**S9. The structural layout** is removed for what S5 left to it, after the
model of an abstract-layout map owner.

**S10. Recursive declarations**
([`native-nominal-types.md`](native-nominal-types.md), stage 6).

Global invariants and the audit of native models stay in
[`roadmap.md`](roadmap.md); they add features and close no failure above.

## Method

- Design, diagnosis and review are done by the strongest model; an
  implementation follows a written brief and reports a disagreement instead of
  deviating; a mechanical sweep goes to the smallest model that can check
  itself.
- A decision's analysis and a failure's diagnosis read the source through the
  language server and `git`; they build nothing.
- An implementation makes its coordinated edits, then builds once; test
  modules are built by name; the full gate runs once per step.
- Cost is attributed per solver step (`verex.solverProfile`) and per Lean
  category (`lean --profile`), on ours and on a built parent, before a cause
  is named.
- A baseline is taken from the parent on a restack and re-recorded; it never
  records a timeout or a budget stop.
