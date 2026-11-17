# Plain values: the semantics of conditions, facts and the collapse

The design note that [`plain-values.md`](plain-values.md) requires before its
phases 1 and 2. It is a design read from the source at `78dcaf9df45`. Nothing
here is implemented. This is its fourth revision. The first proposed new rules
for prophecies, for what a loan's death owes and for joins, and a review
showed each to conflict with the denotation as it is. The second replaced them
by one translation, and a third review accepted that with corrections. The
fourth incorporates the settlement and slice audit and the user's decisions on
unspecified total reads (`U5`), slice provenance (`U6`), and contract domain
fields (`U7`). Section 12 lists what changed.

## 1. The design in one sentence

**The plain design is the carriers' design read through one bijection.** A
certified value is a plain value together with its validity, so:

- wherever the carriers quantify over a certified value, the plain design
  quantifies over a plain value under the hypothesis `Holds`;
- wherever the carriers construct a certified value, the plain design checks
  `Holds`.

What is checked, where, and in which order stays exactly what it is today.
The representation changes; the protocol does not. Nothing below invents a
rule about when a value is valid.

The bijection is the source's own: `NTy.deepHolds_forget` and
`NTy.holds_forget` give validity of a forgotten value, and `NTy.ofDeepHolds`
with `NTy.forget_ofDeepHolds`, `NTy.forget_injective` and
`NTy.ofDeepHolds_forget` give the reconstruction and both round trips. It
covers values. Three things in this note are **not** its image and are
exceptions with obligations of their own, audited separately:

- a default that was certified becomes a plain default with no fact
  (section 2);
- `DataInvariantsSatisfiable` is removed, which makes the semantics defined
  where it was not, so each former use of an inhabitant is justified again
  (sections 2 and 8);
- the repair of a settlement that assumed its equation unchecked (section 3).

| Part of the carriers' design | In the source | Fate |
|---|---|---|
| The representation: a declared type's value is a subtype of its plain value | `NTy.carrier` at `.struct`, `.enum`, `.nominal`; the second tower `NTy.plainOf`; `forget`, `ofDeepHolds`; `ParamPlain` with `of` and `certify` | withdrawn |
| The predicate: validity of a plain value at every depth | `Carriers.own`, `NTy.deepHolds`, its fold `NTy.Holds` | kept, over the one carrier |
| The protocol: the steps that owe the predicate, their order, and what is valid by choice | `certifyingAt` and its kinds, `Term.handedOver`, `Term.returned`, `Term.assumeCurrent`, `Lender.resolve`, `Mutables.resolve`, the choice of a prophecy among certified values | kept |

The condition layer that predates the carriers (`constructedAt`,
`mutationEndAt`, the condition tables) does not take over. Its markers are
`Spec.pure`, so a condition behind one is the solver's and not the theorem's.
It stays for behavioural clauses only.

## 2. The translation

