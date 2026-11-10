# Native nominal types

A struct or enum type is to be carried as an atomic identity, a declaration
handle and its type arguments, whose values are those of a native Lean datatype
generated for the declaration. This replaces two representations: the finite
structural layout (`NTy.struct`, `NTy.enum`), which spells a declaration's
fields at every mention and cannot express a recursive declaration, and the
family-keyed carrier of `NTy.nominal`, whose runtime instance is the subtype of
runtime values a unit admits. The design was checked on a standalone prototype
(a generic struct, a nested struct with an invariant, a live loan); this plan
is its integration.

## Principles

- The design is for the verifier's performance: the cost of a type is paid in
  its substitution, its unfolding and the size of the terms that mention it.
- A representation is canonical by construction. No equivalence rule reconciles
  two spellings of one type or one projection.
- A term names what it uses and does not inline it. A declaration's layout, its
  carrier, its dictionaries and its model are ordinary named definitions.
- An instantiation at ground arguments has a fixed meaning. It is computed once,
  bound to a named constant, and reused; no mention repeats the substitution.
- Only a declaration states an invariant of its own. A certified value carries
  its declaration's invariant and its fields'; the current of a live loan is
  plain, its prophecy certified.
- The plain layer does not depend on `Carriers.own`. Replacing `own` changes
  what is certified and leaves every plain type, codec and value the same term.
- Field operations use native constructors and projections through a checked
  one-layer model. A codec relates a value to storage and to the runtime
  semantics, and is no part of a field read, a construction or a match.
- A declaration without a generated carrier is diagnosed. No type is carried as
  a runtime value for want of a native one.

## Design

### Identity

`NTy.nominal source arguments productive` names a declaration at its type
arguments. Substitution visits the arguments only. `productive` is the
declaration's inhabitability at inhabited arguments, as `ntyOf` computes it, so
that `NTy.inhabitable` stays a function of the type.

The one-layer layout of a nominal is `NTy.nominalBody? unit source arguments`:
the declaration's field row, or its variant names and rows, at the arguments,
in which a nested declaration is again a `.nominal`. It is metadata: no carrier
is computed from it.

### Plain basis

`NTy.plainOf F (.nominal s a p)` is `F.nominals.nominal s p (a.plainSorts F)`:
the family's functor for the declaration at the plain types of the arguments. A
generic declaration is thereby one Lean type constructor over plain argument
types, and instantiating a family substitutes the argument types and keeps the
functor.

The functor's operations are the fields of one structure, `NominalFunctor`,
which `PlainFamily` and `Carriers` hold as the single field `nominals` and
`NominalCarriers` extends. A family names its functor and does not spell its
operations: a term over a unit reduces to the family's constructor literal, and
that literal carries one argument for the nominal layer rather than one per
operation and law, so `grind`'s walk over the literal's arguments costs what it
cost before nominals. For the same reason a unit's instance is a named
definition that is not reducible: a reduction of the unit's carriers stops at
the name instead of exposing the instance's own literal.

The operations take the argument's operations as dictionaries, in a list
indexed by the argument types:

| Field | Reads |
|---|---|
| `nominal` | `StructHandle → Bool → List Type → Type` |
| `nominalEncode`, `nominalDecode?` | the arguments' encoders and decoders; they invert, and are tight and loan-free, where the arguments' do |
| `nominalInhabitant` | the arguments' inhabitants, at `productive = true` |
| `nominalDecEq`, `nominalOrder`, `nominalEqual` | the arguments' equality, order and ghost-insensitive equality; the last two agree with the encoding's (`TypedOrder`, `TypedErasure`) |
| `nominalHolds` | the predicate family it certifies against, the argument row, the arguments' invariants |

A generated instance defines each on the native structure. No operation of a
value goes through its encoding.

`NominalFunctor` mentions nothing of the certified layer, so `PlainFamily`
holds it as it stands.

Instantiating a family gives a nominal the same functor at two argument lists,
the induced family's and the outer one's at the substituted row. A value moves
between them by the cast along their equality. At a concrete row the lists are
definitionally equal and the cast reduces to the identity, so a verification
condition sees one spelling.

### Certification

A declaration's own invariant is `Carriers.nominalOwn`, a predicate of its
native plain value at a handle and an argument row.
`NTy.deepHolds (.nominal s a p) x` is `nominalHolds Carriers.nominalOwn s p a … x`,
with the law that it implies `nominalOwn s p a _ x`. `nominalHolds` is an
interpreter of the predicate family it is given, not a type of certificates
fixed when the family was made; a generated one recurses over the native value.
The certified carrier of a nominal is the subtype of its plain type by
`deepHolds`, with `forget` the projection and `ofDeepHolds` the constructor, as
for a struct today.

`Carriers.own`, a predicate of an encoding, serves the structural layout only
and goes with it.

