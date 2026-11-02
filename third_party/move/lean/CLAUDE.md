# CLAUDE.md

This file gives repository-local guidance for working in
`third_party/move/lean`. Also follow the Aptos Core guidance in
[`../../../CLAUDE.md`](../../../CLAUDE.md) and the applicable `AGENTS.md`.

## Project overview

This directory contains experimental Lean 4 implementations of a shared
language-neutral IR, semantic profiles for Move and Rust, a Lean-authored Move
language and verifier, a logical Move bytecode model, and source frontends and
end-to-end tests around them.

The current codebase is the verex stack: `verex-ir`, `verex-move`,
`verex-rust` (with `rust-exporter`), and `verex-e2e-tests`. The `move`,
`move-model`, and `transpiler` packages under `v0/` are **deprecated**: they exist only as
reference for solutions being rebuilt on the verex codebase. Do not add
functionality or tests to them, do not include their suites in test runs or
CI, and do not root new work in them. The Move exchange frontend and VIR
adapter formerly hosted by the transpiler now live in
`verex-move/VerexMove/Frontend`. Nothing links the deprecated packages:
compiler-v2 elaborates `.lean` sources in `verex-move`, whose XIR backend
(`VerexMove/Xir`) lowers a VerexSL module to the XIR compiler-v2 loads.

Keep these claims separate:

1. `verify f` proves a theorem about the generated source semantics of a
   Lean-authored function.
2. The compiler lowers supported declarations to XIR and production Move
   bytecode.
3. A compiler-correctness theorem connecting the source theorem to emitted
   bytecode is future work. Do not describe source verification as verification
   of the emitted bytecode.

There is no root Lake workspace. Every top-level component is an independent
Lake package with its own `lakefile.toml`, manifest, and pinned
`lean-toolchain`. Run `lake` commands from the package directory, or in a
subshell as shown below.

## Dependency shape

The principal one-way dependencies are:

```text
verex-ir <- verex-move <- verex-e2e-tests
          <- verex-rust <- rust-exporter
             verex-rust <- verex-e2e-tests

move-model <- move <- transpiler        (deprecated, reference only)
```

`verex-e2e-tests` is deliberately test-only and may depend on every layer it
exercises. Avoid introducing reverse dependencies into production packages.

## Tree map

| Path | Contents |
|---|---|
| [`README.md`](README.md) | Tree-level overview, architecture, setup, and the main build/test path. Start here. |
| [`doc/`](doc/) | Designs, the plan, status, historical designs, and talks; see "Documentation". |
| [`verex-ir/`](verex-ir/) | Shared typed VIR, versioned RawUnit JSON import, validation, structured semantics, interpreter, proofs, and unit tests. It owns the neutral IR boundary. |
| [`verex-move/`](verex-move/) | Move semantic profile and intrinsic registry over `verex-ir`, the Move exchange frontend (`VerexMove/Frontend`): CLI invocation, XAST decoding, and VIR encoding, and the XIR backend (`VerexMove/Xir`) compiler-v2 compiles `.lean` sources through. |
| [`verex-rust/`](verex-rust/) | Rust semantic profile, source backend, registry, Lean-owned import CLI, and Lean integration tests. |
| [`verex-rust/rust-exporter/`](verex-rust/rust-exporter/) | Standalone Rust crate using the pinned Rustc Public API to export borrow-checked optimized MIR as RawUnit JSON. Fixtures and JSON baselines live under `tests/`. |
| [`v0/move-model/`](v0/move-model/) | **Deprecated, reference only.** Logical model of Move stackless bytecode: IR, execution semantics, checking, interpreter, reference elimination, prover stages, XIR, and source/masm frontends. |
| [`v0/move/`](v0/move/) | **Deprecated, reference only.** The original Verex Move surface language, source semantics, contracts, `verify`, compiler lowering, and `Move/Tests` regressions, being rebuilt over the unified VIR. |
| [`v0/transpiler/`](v0/transpiler/) | **Deprecated, reference only.** `aptos move exchange --format ast` decoder, Move-to-Verex printer, intrinsic handling, reporting, CLI, and printer/elaboration baselines. Its exchange frontend and VIR adapter were ported to `verex-move`. |
| [`verex-e2e-tests/`](verex-e2e-tests/) | Discoverable Move-to-VerexSL and Rust-to-VerexSL source/result baselines. Assertion-style legacy tests remain with their owning packages. |
| [`scripts/`](scripts/) | The core smoke suite (`smoke.sh`, see "Between turns"), package verification with one process per module (`verify-modules.py`), the Rust pipeline benchmark (`bench-rust-pipeline.sh`), single-target isolation for cost and debugging (`isolate-target.py`), and the baseline error delta after a `UB=1` run (`exp-error-delta.sh`); see `doc/designs/performance.md`. The verification benchmark driver (`verex-bench.py`); see `doc/status/verification-benchmarks.md`. |
| [`bench/`](bench/) | The problems of the verification benchmark (`problems.toml`), read from the tree at the benchmarked commit, and a Move package of problems written for it (`package/`). |
| [`v0/`](v0/) | **Deprecated, reference only.** The original stack: `move`, `move-model`, `transpiler`, and their `scripts/` proof-cost tools. Nothing links them and they are not built in CI. |

