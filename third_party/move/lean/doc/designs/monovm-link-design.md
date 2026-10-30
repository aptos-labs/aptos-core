# Linked MonoVM differential execution

## Purpose

Verex needs an executable comparison partner for Move programs: a test
compares the behavior of compiler-produced Move code with the original
validated VIR and with the VIR obtained after a VerexSL source round trip.

This is differential testing, not validation against a trusted reference.
MonoVM and the Verex pipeline are both new implementations; neither side is
the baseline. A failing comparison is a finding that can locate a bug in
either implementation, or a genuine ambiguity in the shared understanding of
Move semantics, and that symmetry is the point of the harness.

MonoVM is a Rust library in the same repository, linked into the Lean e2e
test driver rather than invoked as a CLI or a long-lived subprocess:

```text
Move sources ──► compiler + MonoVM ───────────────► observable outcome
      │
      └────────► Move frontend ─► validated VIR ─► VerexIR interpreter
                                      │
                                      └─► VerexSL ─► VIR interpreter
```

The first comparison checks the Move-to-VIR semantic boundary; the second
checks that canonical VerexSL printing and re-elaboration preserve it. This
is testing evidence, not a proof that either runtime implements Move.

## Boundary

Rust has no stable ABI, and MonoVM's arenas, loaders, and runtime pointers
must not escape their execution guard. The linked interface has three
layers:

```text
Lean `ByteArray`
    │ `@[extern "verex_monovm_run"]`
small C shim using `lean/lean.h`
    │ pointer + length C ABI
Rust static library
    │ compile, load, execute, normalize
MonoVM
```

Only owned bytes cross the Rust boundary. The C shim copies the Rust result
into a Lean `ByteArray`, calls the Rust deallocator, and returns an ordinary
Lean `IO` result; it is written in C against the pinned toolchain's
`lean/lean.h`, so the Rust crate never touches the Lean object ABI.

The Rust library exports a minimal versioned ABI:

```c
uint32_t verex_monovm_abi_version(void);
int32_t verex_monovm_run(
    const uint8_t *request, size_t request_len,
    struct verex_buffer *response);
void verex_monovm_buffer_free(struct verex_buffer response);
```

Every domain failure (compilation, loading, execution, malformed request)
travels inside the response payload. The `int32_t` status is transport
only: zero means the response buffer is valid; nonzero means no response
could be produced, surfaced in Lean as an `IO` error. `verex_monovm_run`
catches Rust panics and encodes them as internal harness errors; this
relies on unwinding, so the adapter crate refuses to compile under
`panic = "abort"` (`compile_error!` under `cfg(panic = "abort")`).

## Request and response

The payload is deterministic JSON (stable field order, no maps whose
iteration order can vary); integers are decimal strings so every width
round-trips, addresses canonical `0x…` strings. Its `version` is
independent of the native ABI version.

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

One request contains all calls for a package: MonoVM compiles and loads the
package once and executes every call while its `ExecutionGuard` is live.
Calls within a request are serialized; nothing mutable outlives the
request. Each call produces exactly one normalized outcome:

- `returned`: typed return values and mutable-reference outputs;
- `aborted`: code, with message and location as triage-only fields;
- `exhausted`: the gas or heap budget ran out;
- `failed`: the program failed at runtime without aborting, with the
  engine's classification;
- `error`: compilation, loading, ABI, or internal failure, with a stage.

`failed` and `error` are different claims. A Move runtime failure
(overflow, out-of-bounds index, missing resource) is an outcome of the
program and reaches the comparison; `error` says no outcome was obtained.
On the MonoVM side the split follows `ExecutionErrorKind`:
`InvalidOperation` and `RuntimeLimitExceeded` are the program's,
`LinkingError` and `InvariantViolation` are not. Only the classification is
compared.

`exhausted` is separate from `aborted` because the sides meter different
resources: the VerexIR interpreter is fuelled, MonoVM gas- and heap-bounded.
Exhaustion on either side is inconclusive for semantic equality; a test
raises the budget, never records exhaustion as agreement or mismatch.
Requests set a finite gas budget so divergent programs terminate.

Semantic equality compares outcome kinds, returned values, resource effects,
and abort codes. Abort locations, error stages, diagnostic text, gas and GC
counts are carried for triage and never compared. The response header
echoes the adapter's compiler and ABI identity, because the Lean frontend
compiles through the separately built Move CLI while the adapter compiles
through the workspace library; tests log both, and both are built from the
same checkout.

The value schema covers `Unit`, booleans, fixed-width integers, addresses,
and vectors of those; it grows to nominal values and resource effects with
the verticals that need them. The adapter, not Lean, translates MonoVM
frame storage into the schema.

## Rust implementation

A small `staticlib` adapter crate (`mono-move/lean-link`) is a workspace
member next to the mono-move crates and depends only on the production
crates, never on `mono-move-testsuite`. Its execution core is the call
machinery of the MonoMove transaction executor: `production_natives`, lazy
loader construction, an idle `InterpreterContext` over a plain `GasMeter`
budget, `call_function` with BCS argument placement (`place_args`, covering
signers and `&signer`), type arguments through `intern_type_tag`. Compiling
sources is a thin adapter-owned step over `move-compiler-v2`.

