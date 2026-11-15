# Verex end-to-end baselines

This package contains source-to-VerexSL baseline tests for the two frontend
paths:

```text
Move source -> compiler-v2 XAST -> validated Move VIR -> VerexSL or error
Rust source -> rustc MIR exporter -> validated Rust VIR -> VerexSL or error
```

A Rust baseline also lowers its rendering back and compiles each function
to its denotation, as verification does; a function with none is listed after
the source with the construct that stops it (`-- no denotation for …`).

Inputs and expectations are side by side in feature directories:

```text
VerexE2ETests/MoveToVerexSL/
  move_scalar.move
  move_scalar.exp.lean
  MoveStdlib/
    Move.toml
    sources/hash.move
    sources/hash.exp.lean

VerexE2ETests/RustToVerexSL/
  scalar.rs
  scalar.exp.lean
  raw_pointer.rs
  raw_pointer.exp.lean
```

The driver discovers every direct `.move` or `.rs` frontend input and every
immediate Move package identified by `Move.toml`, then derives side-by-side
`.exp.lean` paths. A successful expectation contains canonical VerexSL. An
unsupported or invalid input contains the normalized, source-located concrete
diagnostic as Lean line comments beginning with `-- error:`. The expectation
therefore remains valid target-language source. Adding a case requires no
driver registration.

The real MoveStdlib package is owned here. All 15 exported modules currently
produce parse-checked canonical VerexSL. Individual declarations which the
Move XAST boundary cannot yet represent remain visible as source-located
`-- unsupported Move declaration ...` comments in the corresponding module
baseline. Legacy generated Move-flavored Lean fixtures remain in the
transpiler package and read this canonical source package.

`lake test` runs an executable baseline driver, so checks run even when Lake
can reuse every compiled module. A mismatch reports a line diff. Use
`UB=1 lake test` to replace expectations; `UPBL` and `UPDATE_BASELINE` are
accepted aliases. Review the resulting source diff before committing it.

`VerexE2ETests/Check/` holds VerexSL checks: every `.lean` file below it,
at any depth, is a VerexSL source — modules with specifications, `verify`
commands, proof scripts where real mathematics lives — that the driver
elaborates in its own `lean` process under a heartbeat cap, several at once
(`VEREX_E2E_JOBS`, by default half the hardware threads; the processes
share those threads for the targets each proves at once, `verex.jobs`). What `lean`
prints is the baseline, verbatim, beside the source as `<name>.exp`,
following compiler-v2's baseline convention: a clean check has no
expectation file, a check that prints anything has exactly that output as
its expectation, and `UB=1` writes or removes the file. Positive and
negative tests are the same kind of file. Checks are grouped by language
feature (`Scalars/`, `Structs/`, `Enums/`, `Vectors/`, `Control/`,
`References/`, `Storage/`, `Calls/`, `Generics/`, `Specifications/`,
`Modules/`), with larger programs under `Examples/`; a file whose name ends
in `Errors` holds negative cases. The ledger is the
[roadmap](../doc/plan/roadmap.md). Everything
under `Check/` is VerexSL; the frontend paths keep their own directories.

Every check also runs under `VEREX_E2E_TIMEOUT`, described with the `verify`
suite below. A run reports every check that did not pass, not the first, and
keeps the file stems of those in the untracked standing record
`.lake/check-failed`, comma-separated, so that
`VEREX_E2E_SUITE=check VEREX_E2E_FILTER=$(cat .lake/check-failed)` reruns
them; a run removes the checks that matched from the record, adds the ones
that differed, and leaves the checks it did not run in it. The queue starts
with the recorded checks, then the largest source first.

`VerexE2ETests/SourceVerify/` verifies Move and Rust sources as
`verex-move verify` and `verex-rust verify` do
([`doc/designs/source-verification.md`](../doc/designs/source-verification.md)): every
`.move` and `.rs` file there, a Rust file with the `.spec.lean` items beside
it and a Move file with its proof file, is rendered as VerexSL and
elaborated, and its messages, in the coordinates of the Move, Rust,
specification, or proof file, are the baseline `<name>.exp` — none when the
source verifies. A Move file's rendering goes to `.verex/` beside it, where
its proof file's header finds it; the proof file is then elaborated on its
own, as the editor elaborates it, and its messages are `<name>.proof.exp`.