Within each package, the root `Foo.lean` is the public import, source modules
live under `Foo/`, and tests are either under `Foo/Tests/` or exposed through a
test library listed in that package's `lakefile.toml`. `.lake/` contains local
build output and is not source.

## Documentation (`doc/`)

Read the design for the layer being changed. Designs hold the model and
the decisions, not status; what is open is in the plan, and state that the
source does not show is in the status directory.

| Directory | Contents |
|---|---|
| [`doc/user/`](doc/user/) | The Verex Book: concise user documentation for VerexSL and the proof language, for expert users; not a replacement for the designs. |
| [`doc/designs/`](doc/designs/) | The current designs (below). |
| [`doc/plan/`](doc/plan/) | [`roadmap.md`](doc/plan/roadmap.md): what is open, by track, in priority order. Approved but unscheduled plans beside it ([`native-function-values.md`](doc/plan/native-function-values.md), [`unsafe-pointers.md`](doc/plan/unsafe-pointers.md)). |
| [`doc/status/`](doc/status/) | [`prover-test-problems.md`](doc/status/prover-test-problems.md): what the Move Prover's tests show under Verex and why; [`assumptions.md`](doc/status/assumptions.md): the hypotheses of the public theorems nothing discharges; [`measurement-debts.md`](doc/status/measurement-debts.md); [`verification-benchmarks.md`](doc/status/verification-benchmarks.md): the nightly benchmark and its history. |
| [`doc/historical/`](doc/historical/) | Executed or superseded designs, kept for their rationale, not updated. |
| [`doc/talk/`](doc/talk/) | Talk sources and PDFs. |

Designs:

- [`vir-design.md`](doc/designs/vir-design.md): the unified VIR: ownership, stages, profiles, validation, serialization, frontend and backend contracts.
- [`elaboration-design.md`](doc/designs/elaboration-design.md): runtime model, big-step semantics, interpreter, correctness obligations.
- [`denotation.md`](doc/designs/denotation.md): one `denote` of validated VIR into `Spec` and one agreement theorem against `BigStep`.
- [`native-verification.md`](doc/designs/native-verification.md): verification over native values only: constructs, enforcement, the solver's normal form and splits, specialized contracts.
- [`proof-strategy.md`](doc/designs/proof-strategy.md): bounded automation, authored proofs, no SMT; the authored interface.
- [`performance.md`](doc/designs/performance.md): how to measure, build cost, what was learned, open levers, rejected experiments.
- [`source-verification.md`](doc/designs/source-verification.md): `verex-move verify` / `verex-rust verify` / `move prove --lean`, proof files, pragmas, message mapping.
- [`verex-sl.md`](doc/designs/verex-sl.md): the profile-aware Verex source language.
- [`prophetic-references.md`](doc/designs/prophetic-references.md): the prophecy ownership model of references.
- [`static-memory.md`](doc/designs/static-memory.md): one typed memory per resource type.
- [`intrinsic-maps.md`](doc/designs/intrinsic-maps.md): the abstract map value of intrinsic map types, roles, validity, typed map lemmas.
- [`ghost-fields.md`](doc/designs/ghost-fields.md): ghost fields and validity slots.
- [`state-labels.md`](doc/designs/state-labels.md): state labels as memory variables.
- [`higher-order-functions.md`](doc/designs/higher-order-functions.md): closures in VIR, runtime, denotation, behavioral predicates, the typed carrier of function types.
- [`lemmas.md`](doc/designs/lemmas.md): `spec lemma`s and `proof` blocks.
- [`numeric-representation.md`](doc/designs/numeric-representation.md): integers as mathematical integers, not the Prover's bit vectors.
- [`relocatable-namespaces.md`](doc/designs/relocatable-namespaces.md): namespaces linked by relocation; verification across modules.
- [`rust-mir-design.md`](doc/designs/rust-mir-design.md): the Rustc Public frontend, Rust profile, source backend, milestones.
- [`monovm-link-design.md`](doc/designs/monovm-link-design.md): MonoVM linked for differential execution.
- [`verex-rust/rust-exporter/README.md`](verex-rust/rust-exporter/README.md): exporter scope and commands.

