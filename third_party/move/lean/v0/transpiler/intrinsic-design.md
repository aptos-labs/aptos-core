# Generic intrinsic-map design

Status: **profile graph/signature validation complete; semantic interpretation remains suspended**.

Schema v3, the compiler-v2 exporter, Lean decoder, and reduced six-type source
fixtures preserve the resolved intrinsic declarations and all 158 bindings.
No semantic intrinsic implementation is enabled. The transpiler returns a
fatal error for a module containing an intrinsic declaration, and authored
Leaner intrinsic attributes are rejected during module expansion. This is
deliberate: partial map semantics must not elaborate as ordinary structures or
unconstrained specification functions.

> **Target-architecture note.** The generic map carrier and role
> interpretations below remain design input, but the former attribute-driven
> prototype has been removed. The independent
> [unified LIR project](../../designs/lir-design.md) moves intrinsic declarations,
> graph construction, required/optional-role checks, duplicate checks, target
> signature checks, diagnostics, and reporting into the Move semantic profile
> of `LeanerIR`. That validation boundary now exists and is exercised by the
> six-owner corpus; E16 stays explicitly rejected until the generic semantic
> interpretation exists. The LIR project is not itself an E16 deliverable.

## Work required before resuming E16

The rejection may be removed only after all of these are implemented:

1. First-class intrinsic owner/binding/location nodes in raw and validated
   LIR, populated identically by Move and Leaner frontends.
2. One Move-profile role schema covering required and optional roles,
   dependencies, duplicates/shared targets, target kind/module, cycles, and
   exact physical signatures, with located diagnostics.
3. A generic physical-to-logical map view whose laws are defined once and do
   not specialize on `Table`, `SmartTable`, `OrderedMap`, or another Aptos
   name.
4. Interpretation of every transported core specification role without
   leaving mapped functions as unconstrained `spec opaque` declarations.
5. Executable summaries and abort conditions for constructors, lookup,
   mutation, deletion, borrowing, bulk operations, and destruction.
6. Ordered-map and iterator observations, rank/key relationships, version
   validity, and their source axioms.
7. Verification lowering, Move/Leaner source emission, and capability reports
   driven solely by validated LIR rather than frontend attributes.
8. Six-type corpus gates proving frontend agreement, exact rejection of
   unknown roles, source round trips, and preservation of executable and
   specification semantics.

## Problem statement

Move's `pragma intrinsic = map` does not declare a new concrete map semantics
for every annotated structure.  It says that a physical Move structure is to
be viewed through the prover's one abstract map model, and maps semantic roles
of that model onto functions in the structure's module.

The six current source fixtures contain 158 binding **occurrences**:

- 95 bindings to executable Move functions;
- 63 bindings to specification functions.

Those numbers repeat the same roles across `Table`, `TableWithLength`,
`SimpleMap`, `SmartTable`, `OrderedMap`, and `BigOrderedMap`.  They are not 158
different semantic entities.  Across the six types there are 62 distinct role
names: 37 executable roles and 25 specification roles.  Even those 62 roles
mostly compose a much smaller map algebra: empty, membership, lookup, update,
deletion, length, ordering, and iterator validity.

E16 must therefore add one generic model and instantiate it from the XAST
mapping.  It must not generate a hand-written model or a new collection of
axioms for each concrete map type.

The model and interpretation belong to the Move profile of `LeanerIR`, not to
the Move-to-Leaner transpiler or Leaner attribute elaboration. During the
suspension, the transpiler only decodes the transported declaration and then
rejects its owning module.

## Constraints

1. The physical Move structure and executable bodies remain unchanged.  The
   generated Leaner source must still compile to the original Move layout and
   code.
2. Specification expressions see an abstract map, not the structure's
   physical fields.  This matches the Move Prover, which erases intrinsic-map
   fields from the verification representation.
3. The semantic laws and role interpretations are declared once in the Move
   LIR profile, independently of Aptos module names and of the transpiler.
4. The XAST role mapping is authoritative.  The consumer must not rediscover
   mappings from source text or assume that `new`, `add`, or `spec_get` have
   conventional names.
5. Adding another `intrinsic = map` structure requires no semantic code.  A
   validated XAST mapping is sufficient to instantiate the generic model.
