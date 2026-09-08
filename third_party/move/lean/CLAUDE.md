# CLAUDE.md

This file gives repository-local guidance for working in
`third_party/move/lean`. Also follow the Aptos Core guidance in
[`../../../CLAUDE.md`](../../../CLAUDE.md) and the applicable `AGENTS.md`.

## Project overview

This directory contains experimental Lean 4 implementations of a shared
language-neutral IR, semantic profiles for Move and Rust, a Lean-authored Move
language and verifier, a logical Move bytecode model, and source frontends and
end-to-end tests around them.

The current codebase is the leaner stack: `leaner-ir`, `leaner-move`,
`leaner-rust` (with `rust-exporter`), and `leaner-e2e-tests`. The `move`,
`move-model`, and `transpiler` packages are **deprecated**: they exist only as
reference for solutions being rebuilt on the leaner codebase. Do not add
functionality or tests to them, do not include their suites in test runs or
CI, and do not root new work in them. The Move exchange frontend and LIR
adapter formerly hosted by the transpiler now live in
`leaner-move/LeanerMove/Frontend`; nothing current links the deprecated
packages.

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
leaner-ir <- leaner-move <- leaner-e2e-tests
          <- leaner-rust <- rust-exporter
             leaner-rust <- leaner-e2e-tests

