# CLAUDE.md

Repository-local guidance for working in `third_party/move/lean`. Also follow
the Aptos Core guidance in [`../../../CLAUDE.md`](../../../CLAUDE.md).
[`README.md`](README.md) describes the project, its packages, and the build
and verification commands; this file holds the working rules.

## Rules of the tree

- The current codebase is the verex stack: `verex-ir`, `verex-move`,
  `verex-rust` (with `rust-exporter`), and `verex-e2e-tests`. The `move`,
  `move-model`, and `transpiler` packages under `v0/` are **deprecated**,
  reference only: do not add functionality or tests to them, do not include
  their suites in test runs or CI, and do not root new work in them.
- Keep three claims separate: `verify f` proves a theorem about the generated
  source semantics of a function; the compiler lowers supported declarations to
  XIR and production Move bytecode; a compiler-correctness theorem connecting
  the two is future work. Do not describe source verification as verification
  of the emitted bytecode.
- There is no root Lake workspace. Every top-level component is an independent
  Lake package with its own `lakefile.toml`, manifest, and pinned
  `lean-toolchain`; run `lake` from the package directory. All packages pin Lean
  `v4.32.2`, and the exporter pins `nightly-2026-07-23`
  ([`rust-toolchain.toml`](verex-rust/rust-exporter/rust-toolchain.toml)). Never
  change a version in only one package.
- Keep production dependencies one-way:
  `verex-ir <- verex-move`, `verex-ir <- verex-rust <- rust-exporter`, and both
  `<- verex-e2e-tests`. `verex-e2e-tests` is deliberately test-only and may
  depend on every layer it exercises. Put source/result tests that genuinely
  cross layers there, and keep assertion-style unit tests in the owning
  package's `Tests` tree instead of adding a reverse dependency.
- Within a package, the root `Foo.lean` is the public import, source modules
  live under `Foo/`, and tests are either under `Foo/Tests/` or exposed through
  a test library listed in `lakefile.toml`. `.lake/` is build output, not source.
- Every source file in this tree, Lean and Rust alike, carries the Apache
  header (`Copyright © Aptos Foundation` / `SPDX-License-Identifier:
  Apache-2.0`), not the Innovation-Enabling header used elsewhere in the
  repository. The Rust fixtures' headers are exported as comments, so a header
  change regenerates the RawUnit and Rust e2e baselines.
- Do not add `sorry`, axioms, or other admissions to make a proof compile unless
  the design explicitly models an assumption and the change is requested.

## Documentation (`doc/`)

Read the design for the layer being changed; the index is
[`doc/README.md`](doc/README.md). Designs hold the model and the decisions, not
status: what is open is in [`doc/plan/roadmap.md`](doc/plan/roadmap.md), and
state that the source does not show is in [`doc/status/`](doc/status/).

## Exchange frontend

Core Lean libraries build without an Aptos CLI. **The contract for
`APTOS_MOVE_CLI` is that it names a Cargo ci-profile build of the standalone
`move` CLI** (`cargo build --locked --profile ci -p aptos-move-cli --features
binary --bin move` → `<workspace>/target/ci/move`), which the E2E driver runs
and uses by default, once per suite invocation; pointing the variable anywhere
else draws a warning because this checkout's build cannot freshen it.
`APTOS_CLI=<path-to-full-aptos-cli>` is the unmanaged escape hatch whose
freshness its setter owns, and receives `aptos move exchange` where the
standalone CLI receives `exchange`. An unrelated `move` executable on `PATH` is
never a substitute.

## Build and test

Build or test the package you changed from its own directory. These four suites
are the test matrix:

```bash
(cd verex-ir        && lake build && lake test)
(cd verex-move      && lake build && lake test)
(cd verex-rust      && lake build && lake test)
(cd verex-e2e-tests && lake build && lake test)
```

Run downstream suites when changing a shared boundary:

| Changed area | Minimum relevant verification |
|---|---|
| `verex-ir` schema, validation, or semantics | `verex-ir`, `verex-move`, `verex-rust`, and `verex-e2e-tests` tests |
| Rust profile/import/source backend | `verex-rust` tests plus Rust exporter tests when its code or fixtures are involved |
| Move profile or the exchange frontend/VIR adapter | `verex-move` and `verex-e2e-tests` tests |

For a fast check of one Lean file, run it from its package directory so imports
and relative test assets resolve correctly (`cd verex-ir && lake env lean
VerexIR/Tests/Validation.lean`).

### Editing through the language server

A built package can also be edited through the Lean language server, which
re-elaborates only from the first changed command instead of redoing the file.
It needs the package built once; nothing in the server builds imports, and the
`.lake` of one worktree does not serve another. Prefer it over a `lake env lean`
run per attempt, and prefer `lake build <Module>` over a whole-package build
when a rebuild is needed.

