# Linked MonoVM differential execution

## Purpose

Leaner needs an executable comparison partner for Move programs. A test should
be able to compare the behavior of compiler-produced Move code with both the
original validated LIR and the LIR obtained after a LeanerLang source round
trip.

This is differential testing, not validation against a trusted reference.
MonoVM and the Leaner pipeline are both new implementations; neither side is
the baseline. A failing comparison is a finding that can locate a bug in
either implementation — or a genuine ambiguity in the shared understanding of
Move semantics — and that symmetry is the point of the harness.

MonoVM is a Rust library in the same repository. Link it into the Lean e2e test
driver instead of invoking a CLI or maintaining a long-lived subprocess:

```text
Move sources ──► compiler + MonoVM ───────────────► observable outcome
      │
      └────────► Move frontend ─► validated LIR ─► LeanerIR interpreter
                                      │
                                      └─► LeanerLang ─► LIR interpreter
```

The first comparison checks the Move-to-LIR semantic boundary. The second
checks that canonical LeanerLang printing and re-elaboration preserve it.
This is testing evidence, not a proof that either runtime implements Move
correctly.

## Boundary

Rust has no stable ABI, and MonoVM's arenas, loaders, and runtime pointers must
not escape their execution guard. The linked interface therefore has three
layers:

```text
Lean `ByteArray`
    │ `@[extern "leaner_monovm_run"]`
small C shim using `lean/lean.h`
    │ pointer + length C ABI
Rust static library
    │ compile, load, execute, normalize
MonoVM
```

Only owned bytes cross the Rust boundary. No Rust, Lean, MonoVM, compiler, or
arena object is shared. The C shim copies the Rust result into a Lean
`ByteArray`, calls the Rust deallocator, and returns an ordinary Lean `IO`
result. The shim is written in C against the pinned toolchain's `lean/lean.h`
so the Rust crate never touches the Lean object ABI and no third-party
`lean-sys` style binding is pinned.

The Rust library exports a minimal versioned ABI:

```c
uint32_t leaner_monovm_abi_version(void);
int32_t leaner_monovm_run(
    const uint8_t *request, size_t request_len,
    struct leaner_buffer *response);
void leaner_monovm_buffer_free(struct leaner_buffer response);
```

Every domain failure — compilation, loading, execution, malformed request —
travels *inside* the response payload. The `int32_t` status is transport-only:
zero means the response buffer is valid; nonzero means no response could be
produced at all, and Lean surfaces it as an `IO` error.

`leaner_monovm_run` must catch Rust panics and encode them as internal harness
errors. A panic or Rust allocation must never cross the C boundary. This
containment relies on unwinding: no current workspace Cargo profile sets
`panic = "abort"`, and the adapter crate must refuse to compile under one
(`compile_error!` under `cfg(panic = "abort")`) so a future profile change
cannot silently disable containment.

## Request and response

The payload is deterministic JSON initially. Its top-level `version` is
independent of the native ABI version so either can evolve without silently
mis-decoding the other.

A request contains:

- the exact source bundle, named-address assignments, and compiler options;
- a list of module/function calls, type arguments, typed arguments, and gas or
  heap limits;
- the initial resource state and deterministic native context when required.

One request contains all calls for a package. MonoVM compiles and loads the
package once, then executes every call while its `ExecutionGuard` is live.
This avoids unsafe persistent handles and makes the linked call competitive
with a worker process.

Each call produces exactly one normalized outcome:

- `returned`: typed return values and mutable-reference outputs;
- `aborted`: code, message when present, and module/script location;
- `exhausted`: the gas or heap budget ran out before the call completed;
- `failed`: the program failed at runtime without aborting, carrying the
  engine's classification of the failure and diagnostic text;
- `error`: compilation, loading, ABI, or internal-runtime failure with a stage
  and diagnostic location.

`failed` and `error` are different claims and must not be conflated. A Move
runtime failure — arithmetic overflow, an out-of-bounds index, a missing
resource — is an outcome *of the program*, and an engine that reports it as
something other than an abort is still telling us what the program did; that
has to reach the comparison. `error` says the opposite: no outcome was
obtained, so there is nothing to compare. Collapsing the first into the second
would discard real evidence, and turning the second into the first would
manufacture it. On the MonoVM side the split follows `ExecutionErrorKind`
directly: `InvalidOperation` and `RuntimeLimitExceeded` are the program's,
while `LinkingError` (the request named something unresolvable) and
`InvariantViolation` (a VM bug) are not. Only the classification is compared;
the diagnostic text beside it is each engine's own wording.

`exhausted` is separate from `aborted` because the two sides meter different
resources: the LeanerIR interpreter is fuelled, MonoVM is gas- and heap-
bounded. Exhaustion on either side is inconclusive for semantic equality — a
test must raise the budget, never record exhaustion as agreement or mismatch.
Requests must set a finite gas budget so runs terminate on divergent
programs.

