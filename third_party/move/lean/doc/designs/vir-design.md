# Unified Verex intermediate representation

## Scope

This document defines the language-neutral VIR shared by Move, Verex source
under the Move and Rust profiles, and Rust MIR: its stages, profiles,
validation, serialization, and the contracts of its frontends and backends.
The schema itself is `verex-ir/VerexIR/Syntax.lean`; this document states
the rules the schema serves. The Rust frontend's own design is
[`rust-mir-design.md`](rust-mir-design.md). Open work:
[`roadmap.md`](../plan/roadmap.md).

Reference types and lifetime identities are core VIR nodes rather than opaque
Move payloads. Both Move frontends create explicit `inference` lifetimes for
elided source lifetimes; generating and solving their constraints is a
separate semantic-checking pass. A raw unit can rely on upstream
Lean/compiler-v2 typing, and some source constructs are carried as named
untrusted evidence instead of executable VIR semantics. No such construct is
silently treated as checked.

## Objective and boundary invariant

VIR is the complete, source-independent semantic representation of a checked
Verex compilation unit:

```text
Move source -----------+
Verex source ---------+----> raw VIR ----> validated VIR
Rust crate/MIR import -+                         |
                                                 +--> Move source
                                                 +--> Rust source
                                                 +--> profile-selected Verex source
                                                 +--> profile-specific execution
                                                 +--> verification obligations
```

All frontend paths for the same semantic profile must produce semantically
equivalent VIR for equivalent programs. Source backends must emit a
semantically equivalent program in a compatible target profile. Executable
lowering may erase proof-only declarations, but that lowering is not VIR and
is never the input to source analysis, validation, or reporting.

Two central rules, shared with the Rust frontend:

> Function origin is metadata; function meaning is semantic.

> VIR is the union of known Move and Rust semantics, not merely their
> intersection. Profiles extend that union; they do not hide known nodes.

Origin records whether a body came from Lean elaboration, compiler-v2, rustc,
or generated source; a separate semantic profile records whether its
integers, references, storage, destruction, and throws have Move or Rust
meaning. No semantic pass dispatches on origin. Frontends and backends are
explicitly partial: a frontend rejects a source construct it cannot lower
faithfully; a backend publishes a capability set and rejects a valid VIR
construct it cannot represent; neither encodes a known Move/Rust construct
as an opaque profile value to avoid extending the core schema. `ProfileValue`
is reserved for semantics outside the known union.

Ownership rules:

- frontend adapters only decode and expand source-specific forms and
  construct raw VIR plus explicit upstream assumptions or alignment evidence;
- authoritative name resolution, type checking, semantic analysis, and
  backend-independent diagnostics operate on VIR;
- every backend consumes validated VIR, and capability checks and reports
  operate on it;
- no analysis consults XAST, Lean syntax, Lean environment extensions, or a
  compiler-v2 model after raw VIR construction;
- no backend silently drops or weakens a semantic VIR node;
- source locations are VIR metadata identified uniformly by `LocId`.

"One analysis point" is one public checked-construction boundary,
`VerexIR.Validation.validate : ProfileRegistry → RawUnit → Except (Array
Diagnostic) ValidatedUnit`, implemented as a pipeline of small passes.

## Ownership and dependencies

VIR is owned by no source frontend, transpiler, verifier, or bytecode
backend. `verex-ir` depends only on Lean base libraries; `verex-move` and
`verex-rust` depend on it and it never on them. The neutral raw-VIR
serializer owns the transport schema; XAST and the Rustc Public adapter
mechanically construct `RawUnit` documents with source-specific provenance
and import evidence and own no semantic policy. Profile libraries provide
schemas, well-formedness rules, semantic interpretations, and capability
definitions, never depending on a frontend AST; applications assemble a
`ProfileRegistry` and enter through `validate`.

`verex-ir` owns the staged data, identifiers, resolution and typing,
canonicalization, validation, indexes, profile registration, capability
queries, and serialization. Frontends own parsing, expansion, and raw-VIR
construction; every upstream fact they rely on is a hint, certificate, or
named trust assumption. Backends own representation choices and capability
checks only.

## Semantic data, provenance, alignment

VIR separates three kinds of data and keeps none in frontend-owned side
tables:

- **semantic data** affects execution or proof meaning (declarations, types,
  bodies, conditions, frames, state anchors, intrinsic bindings, visibility,
  abilities, language-defined pragmas) and is never dropped;
- **provenance** affects diagnostics or source reconstruction (locations,
  comments, documentation formatting, authored spellings, surface notation,
  generated-node reasons); a backend omits it only if its contract disclaims
  source-quality reconstruction;
- **alignment evidence** does not change a body's denotation but controls the
  external claim a theorem supports (producer and schema versions, source
  hashes, target configuration, upstream checking status, imported CFG,
  structurization witness, semantic hashes, trusted assumptions); it may be
  erased for local reasoning but must be present and checked before a
  theorem is presented as a claim about Move source, Rust source, or MIR.