**A file's diagnostics are trustworthy only when no file it transitively imports
has been edited.** Imports resolve against built `.olean` files, so after an
edit upstream the diagnostics of a downstream file describe the *previous*
upstream: they can be falsely red (naming an API that now exists) and falsely
green (missing a break the edit caused). `lake env lean` reads the same stale
oleans, so this is the build model, not a property of the editor. Decide which
mode a task is in before starting it, and do not drift between them:

- *Within one module*: iterate edit/diagnose freely, with no rebuild.
- *Across modules*: make every coordinated edit without consulting diagnostics,
  rebuild the changed modules, and only then read diagnostics for the set. Each
  rebuild is a generation boundary.

Reuse is by prefix: everything from the edit point onward re-elaborates, so an
edit early in a file of several thousand lines costs the whole file while one
late in it costs a declaration. That is a cost to expect, not a reason to place
a declaration anywhere but in the part of its topic. Read this tree through the
server's own queries (file outline, hover, declaration, references, symbol
search) rather than by opening whole modules; the largest files here are
thousands of lines, and the saving is the context the rest of the task needs.
No package in this stack depends on Mathlib, so Mathlib search and the lemma
names it suggests do not apply.

The Move Prover's lean gate (`MVP_TEST_FEATURE=lean cargo test -p move-prover
--test testsuite`) builds `verex-move` itself and runs a private copy of it
under `target/tmp`, so a concurrent Lake build cannot change what a test loads,
and a harness failure of the verifier is never written as a baseline; see
`doc/designs/source-verification.md` ("From the Move Prover").

### Between turns: the smoke suite

Between edit batches, run `scripts/smoke.sh`, described in
[`.cursor/skills/verex-smoke/SKILL.md`](../../../.cursor/skills/verex-smoke/SKILL.md):
it builds in place and runs the fixtures that catch this stack's regressions,
read-only against their `.exp`, and refuses `UB=1`. `--rust` runs the Rust path
instead, `--no-exe` skips the executable and the two items needing it. Green
smoke is not the commit gate; the four suites and the Move Prover's lean gate
are.

While iterating, build libraries, not executables: `lake build` in `verex-ir`,
`lake build VerexMove` in `verex-move`, and `lake build VerexE2ETests` in
`verex-e2e-tests`. The default targets of the last two also build the
`verex-move` executable and the e2e driver, whose C compilation and linking the
Lean fixtures do not need (10–30 s after an edit of one part of the solver). The
smoke suite builds the executable beside its Lean fixtures. To verify a Move
package while iterating,
`scripts/verify-modules.py <package> [--filter <part>]` runs one `verex-move
verify` process per module, several at a time.

For repeated verifier measurements, build the native executable once per source
revision and use `verex-move elaborate <file>` or
`scripts/verex-bench.py run --only …`; `lake env lean` interprets the verifier,
so its timings and heartbeats are not directly comparable. Isolate a slow target
with `scripts/isolate-target.py <file> <module> <target> -o <out>` and run
`verex-move elaborate <out>` before repeating a whole suite. See
[`doc/designs/performance.md`](doc/designs/performance.md) for profiling and
measurement details.

The solver, the contract generator and `verify` are split into parts for build
time, listed in [`doc/designs/performance.md`](doc/designs/performance.md)
("Modules split for edits"); the last part of each keeps the original module
name, so importers are unchanged. Put a new declaration in the part of its
topic; one that another part uses cannot be `private`. Keep the import
boundaries: `Solve.Normal` and `Solve.Decide` independent of each other, and
`Verify.Contracts` free of the solver, so that an edit of one rebuilds neither
the other nor that part.

When writing or extending large metaprogram functions, prefer coherent helper
functions over a single growing body. Smaller functions can reduce optimization
and code-generation work in both Lean and the C compiler; this is a build-time
consideration as well as a readability choice. Profile before refactoring for
speed, and measure Lean compilation, C compilation, and verifier runtime
separately: inlining can undo the split, and function boundaries can affect
runtime performance. Splitting functions within a module does not by itself
enable parallel compilation or finer incremental rebuilds; those benefits
require suitable module boundaries and independent dependencies.

### Parallel agents on one worktree

Several agents can work this tree at once if the worktree itself stays green
and buildable between their landings. The rules that make that hold:

- **Partition by file, along the split modules.** Each agent owns a disjoint
  set of source files and treats every other file as read-only. A change
  needed in someone else's file is requested with its exact shape (statement,
  home, reason), not made. The module split for build time doubles as the
  ownership boundary, so one agent's edits never rebuild another's part.
- **Iterate in a copy outside the repository.** Copy the package to scratch
  space and seed its `.lake` from the worktree's built state (in a worktree
  `.lake` is a symlink; copy its resolved target). Build, probe, and measure
  in the copy. Re-sync files you do not own from the worktree before each
  round of work, since others land theirs meanwhile.
- **Port whole files, only green.** When the copy builds and the item's
  fixtures meet their gates, copy the owned files back whole-file and run one
  verification build in the worktree. Never port a partial state; a
  half-landed cross-file change is the one thing that breaks everyone else.
- **Long sweeps read a snapshot.** A build deletes and rewrites oleans under
  a running reader, so a fixture sweep runs against an rsync snapshot of
  `.lake/build`, never the live tree, and nobody builds in the worktree
  while a snapshot is being taken.
- **Checkpoint on green.** A checkpoint commit requires the four packages
  building and the owning suites green, with the diff checked for admissions.
  Report cost before/after for leaf-pipeline changes, and follow the baseline
  rules below — in particular, a clean fixture has no `.exp` at all, so never
  record an empty one.

For the inner loop inside one module, prefer the language server (previous
section); `lake build` remains the gate before any port.

### Diagnosing a failure

A probe is expensive here: a scratch file that imports `VerexSL` and runs
`verify` interprets the verifier and takes minutes, and a rebuild after an edit
of the solver or of `Types.lean` takes longer. Analyse the code first, and
spend a probe only on a question the source cannot answer.

- **Read before running.** Start from the message and the code that emits it
  (`grep` the text), then read the definitions on the path through the language
  server's queries. State the hypothesis the probe is to decide before writing
  it; a probe run to "see what happens" usually has to be run again.
- **Compare the sides.** After a merge or a restack, `git show <rev>:<file>`
  and `git diff` of the two parents answer most questions without elaborating
  anything: which side a binder, an arity, or a match arm comes from. A branch
  tip is not evidence that a test passed there; check that it built.
- **Check the signature, not the source text.** Section `variable`s reach a
  declaration by use: a theorem takes every instance variable whose
  dependencies it has, and a definition also takes one its body's proofs use,
  which `omit` does not prevent. `#check @name` on the built package is the
  cheap, decisive query; metaprograms that build the application with
  `mkAppN` or test it with `isAppOfArity` must agree with what it prints.