6. Unknown future roles fail with a precise diagnostic.  They are not silently
   rendered as opaque functions.
7. Validated first-class LIR is the stable semantic interface. Surface
   attributes are only a possible Leaner source encoding emitted after the
   graph has been checked; the transpiler does not own a parallel role table.

## Architecture

```text
Move/XAST or Leaner intrinsic declaration
               |
               v
     raw first-class LIR owner/bindings
               |
               v
       Move-profile LIR validation
               |
       +-------+--------+
       |                |
       v                v
source/executable    logical specification terms and contracts
backends             over one generic map carrier
```

There are three layers.

### 1. Generic logical carrier

`Move` provides one abstract carrier, tentatively:

```lean
namespace Move.Intrinsic

opaque Map (K V : Type) : Type

namespace Map

opaque empty : Map K V
opaque contains : Map K V → K → Prop
opaque get [Inhabited V] : Map K V → K → V
opaque set : Map K V → K → V → Map K V
opaque erase : Map K V → K → Map K V
opaque length : Map K V → Int

end Map
end Move.Intrinsic
```

The exact namespace is an implementation detail.  The important point is that
the type is generic in `K` and `V` and independent of every Aptos structure.
`get` is intentionally unspecified when the key is absent, matching MSL.

The core laws are stated once beside this carrier:

- `empty` contains no key and has length zero;
- `set m k v` contains `k` and gets `v` at `k`;
- `set` preserves all other keys and values;
- `erase` removes exactly its selected key;
- `length` changes iff membership changes;
- extensional maps are equal.

This is an abstract mathematical model, not a runtime container.  An
axiomatic carrier is preferred over a `HashMap` or association list because
Move type parameters have structural equality but do not carry Lean
`Hashable` or `DecidableEq` dictionaries at runtime.  The trust is centralized
in one small model, like the existing hash and BCS intrinsic models.

The ordered extension adds generic `keyAt` and `rank` observations and their
inverse/membership laws.  The iterator extension adds an abstract validity
version.  Structural updates change the version; value-only replacement
preserves it.  These remain extensions of the same carrier, not separate
models for `OrderedMap` and `BigOrderedMap`.

### 2. Mechanical physical-to-logical adapter

After LIR validation, the Leaner source backend may keep the ordinary physical
declaration for each intrinsic structure `M<K, V>` and encode the validated
owner metadata as a source attribute:

```lean
@[intrinsic_map]
struct M (K V) where
  -- unchanged physical fields
```

When the generated source is read again, the Leaner frontend reconstructs the
same LIR declaration.  The Lean verification backend then supplies one
Lean-only projection:

```lean
opaque M.intrinsicView : M K V → Move.Intrinsic.Map K V

instance : ModelDomain (M K V) (Move.Intrinsic.Map K V) where
  project := M.intrinsicView
```

This adapter is generated uniformly from validated LIR; it contains no
operation semantics and no per-type laws.  Move source export keeps the
physical `struct M` as the exported type.

The projection and its `ModelDomain` instance are the only adapter generated
per intrinsic type.  In particular, the transpiler must not emit six copies
of the generic map axioms.

### 3. Declaration attributes and role interpretation

After validation, the Leaner source backend may invert the role-to-target
lists and decorate each target declaration.  The owning intrinsic type is an
explicit attribute argument because the function name alone is not the
semantic identity:

```lean
@[map_new (M)]
fun new {K V} ...

@[map_spec_new (M)]
spec opaque spec_new {K V} : M K V

@[map_spec_get (M)]
spec opaque spec_get {K V} (m : M K V) (k : K) : V
```

The executable `new` role is `map_new`; `map_spec_new` belongs on the mapped
specification function.  The LIR validator must reject assigning incompatible
roles to one target, sharing one target across different intrinsic types, or
binding a target outside the type's own module, regardless of which frontend
produced the declaration.

`spec fun` and `spec opaque` now accept the same leading source-attribute
grammar as `struct` and `fun`.  While E16 is suspended, `intrinsic_map` and
`map_*` attributes are rejected rather than interpreted.  Once enabled, a
Leaner frontend must reconstruct first-class LIR bindings from them; they are
semantic metadata, not ordinary Move bytecode attributes.

