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

Two boundaries still hold runtime values: the captures of a closure value
and the entries of the requires table. Their replacement is
[`higher-order-functions.md`](higher-order-functions.md), "Function values
as native data".

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
| Memory | reads are `Option.getD` of the carrier at `NTy.storageKey κ k`; `exists` is `isSome`; `MemoryInvariants` over carriers |
| Specification functions | definitions and results over native bundles, per instantiation for generic ones (the `.at_…` scheme); opaque functions are `opaqueSpec` constants over native bundles; a definition's value outside its domain is `opaqueSpec "<f>.outside"` of its resolved type arguments |
| Lemmas | native bundles, instantiated like specification functions; a parameter of native type is bound at the lemma's frame |
| Behavioral predicates | `EnsuresOf`, `AbortsOf`, `ResultOf`, `StateOf`, `RequiresOf` over an invocation meaning (`closureMeaning executable` or a model unit's `invoke`) with native arguments and results; a `.ref` position is an `(entry, final)` pair |
| Stored invariants | `StoredCase`: stated at the resource's own type arguments; a generic frame moves its view of the value there (`Skolems.reframe`), so the invariant holds at every frame by construction |
| Type parameters | a value of a parameter type is native at every instantiation, its native type the argument's; an instance substitutes carriers, never domains |
| Failures | `Failure := ThrowKind × Array Int`, related to the runtime failure by the injective `Failure.encode`; abort codes by `abortCodeMatches` |
| Hash, serialization, signer facts | `opaqueValue` applications, as a clause's call builds them; `bcs::serialize` is a chosen function of which only non-emptiness is known |
| Mutable-reference lending in calls | unchanged: `NTy.lend`, `lendArguments`, `argumentsResolve` define what a run is, at the agreement boundary |
| Obligation markers | unchanged; they carry no values |

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