Semantic equality compares outcome kinds, returned values, resource effects,
and abort codes. Abort locations, error stages, and diagnostic text are
carried for triage but not compared: two independent implementations
legitimately differ in how they describe a failure, and the normalization
step must not quietly encode either side's rendering as the expected one.

The response also carries resource writes, deletions, and events once those
observables are supported on both sides. Gas and MonoVM GC counts are retained
as diagnostics but are not part of semantic equality. The response header
echoes the adapter's compiler and native ABI identity, because the Lean Move
frontend compiles through the separately built `APTOS_MOVE_CLI` binary while
the linked adapter compiles through the workspace library: both come from the
same repository, but a stale CLI build can silently diverge. Tests log both
identities, and the setup documentation requires building the CLI and the
static library from the same checkout.

The first vertical supports `Unit`, booleans, fixed-width integers, addresses,
and vectors of those types. The schema then grows to nominal structs/enums and
resource effects. The adapter, not Lean, is responsible for translating
MonoVM frame storage into the stable recursive value schema.

## Rust implementation

Add a small `staticlib` adapter crate (working name `mono-move-lean-link`)
as a workspace member next to the mono-move crates. It depends only on the
production mono-move crates, never on `mono-move-testsuite`, whose engine
pulls in the legacy MoveVM and test-only natives.

The execution core is the call machinery of the MonoMove transaction
executor (`mono-move-aptos-transaction-executor`), which is the narrow
reusable engine this design called for: `production_natives`, lazy loader
construction, an idle `InterpreterContext` over a plain `GasMeter` budget,
`call_function` with BCS argument placement (`place_args`, covering signers
and `&signer` parameters), and type arguments through `intern_type_tag`.
Those call helpers are crate-private today; making them public is a
mono-move-owned change, not something the adapter works around. Compiling
sources stays a thin adapter-owned step over `move-compiler-v2`, following
the testsuite's compile module as the working example.

Two runtime-API consequences fall out of the value schema:

- Direct calls meter with a plain per-run `GasMeter` budget — MonoMove gas
  units are uncalibrated, so the budget is a termination bound and
  diagnostic, never a compared quantity — and `with_heap_size` bounds the
  heap; the adapter threads the request's limits through both.
- Input marshalling is solved by the executor's BCS placement; result
  extraction is not. Reading return values out of the root frame is still
  `_for_test`-only, so the remaining mono-move-owned addition is a supported
  serializer for root-frame results by return type — the BCS inverse of
  argument placement — with vectors first and nominal values later.

Execution inside one request is serialized. Separate Lean test processes may
run independently; no mutable MonoVM state is global across calls.

## Lake integration

Lake should own the native dependency:

1. A custom Lake target runs Cargo in release mode and produces the Rust
   `staticlib` in a content-addressed target directory.
2. A second target compiles the C shim.
3. `moreLinkObjs` attaches both outputs only to the e2e test driver.
4. The target trace includes the Cargo lockfile, relevant Rust sources, target
   triple, compiler profile, and native ABI version.

This differs deliberately from the leaner-rust exporter precedent, where the
Lean driver runs Cargo at test runtime and caches by input hash: a subprocess
can be built lazily, but a linked library must exist at Lake link time, so
Lake targets must own this build. Custom targets and target-valued
`moreLinkObjs` cannot be declared in TOML, so the consuming package moves
from `lakefile.toml` to an equivalent `lakefile.lean`.

Lake's `extern_lib` mechanism should not be used. It is not deprecated, but it
attaches at package scope: Lake links a package's extern libraries into every
executable of that package and of every downstream dependent. Only the native
test executable needs the foreign symbols, so the per-executable
`moreLinkObjs` scoping is the correct one; ordinary LeanerIR libraries and the
language server stay free of the native dependency.

The Lean-facing module is intentionally small:

```lean
@[extern "leaner_monovm_run"]
opaque runBytes (request : @& ByteArray) : IO ByteArray

def run (request : Request) : IO Response := do
  decodeResponse (← runBytes (encodeRequest request))
```

Elaborating this module needs no native symbols (`runBytes` is opaque), but
evaluating it does: under `lake env lean` or `#eval` the interpreter has no
`leaner_monovm_run` and fails. Harness calls therefore run only through the
linked test executable.

## Test integration

Executable Move fixtures contain concrete calls and expected input types. They
remain automatically discovered beside the existing Move-to-LeanerLang
baselines. For every call the e2e driver compares:

1. MonoVM outcome versus the original validated-LIR interpreter outcome;
2. the original outcome versus freshly elaborated LeanerLang LIR;
3. the final resource/event state for every represented observable.

