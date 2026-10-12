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
| [`designs/`](designs/) | The current cross-package design documents: unified VIR, VIR elaboration/verification, the VerexSL surface, the Rust MIR frontend, and the MonoVM differential-testing link. |
| [`verex-ir/`](verex-ir/) | Shared typed VIR, versioned RawUnit JSON import, validation, structured semantics, interpreter, proofs, and unit tests. It owns the neutral IR boundary. |
| [`verex-move/`](verex-move/) | Move semantic profile and intrinsic registry over `verex-ir`, the Move exchange frontend (`VerexMove/Frontend`): CLI invocation, XAST decoding, and VIR encoding, and the XIR backend (`VerexMove/Xir`) compiler-v2 compiles `.lean` sources through. |
| [`verex-rust/`](verex-rust/) | Rust semantic profile, source backend, registry, Lean-owned import CLI, and Lean integration tests. |
| [`verex-rust/rust-exporter/`](verex-rust/rust-exporter/) | Standalone Rust crate using the pinned Rustc Public API to export borrow-checked optimized MIR as RawUnit JSON. Fixtures and JSON baselines live under `tests/`. |
| [`v0/move-model/`](v0/move-model/) | **Deprecated, reference only.** Logical model of Move stackless bytecode: IR, execution semantics, checking, interpreter, reference elimination, prover stages, XIR, and source/masm frontends. |
| [`v0/move/`](v0/move/) | **Deprecated, reference only.** The original Verex Move surface language, source semantics, contracts, `verify`, compiler lowering, and `Move/Tests` regressions, being rebuilt over the unified VIR. |
| [`v0/transpiler/`](v0/transpiler/) | **Deprecated, reference only.** `aptos move exchange --format ast` decoder, Move-to-Verex printer, intrinsic handling, reporting, CLI, and printer/elaboration baselines. Its exchange frontend and VIR adapter were ported to `verex-move`. |
| [`verex-e2e-tests/`](verex-e2e-tests/) | Discoverable Move-to-VerexSL and Rust-to-VerexSL source/result baselines. Assertion-style legacy tests remain with their owning packages. |
| [`scripts/`](scripts/) | The Rust pipeline benchmark (`bench-rust-pipeline.sh`), single-target isolation for cost and debugging (`isolate-target.py`), and the baseline error delta after a `UB=1` run (`exp-error-delta.sh`); see `designs/perf-notes.md`. The verification benchmark driver (`verex-bench.py`); see `designs/verification-benchmarks.md`. |
| [`bench/`](bench/) | The problems of the verification benchmark (`problems.toml`), read from the tree at the benchmarked commit, and a Move package of problems written for it (`package/`). |
| [`v0/`](v0/) | **Deprecated, reference only.** The original stack: `move`, `move-model`, `transpiler`, and their `scripts/` proof-cost tools. Nothing links them and they are not built in CI. |

Within each package, the root `Foo.lean` is the public import, source modules
live under `Foo/`, and tests are either under `Foo/Tests/` or exposed through a
test library listed in that package's `lakefile.toml`. `.lake/` contains local
build output and is not source.

## Design and architecture documents

Read the document for the layer being changed; many contain explicit status,
non-goals, deferred-work registers, and testing requirements.

### Current designs (`designs/`)

The current cross-package designs live in [`designs/`](designs/). The design
documents kept inside `v0/` (`move/`, `move-model/`, `transpiler/`) describe the
older source-specific work those packages own.

- [`designs/roadmap.md`](designs/roadmap.md): the single status document —
  what is open across the designs in priority order, and the test ledger
  (gates, the Check fixtures under `verex-e2e-tests/VerexE2ETests/Check/`,
  and their conventions).
- [`designs/vir-design.md`](designs/vir-design.md): the master design — unified
  VIR ownership, stages, profiles, validation, serialization, frontend/backend
  contracts, deferred-work register, and phase roadmap.
- [`designs/elaboration-design.md`](designs/elaboration-design.md): VIR
  elaboration, verification, execution, runtime semantics, and migration plan.
- [`designs/denotation.md`](designs/denotation.md): the current
  verification design — one `denote` of validated VIR into `Spec` and one
  agreement theorem against `BigStep`, replacing per-target generated
  agreement proofs; milestones D0–D6 with their fixture gates.
- [`designs/perf-notes.md`](designs/perf-notes.md): how to measure
  performance (stage logs, heartbeat stages, profiler categories, kernel
  unfold diagnostics), what verification, elaboration, and the kernel pay
  for, rejected experiments, and the open leads.
- [`designs/verification-benchmarks.md`](designs/verification-benchmarks.md):
  the nightly benchmark of standard problems, heartbeats per phase, history
  from recent CI runs, curves, Slack reports, and local comparison.
- [`designs/verex-sl.md`](designs/verex-sl.md): the profile-aware Verex
  source language over the shared IR, generalizing the Move-profile surface.
- [`designs/prophetic-references.md`](designs/prophetic-references.md):
  prophecy-based ownership model for references in validated VIR — the
  RustHorn encoding run by the interpreter, big-step relation, and
  verifier alike, with loan elimination in validation and milestones.
- [`designs/rust-mir-design.md`](designs/rust-mir-design.md): Rustc Public
  frontend decision, Rust profile, structured VIR boundary, Rust source
  backend, references, generics, unsafe boundary, and milestones.
- [`designs/relocatable-namespaces.md`](designs/relocatable-namespaces.md):
  validated namespaces as relocatable objects, linked into units by
  relocation without re-validation, and verification across modules;
  milestones R1–R4.
