# Data invariants as part of a type

A data invariant is a property of a struct or enum value that is part of what
it means for the value to be of its type, as an integer's range is part of
being a `u64`. This document describes how the verifier carries that property:
the two families of carriers, where the obligation falls, what happens while a
value is lent, what crosses a call boundary, and what the storage ledger entry
then assumes.

The design follows the Move Prover — its paper
(`move-prover/doc/paper21/design.tex`, "Global Invariant Injection") and its
passes (`bytecode-pipeline/src/data_invariant_instrumentation.rs`,
`memory_instrumentation.rs`, `well_formed_instrumentation.rs`) — and departs
from it only where a Lean proof forces a departure. It is designed for the
language, not for any corpus.

A declaration below is cited by its name rather than by a line, with its file
relative to `third_party/move/lean/verex-ir`. The Move Prover's own sources are
under `third_party/move/move-prover`.

## Two kinds, kept apart

A **data invariant** is part of the type of a struct or enum value. It is owed
where a value is made or a mutation of one ends, and holds of every value
anywhere else, like the bound of a `SpecInt`.

A **global invariant** (`invariant` in a module spec) is a property of global
memory. It is owed after a write of memory it reads and assumed at the entry of
functions whose accessed memory overlaps it.

A global invariant may read a stored value and use its data invariant; a data
invariant is never a property of memory. Restating the data invariants of
stored values as a property of all memory, owed at writes, exits and joins,
mixes the two kinds. The global-invariant scheme is not implemented; it is an
open problem in [`../plan/data-and-global-invariants.md`](../plan/data-and-global-invariants.md).

## Where a meta-argument goes