Printer baselines remain useful and independent: a semantic mismatch must not
be hidden by updating an `.exp.lean` file.

### Recorded results

Agreement alone does not say *what* the engines agreed on, so each fixture
pairs with a side-by-side `.exp` file recording every engine's normalized
outcome per step. That makes the observed semantics a reviewable asset: a
change in what MonoVM or the interpreter returns shows up as a source diff.

The recorded outcomes are the whole check. Equal lines are agreement, and a
divergence appears as a diff of the line that changed, so the baseline is the
single pass/fail mechanism and there is no second assertion to keep in step
with it. Agreement is therefore recorded as nothing at all — the equal outcome
lines already say it, and a redundant "agree" line in a baseline is a line
that can disagree with the data beside it.

A divergence *is* named: the step records an explicit `ERROR:` line for each
comparison that failed, so a finding reads as a finding rather than as one
more changed value in a diff. Exhaustion never produces one — an
`inconclusive` outcome, one engine exhausting its budget where the other did
not, states on its face that the step is evidence about neither, and the two
sides meter different resources. A recorded `ERROR:` line is an open finding
that belongs in the register above, not an accepted expectation.

Only normalized content is recorded, so the file is deterministic. Gas usage,
collection counts, abort locations and messages, and the adapter's build
identity are triage output and never enter a baseline; recording gas would make
the file churn on unrelated gas-schedule changes and would invite reading it as
an expectation. Normalization is also where representation differences are
removed — integers compare numerically, addresses by identity rather than
spelling — so that a diff always means a semantic difference.

### Mismatch triage

Neither side is the baseline, so a mismatch is triaged, never baselined away:

- The failure report prints both outcomes symmetrically, with the full
  request, so the case reproduces on either side in isolation.
- The legacy MoveVM is the mature third arbiter. `mono-move-testsuite`
  already executes Move sources differentially on MoveVM and MonoVM, so the
  minimized program can be dropped into that suite to vote MonoVM right or
  wrong before touching the Lean side.
- The finding is classified as a MonoVM bug, a Leaner frontend or interpreter
  bug, or a semantic-boundary question neither implementation settles; the
  last kind feeds the LIR semantics documents rather than a code fix.
- The minimized repro becomes a regression test owned by whichever side was
  wrong, in addition to the e2e fixture.

Minimum gates are:

- Rust ABI encode/decode and panic-containment tests;
- one successful scalar call and one abort through the linked Lean test
  executable;
- one exhaustion case on each side, reported as inconclusive rather than as
  agreement or mismatch;
- repeated batch calls with deterministic results and no retained state;
- a Move stdlib consumer vertical, followed by composite values, mutable
  references, resources, and events;
- the same tests under debug and release Lean builds, with MonoVM always built
  using the explicitly selected Cargo profile.

## Non-goals

- The linked adapter is not a general Lean binding to MonoVM internals.
- It does not execute Move specifications, invariants, or proof obligations.
- It does not make MonoVM part of the trusted proof kernel.
- It does not compare gas or GC behavior until those are explicitly modeled.

## Findings register

Findings the harness has produced. Each one names the side it belongs to, so
the register stays a triage record rather than a to-do list.

- **M3, 2026-08-27 — Move `while`-loop lowering is not executable-capable.**
  A fixture whose body uses a Move `while` loop fails `prepareExecution`
  with three `LIR-SEMANTIC-TYPE` errors (pattern assignment result not
  `Unit`, valueless `break` targeting a non-`Unit` loop, non-`Unit`
  fallthrough body). This is a Leaner frontend/semantic-preparation gap,
  not a MonoVM issue: the round-trip executable corpus simply never lowered
  a Move loop. The differential fixtures use recursion until the loop
  lowering is classified; the first `while`-loop fixture added afterwards
  becomes its regression test.

- **M4, 2026-08-27 — the LeanerLang printer emitted unreadable `>>` in vector
  literals.** *Fixed.* A `vector<vector<u64>>` literal rendered as
  `vector<Vector<u64>>[…]`, whose `>>` the LeanerLang lexer reads as a shift
  token, so canonical source did not re-import. Both the semantic renderer
  (`LeanerLang/Print.lean`) and the formatter (`LeanerLang/Print/Layout.lean`)
  built that type argument by hand instead of through the helper each module
  already had for exactly this (`typeArguments` / `closeAngles`); the type
  printers used the helper, so only expression positions were affected. Both
  now route through their helper.
  `MoveToLeanerLang/move_vector_literals.move` is the regression test.
  This is a Leaner printer bug, not a MonoVM or semantic one — it was found
  because the round-trip leg re-elaborates what the printer emits, which no
  printer baseline checked for nested generic literals.