- [`designs/source-verification.md`](designs/source-verification.md): the
  `verex-move verify` / `verex-rust verify` commands, Rust `.spec.lean`
  files, Move frames, and how messages map back to the original sources.
- [`designs/lemmas.md`](designs/lemmas.md): Move `spec lemma`s and `proof`
  blocks translated: VIR lemmas and proof steps (`apply`, `split`), lemma
  theorems by well-founded recursion, and `apply` sites in the solver.
- [`designs/prover-test-problems.md`](designs/prover-test-problems.md): the
  registry of problems the Move Prover's unit tests show under Verex
  (`MVP_TEST_FEATURE=lean`), each with its message and tests.
- [`designs/static-memory.md`](designs/static-memory.md): the
  denotation's global memory as one typed memory per resource type,
  related to runtime globals once in the agreement; milestones S1–S5.
- [`designs/static-typing.md`](designs/static-typing.md): semantic types,
  a proved-sound static type checker run at preparation, and the phases
  toward preservation of runtime typing for the big-step semantics.
- [`designs/monovm-link-design.md`](designs/monovm-link-design.md): linking
  MonoVM into Lean test executables for differential execution testing.
- [`designs/higher-order-functions.md`](designs/higher-order-functions.md):
  function values: closure masks and instantiations in VIR, closures as
  runtime values in the denotation, behavioral predicates defined from a
  closure's meaning, state labels, and milestones H1–H5.
- [`designs/unsafe-pointers.md`](designs/unsafe-pointers.md): proposal for
  the Rust unsafe profile over the prophetic model, scheduled as Rust M5.
- [`designs/state-labels.md`](designs/state-labels.md): state labels of
  Move specifications (`..S |~ …`, `exists S in *`) as memory variables,
  defined by state-change predicates or quantified; exchange, VIR,
  VerexSL, contracts, solver witnesses; milestones S1–S4.
- [`verex-rust/rust-exporter/README.md`](verex-rust/rust-exporter/README.md):
  exact exporter scope, supported MIR fixtures, commands, and known boundaries.

Executed or superseded designs move to
[`designs/historical/`](designs/historical/) (`verification-v2.md`,
`certifying-execution.md`, `generic-route.md`, and
`verification-perf-audit.md`, all replaced by `denotation.md`;
`test-organization.md` and `test-organization-history.md`, the earlier
check ledger, replaced by the roadmap's test ledger); they are reference
only and are not updated.

### Verex Move source and verification (deprecated packages)

- [`v0/move/Move/README.md`](v0/move/Move/README.md): example-driven architecture
  and module tour.
- [`v0/move/Move/verex-move.md`](v0/move/Move/verex-move.md): language reference,
  grammar, types, expressions, functions, specifications, verification, and
  compilation.
- [`v0/move/Move/verification-design.md`](v0/move/Move/verification-design.md):
  relational source semantics, effects, references, contracts, verification
  interface, tests, and proof obligations.
- [`v0/move/Move/design-plan.md`](v0/move/Move/design-plan.md): lowering from Verex
  Move through VIR and MoveModel IR to XIR and bytecode.
- [`v0/move/Move/project-plan.md`](v0/move/Move/project-plan.md): implemented source
  features, unsupported language coverage, diagnostics, and roadmap.
- [`v0/move/Move/loop-design.md`](v0/move/Move/loop-design.md): loop syntax, lowering,
  fixed-point verification, and diagnostics.
- [`v0/move/Move/invariant-design.md`](v0/move/Move/invariant-design.md): data and
  global invariants and where proof obligations arise.
- [`v0/move/Move/address-design.md`](v0/move/Move/address-design.md): address surface
  model and compiler representation.
- [`v0/move/Move/unified-int-design.md`](v0/move/Move/unified-int-design.md) and
  [`v0/move/Move/int-widening-design.md`](v0/move/Move/int-widening-design.md): integer
  representation, operations, and specification coercion policy.
- [`v0/move/Move/structural-equality-design.md`](v0/move/Move/structural-equality-design.md):
  runtime structural equality and the remaining verification-model gap.
- [`v0/move/Move/performance-analysis.md`](v0/move/Move/performance-analysis.md) and
  [`v0/move/Move/proof-simplification-plan.md`](v0/move/Move/proof-simplification-plan.md):
  proof cost, simplifier behavior, benchmarks, and proof cleanup.
- [`v0/move/Move/overview.md`](v0/move/Move/overview.md): short conceptual overview.

### Move model and transpilation (deprecated packages)

- [`v0/move-model/MoveModel/README.md`](v0/move-model/MoveModel/README.md): model
  overview and links to the detailed
  [`IR`](v0/move-model/MoveModel/IR/README.md),
  [`Frontend`](v0/move-model/MoveModel/Frontend/README.md), and
  [`Prover`](v0/move-model/MoveModel/Prover/README.md) guides.
- [`v0/transpiler/transpile-design.md`](v0/transpiler/transpile-design.md): XAST
  exchange format, export stage, decoding/printing, semantic mapping,
  validation, baselines, and feature scoreboard.
- [`v0/transpiler/intrinsic-design.md`](v0/transpiler/intrinsic-design.md): generic
  intrinsic-map representation, lowering, diagnostics, and acceptance tests.
- [`verex-e2e-tests/README.md`](verex-e2e-tests/README.md): source/result
  baseline conventions for the two cross-language paths.

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
`lake env`; see `designs/source-verification.md` ("From the Move CLI").
A function the automatic verification does not establish within its budget
is proved in the VerexSL proof file beside its Move file
(`foo.proof.lean`: `verify f by …` items and their lemmas), which the
failure message names; see the design's "Proofs". The commands, flags, and
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
