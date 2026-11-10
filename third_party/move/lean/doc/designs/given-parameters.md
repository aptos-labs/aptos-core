# Canonical parameters in Given verification

## One abstract sort per parameter

A generic body verifies at one uninterpreted sort per type parameter. Its
`NTy.carrier` and `NTy.plain` APIs retain their separate roles
([`data-invariants.md`](data-invariants.md)), but `GivenFrame`
(`VerexIR/Proofs/Denote/Interpretation.lean`)
chooses the same underlying abstract representation for both parameter roles.
For a parameter, forgetting and certification are identities and well-formedness
is definitionally `True` (`GivenFrame.toCarriers` sets the lent layer's `of` and
`certify` to the identity and its `holds` to `True`, and
`GivenFrame.deepHolds_param` is that equation). The frame keeps the carrier's codec, inhabitants,
equality, and order. Specification reads and read-backs use the same abstract
sort, and specification key comparisons use the carrier's order. The map-view
producer emits identity reads directly for given parameters instead of inserting
concrete read/unread conversions. Runtime read-back and its fallback still come
from the supplied interpretation.

The plain/certified API distinction is separate from invariant obligations.
Allowing an arbitrary isomorphism between two abstract parameter sorts adds
representation freedom that the generic body does not use: it cannot open or
construct a value of the parameter type. The extra representation produces `certify (of value)` round trips and
injectivity obligations in ordinary field observations. Choosing one abstract
representation removes these terms at their producer, without adding inverse-law
normalization rules.

## ADTs keep their certificates