The Move Prover verifies one function at a time and closes its argument with
two facts about Move that it never states in logic: only the defining module
packs a struct, and every function that can invalidate an invariant is in a
verified cluster. In Verex each function's theorem is `∀ arguments initial,
permitted → wp body … initial`, and the unit's run theorems
(`VerexIR/Proofs/Denote/Agreement.lean`) are Lean proofs too, so a
meta-argument has nowhere to live: it is either a premise of the function's
theorem or a hypothesis of the public theorem, in the assumption ledger
([`../status/assumptions.md`](../status/assumptions.md), where
`GlobalsPreserved` and `FailuresIntegral` are).

The obligations stay where the Prover puts them, and a break is caught in the
function that causes it — the modular argument. For data invariants the ledger
entry is the typing of storage, which is already there, because the invariant
is part of the carrier of its type; the denotation of a pack that breaks it is
undefined, which the refinement from the big-step semantics admits and
verification excludes.

## Data invariants

**Carried by the type.** A data invariant is part of the type: the carrier of
`u64` is `SpecInt` with its `fits` certificate (`SpecInt`,
`VerexIR/Proofs/Representation.lean`), a value read from anywhere has its
bounds by type, and the leaf unpacks them. The carrier of a struct or enum
declaration with a data invariant is the subtype of its plain value by the data
invariants at every depth. Two families of carriers, both in
`VerexIR/Proofs/Denote/Types.lean`:

- `NTy.plainOf`, and `NTy.plain` at a family's lent layer: a struct is its row
  (`HListPlainOf`), an enum its row under its name (`variantPlainOf`), a
  reference a pair of plain values, a type parameter the lent carrier the
  family supplies (`ParamLent.carrier`). No certificate anywhere.
- `NTy.carrier`: at a struct or an enum, the subtype of the plain layer by
  `NTy.deepHolds`; at a mutable reference, `referent.plain × referent.carrier`;
  at a type parameter, `Carriers.carrier`; at a function type, the family's
  `functionCarrier`; structural everywhere else.
- `NTy.deepHolds` of a plain value: at a struct or an enum, the declaration's
  own clauses of the value's encoding (`Carriers.own`) and the fields'
  `deepHolds`; through tuples, vectors, tables and ghost slots; of a reference,
  the **prophecy's only**, the current being open while lent; of a type
  parameter, what the family's lent layer holds of its values
  (`ParamPlain.holds`); `True` at a scalar and at a function value.

The certified cases spell the plain layer out (`NTy.plainOf Carriers.toPlain`,
`HListPlainOf Carriers.toPlain`, `variantPlainOf Carriers.toPlain`) rather than
naming the wrapper abbreviations. A subtype stores its carrier type, so every
`.val` read off a certified value carries that spelling into the implicit
arguments of the projections taken from it, and one type read at two spellings
is two terms to a congruence closure that no equation of a leaf relates.

A certified value is a plain value with one certificate for everything inside
it, so a value read from a certified owner at any depth has its invariant by
projection of the owner's (`Var.selectPlain_deepHolds`,
`VerexIR/Proofs/Denote/Term.lean`). The own clauses come from `Carriers.own :
StructHandle → NRow → RuntimeValue → Prop`, a field of the `Carriers` class and
of `FunctionCarriers`, over a value's *encoding*: the class precedes the
carriers, so it cannot speak of rows, and an encoding is what a value keeps
across frames (`Skolems.encode_reframe`,
`VerexIR/Proofs/Denote/Resources.lean`), so the certificate transports with it.
A declaration without an invariant has `own = True` — the field's default — so
every struct is wrapped uniformly and nothing is decided on a symbolic handle;
the wrapper over a trivial predicate is the cost, measured by the cost gate.

**The predicate, and the instance that carries it.** The verifier generates the
predicate so that it is closed before the family it belongs to exists
(`ensureDataOwn`, `VerexSL/Verify/Contracts.lean`; `declareCarrierInstance`,
`VerexSL/Verify/FunctionValues.lean`):

1. a base instance of the unit's function values whose `own` is the default,
   with the theorem that it is trivially true and the inhabitants that follow
   from it (`CarriersNonempty.ofTrivial`);
2. per declaration a named constant holding its clauses, elaborated at the base
   family over the decoded plain row (`Contract.dataOwnTerms`,
   `VerexSL/Contract.lean`);
3. the unit's predicate `dataOwn`, which selects among those constants by the
   declaration's handle and applies `OwnAtArguments`
   (`VerexIR/Proofs/Denote/Resources.lean`): the clauses hold of an encoding
   when they hold of whatever plain row the plain codec decodes it to, at the
   frame the type arguments induce;
4. the instance proper, the base with `own := dataOwn`, where any declaration
   contributes one, and the base itself otherwise.

`OwnAtArguments` applies the predicate to the decoded row directly; it does not
manufacture a certified value in order to read a plain clause, and
`OwnAtArguments.plainEncode` discharges the decoding premise of an encoded
value. Each declaration's clauses stay behind their own constant, which the own
observer opens at the actual arguments rather than every normalization of a
certificate mentioning the predicate. The plain layer of the base family and of
the instance proper is the same, since `NTy.plainOf` does not read `own`, so an
obligation at any frame reduces to the clauses of the reframed plain row.

Every certified operation on a struct or an enum is the plain one on `.val`,
and the codec of a certified type decodes only where `deepHolds` holds of the
plain value it decoded (`Codec.certified`, used by `NTy.codec` at `.struct` and
`.enum`), as `Carriers.runtime` decodes only `Plain` values, with classical
decidability as there; tightness is kept (`certified_tight`).

### Inhabitants, by hypothesis

A certified type may be empty — an unsatisfiable invariant — and the denotation
needs inhabitants: every type parameter's carrier is `Inhabited`
(`Carriers.inhabited`), a clause reading an absent resource reads a default
([`denotation.md`](denotation.md)), a map read of a missing key returns one, a
total variant-field read takes one. The Move Prover has the same need and meets
it silently: Boogie's `select` of an absent key is an arbitrary value that
`assume forall a: WellFormed(mem[a])` declares well-formed, so an unsatisfiable
invariant makes a module's verification vacuous.

Verex says it. `DataInvariantsSatisfiable unit`
(`VerexIR/Proofs/Denote/Types.lean`) asks of every struct and every enum the
unit's declarations give values that it have a value at the public family. It
is a hypothesis of the public theorem beside `GlobalsPreserved` and
`FailuresIntegral` ([`../status/assumptions.md`](../status/assumptions.md)),
threaded as the first binder of the unit's generated artifacts, and the class
`CarriersNonempty` — declared after `NTy.carrier`, so that it may speak of
carriers — is built from it (`CarriersNonempty.ofSatisfiable`). Only the
functions that make inhabitants take it (`NTy.inhabitant`,
`Carriers.instantiate`'s `inhabited`, a literal of parameter type);
`NTy.carrier` does not. A unit whose declarations state no invariant has `own`
at its default, its instance is `CarriersNonempty.ofTrivial`, and it carries no
such hypothesis (`declaresDataInvariants`, `VerexSL/Verify/Members.lean`).

### What a read owes

Nothing. A value read from memory, a parameter, a result, a field or a call's
result has its invariant as a property of the value, by projection of the
certificate of whatever it was read out of. No clause states it, nothing is
re-assumed at a loop head or after a call, nothing propagates at a write, and
no predicate over memory is involved. The one assumption at entry is of a
`&mut` parameter's current, which is plain and which the caller certified at
the hand-over ("A call boundary"). Typed memory `Memory unit`
(`VerexIR/Proofs/Denote/Types.lean`) is a function into certified carriers, so
a typed memory is one whose stored values satisfy their invariants, and a
function that neither packs nor mutates such a value sees nothing of data
invariants.

For a declaration whose invariant the carriers cannot state, a second layer
does state the invariants of stored values over memory, and a function reaching
such a declaration assumes and owes them ("The stored-invariant layer beside
it").

### Where a value is made

`Term.pack` and `Term.variant` denote `certifyingAt site .construction`, which
is `Spec.certified` of the declaration's `deepHolds`
(`VerexIR/Proofs/Denote/Term.lean`; `Spec.certified`,
`VerexIR/Proofs/Spec.lean`: "Create a value which certifies a data invariant.
Creating one is the only place an invariant is owed: nothing checks it at run
time, so the operation is undefined exactly where the invariant fails, and `wp`
turns that into a positive proof obligation at the creation site").

`certifyingAt site kind` carries the site the obligation is reported at and
what the site does, and the solver cuts at it (`wp_certifyingAt`,
`wp_certifyingAt_intro`; `certifyingGoal?` in
`VerexIR/Proofs/Denote/Solve/Invoke.lean`, which names the obligation
`data_invariant_N` and maps the kind to a `Provenance`). There are five kinds
(`CertifyKind`):

| Kind | Where it is produced |
|---|---|
| `construction` | `Term.pack`, `Term.variant` |
| `mutationEnd` | a write into a local's own value (`Proj.setOwner?`) and every owing arm of `Lender.resolve` |
| `exit` | `Mutables.resolve` of a `&mut` parameter, and `Term.returned` |
| `copyOut` | `Term.deref`, and `Term.readPlace` through a dereference |
| `handOver` | `Term.handedOver` |

A certificate-free referent owes nothing anywhere: `NTy.certificateFree`
decides of the type, and the compiler emits `Term.copy` in place of
`Term.deref` and `Term.copyPlace` in place of `Term.readPlace` through a
dereference, while `Mutables.resolve` and `Mutables.assumedAtEntry` skip the
step altogether.

The agreement with the big-step semantics, `compileFunction_agrees`, is
`Spec.Refines`, which relates the outcomes of a start only where the denotation
is defined there: a run that packs a value breaking its invariant, or that ends
a mutable loan over one, keeps its runtime outcomes while the denotation is
undefined at its start, so the refinement asserts nothing of that start, and
`Satisfies` excludes it, so verification owes the invariant in the function
that breaks it (`VerexIR/Proofs/Denote/Agreement.lean`).

### Open while lent, repacked where the loan dies

A value under a mutable loan may break its invariant between writes, as the
Move Prover permits, and it may do so at any depth: `memory_instrumentation.rs`
emits `PackRefDeep` only before a write-back to a local or global root and for
a `&mut` parameter at exit; a write-back into another reference is never
checked. So the current of a `&mut σ` is *plain* — σ with every certificate
inside it removed — and the prophecy is certified:

```
NTy.carrier (.ref σ) = σ.plain × σ.carrier
```

A write through the reference (`Term.mutate`, and `Proj.setOwner?` through a
`.deref` step) acts on the plain current and certifies nothing. The owner has
held the prophecy since the borrow (`Term.borrowPlace`, `Term.globalBorrow` for
a slot), a value of the carrier and so well-formed by type. Where the loan
dies, `Lender.resolve` owes `certifyingAt site .mutationEnd` of the current —
`deepHolds`, every level at once, which is `PackRefDeep` — and only then
assumes `current = NTy.forget prophecy`. The order matters: a violation is
undefined before the equation could prune it, so a run that breaks the
invariant and reaches the loan's death has no outcome, and one that aborts
first leaves an unobservable well-formed prophecy in the owner. A value copied
out of the plain current into an owned position (`let c = *r`, `let c =
r.inner`) is certified at the copy — stricter than the Prover, which lets a
broken value escape that way, and the right place.

### A borrowed path

`&mut s1.s2.s3`, where more than one level carries an invariant: the owner's
value with the prophecy set at the path would need the enclosing levels'
certificates at the borrow, which nothing gives. The prophecy is therefore the
*owner's* — a well-formed `s1'` chosen at the borrow, constrained to agree with
`s1` off the path, the reference's prophecy being its component. At the death
the enclosing levels rebuilt with the current are certified bottom-up (`s3`,
then `s2` with the new `s3`, then `s1`) before the equation, which is what the
structure of `NTy.deepHolds` gives and the order the Prover's `PackRefDeep`
checks; a marker ending several loans reconciles them in decreasing instance
order ([`prophetic-references.md`](prophetic-references.md)). A reborrow `&mut
*b` or `&mut b.f` takes its prophecy from `b`'s prophecy, a certified value, at
its path, and its current from `b`'s current; its death writes back into `b`'s
current, a reference, and owes nothing. Only the write-back into a root is a
cut.