### Import receipts and source alignment

A serialized `RawUnit` is untrusted transport data, even as an ephemeral file
private to the build. The import gate is owned by Lean: it invokes the
selected producer, consumes its output, and validates the raw VIR. The
boundary distinguishes **upstream checks** (compiler-v2 or rustc admission),
an **import receipt** constructed by the Lean-owned orchestration (producer
version, invocation configuration, input hashes, output semantic hash), and
**alignment**, which either proves that the imported VIR represents the
source artifact or names the translation as an explicit trusted assumption.
An in-process frontend may skip the file and receipt but not the alignment
obligation. VIR validation proves well-formed internal semantics; a
source-level theorem additionally requires alignment, and a claim about
emitted code requires backend alignment. A frontend-produced `checked` label
without a checked receipt and alignment is reported as a producer assertion.

Semantic equivalence erases provenance:

```text
semanticEquivalent(a, b) :=
  alphaEquivalent(canonicalize(semanticProjection(a)),
                  canonicalize(semanticProjection(b)))
```

`semanticProjection` erases no executable or specification meaning;
provenance fidelity is tested separately.

## Representation stages

```text
RawUnit --normalize/structurize--> StructuredUnit --resolve--> ResolvedUnit
        --infer/check types--> TypedUnit --validate/index----> ValidatedUnit
```

`RawUnit` is the only frontend output: declarations, name and type spellings,
structured bodies or profile-tagged CFG bodies with neutral control,
specifications, profile configurations, provenance, alignment evidence,
dependency interfaces, and optional resolution/type hints. Structurization
converts reducible CFG bodies such as imported MIR and records a checked
block/edge correspondence; unsupported CFG shapes are diagnosed there.
Resolution replaces name uses with checked declaration or dependency IDs
while retaining authored spellings. Typing assigns a checked type to every
expression, pattern, local, parameter, result, and type argument. Validation
adds derived indexes and certifies all backend-independent rules;
`ValidatedUnit` is the only input to backends and most analyses.

Only `RawUnit` and `ValidatedUnit` are public construction boundaries; the
constructors of the other forms are private. Frontend resolution and type
information is a hint the shared checker verifies; a disagreement is a
diagnostic. Structurization and validation run in one checked call, with
upstream typed IDs recorded in import evidence while the checker
independently checks table safety, arena acyclicity, profile vocabulary,
declaration shape, and CFG structure. Materializing the intermediate stages
as values changes exposure, not the boundary.

Staging keeps MIR graph analysis inside VIR: the Rustc Public producer
exports MIR-shaped raw nodes and owns neither the structurizer, loop
identities, nor diagnostics. The raw boundary preserves MIR administrative
statements and call/drop/assert/cleanup terminators; the structurizer
classifies storage lifetime markers, place mentions, and user-type
ascriptions as inert administration and erases them after validation, and
under `panic=abort` lowers ordinary bounds, overflow, division, and remainder
assertions to explicit panic branches. Destructive and provenance-sensitive
statements, cleanup edges, and exceptional terminators are rejection
boundaries.

## Compilation units and dependencies

The checked object is a compilation unit, not an isolated module: calls,
friends, abilities, specifications, and intrinsic bindings refer across
module boundaries. A unit holds a version, one interned `Tables` snapshot,
profile configurations, owned namespaces, dependency interfaces, and import
evidence. A namespace is a language-neutral hierarchical declaration
container of which a Move module, Rust crate or module, and generated Verex
namespace are profile-specific views; the core does not bake Move
address/module identity into every declaration.

The `Tables` snapshot contains the identities referenced by owned namespaces
and dependency interfaces, once per unit. Owned namespaces occupy the leading
namespace-table positions so their dense `NamespaceId`s index the owned
array; dependency-only namespaces follow. Expression, pattern, place, and
local arenas are namespace- or declaration-owned.

A dependency interface is VIR data: exported declarations, types and
abilities, signatures, visible contracts and specification functions,
profile metadata, intrinsic declarations, and locations. It is not a pointer
into a Lean environment, compiler-v2 model, or rustc context. Full dependency
namespaces may be provided for analyses requiring bodies; an analysis reports
explicitly when an interface lacks required information.

## Semantic profiles and extension points

Known semantic families are first-class (`Profile.move`, `Profile.rust`);
numeric `ProfileId`s are reserved for extensions. A unit carries at most one
configuration per profile, resolved by constructor, never by array position;
checker and semantic registries are keyed by `Profile` and reject duplicate
implementations. Each function records its profile; origin is stored
separately.

