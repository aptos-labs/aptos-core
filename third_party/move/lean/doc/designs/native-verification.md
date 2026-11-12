# Native verification: no runtime values in verification

## Rule

Verification is stated and carried out over native values only. The Lean
type of every value a verification condition, a leaf, a contract clause, a
specification function, a lemma, a behavioral predicate or a solver fact
mentions is the value's carrier (`NTy.carrier` at the contract's family,
spelled reduced where the type is concrete: `SpecInt`, `Bool`, `String`,
products for rows, sums for enums, `SpecVector`, `SpecEntries`/`SpecMap`,
`Option` for memory slots).

`RuntimeValue` and its vocabulary (`RuntimeValue.*`, `RuntimeState`,
`Maps.*` over snapshots, `ClosureValue.encode`, `Codec.encode`/`decode?`)
exist in exactly three places:

1. the runtime semantics (`VerexIR.Semantics`, the big-step relation and the
   interpreter), which define what programs do;
2. the codecs (`VerexIR.Proofs.Representation`, `Denote.Types`'s `NTy.codec`
   and `NTy.encode`), which define how carriers are represented at run time;
3. the agreement (`Denote.Agreement`, `Proofs.Meaning`, `Proofs.Invocation`,
   `Proofs.Behavior`'s bridge to the big-step invocation, the definitions of
   `propheticRun` and `closureMeaning`), which proves that the typed
   denotation verified here agrees with the runtime semantics.

Nothing the verifier runs, prints or asks a proof author to reason about
mentions them. There is no option, no encoded fallback, and no construct
that is "typed where decided": a construct is native or it is reported as
unsupported.