The unit of work of that suite is a *name*: a source file's stem (its
`.move` and `.rs`, with the proof-file check of a Move file), a crate's name
(`CargoFixture`), a package's name (`MoveStdlib`), or `StaleRendering`, the
test that a proof file whose Move file changed warns at its header.
`VEREX_E2E_FILTER` selects names, packages and `StaleRendering` among them,
and no filter runs every name. One verification already uses more than a
core, so a run of several names fans out: the driver verifies each name in a
child of itself — one name, one worker, and `verex.jobs` set to the threads
that worker plans for itself, unless the caller's `VEREX_OPTIONS` sets
`verex.jobs` itself — and prints `ok <name>` or `FAIL <name>` with the
child's output as each one ends, followed by the count and every name that
failed. `VEREX_E2E_JOBS` is the number of workers, by default half the
hardware threads bounded by one worker per 3 GB of available memory; one
worker runs the names in the driver's own process, where the first mismatch
ends the run. The parent records what it ran in two untracked files under
`.lake`: `verify-durations`, one `<milliseconds> <name>` per line, and
`verify-failed`, the names known not to verify, comma-separated, so that
`VEREX_E2E_FILTER=$(cat .lake/verify-failed)` reruns them. `verify-failed` is
a standing record rather than one run's list: a fan-out run removes the names
it verified from it and adds the ones it failed, and the names it did not run
stay, so a run of a selection never erases what another selection recorded.
The queue starts with the names of `verify-failed`, then the longest previous
duration first, a name without a recorded duration counting as the longest.

`VEREX_E2E_TIMEOUT` is the seconds one fixture's process may run, 240 by
default and none at all when it is `0`. It holds wherever the driver runs a
fixture in a child: each name of a fan-out `verify` run and each check, the
latter even with one worker, since a check is always elaborated in a `lean`
of its own. A `verify` run in the driver's own process — one worker — has no
timeout. At expiry the process and everything it started are killed, so a
`lake` leaves no `lean` behind. A timed-out name of `verify` is a failure
like any other: `FAIL <name>`, then `timed out after <n> s`, counted and
recorded. A timed-out check is reported in the collected error as
`<file> timed out after <n> s`, and the run fails. A timeout is never
recorded as a baseline: a killed process produced no output to compare, so
`UB=1` neither writes nor removes its `.exp`.

The `VEREX_E2E_SUITE` environment variable selects one suite: `move`, `rust`,
`verify`, `check`, `monovm`, or `monodiff`. The `monovm` suite is the linked MonoVM smoke check: it calls the
`mono-move-lean-link` adapter staticlib through the C shim that only the
`VerexE2ETestDriver` executable links (`moreLinkObjs` in the `lakefile.lean`;
the package libraries and the language server stay free of the native
dependency). Lake owns the native build — the `monovm_staticlib` target runs
Cargo in the explicitly selected release profile from this checkout on each
build, letting Cargo check all transitive dependencies. The resulting archive
trace controls relinking of the driver. The `monovm_shim` target compiles the
shim with `leanc`. The suite runs the whole
differential story only through the linked executable; `VerexE2ETests.MonoVM`
modules elaborate under `lake env lean` but do not evaluate there.

The `monodiff` suite is the three-way differential harness over the executable
Move fixtures in `VerexE2ETests/MonoDifferential/`. Each `.move` fixture
carries `// RUN:` execute directives and pairs with a side-by-side `.exp`
recording what all three engines produced for every step — the linked MonoVM
adapter, the validated Move VIR, and the VIR after the VerexSL round trip:

```text
step 0: 0x42::scalar::add(1, 2)
  mono:       returned 3
  VIR:        returned 3
  round trip: returned 3
```

The baseline is the whole check: equal lines are agreement, and a divergence
between the engines appears as a diff of the line that changed. Agreement
records nothing extra, but a divergence adds an explicit line naming it:

```text
  mono:       failed InvalidOperation (Add: under/overflow)
  VIR:        aborted 300
  round trip: aborted 300
  ERROR: mono and VIR diverge
```

Exhaustion never produces one — an `inconclusive` line means an engine ran out
of its own budget, which proves nothing about the others. A recorded `ERROR:`
line is an open finding, tracked in the findings register of
`doc/designs/monovm-link-design.md`, not an accepted expectation.

Only normalized content is recorded, so the file is deterministic: gas usage,
collection counts, abort locations, and the adapter's build identity are
triage output, never expectations. Integers compare numerically and addresses
by identity rather than spelling, so a diff always means a semantic change.
`UB=1` updates these like any other baseline — review the diff.

Reusable discovery, diffing, and update support lives in
`VerexIR.TestInfra.Baseline`. Assertion-style frontend, printer, VIR-boundary,
intrinsic, round-trip, and behavior tests live with their implementations under
`transpiler/Transpiler/Tests` or `verex-rust/VerexRust/Tests`; they are
deliberately not E2E tests.