The core fixes the known Move/Rust declaration, type, ability, trait,
implementation, attribute, place, control, call, contract, location,
diagnostic, and outcome union. A registered profile schema selects the
language rules for those nodes (Move rollback against Rust panic cleanup)
and defines only additional types, operations, state components, effects,
outcomes, and capabilities outside the known union, with its own reference,
operation, and intrinsic checks and an interpretation. Extension wire values
use stable `(profile, tag, payload)` encodings decoded by the registry into
closed checked values. Structural links to VIR nodes use typed ID fields,
never strings inside payloads; a Move construct encoded as a tag is not the
profile contract. Profiles may not add a second control-flow tree, contract
representation, location mechanism, or reporting path.

The Move profile supplies rules for core abilities, references and abort plus
signer and resource types, global resource and event state, and intrinsic
maps; the Rust profile supplies rules for references, drop and panic plus
initializedness, partial moves, slices, loan facts, allocation and cell
state, and cleanup. Traits, implementations, associated items, and evidence
passing are core, not Rust extensions, so a future Move trait feature uses
the same declarations, calls, interpreter interface, contracts, and proof
rules. `EvidenceId` is an opaque generic-argument slot until the checked
evidence table and dictionary semantics exist, which precede interpreting
trait calls or transferring proofs through trait contracts.

`verex-move` owns the Move profile and `verex-rust` the Rust profile against
the frozen interface. Verex source and compiler-v2/XAST may construct the
Move profile; VerexSL and imported MIR the Rust profile: a same-origin
frontend can construct different profiles, and different origins the same
semantics. A direct call across incompatible profiles is invalid;
interoperation requires an explicit validated boundary adapter defining type
conversion, state/effect projection, and outcome translation. A Rust panic
is never printed or interpreted as a Move abort.

## Required language coverage

A unit includes the union of constructs accepted from the Move and Rust
frontends plus registered extensions: namespaces, imports, friends,
attributes and pragmas, docs and comments; constants; structures, enums,
traits, implementations, and intrinsic declarations; executable and
specification functions with their signatures, bodies, contracts, and loop
specifications; specification variables, axioms, and invariants; in-body
assertions and assumptions; structured bodies and raw reducible CFG bodies
with structurization evidence; explicit ownership operations, calls,
returns, and argument-carrying throws; state anchors; intrinsic role
bindings; generics with evidence; and provenance, producer configuration,
and alignment evidence. Lean proofs and declarations outside a registered
Verex namespace are not source-language semantics and stay outside VIR. A
construct may not survive only in a frontend side table.

## Identifiers, tables, and arenas

Identifiers are distinct structures, not interchangeable `Nat` aliases, one
per table (files, locations, origins, alignment, extension profiles, types,
namespaces, names, declarations of each kind, locals, places, lifetimes,
evidence, expressions, patterns). Every table has checked lookup returning
`Except Diagnostic`; unchecked lookup is internal and usable only after
table validation. IDs are stable within a unit snapshot; canonicalization
does not reorder declaration or arena tables; a compaction pass, if ever
needed, applies one checked old-to-new remapping.

Types, locations, module references, and qualified names are interned once
per unit; types bottom-up, recursive data types through declaration IDs. An
interned type has no source range, so occurrences use a located `TypeUse`
in signatures, fields, locals, casts, and generic arguments. A namespace is
a finite, deterministic record of its declarations by kind (constants,
structs, associated items, traits, implementations, functions, specification
functions and variables, invariants, intrinsics, comments), its arenas, its
imports, profile metadata, attributes, and pragmas. Trait and implementation
declarations are ordinary core declaration tables referenced through checked
IDs, not profile payloads. Declaration arrays retain source order.

## Types, places, operations, and structured bodies

The core type language is the known Move/Rust union: unit, never, Boolean,
character, fixed-width integers, tuples, vectors with an optional fixed
length (Move `vector<T>` and Rust `Vec<T>` against Rust arrays), nominal
types with generic arguments, function types with abilities, type
parameters, references, and a profile extension. Function-value abilities,
generic predicates (abilities, trait implementation, associated type and
constant equalities, lifetime outlives, const equality), structure
abilities, trait identities, implementations, and associated-item bindings
are structural data; profiles define language-specific admission rules
rather than replacing them with tags.

Pure tuple/vector operations, indexing, Boolean operations, comparisons, and
integer arithmetic are core nodes. Fixed-width arithmetic is modular; the
`checked*` node carries its failure `ThrowKind`, so Move emits checked
arithmetic with `abort` and a Rust frontend preserves checked `panic` or
modular behavior without a profile hook; overflowing nodes return the
modular value and a Boolean flag. Characters are Unicode scalar values with
their own type, constant, scalar validation, and ordering; literal and range
patterns share those rules. Bitwise, shift, cast, and value-level ownership
operations use the same typed primitive family; unsupported static
obligations are capability diagnostics. Value-level borrow, dereference, and
freeze form a typed reference-operation family. A direct local read is
non-consuming; validation infers the access of a value type without `Copy`:
a selection or index of it with `Copy` copies its place, the operand of an
observer (discriminant, variant test, length, index check) reads it, and
otherwise the local is moved; a reference local keeps its read. Field and
variant operations carry strong qualified references; enum variants may
record an observable integer discriminant distinct from array position.

