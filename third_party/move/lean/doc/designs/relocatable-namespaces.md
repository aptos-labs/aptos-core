# Relocatable namespaces

A validated VIR namespace behaves like a Lean `.olean`: produced and checked
once, then included in any unit that uses it, with its validation results
reused rather than recomputed, and eventually its theorems. Open work:
[`roadmap.md`](../plan/roadmap.md), section 2.

## Why

A Move module is verified by inlining the bodies of the modules it uses, as
the Move Prover does, so a module needs the VIR of its dependencies. With
unit-relative identities every importer must renumber each dependency,
re-validate it, re-quote it, and re-derive every certificate about it: cost
quadratic over a package. Move module dependencies form a DAG, so everything
established about a module is independent of its importers.

## Model

1. **Relocatable namespaces.** A relocatable namespace carries its own
   tables (files, locations, origins, alignments, lifetimes, types, names,
   and namespace references) holding exactly the entries it uses, with its
   IDs indexing them. Entry 0 of its namespace references is itself; every
   other entry names another namespace by path (`NamespaceRef`). It carries
   its validation results: the structurization witnesses and the
   initialization and borrow certificates of its functions.
2. **Relocation.** One operation moves a namespace from one set of tables
   into another, interning every entry it uses and renumbering its IDs.
   Relocating into empty tables extracts a namespace from a unit; relocating
   into a unit's tables links it. The traversal is a `Remap` class whose
   instances are derived for every VIR type, so no ID field is missed; a
   round trip (extract, link, print) is its check. Types are interned up to
   the locations of their type arguments (`Ty.eraseLocs`).
3. **Linking without re-validation.** A unit links validated namespaces and
   checks only the boundary: each reference resolves to a namespace in the
   unit, and the declarations a namespace was lowered against equal that
   namespace's exports. The unit-wide parts of a validated unit (the
   resolution index, counts) are rebuilt in time linear in the unit.
4. **Downstream code unchanged.** Validation, the semantics, the printer,
   and the proofs work on unit-numbered IDs.

The VerexSL registry holds each module's validated unit, the module at
namespace 0 with the namespaces it reaches linked in. A `verex module` that
uses others is lowered against their interfaces, then links their
namespaces, extracted from their own units, in place of those interfaces.
Verification names a function of the unit by its name in the unit's module,
or its path-qualified name in another namespace. A callee of another module
is inlined like a module's own; one used through its contract (opaque) is
verified in the importing unit as well. A clause authored in another
module's file is reported at its position in that file.

## Proof reuse and shipped namespaces

A theorem about a namespace is stated about the unit it was proved in.
Reusing it in another unit needs either a theorem that the semantics is
invariant under consistent renumbering, or semantics over namespace-local
IDs with runtime identities by path; whichever is cheaper makes a package's
verification cost linear. With that, the exchange format carries
relocatable namespaces with their certificates and theorems, so a frontend
ships a precompiled, verified package that a unit links like a registered
module.

## Non-goals

- Separate compilation of cyclic namespace groups: Move forbids module
  cycles, and a Rust crate is one namespace.
- Changing the surface language beyond modules calling one another.