move-model <- move <- transpiler        (deprecated, reference only)
```

`leaner-e2e-tests` is deliberately test-only and may depend on every layer it
exercises. Avoid introducing reverse dependencies into production packages.

## Tree map

| Path | Contents |
|---|---|
| [`README.md`](README.md) | Tree-level overview, architecture, setup, and the main build/test path. Start here. |
| [`designs/`](designs/) | The current cross-package design documents: unified LIR, LIR elaboration/verification, the LeanerLang surface, the Rust MIR frontend, and the MonoVM differential-testing link. |
| [`leaner-ir/`](leaner-ir/) | Shared typed LIR, versioned RawUnit JSON import, validation, structured semantics, interpreter, proofs, and unit tests. It owns the neutral IR boundary. |
| [`leaner-move/`](leaner-move/) | Move semantic profile and intrinsic registry over `leaner-ir`, plus the Move exchange frontend (`LeanerMove/Frontend`): CLI invocation, XAST decoding, and LIR encoding. |
| [`leaner-rust/`](leaner-rust/) | Rust semantic profile, source backend, registry, Lean-owned import CLI, and Lean integration tests. |
| [`leaner-rust/rust-exporter/`](leaner-rust/rust-exporter/) | Standalone Rust crate using the pinned Rustc Public API to export borrow-checked optimized MIR as RawUnit JSON. Fixtures and JSON baselines live under `tests/`. |
| [`move-model/`](move-model/) | **Deprecated, reference only.** Logical model of Move stackless bytecode: IR, execution semantics, checking, interpreter, reference elimination, prover stages, XIR, and source/masm frontends. |
| [`move/`](move/) | **Deprecated, reference only.** The original Leaner Move surface language, source semantics, contracts, `verify`, compiler lowering, and `Move/Tests` regressions, being rebuilt over the unified LIR. |
| [`transpiler/`](transpiler/) | **Deprecated, reference only.** `aptos move exchange --format ast` decoder, Move-to-Leaner printer, intrinsic handling, reporting, CLI, and printer/elaboration baselines. Its exchange frontend and LIR adapter were ported to `leaner-move`. |
| [`leaner-e2e-tests/`](leaner-e2e-tests/) | Discoverable Move-to-LeanerLang and Rust-to-LeanerLang source/result baselines. Assertion-style legacy tests remain with their owning packages. |
| [`scripts/`](scripts/) | Proof-cost benchmark and simplifier diagnostics for the `move` package. |

Within each package, the root `Foo.lean` is the public import, source modules
live under `Foo/`, and tests are either under `Foo/Tests/` or exposed through a
test library listed in that package's `lakefile.toml`. `.lake/` contains local
build output and is not source.

## Design and architecture documents

Read the document for the layer being changed; many contain explicit status,
non-goals, deferred-work registers, and testing requirements.

### Current designs (`designs/`)

The current cross-package designs live in [`designs/`](designs/). The design
documents kept inside `move/`, `move-model/`, and `transpiler/` describe the
older source-specific work those packages own.

- [`designs/roadmap.md`](designs/roadmap.md): the priority ordering across the
  current designs and the items deliberately not scheduled.
- [`designs/lir-design.md`](designs/lir-design.md): the master design — unified
  LIR ownership, stages, profiles, validation, serialization, frontend/backend
  contracts, deferred-work register, and phase roadmap.
- [`designs/elaboration-design.md`](designs/elaboration-design.md): LIR
  elaboration, verification, execution, runtime semantics, and migration plan.
- [`designs/denotation.md`](designs/denotation.md): the current
  verification design — one `denote` of validated LIR into `Spec` and one
  agreement theorem against `BigStep`, replacing per-target generated
  agreement proofs; milestones D0–D4 with their fixture gates.
- [`designs/test-organization.md`](designs/test-organization.md): verification
  checks as baselines under `leaner-e2e-tests/LeanerE2ETests/Check/` and
  the ledger of v0 tests ported so far.
- [`designs/leaner-lang.md`](designs/leaner-lang.md): the profile-aware Leaner
  source language over the shared IR, generalizing the Move-profile surface.
- [`designs/prophetic-references.md`](designs/prophetic-references.md):
  prophecy-based ownership model for references in validated LIR — the
  RustHorn encoding run by the interpreter, big-step relation, and
  verifier alike, with loan elimination in validation and milestones.
- [`designs/rust-mir-design.md`](designs/rust-mir-design.md): Rustc Public
  frontend decision, Rust profile, structured LIR boundary, Rust source
  backend, references, generics, unsafe boundary, and milestones.
- [`designs/monovm-link-design.md`](designs/monovm-link-design.md): linking
  MonoVM into Lean test executables for differential execution testing.
- [`designs/unsafe-pointers.md`](designs/unsafe-pointers.md): proposal for
  the Rust unsafe profile over the prophetic model, not yet scheduled.
- [`leaner-rust/rust-exporter/README.md`](leaner-rust/rust-exporter/README.md):
  exact exporter scope, supported MIR fixtures, commands, and known boundaries.

Executed or superseded designs move to
[`designs/historical/`](designs/historical/) (`verification-v2.md`,
`certifying-execution.md`, `generic-route.md`, and
`verification-perf-audit.md`, all replaced by `denotation.md`); they are
reference only and are not updated.

### Leaner Move source and verification (deprecated packages)

- [`move/Move/README.md`](move/Move/README.md): example-driven architecture
  and module tour.
- [`move/Move/leaner-move.md`](move/Move/leaner-move.md): language reference,
  grammar, types, expressions, functions, specifications, verification, and
  compilation.
- [`move/Move/verification-design.md`](move/Move/verification-design.md):
  relational source semantics, effects, references, contracts, verification
  interface, tests, and proof obligations.
- [`move/Move/design-plan.md`](move/Move/design-plan.md): lowering from Leaner
  Move through LIR and MoveModel IR to XIR and bytecode.
- [`move/Move/project-plan.md`](move/Move/project-plan.md): implemented source
  features, unsupported language coverage, diagnostics, and roadmap.
- [`move/Move/loop-design.md`](move/Move/loop-design.md): loop syntax, lowering,
  fixed-point verification, and diagnostics.
- [`move/Move/invariant-design.md`](move/Move/invariant-design.md): data and
  global invariants and where proof obligations arise.
- [`move/Move/address-design.md`](move/Move/address-design.md): address surface
  model and compiler representation.
- [`move/Move/unified-int-design.md`](move/Move/unified-int-design.md) and
  [`move/Move/int-widening-design.md`](move/Move/int-widening-design.md): integer
  representation, operations, and specification coercion policy.
- [`move/Move/structural-equality-design.md`](move/Move/structural-equality-design.md):
  runtime structural equality and the remaining verification-model gap.
- [`move/Move/performance-analysis.md`](move/Move/performance-analysis.md) and
  [`move/Move/proof-simplification-plan.md`](move/Move/proof-simplification-plan.md):
  proof cost, simplifier behavior, benchmarks, and proof cleanup.
- [`move/Move/overview.md`](move/Move/overview.md): short conceptual overview.

### Move model and transpilation (deprecated packages)

- [`move-model/MoveModel/README.md`](move-model/MoveModel/README.md): model
  overview and links to the detailed
  [`IR`](move-model/MoveModel/IR/README.md),
  [`Frontend`](move-model/MoveModel/Frontend/README.md), and
  [`Prover`](move-model/MoveModel/Prover/README.md) guides.
- [`transpiler/transpile-design.md`](transpiler/transpile-design.md): XAST
  exchange format, export stage, decoding/printing, semantic mapping,
  validation, baselines, and feature scoreboard.
- [`transpiler/intrinsic-design.md`](transpiler/intrinsic-design.md): generic
  intrinsic-map representation, lowering, diagnostics, and acceptance tests.
- [`leaner-e2e-tests/README.md`](leaner-e2e-tests/README.md): source/result
  baseline conventions for the two cross-language paths.

## Toolchains and setup

All Lake packages pin Lean `v4.32.2`. Use the checked-in toolchain files; do
not silently change the Lean version in only one package. The Rust exporter
separately pins `nightly-2026-07-23` with `rustc-dev`, `rust-src`, and
`llvm-tools-preview` in
[`rust-toolchain.toml`](leaner-rust/rust-exporter/rust-toolchain.toml).

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

## Build and test

Build or test the package you changed from its own directory. These four
suites are the test matrix; the deprecated packages are not part of test runs
or CI (their libraries still compile as e2e dependencies):

```bash
(cd leaner-ir        && lake build && lake test)
(cd leaner-move      && lake build && lake test)
(cd leaner-rust      && lake build && lake test)
(cd leaner-e2e-tests && lake build && lake test)
```

`leaner-ir`'s suite includes a verification-cost benchmark
(`LeanerLang/Tests/DenotePerformance.lean`) that gates every `verify`
target against `DenotePerformance.exp` on elaborator heartbeats and
proof-term size. A deliberate cost change is recorded with
`lake build && UB=1 lake env lean LeanerLang/Tests/DenotePerformance.lean`;
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
| `leaner-ir` schema, validation, or semantics | `leaner-ir`, `leaner-move`, `leaner-rust`, and `leaner-e2e-tests` tests |
| Rust profile/import/source backend | `leaner-rust` tests plus Rust exporter tests when its code or fixtures are involved |
| Move profile or the exchange frontend/LIR adapter | `leaner-move` and `leaner-e2e-tests` tests |

For a fast check of one Lean file, run it from its package directory so imports
and relative test assets resolve correctly:

```bash
cd leaner-ir
lake env lean LeanerIR/Tests/Validation.lean
```

Do not add `sorry`, axioms, or other admissions to make a proof compile unless
the design explicitly models an assumption and the change is requested.

### Rust exporter

Run the project-owned exporter checks from its standalone crate:

```bash
cd leaner-rust/rust-exporter
cargo test --features rustc-public
cargo fmt --check
cargo clippy --features rustc-public --all-targets
```

Build and invoke it directly with the matching sysroot library path:

```bash
cargo build --features rustc-public
LD_LIBRARY_PATH="$(rustc --print sysroot)/lib" \
  target/debug/leaner-rust-export --output /tmp/basic.raw.json -- \
  tests/raw-unit/basic.rs --crate-type=lib --edition=2024