- **M4, 2026-08-27 — the adapter reported a runtime failure as a harness
  error.** *Fixed.* A `u8` overflow reached the comparison as
  `error (adapter run failure: …)`, which claims no outcome was obtained. It
  is an outcome of the program: MonoVM classifies it `InvalidOperation`. The
  adapter now propagates every execution error the program produced as the
  `failed` outcome carrying that classification, and keeps `error` for the
  cases that really are the harness's — `LinkingError`, which means the
  request named something unresolvable, and `InvariantViolation`, a VM bug
  that must stay loud rather than become something to compare.

- **M4, 2026-08-27 — the engines classify arithmetic overflow differently.**
  *Open; semantic-boundary question, owned by neither engine yet.* With the
  adapter fixed above, the comparison is now between two program outcomes and
  they still disagree: for `u8` `x + 200` with `x = 100`, MonoVM reports
  `failed InvalidOperation` while the Leaner path aborts carrying code `300`.
  Both detect the overflow; they disagree about whether that is a runtime
  failure or an abort with a code. Expectation is that the Leaner side should
  also report an under/overflow failure rather than synthesize an abort code,
  but this is the third triage category — it settles in the LIR semantics
  documents, not by patching one side to imitate the other, and the adapter
  must not reclassify a failure as an abort to manufacture agreement.
  `MonoDifferential/arithmetic_overflow.move` holds the case, and its
  recorded `ERROR:` line keeps the divergence visible in review; it is the
  first fixture whose baseline records an open finding rather than agreement.

- **M4, 2026-08-27 — `prepareExecution` did not scale to a package-size
  unit.** *Fixed by the LIR validation rework, re-measured 2026-08-28.* The
  same staged MoveStdlib unit (15 namespaces, 291 functions) that previously
  ran **more than 16 CPU-minutes without completing** now prepares in **2 ms**,
  and `reimportUnit` over it in **3 ms** against a previously observed **57+
  CPU-minutes**. Nothing in this design routed around it, so nothing has to be
  unwound. The remaining stdlib blockers are the ones below, and they are not
  about cost.

- **M4, 2026-08-28 — Move's vector natives had no executable meaning.**
  *Partly fixed.* `vector::empty` and `vector::length` now lower to the LIR
  `vector` and `length` operations in the Move frontend, following the logical
  bytecode model, which already treats these natives as operations rather than
  callable functions. `MonoDifferential/stdlib_vector.move` compares clean
  across all three engines. Still calls, and so still absent bodies: the
  length-changing natives (`push_back`, `pop_back`, `swap`, `move_range`,
  `destroy_empty`), which need LIR operations that do not exist yet, and
  `borrow`/`borrow_mut`, which need the value-borrow to place-borrow
  normalization — LIR has the place vocabulary but the encoder never builds
  places. Element access is available meanwhile through Move's index notation.
  The other 12 stdlib natives (hash, BCS, string, signer, `cmp`, `mem`,
  feature flags) are deliberately left absent per the milestone's scope; a
  fixture reaching one records the resulting error in its baseline.

- **M4, 2026-08-28 — the frontend built value borrows, which are not
  executable.** *Fixed.* LIR distinguishes borrowing a *place* from borrowing
  a *value*, and classifies the value form `unsupportedExecutable`: it names
  no location to point at. The Move frontend only ever built the value form,
  because it never constructed places at all — so no Move `&x` prepared for
  execution, and every gap in the vector story traced back to this one missing
  capability.

  The encoder now recognizes the storage path an expression denotes — locals,
  parameters, dereferences, and indexes — and borrows it as a place.
  `vector::borrow`/`borrow_mut` become a borrow of the indexed place, and
  `push_back` writes its place directly rather than borrowing it and reading
  back through the loan, which was the borrow conflict recorded here earlier.
  Move's index notation compiles to `vector::borrow`, so `v[i]` runs too.

  `MonoDifferential/stdlib_vector_push.move` now agrees across all three
  engines on realistic vector code: build with `push_back`, read with
  `borrow` and index notation, and sum in a `while` loop. Printer baselines
  did not move at all — a place borrow and a value borrow print the same
  `&x` — so this is representation, not surface.

  Field paths are still not recognized, since a place field names an interned
  `NameId` rather than the field string the frontend carries; a borrow through
  a struct field therefore still takes the value form and stays
  non-executable.