The lender a dying loan writes back to is one of five forms (`Lender`,
`VerexIR/Proofs/Denote/Term.lean`): `slot` (a global borrow, or a `&mut`
parameter at an exit: the current itself is certified and the equation
follows), `place` (the local the borrow is rooted in with the path it was taken
at: the rebuilt owner is certified, then the write-back and the equation are
assumed, and where the path dereferences — a reborrow — nothing is certified
and only the two equations are assumed),
`candidate` (a possible owner: the obligation only, no equation),
`referent` (the obligation of a loan no local lends, no equation) and
`equation` (one prophecy equation, stated beside several candidates).

**A holder several loans name.** A reference joined from borrows of different
owners keeps no record of the owner it came from, so writing its current back
into a lender it need not hold would constrain a foreign owner and, together
with that owner's own resolve, make the state contradictory. The compiler
therefore resolves per holder, not per dying loan (`holderResolves`,
`deathEffect`, `VerexIR/Proofs/Denote/Compile.lean`): every loan fact naming
the holder is a possible origin; the origins are deduplicated by the lender
they resolve to, canonically, by resolved root and the path's steps
(`lenderIdentity`, `Proj.steps?`); a path the comparison cannot read exactly —
one containing a variant's choices — keeps its borrow expression's identity and
is merged with nothing. Where one lender is left, its precise resolve is stated,
write-back and all. Where several are, **each distinct lender contributes its
own obligation** — the referent's invariants for a loan no local lends, the
rebuilt owner for a borrow of a local place, nothing for a reborrow through a
dereference — and one `equation` resolve is stated last, since a borrow
constrains the owner it sets to agree with the reference's prophecy at the path
and that one relation is what every such lender states. There is no
representative loan and no obligation is dropped. An ancestor loan naming the
holder is treated as a possible origin and so may be asked for an obligation it
does not owe: sound, and an over-approximation whose cost is an open problem in
[`../plan/data-and-global-invariants.md`](../plan/data-and-global-invariants.md).