Inputs are marshalled by the executor's BCS placement; results are read
from the root frame by a supported serializer by return type, the BCS
inverse of placement. Gas is a termination bound and diagnostic, never a
compared quantity; `with_heap_size` bounds the heap.

```text
mono-move/lean-link/
  src/lib.rs        C ABI, panic containment, entry
  src/payload.rs    JSON request/response and value schema
  src/marshal.rs    value schema ↔ BCS args/results
  src/engine.rs     compile → load → run over mono-move
lean/verex-e2e-tests/
  shim/monovm_shim.c                     C shim against lean/lean.h
  VerexE2ETests/MonoVM/Link.lean         opaque extern, transport, codecs
  VerexE2ETests/MonoVM/Payload.lean      request/response/value schema
  VerexE2ETests/MonoVM/Directives.lean   fixture directive parser
  VerexE2ETests/MonoDifferential/        driver and executable fixtures
```

## Lake integration

Lake owns the native dependency: a custom target runs Cargo in the selected
release profile and produces the staticlib in a content-addressed directory;
a second compiles the C shim; `moreLinkObjs` attaches both to the e2e test
driver only. The target trace covers the adapter sources, the workspace
`Cargo.lock`, the profile, the target triple, and the ABI version. Custom
targets cannot be declared in TOML, so the package uses a `lakefile.lean`.

`extern_lib` is not used: it attaches at package scope and would link the
foreign symbols into every executable of the package and its dependents.
Only the test driver needs them, so ordinary libraries and the language
server stay free of the native dependency.

```lean
@[extern "verex_monovm_run"]
opaque runBytes (request : @& ByteArray) : IO ByteArray
```

Elaborating this module needs no native symbols, but evaluating it does:
harness calls run only through the linked test executable, never under
`lake env lean` or `#eval`. The Lean side asserts that the native ABI
version matches its constant and reports a mismatch as a rebuild hint.

## Test integration

Fixtures are ordinary Move source files under
`VerexE2ETests/MonoDifferential/` whose directives are Move comments in the
`// RUN: publish` / `// RUN: execute <addr>::<m>::<f> --args …` grammar the
mono-move differential suite consumes; Lean-side knobs (`--fuel`, `--gas`)
live in a namespaced `// VEREX:` directive. A file that runs in both
harnesses is the triage path made concrete. Per execute step the driver runs
three interpretations: the linked MonoVM call; the validated VIR through
`prepareExecution` and `Interpreter.run` under the directive's fuel; and the
same after the canonical VerexSL round trip (print, re-elaborate, lower,
validate, prepare), through `VerexMove.Frontend.Cli.exportMoveFiles` and
`VerexMove.Frontend.VIR.Backend.fromXast`.

Normalization maps both sides into one outcome shape: Lean `.returned` and
`.threw .abort` onto `returned` and `aborted`; `outOfFuel` and `exhausted`
onto an explicit `inconclusive`, never a result; every other interpreter
error onto `error`. Integers compare numerically and addresses by identity,
so a diff always means a semantic difference. Normalization is a pure Lean
function with its own unit tests.

**Recorded results.** Each fixture pairs with a side-by-side `.exp`
recording every engine's normalized outcome per step. Equal lines are
agreement, and agreement is recorded as nothing else: a redundant "agree"
line can disagree with the data beside it. A divergence records an explicit
`ERROR:` line per failed comparison, so a finding reads as a finding; an
`inconclusive` step never produces one. Only normalized content is
recorded; gas, collection counts, abort locations, messages and build
identity are triage output. Printer baselines remain independent: a
semantic mismatch is never hidden by updating an `.exp.lean` file.

**Mismatch triage.** Neither side is the baseline, so a mismatch is
triaged, never baselined away. The report prints both outcomes
symmetrically with the full request. The legacy MoveVM is the third
arbiter: `mono-move-testsuite` executes Move sources differentially on
MoveVM and MonoVM, so the minimized program votes MonoVM right or wrong
before the Lean side is touched. A finding is a MonoVM bug, a Verex
frontend or interpreter bug, or a semantic-boundary question neither
implementation settles; the last kind feeds the VIR semantics documents,
not a patch to one side. The minimized repro becomes a regression test of
whichever side was wrong. A recorded `ERROR:` line is an open finding, not
an accepted expectation.

## Resources and events

Initial resource state travels in the request as a stable type schema plus
BCS bytes, seeded into an in-memory `StateView` and served through the
production `StateViewModuleProvider` and `StateViewResourceProvider`, the
providers the transaction executor itself runs on. Each call reports
resource writes, deletions and events, from the interpreter's read-write set
drained per call and the transaction context's event store on the MonoVM
side, and from the profile's global-state updates on the Lean side; the
final resource and event state is part of semantic equality. Mutable-
reference out-parameters are read back from the root frame after the run on
the MonoVM side and normalized from the interpreter's reference outputs on
the Lean side, with a directive spelling for a `&mut` argument.

## Non-goals

- The adapter is not a general Lean binding to MonoVM internals.
- It does not execute Move specifications, invariants, or proof obligations.
- It does not make MonoVM part of the trusted proof kernel.
- It does not compare gas or GC behavior until those are explicitly modeled.
- Full-transaction mode (`execute_user_transaction` with seeded accounts
  against the legacy AptosVM) is out of scope: prologue, epilogue and fee
  charging inject framework noise the comparison would have to filter, and
  the Lean side would have to model chain semantics.