A closure value's captures and the requires table are native too: the
function carrier is data generated per link set
([`higher-order-functions.md`](higher-order-functions.md), "Function values
as native data").

## Why

Programs are statically typed, and generic code is checked at a fixed
family of types, so the verifier has no reason to reason about types or
representations. A verifier over encodings pays for them in every leaf:
lifting, decoding and transport machinery in the solver, two spellings of
every value, and gaps wherever a construct was native only where a
decision procedure could tell. Over native values a leaf holds the
carriers and nothing else, and the authored proofs are written in the
specification's own vocabulary.

## What is verified

The statement of a target is native: `Satisfies (propheticMeaning
executable Θ typeInstantiation handle σs shape) (typedContract …)`, over
`Memory unit`, `HList σs` and `shape.carrier` (`Verify/Members.lean`,
`Proofs.Contract`). `Memory unit` stores carriers and is keyed by
`StorageKey` of native addresses. The rule reaches the two layers below
that statement: the translator that assembles `typedContract`, and the
denotation and the solver.

## Constructs and their native forms

| Construct | Native form |
|---|---|
| Clause domain of aggregates | `Domain.native τ`: the carrier `NTy.carrier carriers τ`; the clause domains `integer`, `boolean` and `text` classify how a clause reads a scalar |
| Binders | the carriers themselves; a binder carries its clause value and its native value |
| Integers in carrier positions | the certified `SpecInt.val`, a certified literal `SpecInt.mk`, else the cut `SpecInt.ofInt`; a clause integer stored in a fixed-width field is defined only in range (`bindCuts`) |
| Structs, tuples | rows (`HList`), `Var.select`, `Var.update`; a ghost slot is a row component of type `.ghost σ` |
| Enums | injections, `variantName`, `Which.project?`, native `match`; ordering enums by `orderingChoice`; a variant field read totally takes the field type's inhabitant under another variant, and a field of a type without one (a function type) is bound by a match arm or a `let` of a Boolean clause to what `Choices.select?` holds, universally |
| Vectors | `SpecVector` only: literals, `append`/`push` total under the length bound, `extract`, `set`, element quantifiers over `values` |
| Maps | the owner is a row and the intrinsic roles act on its `SpecEntries` component wherever it sits; a table's typed map is `SpecMap.ofEntries keys values x`, and a map written back into a table reads back as itself where its length fits and its entries read back; `new` is the empty owner |
| Equality and order | carrier equality after ghost erasure (`NTy.equal`, identity types by `holdsContents`); `NTy.order`, defined by recursion on the type and used by `Term.order` and the `.compare` primitive, with one theorem `NTy.order_encode` relating it to the order of encodings for the agreement |
| Memory | reads are `Option.getD` of the carrier at `NTy.storageKey κ k`; `exists` is `isSome`; `Memory unit` is a function into certified carriers |
| Data invariants | part of the carrier of the type: the plain layer `NTy.plain`, the certified `NTy.carrier` as its subtype by `NTy.deepHolds`, and one certifying step `certifyingAt` where a value is made, a mutation ends, a loan dies, a reference is handed over or returned ([`data-invariants.md`](data-invariants.md)) |
| Specification functions | definitions and results over native bundles, per instantiation for generic ones (the `.at_…` scheme); opaque functions are `opaqueSpec` constants over native bundles; a definition's value outside its domain is `opaqueSpec "<f>.outside"` of its resolved type arguments |
| Lemmas | native bundles, instantiated like specification functions; a parameter of native type is bound at the lemma's frame |
| Behavioral predicates | `EnsuresOf`, `AbortsOf`, `ResultOf`, `StateOf`, `RequiresOf` over an invocation meaning (`closureMeaning executable` or a model unit's `invoke`) with native arguments and results; a `.ref` position is an `(entry, final)` pair |
| Stored invariants | `DataInvariants` and `StoredCase` over carriers, the second invariant layer, which states the invariants of the declarations a carrier cannot ([`data-invariants.md`](data-invariants.md), "The stored-invariant layer beside it") and whose removal is open ([`../plan/data-and-global-invariants.md`](../plan/data-and-global-invariants.md)): stated at the resource's own type arguments; a generic frame moves its view of the value there (`Skolems.reframe`), so the invariant holds at every frame by construction |
| Type parameters | a value of a parameter type is native at every instantiation, its native type the argument's; an instance substitutes carriers, never domains |
| Failures | `Failure := ThrowKind × Array Int`, related to the runtime failure by the injective `Failure.encode`; abort codes by `abortCodeMatches` |
| Hash, serialization, signer facts | `opaqueValue` applications, as a clause's call builds them; `bcs::serialize` is a chosen function of which only non-emptiness is known |
| Aggregators | `NativeModel.aggregator`: a frame reads an element through `Skolems.aggregatorValuePlain` and tests it by `Skolems.isAggregatorElement`, which are integers and decided at `u64`/`u128` at any frame (`aggregatorValuePlain_int`) and leave a type parameter unconstrained; the abort tests the value `type_name` denotes |
| Mutable-reference lending in calls | unchanged: `NTy.lend`, `lendArguments`, `argumentsResolve` define what a run is, at the agreement boundary |
| Obligation markers | unchanged; they carry no values |

## Nominal declaration descriptors

A quoted ADT use identifies its declaration and supplies its type arguments.
The quotation layer defines one parameterized native descriptor per declaration
of a defined unit. Nested ADTs refer to their descriptors instead of copying
field trees. Descriptor identity includes the unit constant and the declaration
handle: numeric handles are relocated by linking and do not identify a declaration
across different units. Distinct declarations retain distinct descriptor names
even when their fields coincide, and phantom type arguments remain explicit.
Proof-only augmented units reuse the registered source unit's descriptor scope;
they do not define a second unit or a second set of ADT descriptors.

The validated declaration supplies the descriptor body. Quotation checks an
incoming finite native layout against that declaration before naming it; it does
not discard an inconsistent field row. Parameter substitution operates on the
explicit arguments and preserves the descriptor head. Resolving parameters before
quotation keeps resolution from depending on whether a definition has been opened.
A missing unit scope is an error for ADT quotation; low-level synthetic tests
request raw layouts explicitly. Legacy operations that inspect a field or variant
use one shallow descriptor view and retain the original descriptor in emitted
types. Descriptors are ordinary definitions. The kernel checks them against the
structural backend, while routine reducible normalization retains their names.
An observer opens one declaration at the operation boundary when constructing its
existing projection or certificate proof. No global simplification rule unfolds
the descriptors, and no additional family of equations equates separately
generated layouts.

Descriptors separate declaration identity from its current implementation. The
initial backend still uses the finite structural `NTy` layout, its native products
and variants, and its existing codec proofs. Recursive declarations require a
backend with nominal carrier families or generated mutual inductives and proofs
for their codecs and invariant boundaries. A cyclic definition cannot be represented
by recursively expanding this finite backend, nor does naming that expansion make
it recursive. Source-level nominal references already provide the finite graph
needed for that later backend. Mutually recursive declarations must be bound as
a group; their codecs and recursive invariant predicates must recurse over
finite values, rather than expand the type graph. Carrier generation must
distinguish recursive value structure from reference and function handles,
and check positivity before generating inductives. The Move profile continues
to reject recursive Move types.

This representation change does not move invariant obligations onto generic
parameters. Only the declared ADT boundary introduces its own invariant, and the
existing construction, mutation-completion, and loan-completion checks remain.

Certified struct-field access uses one named operation, `Var.selectCertified`.
Its definition is uniform across type instantiations; choosing a different
implementation for certificate-free fields would break that conversion. Table
and vector observations open the operation only at the field boundary, then
specialize the field's family through checked conversion. The owner's invariant
and its proof remain behind their names. Existing table and vector laws apply
to that canonical field operation.

Field quotation also retains the owner's descriptor in `Term.fieldAt`. Its
denotation evaluates the owner at that same type index before selecting the
field. This keeps a global field read and an existence test on the same nominal
resource key; opening a structurally indexed field constructor first would
reconstruct the owner's full layout in the recursive read. Construction,
destructuring, and variant operations likewise retain the owner descriptor in
`Term.packAt`, `letFieldsAt`, `variantAt`, `isVariantAt`, and `payloadAt`. Their
denotations use that owner index when reading or writing storage; observing an
operation never reconstructs a second resource type from its field layout.

A carrier family also stores its canonical plain reader together with a proof
that it agrees with the existing total codec interpretation. Instantiation
selects the concrete argument's reader directly. This preserves decoding for
invalid plain values while avoiding a second reader assembled from codec
composition. Recovering a plain table from its typed reading still requires
the table's actual `deepHolds` evidence; the total decoder is not assumed
injective outside that boundary. A certified argument and a plain argument use
that same reader at construction; the existing forget/read law accounts for the
certificate. Named-family inhabitants use one checked projection spelling without
replacing an independent choice of inhabitants.

Given declaration tables are ordinary named definitions as well. Routine
normalization retains the table application in dependent binder types, rather than
expanding the entire declaration registry. A producer that obtains an own-invariant
proof opens only its selected declaration clause. Observed maps of projected table
fields retain forward read equations; naming a map never reconstructs its owner
from that reading.

## Enforcement

A native-only check reports an error naming the first offending subterm:

- on the contract `buildContract` assembles, for every function
  (`reportRuntimeValues`, `VerexIR/Proofs/Denote/NativeOnly.lean`);
- on the goal and hypotheses of every automatic leaf when the solver first
  enters it, and on a prepared leaf and each case of its split
  (`verex_check_native`);
- on an authored proof where an identifier it wrote elaborated to a
  runtime constant, read from the info trees of its elaboration
  (`reportAuthoredRuntimeNames`).

The check looks for the constant `VerexIR.RuntimeValue` and every
declaration under it. It is not an option: a violation is a verification
error.

## Solver

One normal form over carriers: the equations of `NTy.equal`, `NTy.unread`,
`NTy.order`, `SpecInt.val_ofInt` under its bounds, `BitOp.eval_*`,
`Var.update`, and a map's definition in the context rewritten into its
reads (`verex_subst_maps`). Typed maps are named by `verex_prepare typed`
(`verex_name_maps`); position facts are `typedMapPositionFacts`. Cuts
meet a named local through the chain of names that defines it
(`cutNamedValue`).

An equation with a pair on either side, or at a tuple's type whatever its
sides, is stated at its components, as `NTy.equal` compares rows
(`pairInjectionRows`, `Prod.ext_iff`), where its type spells a row (a product,
`HList`, a tuple's carrier; never by reducing the type), and a row of one value's reads is that
value (`rowOfReads`, by structure eta, with a unit end only where the value's
own rest is a unit). A pattern equated to an opaque read therefore fixes its
locals to that read's projections, which substitution takes into every other
fact. A local the other side does not mention stays whole, for its
substitution, and so does a struct's value compared whole with no side
constructed. A run's observation, an equation with a side `result_of` or
`state_of`, stays whole: the run deciders read it as stated, and what they derive
of a local decomposes. A rewrite through such an equation of a term holding the opaque
value whole is given up: `f(e)` no longer meets `f((a, b))` where `e = (a, b)`.

Variant splits: the walk splits only the variants its goal tests, and an
obligation a hypothesis states closes before it is split. A constructor's
tests compute by definition, so a split's cases are evaluated in place
(`reduceVariantComputations`) and decided (`variant_decision`). Where a
leaf's observations leave one case of a local, that case replaces the
leaf (`verex_determined_variants`). A leaf's automatic attempt ends at its
prepared form; each prepared goal is an attempt of its own within the leaf
budget (`verex_prepared_case`): the prepared deciders, else a split on the
first local it tests whose cases are left open as attempts, else the
fallbacks. Linear arithmetic reads the facts it needs no case split for
first (`verex_omega_unsplit`), then the whole context within a
speculative attempt's allowance (`verex_linear`).

Behaviors: rules are applied by binder role; result reads are rewritten by
all run facts at once, quantified over their default; a call's
observations (`result_of … = r` at every default, `¬ aborts_of`) are
recognized at the native predicates' arities. `requires_of` holds of any
closure where no table entry states a precondition
(`verex_requires_everywhere`).

Authored proofs and the names they use are native: no proof file spells a
runtime value or an encoding, and a module's map lemmas, named from its
specification functions, are the typed maps' lemmas (`ensureMapLemmas`,
`VerexIR.SpecMap.*`).

## Specialized contracts

A generic call's callee contract is stated at the frame the call induces
(`Skolems.instantiate θ Θ`), the caller's clauses at its own frame, so
their specification functions would never meet. For each generic callee
and each type-argument row `θ` a caller uses, through the contract its
theorem, interface or self hypothesis states (`specializationCertificates`,
`Verify/Members.lean`; `ensureSpecialization`, `Verify/Contracts.lean`):

- SC1. The specialized contract `f.at_θ.typedContract`: the generic
  contract at the frame `θ` induces, read in the caller's view
  (`Contract.ofSkolem θ`), each condition normalized once, alone. On native
  values the conversion at a type parameter is the identity on carriers,
  so the normalization is definitional. Retranslating the callee's clauses
  at the caller's frame was tried and dropped: its equivalence needs a
  complete commuting normal form of every operation indexed by type data.
- SC2. The transport of each recursive definition the generic contract
  applies (`ensureDefinitionTransport`, `Contract/Clauses.lean`):
  `g.at_σ.spec unit (instantiate θ Θ) b = g.at_(σ.subst θ).spec unit Θ b`,
  by well-founded induction on the source's relation, the definitions the
  bodies apply transported first.
- SC3. The equivalence `f.at_θ.typedContract.equivalent`: the
  normalization's equations read backwards, a declaration checked by the
  kernel once per module, contract and `θ`.
- SC4. The call rule: `wp_specialized` reads the call at the caller's
  types and `satisfies_specialized` gives the callee's satisfaction of the
  specialized contract (`Denote/Closures.lean`); the solver finds the
  equivalence by the generic contract and `θ` (`specializedCall?`) and
  goes on with the ordinary call rules. Callers never verify a callee
  again; the specialized contract is a corollary of the generic theorem.
- SC5. The specialized contract is spelled as a direct contract at the
  caller's types: a quantifier over a type argument's integers binds an
  integer with the type's membership (`forallCertified`), and a type
  parameter's order is the caller's order at the argument
  (`respellInducedFamily`, `NTy.readOrder`). An operation at the induced
  frame, a field's selection or a variant's projection included, is spelled
  at the caller's frame with the types and the indices it takes (`Var`,
  `Which`, `Choices`) substituted (`respellOperation?`), where the two
  convert; the canonical family pass of a leaf does the same
  (`canonicalFamilies`), so that a step's expression over a generic
  callee's result spells the callee's contract as the leaf states it. An
  operation converts only where it reduces on its indices alone. A
  variant's projection does not: it matches on the choice and the value
  together, which keeps it an atom where the value is unknown, as the
  automatic normal form needs. A step proof's leaf respells it by an
  equation proved case by case on the value (`whichProjectFamily`); the
  automatic normal form keeps the callee's spelling.

## Not changed

The runtime semantics, the codecs and the agreement theorems. They are the
reason runtime values exist, and the verifier's claim about a function is
the agreement theorem applied to its native contract. The levers to
measure again on native leaves are in
[`performance.md`](performance.md); status is in
[`roadmap.md`](../plan/roadmap.md), section 1.