A path's index operands are recorded as the path, not as their values
(`PlaceIndex.slot` names a local, and `PlaceIndex.resolve?` reads it from the
environment), so a `place` lender reads the operand where the loan ends rather
than where the borrow was taken. The two agree only while the index local is
not written, so the compiler refuses a function that may write an index local
while a mutable loan whose place indexes by it may still be resolved
(`writesIndexUnderLoan`, `VerexIR/Proofs/Denote/Compile.lean`, read by
`compileFunctionIn`; the message is "a write to a local a live mutable borrow
uses as an element index"). The borrow analysis decides it and records it per
function in the borrow certificate (`BorrowCertificate.writesIndexUnderLoan`,
`VerexIR/Validation/Validated.lean`; `indexWriteDiagnostics`,
`VerexIR/Validation/Borrowing.lean`), at every write path of a local: an
assignment, a move out of it, a write to its place, a mutable borrow of the
whole local, a destructuring assignment and a `let` binding it. Liveness is the
analysis's own active set at the write: a loan stays active while any of its
holders may be read before being overwritten by what follows — the rest of
every enclosing sequence, the loop body again across the back edge, and the
code after a branch or a loop (`LiveOut`, `liveAfter`). A loop that borrows an
element, writes through it and advances the index after the loan has died is
carried. Carrying the operands as evaluated at the borrow, which would make the
refusal unnecessary, is an open problem in the same file.

### A call boundary

A `&mut σ` passed to a call crosses with its current plain and its prophecy
certified, as it is held. The caller owes `deepHolds` of the current at the
hand-over, one certifying cut per `&mut` argument (`Term.handedOver`); the
callee assumes it of each `&mut` parameter at entry (`Term.assumeCurrent`,
placed by `Mutables.assumedAtEntry`) — the Move Prover's entry-point assumption
(`WellFormed` of every parameter, deep through a reference,
`well_formed_instrumentation.rs`, `memory_instrumentation.rs`) — and owes it
again where its loan on the parameter ends, at its exit (`Mutables.resolve`,
`PackRefDeep` at scope end). The Prover asserts nothing at the hand-over
itself, which leaves its entry-point assumption unjustified where a callee
receives a reference opened by a reborrow that died before the call; the cut
here closes that. Without the entry assumption nothing of a `&mut` parameter is
known, and a callee that does not write through it could not meet its exit
obligation. The assumption is not a `requires` the caller states or sees: it is
implicit in the parameter's type, as an integer's bound is, and discharged by
the caller's cut.

In the concrete denotation a reference whose referent mentions a type parameter
makes the two families meet across a frame: the callee's plain carrier of `T`
must be the caller's *plain* carrier of the argument, while the certified one is
the argument's certified carrier. So `Carriers` carries, per parameter, the lent
layer as well — `Carriers.lent`, a `ParamPlain` with the map that forgets a
certified value's certificates (`of`, with `encode_of`), the invariants its
values satisfy (`holds`, with `holds_of`), and the certification that inverts
the map (`certify`). Its default is the certified side with the identity map and
`holds = True`, so only `Carriers.instantiate` sets it. `NTy.forget` is the map
on whole types. The callee can neither open nor break a value of `T`; it writes
whole certified values into the current, which forget.

### A returned reference

A function whose result is a mutable reference over a referent that carries
certificates certifies the referent's current where it leaves it
(`certifiedResult`, `VerexIR/Proofs/Denote/Compile.lean`, emitting
`Term.returned`, whose denotation is `certifyingAt site .exit`). **Every normal
exit is certified, not only the one that falls off the end**: an explicit
`return` leaves through `Flow.return_`, which `Flow.bind` passes on without
running a continuation, so the certification is stated on the returned value
itself, before the control outcome is built. The slot the reference left is
empty, so no resolution at the exit reaches it, and the caller reads the
reference it receives as one whose invariants hold, exactly as a callee reads a
`&mut` parameter. The case is decided by the compiler over the result shape it
knows, so the denotation keeps no case on it; a result that is not a mutable
reference, and one whose referent is certificate-free, owe nothing.

### Generic declarations

The invariant is stated at the frame the instance's type arguments induce, and
the carrier at a frame is the subtype at that frame. A value moved between
frames keeps its encoding (`Skolems.encode_reframe`), and the invariant is a
property of the encoded value, so the certificate transports.

A generic body is verified at one given sort per type parameter. Only ADTs
declare data invariants, so an uninterpreted parameter has none to state,
assume or establish, and what a parameter's values satisfy is whatever the
family's lent layer says (`ParamPlain.holds`, `True` at the default). An ADT
declared inside the generic module still owes its declared invariant, and
containers retain their structural validity conditions. The verification frame
and the concrete instantiation frame are distinct, and the export of a given
proof to the concrete semantics goes through an explicit relational
certificate: [`given-parameters.md`](given-parameters.md) is that design.

### Naming a value in a specification

A clause cannot name a certificate, so each position a specification names
holds its values at one of the two layers (`VerexSL/Contract/Values.lean`):

- `holdsDeclared` is a syntactic predicate on `NTy`, true at a struct or an
  enum and through tuple, vector, table, reference and ghost, false at a
  function type, at a scalar and at a type parameter. Those are the only types
  whose values carry a certificate a clause would have to name.
- `namedType` takes the plain layer where `holdsDeclared` holds and the carrier
  everywhere else; `namedValueOf` is its counterpart on values, forgetting the
  certificates of a value in a plain position and handing every other value
  over as it is, so one type keeps one spelling.
- `certificateFree` (in the same file) is a *semantic* test on a quoted type,
  `isDefEq (NTy.plain …) (NTy.carrier …)`: whether the two layers are the same
  type at this family. It is not `NTy.certificateFree`
  (`VerexIR/Proofs/Denote/Types.lean`), which is a syntactic predicate that is
  false at a type parameter because a parameter's values satisfy what its
  family lends. Two names, two things.

### The storage ledger entry

Runs keep storage encoding a typed memory (`StorageEncodes`, kept by
`GlobalsPreserved`, `VerexIR/Proofs/Denote/Agreement.lean`, a hypothesis of the
public theorem). Typed memory is into certified carriers, so "typed" includes
the data invariants and a storage whose value breaks one encodes no memory. The
conditions under which the hypothesis holds of a unit are:

- every pack, every loan death and every `&mut` hand-over at a call in every
  unit is checked (this design);
- a native model, and an opaque callee of another unit, that returns or stores a
  value of such a type states the invariant in its contract (the audit of the
  native models is open:
  [`../plan/data-and-global-invariants.md`](../plan/data-and-global-invariants.md));
- no other module packs the type (Move's type system; `friend` does not change
  it);
- the unit passes Move's reference safety, so no slot under a live mutable loan
  is read until the loan dies — a stackless-bytecode pass that the AST export
  the Verex frontend reads stops before, so it is a condition here and not
  inherited.

The entry needs nothing further: the invariant is part of the type, so nothing
is assumed per function, stated per step or looked up in a table.

### Cost shape

The work the layer adds is determined by the term and is proportional to it,
which is what [`performance.md`](performance.md) asks of a walk. A pack, a
mutation's end, a loan's death, a `&mut` hand-over at a call and a returned
reference each add one obligation. A read adds nothing, its invariant being a
projection of the value it was read out of; a write through a reference adds
nothing; joins carry nothing for data invariants, so no arrival is dropped or
split; nothing is tried and nothing is searched for at a leaf. Terms grow with
the code.

### Two decided points

- **A data invariant is never a precondition and never a premise.** A callee's
  reads have their invariants by type, so there is nothing for a caller to
  establish at a call.
- **The Move Prover at a call with a `&mut` argument** asserts nothing at the
  hand-over (`memory_instrumentation.rs` emits `PackRef` at a reference's death
  and `PackRefDeep` at a write-back to a root and at a `&mut` parameter's scope
  end only) and assumes the invariant at the callee's entry. This design
  certifies the current at the hand-over, so the assumption is justified ("A
  call boundary").

## The stored-invariant layer beside it

A second layer states the data invariants of stored values as a predicate over
memory. It is live in the tree beside the carriers, and which layer owes a
declaration's invariant is decided per declaration, once per unit, by
`dataOwnTerms` (`VerexSL/Contract.lean`): a declaration whose invariant reads
the executable unit, a precondition or memory (`invariantReads`) is not carried,
nor is one whose clauses do not translate, and every other declaration with an
invariant is carried and is `Carriers.own`'s.

Where a declaration is carried, the second layer states nothing of it.
`storedInvariantTerm` is handed the carried handles and leaves them out at every
depth (`dataInvariantTerms`); `slotDataInvariantTerms` leaves them out of the
parameter and result boundary; `requiresTerm` states `storedAssumed` only where
some declaration is left over, as the loop frame carries the memory predicate
only then; the data-invariant branch of `owesMemoryInvariants` selects only a
resource whose invariant is not part of the carrier of its type; and
`markedInvariant` (`VerexSL/Verify/Contracts.lean`) keeps the carried
declarations out of the two conditions tables. One thing is emitted either way:
the compiler wraps a pack of any type carrying an invariant in a
`Term.constructed` marker and a mutation's end in a `Term.mutationEnd`
(`ns.carriesInvariant` decides, `VerexIR/Proofs/Denote/Compile.lean`), so at a
carried declaration's site the marker stands in the term with no condition
behind it and checks nothing.

What the second layer consists of, for the declarations it does own:

- `DataInvariants invariant memory` (`VerexIR/Proofs/Denote/Resources.lean`):
  every value in memory satisfies the invariant of its resource type, with
  `DataInvariants.set` for a write; `StoredCase` states one declaration's
  invariant at a resource's own type arguments, moved between frames by
  `Skolems.reframe`.
- `storedInvariantTerm` translates each `key` declaration's invariant once per
  unit, and `requiresTerm` states `storedAssumed` of a function reaching such
  memory; the exit clause and `keepsStoredInvariants` owe it again, and loop
  frames carry the implication (`VerexSL/Contract.lean`,
  `VerexSL/Verify/Contracts.lean`).
- `Term.constructed` and `Term.mutationEnd`, with the steps `constructedAt` and
  `mutationEndAt`, their cut rules `wp_constructedAt` and `wp_mutationEndAt`,
  and the two conditions tables the solver reads the predicate from
  (`constructionConditions`, `mutationConditions`). These are the obligation
  sites of a declaration this layer owns, at the same places the carriers owe
  theirs.
- the solver's `verex_data_invariants`, `verex_stored_cases`,
  `verex_stored_reads`, `storedInvariantsProof?`, `storedInvariantsStep` and
  `proveStoredInvariants`, which prove the memory predicate at leaves, at join
  arrivals and at `exposeNormalEntries`.
- `owesMemoryInvariants`, which selects a specification-less function that
  writes such a resource for verification, and `handlesDataInvariants`, which
  does the same for a function holding a reference whose referent carries one.

An invariant that reads the executable unit is what keeps this layer alive: a
clause stating the behaviour of a stored function value is quantified over
states, and a carrier cannot mention the memory those states are. An invariant
that reads memory is not a data invariant at all but a global invariant in a
data invariant's syntax, and the layer is where it lands meanwhile. Both, and
the removal of the layer and of the empty markers once no declaration selects
them, are open problems in
[`../plan/data-and-global-invariants.md`](../plan/data-and-global-invariants.md).
