# Leaner Rust frontend through the shared Leaner IR

Status: living design and frontend decision

## Decision

Leaner will build its Rust frontend in a new `leaner-rust` package around a
small, project-owned exporter using
**[Rustc Public](https://rust-lang.github.io/rustc_public/getting-started.html)**
(the project formerly called Stable MIR).  The exporter maps in-memory MIR to
the versioned, Rust-profile `RawUnit` exchange format owned by `LeanerIR`,
serializes it, and stops the compiler before code generation.  It will not use
Charon as the production import boundary and it will not introduce a second
on-disk MIR format.

The exporter owns only extraction, mechanical MIR-to-raw-LIR mapping, and
serialization.  `LeanerIR` owns the exchange schema, validation, CFG
simplification, structurization, semantic interpretation, canonical
Rust/Leaner source printing, and all proof-facing diagnostics.  This keeps the
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

Rustc Public is the best long-term fit because Leaner wants the compiler's
actual typed, borrow-checked Rust input—not a verifier-oriented dialect that
we must adopt as an additional source language.  It lets us track the Rust
toolchain deliberately, preserve raw-pointer and other low-level MIR
operations for a later unsafe model, and choose Leaner's own structured IR and
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
another semantic boundary before LeanerIR, and its support cadence becomes a
constraint on Leaner's Rust-at-tip goal.  It is an excellent validation aid,
not the right owner of Leaner's source-of-truth semantics.

| Dimension | Rustc Public exporter (chosen) | Charon |
|---|---|---|
| Rust compatibility | Directly interrogates the designated rustc revision; a new MIR form can be preserved or rejected by Leaner. | Adds a custom driver and its own pinned nightly compatibility window. |
| Input representation | Raw, typed MIR-shaped CFG; Leaner validates every simplification and structurization. | ULLBC simplifies MIR; LLBC additionally reconstructs structured control flow. |
| Leaner proof boundary | One project-owned exchange and one checked LeanerIR path. | Mature analysis IR, but its transformations become an extra boundary or must be duplicated. |
| Unsafe growth | Preserves the compiler's pointer/unsafe operations in v1 even before Leaner models them. | Useful prior art and has unsafe-code support, but Leaner would inherit a second IR's operation choices. |
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
it or reject it at a precise source location.  It does *not* mean that Leaner
claims verification coverage for every standard-library implementation,
platform ABI, unsafe operation, optimizer transformation, or target.  The
artifact's declared profile, capability report, and theorem assumptions define
the actual coverage.

## Summary

This project imports borrow-checked Rust MIR into the language-neutral
`LeanerIR` boundary used by every Leaner frontend.  Leaner source frontends and
the Rust MIR adapter construct profile-tagged `RawUnit`s; the single
`LeanerIR.Validation.validate` pipeline produces the `ValidatedUnit` consumed
by common semantics, contracts, verification, executable lowering, and source
backends.  No frontend produces a separate proof-facing function meaning.

The Rust source printer is a required backend, not an attempt to recover the
original Rust tokens.  For a supported Rust-profile unit it emits readable,
canonical standard Rust (`.rs`), which rustc must re-import to an equivalent
validated LIR body.  The profile-selected Leaner source backend remains useful
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
reject valid LIR they cannot represent. Profile extension values are reserved
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
- Converge Leaner-source and Rust-MIR frontends on `RawUnit`, then on one
  validated, typed, named LIR unit.
- Reproduce imported Rust as readable, canonical Rust source from validated
  LIR, and make the profile-selected source printers available to every
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
- Making canonical generated Leaner source textually round-trip to the
  original `.rs` file.
- Exact round-tripping of Rust syntax such as `for`, `?`, methods, and pattern
  syntax after MIR lowering.
- Reimplementing rustc's type checker or trait solver.  Imported Rust must
  still pass rustc compilation and borrow checking before MIR export.  Leaner
  will additionally check the resulting LIR's borrowing discipline so those
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
Leaner source (selected profile)       Rust source
              |                            |
              | frontend elaboration       | rustc + exporter
              v                            v
          profile RawUnit          Rust-profile RawUnit
                    \                    /
                     +-- validation --+
                                     v
                              ValidatedUnit
                    |          |             |
                    |          |             +-- canonical Rust / Leaner source
                    |          |                    |
                    |          |                    +-- Rust: recompile / re-import
                    |          |                    +-- Leaner: re-elaborate
                    |          |                    +-- semantic comparison
                    |          +-- LIR relational meaning
                    |                 +-- spec / verify / summaries
                    |                 +-- kernel-checked theorem
                    +-- language-specific executable lowering
```

`LeanerIR` is the common boundary.  The existing
[`Move.Compiler.LIR`](../move/Move/Compiler/LIR.lean) is the named stackless IR
(NSIR) executable backend, not the semantic source of truth and not a
frontend target.  A Move-profile validated unit may lower to NSIR, but source
analysis, validation, verification, diagnostics, and source generation never
recover their meaning from NSIR or from retained Lean syntax.

The shared LIR preserves names, scopes, places, and explicit semantic
operations while making structured control the proof-facing representation.
This is what makes it simultaneously a Leaner language and a source-generation
boundary.

## The structured shared LIR as a transpiler boundary

`LeanerIR` is neither Lean LCNF nor raw Rust MIR.  Both contain
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
mechanically decodes it; `LeanerIR.Validation.validate` performs the
administrative simplification, dominance/post-dominance and natural-loop
analysis, and constructs the structured LIR.  Rust MIR for the intended safe,
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
toolchain is promoted. Readable Rust is recovered from validated LIR, not by
trying to invert rustc's optimization choices.

The structurizer produces a witness connecting MIR blocks and edges to the
structured body.  `SwitchInt` regions become `if` or `match`; a natural-loop
header and backedge become `loop` or `while`; edges to that header become
`continue`; and edges to the loop exit become `break`.  Thus source printing
and verification consume a direct structured AST, not a CFG plus a secondary
presentation overlay.

The initial importer rejects a graph it cannot structurize precisely, with a
diagnostic at the relevant Rust origin.  A raw block/jump fallback is not part
of the shared Leaner language.  The raw CFG and the structurization witness
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
grouping may differ.  If normalized LIR equality is unnecessarily strong, the
replacement obligation is a checked semantic equivalence over the Rust
profile.  Original Rust provenance remains associated with the first imported
unit, while generated Rust gets a `GeneratedSourceMap` back to its LIR nodes.

The corresponding Leaner-source property is separate and optional:

```text
semanticProjection(validate(elaborateLeaner(printLeaner(validatedUnit))))
  ≈α semanticProjection(validatedUnit)
```

It is a source-backend test, not a route by which Leaner source syntax derives
semantics or proofs.

This is deliberately a family of profile-selected target surfaces over one
core and one Leaner surface language, not separate Leaner Move and Leaner Rust
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
generated ranges to LIR node IDs. It never reads original `.rs` tokens, macros,
or retained Lean syntax to fill a semantic gap. Comments already admitted in
the shared LIR provenance table may be rendered, but never supply semantics.

The printer prefers ordinary Rust constructs—bindings, assignments, `if`,
`match`, `loop`, `break`, `continue`, direct calls, structs, and enums—rather
than reproducing rustc temporaries.  It may choose different but canonical
surface spelling, such as a `loop` instead of a recovered `while`, explicit
match arms instead of a source `if let`, or the smallest valid `unsafe` block
around an unsafe LIR operation. It does not promise token-identical comment
attachment, macro, or control-sugar round trips.

Every printed construct has a profile rule.  In particular, the Rust backend
must preserve integer width and overflow configuration, target-dependent
`usize`/`isize`, panic behavior, drop order, visibility and signatures,
generic predicates, selected trait/impl calls, and unsafe operations.  It
emits an explicit diagnostic—not a lossy approximation—when the accepted LIR
node cannot yet be represented in standard Rust under the artifact's target
configuration.  An imported dependency not emitted as Rust source is an
explicit Cargo dependency or a declared external summary; it is never an
implicit printer assumption.

The backend gate is:

1. render the source and format it with `rustfmt`;
2. compile/check it under the emitted, pinned Rust configuration;
3. export its MIR through the same Rustc Public producer;
4. construct and validate a fresh `RawUnit`; and
5. compare profile-aware semantic LIR and source-map coverage with the input.

This gives users readable Rust generated from LIR while retaining one semantic
path: the generated crate comes back through the same MIR adapter and
`LeanerIR.Validation.validate` as any other Rust crate.

## Small example and representation growth

Consider this current Leaner Move function:

```lean
fun bump_if (flag : Bool) (value : U64) : Action U64 := do
  if flag then
    pure (value + 1)
  else
    pure value
```

`#print_leaner_lcnf bump_if` shows what Lean's compiler currently gives the
Leaner normalizer.  In abbreviated form it is:

```text
bump_if flag value world :=
  spanMarker world;
  cases flag with
  | false => spanMarker world; return (value, world)
  | true  =>
      spanMarker world;
      sign  := instSignUnsigned;
      width := instWidthW64;
      one   := MoveInt.ofInt (Int.ofNat 1);
      sum   := MoveInt.add sign width value one;
      return (sum, world)
```

The actual base LCNF uses roughly two dozen generated binders because it is
ANF-like, threads `World` explicitly, resolves type-class arguments, expands
numeric literals, and retains span-marker calls.  Normalization collapses it
to the useful structured body:

```text
if flag then
  one := 1; sum := add value one; return sum
else
  return value
```

That shared body has two source parameters, two temporary locals, one
structured branch, and two value-producing instructions.  A canonical source
printer can therefore emit essentially the authored function above; it does
not print the LCNF noise.

A mutable-reference example behaves similarly:

```lean
fun bump_ref (slot : &mut U64) : Action Unit := do
  slot := *slot + 1
```

Base LCNF expands the state thread, read, numeral construction, addition, and
write into roughly twenty generated names.  The structured LIR is one sequence
with four operations: `readRef`, `loadInt 1`, checked `add`, and `writeRef`.
Its proof-facing meaning expands for a different reason:

```lean
fun slot =>
  withMutation slot fun mutation =>
    bind (pure mutation.read) fun old =>
    bind (checkedAdd old 1) fun new =>
    pure ((), mutation.write new)
```

The LCNF growth is temporary compiler administration.  The semantic growth
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
- LIR import deliberately keeps one generic body; interpreter instantiation
  supplies arguments and evidence without duplicating it. Any future native
  code-generation artifact is outside the LIR translation boundary;
- nested mutable loans add prophecy variables linearly, while alias or
  disjointness case splits can become combinatorial if borrow evidence is not
  used effectively.

## One semantic function interface

Every registered function is a declaration in a `ValidatedUnit`.  LIR semantic
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
LIR data; the import receipt and its justification live in the checked import
envelope.  The function's relational meaning is derived only after the unit
passes `LeanerIR.Validation.validate`; it is not selected by inspecting which
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

## Mandatory retirement of the direct source path

Today Leaner Move still has two paths: retained Lean syntax generates the
proof-facing `sourceSpec`, while typed base LCNF reaches NSIR and
`MoveModel.IR` for executable compilation.  That duplication is a migration
problem, not an architecture available to the Rust frontend.

The target is one path for every frontend:

```text
Leaner source ----+
Move source ------+--> RawUnit --> LeanerIR.Validation.validate --> ValidatedUnit
Rust MIR exchange-+                                          |
                                                            v
                                           LIR semantic elaboration
                                      -> meaning / contracts / `verify`
                                      -> interpreter / executable backends
                                      -> Rust and Leaner source backends
```

Leaner source remains a frontend: Lean elaboration may parse, expand syntax,
and construct a `RawUnit`, but no semantic pass reads retained syntax, Lean
environment extensions, LCNF, NSIR, or `MoveModel.IR` after raw-LIR
construction.  `spec`, `verify`, summaries, loop obligations, proof tactics,
the interpreter, executable lowering, and every source printer look up the
same validated LIR declaration.

The existing direct verifier is a temporary comparison oracle while features
are migrated.  It must not become a common `FunctionMeaning` adapter and it
cannot be used as a fallback when the LIR path lacks support.  Once corpus
coverage reaches the retirement gate, the retained-source declaration store,
source reparser, source-to-spec translator, and LCNF body rediscovery are
deleted.  Compatibility command names may remain only as thin LIR façades.

## `leaner-rust`: Rustc Public driver and raw-LIR boundary

The Rust frontend is one new Lake package, `leaner-rust`, rather than separate
Rust-profile and MIR-transport projects.  It contains the Lean Rust-profile
library, its integration tests, and the Rust exporter source in a sibling
Cargo crate:

```text
leaner-rust/
  lakefile.toml                 # requires ../leaner-ir
  LeanerRust/                   # profile registration, capability policy,
                                # RawUnit import façade, and tests
  rust-exporter/                # `leaner-rust-export` Rust binary
    Cargo.toml                  # Rustc Public / rustc-driver dependencies
    src/main.rs                 # Cargo invocation and driver callback
    src/export.rs               # MIR -> Rust-profile RawUnit mapping
    src/json.rs                 # versioned LeanerIR RawUnit encoder
  Tests/                        # Rust fixtures, JSON baselines, Lean checks
```

`LeanerIR` owns the versioned, language-neutral `RawUnit` serializer.  The
Rust crate implements that schema rather than inventing a second wire format.
The generic receipt, freshness, and alignment rules are defined by the
[shared LIR import model](lir-design.md#import-receipts-and-source-alignment).
Lean owns the exporter invocation and consumes its detached output as
untrusted, normally private transport data.  The Rust-specific receipt binds
the rustc/exporter revision, selected MIR phase, crate graph, target and
semantic options, and asserted upstream type/borrow-check status.  The
resulting file is the only on-disk hand-off:

```text
Rust crate
  -> leaner-rust-export / Rustc Public callback
  -> Rust-profile RawUnit JSON
  -> LeanerIR RawUnit decoder
  -> LeanerIR.Validation.validate
  -> ValidatedUnit
```

The production exporter runs a normal rustc analysis for the selected crate,
then, in its post-analysis callback:

1. queries public MIR, type, trait, instance, layout, and source information;
2. mechanically maps it to a Rust-profile raw CFG inside `RawUnit`;
3. serializes the complete, detached JSON artifact; and
4. returns `Compilation::Stop`, skipping MIR emission, LLVM code generation,
   object production, and linking.

It does not structurize control flow, decide Leaner support, lower raw pointers
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

The target is the body after type and borrow checking, but before
semantics-obscuring MIR optimizations.  This retains explicit places, bounds
and overflow checks, drops, cleanups, and source locations that Leaner needs
for a proof-relevant model.  The M0 exporter spike must demonstrate the exact
Rustc Public query and bind its phase in the import receipt.  We will not name
a private rustc query as part of the Leaner contract.

If the first public surface can only expose a different phase, the spike must
compare it with the target shape on the fixture corpus and document every
semantic consequence.  It may not quietly compensate with ad-hoc
`rustc_middle` access.  Charon ULLBC is acceptable as a temporary comparison
or experiment input, but not as the production definition of this phase.

### Rust-profile `RawUnit` payload

The generic exchange and receipt schema belongs to LIR.  The Rust payload adds
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
have first-class raw-LIR tags from v1 even when the first semantic profile
rejects them.  That preserves the Rust input truthfully, gives actionable
diagnostics, and prevents a future unsafe frontend from inventing a new import
format.  A profile must explicitly declare whether it supports each tag.

A successful rustc type and borrow check is required upstream admission
evidence, but it is not the only proof-facing justification for borrowing.
Leaner will run its own borrow checker on validated LIR, migrating the useful
rules from the older translation, and expose its checked facts or certificate
to the verifier.  Those LIR rules may deliberately accept a broader abstract
language than rustc; imported Rust remains limited to programs rustc accepts.
A trust report is part of every theorem that claims to apply to the imported
Rust program.

### Invocation, distribution, and caching

`leaner-rust-export` is a special Leaner tool called by the Lean/Lake build
driver, not a Cargo plugin that every user installs or compiles.  For now it
runs against a Leaner-managed, pinned nightly toolchain with matching
`rustc-dev` and LLVM libraries.  Leaner setup either ships that paired driver
and sysroot or downloads them into a versioned local cache.  An optional
environment/configuration override permits development against another
explicit toolchain.

Users continue to use their ordinary stable `cargo` workflow.  The export tool
performs a separate `cargo check`-like analysis only when Leaner imports a
Rust crate.  It stops the selected root crate after extraction; dependencies
are prepared as normal check/metadata artifacts so Cargo still receives the
artifacts it expects.  The cache key includes the manifest and lockfile,
sources, features, target, rustc/exporter revision, and semantic profile.  If
Rustc Public later provides a stable distribution surface, Leaner may revisit
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
design choices for Charon and Aeneas, but Leaner needs the raw CFG as evidence
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
  -> structured LIR (`if`, `match`, `while` / `loop`, `break`, `continue`)
```

Selection headers and their joins recover `if` and `match`.  A natural-loop
header and backedge recover a `while` or `loop`; its header edge is
`continue`, and its exit edge is `break`.  The structurizer records a checked
correspondence from these regions back to the MIR blocks and edges.  Its
alignment obligation is that the MIR CFG and recovered structured LIR have the
same outcomes for every supported input.

Reducibility is an import condition, not an assumed universal Rust property.
The initial frontend rejects a function whose MIR cannot be structurized under
the supported rules, rather than exposing general `goto`/basic-block syntax in
Leaner or using an approximate proof rule.  This keeps user-facing loop
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

Importing only monomorphized MIR would produce one Leaner body per concrete
Rust instance. Proving every copy independently is neither necessary nor a
good default, and it would prevent the LIR interpreter from operating over the
source program's generic structure. The import contract therefore requires
one generic body for each generic Rust item. `RawUnit` and `ValidatedUnit`
never contain a monomorphization manifest or a cloned concrete body as their
semantic representation. A Rustc Public API that exposes only a substituted
instance is insufficient for the supported item and blocks the exporter spike.

A concrete call can still carry resolved implementation-selection evidence.
That evidence binds an already generic trait call to a validated implementation
dictionary; it is not body cloning or monomorphization. The interpreter
instantiates the generic body with its supplied type/const/lifetime arguments
and evidence values, preserving one LIR declaration and meaning.

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

Traits are a language-neutral core-LIR facility, not part of the Rust profile:
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
it does not reverify the two calls.  In generated Leaner source, Lean type
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
allowed to replace the generic LIR body.

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

The current Move path already provides partial precedent: generic Leaner
functions are exported as one parametric declaration. The Rust design preserves
that separation and uses per-instance verification only for behavior that is
genuinely type- or implementation-dependent; such a view is derived from,
never substituted for, generic LIR.

## Ordinary references

This landed as the prophetic ownership model of validated LIR
([`prophetic-references.md`](prophetic-references.md)), which generalizes
the V0 `Mutation` structure to the shared IR. A shared reference is an
observation; a mutable borrow is a runtime value owning the loaned content,
the lender keeps a hole, the loan's certified death writes the current value
back, and a dying frame exports unreconciled loans through the state's
pending set — the prophecy is the contract-level name of that export.

The two requirements Rust added are covered by construction:

1. reference-bearing values such as `Option<&mut T>`, `&&T`, and structures
   with lifetime parameters need nothing special — borrows nest as values
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
aliased in ways that references may not.  Leaner must retain that distinction
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
constructs a contract from its LIR signature and logical parameter view.  It
must not inspect body provenance or retained frontend syntax.

For example, the same form should be valid for a Rust-MIR or Leaner-source
frontend declaration after both enter the same validated unit:

```lean
spec Crate.increment (value : &mut U64) where
  ensures value = old(value) + 1

verify Crate.increment
```

As in Leaner Move, mutable parameters are presented by referent value:

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

- a Leaner-source function calling an imported Rust function;
- an imported function calling a Leaner-source model;
- imported functions verified against other imported summaries;
- opaque external functions with explicitly trusted or separately proved
  contracts.

Every final theorem should report any trusted external summaries on which it
depends.

## Alignment and theorem meaning

The [shared LIR model](lir-design.md#import-receipts-and-source-alignment)
defines the distinction between a proof about validated LIR and a claim about
its source.  Rust adds one temporary assumption: the pinned Rustc Public
exporter preserves the selected MIR body when constructing `RawUnit`.
Successful rustc type and borrow checking is upstream admission evidence, not
a proof of that translation.  M7 replaces the trusted-exporter assumption with
checked translation/refinement results and composes them with `f.verified`.

## Source provenance and diagnostics

The current compiler already carries important pieces:

- named LIR locals include an optional source name;
- LIR instructions and terminators carry optional authored spans;
- `MoveModel.IR.Module` has function, block, instruction, and terminator
  source maps plus user-facing local names.

See [`Move.Compiler.LIR`](../move/Move/Compiler/LIR.lean) and
[`MoveModel.IR.Module`](../move-model/MoveModel/IR/Module.lean).

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
canonical `.rs` files to LIR node IDs and their original origins.  Re-import
records the reverse association.  The Leaner source backends provide the same
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

This register mirrors Rust-specific entries in the core LIR design. A deferred
item stays here until it lands, is explicitly rejected, or is superseded by a
recorded decision.

| Deferred item | Current safe boundary | Reactivate by |
|---|---|---|
| Complete/frozen RawUnit JSON v1 Rust mirror | The typed Rust mirror emits all Rust integer type widths and arbitrary-precision integer values, tuples, fixed arrays, references, direct calls, plain structs, nominal enums, constructors, actual discriminants, downcast/field/index places, MIR assertions and administration, and drop/unwind terminators. Rust serde and the Lean decoder reject duplicate and unknown fields recursively; exhaustive raw-control, core type/value/place, operation/expression/specification, and declaration corpora round-trip canonically and reject recursive unknown-field mutations. Only approval and publication of the current v1 spelling as an external compatibility contract remain before the contract is frozen | M0 before fixtures beyond the supported mirror become compatibility artifacts |
| Public function predicate query | `FnDef::generics_of` exposes lifetime/type parameter declarations, so unconstrained generic function bodies and local direct generic calls are retained once. Predicate queries remain public only on `TraitDecl`; the generic trait fixture exposes the trait's `Self` predicate and direct trait-method callee but not the function's `T: Step` clause. Const-function binder types are also unavailable | M0 before the generic trait RawUnit fixture can satisfy the gate. Filed upstream as [rust-lang/rust#161892](https://github.com/rust-lang/rust/issues/161892); the public queries are requested rather than reading `rustc_middle` |
| Complete MIR-to-RawUnit mapper | The selected phase is generic optimized MIR. A CFG-driven mapper now handles compilation-unit scalar/reference/aggregate/nominal/function type and name tables, the never type, borrowed UTF-8 `str` and its byte length, all fixed and pointer-width Rust integers, tuples, fixed arrays, unsized slices, nested references, composites containing references, safe non-variadic Rust-ABI function pointers to local nongeneric functions, concrete type-, lifetime-, and evaluated scalar const-parameterized local ADTs, unconstrained lifetime/type-generic functions and their local direct calls, lifetime entries, multiple functions, declaration-local ID remapping, shared namespace arenas, integer constants/primitives and integer casts, explicit copy/move/borrow/read including `CopyForDeref`, tuple/vector and struct/enum aggregate construction including compact repeated arrays, tuple and dynamic/constant fixed-array indexing, dynamic and from-end slice indexing, fixed-array and dynamic-slice length, pointer metadata, from-end subslice places, dereference/downcast/field places, Boolean and integer switches, direct and function-pointer calls, enum discriminant reads and updates, ordinary panic assertions, inert storage/place/type administration, drop/unwind terminators, comments, and source ranges. Unsupported MIR is rejected at its exact node instead of falling through a fixture shape; the selected post-analysis normalization removes `OpaqueCast` projections before export and treats rustc's last-use `Copy` of an unconstrained type parameter or mutable reference as the consuming source move required by shared LIR | M1: add remaining admitted type/rvalue forms, symbolic const-dependent type shapes, destructive/provenance MIR administration exposed by Rustc Public, and constrained/const generic functions once their metadata is public; M4 adds cleanup and effectful drop glue |
| Checked implementation-selection evidence and dictionary interpretation | Generic bodies, binder kinds, traits, impls, associated items, and generic arguments are core; `EvidenceId` is opaque | M2 before trait method calls execute or transfer contracts/proofs |
| Remaining MIR administrative statements and exceptional control | Direct calls lower to assignment plus their normal continuation; non-Boolean switches lower to typed matches; rustc's impossible exhaustive-enum default becomes a non-matching path. Rustc's borrow-check-only `FakeRead` normalizes to an inert place mention after explicit rustc admission, and the shared structurizer erases validated storage live/dead, place mention, and user-type ascription nodes. Ordinary bounds/overflow/division/remainder assertions lower to explicit panic branches under `panic=abort`. RawUnit and the Rust mirror retain deinitialization, discriminant updates, and retags; the optimized-MIR mapper resolves the public `SetDiscriminant` case to a stable enum variant. The public statement API exposes neither `Deinit` nor `Retag`; its ubiquitous `Rvalue::Use(..., WithRetag::Yes)` is explicitly normalized only under the admitted, rustc-checked `unsafe=reject` profile because it has no safe observable runtime effect. An unsafe/provenance profile must preserve it. These destructive/provenance nodes, cleanup edges, pointer-alignment assertions, and reachable `unreachable` blocks remain rejected by structurization | Rust M4 for destruction, cleanup, and richer panic behavior; M5/unsafe profile for retag/provenance and pointer-alignment assertions |
| Drop flags, cleanup scopes, and panic semantics | Under fixed `panic=abort`, a drop terminator with an unreachable unwind edge structurizes to an explicit typed `drop` operation followed by its normal continuation. Cleanup edges and effectful drop glue remain preserved/rejected; no cleanup is approximated | Finish M4 drop effects and flags; full unwinding requires a later separately approved milestone |
| Rust import-receipt/cache enforcement | The M1.5 file and Cargo drivers key private artifacts by canonical inputs, exporter binary, pinned toolchain declaration, Rust profile, and compiler selection. Cargo mode additionally binds locked metadata, all local/path package files, package/features/target, and uses a content-addressed dependency target directory. Every cache hit is revalidated and its artifact bytes must match the sidecar digest. Import, execution, and verification results retain a Lean-owned mode/input/cache-key/artifact-digest receipt, and named registrations persist the same receipt across module imports. The exporter records its pinned rustc admission assertion as trusted import evidence, and shared validation retains it for reporting; the cache digest is not yet a proof-grade semantic-body/alignment receipt | Complete the shared proof-facing receipt gate; M7 still requires alignment before source-level theorem claims |
| Raw pointers, unsafe operations, allocation/provenance, and UB | The probe observes raw-pointer types and operations; artifact mode rejects the first unsupported raw-pointer type precisely and leaves no partial JSON. Ordinary references are not a substitute | M5/U0–U2 |
| Trait objects, specialization, higher-ranked bounds outside the initial fragment, async/coroutines, FFI, and inline assembly | Outside the supported source fragment and rejected explicitly | Add only through separately scoped post-M2/M5 milestones with core/profile classification first |
| Broader library models (`Box`, `Rc`, slices, `RefCell`, and peers) | No accidental dependence on their unsafe implementation is admitted | M6 or a separately approved library-model milestone |

## Proposed milestones

### M0 — Rustc Public exporter spike and import contract

- Create the `leaner-rust` Lake package and its `rust-exporter` Cargo crate.
  The Lean package depends only on `leaner-ir`; the Rust crate contains all
  Rustc Public/rustc-driver bootstrap dependencies.
- Freeze RawUnit JSON v1, including the language-neutral generic,
  trait/implementation and associated-item schema, plus the
  Rust-profile capability vocabulary. Add fixtures for scalar arithmetic,
  enums, loops, direct calls, a generic trait method call, ordinary borrows,
  drops, a raw-pointer operation, and an unsupported operation.
- Implement the `leaner-rust-export` driver against the pinned Rustc Public
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

Current M0 progress on the pinned `nightly-2026-07-23` toolchain:

- The driver builds against Rustc Public, runs in its post-analysis callback,
  and returns `ControlFlow::Break`; integration tests confirm the successful
  early stop on every fixture.
- Every detached RawUnit records rustc analysis admission as explicit trusted
  import evidence. Shared validation checks and retains this producer claim;
  source/configuration hash binding remains the M1.5 receipt boundary.
- The executable corpus probe observes one unspecialized generic body, its
  trait declaration and `Self` predicate, the direct `Step::step` identity,
  source filenames, enum/loop switches, ordinary borrows, drop and cleanup
  edges, raw-pointer types/operations, and unsupported inline assembly.
- The probe uses only Rustc Public for mapping observations. `rustc_driver`,
  `rustc_interface`, and `rustc_middle` remain startup dependencies needed by
  the `run!` bridge; the mapper does not query them.
- Generic optimized MIR is the selected public phase. Plain lifetime/type
  generic functions and local direct calls retain one parametric body and
  explicit core instantiations; the `generic_identity` artifact prepares,
  executes, prints as LeanerLang, and round-trips through canonical Rust.
  No public function-predicate query exposes the trait fixture's `T: Step`
  clause, and no public query exposes a const function binder's declared type;
  those remaining API gaps are explicit M0 boundaries recorded above.
- Detached artifacts are implemented for the nullary Bool fixture, a
  two-parameter `u32` scalar primitive, a Boolean-controlled diamond, and a
  Boolean natural loop with a backedge and continuation exit. Rust emits
  canonical RawUnit JSON after the callback stops compilation; all four
  executable baselines are decoded, canonically re-encoded, structurized,
  validated, and prepared for execution by Lean. The Rust mirror rejects all
  schema shapes not yet represented by typed nodes.
- A recursive direct-call artifact additionally preserves the qualified callee,
  argument expressions, return destination, normal successor, and
  unwind-unreachable action. The shared structurizer lowers the destination to
  a Unit-typed assignment and continues at the normal successor; recursive and
  multi-function call baselines validate and prepare for execution.
- Safe, non-variadic Rust-ABI function pointers to local nongeneric functions
  use the core function type, closure-construction call, and indirect invoke.
  The canonical baseline validates, prepares, and executes `apply(41) = 42`;
  higher-ranked, generic, unsafe, foreign-ABI, variadic, and external targets
  remain exact boundaries.
- The mapper is now driven by supported MIR nodes rather than exact fixture
  shapes. It emits multiple local functions in deterministic identity order,
  resolves a non-recursive direct call through the compilation-unit name
  table, remaps each function's MIR local arena to parameter-first LIR locals,
  shares namespace expression/place arenas, and records separate source ranges
  for function declarations, locals, statements, and terminators.
- A `u32` `SwitchInt` artifact retains its ordered constant cases and default
  edge. Lean decodes and re-encodes it canonically, and the shared structurizer
  lowers it to a typed match with literal arms and a wildcard default.
- Ordinary shared references now use core LIR reference types and an interned
  inference-lifetime entry. Copying through a dereference validates and
  prepares for execution; a second fixture preserves the MIR `Ref` as a
  place-based immutable borrow and passes that reference to a direct call.
  MIR temporaries are declared as writable storage cells, so the combined
  borrow-and-call artifact also prepares for execution.
- Mutable reference types and dereference destinations use the same core
  vocabulary. A Rust assignment through `&mut u32`, followed by a read through
  that reference, validates and prepares for execution without a Rust-specific
  operation tag.
- Source-to-LeanerLang error baselines verify that raw pointers and inline
  assembly are rejected at the first unsupported typed node/terminator and
  never leave a partial RawUnit file. The exporter-local diagnostic capability
  probe still observes these constructs so future support cannot silently
  regress their detection.
- The detached enum artifact uses core nominal declarations, actual integer
  discriminants, a value-producing `DataOperation.discriminant`, and typed
  downcast/field places. Its `SwitchInt` structurizes successfully; rustc's
  impossible default block is represented by absence of a matching arm rather
  than a Rust-only panic. Its pointer-width discriminant now resolves from the
  explicit width recorded for rustc's selected target and never from the Lean
  host. General enum execution remains at the separate ownership boundary for
  observational reads of non-`Copy` values.
- Plain Rust structs now use `StructDecl.fields`, while struct and enum MIR
  aggregates use the core constructor-call operation. Owned return places are
  consumed with `move` rather than incorrectly requiring nominal values to be
  `Copy`; constructor and field-selection fixtures validate and execute.
- A non-`Copy` local struct-field move remains a projected `move` through the
  Rust artifact rather than being widened to a whole-local consume. Its baseline
  validates, prepares with initialization evidence, and executes by returning
  exactly the selected field.
- A single-variant enum fixture exercises the same rule through an explicit
  downcast and static payload field. It validates and executes without relying
  on the target-width discriminant semantics needed by general multi-variant
  matches.
- The mapper interns every Rust fixed integer width plus `usize`/`isize`
  deterministically. A single executable baseline covers `u8` through `u128`,
  `i8` through `i128`, and a negative signed constant. Pointer-width values
  execute using the validated Rust target-width profile option.
- Fixed-width `IntToInt` MIR casts map to the executable core `cast` primitive.
  One baseline covers unsigned truncation, signed-to-unsigned conversion, and
  sign extension; provenance, float, pointer, transmute, and subtype cast kinds
  remain exact rejections rather than being conflated with integer conversion.
- MIR left/right shifts map to fixed-width core shifts. Rustc's explicit
  overflow assertions retain the out-of-range behavior before the partial leaf
  operation; a `u32`/`u8` baseline preserves both directions, validates, and
  prepares for execution.
- Rust Boolean `&`, `|`, and `^` use the same typed bitwise nodes as integers
  while remaining distinct from logical operators. A three-result baseline
  validates and prepares for execution.
- Rust Boolean ordering uses the existing typed comparison nodes with
  `false < true`. A four-result baseline covers every ordering relation and
  prepares for execution without treating Booleans as integers.
- Rust `char` maps to a distinct target-independent core character type and
  numeric Unicode-scalar constant. Validation rejects surrogate and out-of-range
  code points; a crab-literal baseline covers every scalar ordering relation and
  character switch cases, plus `char`-to-integer and Rust's safe `u8`-to-`char`
  casts, and prepares for execution without conflating characters with `u32`.
- Integer `!` maps to fixed-width core complement and reconstructs signed
  results from the complemented two's-complement bits. `u8` and `i8` baselines
  validate and prepare for execution.
- Rustc Public `CheckedBinaryOp` addition, subtraction, and multiplication map
  to typed core overflowing operations returning `(wrapped_value, overflowed)`.
  A three-function intrinsic fixture retains all three optimized MIR forms and
  prepares for execution without conflating them with throwing checked nodes.
- Shared fixed-width signed division and remainder semantics now round toward
  zero as Rust requires. Checked remainder also validates the quotient, so the
  signed `MIN % -1` overflow cannot be mistaken for a successful zero result.
- Rust integer constants and enum discriminants use arbitrary-precision JSON
  number carriers in the typed mirror. A `u128::MAX` baseline decodes to Lean's
  unbounded `Int`, validates against the exact unsigned 128-bit range, and
  prepares for execution without truncation or quoting the JSON number.
- Nonempty Rust tuples and fixed arrays use core tuple/vector types and
  constructors. Tuple field projections become literal indexed places; the
  aggregate baseline validates and prepares for execution without a Rust-only
  tag. A separate baseline preserves dynamic fixed-array indexing as a core index
  place, including the pointer-width `usize` operand, evaluated fixed-array
  length, constant-index projection, and bounds-check assertion. The assertion
  structurizes to an explicit panic branch, and the baseline executes both an
  in-bounds index and the preserved panic using the recorded target width. A
  separate repeated-array baseline maps MIR
  `Repeat` to the core `repeatVector` primitive. Its fixed length stays in the
  result type, the operand remains singular, shared validation enforces the
  element's `Copy` ability, and the fixture executes without making RawUnit
  size proportional to the array length.
- Rust slices use the dynamic core vector type behind a profile-tagged shared
  reference. MIR `PtrMetadata` lowers to core reference dereference plus
  `length`, while dynamic slice indexing uses the existing place index and
  explicit bounds assertion. Canonical Leaner source uses receiver auto-deref
  as `values.length` and `values[index]`, while the lowered LIR retains the
  explicit dereference. Its baseline validates and structurizes; target
  width is no longer inferred or a preparation blocker, while complete slice
  execution remains part of the reference/place integration.
- Rust's unsized UTF-8 `str` maps to the core string type behind ordinary
  shared or mutable references. Shared and consuming mutable-reference
  identity functions validate, execute over string heap values, print as
  LeanerLang `&string` / `&mut string`, and re-import through canonical Rust
  `&str` / `&mut str` with equal outcomes and final heaps. The retained typed
  `core::str::len` call maps to core `length`, whose string semantics count
  UTF-8 bytes rather than Unicode scalar values.
- Slice rest patterns map MIR `Subslice` to a first-class core subslice place,
  including the from-end stop convention. Core place semantics preserve
  range reads and equal-length writes; the fixture's explicit empty-slice arm
  structurizes without treating rustc's unreachable pattern defaults as panic.
- From-end constant slice indexes lower compositionally to core `length`,
  pointer-width subtraction, and the existing index place. The pattern baseline
  validates and structurizes while retaining the runtime-dependent offset and
  the explicit target-width semantics.
- Rustc's selected post-analysis normalization removes MIR `OpaqueCast` place
  projections before the Rustc Public body is observed, so the mapper does not
  invent a semantic node for a projection absent from its frozen input phase.
- Rust's `!` maps directly to the core never type. A diverging-loop baseline
  validates and structurizes without manufacturing a return value or adding a
  Rust-only type tag.
- Reference types are interned inside-out, so nested references retain every
  reference layer and dereference projection rather than requiring a scalar
  referent. Composite and reference types share one dependency-driven interner,
  so tuples/arrays/slices may contain references and references may target
  composites without separate implementations. `&&u32` and reference-tuple
  baselines validate and prepare for execution.
- A surviving MIR `CopyForDeref` maps to the core non-consuming `read`
  operation, matching Rustc Public's guarantee that its only use is an
  immediate dereference; optimized fixtures may normalize it to ordinary copy.
- Local generic ADTs with concrete type and lifetime arguments use core generic
  binders, type-parameter nodes, lifetime entries, and nominal arguments. The
  `Wrapper<u32>` and `Borrowed<'_, u32>` baselines validate and prepare for
  execution, including recursive substitution of a reference-bearing generic
  field. An `Outer<u32>` baseline additionally checks that substituted fields
  discover and intern a nested concrete `Wrapper<u32>` nominal type, without
  pretending that the missing generic function-predicate query has been
  solved. A `Tagged<u32, 3, true>` baseline additionally preserves
  evaluated integer and Boolean const binders/arguments, derives their
  declared `usize`/`bool` types from the admitted concrete instantiation, and
  prepares for execution. Symbolic consts inside type shapes remain an exact
  boundary.
- A concrete `Maybe<u32>` enum preserves its generic nominal argument through
  discriminant reads and payload projection, then validates and structurizes
  its exhaustive match; generic arguments no longer trigger an obsolete
  discriminant rejection.
- Signed integer negation maps to the core `negate` primitive. Its optimized
  MIR overflow assertion structurizes to an explicit panic branch, so
  debug-mode `-MIN` behavior is retained and the baseline prepares for execution.
- MIR `Drop` terminators detach their place, normal successor, and unwind
  action in canonical RawUnit JSON. Under the fixed `panic=abort` profile, the
  exact `drop place -> normal / unwind unreachable` shape structurizes to a
  typed core `drop` operation followed by the normal continuation. Its
  artifact validates, prepares, and prints as `drop(place)` in
  LeanerLang. Cleanup edges and effectful drop glue remain explicit M4
  boundaries; no cleanup behavior is erased or approximated.
- MIR storage-live/dead, place-mention, and user-type-ascription statements
  have typed RawUnit mirror forms; borrow-check-only fake reads normalize to
  place mentions after rustc admission, while optimized-MIR coverage/counter
  nodes are discarded as compiler instrumentation. The mirror also has typed
  deinitialization, discriminant-update, and retag forms; the mapper preserves
  the `SetDiscriminant` case exposed by Rustc Public and resolves its variant
  name, while the pinned public statement API exposes neither `Deinit` nor
  `Retag`. A division fixture detaches its `expected=false`
  division-by-zero assertion and unwind action in canonical JSON. Shared
  validation erases the inert administrative nodes and structurizes the
  assertion as an explicit panic branch. Generated empty sequences, joined
  branches and switches, and exiting loops carry Unit, while a sequence with a
  value-producing tail inherits that tail's type; the fixed-width and control
  fixtures then prepare for execution without a Rust-specific validation path.
- Shared validation retains a checked correspondence witness for every raw CFG
  it structurizes. The witness covers every reachable raw block and edge and
  records branch, switch, natural-loop membership/exit, and direct-call normal
  continuation regions; independent mutation tests reject missing or changed
  entry, block, edge, and region facts before `ValidatedUnit` construction.

### M1 — Raw MIR ingestion, provenance, and structured control

- Complete the MIR-to-RawUnit mapper for types, named locals, places, basic
  blocks, direct calls, source origins, and the information needed to
  structurize reducible control.
- Register the `LeanerRust` profile and decode the generic RawUnit serializer
  through `LeanerIR.Validation.validate`.
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

- Add a Lean/Lake driver command that invokes `leaner-rust-export` for a Cargo
  manifest, package, feature set, and target, then registers the validated
  unit for proof and source-backend consumers.
- Define the tool cache and artifact location from source/Cargo-lock hashes,
  features, target, profile, exporter, and rustc revision.
- Implement the Leaner-managed exporter/sysroot installation plus an explicit
  developer override.  Normal users must not install nightly, `rustc-dev`, or
  the driver with `cargo install`.
- Run dependencies with ordinary check/metadata artifacts and stop only the
  extracted root crate after analysis.

Gate: a clean checkout imports a fixture through one Leaner setup/build command
without relying on the user's Rust toolchain, and a repeated build is cached.

Current M1.5 progress:

- The `leaner-rust` Lake executable provides `import-file` for a
  self-contained Rust library source. It builds `rust-exporter` from its local
  `rust-toolchain.toml`, invokes the resulting binary with the matching sysroot
  library path, decodes the detached JSON, and validates it through
  `LeanerIR.Rust.validate` before publishing the requested artifact.
- `LEANER_RUST_EXPORTER` and `LEANER_RUST_SYSROOT` are explicit development
  overrides. The normal path never uses `cargo install`; it asks Cargo/rustup
  to build the checkout-owned exporter with the pinned toolchain. The managed
  checkout path uses Cargo's optimized release profile because the semantic
  round-trip corpus launches the mapper once per fixture; the selected binary
  bytes remain part of every cache key. Within one Lean process the managed
  exporter, sysroot path, and invariant exporter/toolchain hash are memoized;
  binary and toolchain metadata guard the cached hash. This avoids rebuilding,
  rediscovering, or rehashing the same tool for every fixture without changing
  cross-process Cargo freshness or artifact receipts. Shipping or
  downloading a prebuilt paired driver/sysroot remains required before this
  counts as the normal-user managed distribution promised by the gate.
- `scripts/bench-rust-pipeline.sh source` reports exporter setup, cache-key,
  rustc/export, RawUnit validation, Rust-source/map, semantic-projection,
  preparation, and interpreter timings. Its `e2e` mode separately reports
  Rust import, LeanerLang printing, and LeanerLang re-import/elaboration.
  `LeanerRust.Benchmark` is inert unless `LEANER_RUST_BENCHMARK` is set, so the
  phase probes remain available without affecting ordinary test output.
- File-mode cache keys bind the canonical source path and content, exporter
  binary, toolchain declaration, profile name/version/options, target, and all
  other rustc arguments. A key match is still decoded and validated; corrupt
  cache content is never returned as a `ValidatedUnit`. Each result retains a
  Lean-owned file-mode receipt with a private constructor, canonical input
  identity, cache key, and artifact-byte digest. Import and prepared-result
  wrappers also have private constructors, so consumers cannot recombine a
  receipt with a different validated unit. A changed cached artifact is rebuilt
  even when its replacement is independently valid RawUnit JSON.
- `import-crate` accepts a manifest, exact library package, feature/default-
  feature selection, and target. It runs locked Cargo metadata/check under the
  pinned toolchain and installs the exporter as `RUSTC_WRAPPER`. Dependency
  crates run through Cargo's rustc and produce ordinary metadata; the wrapper
  intercepts only the named primary library and stops it after extraction.
- Cargo-mode keys additionally bind Cargo metadata and the lockfile, every
  file in local/path package roots, and package/features/target selection.
  An uncached key gets a content-addressed Cargo target directory, avoiding a
  destructive clean and ensuring Cargo cannot mark a changed root fresh before
  the wrapper emits its artifact. `LEANER_RUST_CACHE` overrides the default
  checkout-local cache for development.
- Lean integration tests perform fresh and repeated cached imports through
  both file and Cargo modes; the Cargo fixture includes a path dependency to
  exercise ordinary dependency compilation. Imported results now expose an
  explicit executable-preparation boundary and a separate verification-
  preparation boundary, each retaining the same validated unit,
  declaration/location identities, mode/input/cache-key/artifact-digest receipt, and cache
  status; both integration modes
  execute their runtime-prepared artifact through the shared interpreter.
- The explicit `#import_rust_file "source.rs" => "unit.raw.json" as name` and
  `#import_rust_crate` commands import through that same driver and store the
  resulting `ValidatedUnit` in a persistent Lean environment extension. The
  Cargo form requires the manifest, exact package, feature/default-feature
  selection, target (empty for the host), output, and registration name, so
  configuration affecting extraction remains visible at the call site.
  Registrations and their receipts survive module imports, reject assigning
  different semantics or input receipts to an existing name, and remain
  subject to the separate execution/
  verification preparation boundaries used by source and proof consumers.
- Prebuilt exporter/sysroot setup remains before the full M1.5 gate is closed.

### M2 — Basic semantics and contracts

- Support scalar/ADT operations, assignments, direct calls, structured
  selections, and recovered reducible loops without custom destruction.
- Generate function meanings only from validated LIR bodies.
- Verify generic bodies parametrically over trait evidence and method
  contracts for the initial methods, associated-types, and trait-inheritance
  subset.
- Emit canonical Rust source for the accepted scalar/ADT/control subset and
  check the `Rust → MIR → RawUnit → ValidatedUnit` semantic round trip.
- Exercise Leaner source printing separately as another validated-LIR backend.
- Attach the existing-style `spec` / `verify` interface.
- Export modular summaries and source-positioned proof obligations.

Current M2 progress: `LeanerRust.Source.render` is the first standard-Rust
backend and accepts only the shared `ValidatedUnit` boundary. It emits explicit
local storage and structured control for the target-independent Boolean and
fixed-integer scalar subset, including assignments, branches, loops, returns,
modular arithmetic/bitwise operations, casts, tuples, direct calls, and plain
struct construction/selection, fixed arrays, function pointers, and Unicode
scalar character literals/operations. Panic branches use stable
`panic=abort` terminal spelling; the distinct non-panic abort outcome is
rejected instead of conflated. Seventy-four scalar, character, string,
aggregate, call, struct, reference, ownership, overflow, and control cases are
rendered deterministically, compiled through
the same pinned Rustc Public importer, revalidated, and compared by observable
interpreter outcome. Every executable case is also compared through the
provenance-free `LeanerRust.Equivalence` semantic projection. That projection
alpha-normalizes binder and local spelling, Copy loads, associative block
nesting, and the exact return/call forwarding, shift-distance cast, and
duplicated panic-guard administration introduced by standard-Rust rendering
and rustc MIR. It also contracts the exact borrowed enum-switch lowering and
the redundant bounds guards rustc restores around already guarded fixed-array,
slice, and from-end slice accesses. Unsupported declaration, contract,
attribute, and specification
families fail the comparison explicitly rather than disappearing from it. The
independent LeanerLang backend covers all forty-nine checked-in successful
Rust RawUnit fixtures:
scalar and fixed-width signatures, unary and Boolean bitwise operations, pure
tuple construction, context-directed integer casts, repeated fixed vectors,
comparisons, shifts, guarded division/remainder and negation, panic paths, and
full-range `u128` literals, plus direct recursive calls selected by a branch.
Unicode scalar literals, ordering, casts, and literal/wildcard classification
matches are included, together with tuple/fixed-vector construction, tuple
projection, dynamic and literal fixed-array indexing, its panic path, and
fixed-array destructuring, plus plain struct declarations, positional
construction, typed field selection, direct Boolean branches, and exhaustive
integer literal switches, plus direct generic-field instantiation and non-`Copy`
partial field moves, recursively substituted nested generic-field loads,
overflow-reporting add/subtract/multiply, plus typed integer/Boolean const binders and concrete const
arguments on a generic ADT. Each fixture is printed,
freshly elaborated, compared through the executable semantic projection, and
reprinted byte-for-byte. The
importer recognizes only the exact core integer
methods emitted for wrapping/overflowing arithmetic and maps them back to core
primitives. The executable corpus covers every arm of a non-Boolean integer
switch, both the normal and terminal-panic assertion paths, an explicit abort,
and the full unsigned-128 constant range. It also records
all fixed signed and unsigned integer parameter widths, including their minimum
or maximum boundary values, across source re-import. It also records
rustc's `0:0` compiler-generated spans as explicit
generated locations rather than inventing source bytes or rejecting re-import.
Enum declarations preserve explicit discriminants and named/positional fields;
downcast-field source reconstruction is exercised by a single-variant partial
move. Complete discriminant/field-selection switches render as direct enum-
pattern matches and pass semantic alpha-comparison across both variants.
General and guarded enums are target-width executable and compare both
variants plus guarded/fallback outcomes across source re-import; discriminant
inspection is treated as a non-consuming tag projection rather than a copy of
the non-`Copy` payload. Borrowed downcast fields are reconstructed as
reference-preserving matches. The
never-returning loop similarly re-imports for signature shape, omitting rustc's
synthetic never-typed return local because declaring a local of type `!` still
requires an unstable Rust source feature. The four executable reference cases
also compare recursive signature shape while treating regenerated inference-
lifetime IDs as non-semantic. Function exit reclaims only heap cells allocated
to stabilize that frame's borrowed locals, so these stateful comparisons do
not expose interpreter-internal local storage or renumber inherited slots.
Array-index, borrowed-slice, and from-end slice-index units re-import with equal
recursive signature shape and compare normal or bounds-panic interpreter
outcomes across canonical-source re-import. The exporter records
rustc's selected 16-, 32-, or 64-bit target width in the validated Rust profile;
preparation requires that state for `usize`/`isize`, and primitive evaluation
uses it for wrapping, checked, overflowing, cast, shift, and length results
without consulting the Lean host. The array-index round trip exercises both an
in-bounds result and its preserved bounds panic through this path. The same
unit executes fixed-array destructuring and a statically safe literal index
after re-import, checking that only rustc's redundant restored guards are
contracted. Value-level
reference dereference remains explicit in LIR but canonical Leaner slice
indexing uses receiver auto-deref. From-end owned
subslice borrows render as slice-rest pattern matches rather than range indexing,
so their canonical Rust re-import stays dependency-free and compares normal and
empty-slice outcomes semantically. General range indexing still lowers through
the dependency types `std::ops::RangeFrom` and `Index` and remains behind the
standard-library dependency model. Type- and
lifetime-generic structs, nested
generic structs, and a generic enum re-import with binder, field, variant, and
concrete signature shape preserved; explicit reference lifetimes use their
declaration binder. Their concrete `u32` functions now also compare interpreter
outcomes across source re-import for direct, nested, lifetime-bearing, and both
generic-enum variant values. Const-generic ADT declarations retain checked
binder types when the admitted artifact contains a concrete instantiation from
which Rustc Public exposes the type. The `Tagged<u32, 3, true>` declaration and
reader now render, re-import, and compare semantically. Symbolic const-dependent
types and generic-function const metadata remain unavailable from the pinned
frontend API. Validated lifetime/type/const binders, lifetime-outlives predicates, and type-binder
`Copy` abilities or equivalent core `Copy` predicates on functions and nominals
have canonical source spelling;
constraints are rendered as standard Rust `where` clauses, with source-map
provenance for function constraints. They are covered at the mapped rendering
boundary rather than re-import: the pinned Rustc Public frontend does not expose
the generic predicates needed to emit such a unit. Other non-`Copy` predicates and binder
abilities, consts, and evidence remain explicitly rejected rather than omitted.
`renderWithSourceMap` also returns
deterministic half-open UTF-8 byte ranges for every emitted function,
nominal declaration and generic binder, variant, field,
declaration/signature/local/expression type use, expression, pattern, place occurrence,
and function-scoped local
declaration/reference. Each range retains its stable LIR node ID and
`LocId`; function-owned nodes additionally retain their import `OriginId` and
checked `AlignmentId`. Nominals carry no invented import provenance because
`StructDecl` has none in the schema. Unicode cases verify that mapped ranges
never split a scalar. Places use their owning expression's location and
function import provenance because place arenas have no independent locations.
The complete executable source corpus independently traverses every structured
body and requires a valid range for every reachable emitted expression,
pattern, and place, excluding only unspelled uninitialized patterns and enum
downcast administration; declaration, binder, field, variant, local, and
spelled type-use coverage remains mandatory as well.
Dependency imports and unrendered namespace, function, or nominal semantic
metadata—including contracts, abilities, properties, pragmas, and attributes—
are explicit source-backend errors rather than silently omitted.
Plain type-generic identity and local direct-call bodies now preserve their
binders and inferred instantiations through RawUnit, execution, canonical
LeanerLang, canonical Rust, and semantic source re-import. LeanerLang omits the
type argument on the direct call because its ordinary argument type determines
it. Rustc's optimized last-use `Copy` spelling for an unconstrained parameter
is normalized to the source-valid consuming move; a body that really requires
two copies still fails preparation without `Copy` evidence. An inherent
non-generic method and its receiver call use the same checked local
function/ADT boundary, retain the explicit overflow path, and execute after a
fresh LeanerLang elaboration. Canonical LeanerLang also contracts rustc's
overflowing-result tuple plus Boolean assertion branch for checked add,
subtract, and multiply into the corresponding `checked*Panic` primitive. The
method fixture exercises both its ordinary result and overflow panic before
and after fresh elaboration, so the source cleanup does not weaken checked Rust
arithmetic into a modular operation. Constrained and
const-generic function metadata, semantic normalization of remaining
validation-only bodies, generic function contracts/evidence, and proof
summaries remain before M2 is complete.

### M3 — Ownership and ordinary references

- Model moves and initializedness.
- Implement the LIR borrow checker, migrating applicable rules from the older
  translation, and expose its checked loan/lifetime facts or certificate to
  the verifier.
- Retain rustc's successful borrow check as upstream admission evidence and a
  differential oracle, not as the semantic authority for LIR borrowing.
- Reuse and extend prophecy semantics for `&T`, `&mut T`, reborrows, calls,
  and returned references.
- Add positive and negative agreement tests against Rust execution.

Current M3 progress: shared preparation runs a path-sensitive definite-
initialization analysis over structured bodies. Parameters start initialized;
direct-local move/drop consumes them, direct writes and pattern bindings
reinitialize them, and branch/loop joins conservatively diagnose later reads
unless every reaching path is initialized. Place indexes are checked too. Both
execution and verification preparation now expose the same private-wrapper-
backed certificate for each accepted structured function, bound to its
namespace/function identity, root, initial parameter locals, and local count.
Static local struct-field moves now consume only their selected path; sibling
fields remain readable, writes reinitialize a moved path, and branch joins
retain only path facts true on every predecessor. Variant-qualified static
field moves retain enum downcasts in that path and are exercised from Rust
source through preparation and execution. Literal-indexed tuple and fixed-
vector elements use the same path facts: sibling indexes remain available and
rewriting a moved element restores its path. A dynamic index is also executable
but conservatively consumes its entire owning local; an owned subslice uses the
same whole-local rule. A separate conservative structured-
body pass keeps each local-rooted shared or mutable loan live while needed,
tracks local aliases created by value/move/copy/read transfer, direct
assignment, and nested binding patterns, and propagates parent loans through
locally bound reborrows. Direct-local and projected expression assignments and
write operations release an overwritten holder before evaluating an independent
replacement and then attach the replacement loan at the destination carrier;
pattern assignments do the same for their direct bindings. Whole tuple,
vector, and same-unit namespace-resolved struct/enum values
retain source-aligned carrier paths through lets, assignments, and direct-local
writes, so projected uses retain and transfer only overlapping component loans;
a projected move or drop releases only the selected carriers, and conditional
or match result joins preserve the paths of their alternative producers.
Tuple and nominal-constructor patterns attach each child binding only to its
matching carrier path, including aggregate-valued conditional and match
producers; unknown producer shapes retain conservative all-to-all propagation.
Wildcard-only declarations
and pattern assignments end
newly created uncarried loans without ending loans held by another alias. The
analysis ends a loan after every known holder's last use in a sequential structured
block, shortens holders independently across mutually exclusive `if` branches and match arms,
preserves only later-used carrier loans through nested operands and initializers,
transfers
argument loans whose instantiated, structurally known direct/tuple/vector
parameter lifetimes transitively outlive a structurally known call result while
ending uncarried temporary argument loans after the call, removes loans
unused by both a loop body and its continuation before the loop fixed point,
ends unbound temporary loans at wildcard let and assignment patterns,
releases a direct reference holder when it is moved or dropped while retaining
a moved loan only for the carried temporary value,
checks supported direct-local place indexes as actual loan use sites instead of
retaining every unrelated holder, distinguishes unequal nonnegative literal
indexes, disjoint forward subslices, and literal indexes outside a forward
subslice while treating dynamic/from-end projections conservatively, and otherwise
merges holder sets and loans
conservatively across iterations. It rejects
overlapping reads/writes/consumption and loans live at normal or explicit
reference-bearing returns while accepting an unrelated returned parameter
after a local loan dies. Moving or dropping through a dereference is rejected
by safe-profile preparation, while the admitted Rust `CopyForDeref` path and
ordinary dereference reads and writes remain supported. The pass exposes
reference-parameter and checked loan-site/lifetime plus holder-local
certificates through both preparation wrappers. Certificates also solve and
retain reflexive, `static`, declared, reborrow-parent, explicit-freeze, structurally paired direct-call
argument/parameter/result, function-result boundary, and transitively implied
outlives facts. Compatible reference-bearing structural tuples, vectors,
references, and function types may use distinct lifetime identities at direct
calls, normal fallthrough, and explicit returns. Constraints recursively follow
shared-reference covariance, mutable-referent invariance, and function-argument
contravariance; nominal variance remains explicit rather than guessed.
Loan-site history is distinct from expression-carried loans. Same-unit direct-call
result-source selection follows transitive declared predicates through direct,
tuple, vector, instantiated type-parameter, and namespace-resolved nominal field
shapes; recursive back-edges with the same fully instantiated arguments are cut
while sibling reference lifetimes remain visible, and uncarried temporary loans
from unselected parameters end after the call. Same-unit calls retain packed
multi-result indexes plus structural tuple/vector/reference and acyclic instantiated
struct/enum carrier paths when declared lifetime relations identify the returned
component. Recursive nominal back-edges conservatively retain their subtree
root without widening nonrecursive siblings. Argument-changing recursive and
external-dependency source selection remains conservative; dependency/unknown aggregate alias
propagation, authoritative nominal variance, remaining nominal/function-
variance use-site constraints, full returned-reference region precision,
and the prophecy extension remain before
the M3 gate is complete. Canonical-source round trips now provide positive
Rust differential cases for a local shared borrow passed through a direct call
and for nested-struct plus variant-qualified partial moves. Direct shared,
mutable, nested, and tuple-contained reference cases compare both returned
outcomes and final heap state after canonical-source re-import, including an
observable mutable write through `&mut`. A negative overlapping-mutable-borrow
fixture is rejected by rustc with E0499 before artifact emission. Together,
these cases retain rustc as an upstream differential oracle while the shared
LIR preparation pass remains the semantic authority for admitted artifacts.

### M4 — Drop and panic

- Add explicit destruction and drop-flag behavior.
- Implement the fixed `panic=abort` terminal semantics; do not support
  unwinding or `catch_unwind` in this slice.
- Ensure guard and destructor effects are visible to proofs.

Current M4 progress: ordinary rustc assertions and unconditional MIR `Abort`
terminators structurize to the shared terminal Rust panic outcome under the
fixed `panic=abort` profile. The exporter also normalizes rustc Public's
retained calls to the well-known abort intrinsic and stable
`std::process::abort` entry point to that raw terminator; the stable-source
artifact validates, prepares, and executes as terminal panic. A raw MIR drop
with an unreachable unwind edge structurizes to an explicit core drop and its
normal continuation, validates and prepares, and is visible as
`drop(place)` in the LeanerLang E2E baseline. Cleanup edges, `Resume`,
effectful drop glue, and drop flags remain explicit rejection boundaries
rather than being erased.

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

- State and prove the generic core-LIR semantic results.
- Validate or prove refinement from the MIR exchange through `RawUnit` to the
  supported `ValidatedUnit` fragment.
- Establish semantic equivalence for the
  `ValidatedUnit → Rust source → MIR → RawUnit → ValidatedUnit` round trip,
  and separately for supported Leaner source backends.
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
   single LIR pipeline;
3. emit readable canonical Rust source from the resulting `ValidatedUnit`;
4. recompile that source and recover an equivalent validated Rust-profile
   body through the same MIR importer;
5. preserve a generic body and its trait obligations rather than requiring a
   separate proof for every concrete use;
6. interpret the resulting LIR for a Rust fixture and obtain the same
   observable result as the Rust execution.

Later verification sessions extend this to:

7. attach a normal Leaner-style contract to the registered function;
8. prove that contract with the common verification interface;
9. transfer that theorem to concrete calls through validated evidence, without
   replacing the generic LIR body;
10. report failed obligations at their original Rust locations;
11. state exactly which alignment theorem or trusted importer assumption makes
   the result a claim about the Rust MIR body.

When these steps hold, the provenance of a function body no longer affects
how it is specified or proved.  It affects only how its meaning is justified
and where diagnostics are reported.