The choice is specific to abstract parameters. A nominal ADT's plain row and
certified carrier remain distinct. `GivenFrame.withDeclarations`
(`VerexIR/Proofs/Denote/GivenOwn.lean`) still installs
the unit's declared ADT clauses as the frame's `own`, without changing the given
sorts, their codecs or their parameter well-formedness, and construction,
mutation completion, loan end,
and hand-over still assert those clauses. A live loan may temporarily violate an
ADT invariant; returning it or passing it to another function must re-establish
the invariant. Nesting an ADT does not erase its certificate obligations. An ADT
use names the unit's declaration descriptor with explicit type arguments
([`native-verification.md`](native-verification.md#nominal-declaration-descriptors)).

## One interpreter, and the concrete export

One interpreter serves both frames. `Interpretation`
(`VerexIR/Proofs/Denote/Interpretation.lean`) is the carrier family together
with the state the denotation runs over; `Interpretation.concrete` is the
runtime family over `Memory unit` and `GivenFrame.interpretation` is a given
frame over a state of its own, and the concrete adapter's certifying step is
definitionally the concrete `Spec.certified`
(`Interpretation.certifying_concrete`). Contract translation takes the same
explicit interpretation, so producers emit canonical values and observations at
that interpretation instead of leaving the solver to recover the distinction
through representation equivalences.

Concrete carrier families are unchanged. Their parameter instantiations can have
different plain and certified representations and real invariants. A generated
Given theorem contains neither a concrete frame nor a hidden transfer assumption.
Its concrete export takes a named `givenTransfer` proposition containing an
explicit relational certificate. That certificate relates concrete arguments,
state, results and errors to the chosen Given interpretation, accounts for the
external operation contracts, and covers permitted starting states. No total
identification with arbitrary raw runtime values is required by the canonical
Given representation.

Source-level regression tests cover generic construction, an invalid intermediate
mutation repaired within a loan, nested generic values, whole-value swapping, and
rejection of invalid packing, loan completion and hand-over. They also require
actual Given proof artifacts and named propositional transfer evidence, and
reject failed verification without retaining a proof artifact.

Generated proof terms reference named definitions instead of copying their
bodies. Each namespace publishes one `givenOwnDeclarations` table, and all
installed Given families, contracts, operation premises, and theorem binders
refer to that table. The declaration remains reducible so computation can select
an ADT clause when a proof requires it; no global unfolding rule expands the table
throughout theorem telescopes.

## Canonical emission

An instantiated call exposes native values in the caller's representation,
including the implicit type annotations of enclosing expressions. Generic source
bodies keep their own parameter frame; after substitution, newly emitted native
operations and plain defaults use the outer family and substituted nominal type.
The normalizer applies this policy before recording an expression as normal,
both when unfolding a call and when exposing a certified invariant fact.
Canonicalizing the initial opaque predicate alone is insufficient: unfolding it
can introduce fresh operations at the generic family.

This conversion preserves semantic indices such as a `Flow`'s family and result
shape. Its native representation arguments are canonicalized coherently, and
operation-prefix conversions are checked by definitional equality. Nominal type
descriptors and declaration tables stay named. A conversion that requires an
unavailable outer instance is deferred; instances opened temporarily inside a
binder cannot escape through the normalization cache.

Constructor indices are rebuilt as a unit: the substituted context row, optional
index type, and contained index must agree. An opaque dependent index defers conversion of the entire operation.
Fresh product and subtype projections use inferred projection applications,
including equations emitted by cases, so clause readers and arithmetic deciders
see the same syntax. Conversion checks for repeated operation prefixes are
cached within one emission scope; unresolved expressions are not cached.

## Which members take the route, and what happens when they cannot

A member is verified through a Given proof when its declaration is generic, is
not verified through a model, belongs to no cycle of calls, and is not being
summarized for its callers (`verifyMember`, `VerexSL/Verify/Members.lean`).
`declareGivenFunctionCore` (`VerexSL/Verify/Given.lean`) then declares
`givenFunction`, `givenTypedContract` and `givenTypedVerified` beside the
member, and every external callable the body reaches is an explicit `Satisfies`
premise at the same carrier family. As on the concrete route, the body is
proved against the implementation view of its contract
(`ContractView.implementation`) and a callee premise states the interface view
a caller assumes (`ContractView.interface`).

Before the route is chosen, the whole interpretation capability is checked
(`givenDataOwnFor`), not only the body's operations: declaration clauses belong
to the unit, so a body that uses no unsupported operation can still require an
ADT interpretation the given frame cannot state. `givenUnsupported?` reports, in
order, a ghost interpretation, an in-body assumption, storage (memory the
contract's frame uses, or any `modifies` clause, whose slots at resource types
only the concrete interpretation states), a recursive or
function-value call, type reflection (a native whose value the translator
computes from the concrete frame, and the aggregator model, which reads that
frame for its element type and fields), calls to callees without a contract, a
behavioral contract, and an authored assertion; `givenDataOwnFor` adds the
data-invariant interpretation of a declaration whose clauses do not translate.
A callee without a declared contract is listed because the Given proof can only
state a callee as a `Satisfies` premise: a contract-less callee is read through
its body, which the given carriers do not interpret, so the premise would say
nothing and the caller could not establish its own clauses. A native, an
intrinsic map's role and a prelude model state their own contract, which a
caller assumes as the Move Prover does, so they are exempt.

**The fallback is concrete verification, with the callee inlined, and it is
logged as information, not as an error** (`VerexSL/Verify/Members.lean`). Only
an explicit capability result selects the concrete path; a translation bug and a
failed proof remain errors. Clauses that did translate are reused by the
emitter.

## The audit of a Given artifact

The no-fallback audit (`requireNativeArtifacts`, `VerexSL/Verify/Members.lean`)
runs at elaboration time over the Given artifacts as well as the concrete ones.
Beside the checks every member passes, it requires that each named transfer
proposition exists and is a parameterless `Prop`; that the Given body theorem
exists and states satisfaction of the shared Given interpreter, its meaning
mentioning `Function.denoteIn`; that no binder of that theorem assumes a
concrete transfer (`Spec.RelationalRefinesAt`, `FunctionRelationalRefines`,
`Contract.RelationalTransfer`, `Contract.RelationalCertificate`, or one of the
unit's own transfer propositions); and that the Given body theorem does not
depend on `compileFunction_agrees` or `compileFunction_least_cycle`, the
concrete compilation axioms that the concrete artifacts may rest on
([`../status/assumptions.md`](../status/assumptions.md)).