| The carriers | The plain design |
|---|---|
| `∀ x : τ.carrier` in a statement: a parameter, a callee's result, the environment after a join or at a loop head, a havocked value | `∀ x : τ.carrier, Holds τ x →`, with the one carrier |
| `certifyingAt site kind τ v`, which returns the certified value | `checkingAt site kind τ v`, which returns nothing; `v` stays the bound plain term |
| `Spec.choose` over a certified carrier: a prophecy, a havoc | a choice over the plain carrier followed by the assumption of `Holds`: the choice's domain is the valid values, as it is today |
| `.val` of a certified value | nothing |
| A field read off a certified value (`Var.selectCertified`, `Which.projectCertified?`, `Proj.getCertified?`, `Pat.bindPlain`) | the plain field read; the field's fact comes from the owner's fact by the declaration's opening equation. Extraction, never a new check |
| `ofDeepHolds v h` written directly | the value `v`; the proof `h` is the fact, already in hand |
| A result typed certified in a rule (`wp_summary`'s result, a native model's, an opaque callee's) | the result with `Holds` stated in the rule or the contract: a contractual receipt |
| A decoded stored value (`Codec.certified`) | unchanged; section 7 |
| A certified collection or row built by an operation of the interpreter from certified parts (the vector's insert, remove, swap and concat arms of `Term.denoteIn`, a tuple's or a row's construction) | the plain result, with its fact from the parts' facts by a library lemma of preservation. No new check |
| An inhabitant or default of a certified type | a plain default with **no** fact; see the audit below |

Only a former certification cut, or a repair this note names, introduces a
check. Every direct certificate construction, certified selector, structural
operation on certified parts, abstract or native result rule, decode,
arbitrary choice and default in the tree is classified into one of four kinds
before the change: fact-preserving extraction or preservation, checked
production, contractual receipt, or a raw value without a fact. That inventory
is a preparation (section 8).

**The audit of defaults.** A default that was certified is not the image of a
plain default. `NTy.readBack` decodes with an inhabited fallback, at structs,
enums, arrays, tables, references and nominals, and the old fallback was
certified. Each such use is shown to be one of: unreachable under the
correspondence of the inputs it decodes; confined to a raw interface whose
value never enters an environment that is stated valid; or given an explicit
obligation before it enters one. This audit is a gate before the switch. An
observable default left unresolved cannot be waived by citing the bijection.

The audit was made twice, independently, and the two agree. No default reaches
a check, memory, a slot of an environment, a premise of a join or a loop, or a
contract's receipt: none launders an invalid value into a place stated valid.
What changes is what a **specification** may conclude from a total read. These
reads are total by a fallback, and today the fallback is a certified value, so
its invariant is free; after the change it is an **uninterpreted (`opaque`)
plain value** (`U5`, taking a proof-irrelevant `Nonempty` witness backed by
`NTy.plainInhabitant` solely to discharge `Classical.choice` without axioms),
with neither a `Holds` fact nor concrete structural-zero equations:

| Total read in a specification | Where | Off-domain value | A fact needs |
|---|---|---|---|
| A resource that is absent | `nativeSlot` | `opaque` plain fallback | the resource's existence, that is, a successful read |
| A field of a variant the value is not in | `typedVariantField?` | `opaque` plain fallback | the variant test or the selection's equation |
| A missing key of a map, a position outside it | `SpecMap.get`, `valueAt`, `keyAt` | `opaque` plain fallback | membership or the bound |
| An index outside a vector | the clause translator's `indexVector` | `opaque` plain fallback | the bound; with it the read is the element and has the vector's fact |
| `result_of` where no run returns | `ResultOf` | `opaque` plain fallback | a returning run |
| An unspecified result of an aborting or out-of-domain branch | `nativeArbitrary`, `outsideValue` | `opaque` plain fallback (already opaque) | nothing gives one; it is raw |
| A ghost slot a specification leaves unspecified | `unspecifiedGhost` | `opaque` plain fallback (already opaque) | an explicit statement of the slot |

The rule is one: **an off-domain total read in a specification is unspecified,
and has a fact (or a concrete value) only under the premise that makes the read
successful.** Using an `opaque` fallback rather than `NTy.plainInhabitant`
directly prevents specifications from proving accidental structural-default
equations (such as `!exists<Counter>(a) ==> global<Counter>(a).n == 0`), while
refusing any `Holds` axiom on the fallback avoids bringing back
`DataInvariantsSatisfiable` under another name. This matches the Move Prover's
intent that off-domain reads are unspecified without inheriting Boogie's
accidental `WellFormed` axiom on absent reads. Because `plainFallback` is
indexed by the result type (with a proof-irrelevant `Nonempty` witness), two
off-domain specification reads of the same type denote the same unspecified
value (for instance `global<T>(a) == global<T>(b)` when neither resource exists),
preserving today's congruence behavior without exposing structural defaults or
`Holds`. A specification that relied on the free fact states the premise instead.

Other findings of the audit:

- `NTy.readBack`'s fallback is unreachable for a value the view wrote
  (`NTy.readBack_readWrite`) and at a certificate-free type. The change
  narrows it: today the checked decoder also takes it for a well-shaped value
  that fails its invariant; a plain decoder takes it for a shape mismatch
  only. `NTy.readBack` keeps its irreducible `NTy.inhabitantOf` fallback (used
  internally by `TypedReads` and matched by transport simprocs), and no theorem
  may claim that an arbitrary plain read recovers a valid value.
- No place relies on an empty certified type. `Spec.choose` over an empty
  carrier has no outcome, and the choice over plain values under `Holds`
  reproduces that exactly, so nothing becomes newly provable. The global
  vacuity of a unit with an unsatisfiable invariant, which the satisfiability
  hypothesis caused, disappears.
- `NTy.inhabitant` returns to the total structural plain inhabitant form it had
  before the carriers (`NTy.plainInhabitantOf`), supplying `[Inhabited]` to the
  `opaque` specification fallbacks and `NTy.inhabitantOf` without any
  satisfiability hypothesis. The array, slice, character and nominal arms are
  newer and keep their own productivity rules.
- `VerexSL/Tests/PlainDefaults.lean` and `TypedMaps.lean` (`abortsNewFrom_indexed`)
  are updated to pin the opaque specification fallback spelling and stay the
  tripwire of this gate.

Before the switch, each row of the table is traced to the facts it feeds, and
a control per row shows that the fact is refused without its premise and
granted with it.

`NTy.deepHolds : (τ : NTy) → τ.carrier → Prop` keeps today's arms: a struct's
or an enum's own clauses (`Carriers.own`) and its fields'; through tuples,
vectors, tables and ghost slots; `True` at a scalar and a function value; at a
mutable reference the **prophecy's** validity, the current being open; at a
slice reference both readings; at a type parameter the family's predicate
(section 6). `own` reads an encoding and a generic frame interprets it, so the
single-carrier definitions keep the encoding and family-transport laws `own`
needs today.

`NTy.Holds` stays the fold that no transparency opens, with one generated
opening equation per declaration. A type with no invariant at any depth has
`Holds` equal to `True` by a decision made when the unit is set up, so a
concrete invariant-free type leaves no clause anywhere.

No theorem derives `Holds τ x` from `x`. `NTy.holds_forget` and its kind have
no counterpart.

## 3. Producers

### The step

```
checkingAt site kind τ (value : τ.carrier) : Spec State Failure Unit
wp (checkingAt site kind τ v) ensures aborts s ↔
  NTy.Holds τ v ∧ (NTy.Holds τ v → ensures () s)
```

It keeps the site and the kind (`CertifyKind`: construction, mutation end,
exit, copy out, hand-over). The plain value is bound once, and that same term
is what the check, the later steps and the report name; no solver code may
depend on the former certified result's binder to find it.

### Where, and in which order

The governing invariant is uniform: **before a settlement or borrow equality
restricts an actual runtime value to a certified (valid) domain, its validity
at that domain is established independently of that equality, by a check or an
existing preservation proof.** Borrow-time restrictions on future values retain
matching settlement obligations at the loan's death.

Each call of `certifyingAt` keeps its place, with the settlement and slice
holes repaired. Read at the revision:

| Step | Today | Plain |
|---|---|---|
| A pack (`Term.packAt`) | certifies the packed value | checks it |
| A loan's death on a slot, and at a place with an absent owner (`Lender.resolve`) | certifies the current, then assumes the equation | the same order |
| A referent check (`Lender.resolve` at `.referent`) | certifies the current and assumes no equation | the same |
| The equation of a combined settlement (`Lender.resolve` at `.equation`, emitted by `holderResolves` in `Compile.lean` after the candidates' and the referent's obligations) | assumes the equation and certifies nothing itself. For a place whose path has `hasDeref`, `holderResolves` emits **no** obligation, so a settlement whose possible origins all have `hasDeref` emits the equation alone | **repair:** `holderResolves` emits one unconditional `.referent` first, then `.candidate` for every place origin (`hasDeref` included), then `.equation` |
| A loan's death at a place of a local, no dereference on the path | certifies the owner rebuilt with the current, at every depth, then assumes two equations | the same order; referent validity is implied by the rebuilt owner (`Proj.deepHolds_of_set?`). Sibling fields hold what they hold today, prophecies of other loans included, valid by their choice |
| A loan's death at a place with `path.hasDeref` (`.deref`, `.sliceIndex`, or a path above one) | **assumes both equations, certifies nothing** | **repair:** checks the referent's current first, then checks the rebuilt owner, then assumes both equations. When the path starts with a direct `.deref rest`, the owner is an ordinary reference whose validity is its unchanged prophecy's (`Proj.set?_deref_deepHolds`), so `Lender.resolve_place_deref` erases the owner check; at `.sliceIndex` or a struct holding a reference, the rebuilt owner check stays and catches cross-field violations |
| A candidate holder (`.candidate`) | where the owner is present and the path rebuilds: certifies the rebuilt owner when `!path.hasDeref`, nothing when `path.hasDeref`, the owner is absent or the path fails | **repair:** checks the rebuilt owner whenever the owner is present and the path rebuilds (`hasDeref` split removed; direct-root `.deref rest` erases via `Lender.resolve_candidate_deref`, and the referent is checked once by `.referent`). Today's conservatism over branch-dependent candidates is kept |
| A hand-over (`Term.handedOver`) | certifies the current | checks it. Kept for a call the walk inlines too: an inlined callee can receive an open current |
| A returned reference (`Term.returned`) | certifies the current and assumes no equation; the caller settles it | the same |
| A `&mut` parameter's exit (`Mutables.resolve` at `.cons` and `.at`) | certifies the current, then assumes the equation; at a certificate-free type the check is trivially true and omitted. An absent or moved slot skips both | the same |
| A write into an owner through a path (`Proj.setOwner?`, used by `writePlace` and `takeAt`) | certifies as its definition states | the same |
| A slice borrow or reborrow (`borrowArraySliceAt`, `borrowVectorSliceAt`, `reborrowSliceAt`) | chooses a certified slice and owner and equates the extracted `current` with the certified slice's current, with **no** check when `path.hasDeref` | **repair (`U6`):** checks the extracted `current` elements before the existential assumption when `path.hasDeref` (erased by `Proj.get?_deepHolds` when `!path.hasDeref`) |
| Moving a slice out (`takeSliceAtValue`) | chooses a certified `cleared` owner and equates `path.set? (ofArrays final final) value` with `forget cleared`, without checking the computed cleared owner | **repair (`U6`):** rebuilds the cleared owner concretely, checks it at the owner's type before the update, and installs it |
| A slice reference's resolution (`Mutables.resolve` at `.sliceAt`, `SliceLender.resolve`, `resolveSliceAt`) | assumes `reference.current = reference.final` and checks nothing; carries no lender provenance for an enclosing owner | **repair (`U6`):** checks `reference.currentVector` unconditionally before the settlement equation (`Mutables.resolve` at `.sliceAt` has no path, and `SliceLender.resolve` checks `currentVector` first), and carries slice lender provenance (`SliceLender`) so any enclosing local/candidate owner rebuilt with the settled slice is checked before assuming `current = final` |
| A copy out of an open current (`Term.deref`, `Term.observe`, `Term.readPlace`) | certifies the copy | checks it |
| A publish or a ghost store (`Term.globalPublish`, `Term.ghostStore`) | owes by typing | restates the value's fact as a check before the write; section 7 |

All settlement and slice repairs above are made on the present tree with
`certifyingAt` before the representation changes (section 8). Apart from those
soundness repairs, the checks and their guards are today's, so the translation
adds no logical rejection.

A missing or moved holder skips its settlement and assumes no equation. A
loan handed to a call or returned stays under the present correspondence of
transferred and returned loans.

Known conservatism, unchanged: possible ancestor origins and the candidate
owners of a branch not taken are checked, and where an owner's own clause
relates a field to a sibling under another live loan, the first death has to
prove the clause against the sibling's prophecy. It is rejected when the
clause cannot be proved of every allowed prophecy or from the path's
conditions. Refining that is for later (section 10).

## 4. Facts

A fact is a hypothesis `NTy.Holds τ x` about a term of the goal. By the
translation every value the carriers typed as certified has one, from where
its certificate came:

| Origin | Mechanism | In the theorem |
|---|---|---|
| A check | the hypothesis of `checkingAt`'s rule | yes |
| A parameter | a clause of `requires`; for a `&mut` parameter, of its prophecy by choice and of its current by the clause `Term.assumeCurrent` states today | yes |
| A call's result, and what a `&mut` argument holds after the call | clauses of the callee's `ensures`, proved in the callee | yes |
| A prophecy | the domain of its choice | yes, by the denotation |
| A read of storage | section 7 | yes |
| A field | the owner's fact, opened | derived |
| An equation | from `Holds τ c` and `c = p` | derived |

### One form at an interface

Where a producer hands terms to consumers, each kind of thing has one form.
The producer emits it and the consumers read only it. Two forms of one thing
at the same interface are a defect of the producer that emits the second; it
is repaired there, not by a consumer that accepts both, a recognizer that
folds one into the other, or a step run again until the two meet. The form is
stated here, independently of what the normalizer happens to leave, and the
normalization is idempotent on it. Structural steps (a case split, an
injection, a substitution) preserve the form at their output: however a step
is implemented, it does not expose another spelling to the next consumer.

At the prepared leaf, for what its consumers are presented with:

- A fact is `NTy.Holds τ v`, folded, with this one head, also at a type
  parameter over a family that is not known. `Carriers.holds` appears only
  where the family is known and its predicate reduces (section 6).
- A type is resolved: no substitution pending on a spelled declaration, and a
  frame's reading of a parameter is that reading, with no dropped mark left
  around it. A native value's type uses the descriptor of the caller's
  family; no unfolded carrier, as a raw product or sum or a projection of the
  frame's carriers, stands in its place.
- A value at the caller's family has no transport around it.
- A proposition has neither the wrapper of a default argument nor the marker
  of an obligation. Which obligation it was, and where in the source it
  came from, stay in the diagnostic record, and an obligation that is still
  owed keeps its identity until it is decided.
- A statement the kernel makes is instantiated by the solver, which does not
  state its telescope again or encode the same fact a second way.
- A read over a container operation, where the facts in hand decide the
  conditions of the operation's lookup law, is reduced by that law to the
  value written or to the read of the base at the index the law gives.
  Where they do not, the leaf keeps the read of the operation's result and
  the normalization does not expand it into the cases; a proof applies the
  law and splits its conditions where it needs to.
- A vector grown by one element is `SpecVector.push` of that element, in a
  clause as in a program: a clause's concatenation with a one-element
  literal is that push, and other concatenations stay `SpecVector.append`. A
  push onto a literal vector is the literal, where the grown length is within
  the bound; a symbolic vector's push stays. The values of a nest of appends
  and pushes are one concatenation under the single bound on its total size;
  without that bound the nest is left as it is, and a lone push is read by
  its own law.
- The element of a successful read of a typed literal lookup, an array's or a
  list's at an explicit carrier, is the clause's total read of that position,
  and the position's bound is a fact beside it. A symbolic container's
  successful read is a named element with its lookup equation. The total
  read's fallback is the type's, never chosen from the carrier alone, and two
  fallbacks are not identified off the domain.
- An owner's validity clause reads a fixed-width map value through the
  integer view its other clauses read; a value of another kind keeps its
  native reader.

Forms that differ because their roles do are not two forms of one thing, and
each has its stage and its conversion: the runtime encoding and the native
value at the codec; a local's plain value and the cell of storage that holds
only valid values; a folded fact and the clauses it opens to; a fact over a
generic family and its transport to the caller's; the resolution of a type
and a substitution into it; a container operation and a read of its result;
a function value's carrier and the behavioural predicates about it; a slice
view backed by its container's loan and a slice reference standing alone.

The check of a prepared leaf, which rejects runtime values, is extended to
the specific forms the list above excludes and the implementation names, and
it reports the boundary, the term and its origin where one is known.

### Prophecies

A prophecy is chosen among valid values, as today: a local borrow's, a global
borrow's, which the denotation writes into memory at once, and a returned
reference's alike. One rule, and the present one. It is justified by the
order of every settlement: the current is checked before the equation that
identifies it with the prophecy, so a run whose final value is invalid fails
its check and is not pruned by the equation. The settlements are the rows of
section 3 that assume an equation, the standalone `.equation` among them;
with the repair every one of them is preceded by its check on every path that
reaches it.

A reference that a callee returns live is not settled at the callee's exit.
Returning it does not choose its prophecy again: the prophecy keeps the fact
it had from the incoming value or from its original choice. Its current is
checked by `Term.returned`, and the caller settles it. A specification that
reads the final value reads the prophecy. The contract's settlement premises
(`settledReference`, the `readsFinal` case of `buildContract`) keep their
form, and the prophecy's fact does not depend on them (section 5).

### What is open

The current of a mutable reference, including one reached inside an
aggregate. It has no fact until a step checks it. A slice reference is not
open in this sense: both its readings are valid today and stay so. The slot of
a lent local holds the prophecy during the loan and is valid by the choice. So
no new analysis of which subvalues are closed is needed for the facts an
environment carries. The borrow analysis still decides which deaths the
compiler emits; it does not decide a set of premises.

Two kinds of term have the plain type and are not asserted valid for that: an
intermediate awaiting its check (a rebuilt owner, a selector's or a path's
result, a value passed to a check), and a raw default until the audit of
section 2 has placed it.

### Joins and loops

Today a join's continuation and a loop's head quantify over an environment of
certified values. The translation is one mandatory premise:

> `Holds` of every value the node newly abstracts that was typed certified:
> each **present** slot of the environment, the flow's result value, and the
> captures it abstracts. At a reference this is its prophecy's validity, at a
> slice reference both readings', and `True` at a type with no invariant. An
> absent or moved slot has no value and no clause.

Each arrival proves the premise instantiated at its own arguments, from the
facts it holds. A shared continuation's node already has fresh parameters and
an instantiation per arrival (`Arrival`, `Node`, `sealNode`,
`summaryArguments` in `SharedContinuations.lean`); it gains this premise
bundle in `Node` and in each arrival's bookkeeping, instantiated through the
same shaping and replacement as the other summary parameters
(`withEntryArguments`, `EntryArgument.select`), added to `sealNode`'s
telescope and passed in `finish?`; `Node.hypotheses` and the solver's count of
introduced binders follow. A bundle that is trivial normalizes away. Memory is
valid by its type under option A and needs no premise; the optional premise
for behavioural stored invariants stays beside this one. Unlike the optional stored-invariant premise of today
(`proveStoredInvariants`), this one cannot be dropped when a proof is not
found: an arrival that cannot prove it is an error at that arrival. A loop's
frame states the same for the slots its body changes.

### Calls and exits

- A call through a contract: the caller proves the `requires` clauses with the
  facts it holds and receives the `ensures` clauses. A function value's
  invocation does so through its invocation contract, and a recursive call
  through the contract it is proved by induction against. No call path gains
  an entry fact it did not discharge.
- An invocation of a function value whose target the walk does not see
  receives the validity of its result from a behavioural fact of that value:
  at arguments that hold, every result of a run holds its type's fact. The
  fact stands beside the other behavioural facts of a function value (that it
  keeps memory, that it is scalar). A function with a function-typed
  parameter assumes it; a call that passes a function value proves it, from
  the target's contract, whose result validity is unconditional, or from the
  caller's own assumption where a parameter is passed through; a stored
  function value carries it in the behavioural stored invariant. It is stated
  once and the method contract of a trait method reads the same statement.
  It is not part of `NTy.Holds`, which stays `True` at a function value, and
  it is not stated for a result type without invariants. A function with
  a body under `pragma verify = false` assumes it as a verified one does: the
  pragma turns off the proof of the body, not this assumption, and a caller
  that passes a named function proves the fact from that function's theorem.
- A call the walk inlines: there is no boundary, the facts remain, and the
  hand-over check stays (section 3).
- An exit proves the validity of its results, which is unconditional
  (section 5), by the fact each result has, and a `&mut` parameter's by its
  check and equation. A result with no fact is proved from its clauses there
  or fails there.

The boundary clauses are those `slotDataInvariantTerms` builds, folded: one
`Holds` per value in place of a declaration's clauses expanded inline. The
source range of each declared clause is kept with the opening equation, so
that an authored proof can open a folded clause and address its parts, and a
report names the clause that failed. This is checked against representative
authored proofs before it lands; folding does not by itself remove the depth
limit of the classifier that decides whether a type has an invariant.

## 5. What the function's theorem says

`Contract` in `VerexIR/Proofs/Contract.lean` gains two generated domain fields
with `True` defaults (`U7`):

```
validArgs   : Args → Prop   := fun _ => True
validResult : Result → Prop := fun _ => True
```

`Satisfies` quantifies over `args` and `initial` under `contract.validArgs args`,
`contract.assumes args initial`, and `contract.requires args initial`; a
successful run establishes `contract.validResult result` unconditionally,
`ensures` unless `mayAbort` excuses it, `frame` unconditionally, and
`¬mustAbort`; an abort satisfies `aborts`; undefined behavior is excluded.
`Contract.summary` and `wp_summary` include `contract.validResult result`
unconditionally in `.ok` (quantifying `result` under `validResult` apart from
the guarded postcondition), while `contract.requires` carries the caller-owed
entry obligations as before. Because both fields live on `Contract` directly,
every summary, generic instantiation, closure invocation (`Invocation.lean`,
`Behavior.lean`, `Closures.lean`), and transfer adapter inherits unconditional
domain validity without a parallel contract hierarchy:

- **The arguments (`validArgs`).** Set to `HList.Holds Γ args`, restricting the
  theorem's argument domain as their certified type did. For a reference
  argument that is its prophecy's validity, which a caller may supply and need
  not be chosen by the function. Its current's validity is not part of
  `validArgs`: it is the entry precondition that `Term.assumeCurrent` and the
  generated entry conditions state in `requires`, discharged by the caller's
  hand-over.
- **Every successful result, unconditionally (`validResult`).** Set to
  `shape.Holds result`, constraining the success domain in `Satisfies`,
  `Contract.summary`, and `Meanings` outside `mayAbort` and outside the
  settlement premises of a returned reference. Stating it only inside `ensures`
  would let a caller receive an invalid result from a run whose postcondition
  was excused, and would make a live returned reference's fact wait for a
  settlement. A concrete body proves `validResult` of its results alongside
  `ensures` and `frame`.

Every check is a conjunct of the body's weakest precondition by the
equivalence of `checkingAt`. The assumption ledger loses
`DataInvariantsSatisfiable`, with the audit of section 2 as its condition.
Under option A of section 7 it gains nothing.

## 6. Type parameters

```
Carriers.holds : (index : Nat) → Carriers.carrier index → Prop
NTy.deepHolds (.param i) v = Carriers.holds i v
```

`Carriers.holds` replaces `Carriers.lent`. Every family sets it, an
instantiated family and a Given frame included, and no family can give a
parameter a second value type.

- A generic body is verified over an arbitrary family. Its `requires` and
  `ensures` state `Holds` of values whose types mention a parameter, through
  `Carriers.holds`. The body cannot construct or open such a value, so an
  output's clause is discharged with an input's fact. This is the preservation
  theorem, proved with the function.
- `Carriers.instantiate` sets `holds i` to `deepHolds` of the argument type.
- The one pair of transports of `Skolems` (`toRuntime`, `ofRuntime`) preserves
  `Holds` in both directions, abstract parameter predicates included. This is
  stated as a law of the class. Today's compatible frames satisfy it: the
  certified and plain transports with `forget_toRuntime` and the round trips
  give it, and `Skolems.instantiate` gets it from `NTy.deepHolds_toSkolem`,
  `NTy.deepHolds_ofSkolem` and the outer frame's law. Once the certified pair
  is gone the law carries what that pair supplied; `own_resolve` covers own
  clauses only. No transport
  and no default manufactures a fact. The second pair of transports and its
  laws go.
- The Given route is parked until the tree is back at the reference
  ([`plain-values.md`](plain-values.md), U2): through the collapse its types
  are adapted so that it compiles, and what follows is done when it is
  switched on again. The Given frame gets an abstract predicate over each
  parameter sort.
  `GivenFrame.toCarriers` and the rules that simplify a parameter's validity
  to `True` change, and the transfer adapter's relational evidence
  (`GivenTransfer.lean`) states the predicate's instantiation. Instantiating
  the sort at the valid values instead was considered and rejected: such a
  sort can be empty, which brings back the satisfiability hypothesis this work
  removes.
- Cost, stated as two claims. A concrete instance with no invariant leaves no
  residual clause: the predicate normalizes to `True` at setup. A generic
  proof keeps its abstract premises and discharges them by the facts of its
  inputs; that this is cheap, and what setup and specialization cost, is
  measured against the reference and not assumed.

## 7. Storage

### The choice

Option A: memory keeps valid values. Recommended by the review and this note,
and decided by the user on 2026-11-13.

`Memory unit` maps a resource and a key to an optional valid value: one flat
subtype at the top of a stored value, the value's carrier under its `Holds`,
and no subtype inside the value. Collection resources (`ResourceType.carrier`
at `.collection`, the allocation bookkeeping of `TableMemory`) get the same
flat form per element; their representation is specified with the rest.

- A read gives a plain value and, where the read succeeds, its fact by a
  theorem (`Skolems.holds_plainAt`). An absent entry or a default gets none.
- A write stores a value under its fact. The producer before it has
  established the fact, and a publish or a ghost store (`Term.globalPublish`,
  `Term.ghostStore`) restates it as a step of its own before the write
  (`checkingAt`, with a kind that names the publish). In a valid program the
  step closes from the fact in hand; a value that reaches the write without
  its fact fails there, so the branch of `Skolems.writePlain` for a value
  that does not hold is never selected by a successful checked write. The
  step is erased at a type without invariants.
- **Open, to be measured:** whether the restated step costs anything. It
  should not, since the solver's write rule needs the same fact. The targets
  that publish (`bench::storage::publish`, `GenericStorage`'s
  `publish_generic` and `publish_u64`, the source controls of the publish and
  of the ghost store) are compared on trees that differ in the step alone,
  before G2b. The storage targets of `DenotePerformance` read and borrow only;
  they are the control that the other paths did not move.
- **A mutable global borrow installs its prophecy in memory at once**
  (`Term.globalBorrow`: it reads the stored value, chooses a prophecy, writes
  it, and returns the old value as the current). The prophecy is chosen among
  valid values, so memory holds a valid value at every moment, and the
  loan's death checks the current before the equation, as `Lender.resolve`
  does at `.slot` today. This is the present protocol under the translation.
- `StorageEncodes` and the agreement axiom keep their meaning: a store with an
  invalid value encodes no memory. This is retained only after the bridge is rebuilt
  for the new types: the encoding, the generic transports, the collection
  resources, the agreement on allocation history and on unnamed resources
  (`AgreeUnnamed`), the correspondence of argument and result loans, and the
  resolution of returned storage (`StorageEncodesReturned`). The bridge is
  more than a codec of memory.

### What it costs to build

The plain read and write exist (`Skolems.plainAt`, `Skolems.writePlain`, their
laws), but the denotation and the solver do not go through them:
`Meanings.interpreterOps` and `Term.denoteIn` use the raw transported read and
write, and the solver's memory steps recognize raw memory. Routing them is
part of the implementation:

- the denotation's storage steps and `Meanings.interpreterOps`;
- the solver: the `Memory.set` rules in `Solve/Normal.lean`,
  `verex_memory_aliases`, `closeFrameWrites` and `verex_stored_reads` in
  `Solve/Decide.lean`, each restated over the folded interface;
- a folded rule for a successful write that takes a proof already in hand, so
  that the classical test inside `writePlain` is never split. At a publish or
  a ghost store the proof is the one the restated check leaves in the context;
  at a global borrow it is the prophecy's assumption. A value without its fact
  (a lost join premise, a model's result without validity) stops at the check,
  with the invariant named;
- the fact of a write is kept or cached, not opened again at each read that
  crosses it. A read across a write of a **distinct** resource needs no fact
  (`Skolems.plainAt_writePlain_other`).

The stored value's type also reaches beyond the two operations, and each is a
work item, not assumed to be free: the frame's transports and their dependent
casts (`Skolems.toRuntime`, `ofRuntime`, `Skolems.resource`, `Memory.set`'s
transport along a resource equality), `StoredCase`, returned storage, state
labels (`Term.AnchorOf` pairs an environment with a state), the memory frames
of contracts and of shared continuations, and a loop's memory frame.

### The alternatives

- **B, plain memory with an assumption at each read.** Rejected unless the
  user accepts its hypothesis: for every source execution the public theorem's
  original premises allow, each initially present resource is valid and each
  successful typed observation of storage returns a valid value. The
  hypothesis has to range over executions before the read assumptions filter
  them, or it justifies nothing. Its preservation proof needs natives, tables,
  opaque callees and returned storage covered. With prophecies chosen valid,
  as under the common protocol, a global borrow does not break the simple
  invariant that every stored value is valid; with an unconstrained prophecy
  it would.
- **C, plain memory with one validity invariant of the state** that the
  denotation's write rules preserve, reads being valid by a library theorem.
  The same logical content as A with the evidence beside the memory instead of
  inside each cell. It avoids dependent packaging and adds invariant threading
  through frames, joins and loops. It is the comparison to make if A's
  transports dominate the measurement.

### What phase 0 measures for this choice

On equivalent semantics and with failing targets kept visible:
`primary_fungible_store::deposit`, the storage targets of the cost tests, a
chain of writes and reads with aliasing, a mutable global borrow returned
across a call, and a table allocation and update. Recorded: setup, leaf
heartbeats, memory normalization and transport cost, the number of repeated
openings of a validity fact, and generated declarations.

## 8. The collapse

### The switch

The final interface switches coherently, in one change prepared in a scratch
copy. At the switch every retained consumer of certified validity has an
explicit source for its fact. Its parts:

1. **The family.** `Carriers` loses `lent` and gains `holds`. `PlainFamily`,
   `ParamLent`, `ParamPlain`, `Carriers.toPlain` go. `Carriers.instantiate`,
   `Carriers.runtime`, `Skolems` with its `Holds`-preserving transports, and
   `GivenFrame.toCarriers` with its predicate are restated.
2. **The towers.** One `NTy.carrier`, one row type, one variant type;
   `NTy.plainOf` and `NTy.plain` go. `deepHolds` is restated over the carrier.
3. **The steps.** `checkingAt`; the arms of `Term.denoteIn` of section 3; the
   choice of a prophecy with its assumption; the certified selectors replaced
   by the plain ones.
4. **Quantified values.** The clauses of `requires` and `ensures`; the result
   rule of a summary and of a native model; the premise bundle of a join and
   of a loop's frame (section 4).
5. **Storage**, as section 7: the flat stored value, the global borrow, the
   routing of the denotation and the memory solvers.
6. **Consumers that carry kept behavior**, replaced by reasoning over the
   plain row: `strengthenEntries` with `certifiedEntries?`, `invokeSite` and
   `siteRequiresArm?` with `certifiedCaptures?`, the certification branch of
   `dataInvariantStep` as the rule of `checkingAt`.
7. **Consumers that only recover a certificate**, deleted: the `forgotten*`,
   `certified*`, `plain*Skolem*` and `inducedLent*` simprocs,
   `certifiedLentValue`, `nominalCertifiedValue`, `holdsWithoutCertificates`,
   the leaf guard on `Subtype.mk`. A subtype of type arguments or of a numeric
   range is not one of these.
8. **Inhabitants and generation.** `NTy.inhabitant` gated on `inhabitable`,
   with the witnesses productivity needs; `CarriersNonempty` and
   `DataInvariantsSatisfiable` go; the carrier instance, the `own` predicate
   with its opening equations, and the frames' literals are regenerated.

`forget` and `ofDeepHolds` are deleted in the switch. A temporary alias used
while editing provides no theorem that yields validity and is removed before
the change lands: deleting them is what forces each caller that got evidence
by construction to name its source.

### What lands before it, each on its own

These do not depend on the representation and are checked on the present
tree:

1. The settlement and slice repairs of section 3, with `certifyingAt`, and
   their controls: `Lender.resolve` and `holderResolves` first (1.1), then the
   slice-current, slice-borrow and `takeSliceAtValue` checks (1.2), and slice
   lender provenance for enclosing owners at slice death (1.2b).
2. The controls: the soundness counterexamples and the loan-death set, each
   run with the validated loan facts captured beside the verdict (the
   `CheckedLoanFact`s, the carrier deaths, the holder paths, the candidates,
   and the emitted order of checks and equations). Among them a combined
   settlement whose candidates all dereference, the four slice soundness cases,
   and a result whose postcondition is excused or guarded by a settlement.
3. The inventory of section 2: every former receipt of a certificate,
   classified, the structural operations included; the audit of defaults and
   controls per row of the total-read table, plus the opaque fallback helper
   for off-domain specification reads (1.3).
4. The folded boundary clauses with their opening rules and clause metadata
   (1.4). This touches contracts, reports and authored proofs and is validated
   as a change of its own, not as a renaming.
5. A benchmark and routing of the storage wrappers (`Skolems.plainAt`,
   `Skolems.writePlain` and its folded write rule, 1.5). Before the switch it
   measures their integration under the present representation, not the cost of
   flat memory; the measurement is repeated after the switch.
6. The new interfaces and library lemmas that can stand beside the old ones
   (1.6): `Contract.validArgs` and `Contract.validResult` with `True` defaults;
   collection `Holds`-preservation lemmas for vector, array, slice and map
   operations; the `Holds`-preservation laws of the transports; the join's
   premise bundle in the node's structure.

Where a new interface lands beside the old one, the existing certificates
discharge its evidence; no proof is made twice in the final form.

## 9. The programs

Each outcome is conditioned on the loan facts the validation actually
produces for the program and on every cut in between; a control records those
facts as well as the verdict.

| Program | Outcome |
|---|---|
| `set_left_bad`: `let field = &mut pair.left; field.value = 101`, `pair : &mut Pair`, `Percent.value <= 100` | rejected at the death of `field`, by the repaired check of the current, before any equation |
| Copy-out: write 101 into `p.n`, `let result = *p`, write 0, return `result` | rejected at the copy; the repair afterwards does not validate the copy |
| Two field loans on one pair, one field invalid while the other's loan dies | the death of the first loan checks the pair rebuilt with the second field's prophecy, which is valid by choice. Accepted where the pair's clauses are provable against that prophecy; rejected, conservatively, where a clause relating the two fields is not |
| An invalid value repaired before the same loan dies | accepted if no copy-out, hand-over or other check sees the invalid value in between |
| An invalid value repaired through the owner after the inner loan died | rejected at the inner death |
| A `&mut` opened by a reborrow that died, then passed to an opaque callee | the reborrow's death checks the referent; the hand-over checks the current with the owner's own clauses |
| A generic `set_value<T>(r : &mut Box<T>, v : T)` at `T := Percent` | the parameter's fact is preserved from the input; acceptance further depends on `Box`'s own clauses and the other exit clauses |
| An initially invalid store | encodes no memory, as today |

Controls to add to the existing sets: candidate roots chosen by a branch with
different own clauses; a loan that dies on one branch only; a loan returned on
one branch and settled on the other; nested reborrows; a loop that spans a
live loan; an identity that returns a live reference, with and without a
specification that reads the final value; a returned reference inside an
aggregate; an invalid mutation through a slice; a closure, a function value
and a recursive call; an abstract parameter predicate that is false of some
value; an invalid native result, default and decoded value; a global borrow
followed by another observation of storage and by the resolution of returned
storage; a table update and allocation. No negative may pass because an
assumption removed its run.

## 10. Deferred: conditions that assume more

The plan's "Modular conditions" proposed that the end of a mutation owe only
the levels on its path, and that a boundary prove no clause. Under this note
the second already holds wherever a fact exists: a boundary clause is
discharged by a fact. The first is an optimization of the deep check of a
rebuilt owner and of the conservatism named in section 3. It needs a loan-path
analysis that says which levels a death closes, with documented conservative
cases, and the present death records are not that analysis. It is not part of
the collapse. It is taken up after the tree is back at the reference, as a
change of its own with its own review. Nothing proves that the deep checks
with explicit facts recover the reference's cost or its acceptance: the tree
is compared with the reference as well as with the repaired carriers, and if
a target of the reference needs this refinement, that is recorded as its
blocker and the question is reopened for that target.

## 11. The nominal carrier (N0)

Candidates: **G**, a generated `structure` or `inductive` per declaration,
with a pack and an unpack to its one-layer row and their two round-trip
theorems closed by case analysis and `rfl`; and **T**, the library's fold over
a named table of layouts.

The test, with setup and the full emission and normalization of each unit
measured against the reference, not two chosen leaves:

1. A non-generic struct with an invariant and a nested struct; an enum with
   three variants.
2. A generic struct at two ground rows and at a parameter row, the
   instantiation named once per row, and instantiated through a Given frame
   and a runtime frame.
3. Spelling under reduction: after each normalization pass the solver runs, a
   value's type and its field reads still show the declaration's name; and two
   declarations with the same layout stay distinct.
4. A recursive group: a list-like enum and a mutual pair, under the same
   carrier interface as case 1, with a codec and a decidable equality.
5. An owner with an abstract layout, with its ownership and model operations.
   Neither candidate gives this for free: its interface, its encoding and its
   relation to the model are specified without inventing fields that can be
   inspected.
6. An authored proof that names a field and a variant.
7. Setup within the reference's certification phase.

Expectations, each a test and none a guarantee: G can keep nominal identity
and admissible mutual recursion, subject to positivity; its normalization
stability, recursive codecs and equality, the abstract owner, authored proofs
and setup cost all have to be shown. T is likely to expose rows in case 3 and
cannot describe case 4 as a fold over earlier entries. The structural
consequences of pack and unpack can be library theorems; recursive codecs,
equality and model interpretation have prerequisites of their own.

## 12. What changed since the first revision

Withdrawn, each shown false or inconsistent by review 2 and checked in the
source:

- "A mutable borrow of a resource leaves the old value in memory until the
  death." `Term.globalBorrow` writes the prophecy at once.
- "A prophecy carries no fact." It conflicts with valid memory under that
  denotation, and with a reference returned live. Prophecies are valid by
  their choice, as today.
- "What a death owes" by closed levels, and a join's premises by static
  openness. The borrow analysis unions live loans at a join and its death
  records are conservative per branch; they do not give exact closure. The
  present protocol needs neither.
- "Every reference held in an aggregate already checks its current." A slice
  reference's resolution does not; the representation supplied it.
- "The stack already goes through the plain read and write." The helpers
  exist; the denotation and the solver use raw memory.
- "Every read past a write has a validity side condition." Not across
  distinct resources.

Corrected in the third revision, each from review 3 and read in the source:

- The standalone `.equation` of a combined settlement, and `holderResolves`
  omitting dereferencing candidates: the repair is in the compiler too.
- `.referent` and `Term.returned` check and assume no equation.
- A slice reference's readings are both valid today; its explicit check is
  defensive, and no second hole is claimed.
- A result's validity is unconditional, outside the postcondition's guards.
- Defaults and the removal of satisfiability are exceptions to the bijection,
  with an audit that gates the switch.
- A clause relating two fields is rejected at the first death only when it
  cannot be proved against the sibling's prophecy.

Added: the translation as the one organizing rule; the classification of
former certificate receipts; the slice repair; the mandatory premise bundle of
a join; the `Holds`-preserving transports; the Given frame's predicate; the
work items of storage; what may land before the switch; the deferral of the
path-only obligation.

Updated in the fourth revision, from the settlement/slice audit and user
decisions:

- Off-domain total reads in specifications use an uninterpreted (`opaque`)
  plain fallback backed by `NTy.plainInhabitant` for `[Inhabited]`, exposing
  neither `Holds` nor structural-zero layout equations (`U5`).
- The settlement rule checks both the referent and the rebuilt owner on
  `hasDeref` paths (`.sliceIndex` and aggregates holding references), erasing
  only direct-root `.deref rest` via `Lender.resolve_place_deref` and
  `Lender.resolve_candidate_deref`, with one unconditional `.referent` in
  `holderResolves`.
- All three remaining slice equations — slice borrow/reborrow current checks
  under `hasDeref`, `takeSliceAtValue`'s cleared owner, and slice lender
  provenance for enclosing owners at slice death — land in phase 1 (`U6`).
- `Contract` gains first-class `validArgs` and `validResult` domain predicates
  defaulting to `True`, enforced unconditionally in `Satisfies`,
  `Contract.summary`, and `wp_summary` (`U7`).