The specification language is core: contract clauses and the
`forall`/`exists`/choice quantifiers are typed nodes; mathematical integers,
ranges, event state, and type/resource/state domains are structural types;
logical operations use the closed `SpecOperation` family with strong
specification-function targets and typed payloads for result indexes, state
anchors, trace kinds, memory ranges, and widths. Function-value summaries are
the closed `BehaviorKind` family (`requiresOf`, `abortsOf`, `ensuresOf`,
`resultOf`, `unchangedOf`, `foldsOf`, indexed `writeOf`) with an explicit
pre/post `MemoryRange`, type-checked against the function-typed first
operand. `Operation.profile` is only for extensions outside the known union.

The core owns reference topology (mode, referent type, lifetime identity);
the attached profile owns aliasing, validity, storage, escape, and
operational rules, so a structural edge can be core while its observable
rules differ. Move `address` and `signer` are core types and address
literals core constants. An elided Move lifetime produces a fresh core
`inference` lifetime; authored lifetime binders map to `parameter` entries of
the same table. Lifetime constraints, inference, and profile-specific borrow
checking are separate checked passes.

Places are first-class (local, dereference, field, index, subslice,
downcast) because both lowerings need them and Rust moves and borrows cannot
be value-only expressions. Operations distinguish move, copy, borrow kinds,
read, write, assert, and drop. `GlobalKind` makes keyed storage first-class
(`contains`, `borrow`, `take`, `publish`, the first type instantiation
naming the resource family), with the same projection and mutation
machinery as heap references. `CallKind` distinguishes direct calls,
constructor packing and unpacking, closure packing, invocation of a callable
expression, and an extension case; the callee declaration supplies its
profile. `throw` is structured control carrying an argument array, so `abort
24` and richer payloads are representable. Arithmetic carries its actual
checked, wrapping, aborting, or panicking behavior. Known Move event and Rust
cell/drop operations are to be promoted like storage; profile operations are
only the fallback until then. A `SurfaceSyntax` (receiver calls, index
notation) is typed provenance, not semantics.

Structured, typed bodies are canonical; a stackless CFG is a derived
lowering. Expressions and patterns carry a location and a type and refer to
children through namespace-owned arenas, which the checker keeps acyclic.
The expression union covers values, constants, locals, parameters, resolved
operations and calls with explicit type/const/evidence instantiations,
blocks, conditionals, matches, sequences, loops, break/continue, returns,
throws, assignments, quantification, specification operations, and in-body
specification blocks; patterns cover variables, wildcards, tuples,
structure/enum destructuring, literals, and ranges. The inverse from a lower
executable IR is intentionally not required.

## Generics, evidence, ownership, and destruction

Generic declarations use one binder model: type, const, lifetime, and
evidence arguments; Move ability constraints and trait, lifetime, and const
predicates inhabit the same positions. Trait method calls use explicit
evidence records once the evidence layer exists; neither VIR nor generated
source relies on rustc, a future Move trait solver, or Lean instance search
to recover a selected implementation. The core does not reimplement Rust
coherence: an importer records upstream admission as alignment evidence,
and the interpreter and verifier consume explicit selection evidence.

Import never monomorphizes: a unit contains one generic body with binders
and predicates, and an interpreted call supplies arguments and evidence
without a cloned body. A compiler-produced concrete instance is at most
provenance for alignment, never a replacement body. If Rustc Public cannot
expose the generic body, the exporter is blocked rather than accepting a
monomorphized import. Proof-facing bodies stay generic; a concrete
executable artifact may exist only after generic interpretation and a
checked instantiation relation are defined, and type-, layout-, impl-, or
const-dependent behavior uses explicit equivalence classes or finite covered
instances, never generalization by dropping the dependency.

Generic parameter indexes are scoped by the declaration binder list and
kind-checked; a trait method signature sees the trait binders followed by
its own; value parameters correspond positionally to the leading local
declarations. Rust initializedness, partial moves, loan facts, drop flags,
and cleanup edges are not provenance: their observable effects become typed
places, operations, or Rust-profile state, and the rustc facts justifying
them are alignment evidence. Reference-bearing ADTs and returned-region
relationships survive signatures and bodies. Dynamic interior mutability
uses explicit identity-bearing cell state and observable guard drop, not
prophecy references disguised as a common operation.

## Specifications and state anchors