The runtime family reads its functor from `NominalCarriers unit`, whose
`supports` names the declarations it carries natively. A unit's native carriers
are the `nominals` field of its `FunctionCarriers unit` instance, and
`FunctionCarriers.toNominals` reads the field back as the instance: the two are
declared together, one per unit, so a term over a unit takes one instance
argument and not two, and every arity in the solver and the contract generator
is the one it was. The class carries both parts of a unit's carriers, and is to
be renamed accordingly in the parent, where the rename is one commit of its own.

That a value's encoding is one the unit types holds of a supported declaration,
so the agreement theorems take the premise that a type's nominals are supported,
which `ntyOf` establishes of what it gives.

### Model

After `NTy.carrier` and rows exist, a model relates a nominal to its one-layer
layout:

- `StructModel Θ s a p fields`: `packPlain`, `unpackPlain`, their inverses, and
  `holds_iff : deepHolds (.nominal s a p) x ↔ nominalOwn s p a _ x ∧
  HListPlain.deepHolds fields (unpackPlain x)`, with `encode_pack` relating the
  codec to the row codec.
- `EnumModel` likewise over `variantPlain names rows`.

Certified projection, construction, destructuring, plain update and
recertification are derived from these once. `NominalModels unit Θ` supplies the
model of every declaration of the unit at every argument row, generic in `own`.

### Per-unit generation

For a defined unit the verifier generates, per declaration in dependency
order: the native structure or inductive over its type parameters' plain types;
its codec with the inverse, tightness and loan-freedom laws; its `holds`; its
model. It then declares the unit's native carriers, a dispatch on handle
literals, as the `nominals` field of the unit's `FunctionCarriers unit`
instance, which `Carriers.runtime unit` reads: one per-unit instance holds
both parts of the unit's carriers, so a term over the unit takes one instance
argument. A `Skolems` frame carries the same functor: its
transports to the runtime family are the functor at transported arguments.

Each declaration's one-layer layout is a named definition over its type
arguments, generated with its carrier; terms and model statements mention it by
name. For each ground instantiation a unit uses, the generator binds once, in an
instantiation cache keyed by handle and argument row, the constants a
verification condition mentions: the plain and the certified type, the argument
dictionaries, the layout at the arguments, and the model. Each is a definition
whose body is the declaration's generic one applied to the cached constants of
its arguments, so the cache is closed under nesting and no entry spells a
layout. A condition at a frame with type parameters mentions the generic
definitions at the frame's parameters.

A handle the unit does not support is the one-point type at `productive = true`
and empty otherwise, never emitted by `ntyOf`, and excluded by the `TypedAs`
premise that a nominal's flag is the unit's.

### Operations

`Term`, `Proj` and `Pat` gain owner-indexed operations whose owner is a nominal
with a layout witness, denoted through the model. `Compile` produces them;
quotation retains handle and arguments; solver observers that open a descriptor
today (`nativeTypeView`) read the layout metadata and the model equations
instead.

## Stages

Each stage lands with the four suites building and its owning tests green.

1. **Basis and certification.** `PlainFamily`/`Carriers` nominal fields as
   above; every `.nominal` arm of `Types.lean`, `FunctionValues.lean`,
   `Term.lean` and the proofs over them; `TypedNominals` and `RuntimeNominal`
   removed; `NominalCarriers unit` with the instance that
   supports no declaration. Nothing emits `.nominal`; no baseline moves.
2. **Model.** `StructModel`, `EnumModel`, the derived certified operations and
   their laws, `NominalModels`; a hand-written instance for a two-declaration
   fixture with invariants and a live loan, as the prototype's.
3. **Generation.** Native structures, codecs, `holds`, models and the unit
   instance for finite declarations: generic structs, nested declarations,
   enums, ghost slots. Abstract-layout map owners keep the structural layout
   until their model is designed.
4. **Operations.** Owner-indexed `Term`/`Proj`/`Pat` operations, `Compile`,
   quotation, solver observers.
5. **Emission.** `ntyOf` gives `.nominal` for every generated declaration.
   Baselines and `DenotePerformance`, `CompositionPerformance` and the
   framework benchmark are compared against the branch base.
6. **Recursive declarations.** Productivity, strict positivity and uniformity
   of a recursive group; mutually inductive carriers; `holds` and codecs by
   recursion on values; the `size` measure. Other recursive shapes keep their
   diagnostic.

After stage 5 the structural layout serves only what stage 3 left to it; its
removal is decided then.

`supports` and the premise it puts on the agreement theorems
(`NTy.supported`, `NominalCarriers.covers`) exist
because a unit has no generated carriers before stage 3. Once an instance
carries every declaration of its unit, a handle it does not know has the
one-point type whose `.unit` the unit admits, `supports` is constantly true,
and the premise and those laws are removed with stage 5.

## Open

- The model of an abstract-layout map owner.
- Whether a certified projection is a primitive projection or the named
  `Var.selectCertified` family; one spelling is chosen for all producers in
  stage 4.