The Move-profile LIR validator normalizes role names to one closed `MapRole`
vocabulary.  This is the language-level role registry, not a table of Aptos
special cases. A binding is represented conceptually as:

```lean
structure MapBinding where
  role : MapRole
  target : QualifiedName
```

Roles then select generic operations or generic source specifications.  For
example:

| XAST role | Generic meaning |
|---|---|
| `map_spec_new` | `Map.empty` |
| `map_spec_has_key` | `Map.contains` |
| `map_spec_get` | `Map.get` |
| `map_spec_set` | `Map.set` |
| `map_spec_del` | `Map.erase` |
| `map_spec_len` | `Map.length` |
| `map_new` | return a physical value whose view is `Map.empty` |
| `map_has_key` | return `Map.contains` of the input view |
| `map_add_no_override` | abort on membership; final view is `Map.set` |
| `map_del_must_exist` | abort on absence; final view is `Map.erase` |
| `map_borrow_mut` | return a value lens; the final view sets that key to the returned loan's final value |

The remaining executable roles are compositions of the same primitives:
bulk insertion, conversion to vectors, ordered endpoints, and iterator
operations.  Their formulas live once in a generic source-spec library.  A
concrete binding only chooses which Move function receives which formula.

Abort-spec roles do not create additional map semantics.  They either name
the generic abort predicate for an executable role or constrain a deliberately
abstract side condition such as degree validation.  If a source supplies an
abort-role binding, calls to it are rewritten to the same predicate used by
the executable role's generic source spec.

## Specification lowering

Mapped specification functions are aliases for generic operations, not new
uninterpreted functions.  After E16 resumes, the Leaner source backend may
print their source-shaped declarations and role attributes.  Re-import
reconstructs LIR, whose verification backend interprets the declarations with
logical signatures and bodies. Conceptually:

```text
@[map_spec_get (M)]
spec opaque spec_get (m : M K V) (k : K) : V
    => Move.Intrinsic.Map.get (m↑) k

@[map_spec_set (M)]
spec opaque spec_set (m : M K V) (k : K) (v : V) : M K V
    => Move.Intrinsic.Map.set (m↑) k v
```

This requires a small generalization of Leaner Move's specification-function
registry.  It currently records only direct integer argument/result
projection.  Intrinsic spec functions additionally record which arguments
are projected with `ModelDomain` and whether their logical result is the
model carrier.  The ordinary specification-expression rewriter then applies
those projections at every call.  No intrinsic-aware typed-expression mode
is added to `Transpiler.Print`.

Other occurrences of an intrinsic value in specifications are projected at
the logical boundary by the same Leaner Move machinery:

- parameters and function results use `value↑`;
- `old(value)` projects the anchored value;
- equality of intrinsic values compares their views;
- pack, unpack, or selection of physical fields in a specification is
  rejected, as it is by the Move Prover.

Source axioms that mention mapped spec functions automatically become axioms
about the generic carrier after this rewriting.  No source axiom is copied or
specialized per map type beyond ordinary type instantiation.

## Executable-function summaries

A mapped executable function keeps its body for compilation, but verification
uses the generic role source spec.  The generated contract is opaque and is
not verified against the physical implementation; that is the same trust
boundary as the Move Prover's native intrinsic backend.

The implementation defines the source specifications in `Move` as generic
combinators.  Leaner Move's source-spec derivation consults the function's role
attributes and instantiates the appropriate combinator.  It does not inspect
the physical body for an intrinsic summary, and the transpiler does not
construct 37 ad-hoc contract strings.  A small declarative role schema in
`Move` records, for each executable role:

- its generic source-spec combinator;
- the positions of map, key, and value arguments;
- the associated abort role, if any;
- whether an update is structural or value-only;
- the logical shape of its result, including returned references.

This schema is exhaustive over `MapRole` and shared by every concrete type.

Today the `module` expander registers a function's open-ended attributes after
it runs `#derive_move_source_spec` and `#derive_move_spec_function`.  The
registration must move before those derivations so the generic derivation
commands can see intrinsic roles.  This ordering fix benefits any future
attribute-driven source semantics and belongs entirely in Leaner Move.