- **M4, 2026-08-28 — `vector::pop_back` needs a value-yielding statement
  sequence the printer cannot render.** *Open.* `pop_back` both shortens the
  vector and yields the element it removed, so it lowers to a binding chain:
  read the last element, write the shortened vector back to its place, yield
  the binding. The IR shape works — the lowering executes and round trips in
  isolation — but printing it inside a loop body emits the chain's tail twice:

  ```text
  let e :=
    do
      let «pop#7» := self[self.length - 1]
      self := core.prim.popVector(self)
      return «pop#7»
      return «pop#7»
  ```

  which then fails to re-import with `a `do` block has more than one final
  return`. `MoveStdlib/sources/configs/features.move` is the repro, through
  the loop that pops a feature vector. Rendering `collectTail`'s tail as a
  value rather than in tail position does not fix it, so the duplication has
  a second source not yet found.

  `pop_back` therefore stays an ordinary call, reported as an absent body,
  and the `popVector` primitive it would need was removed rather than left
  without a producer. Re-adding it is mechanical — `swapVector` in the same
  commit is the worked example of the whole path, from the primitive through
  validation and the surface plumbing to the frontend lowering. The blocker
  is the printer, not the IR.

- **M4, 2026-08-28 — the canonical printer's fixed point held only by
  coincidence.** *Fixed.* `ordered_map.exp.lean` recorded
  "generated Move LeanerLang is not a canonical fixed point" as its expected
  output, so the defect was already blessed into the corpus.
  `renderSemanticNamespace` wrapped a single-statement block in `do` while the
  parser produces the bare statement, so a unit's semantic form depended on
  whether its frontend happened to introduce a block — a distinction invisible
  in the printed source. The fixed point then held only where the two forms
  laid out identically, and broke as soon as an unrelated width changed. A
  block whose single entry binds nothing now renders as that entry. The
  formatter itself was never at fault: it is idempotent on its own output,
  which is what isolated the first render as the outlier.

- **M4, 2026-08-28 — the two paths into LIR disagreed about `length`.**
  *Fixed.* The LeanerLang elaborator lowered Move `x.length` to a call to
  `0x1::std::vector::length` while the Rust profile lowered it to the LIR
  primitive. Once the Move frontend produced the primitive, canonical source
  stopped round tripping: printing a primitive and reading it back yielded an
  unresolvable call. Both profiles now lower to the primitive, and the
  receiver is read through rather than borrowed, matching what the primitive
  takes. A frontend and an elaborator that lower the same surface differently
  is a round-trip bug waiting for the first construct where they meet.

- **M4, 2026-08-28 — specification `let` bindings blocked execution
  preparation.** *Fixed.* A `let` inside a function's `spec` block binds a
  local in that function's *executable* local table, typed in the logical
  domain, and the execution scan required every local declaration to be
  executable. One `num` binding in a specification therefore rejected the
  whole unit — `error.move`, `fixed_point32.move`, `option.move`,
  `vector.move` and most other stdlib modules, independently of natives.

  Execution preparation no longer scans local declaration types, matching the
  namespace level, where specification declarations are already skipped in
  this mode. Nothing is lost: a local the body uses carries its type on every
  expression that reads or writes it, and those are scanned; a local the body
  never uses cannot affect execution whatever its type; parameter types are
  covered by the signature scan.

  Measured on the staged MoveStdlib unit, this and the mutability fix together
  take execution preparation from 91 diagnostics across 7 distinct codes to 65
  across 5. What remains there is absent native bodies, value borrows the
  frontend cannot yet turn into places (struct field paths), and three
  borrow/ability findings not yet triaged.

- **M4, 2026-08-27 — the driver reused original function indices after the
  round trip.** *Fixed.* Declarations are rendered in canonical (sorted)
  order, so a function's position within a namespace is not stable across
  the round trip. The driver resolved the callee once in the original unit
  and reused that index for the round-trip executable, silently calling a
  different function; it surfaced as `argumentArity` errors and one wrong
  result only once a fixture had several functions whose canonical order
  differed from source order. Each unit is now resolved independently by
  name. A harness bug that produced false divergences, not a finding about
  either engine — and an argument for recording each engine's outcome rather
  than only a comparison, since the recorded `.exp` made the cause obvious.

## Implementation plan

Status: planned 2026-08-27; M0–M3 and M4a implemented on 2026-08-27, M4b
partly delivered 2026-08-28 (see the findings register). The milestones
implement the minimum gates above in order; each one is independently
reviewable and leaves the tree building and testing green. M0–M1 are pure
Rust, M2 makes the link, M3 makes it differential, and M4–M5 grow semantic
coverage. The adapter depends only on production mono-move crates from M0 —
the execution core is the transaction executor's call machinery — so no
test-only dependency ever enters the staticlib. MonoMove-side API additions
carry mono-move `TODO` labels under its `AGENTS.md` conventions; adapter and
Lean changes follow the lean-tree change discipline.

### Repository layout