Specifications are first-class declarations and nodes: a block of pragmas,
conditions, and an optional frame; a condition has a kind, properties, an
expression, and a typed payload rather than positional extra expressions.
Condition kinds cover `requires`, `ensures`, typed exceptional conditions,
`succeeds_if`, `assert`, `assume`, `decreases`, effects, updates, structure
and function invariants, loop invariants, global and update invariants, and
axioms. Move `aborts_if` and Rust `panics_if` are surface forms of distinct
exceptional-outcome tags without shared rollback semantics.

State observations are explicit anchors: function entry, block entry, loop
entry, program point. A loop invariant records its loop and its `loopEntry`
anchor; an in-body assertion its `programPoint`; a backend never infers the
anchor from print position. Specification functions retain their authored
signatures (type parameters, parameters, result, uninterpreted and native
flags, executable counterpart, `old` use, body, attached block) even when a
logical intrinsic interprets applications generically; the retained
signature is what lets the checker reject a replacement that does not match
its intrinsic role. Contracts and loops stay clause-preserving; conjunctions
and proof normal forms are derived.

The common relational outcome distinguishes normal return, a typed throw
with its profile's state behavior, undefined or unsupported behavior, and
divergence. Move instantiates `throw abort` with rollback; Rust `throw
panic` with retained mutation and its configured terminal or cleanup
behavior. Framing and effect clauses use the profile's state/effect schema.

## Locations and diagnostics

Locations are interned, never a magic ID. An entry holds a primary span,
related spans, expansion context, a generating pass, and a parent. Every
authored node carries a `LocId`; generated nodes record their pass and the
authored parent; a root with no source has no primary span, an explicit
producer, and no parent. Derived lower instructions keep the location of the
node that produced them; inlining uses the call-site primary plus the callee
definition as related.

Diagnostics are typed data (severity, stable code, primary and related
locations, payload), formatted only at a CLI or editor boundary, so CLI
output, editor diagnostics, tests, and reports agree. Duplicates point at
the later occurrence and relate the first; independent diagnostics
accumulate in deterministic source order, stopping early only when table
safety prevents traversal. Source printers produce a `GeneratedSourceMap`
from generated ranges to node and location IDs, and re-elaboration records
the reverse, so an editor shows canonical source while reporting the
original range. Formatting never changes semantic identities.

## Intrinsic declarations and role graphs

Intrinsic metadata is first-class VIR: a declaration names its model, owner,
and the executable and specification bindings of roles to targets, each
with its location. Raw model and role names are strings so an older checker
can diagnose a new name at its exact location; validation resolves them to
closed semantic types in an index. Each model has one declarative role
schema (target kind, presence, dependencies, signature, interpretation).

The shared checker enforces the profile-independent part: owners resolve to
nominal declarations in their namespace, targets to the corresponding
declaration kind there, and duplicate owners, duplicate roles, and shared
targets are rejected with related locations. The Move-profile pass owns the
closed `MapRole` schema and rejects unknown models and roles, wrong
target-role categories, malformed owner binders, missing required logical
roles, missing declared dependencies, and targets outside the role's exact
physical signature alternatives. No frontend or backend owns another role
registry. Verex attributes such as `@[intrinsic_map]` are surface encodings
the frontend turns into declarations and the backend may reconstruct; the
map carrier is a backend interpretation, not the authority for graph
consistency.

## Checked construction and analyses

`check` returns diagnostics and, on success, a `ValidatedUnit` whose
namespaces carry derived indexes (qualified names, declaration ownership,
call targets, scopes, loop identities, intrinsic roles, dependency links),
validated intrinsic graphs, and semantic features; the raw unit is not
mutated. It composes bounds, normalization and structurization, scope and
name, common and profile type rules, declaration, body, specification,
intrinsic, ownership, ability, borrow-safety, and other backend-independent
passes; profile rules are registered passes, not callbacks into a frontend.
Effect, borrow, and capability analyses take validated VIR and report typed
diagnostics and capability data. A report never searches source comments,
XAST, or printed output; a feature is reported missing only when a node
exists and the selected backend's capability query rejects it.

Order: a frontend constructs a complete `RawUnit` with dependency interfaces
and evidence; VIR bounds-checks tables before following an ID; normalizes
structured input or structurizes raw CFG input and checks the witness;
resolves names and checks hints; infers and checks types and lifetime/borrow
constraints, then upstream hints and certificates; validates declarations,
bodies, specifications, profiles, effects, intrinsics, abilities, and borrow
rules; reports use check results, alignment status, and capability queries;
a backend prints or lowers a validated view. A backend may reject an
unsupported valid feature; it may not weaken it.

## Frontend contracts

**Move.** The adapter exports source constructs into a Move-profile
`RawUnit`, retaining spellings and ranges; compiler-v2 facts are hints or
import evidence, never consulted through the model environment. Compiler-v2
resolves and type-checks before exporting XAST, so this path cannot produce
VIR diagnostics for sources rejected before export; it is a compatibility
path. Resolved intrinsic bindings are copied mechanically, each with its own
location, or a generated location parented by the owner pragma where the
exchange lacks one.