The deprecated packages under `v0/` keep their own design documents
(`v0/move/Move/*.md`, `v0/move-model/MoveModel/README.md`,
`v0/transpiler/*.md`); they describe the old stack and are reference only.
The e2e baseline conventions are in
[`verex-e2e-tests/README.md`](verex-e2e-tests/README.md).

## Toolchains and setup

All Lake packages pin Lean `v4.32.2`. Use the checked-in toolchain files; do
not silently change the Lean version in only one package. The Rust exporter
separately pins `nightly-2026-07-23` with `rustc-dev`, `rust-src`, and
`llvm-tools-preview` in
[`rust-toolchain.toml`](verex-rust/rust-exporter/rust-toolchain.toml).

From the Aptos Core repository root, install the Lean prerequisites with:

```bash
./scripts/dev_setup.sh -p -l
source "$HOME/.profile"
```

Core Lean libraries build without an Aptos CLI. The E2E test driver owns the
freshness of its exchange frontend: once per suite invocation, inside a
checkout, it runs the locked ci-profile Cargo build of the standalone `move`
CLI (`cargo build --locked --profile ci -p aptos-move-cli --features binary
--bin move`), which Cargo's own freshness check makes cheap when nothing
changed. With no variables set, the driver uses the binary it just built
(`<workspace>/target/ci/move`). **The contract for `APTOS_MOVE_CLI` is that
it names a Cargo ci-profile build of the standalone CLI** — normally exactly
that managed binary, which the driver's build keeps fresh; pointing anywhere
else draws a warning because this checkout's build cannot freshen it.
`APTOS_CLI=<path-to-full-aptos-cli>` selects the full CLI as an unmanaged
escape hatch whose freshness its setter owns. The standalone command receives
`exchange` directly; the full CLI receives `aptos move exchange`. An
unrelated `move` executable on `PATH` is never a substitute.

```bash
# Optional: the same build the driver runs, for use outside the E2E suite.
cargo build --locked --profile ci -p aptos-move-cli --features binary --bin move
export APTOS_MOVE_CLI="$(git rev-parse --show-toplevel)/target/ci/move"
```

The reverse direction verifies a Move package in place from the Move CLI:
`move prove --lean --package-dir <package>` (or `aptos move prove --lean`)
exports the package and runs the built `verex-move` executable through
`lake env`; see `doc/designs/source-verification.md` ("From the Move CLI").
A function the automatic verification does not establish within its budget
is proved in the proof file beside its Move file (`foo.proof.lean`: its
header, then `verify f by …` items and their lemmas), which the failure
message names; see the design's "Proofs". A package's rendering goes to
`build/verex-verify.lean`, a single Move file's to `build/verex/` of its
package or, outside packages, to `.verex/` beside it, where the proof file's
header finds it in the editor. The commands, flags, and
pragmas are summarized in [`README.md`](README.md) ("Verifying sources").

## Build and test

Build or test the package you changed from its own directory. These four
suites are the test matrix; the deprecated packages' suites are not part of
test runs or CI:

```bash
(cd verex-ir         && lake build && lake test)
(cd verex-move       && lake build && lake test)
(cd verex-rust       && lake build && lake test)
(cd verex-e2e-tests && lake build && lake test)
```