```

Normally prefer the Lean-owned driver because it builds the pinned exporter,
tracks cache inputs, and validates RawUnit before writing it:

```bash
cd leaner-rust
lake exe leaner-rust -- import-file rust-exporter/tests/raw-unit/basic.rs \
  --output .lake/leaner-rust/basic.raw.json

lake exe leaner-rust -- import-crate Tests/CargoFixture/Cargo.toml \
  --package leaner-rust-cargo-fixture \
  --features extra --no-default-features \
  --output .lake/leaner-rust/cargo-fixture.raw.json
```

`LEANER_RUST_EXPORTER`, `LEANER_RUST_SYSROOT`, and `LEANER_RUST_CACHE` override
the development exporter, paired sysroot, and managed cache respectively.

## Developer tools and generated fixtures

E2E baselines are regenerated with `UB=1 lake test` (or a `UB=1` driver run)
from `leaner-e2e-tests/`; review every diff before committing. The deprecated
transpiler CLI and its printer baselines remain usable for reference but are
not maintained.

Positive Rust exporter inputs and their side-by-side `.exp.json` baselines are
in `leaner-rust/rust-exporter/tests/raw-unit/`; rejection and API-probe inputs
remain under `tests/fixtures/`. The baseline driver discovers every `.rs` in
`raw-unit` automatically. Use `UB=1 cargo test --features rustc-public` to
update expectations, inspect the semantic diff, and commit an update only when
the format or mapped MIR is intentionally changing. The Rust and Lean suites
both validate these artifacts.

The `scripts/` proof-analysis tools and the Account XIR regeneration entry
point target the deprecated `move` package and are kept for reference only.

## Change discipline

- Read existing tests and the relevant design's status/deferred-work sections
  before changing behavior. Preserve explicit unsupported-feature diagnostics;
  do not silently give Rust constructs Move semantics or erase unsupported MIR.
- Keep RawUnit JSON versioned, deterministic, and canonical. Validation is the
  trust boundary: frontends may produce raw data, but semantic consumers should
  use checked/validated forms.
- Keep production dependencies one-way. Put source/result tests that genuinely
  cross several layers in `leaner-e2e-tests`; keep assertion-style unit tests in
  the owning package's `Tests` tree instead of adding a reverse dependency.
- Generated Lean, JSON, XIR, and report files are reviewable test assets. Change
  them only through the owning generator and include them with the source change.
- Ordinary `lake build` must not write source-tree artifacts. Use explicit
  exporter/regeneration commands for generated files.
- Make focused changes and do not modify unrelated dirty worktree files.
