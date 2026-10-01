# Relocatable namespaces

Status: R1 and R2 implemented (`LeanerIR.Relocate`, `LeanerIR.Validation.Link`,
`LeanerLang.Modules`); R3 and R4 open. Open work is listed in
[`roadmap.md`](roadmap.md), section 2.

A validated LIR namespace should behave like a Lean `.olean`: produced and
checked once, then included in any unit that uses it, with its validation
results reused rather than recomputed, and eventually its theorems.
Today a namespace cannot be moved between units: its IDs index tables the
whole unit shares, and runtime and proof identities are unit-relative
indices. This design makes a namespace's content independent of the unit
that contains it.

## Why

A Move module is verified by inlining the bodies of the modules it uses,
as the Move Prover does. A module therefore needs the LIR of its
dependencies. With unit-relative identities every importer must renumber
each dependency, re-validate it, re-quote it, and re-derive every
certificate about it: cost that grows with dependency size per importer,
quadratic over a package. Move module dependencies form a DAG, so
everything established about a module is independent of its importers;
the representation should let that be reused.

## Model

A validated namespace is stored and exchanged as an object that a unit links
by relocation, as a linker relocates object code:

1. **Relocatable namespaces.** A relocatable namespace carries its own
   tables (files, locations, origins, alignments, lifetimes, types, names,
   and namespace references) holding exactly the entries it uses, with its
   IDs indexing them. Entry 0 of its namespace references is itself; every
   other entry names another namespace by path (`NamespaceRef`, the alias
   only a spelling). It carries its validation results with it: the
   structurization witnesses and the initialization and borrow
   certificates of its functions.
2. **Relocation.** One operation moves a namespace from one set of tables
   into another, interning every entry it uses and renumbering its IDs.
   Relocating into empty tables extracts a namespace from a unit;
   relocating into a unit's tables links it. The traversal is a `Remap`
   class whose instances are derived for every LIR type, so no ID field is
   missed; a round trip (extract, link, print) is its check.
3. **Linking without re-validation.** Move module dependencies form a DAG,
   so what validation established about a namespace does not depend on
   the units that include it. A unit links validated namespaces and checks
   only the boundary: each reference resolves to a namespace in the unit,
   and the declarations a namespace copied from another (the signatures it
   was lowered against) equal that namespace's exports. The unit-wide
   parts of a validated unit (the resolution index, counts) are rebuilt,
   in time linear in the unit.
4. **Downstream code unchanged.** Validation, the semantics, the printer,
   and the proofs keep working on unit-numbered IDs.

The LeanerLang registry holds each module's validated unit, the module at
namespace 0 with the namespaces it reaches linked in. A `leaner module` that
uses others (any prefix of one of its paths naming a registered module) is
lowered against their interfaces, then links their namespaces, extracted
from their own units, in place of those interfaces. LIR refers to another
namespace's declarations by name, so the interface need not list them in
the order of the namespace that replaces it. The exchange format can carry
relocatable namespaces the same way, so a frontend can ship a precompiled
package.

Verification names a function of the unit by a key: its name in the unit's
module, its path-qualified name in another namespace. A callee of another
module is inlined like a module's own; one used through its contract
(opaque) is verified in the importing unit as well, until R3 lets its own
theorem carry over. A clause authored in another module's file is reported
at its position in that file: each obligation marker names its file.

## Milestones

Each milestone keeps the test matrix green.

- **R1: relocation (done).** The derived `Remap` traversal, relocatable
  namespaces, extraction and linking with certificates relocated, the
  boundary check, and the round-trip test.
- **R2: modules link modules (done).** The registry stores relocatable validated
  namespaces; a `leaner module` links the ones it uses, and verification
  inlines across modules. The Move standard library becomes verifiable
  module by module.
- **R3: proof reuse (open).** A theorem about a namespace is stated about the unit
  it was proved in, so reusing it in another unit needs either a theorem
  that the semantics is invariant under consistent renumbering, or
  semantics over namespace-local IDs with runtime identities by path.
  Whichever is cheaper makes a package's verification cost linear.
- **R4: shipped namespaces (open).** The exchange format carries
  relocatable namespaces with their certificates, and with R3 their
  theorems, so a frontend ships a precompiled, verified package that a
  unit links like a registered module.

## Non-goals

- Separate compilation of cyclic namespace groups: Move forbids module
  cycles, and a Rust crate is one namespace.
- Changing the surface language: LeanerLang source is unaffected, apart
  from modules being able to call one another.