**Verex source.** One surface language parses macro syntax, expands sugar,
and constructs profile-tagged `RawUnit`s; Move- and Rust-profile constructs
belong to that language, not to dialects. Lean elaboration may locate
dependency interfaces but does not select semantics from origin, and
retained syntax is not consulted after construction. A semantic construct
without a VIR representation is a located construction error.

#### TODO: secure Lean-source discovery and elaboration

Lean source is an
active build input: the Move package integration discovers `.lean` beside
`.move` and invokes `lake env lean` on it, and elaboration can run
metaprograms, host I/O, and child processes with the compiler user's
privileges, so compiling an attacker-controlled package or transitive
dependency can execute arbitrary host code. Before Lean-source discovery is a
production compiler feature: discovery must not imply authorization;
elaboration must require an explicit trust decision for root sources and
every source dependency; the compiler must fail closed before spawning Lean,
naming the package and file; manifests, lockfiles, caches, and provenance
must record the authorized inputs so changes invalidate approval; sandboxing
is defense in depth, not the gate. Until then, packages containing Lean
sources are classified like packages containing build scripts. The VIR
boundary does not mitigate this, since execution happens while constructing
the input.

**Rust MIR.** A Verex-managed `verex-rust-export` Rustc Public driver runs
normal rustc analysis, mechanically maps the in-memory MIR to a Rust-profile
raw CFG in the generic `RawUnit` schema, serializes it, and stops before
code generation. `ImportEvidence` records the rustc, target, edition, panic
and overflow configuration, source hashes, signatures, generic predicates,
implementation-selection evidence, origins, and asserted type/borrow status.
There is no second MIR exchange format. VIR performs administrative
simplification, dominance and natural-loop analysis, reducibility checks,
and structurization, recording the CFG-to-tree witness; an unsupported graph
is a diagnostic at the Rust origin, never a raw `goto` fallback. Rustc's
admission is explicit upstream evidence; Verex runs its own VIR borrow
checker whose rules may accept more than rustc without widening imported
Rust, and both results stay visible in evidence and theorem reports.

**Agreement.** Checked equivalent inputs in one profile have alpha-equivalent
canonical semantic projections under an explicit deterministic mapping of
declaration, local, expression, and generated-name IDs; printed text is
never compared. For Rust the comparison is between structured imported MIR
and a fresh import of canonical Rust emitted from VIR through the same
driver; re-elaborated Rust-profile Verex source is a separate optional
comparison. Alignment evidence is compared by its own freshness and
refinement contract.

## Alignment and theorem meaning

Verification first proves `Contract.Satisfies function.semantics
function.specification` about profile-aware VIR semantics. Claiming that the
source or MIR artifact satisfies the contract additionally requires checked
alignment, `UpstreamExecution artifact input outcome →
function.semantics.Relates input outcome`. Move source, Lean-authored
declarations, generated Verex source, and Rust MIR each have their own
producer-alignment obligation and converge on one VIR meaning; constructing
VIR never proves the correspondence. An adapter may mark alignment as
trusted, bound to pinned producer versions, source and configuration hashes,
and a semantic body hash; the theorem UI reports every trusted importer,
upstream check, and external summary an end-to-end claim depends on. Checked
translation witnesses or refinement theorems can replace those assumptions
without changing contracts or semantics.

## Backend contracts

**Move source.** Emits canonical, semantically equivalent Move from a
compatible validated view, accepting the Move profile or a validated adapter;
a VIR feature Move cannot express is rejected by the capability pass before
printing, never omitted.

**Verex source.** Selects profile-appropriate forms of the one surface
language from the profile, never origin; libraries provide generic
facilities such as intrinsic carriers, but the backend neither validates nor
rediscovers graphs, and emits attributes only as surface encodings. For a
Rust-profile unit it emits canonical Verex source, using a Move-profile form
only where an explicit profile rule proves the meanings equal and otherwise
an unambiguous Rust-profile primitive; unsupported families fail explicitly.

**Rust source.** From a validated Rust-profile view, checked dependency
interfaces, and the recorded target configuration, emits canonical standard
Rust, never reconstructed tokens or macros; retained comments are
non-semantic provenance. It preserves the profile's integer, panic, drop,
generic, trait, and unsafe semantics or reports a located capability error.
The generated crate is re-imported through `verex-rust-export`, and its fresh
`RawUnit` must validate to equivalent semantic VIR.

**Executable.** `VerexMove/Xir` lowers one validated Move namespace to XIR,
the positional stackless CFG compiler-v2 loads and compiles `.lean` sources
through: structured control becomes blocks, a value the locals holding it, a
place is borrowed step by step from its root, and a construct with no
bytecode form is a located error. Proof-only declarations are erased only at
this boundary, the borrow analysis's verdict gates emission, and locations
become the XIR source map. Compiling VIR to Rust machine code is not an
initial backend; a future one must preserve the Rust profile.

