---
name: verex-smoke
description: Use between turns when iterating on the prover of the Verex Lean stack (third_party/move/lean: kernel laws, solver, specification layer): prover-smoke after each edit batch, and the checkpoint gate every commit needs
---

# prover-smoke and the checkpoint gate

`third_party/move/lean/scripts/prover-smoke.sh` is the inner loop for changes
to the prover: small VerexSL fixtures elaborated with the built Lean libraries
alone. It runs no frontend (no `verex-move` or `verex-rust` executable, no Move
CLI, no cargo, no e2e driver) and is not evidence for a frontend change. It
does not replace the checkpoint gate before a commit.

## prover-smoke

```bash
third_party/move/lean/scripts/prover-smoke.sh            # build the libraries in place, run the fixtures
third_party/move/lean/scripts/prover-smoke.sh --jobs N   # Lean processes at once (or PROVER_SMOKE_JOBS)
third_party/move/lean/scripts/prover-smoke.sh --costs    # also print every pinned cost as measured
third_party/move/lean/scripts/prover-smoke.sh --list     # the fixtures, and the deferred ones
third_party/move/lean/scripts/prover-smoke.sh --self-test # the script's own checks: cost reading, process verdicts, cancellation
```

It builds `verex-ir` (the libraries and the test modules its fixtures import)
and the one module `VerexE2ETests.CheckSupport`, then elaborates each fixture
as its own `lake env lean` process (by default half the hardware threads, at
most one per 3 GB of available memory), longest first by the previous run, and
prints one line per fixture with pass/fail, wall and CPU seconds. The tiers
(`scripts/prover-smoke-coverage.md` says what each fixture stands for and what
is not covered):

- the shape matrix `verex-ir/VerexSL/Tests/Shapes/` (mechanism × type shape; a
  positive and a pinned negative per cell; open cells under `expectOpen`,
  listed in its `README.md`);
- control modules of `verex-ir` (the join scheduler, preparation, maps, state
  labels) and boundary controls (late evidence, frames, write chains, imported
  declarations, caches, residuals and the callee's row they belong to
  (`CallArgumentResidual`, `BehaviorDispatchResidual`), a missing or wrong
  dictionary contract (`ForwardedMethodEvidence`), own-invariant bridges);
- rendered source fixtures `verex-ir/VerexSL/Tests/Rendered/` (checked-in
  VerexSL a frontend wrote, with the fixture's diagnostics);
- check fixtures of `verex-e2e-tests/VerexE2ETests/Check/`;
- a performance tier: `bench/lean/OrderedMap.lean`, `Check/Closures/KnownSiteCost`,
  `Check/Examples/Quicksort`, `Tests/Cost/MapScaling`, with heartbeat pins and
  ratios in `scripts/prover-smoke-cost.pins` (±5%).

Rules it encodes:

- Read-only: a diff from an expectation is a failure to report, never to
  absorb; it refuses `UB`, `UPBL` and `UPDATE_BASELINE` and never writes a pin.
- A fixture's process must return what its expectation gives: 0 for one that
  prints nothing, 1 for a check fixture whose `.exp` holds error diagnostics.
  A signal, any other status or a missing record is a failure, as is a
  `PANIC` in a fixture's output or in the build's output.
- An interrupted run stops its own jobs (each a process group) and nothing
  else.
- A pinned target that is not verified fails; a cost more than 5% under its
  pin fails too (check that the target proves what it proved, then lower the
  pin by hand). A pin is not a check of what a target proves.
- A fixture whose `.exp` awaits the baseline refresh is not run and is printed
  as deferred with every run.
- After a failure the raw output is in `verex-ir/.lake/prover-smoke-failed/`.

What it does not check, and the gate does: `verex-move verify` of a source, the
Move Prover's lean tests, the Rust renderings and the `verex-rust` suite, and that the rendered fixtures are
still what the frontends produce (`scripts/prover-smoke-renderings.sh`, which
needs the driver, the Move CLI and the Rust exporter; `--write` regenerates).
The package over compiled namespaces (`VerexE2ETests/CompiledNamespaces`) is in
no suite either: the gate runs it as `scripts/verify-compiled-namespaces.sh`.

Between runs, build libraries, not executables: `lake build` in `verex-ir`,
`lake build VerexMove` in `verex-move`, `lake build VerexE2ETests` in
`verex-e2e-tests`. To verify a Move package or some of its modules while
iterating, `third_party/move/lean/scripts/verify-modules.py <package> [--filter
<part>]` runs one `verex-move verify` process per module, several at a time.

## Checkpoint gate (before every commit)

1. Full build and the four suites: `(cd verex-ir && lake build && lake test)`,
   and the same in `verex-move`, `verex-rust`, `verex-e2e-tests`.
2. `scripts/prover-smoke-renderings.sh` (no argument): every rendered fixture
   is what the frontends render now.
3. The package over compiled namespaces, which no suite covers:
   `third_party/move/lean/scripts/verify-compiled-namespaces.sh` (about 20 s
   with `verex-move` built; it needs the Move CLI of the `APTOS_MOVE_CLI`
   contract, compares the report of `fixture::client` with
   `VerexE2ETests/CompiledNamespaces/client.exp` and never writes it;
   `--self-test` checks its own verdict). `.github/workflows/lean.yaml` runs it
   beside the four suites once that workflow step lands with the script.
4. The Move Prover gate alone, which builds and snapshots `verex-move` itself:
   `cd third_party/move/move-prover && MVP_TEST_FEATURE=lean cargo test
   -p move-prover --test testsuite` (read-only; `UPBL=1` only for an intended
   baseline refresh, then review `git diff --numstat
   third_party/move/move-prover/tests` and attribute each changed `.lean_exp`
   with `git log -1 -- <file>`). A run that reports 440 passed in a fraction of
   a second skipped every test. Its rules:
   - The Move Prover tests run a private copy of the verifier, taken when the
     run starts. The build and copy must observe unchanged source and artifact
     inventories; a concurrent modification discards the copy and retries. Only
     the verifier's import closure is copied, so tests can run beside later Lake
     builds and the Lean suites. A scratch replay (`lake env lean`) and the
     framework command-line run read the live `.lake` trees instead: run those
     beside test runs, never beside a build that rewrites `verex-ir` or
     `verex-move`.
   - A Move Prover test the harness skips ("verifier is not built") is a
     failure, not a pass.
5. For intrinsic-map work, the framework CLI compare against the previous
   stage log: `VEREX_OPTIONS=verex.stageLog=<file> target/ci/move prove --lean
   --package-dir aptos-move/framework/aptos-framework -f ordered_map`, per-module
   heartbeats compared.

## Where the build time goes

An edit rebuilds the edited module, every module importing it, and the C of
the edited module only. The solver, `Contract` and `Verify` are split into
parts so that most edits rebuild one part and its importers: an edit of the
solver's walker costs about 30 s to libraries and 15 s more to the
executable, against 90 s and 70 s before the split. Measurements and the
parts: `third_party/move/lean/doc/designs/performance.md`, "Modules split for
edits". prover-smoke builds no CLI; the gate's steps that need one build it
with cargo (8 min on 2026-10-22 after a restack).