`verex-ir`'s suite includes a verification-cost benchmark
(`VerexSL/Tests/DenotePerformance.lean`) that gates every `verify`
target against `DenotePerformance.exp` on elaborator heartbeats and
proof-term size. A deliberate cost change is recorded with
`lake build && UB=1 lake env lean VerexSL/Tests/DenotePerformance.lean`;
review the diff, which should name only the targets the change was meant
to move.
The build belongs to the command: `lake env lean` only sets the module path,
so regenerating without it records the cost of whatever imports happen to be
built and commits a baseline that its own source does not reproduce.

The package `testDriver` is declared in its `lakefile.toml`; `lake test` is
the normal full package suite. The E2E driver manages its exchange frontend
itself, as described above. Run downstream suites when changing a shared
boundary:

| Changed area | Minimum relevant verification |
|---|---|
| `verex-ir` schema, validation, or semantics | `verex-ir`, `verex-move`, `verex-rust`, and `verex-e2e-tests` tests |
| Rust profile/import/source backend | `verex-rust` tests plus Rust exporter tests when its code or fixtures are involved |
| Move profile or the exchange frontend/VIR adapter | `verex-move` and `verex-e2e-tests` tests |

For a fast check of one Lean file, run it from its package directory so imports
and relative test assets resolve correctly:

```bash
cd verex-ir
lake env lean VerexIR/Tests/Validation.lean
```

### Between turns: the smoke suite

The four suites and the Move Prover's lean gate are the checkpoint gate
before a commit. Between edit batches, run the smoke
suite described in [`.cursor/skills/verex-smoke/SKILL.md`](../../../.cursor/skills/verex-smoke/SKILL.md):
`scripts/smoke.sh` builds in place and runs the fixtures that catch this
stack's regressions, read-only against their `.exp`, with one pass/fail line
each; it refuses `UB=1`. Green smoke is not a commit gate.