**Verification.** Derives relational meaning, conditions, contracts, loop
specifications, and obligations from the profile and the structured body of
the same validated VIR as executable lowering; it never synthesizes `true` or
empty specifications. Lean theorem generation is a UI for obligations, not
specification storage. Call summaries cross origins freely inside a
compatible profile; cross-profile summaries require a validated adapter.

```text
VIR --erase proof-only declarations / lower structured code--> XIR
```

XIR (`move-model-exchange`'s `XirModule`) is one profile-specific lowering,
not the definition of VIR; a source backend never reconstructs declarations
from it, and verification derives no lower IR. VIR JSON is the raw wire
format that replaces XAST's role.

## Serialization and compatibility

`RawUnit` is the process-boundary wire type; deserialization never
constructs a `ValidatedUnit`. The in-memory type and the JSON schema share a
major version, profile payloads carry their own; unknown major versions are
rejected, and a newer minor version is accepted only when every unknown
field is ignorable provenance. Closed enums use stable textual tags;
open-ended attributes, pragmas, and raw intrinsic roles stay strings until
validation. Encoding is deterministic, `decodeRaw(encodeRaw(normalizeRaw r))
= normalizeRaw r`; the strict parser rejects duplicate keys and the typed
decoder unknown fields recursively, as does the Rust serde mirror. The
version is spelled in the defaults of `Version` (read by the codec and by
validation) and in the Rust exporter's literal; a generated constant for the
exporter would make a bump a single edit.

XAST v4 is a compatibility frontend converting immediately to raw VIR, and
the Rust MIR exchange decodes immediately to a raw CFG body; neither
exchange-specific view gets semantic analysis. Persistent imported artifacts
carry a frontend-neutral receipt (producer, source hashes, configuration,
semantic body hash) that the Lean-owned import gate checks before `check`;
a theorem registry keys imported claims by semantic hash and alignment
status so a source change cannot retain an old theorem.

## Laws

1. Canonicalizing a canonical semantic projection changes nothing.
2. Normalized raw units survive JSON exactly, and checking the decoded unit
   yields the same canonical validated semantics.
3. Equivalent inputs for one profile check to alpha-equivalent semantic
   projections, independent of origin.
4. Move → VIR → Move → VIR, Verex → VIR → Verex → VIR, and imported MIR → VIR
   → canonical Rust → MIR import → VIR preserve semantics; Rust-profile VIR →
   Verex source → VIR preserves semantics for the supported surface.
5. One surface may emit another only for the subset whose profile meanings
   agree or through a checked adapter.
6. Move-profile VIR → XIR preserves outcomes of the executable language;
   verification lowering preserves clauses, frames, anchors, invariants, and
   axiom denotations; accepted raw MIR and its structured VIR have the same
   outcomes under the Rust profile.
7. Corresponding invalid inputs that reach raw VIR produce equal diagnostic
   codes and data after locations are erased.

Semantic equivalence is not equal text or equal provenance.

## Deferred-work register

A deferral is recorded here, and in a frontend design when applicable, with
its reactivation point, and removed only when the work lands, is rejected,
or is superseded by a recorded decision.

| Deferred item | Reactivate by |
|---|---|
| Strict RawUnit JSON v1: the deterministic encoding frozen and published as the external compatibility contract | Before RawUnit JSON is emitted outside the repository |
| Full dependency interfaces and authoritative cross-namespace resolution (today one `Tables` snapshot with a declared interface for every external import) | Before dependency signatures or bodies participate in typing, interpretation, or verification |
| Checked trait-selection evidence table, associated normalization, dictionary semantics | Before trait calls are interpreted or trait contracts transfer proofs (Rust M2) |
| Structurization and semantics of the remaining raw MIR administration and exceptional control: destructive and provenance nodes, cleanup edges, pointer-alignment assertions, reachable unreachable/resume terminators, all rejected today | Rust M4; the unsafe profile for pointer-alignment assertions |
| Drop flags, cleanup scopes, panic behavior, and unwinding beyond fixed `panic=abort` | Rust M4; full unwinding separately approved |
| Authoritative resolution, whole-unit semantic type checking, lifetime constraints, and the rest of VIR borrow checking: general non-lexical loan death, dependency and unknown aggregate alias propagation, nominal variance, use-site region constraints | VIR Phase 2; Rust M3 |
| Freshness and alignment: binding retained import-evidence statements to source, configuration, and semantic hashes; the Rust import receipt in the validated-unit proof boundary | Rust M1.5; M7 before a theorem about an external Rust artifact |
| Raw pointers, allocation and provenance, unsafe cells and unions, explicit UB semantics | Rust M5 ([`unsafe-pointers.md`](../plan/unsafe-pointers.md)) |
| Prophetic-reference residue: a loan-typing environment for walker-level preservation, non-lexical loan death, two-phase borrows, interior mutability ([`prophetic-references.md`](prophetic-references.md)) | The evaluator induction; Rust M3; a two-phase/interior-mutability design before `Cell`-style constructs |
| Source printers and round trips: normalization of validation-only bodies, Move/Verex-native alpha-equivalence, Move/Verex source maps, cross-language location fidelity, corpus-wide round trips | VIR Phase 6; Rust M2/M7 |
| Materialized `StructuredUnit`, `ResolvedUnit`, `TypedUnit` stages and the full staged API | With authoritative resolution and typing |

## Phases

The phases name the work and its gates; status is in
[`roadmap.md`](../plan/roadmap.md).

0. Freeze the boundary: every type, operation, effect, outcome, and state
   component classified as core, Move, Rust, or unsupported. Gate: nothing
   unclassified.
1. Neutral core, profiles, IDs, and wire format. Gate: schema, malformed
   table, normalization, and JSON tests pass without a frontend.
2. Shared checking and the Move profile; `validate` the only route to
   `ValidatedUnit`. Gate: raw fixtures exercise core-union, Move-only,
   synthetic-profile, and invalid cross-profile operations; nothing
   constructs validated values directly.
3. The XAST compatibility frontend, the first backend, and the intrinsic
   role schema. Gate: intrinsic error fixtures use shared codes; no printer
   or report pass accepts XAST.
4. Move and Verex frontends building raw VIR directly, authored locations
   captured before expansion. Gate: the Verex tests and the Move corpus dump
   validated VIR; an inventory test detects side-table-only semantics.
5. The `verex-rust` boundary against the frozen profile interface. Gate: the
   safe sequential Rust subset reaches validated VIR without a
   frontend-specific analysis path.
6. Source round trips, compared as semantic VIR. Gate: corpora pass semantic
   and location fidelity tests.
7. Lower backends: validated VIR to XIR, verification over validated VIR.
   Gate: no semantic analysis reads frontend syntax after raw construction;
   no backend accepts unchecked VIR.

Every compatibility adapter has a named removal phase and gate. A missing
VIR feature is explicitly unsupported; a frontend side table is not a
substitute.

## Tests

Each package's `lake test` covers its layer: schema (every variant, invalid
ID, cyclic edge, unknown tag, deterministic encoding), locations, one
fixture per diagnostic code, frontend differentials compared as canonical
projections, profiles (same syntax, different semantics; cross-profile calls
rejected), structurization, backend round trips, the stdlib and framework
corpora, and mutation (removing a condition, anchor, binding, type use, or
location, or changing a hash, fails a gate). Readable source baselines are
not the semantic oracle.