- **Positional names move.** Equation lemmas (`f.eq_12`) and constructor
  indices are numbered by arm order, so inserting an arm silently retargets a
  metaprogram that names one. Search for such names when adding a constructor.
- **Reducible definitions hide from the simplifier's index.** `NTy.subst`,
  `NRow.subst` and `NTy.unmark` are reducible, so a closed application of one
  is indexed by what it reduces to: a `simproc` on the unreduced pattern does
  not fire, and "made no progress" on such a term is not a defect. A residue
  like `(NTy.int 64 false).unmark` is the same type to a tactic and a different
  term to a metaprogram that matches syntax.
- **Make the one probe small.** Isolate the target
  (`scripts/isolate-target.py`), set `pragma verify = false` and verify the one
  function, and print the one hypothesis or argument in question rather than
  the whole goal. Prefer the built executable (`verex-move elaborate`) over
  `lake env lean` for anything repeated.

### Baselines and generated fixtures

`verex-ir`'s suite includes a verification-cost benchmark
(`VerexSL/Tests/DenotePerformance.lean`) that gates every `verify` target
against `DenotePerformance.exp` on elaborator heartbeats and proof-term size. A
deliberate cost change is recorded with `lake build && UB=1 lake env lean
VerexSL/Tests/DenotePerformance.lean`; review the diff, which should name only
the targets the change was meant to move. The build belongs to the command:
`lake env lean` only sets the module path, so regenerating without it records
the cost of whatever imports happen to be built and commits a baseline that its
own source does not reproduce.

E2E baselines are regenerated with `UB=1 lake test` (or a `UB=1` driver run)
from `verex-e2e-tests/`; review every diff before committing. Rust exporter
inputs and their `.exp.json` baselines live under
`verex-rust/rust-exporter/tests/`, described in its
[`README.md`](verex-rust/rust-exporter/README.md); regenerate with `UB=1 cargo
test --features rustc-public`, inspect the semantic diff, and commit an update
only when the format or mapped MIR is intentionally changing. The Rust and Lean
suites both validate these artifacts.

### Rust exporter

Run the project-owned checks from its standalone crate: `cargo test --features
rustc-public`, `cargo fmt --check`, and `cargo clippy --features rustc-public
--all-targets`. Prefer the Lean-owned driver (`lake exe verex-rust -- import-file
…` / `import-crate …`) over invoking the exporter binary, because it builds the
pinned exporter, tracks cache inputs, and validates RawUnit before writing it;
its commands and the `VEREX_RUST_EXPORTER`, `VEREX_RUST_SYSROOT` and
`VEREX_RUST_CACHE` overrides are in the crate's
[`README.md`](verex-rust/rust-exporter/README.md).

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
- Generated Lean, JSON, XIR, and report files are reviewable test assets. Change
  them only through the owning generator and include them with the source change.
- Ordinary `lake build` must not write source-tree artifacts. Use explicit
  exporter/regeneration commands for generated files.
- Make focused changes and do not modify unrelated dirty worktree files.