```text
third_party/move/
  mono-move/lean-link/                    Rust adapter crate (workspace member)
    Cargo.toml                            crate-type rlib + staticlib
    src/lib.rs                            C ABI, panic containment, entry
    src/payload.rs                        JSON request/response and value schema
    src/marshal.rs                        value schema ↔ BCS args/results
    src/engine.rs                         compile → load → run over mono-move
    tests/                                ABI, payload, and containment tests

  lean/leaner-e2e-tests/
    lakefile.lean                         replaces lakefile.toml in M2
    shim/monovm_shim.c                    C shim against the pinned lean/lean.h
    LeanerE2ETests/MonoVM/Link.lean       opaque extern + transport + codecs
    LeanerE2ETests/MonoVM/Payload.lean    request/response/value schema
    LeanerE2ETests/MonoVM/Directives.lean fixture directive parser
    LeanerE2ETests/MonoDifferential/Baseline.lean   differential driver
    LeanerE2ETests/MonoDifferential/*.move executable fixtures
```

Public exposure of the executor's call helpers (`call_function`,
`place_args`, `param_types`) lands in `mono-move-aptos-transaction-executor`;
the supported root-result serializer lands in `mono-move-runtime` next to
today's `_for_test` readers. Both are mono-move-owned.

### Payload v1

Deterministic JSON (stable field order, no maps whose iteration order can
vary) on both sides of the C boundary. Integers are decimal strings so every
width round trips exactly; addresses are canonical `0x…` strings.

```json
{ "version": 1,
  "compile": {
    "sources":   [{ "name": "counter.move", "text": "..." }],
    "stdlib":    "move-stdlib",
    "addresses": { "std": "0x1" },
    "language":  2 },
  "limits": { "gas": 1000000000, "heap": null },
  "calls": [
    { "function": "0x42::counter::bump",
      "args": [ { "kind": "integer", "width": 64, "value": "3" } ] } ] }
```

```json
{ "version": 1,
  "identity": { "abi": 1, "rustc": "...", "profile": "release" },
  "outcomes": [
    { "kind": "returned", "values": [ ... ],
      "gas_used": 412, "gc_count": 0 } ] }
```

One entry per request call, in request order, each exactly one of:

- `returned` — `values` in the stable recursive schema, plus mutable-reference
  outputs once M4 reads back out-parameters;
- `aborted` — `code`, and `message`/`location` as triage-only fields;
- `exhausted` — which resource (`gas` or `heap`) ran out;
- `error` — `stage` (`compile`/`load`/`run`/`abi`/`internal`) and message.

The value schema starts as `unit`, `bool`, `integer` (width-tagged), and
`vector`, and grows to `address`, `signer`, and nominal values with the
verticals that exercise them. `gas_used` and `gc_count` are diagnostics and
never enter semantic equality.

### M0 — Adapter staticlib with the versioned C ABI

- Add `mono-move-lean-link` as an aptos-core workspace member next to the
  mono-move crates with `crate-type = ["rlib", "staticlib"]`. The rlib keeps
  ordinary Rust tests linkable; the staticlib is the artifact Lake links.
- Export `leaner_monovm_abi_version`, `leaner_monovm_run`, and
  `leaner_monovm_buffer_free` exactly as in the boundary section. Guard with
  `compile_error!` under `cfg(panic = "abort")` and wrap the whole run in
  `catch_unwind`, encoding a caught panic as an `internal`-stage error.
- Implement payload v1 encode/decode with serde. Every domain failure —
  compile, load, run, malformed request — is an outcome or error stage inside
  the response; the `int32_t` stays transport-only.
- Build the execution core over the transaction executor's call machinery,
  mirroring its own e2e recipe: `GlobalContext::with_num_execution_workers(1)`
  with one execution guard per request, `production_natives(&guard)`, an
  in-memory module provider fed by the adapter's compile step, an idle
  `InterpreterContext` over the request's `GasMeter` budget and heap size,
  and one `call_function` per call. The prerequisite mono-move change —
  making `call_function`, `place_args`, and `param_types` public — lands
  first.
- Tests: payload round trips; malformed requests yield `abi`-stage errors;
  the `catch_unwind` wrapper is unit-tested against a forced panic; a symbol
  test inspects the built staticlib archive for the three exported names.

Gate: `cargo test -p mono-move-lean-link` passes; a scalar call and an
abort both return normalized outcomes through the plain Rust entry the C ABI
calls; and the crate's dependency graph contains no test-only crates.

### M1 — Supported marshalling and batched execution

- Marshal inputs through the executor's placement path: the adapter encodes
  each request value as BCS against the loaded parameter type and hands
  `place_args` the bytes and signer addresses — signers, `&signer`, and
  generics (request `TypeTag`s through `intern_type_tag`) come with it. No
  adapter-side frame-slot arithmetic.
- Add the one remaining mono-move-owned extraction API to
  `mono-move-runtime`: a supported serializer for root-frame results by
  return type, the BCS inverse of placement, replacing the `_for_test`
  readers. Vectors first, nominal values with M5.