## Ordered and iterator operations

Ordering is part of the generic map view, not a reason to create an
`OrderedMap` specialization.  `keyAt`, `rank`, front/back, predecessor, and
successor share one abstract enumeration whose laws connect it to membership
and length.

Iterator validity is also generic.  The carrier exposes an abstract version,
while the concrete iterator value exposes a generated, opaque captured
version.  The mapped validity predicates compare those versions.  Two
iterator role names (`map_spec_iter_valid` and
`map_spec_leaf_iter_valid`) select different concrete iterator carriers but
the same version discipline.

The leaf offset is an abstract position tied to `keyAt`/`rank` by the source's
own axioms.  It does not require a `BigOrderedMap`-specific model.

## Validation and diagnostics

Compiler-v2 currently resolves mappings for XAST transport, but its result is
only an import hint. The target Move-profile LIR validator rechecks:

- the intrinsic kind is `map`;
- the structure has the generic key/value shape required by the map model;
- every emitted role attribute is known to `MapRole`;
- roles required by another bound role are present (for example, mutation
  through a returned value requires lookup and update semantics);
- no target is assigned incompatible roles;
- each target has one of the exact physical signatures declared for its role.

A missing optional role merely leaves that operation unavailable. A missing
semantic dependency or unknown role makes the affected module unsupported
with an exact LIR diagnostic. Falling back to an opaque uninterpreted function
would silently lose the intrinsic contract and is not allowed. Until that
semantic interpretation exists, the owning module is rejected before
printing even after validation succeeds.

## Implementation sequence

1. **Done and retained.** XAST schema/export/decoding transports validated
   owner and binding data for all six source fixtures.
2. **Done and retained.** Leading attributes on `spec fun`/`spec opaque` are a
   general Leaner syntax capability; intrinsic attributes themselves are
   rejected while suspended.
3. **Done.** First-class LIR intrinsic declarations, shared graph
   integrity, and the closed 62-role Move registry with owner-shape,
   required-role, dependency, and exact physical-signature validation are
   implemented and pass the six-type, 158-binding source corpus.
4. **Suspended.** Add the generic carrier/view and interpret the core mapped
   spec operations (`new`, `has_key`, `get`, `set`, `del`, `len`).
5. **Open.** Bind core executable summaries (`new`, length/empty/membership,
   add/upsert/delete, borrow/borrow_mut, destroy-empty) and abort roles.
6. **Open.** Add bulk, ordered, and iterator role families.
7. **Open.** Enable the full six-type LIR/frontend/source-round-trip gate.

Each step must leave unknown or not-yet-implemented roles explicitly reported;
partial implementation must not claim E16 complete.

## Tests and acceptance criteria

- The existing source-fixture test verifies all six types and all 158 binding
  occurrences, then verifies that every owning module fails with the explicit
  suspension error and emits no Leaner source.
- Authored `intrinsic_map` and `map_*` attributes fail with the same suspension
  policy instead of becoming bytecode metadata.
- After resumption, generated output may use `@[intrinsic_map]` and
  owner-qualified role attributes only as an encoding of already validated
  LIR.
- A Leaner Move vocabulary test verifies that all 62 distinct role attributes
  in those fixtures normalize successfully and that dependency errors are
  deterministic.
- Generic carrier laws are tested once, outside any Aptos namespace.
- Leaner expansion contains exactly one view adapter per intrinsic type and no
  copied map-law axiom blocks.
- Calls through differently named bindings (`SimpleMap::create` versus
  `Table::new`) lower to the same generic operation.
- The transpiler contains no intrinsic contract templates and performs no
  intrinsic-specific expression rewriting.
- Core contracts for all six types elaborate against `Move`.
- Ordered and iterator fixtures exercise `keyAt`/`rank` and version validity
  without referring to the concrete representation.
- After resumption, the transpilation report identifies the validated generic
  model and reports unsupported roles from LIR capability data. During
  suspension, the module-level fatal error is the only report.
- No mapped specification function remains as an unconstrained `spec opaque`
  declaration.

E16 is complete only when every transported role used by the Aptos framework
is either interpreted by this generic model or rejected as an explicit,
documented language exclusion.