While iterating, build libraries, not executables: `lake build` in
`verex-ir`, `lake build VerexMove` in `verex-move`, and `lake build
VerexE2ETests` in `verex-e2e-tests`. The default targets of the last two
also build the `verex-move` executable and the e2e driver, whose C
compilation and linking the Lean fixtures do not need (10–30 s after an edit
of one part of the solver; see `doc/designs/performance.md`, "Modules split for
edits"). The smoke suite builds the executable beside its Lean fixtures,
and `--no-exe` skips it with the two items that need it. To verify a Move
package while iterating, `scripts/verify-modules.py <package> [--filter
<part>]` runs one `verex-move verify` process per module, several at a time.

Profile code generation as well as proofs when investigating build time.
Splitting large metaprograms in `VerexIR/Proofs/Denote/Solve.lean` and
`VerexSL/Verify.lean` into smaller functions reduces compiler work; see
[`doc/designs/performance.md`](doc/designs/performance.md) for the measurements.

The solver, the contract generator and `verify` are split into modules for
build time; the last part of each keeps the original module name, so
importers are unchanged:

| Module | Parts, in import order |
|---|---|
| `VerexIR.Proofs.Denote.Solve` | `Solve.Base` (options, accounting, leaf facts, caches); `Solve.Decide` (leaf deciders) and `Solve.Normal` (the normal form's simprocs and lemmas), which do not import each other; `Solve.Invoke` (splits, behaviors, invocations); `Solve` (walker, authored tactics) |
| `VerexSL.Contract` | `Contract.Values` (native types, values, maps, owners); `Contract.Clauses` (the clause translator); `Contract` (invariants, contract assembly) |
| `VerexSL.Verify` | `Verify.Contracts` (compiled functions, contracts, loop invariants; no solver import); `Verify.Members` (targets, cycles, lemmas); `Verify` (commands) |

Put a new declaration in the part of its topic; one that another part uses
cannot be `private`. Keep these import boundaries: `Solve.Normal` and
`Solve.Decide` independent of each other, and `Verify.Contracts` free of the
solver, so that an edit of one rebuilds neither the other nor that part.

The Move Prover's lean gate (`MVP_TEST_FEATURE=lean cargo test -p move-prover
--test testsuite`) builds `verex-move` itself and runs a private copy of it
under `target/tmp`, so a concurrent Lake build cannot change what a test
loads, and a harness failure of the verifier is never written as a
baseline; see `doc/designs/source-verification.md` ("From the Move Prover").

Do not add `sorry`, axioms, or other admissions to make a proof compile unless
the design explicitly models an assumption and the change is requested.

### Rust exporter

Run the project-owned exporter checks from its standalone crate:

```bash
cd verex-rust/rust-exporter
cargo test --features rustc-public
cargo fmt --check
cargo clippy --features rustc-public --all-targets
```

Build and invoke it directly with the matching sysroot library path:

```bash
cargo build --features rustc-public
LD_LIBRARY_PATH="$(rustc --print sysroot)/lib" \
  target/debug/verex-rust-export --output /tmp/basic.raw.json -- \
  tests/raw-unit/basic.rs --crate-type=lib --edition=2024
```

Normally prefer the Lean-owned driver because it builds the pinned exporter,
tracks cache inputs, and validates RawUnit before writing it:

```bash
cd verex-rust
lake exe verex-rust -- import-file rust-exporter/tests/raw-unit/basic.rs \
  --output .lake/verex-rust/basic.raw.json

lake exe verex-rust -- import-crate Tests/CargoFixture/Cargo.toml \
  --package verex-rust-cargo-fixture \
  --features extra --no-default-features \
  --output .lake/verex-rust/cargo-fixture.raw.json
```

`VEREX_RUST_EXPORTER`, `VEREX_RUST_SYSROOT`, and `VEREX_RUST_CACHE` override
the development exporter, paired sysroot, and managed cache respectively.

## Developer tools and generated fixtures

E2E baselines are regenerated with `UB=1 lake test` (or a `UB=1` driver run)
from `verex-e2e-tests/`; review every diff before committing. The deprecated
transpiler CLI and its printer baselines remain usable for reference but are
not maintained.

Positive Rust exporter inputs and their side-by-side `.exp.json` baselines are
in `verex-rust/rust-exporter/tests/raw-unit/`; rejection and API-probe inputs
remain under `tests/fixtures/`. The baseline driver discovers every `.rs` in
`raw-unit` automatically. Use `UB=1 cargo test --features rustc-public` to
update expectations, inspect the semantic diff, and commit an update only when
the format or mapped MIR is intentionally changing. The Rust and Lean suites
both validate these artifacts.

The `v0/scripts/` proof-analysis tools and the Account XIR regeneration entry
point target the deprecated `move` package and are kept for reference only.

## License headers

Every source file in this tree, Lean and Rust alike, carries the Apache
header (`Copyright © Aptos Foundation` / `SPDX-License-Identifier:
Apache-2.0`), not the Innovation-Enabling header used elsewhere in the
repository. The Rust fixtures' headers are exported as comments, so a
header change regenerates the RawUnit and Rust e2e baselines.

## Change discipline

- Keep documentation focused on behavior, rationale, open work and reproducible
  measurements. Put validation results in PRs and benchmark artifacts. Do not
  append session diaries, branch/worktree state, temporary log locations or
  dated test-run snapshots to design documents.
- Lean source metaprograms are trusted. We do not defend against adversarial
  `run_cmd` code tampering with verifier registries or environments. Preserve
  proof correctness and the explicit assumption ledger, but do not expand CI
  fixes into sandboxing or authenticating inputs against hostile metaprograms.
- Read existing tests and the relevant design's status/deferred-work sections
  before changing behavior. Preserve explicit unsupported-feature diagnostics;
  do not silently give Rust constructs Move semantics or erase unsupported MIR.
- Keep RawUnit JSON versioned, deterministic, and canonical. Validation is the
  trust boundary: frontends may produce raw data, but semantic consumers should
  use checked/validated forms.
- Keep production dependencies one-way. Put source/result tests that genuinely
  cross several layers in `verex-e2e-tests`; keep assertion-style unit tests in
  the owning package's `Tests` tree instead of adding a reverse dependency.
- Generated Lean, JSON, XIR, and report files are reviewable test assets. Change
  them only through the owning generator and include them with the source change.
- Ordinary `lake build` must not write source-tree artifacts. Use explicit
  exporter/regeneration commands for generated files.
- Make focused changes and do not modify unrelated dirty worktree files.