- Thread the request limits: the `GasMeter` budget per call and
  `with_heap_size` at construction. Exhaustion must surface as the distinct
  `exhausted` outcome, never as a generic error: `RuntimeStatus` carries only
  `Success` and `Aborted`, so out-of-gas arrives through the error path
  today, and the adapter needs a positive signal — classifying the runtime
  error or observing a drained `gas_balance()` — to separate it.
- Compile and load once per request; run every call inside the one guard;
  capture `gc_count` and remaining gas as diagnostics. Calls within a request
  are serialized; nothing mutable outlives the request.

Gate: one request containing a multi-call batch — scalar success, vector
success, abort, and gas exhaustion — returns deterministic normalized
outcomes, and replaying the identical request twice produces byte-identical
responses.

### M2 — C shim, Lake targets, and the first linked call

- Write `shim/monovm_shim.c` against the pinned toolchain's `lean/lean.h`:
  copy the response into a Lean `ByteArray`, call `leaner_monovm_buffer_free`,
  and turn a nonzero transport status into a Lean `IO` error. The shim touches
  no other Lean object representation.
- Migrate `leaner-e2e-tests` from `lakefile.toml` to an equivalent
  `lakefile.lean` (same requires, default targets, and test driver), then add
  two custom targets: one runs Cargo in the explicitly selected release
  profile against the worktree's workspace and exposes the staticlib, one
  compiles the shim with `leanc`. Attach both through target-valued
  `moreLinkObjs` on `LeanerE2ETestDriver` only, adding whatever system
  libraries the Rust staticlib needs (`-lm -ldl -lpthread` are expected;
  confirm at implementation) via `moreLinkArgs`.
- The Cargo target's trace covers the adapter crate sources, the workspace
  `Cargo.lock`, the profile name, the target triple, and the native ABI
  version, so a change on any of them relinks the driver. Running Cargo from
  the worktree root is what makes the same-checkout requirement structural.
- Add `LeanerE2ETests/MonoVM/Link.lean` with the opaque `@[extern]` function,
  `Lean.Data.Json` codecs, and an assertion that the native ABI version
  matches the Lean-side constant — a mismatch must produce an actionable
  rebuild hint, not a decode error. Log the response identity next to the
  resolved `APTOS_MOVE_CLI` path.

Gate: the linked test executable performs one successful scalar call and one
abort; the module still elaborates under `lake env lean` with no native
symbols; the build links and runs, with MonoVM always built in the explicitly
selected Cargo release profile (this Lake version exposes no separate
release-mode Lean executable build, so the Lean side has a single build
shape); no other package gains a native dependency.

### M3 — Differential driver over executable fixtures

- Fixtures are ordinary Move source files whose directives are Move comments
  in the same `// RUN: publish` / `// RUN: execute <addr>::<m>::<f> --args …`
  grammar the mono-move differential suite already consumes; the mono-move
  parser rejects unknown modifiers, so Lean-side knobs (`--fuel`, `--gas`)
  live in a namespaced `// LEANER:` directive instead of extending `// RUN:`.
  A file that runs in both harnesses is the triage path made concrete: the
  minimized repro drops directly into the legacy-VM arbiter.
  `LeanerE2ETests/MonoDifferential/` holds them; the driver discovers them
  automatically. They deliberately carry no `.exp.lean` printer expectation,
  but each pairs with a side-by-side `.exp` recording every engine's
  normalized outcome, per **Recorded results** above.
- Implement the Lean-side directive parser independently; it is small and its
  grammar is fixed by this design.
- Per execute step the driver runs three interpretations: the linked MonoVM
  call; the original validated LIR through `prepareExecution` with
  `LeanerIR.Move.semantics` and `Interpreter.run` under the directive's fuel;
  and the same interpretation after the canonical LeanerLang round trip
  (print, re-elaborate, lower, validate, prepare). The frontend side reuses
  `Transpiler.Cli.exportMoveFiles` and `Transpiler.LIR.Backend.fromXast`.
- Normalize both sides into one outcome shape: Lean `.returned` values and
  `.threw .abort` with the code among its arguments map onto `returned` and
  `aborted`; `InterpreterError.outOfFuel` and the adapter's `exhausted` both
  map to an `inconclusive` outcome that the baseline states explicitly — never
  a result, so it can never read as agreement; every other interpreter error
  maps to `error`. Normalization is a pure Lean function and gets direct unit
  tests over exhausting, aborting, and value-carrying inputs — that is the
  harness's own regression test, with no production hook.
- A divergence is read off the recorded outcomes; triage then reruns the
  fixture with the full request JSON and both toolchain identities in hand,
  per **Mismatch triage** above.

Gate: fixtures covering scalar success, abort, one exhaustion case per side
recorded as inconclusive, and a repeated batch with identical outcomes;
normalization unit tests green; the suite passes under both Lean build modes.

