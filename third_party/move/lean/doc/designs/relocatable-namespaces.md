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

The VerexSL registry persists, per namespace, its relocatable namespace and its
interface, nothing of the units that link it, so an `.olean` grows with its
namespace and not with what it reaches. A `verex module` that uses others is
lowered against their interfaces, then links their namespaces in place of those
interfaces. The file that elaborated a module keeps its linked unit, the module
at namespace 0 with what it reaches; an importer that asks for the unit of an
imported namespace links it then, and is told if the imported namespaces do not
link. A module's memory summary persists beside it (`Verify.lean`), so deciding
which modules' invariants concern a unit links nothing. Its closure
templates, the calls and closures of its bodies with their instantiations in
its own identifiers (`closureTemplates`), persist in its registry entry: a
unit's closure sites are their join (`closureSites`), each namespace's mapped
through its links, the instances reached from the closed functions, and each
site's rows computed in the unit, so finding them compiles no body.
Verification names a function of the unit by its name in the unit's module,
after the scopes it is in,
or its path-qualified name in another namespace. A callee of another module
is inlined like a module's own; one used through its contract (opaque) is
verified in the importing unit as well. A clause authored in another
module's file is reported at its position in that file.

## Compiled namespaces

A package is verified as Lean builds one. Its export is rendered once, each
namespace to a file of its own importing the namespaces it uses
(`renderNamespaces`), and each file is elaborated to an `.olean`, in dependency
order and in parallel (`compileNamespace`, `scripts/verify-modules.py`). The key
is the namespace, not the Move module: a Rust crate is one namespace whose modules
are scopes, and crates form a DAG, so a Rust driver passes the same request.

An `.olean` depends only on what its namespace uses, so it is the same whichever
target imports it. A target's rendering holds its own namespace and its `verify`
items and imports every namespace rendered before it, the set of registered
modules that decides what its unit links. A namespace rendered after it, whose
invariant reads memory one of its functions writes, verifies that function in its
own command, so the rendering also holds what follows the target as far as the last
such namespace; usually there is none. What it links is relocated exactly as
when those namespaces were elaborated in its file, so its unit, targets and proofs
do not change; a message at a position in an imported namespace's rendering maps
through that namespace's source map.

A registered namespace's relocatable namespace is defined where it is registered
(`<path>.object`), so in its own `.olean`. A unit's definition places these
definitions in the unit (`RelocatableNamespace.place`) with their links and the
unit's tables, so the literal a target quotes grows with the number of
namespaces, not with their bodies. The audit compares each definition once with
the namespace registered at its path. A target's own namespace is defined in its
verification process, whose rendering of it differs from the compiled one.

A trace beside each `.olean` hashes what it was made from: the executable, the
libraries, the options, the rendering, and the traces of its imports. Numbering a
declaration's temporaries per declaration keeps a rendering independent of the
export it came from.

## Proof reuse and shipped namespaces

A theorem about a namespace is stated about the unit it was proved in.
Reusing it in another unit rests on a theorem that the semantics is
invariant under consistent renumbering, or on semantics over namespace-local
IDs with runtime identities by path, whichever is cheaper; it makes a
package's verification cost linear. On it, the exchange format carries
relocatable namespaces with their certificates and theorems, so a frontend
ships a precompiled, verified package that a unit links like a registered
module. Neither is built ([`roadmap.md`](../plan/roadmap.md), "2. Verification
across modules", R3 and R4).

## Non-goals

- Separate compilation of cyclic namespace groups: Move forbids module
  cycles, and a Rust crate is one namespace.
- Changing the surface language beyond modules calling one another.