## Non-goals

Byte-for-byte formatting; reconstructing Rust tokens, comment attachment,
macro invocations, or surface constructs from MIR; replacing compiler-v2
parsing or expansion (its imported results are hints, `validate` is
authoritative); reimplementing rustc's trait solver; reconstructing
structured source from stackless bytecode; compiling VIR to Rust machine
code; encoding Lean proofs or helper definitions in VIR; making every
backend total over every VIR version; completing the Rust importer or its
alignment proof inside this project.

## Risks

Another permanent representation (keep XAST a compatibility frontend and
decode MIR immediately); a giant Move-plus-Rust union (small structural
core, checked profiles, no parallel control, specification, or report
systems); duplicated type systems (VIR checking authoritative, upstream
hints tested at the boundary); frontend drift (paired fixtures, shared
diagnostics); lossy adapters (semantic equality tests and a deletion phase);
macro location loss (authored ranges captured early, generated-parent
chains); over-normalization (clause structure, anchors, source order, and
provenance retained, proof normal forms derived separately); schema churn
(strong versioning, open raw names, rejected unknown semantic fields);
unsound imported claims (proofs bound to hashes, trusted assumptions
reported in theorems); large phases (independently shippable gates, no new
semantic debt in frontend paths).

## Decisions

1. `verex-ir` / `VerexIR` is the neutral package and namespace.
2. VIR has a v1 in-memory schema; XAST is a compatibility frontend; the
   deterministic derived JSON encoding is the external wire contract once
   frozen (deferred-work register).
3. Comments are a core table whose producers may leave it empty.
4. The checked implementation is in Lean.
5. Semantic profiles are explicit; the first profile is Move, and
   incompatible cross-profile calls require boundary adapters.
6. The Rust profile's panic mode, target configuration, and imported
   type/borrow trust status belong in `ProfileConfig` and `ImportEvidence`,
   not in hard-coded core choices.

All shared semantic analysis, validation, structurization, capability
reporting, and source emission is downstream of raw VIR, and origin never
selects meaning.