### M4 — Composite values and the Move stdlib vertical

These are two independent halves, and the evidence separates them cleanly.
Composite values need no stdlib module in the Lean unit, because Move 2
vector literals and address constants are built-in expression forms; only
*calling* stdlib functions forces stdlib bodies into the unit. M4a is
therefore implemented while M4b is blocked on a cost outside this design.

#### M4a — Composite values (implemented 2026-08-27)

- Grow the value schema and marshalling to addresses and nested vectors of
  supported element types. `MonoDifferential/composite_values.move` covers
  vectors, nested vectors, empty vectors, addresses, and signed integers in
  one single-module unit.
- Compare addresses by identity, not by spelling: the two engines render the
  same address differently (shortest hex versus padded), so both normalizers
  canonicalize before comparison. Comparing the rendered strings would report
  a mismatch for a representation difference.

Gate: met — all three interpretations agree on every step of the composite
fixture, and the recorded `.exp` shows the agreed values. Two findings came
out of writing it, both now fixed and in the register: the printer emitted
unreadable `>>` in nested vector literals, and the driver reused original
function indices after the round trip.

Still open in the value schema, and deferred to M5 with the rest of the
reference story: mutable-reference out-parameters (read back from the root
frame after the run on the MonoVM side; normalized from the interpreter's
reference outputs on the Lean side). The directive grammar has no spelling
for a `&mut` argument yet either.

#### M4b — Move stdlib vertical (partly delivered)

- The Lean and MonoVM sides reach the stdlib by different routes, and the
  milestone is about making them agree. MonoVM registers the natives; the Lean
  side lowers the ones LIR can express to operations, which is what the logical
  bytecode model already does. A fixture using those needs no stdlib module in
  its unit at all — staging the package is not the mechanism, and is not used.
- Delivered: `vector::empty` and `vector::length`.
  `MonoDifferential/stdlib_vector.move` compares clean across all three
  interpretations.
- Delivered since: `push_back` through the new LIR push operation, and
  `borrow`/`borrow_mut` as place borrows, which also makes Move's index
  notation and `while` loops over vectors executable.
- Delivered since: `swap` through the new LIR swap operation, and
  specification-only locals no longer blocking execution preparation.
- Not delivered, with owners named in the findings register: `pop_back`,
  which needs a printer fix rather than IR work; `move_range` and
  `destroy_empty`; and borrows through struct fields.
  The non-vector natives are out of scope by decision: a fixture reaching one
  records the resulting error in its baseline rather than being made to pass.
- Cost is no longer a factor anywhere here; see the register.

Gate (partly met): the stdlib consumer vertical
compares clean across all three interpretations, or every divergence is a
triaged finding with an owning side.

### M5 — Resources and events

- Carry initial resource state in the request: a stable type schema plus BCS
  bytes, seeded into an in-memory `StateView` and served through the
  production `StateViewModuleProvider`/`StateViewResourceProvider` from
  `mono-move-aptos-state-view-providers` — the providers the transaction
  executor itself runs on, not a test analogue.
- Report resource writes, deletions, and events per call — from the
  interpreter's read-write set drained per call (one context per call with
  `finish()` into `SessionEffects`, or a supported read-write-set reader)
  plus the transaction-context extension's event store on the MonoVM side,
  and from the profile's global-state updates on the Lean side — and include
  the final resource/event state in semantic equality as the design requires.
- Grow nominal value marshalling to match resource payloads.

Gate: a fixture pair that publishes, mutates, and deletes a resource and
emits an event records equal final state on every engine; normalization unit
tests cover a seeded resource divergence.

### Deferred-work register

- Type arguments on generic entries: the mechanism exists (`intern_type_tag`
  over request `TypeTag`s, and `call_function` takes the type list); the
  first verticals stay monomorphic only to keep the value schema small.
- Script transactions and module-publishing payloads.
- Shared named-address assignments across the XAST frontend and the adapter:
  the first verticals use fully numeric module addresses so both compilers
  agree without a shared address table.
- Full-transaction mode: `execute_user_transaction` with seeded accounts,
  compared against the legacy AptosVM through the e2e fake executor — the
  transaction executor's own e2e test already runs exactly that differential.
  Deferred because prologue, epilogue, and fee charging inject framework
  noise (gas-embedded writes and fee events) the comparison must filter, and
  the Lean side would have to model chain semantics the design keeps out of
  scope. Revisit when a production-shaped observables vertical is wanted.
- Gas and GC comparison modeling (diagnostics only until modeled).
- Automation for the legacy-VM arbiter step of mismatch triage.
- Faster local-iteration Cargo profiles; any change stays a trace-affecting
  lakefile edit, never an untraced environment default.
- Non-Linux link support for the shim and staticlib.
