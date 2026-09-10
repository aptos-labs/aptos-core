# Leaner end-to-end baselines

This package contains source-to-LeanerLang baseline tests for the two frontend
paths:

```text
Move source -> compiler-v2 XAST -> validated Move LIR -> LeanerLang or error
Rust source -> rustc MIR exporter -> validated Rust LIR -> LeanerLang or error
```

Inputs and expectations are side by side in feature directories:

```text
LeanerE2ETests/MoveToLeanerLang/
  move_scalar.move
  move_scalar.exp.lean
  MoveStdlib/
    Move.toml
    sources/hash.move
    sources/hash.exp.lean

LeanerE2ETests/RustToLeanerLang/
  scalar.rs
  scalar.exp.lean
  raw_pointer.rs
  raw_pointer.exp.lean
```

The driver discovers every direct `.move` or `.rs` frontend input and every
immediate Move package identified by `Move.toml`, then derives side-by-side
`.exp.lean` paths. A successful expectation contains canonical LeanerLang. An
unsupported or invalid input contains the normalized, source-located concrete
diagnostic as Lean line comments beginning with `-- error:`. The expectation
therefore remains valid target-language source. Adding a case requires no
driver registration.

The real MoveStdlib package is owned here. All 15 exported modules currently
produce parse-checked canonical LeanerLang. Individual declarations which the
Move XAST boundary cannot yet represent remain visible as source-located
`-- unsupported Move declaration ...` comments in the corresponding module
baseline. Legacy generated Move-flavored Lean fixtures remain in the
transpiler package and read this canonical source package.

`lake test` runs an executable baseline driver, so checks run even when Lake
can reuse every compiled module. A mismatch reports a line diff. Use
`UB=1 lake test` to replace expectations; `UPBL` and `UPDATE_BASELINE` are
accepted aliases. Review the resulting source diff before committing it.

`LeanerE2ETests/Check/` holds LeanerLang checks: every `.lean` file below it,
at any depth, is a LeanerLang source — modules with specifications, `verify`
commands, proof scripts where real mathematics lives — that the driver
elaborates in its own `lean` process under a heartbeat cap. What `lean`
prints is the baseline, verbatim, beside the source as `<name>.exp`,
following compiler-v2's baseline convention: a clean check has no
expectation file, a check that prints anything has exactly that output as
its expectation, and `UB=1` writes or removes the file. Positive and
negative tests are the same kind of file. Everything under `Check/` is
LeanerLang; the frontend paths keep their own directories.

The `LEANER_E2E_SUITE` environment variable selects one suite: `move`, `rust`,
`check`, `monovm`, or `monodiff`. The `monovm` suite is the linked MonoVM smoke check: it calls the
`mono-move-lean-link` adapter staticlib through the C shim that only the
`LeanerE2ETestDriver` executable links (`moreLinkObjs` in the `lakefile.lean`;
the package libraries and the language server stay free of the native
dependency). Lake owns the native build — the `monovm_staticlib` target runs
Cargo in the explicitly selected release profile from this checkout, and
`monovm_shim` compiles the shim with `leanc`. The suite runs the whole
differential story only through the linked executable; `LeanerE2ETests.MonoVM`
modules elaborate under `lake env lean` but do not evaluate there.

The `monodiff` suite is the three-way differential harness over the executable
Move fixtures in `LeanerE2ETests/MonoDifferential/`. Each `.move` fixture
carries `// RUN:` execute directives and pairs with a side-by-side `.exp`
recording what all three engines produced for every step — the linked MonoVM
adapter, the validated Move LIR, and the LIR after the LeanerLang round trip:

```text
step 0: 0x42::scalar::add(1, 2)
  mono:       returned 3
  LIR:        returned 3
  round trip: returned 3
```

The baseline is the whole check: equal lines are agreement, and a divergence
between the engines appears as a diff of the line that changed. Agreement
records nothing extra, but a divergence adds an explicit line naming it:

```text
  mono:       failed InvalidOperation (Add: under/overflow)
  LIR:        aborted 300
  round trip: aborted 300
  ERROR: mono and LIR diverge
```

Exhaustion never produces one — an `inconclusive` line means an engine ran out
of its own budget, which proves nothing about the others. A recorded `ERROR:`
line is an open finding, tracked in the findings register of
`designs/monovm-link-design.md`, not an accepted expectation.

Only normalized content is recorded, so the file is deterministic: gas usage,
collection counts, abort locations, and the adapter's build identity are
triage output, never expectations. Integers compare numerically and addresses
by identity rather than spelling, so a diff always means a semantic change.
`UB=1` updates these like any other baseline — review the diff.

Reusable discovery, diffing, and update support lives in
`LeanerIR.TestInfra.Baseline`. Assertion-style frontend, printer, LIR-boundary,
intrinsic, round-trip, and behavior tests live with their implementations under
`transpiler/Transpiler/Tests` or `leaner-rust/LeanerRust/Tests`; they are
deliberately not E2E tests.
