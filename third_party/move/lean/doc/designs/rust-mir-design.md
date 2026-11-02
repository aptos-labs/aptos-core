# Verex Rust frontend through the shared Verex IR

Open work: [`roadmap.md`](../plan/roadmap.md).

## Decision

Verex will build its Rust frontend in a new `verex-rust` package around a
small, project-owned exporter using
**[Rustc Public](https://rust-lang.github.io/rustc_public/getting-started.html)**
(the project formerly called Stable MIR).  The exporter maps in-memory MIR to
the versioned, Rust-profile `RawUnit` exchange format owned by `VerexIR`,
serializes it, and stops the compiler before code generation.  It will not use
Charon as the production import boundary and it will not introduce a second
on-disk MIR format.

The exporter owns only extraction, mechanical MIR-to-raw-VIR mapping, and
serialization.  `VerexIR` owns the exchange schema, validation, CFG
simplification, structurization, semantic interpretation, canonical
Rust/Verex source printing, and all proof-facing diagnostics.  This keeps the
Rust-specific trusted/transformation surface small and ensures that the same
validation path is used for future producers.

Charon remains valuable in three deliberately limited roles:

- a reference implementation and corpus source while defining the exchange;
- a differential-testing oracle for the raw CFG and recovered structured
  control flow; and
- a temporary experiment adapter if a necessary Rustc Public query is not yet
  available.

An experiment adapter must emit the *same* `RawUnit` exchange schema and must
not make its LLBC structurization or its semantic translation authoritative.
It is removed once the required Rustc Public surface is available.  We do not
maintain two production frontends with independently observable behavior.

### Why this decision

Rustc Public is the best long-term fit because Verex wants the compiler's
actual typed, borrow-checked Rust input—not a verifier-oriented dialect that
we must adopt as an additional source language.  It lets us track the Rust
toolchain deliberately, preserve raw-pointer and other low-level MIR
operations for a later unsafe model, and choose Verex's own structured IR and
semantics.  It also avoids making a second project and its pinned compiler
driver a release dependency for every accepted Rust crate.

This should not be read as a claim that an arbitrary Rust crate will always
import without work.  [Rustc Public's current documentation](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_public/index.html)
says that its APIs cover function bodies, types, monomorphic instances, and
ABI information, but the crate is still in-tree, requires nightly plus
`rustc-dev`, and is explicitly subject to breaking change.  A stable API
boundary, once available, would still not be a stable semantic model of every
Rust feature.  The frontend therefore supports a versioned *Rust profile* and
tests a rolling toolchain before declaring it supported; it does not silently
accept a new compiler or a new MIR construct.

Charon has real advantages: it already serializes ULLBC/LLBC, reconstructs
structured control flow, and has broad experience with Rust analysis,
including unsafe syntax.  Its cost is equally real: it is a custom rustc
driver built with a pinned nightly, its simplification and structurization are
another semantic boundary before VerexIR, and its support cadence becomes a
constraint on Verex's Rust-at-tip goal.  It is an excellent validation aid,
not the right owner of Verex's source-of-truth semantics.

| Dimension | Rustc Public exporter (chosen) | Charon |
|---|---|---|
| Rust compatibility | Directly interrogates the designated rustc revision; a new MIR form can be preserved or rejected by Verex. | Adds a custom driver and its own pinned nightly compatibility window. |
| Input representation | Raw, typed MIR-shaped CFG; Verex validates every simplification and structurization. | ULLBC simplifies MIR; LLBC additionally reconstructs structured control flow. |
| Verex proof boundary | One project-owned exchange and one checked VerexIR path. | Mature analysis IR, but its transformations become an extra boundary or must be duplicated. |
| Unsafe growth | Preserves the compiler's pointer/unsafe operations in v1 even before Verex models them. | Useful prior art and has unsafe-code support, but Verex would inherit a second IR's operation choices. |
| Delivery risk | Public surface remains in-tree and may omit a needed query or change; we carry a compatibility job. | Faster prototype and serializer today, but permanently depends on Charon's driver/toolchain release cadence. |
| Assigned role | Production exporter. | Differential oracle, corpus source, and narrowly scoped fallback experiment. |

### Support and toolchain policy

The production exporter is built and tested against one pinned Rustc Public
toolchain.  CI also runs a compatibility job against a recent Rust tip.  A
green compatibility job promotes that revision only after the exchange schema,
importer, and corpus tests pass; a regression blocks promotion and is reported
as an exporter compatibility issue.  Each artifact records the exact rustc
commit/release and Rustc Public API version, so Lean never proves a stale or
differently interpreted body.

"Full Rust at tip" consequently means that we use rustc to type-check every
crate and retain every encountered construct faithfully enough to either model
it or reject it at a precise source location.  It does *not* mean that Verex
claims verification coverage for every standard-library implementation,
platform ABI, unsafe operation, optimizer transformation, or target.  The
artifact's declared profile, capability report, and theorem assumptions define
the actual coverage.

## Summary

This project imports borrow-checked Rust MIR into the language-neutral
`VerexIR` boundary used by every Verex frontend.  Verex source frontends and
the Rust MIR adapter construct profile-tagged `RawUnit`s; the single
`VerexIR.Validation.validate` pipeline produces the `ValidatedUnit` consumed
by common semantics, contracts, verification, executable lowering, and source
backends.  No frontend produces a separate proof-facing function meaning.

The Rust source printer is a required backend, not an attempt to recover the
original Rust tokens.  For a supported Rust-profile unit it emits readable,
canonical standard Rust (`.rs`), which rustc must re-import to an equivalent
validated VIR body.  The profile-selected Verex source backend remains useful
for presentation, but generated Lean syntax is not the Rust round-trip
mechanism.
This makes the shared IR a genuine interchange representation rather than a
one-way compiler detail.

The central design rule is:

> Function origin is metadata; function meaning is semantic.

The shared schema is the union of known Move and Rust semantics, not only
their intersection. A Rust construct does not become an opaque profile
payload merely because Move cannot print it, and the converse holds for Move.
Frontends may reject inputs they cannot faithfully lower and backends may
reject valid VIR they cannot represent. Profile extension values are reserved
for semantics outside the currently known union.

A specification applies to the observable behavior of a typed function, not
to whether its body came from Lean elaboration or rustc.  Origin still matters
for two non-semantic purposes:

1. alignment establishes that the internal function represents its upstream
   Lean declaration or Rust MIR body;
2. source provenance maps diagnostics and proof obligations to the correct
   source.

## Goals

- Import an explicitly supported, safe, sequential Rust subset from MIR.
- Converge Verex-source and Rust-MIR frontends on `RawUnit`, then on one
  validated, typed, named VIR unit.
- Reproduce imported Rust as readable, canonical Rust source from validated
  VIR, and make the profile-selected source printers available to every
  frontend that reaches the same unit.
- Require generated Rust to recompile and re-import to an equivalent validated
  Rust-profile body.
- Reuse one `spec` / `verify` interface and one modular contract system for
  both kinds of definition.
- Reuse the existing ownership-passing and prophecy model for ordinary Rust
  references where its assumptions apply.
- Preserve enough Rust provenance to report translation, lowering, and proof
  failures at the original `.rs` locations.
- State the trust and alignment boundary precisely; a proof about an imported
  AST must not silently be presented as a proof about Rust MIR.
- Keep source metadata outside semantic equality and proof terms.

## Initial non-goals

- Reconstructing the original Rust spelling, macro invocations, or surface
  constructs after MIR lowering. Source comments are retained separately as
  non-semantic provenance, but exact token attachment is not promised.
- Making canonical generated Verex source textually round-trip to the
  original `.rs` file.
- Exact round-tripping of Rust syntax such as `for`, `?`, methods, and pattern
  syntax after MIR lowering.
- Reimplementing rustc's type checker or trait solver.  Imported Rust must
  still pass rustc compilation and borrow checking before MIR export.  Verex
  will additionally check the resulting VIR's borrowing discipline so those
  facts can be consumed by the verifier.
- Accepting unsafe Rust in the initial safe subset.  Unsafe-related MIR is
  preserved or rejected with provenance first; checked raw-pointer semantics
  arrive in the later unsafe milestones.
- A complete model of Rust's unsafe memory semantics, concurrency, atomics,
  FFI, inline assembly, async/coroutines, trait objects, or arbitrary
  standard-library implementations in the first release.  Raw-pointer support
  is a planned staged extension, not an excluded architectural direction.
- Compiling the shared AST back into Rust machine code.
- Treating a Rust panic as a Move transaction abort.

## Architecture

```text
Verex source (selected profile)       Rust source
              |                            |
              | frontend elaboration       | rustc + exporter
              v                            v
          profile RawUnit          Rust-profile RawUnit
                    \                    /
                     +-- validation --+
                                     v
                              ValidatedUnit
                    |          |             |
                    |          |             +-- canonical Rust / Verex source
                    |          |                    |
                    |          |                    +-- Rust: recompile / re-import
                    |          |                    +-- Verex: re-elaborate
                    |          |                    +-- semantic comparison
                    |          +-- VIR relational meaning
                    |                 +-- spec / verify / summaries
                    |                 +-- kernel-checked theorem
                    +-- language-specific executable lowering
```

`VerexIR` is the common boundary.  Executable lowerings are backends of a
validated unit, not the semantic source of truth and not frontend targets:
source analysis, validation, verification, diagnostics, and source
generation never recover their meaning from a lowering or from retained
Lean syntax.

The shared VIR preserves names, scopes, places, and explicit semantic
operations while making structured control the proof-facing representation.
This is what makes it simultaneously a VerexSL and a source-generation
boundary.

## The structured shared VIR as a transpiler boundary

`VerexIR` is neither Lean syntax nor raw Rust MIR.  Both contain
frontend-specific administrative detail.  Its public construction boundary is
`RawUnit`; its only backend and semantic input is `ValidatedUnit`.  Between
them it is a normalized, typed language with:

- named declarations, parameters, locals, types, and generic parameters;
- explicit places (`local`, `deref`, `field`, `index`, and enum downcast);
- explicit moves, copies, borrows, reads, writes, calls, assertions, drops,
  and typed throws;
- structured sequencing, `if`, `match`, `loop`, `break`, `continue`, return,
  and argument-carrying throw;
- source origins, lexical scopes, and preferred user names in parallel
  metadata;
- a dialect/effect profile for semantic differences that syntax alone cannot
  express.

Each `RawUnit` owns exactly one compilation-unit `Tables` snapshot. All owned
Rust modules and dependency interface identities use that shared namespace,
name, type, lifetime, location, origin, and alignment space; namespaces do not
carry private copies. Function parameters are positional declarations matching
the leading function locals. Trait method parameters have no `LocalId` because
a method signature is valid without a body-local arena. Generic call and
constructor instantiations carry kinded `GenericArgument`s rather than an
assumed type-only list.

Raw MIR remains a CFG only in the imported `RawUnit`.  The Rust adapter
mechanically decodes it; `VerexIR.Validation.validate` performs the
administrative simplification, dominance/post-dominance and natural-loop
analysis, and constructs the structured VIR.  Rust MIR for the intended safe,
sequential subset is expected to be reducible often enough for this to be the
normal path; reducibility is checked for each import, never assumed from the
source language.

The exchange already has explicit raw statement forms for storage lifetime,
deinitialization, discriminants, retags, place mentions, user-type ascriptions,
and profile-defined administration, plus call/drop/assert terminators and
cleanup/unwind actions. Forms whose state or cleanup semantics are not yet
implemented are bounds-checked and rejected by the shared structurizer; they
are never erased during export.

### Selected Rustc Public MIR phase

The production input is the generic optimized MIR returned by Rustc Public's
ordinary `CrateItem::body` / `FnDef::body` query. On the pinned
`nightly-2026-07-23` implementation this reaches
`tcx.instance_mir(InstanceKind::Item(...))`, which selects `optimized_mir` for
ordinary functions while retaining the unspecialized generic body. This is an
explicit design choice: the exporter follows the public semantic interface
instead of querying a more source-like internal phase through `rustc_middle`.

Consequences are handled at the exchange boundary. Optimizer-created or
removed administrative detail is never assumed stable; every emitted artifact
records its toolchain, the mapper exhaustively preserves or rejects the public
MIR forms it sees, and compatibility corpus changes are reviewed before a
toolchain is promoted. Readable Rust is recovered from validated VIR, not by
trying to invert rustc's optimization choices.

The structurizer produces a witness connecting MIR blocks and edges to the
structured body.  `SwitchInt` regions become `if` or `match`; a natural-loop
header and backedge become `loop` or `while`; edges to that header become
`continue`; and edges to the loop exit become `break`.  Thus source printing
and verification consume a direct structured AST, not a CFG plus a secondary
presentation overlay.

The initial importer rejects a graph it cannot structurize precisely, with a
diagnostic at the relevant Rust origin.  A raw block/jump fallback is not part
of the shared VerexSL.  The raw CFG and the structurization witness
remain alignment material, useful for checking the translation and for
diagnostics but outside the central semantics.

The primary Rust round-trip property is approximately:

```text
semanticProjection(
  validate(importMir(rustc(printRust(validatedUnit)))))
  ≈α
semanticProjection(validatedUnit)
```

`importMir` constructs a fresh `RawUnit`; the ordinary validation pipeline
must accept it.  Alpha-renaming, compiler temporaries, and printer-only
grouping may differ.  If normalized VIR equality is unnecessarily strong, the
replacement obligation is a checked semantic equivalence over the Rust
profile.  Original Rust provenance remains associated with the first imported
unit, while generated Rust gets a `GeneratedSourceMap` back to its VIR nodes.

The corresponding Verex-source property is separate and optional:

```text
semanticProjection(validate(elaborateVerex(printVerex(validatedUnit))))
  ≈α semanticProjection(validatedUnit)
```

It is a source-backend test, not a route by which Verex source syntax derives
semantics or proofs.

This is deliberately a family of profile-selected target surfaces over one
core and one Verex surface language, not separate Verex Move and Verex Rust
dialects.  Implementing Rust-profile surface forms is deferred to a later
session.  A Rust operation can use a Move-profile construct only when its
integer, panic, drop, storage, and reference semantics match.  Otherwise the
one language needs an explicit profile-selected construct or primitive.  A
backend must never disguise different behavior merely to produce prettier
text.

## Rust source backend

The Rust-profile source backend consumes only `ValidatedUnit` plus the checked
dependency interfaces and profile configuration recorded with it.  It emits a
canonical Rust package or module set: readable `.rs` files, the edition and
compiler settings needed to reproduce the profile, and a source map from
generated ranges to VIR node IDs. It never reads original `.rs` tokens, macros,
or retained Lean syntax to fill a semantic gap. Comments already admitted in
the shared VIR provenance table may be rendered, but never supply semantics.

The printer prefers ordinary Rust constructs—bindings, assignments, `if`,
`match`, `loop`, `break`, `continue`, direct calls, structs, and enums—rather
than reproducing rustc temporaries.  It may choose different but canonical
surface spelling, such as a `loop` instead of a recovered `while`, explicit
match arms instead of a source `if let`, or the smallest valid `unsafe` block
around an unsafe VIR operation. It does not promise token-identical comment
attachment, macro, or control-sugar round trips.

Every printed construct has a profile rule.  In particular, the Rust backend
must preserve integer width and overflow configuration, target-dependent
`usize`/`isize`, panic behavior, drop order, visibility and signatures,
generic predicates, selected trait/impl calls, and unsafe operations.  It
emits an explicit diagnostic—not a lossy approximation—when the accepted VIR
node cannot yet be represented in standard Rust under the artifact's target
configuration.  An imported dependency not emitted as Rust source is an
explicit Cargo dependency or a declared external summary; it is never an
implicit printer assumption.

The backend gate is:

1. render the source and format it with `rustfmt`;
2. compile/check it under the emitted, pinned Rust configuration;
3. export its MIR through the same Rustc Public producer;
4. construct and validate a fresh `RawUnit`; and
5. compare profile-aware semantic VIR and source-map coverage with the input.

This gives users readable Rust generated from VIR while retaining one semantic
path: the generated crate comes back through the same MIR adapter and
`VerexIR.Validation.validate` as any other Rust crate.

## Small example and representation growth

Consider this current Verex Move function:

```lean
fun bump_if (flag : Bool) (value : U64) : Action U64 := do
  if flag then
    pure (value + 1)
  else
    pure value
```

Its structured body is:

```text
if flag then
  one := 1; sum := add value one; return sum
else
  return value
```

That shared body has two source parameters, two temporary locals, one
structured branch, and two value-producing instructions.  A canonical source
printer can therefore emit essentially the authored function above.

A mutable-reference example behaves similarly:

```lean
fun bump_ref (slot : &mut U64) : Action Unit := do
  slot := *slot + 1
```

The structured VIR is one sequence with four operations: `readRef`,
`loadInt 1`, checked `add`, and `writeRef`.  Its proof-facing meaning
expands:

```lean
fun slot =>
  withMutation slot fun mutation =>
    bind (pure mutation.read) fun old =>
    bind (checkedAdd old 1) fun new =>
    pure ((), mutation.write new)
```

The semantic growth
is real: every checked operation has success and exceptional behavior, and
every mutable borrow introduces a future value plus a reconciliation
condition.  Keeping `bind`, `withMutation`, and checked operations abstract,
then reasoning by weakest-precondition rules, keeps the generated term and
symbolic execution linear in straight-line body size.  Naively unfolding the
relations can duplicate continuations across normal, exceptional, and
undefined outcomes.

The remaining important growth modes are:

- each source conditional adds a structured branch linearly, but proof paths
  can be exponential in independent branches;
- loops add a structured fixed point, not source-sized unrolling, but need
  invariants;
- raw MIR adds storage markers, drop flags, cleanup edges, and bounds/overflow
  assertions that a semantics-preserving normalization pass must classify;
- VIR import deliberately keeps one generic body; interpreter instantiation
  supplies arguments and evidence without duplicating it. Any future native
  code-generation artifact is outside the VIR translation boundary;
- nested mutable loans add prophecy variables linearly, while alias or
  disjointness case splits can become combinatorial if borrow evidence is not
  used effectively.

## One semantic function interface

Every registered function is a declaration in a `ValidatedUnit`.  VIR semantic
elaboration exposes a small Lean-facing handle with the equivalent of:

```lean
structure FunctionArtifact where
  id          : FunctionId
  signature   : Signature
  body        : FunctionBody
  dialect     : Dialect
  sourceMap   : OriginMap
  alignment   : AlignmentStatus
```

`FunctionBody`, the signature, profile, origins, and alignment reference are
VIR data; the import receipt and its justification live in the checked import
envelope.  The function's relational meaning is derived only after the unit
passes `VerexIR.Validation.validate`; it is not selected by inspecting which
frontend produced the artifact, nor by re-walking retained source.

The dialect describes real semantic differences, not provenance.  For
example:

- Move aborts roll transaction state back; Rust panics do not.
- Move global storage is organized by resource family and address; Rust state
  may include allocations and interior-mutable cells.
- Move has abilities; Rust has ownership, destruction, traits, and lifetimes.

Two functions with the same body producer can therefore still live in
different semantic profiles.  Conversely, Lean and MIR frontends can produce
functions in one profile and receive identical verification treatment.

## One path for every frontend

Every frontend reaches the semantics the same way:

```text
Verex source ----+
Move source ------+--> RawUnit --> VerexIR.Validation.validate --> ValidatedUnit
Rust MIR exchange-+                                          |
                                                            v
                                           VIR semantic elaboration
                                      -> meaning / contracts / `verify`
                                      -> interpreter / executable backends
                                      -> Rust and Verex source backends
```

Verex source is a frontend: Lean elaboration parses, expands syntax, and
constructs a `RawUnit`, and no semantic pass reads retained syntax or Lean
environment extensions after raw-VIR construction.  `spec`, `verify`,
summaries, loop obligations, proof tactics, the interpreter, executable
lowering, and every source printer look up the same validated VIR
declaration.  There is no source-specific elaboration path and no fallback
to one when the VIR path lacks support.

## `verex-rust`: Rustc Public driver and raw-VIR boundary

The Rust frontend is one new Lake package, `verex-rust`, rather than separate
Rust-profile and MIR-transport projects.  It contains the Lean Rust-profile
library, its integration tests, and the Rust exporter source in a sibling
Cargo crate:

```text
verex-rust/
  lakefile.toml                 # requires ../verex-ir
  VerexRust/                    # profile registration, capability policy,
                                # RawUnit import façade, and tests
  rust-exporter/                # `verex-rust-export` Rust binary
    Cargo.toml                  # Rustc Public / rustc-driver dependencies
    src/main.rs                 # Cargo invocation and driver callback
    src/export.rs               # MIR -> Rust-profile RawUnit mapping
    src/json.rs                 # versioned VerexIR RawUnit encoder
  Tests/                        # Rust fixtures, JSON baselines, Lean checks
```

`VerexIR` owns the versioned, language-neutral `RawUnit` serializer.  The
Rust crate implements that schema rather than inventing a second wire format.
The generic receipt, freshness, and alignment rules are defined by the
[shared VIR import model](vir-design.md#import-receipts-and-source-alignment).
Lean owns the exporter invocation and consumes its detached output as
untrusted, normally private transport data.  The Rust-specific receipt binds
the rustc/exporter revision, selected MIR phase, crate graph, target and
semantic options, and asserted upstream type/borrow-check status.  The
resulting file is the only on-disk hand-off:

```text
Rust crate
  -> verex-rust-export / Rustc Public callback
  -> Rust-profile RawUnit JSON
  -> VerexIR RawUnit decoder
  -> VerexIR.Validation.validate
  -> ValidatedUnit
```

The production exporter runs a normal rustc analysis for the selected crate,
then, in its post-analysis callback:

1. queries public MIR, type, trait, instance, layout, and source information;
2. mechanically maps it to a Rust-profile raw CFG inside `RawUnit`;
3. serializes the complete, detached JSON artifact; and
4. returns `Compilation::Stop`, skipping MIR emission, LLVM code generation,
   object production, and linking.

It does not structurize control flow, decide Verex support, lower raw pointers
to references, replace unsupported library calls with magic primitives, or
construct `ValidatedUnit`.  No compiler handle, rustc-local index, or debug
rendering crosses the process boundary.  Unknown MIR forms are precise export
errors; an unknown RawUnit schema tag is a hard Lean decode error.

Rustc Public objects are callback-scoped, so the exporter must fully detach
the JSON before it returns.  The current documented Rustc Public startup path
does require `rustc_private` bridge crates, including `rustc_middle`, to start
the compiler.  That implementation detail is confined to `rust-exporter`;
the exporter makes no direct `rustc_middle` semantic queries.  If a needed
fact is unavailable through Rustc Public, the response is an upstream request
or a narrowly labelled temporary experiment adapter, not an untracked private
query in the production mapper.

### Selected MIR phase

The phase is the generic optimized MIR of the public body query (above),
bound in the import receipt; no private rustc query is part of the Verex
contract. Explicit places, bounds and overflow checks, drops, cleanups, and
source locations survive that phase, which is what the proof-relevant model
needs. Charon ULLBC is acceptable as a comparison or experiment input, but
not as the production definition of this phase.

### Rust-profile `RawUnit` payload

The generic exchange and receipt schema belongs to VIR.  The Rust payload adds
only the source facts needed to interpret MIR:

- declarations and dependencies: canonical item/type/trait/impl identities,
  trait/implementation definitions and associated-item bindings, ADT layouts
  as required by the profile, generic predicates, and direct
  callee/implementation-selection evidence;
- raw Rust CFG bodies: typed locals, places and projections, basic blocks,
  statements, terminators, constants, drop flags/cleanup information, and
  explicit source origins; and
- multi-file provenance: ranges, expansion chains, and enough origin data to
  distinguish user code from compiler-generated code.

Raw pointers, pointer casts, `UnsafeCell`, unions, and calls marked unsafe
have first-class raw-VIR tags from v1 even when the first semantic profile
rejects them.  That preserves the Rust input truthfully, gives actionable
diagnostics, and prevents a future unsafe frontend from inventing a new import
format.  A profile must explicitly declare whether it supports each tag.

A successful rustc type and borrow check is required upstream admission
evidence, but it is not the only proof-facing justification for borrowing.
Verex runs its own borrow checker on validated VIR and exposes its checked
facts or certificate to the verifier.  Those VIR rules may deliberately accept a broader abstract
language than rustc; imported Rust remains limited to programs rustc accepts.
A trust report is part of every theorem that claims to apply to the imported
Rust program.

### Invocation, distribution, and caching

`verex-rust-export` is a special Verex tool called by the Lean/Lake build
driver, not a Cargo plugin that every user installs or compiles.  For now it
runs against a Verex-managed, pinned nightly toolchain with matching
`rustc-dev` and LLVM libraries.  Verex setup either ships that paired driver
and sysroot or downloads them into a versioned local cache.  An optional
environment/configuration override permits development against another
explicit toolchain.

Users continue to use their ordinary stable `cargo` workflow.  The export tool
performs a separate `cargo check`-like analysis only when Verex imports a
Rust crate.  It stops the selected root crate after extraction; dependencies
are prepared as normal check/metadata artifacts so Cargo still receives the
artifacts it expects.  The cache key includes the manifest and lockfile,
sources, features, target, rustc/exporter revision, and semantic profile.  If
Rustc Public later provides a stable distribution surface, Verex may revisit
how it delivers this managed tool; that does not change the RawUnit or
validation boundary.

### Why Charon is not the boundary

[Charon](https://github.com/AeneasVerif/charon) is highly relevant prior art.
It serializes both ULLBC (simplified, unstructured MIR) and LLBC (its
structured form), and it covers crate-level items such as functions, traits,
implementations, type definitions, and globals.  That makes it very useful
for early fixtures and differential tests.

But Charon is intentionally more than extraction: it simplifies MIR and
performs control-flow reconstruction through transformation passes.  It also
requires a custom driver compiled with a pinned nightly.  Those are good
design choices for Charon and Aeneas, but Verex needs the raw CFG as evidence
and must validate its own structurization into the shared IR.  Importing LLBC
would duplicate a proof-relevant transformation outside our checked boundary;
importing ULLBC would still make a separately versioned driver a permanent
compatibility gate.  We keep the interoperation option without making either
choice authoritative.

## Control flow and loops

Raw MIR represents every loop as CFG backedges, but it need not impose that
form on the shared language.  The importer recovers a structured body from a
reducible MIR CFG:

```text
validate MIR CFG
  -> simplify administrative control
  -> compute dominators, post-dominators, and natural loops
  -> recover selections and loop regions
  -> structured VIR (`if`, `match`, `while` / `loop`, `break`, `continue`)
```

Selection headers and their joins recover `if` and `match`.  A natural-loop
header and backedge recover a `while` or `loop`; its header edge is
`continue`, and its exit edge is `break`.  The structurizer records a checked
correspondence from these regions back to the MIR blocks and edges.  Its
alignment obligation is that the MIR CFG and recovered structured VIR have the
same outcomes for every supported input.

Reducibility is an import condition, not an assumed universal Rust property.
The initial frontend rejects a function whose MIR cannot be structurized under
the supported rules, rather than exposing general `goto`/basic-block syntax in
Verex or using an approximate proof rule.  This keeps user-facing loop
invariants, source generation, and WP reasoning over the structured loop node
rather than over raw block numbers.

The first useful slice matches the currently supported Move-shaped surface:
function definitions, structs, enums, ordinary references, selections, and
reducible loops.  Cleanup-only and unwind control shapes remain outside that
slice.

The main exceptional case is cleanup.  The initial profile fixes
`panic=abort`: panic/abort is a terminal Rust exceptional exit with no
unwinding or recovery, and it does not gain Move transaction rollback
semantics.  Full unwinding later needs structured cleanup scopes and explicit
drop behavior; it does not require making CFG the central calculus.

Loop invariants attach to imported structured-loop identities, not Rust block
numbers.
A stable loop identity should combine:

- the stable function identity;
- the loop nesting/discovery path;
- its source origin;
- a semantic fingerprint used to detect stale annotations.

The proof UI can expose names such as `Crate.sum.loop0` while retaining the
original Rust loop span for diagnostics.

## Types and operations

The shared AST owns the known Move/Rust union directly. The first Rust subset
uses core booleans, fixed-width integers, tuples/products, structures, enums,
vectors, references/lifetimes, abilities/traits, direct and closure calls,
constructor/destructor calls, branches, loops, and throws. Rust-only does not
mean profile-opaque; it means a Move backend may diagnose that valid node as
unsupported.

Important core-union additions or refinements include:

- `usize` / `isize` with target-dependent width;
- fixed arrays as length-indexed core vectors, plus an explicit plan for
  slices and other dynamically sized types;
- initialized versus moved/uninitialized places;
- partial moves out of aggregate fields;
- explicit destruction and drop glue;
- Rust overflow, bounds-check, and panic operations;
- references occurring recursively inside tuples, structs, and enums;
- modeled library types and operations rather than accidental dependence on
  their unsafe implementations.

Operations should carry their actual effect.  A generic `call` may name a
shared function identity, but panic, resource access, cell access, and Move
transaction abort must remain distinguishable in the semantics.

## Generics, core traits, and the specialization boundary

Importing only monomorphized MIR would produce one Verex body per concrete
Rust instance. Proving every copy independently is neither necessary nor a
good default, and it would prevent the VIR interpreter from operating over the
source program's generic structure. The import contract therefore requires
one generic body for each generic Rust item. `RawUnit` and `ValidatedUnit`
never contain a monomorphization manifest or a cloned concrete body as their
semantic representation. A Rustc Public API that exposes only a substituted
instance is insufficient for the supported item and blocks the exporter spike.

A concrete call can still carry resolved implementation-selection evidence.
That evidence binds an already generic trait call to a validated implementation
dictionary; it is not body cloning or monomorphization. The interpreter
instantiates the generic body with its supplied type/const/lifetime arguments
and evidence values, preserving one VIR declaration and meaning.

The desired separation is:

```text
generic Rust MIR + predicates
             |
             v
generic shared function + explicit trait evidence
       |                              |
       | one parametric proof         | interpreted instantiation
       v                              v
generic theorem                 concrete call result
```

Traits are a language-neutral core-VIR facility, not part of the Rust profile:
a future Move trait feature must use the same trait declarations,
implementations, associated items, evidence arguments, interpreter behavior,
and proof contracts. The shared IR needs enough structure to state and invoke
a generic body, but it does not need to reproduce rustc's full trait solver.
rustc has already checked coherence and well-formedness and selected concrete
implementations at use sites. The importer can preserve:

- type, lifetime, and const parameters;
- `where` predicates and higher-ranked binders in the supported fragment;
- stable trait and associated-item identities;
- associated-type and associated-constant bindings;
- explicit evidence parameters for unresolved generic trait obligations;
- concrete impl witnesses for resolved call sites.

The first exporter/codec slice does not gate on implementation-selection or
proof evidence. It preserves one generic body, core trait declarations,
predicates, associated items, and impl bindings; `EvidenceId` remains opaque
until the checked dictionary table and its interpreter/proof rules are added
in a later milestone.

Operationally, an evidence parameter is dictionary passing even if no runtime
dictionary is emitted.  For example:

```rust
trait Step {
    fn step(&mut self);
}

fn step_twice<T: Step>(value: &mut T) {
    value.step();
    value.step();
}
```

has the schematic shared form:

```text
step_twice<T>(stepEvidence : StepEvidence<T>, value : &mut T):
  call stepEvidence.step(value)
  call stepEvidence.step(value)
```

Rust traits provide method types, not kernel-checked behavioral laws.  A
verification interface must therefore attach contracts separately:

```text
StepContract<T>(evidence)             -- contract for one `step`

step_twice_verified:
  forall T evidence,
    StepContract<T>(evidence) ->
    Satisfies (step_twice<T, evidence>) stepTwiceContract
```

Each concrete `impl Step for Counter` proves `StepContract` once.  Every use
of `step_twice::<Counter>` then obtains the generic theorem by instantiation;
it does not reverify the two calls.  In generated Verex source, Lean type
class syntax is a natural presentation for the evidence parameter and a
separate `LawfulStep`-style class can carry the method contract.  The shared
IR should nevertheless use language-neutral evidence records rather than
depending on Lean instance search.

The interpreter's generic-instantiation law has the form:

```text
interpret(body; types, impls, consts)
  = instantiate(semantics(body), types, impls, consts)
```

Together with validated implementation-selection evidence, this theorem
connects the single generic proof to each concrete call. It is a generic
interpretation theorem, not a new user proof per instance. A future separate
native code-generation backend may additionally prove that any concrete
artifact it produces refines this generic interpretation; that backend is not
allowed to replace the generic VIR body.

Full Rust trait expressiveness should be staged rather than admitted as one
indivisible feature:

- the initial trait slice supports methods, associated types, and trait
  inheritance;
- ordinary static method bounds become evidence parameters;
- associated types can initially become explicit type parameters plus
  equality constraints after rustc normalization;
- associated constants become evidence fields and may require symbolic
  reasoning;
- higher-ranked lifetime bounds require quantified evidence and preservation
  of the relevant region relationships;
- `dyn Trait` needs existential values and runtime vtable dispatch and is a
  separate feature;
- specialization, layout-dependent operations, `TypeId`, `size_of`, const
  generics that affect control flow, and type-specific drop behavior may
  require specialized proof cases.

There are two useful fallbacks when a fully parametric proof is impossible:

1. group concrete call configurations by implementation contract, layout
   facts, and type-level constants, then prove one representative per
   equivalence class;
2. prove a deliberately finite set of argument/evidence configurations and
   attach a checked coverage certificate describing exactly which calls they
   cover.

Neither fallback permits cloned, monomorphized bodies into `RawUnit` or
`ValidatedUnit`.

The current Move path already provides partial precedent: generic Verex
functions are exported as one parametric declaration. The Rust design preserves
that separation and uses per-instance verification only for behavior that is
genuinely type- or implementation-dependent; such a view is derived from,
never substituted for, generic VIR.

## Ordinary references

Ordinary references use the prophetic ownership model of validated VIR
([`prophetic-references.md`](prophetic-references.md)). A shared reference
is an observation; a mutable borrow is a runtime value owning the loaned
content, the lender keeps a hole, the loan's certified death writes the
current value back, and a dying frame exports unreconciled loans through
the state's pending set; the prophecy is the contract-level name of that
export.

The two requirements Rust adds are covered by construction:

1. reference-bearing values such as `Option<&mut T>`, `&&T`, and structures
   with lifetime parameters need nothing special: borrows nest as values
   nest, and reborrowing threads the outer borrow's current through the
   inner loan;
2. signature regions relating returned references to inputs are consumed by
   the borrow analysis's result-source selection, and the dynamic
   realization is uniform: `applyPending` at the call boundary delivers a
   callee's exported finals wherever their holes sit, so even
   `pick`-style dynamic write-back targets need no path bookkeeping in
   specifications.

The borrow certificate remains the license: a program the analysis cannot
certify never reaches the prophetic model, and unsafe constructs are
rejected rather than approximated (below).

## Unsafe Rust and raw pointers

Unsafe Rust is an architectural requirement, not a reason to model raw
pointers as ordinary references or to accept undefined behavior silently.
Rust's `unsafe` marker permits operations for which the compiler does not
establish memory-safety conditions; raw pointers may be null, dangling, or
aliased in ways that references may not.  Verex must retain that distinction
from import through theorem statement.

The shared Rust profile therefore introduces a separate pointer-and-memory
domain.  Schematically, a raw pointer carries an allocation/provenance token,
byte offset, mutability, and metadata for dynamically sized pointees; memory
records allocation liveness, size, alignment, initialized bytes, typed views,
and ownership/alias permissions.  It is intentionally not represented as
`Mutation T` or `&mut T`:

```text
RawPtr T = { provenance, allocation?, offset, metadata, mutability }
Memory  = allocations + byte/init state + typed access/permission state
```

The exact provenance model is a versioned Rust-profile choice.  The first
unsafe profile uses a strict, allocation-based model: a pointer derived from a
live allocation may be offset and dereferenced only within the allocation with
the required alignment, initialized range, type/layout, and current access
permission.  Integer-to-pointer casts, exposed provenance, arbitrary address
fabrication, pointer tagging, packed/unaligned accesses, and pointer-sized
layout tricks are initially rejected.  This is deliberately narrower than
arbitrary unsafe Rust, but it makes the first safety obligations concrete.

Every raw load, store, offset, cast, union field access, mutable-static access,
unsafe call, and FFI/assembly boundary is an explicit shared operation.  Its
semantics can return normal, panic, divergence, or **undefined behavior**.
Undefined behavior is neither a Move abort nor a Rust panic and has no final
state that a functional postcondition may rely on.  A default verified
contract for an imported Rust function must establish absence of UB for all
executions admitted by its precondition, then establish the normal/panic
behavior.  A theorem that omits the no-UB obligation is labelled as a
conditional model theorem, not as a memory-safety result about Rust.

Unsafe functions, unsafe trait implementations, and unsafe blocks expose
extra safety conditions at different modular boundaries.  The Rust frontend
will map them to explicit safety contracts: an unsafe function exports caller
obligations; a verified unsafe block proves the operation-specific obligations
from its enclosing precondition, invariants, and path conditions; an unsafe
implementation proves the trait's safety law.  Safety comments are useful
provenance but are not proof evidence.  If Rustc Public cannot provide precise
unsafe-scope origins, the importer points at the unsafe MIR operation and
records this diagnostic limitation rather than fabricating a source boundary.

The stages are intentionally separate from safe-Rust support:

| Stage | Accepted semantics | Explicit exclusions |
|---|---|---|
| U0 | Serialize and diagnose every unsafe-related MIR form. | No unsafe operation enters validated semantics. |
| U1 | Typed raw pointers derived from live local/heap allocations; checked load/store and simple offsets; safety preconditions. | Integer-derived pointers, unions, FFI, atomics, inline assembly, arbitrary `UnsafeCell`. |
| U2 | Allocation identities, deallocation/reallocation, `Box`/`Vec`-class models, and selected `UnsafeCell`-based library abstractions. | General shared mutation, custom allocators, concurrency. |
| U3 | A profile-specific account of richer provenance, unions, FFI/ABI, atomics, and concurrency. | Any feature without a stated model and alignment argument. |

U1 follows drop, initializedness, and the basic reference model; it is not a
shortcut around them.  Each stage adds corpus tests checked against the pinned
rustc behavior and, where applicable, Miri as a bug-finding differential test.
Miri is not a proof oracle and cannot by itself establish absence of undefined
behavior.

## Interior mutability and `RefCell`

`RefCell<T>` is not simply another prophecy borrow.  It permits mutation
through shared aliases and checks borrow exclusion dynamically.  It may be
stored inline on the stack or inside another owner; heap allocation is not its
defining property.

An abstract single-threaded model can use stable cell identities and a typed
store:

```text
CellState T = {
  value  : T,
  borrow : Unborrowed | Shared Nat | Exclusive
}
```

- `borrow` increments the shared count unless an exclusive guard is live;
- `borrow_mut` enters `Exclusive` only from `Unborrowed`;
- a conflicting operation panics or returns an error, according to the API;
- dropping `Ref` / `RefMut` guards releases the corresponding state;
- forgetting a guard leaves the dynamic borrow outstanding.

This requires explicit identity-bearing state and observable drop behavior.
`Rc<RefCell<T>>` is a later extension that additionally requires allocation
identity and shared ownership/refcount semantics.

## Outcomes and effects

The common relational core should distinguish at least:

- normal return with result and final state;
- language-specific exceptional exit;
- undefined behavior, with no usable final state;
- divergence, handled initially as partial correctness.

Unsupported source features are rejected during import/validation; they are not
an execution outcome.  This distinction prevents an unsupported raw-pointer
operation from being mistaken for a verified-but-undefined program path.

Move instantiates exceptional exit as transaction abort with rollback.  Rust
instantiates it as panic, where prior mutations remain.  The first Rust slice
uses `panic=abort`: panic is terminal, cleanup/unwinding and `catch_unwind` are
unsupported, and the imported artifact records that choice.

The existing `Spec` shape can be generalized, but Rust panic must not be
encoded by reusing Move rollback behavior under another name.

## Specifications

The `spec` command resolves a function declaration in a `ValidatedUnit` and
constructs a contract from its VIR signature and logical parameter view.  It
must not inspect body provenance or retained frontend syntax.

For example, the same form should be valid for a Rust-MIR or Verex-source
frontend declaration after both enter the same validated unit:

```lean
spec Crate.increment (value : &mut U64) where
  ensures value = old(value) + 1

verify Crate.increment
```

As in Verex Move, mutable parameters are presented by referent value:

- `old(value)` denotes the initial referent;
- `value` in a normal postcondition denotes the final referent;
- prophecy carriers remain internal to generated semantics and call rules.

The contract core should share `requires`, normal `ensures`, and framing.
Language-specific exceptional clauses may provide `aborts_if` for Move and
`panics_if` for Rust while lowering to a common exceptional-outcome predicate.

Data invariants and external function models also attach to registered type or
function identities, independent of frontend provenance.

## Proofs and modular calls

For each function `f`, the system exposes the equivalent of:

```lean
f.semantics : FunctionMeaning f.signature
f.contract  : Prop
f.verified  : f.contract
```

`verify f` proves contract satisfaction over `f.semantics`.  Tactics execute
or normalize the shared internal operations but present goals using logical
parameter names and source-level models, not positional local and block IDs.

A proved callee exports a modular summary.  Callers use that summary rather
than unfold its body, regardless of whether caller and callee came from Lean
or MIR.  This permits, within a compatible dialect and type/effect boundary:

- a Verex-source function calling an imported Rust function;
- an imported function calling a Verex-source model;
- imported functions verified against other imported summaries;
- opaque external functions with explicitly trusted or separately proved
  contracts.

Every final theorem should report any trusted external summaries on which it
depends.

## Alignment and theorem meaning

The [shared VIR model](vir-design.md#import-receipts-and-source-alignment)
defines the distinction between a proof about validated VIR and a claim about
its source.  Rust adds one temporary assumption: the pinned Rustc Public
exporter preserves the selected MIR body when constructing `RawUnit`.
Successful rustc type and borrow checking is upstream admission evidence, not
a proof of that translation.  M7 replaces the trusted-exporter assumption with
checked translation/refinement results and composes them with `f.verified`.

## Source provenance and diagnostics

VIR carries the basic pieces: named locals include an optional source name,
and instructions and terminators carry optional authored spans.

The current `SourceSpan` is a half-open byte range in one implicit file.  Rust
requires a multi-file provenance table and macro/desugaring context:

```lean
structure SourceRange where
  file  : SourceFileId
  start : Nat
  stop  : Nat

structure Origin where
  primary     : Option SourceRange
  related     : Array SourceRange
  expansion   : Array SourceRange
  generatedBy : Option PassName
  parent      : Option OriginId
```

Origins live in a parallel map keyed by stable AST node IDs and are excluded
from semantic equality.  Lowering passes obey these rules:

- one source node to many target nodes: all inherit the source origin;
- many source nodes to one target node: retain a primary and related origins;
- synthetic code: identify the generating pass and nearest parent origin;
- inlining: retain call-site and callee-definition origins;
- missing provenance: fall back to the enclosing function.

Translation, lowering, and tactic failures can then report Rust file, range,
and expansion context.  A normal Lean diagnostic is still anchored in the
active `.lean` proof file; underlining an external `.rs` document requires an
LSP bridge or editor integration, but it does not require a different semantic
design.

The Rust source backend emits a `GeneratedSourceMap` from ranges in the
canonical `.rs` files to VIR node IDs and their original origins.  Re-import
records the reverse association.  The Verex source backends provide the same
mapping for generated Lean files.  Diagnostics can therefore show the readable
generated construct while still choosing the original Rust range as the
primary location.  Formatting or regenerating a derived file must not change
semantic identities.

## Rust-specific freshness inputs

The shared import gate owns freshness enforcement.  Its Rust receipt/cache key
additionally includes the manifest and lockfile, source tree, crate graph,
features, target, rustc/exporter revision, selected MIR phase, and semantic
options.  Stable item, local, and loop identities keep proof maintenance
predictable; a changed key regenerates the private exchange before validation.

Proofs should normally use contracts, named logical values, loop invariants,
and high-level stepping tactics.  Proofs that unfold raw MIR blocks,
structurization evidence, or compiler temporaries are allowed to be brittle
across importer changes.

## Deferred-work register

Rust-specific entries mirrored from the core VIR design; an item stays here
until it lands, is rejected, or is superseded by a recorded decision.

| Deferred item | Reactivate by |
|---|---|
| Approval and publication of RawUnit JSON v1 as an external compatibility contract | M0, before fixtures beyond the supported mirror become compatibility artifacts |
| Public function predicate query (function `where` clauses and const-function binder types are not exposed by Rustc Public) | M0; filed upstream as [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892); public queries are requested rather than reading `rustc_middle` |
| Remaining MIR-to-RawUnit mapper forms: symbolic const-dependent types, destructive/provenance administration exposed by Rustc Public, constrained and const generic functions | M1 once their metadata is public; M4 adds cleanup and effectful drop glue |
| Checked implementation-selection evidence and dictionary interpretation (`EvidenceId` is opaque) | M2, before trait method calls execute or transfer contracts and proofs |
| Destructive/provenance nodes, cleanup edges, pointer-alignment assertions, and reachable `unreachable` blocks, rejected by structurization | M4 for destruction, cleanup, and richer panic behavior; M5 for retag/provenance and alignment assertions |
| Drop flags, cleanup scopes, and unwinding (only `panic=abort` with unreachable-unwind drops is structurized) | M4 drop effects and flags; full unwinding is a separately approved milestone |
| A proof-grade semantic-body/alignment receipt (the cache digest binds inputs, not alignment) | the shared proof-facing receipt gate; M7 before source-level theorem claims |
| Raw pointers, unsafe operations, allocation/provenance, and UB (rejected at the first unsupported node, no partial JSON) | M5, U0–U2 |
| Trait objects, specialization, higher-ranked bounds outside the initial fragment, async/coroutines, FFI, inline assembly | separately scoped post-M2/M5 milestones with core/profile classification first |
| Library models (`Box`, `Rc`, slices, `RefCell`, and peers) | M6 or a separately approved library-model milestone |

## Proposed milestones

### M0 — Rustc Public exporter spike and import contract

- Create the `verex-rust` Lake package and its `rust-exporter` Cargo crate.
  The Lean package depends only on `verex-ir`; the Rust crate contains all
  Rustc Public/rustc-driver bootstrap dependencies.
- Freeze RawUnit JSON v1, including the language-neutral generic,
  trait/implementation and associated-item schema, plus the
  Rust-profile capability vocabulary. Add fixtures for scalar arithmetic,
  enums, loops, direct calls, a generic trait method call, ordinary borrows,
  drops, a raw-pointer operation, and an unsupported operation.
- Implement the `verex-rust-export` driver against the pinned Rustc Public
  toolchain.  It must serialize a detached Rust-profile `RawUnit` in its
  post-analysis callback and return `Compilation::Stop`.  The mapper makes no
  direct `rustc_middle` queries; startup-only bridge dependencies stay confined
  to the driver crate.
- Demonstrate the selected post-analysis optimized generic body shape, source
  origins, generic predicates, direct callee identity, drop/cleanup data, and
  raw-pointer tags on that corpus.  Any absent query is a documented blocker
  or an upstream API request.
- Decode the RawUnit documents in Lean and verify schema version, enum
  exhaustiveness, and source tables before validation. The receipt may record
  Rust-specific inputs, but shared freshness enforcement is not an M0 gate.

Gate: the exporter writes deterministic RawUnit JSON and exits before codegen
or linking, and a generic trait fixture has exactly one generic body with core
trait declarations and predicates. Checked trait-selection evidence and
import-receipt enforcement are deferred from this first gate.

### M1 — Raw MIR ingestion, provenance, and structured control

- Complete the MIR-to-RawUnit mapper for types, named locals, places, basic
  blocks, direct calls, source origins, and the information needed to
  structurize reducible control.
- Register the `VerexRust` profile and decode the generic RawUnit serializer
  through `VerexIR.Validation.validate`.
- Validate the structurization witness and reject unsupported control shapes
  before constructing the shared body.
- Import generic predicates, core trait/implementation declarations,
  associated-item bindings, and implementation-selection evidence without
  cloning a generic body.
- Differential-test the raw CFG and structurization result against Charon
  ULLBC/LLBC fixtures, without consuming Charon in the production path.

Gate: fixture crates reach `ValidatedUnit` exclusively through serialized
RawUnit JSON; neither the Lean decoder nor the driver has a frontend-specific
validation or reporting path.

### M1.5 — Lean build integration and managed toolchain

- Add a Lean/Lake driver command that invokes `verex-rust-export` for a Cargo
  manifest, package, feature set, and target, then registers the validated
  unit for proof and source-backend consumers.
- Define the tool cache and artifact location from source/Cargo-lock hashes,
  features, target, profile, exporter, and rustc revision.
- Implement the Verex-managed exporter/sysroot installation plus an explicit
  developer override.  Normal users must not install nightly, `rustc-dev`, or
  the driver with `cargo install`.
- Run dependencies with ordinary check/metadata artifacts and stop only the
  extracted root crate after analysis.

Gate: a clean checkout imports a fixture through one Verex setup/build command
without relying on the user's Rust toolchain, and a repeated build is cached.

### M2 — Basic semantics and contracts

- Support scalar/ADT operations, assignments, direct calls, structured
  selections, and recovered reducible loops without custom destruction.
- Generate function meanings only from validated VIR bodies.
- Verify generic bodies parametrically over trait evidence and method
  contracts for the initial methods, associated-types, and trait-inheritance
  subset.
- Emit canonical Rust source for the accepted scalar/ADT/control subset and
  check the `Rust → MIR → RawUnit → ValidatedUnit` semantic round trip.
- Exercise Verex source printing separately as another validated-VIR backend.
- Attach the existing-style `spec` / `verify` interface.
- Export modular summaries and source-positioned proof obligations.

### M3 — Ownership and ordinary references

- Model moves and initializedness.
- Implement the VIR borrow checker, migrating applicable rules from the older
  translation, and expose its checked loan/lifetime facts or certificate to
  the verifier.
- Retain rustc's successful borrow check as upstream admission evidence and a
  differential oracle, not as the semantic authority for VIR borrowing.
- Reuse and extend prophecy semantics for `&T`, `&mut T`, reborrows, calls,
  and returned references.
- Add positive and negative agreement tests against Rust execution.

### M4 — Drop and panic

- Add explicit destruction and drop-flag behavior.
- Implement the fixed `panic=abort` terminal semantics; do not support
  unwinding or `catch_unwind` in this slice.
- Ensure guard and destructor effects are visible to proofs.

### M5 — Unsafe import and checked raw pointers

- Complete U0: preserve all unsafe-related MIR constructs and reject each
  unsupported semantic form with its Rust origin.
- Define the strict allocation/provenance memory profile and the explicit UB
  outcome; add no-UB safety obligations to the contract interface.
- Implement U1 typed raw-pointer creation, checked loads/stores, simple
  offsets, and modular unsafe-function contracts.
- Compare defined fixture behavior with rustc execution and use Miri only as a
  differential bug-finding test.

### M6 — Reference-bearing values and interior mutability

- Support references nested in products and ADTs.
- Preserve signature region relationships at modular boundaries.
- Add an abstract `RefCell` store, dynamic borrow rules, and guard drop.
- Treat `Box`, `Rc`, slices, and broader library models as separately scoped
  extensions, staged with U2 where they require allocation semantics.

### M7 — Alignment

- State and prove the generic core-VIR semantic results.
- Validate or prove refinement from the MIR exchange through `RawUnit` to the
  supported `ValidatedUnit` fragment.
- Establish semantic equivalence for the
  `ValidatedUnit → Rust source → MIR → RawUnit → ValidatedUnit` round trip,
  and separately for supported Verex source backends.
- Prove that generic type/const/implementation-evidence instantiation preserves
  the function meaning, so generic theorems transfer to concrete calls. Any
  later native specialization must be a separate refinement of this result.
- Compose import alignment with `f.verified` into an explicit theorem about
  the imported Rust MIR body.

## Open design questions

1. Which Rustc Public capability gaps, if any, block the M0 artifact, and
   which should be upstreamed before any temporary adapter is considered?
2. What stability expectations and promotion thresholds should the rolling
   corpus enforce when optimized generic MIR changes between rustc revisions?
3. Which library types receive primitive models in the first useful subset?
4. How are stable loop identities maintained across harmless MIR changes?
5. Which type-observing operations force specialized proof representatives
   instead of the generic theorem?

## Success criterion

For a supported Rust function, the project should be able to:

1. compile and borrow-check the Rust crate with a pinned configuration;
2. decode its MIR and provenance to `RawUnit`, then validate it through the
   single VIR pipeline;
3. emit readable canonical Rust source from the resulting `ValidatedUnit`;
4. recompile that source and recover an equivalent validated Rust-profile
   body through the same MIR importer;
5. preserve a generic body and its trait obligations rather than requiring a
   separate proof for every concrete use;
6. interpret the resulting VIR for a Rust fixture and obtain the same
   observable result as the Rust execution.

Later verification sessions extend this to:

7. attach a normal Verex-style contract to the registered function;
8. prove that contract with the common verification interface;
9. transfer that theorem to concrete calls through validated evidence, without
   replacing the generic VIR body;
10. report failed obligations at their original Rust locations;
11. state exactly which alignment theorem or trusted importer assumption makes
   the result a claim about the Rust MIR body.

When these steps hold, the provenance of a function body no longer affects
how it is specified or proved.  It affects only how its meaning is justified
and where diagnostics are reported.
