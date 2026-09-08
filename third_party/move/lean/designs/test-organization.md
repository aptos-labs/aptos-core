# Test organization: verification checks as baselines

Status: plan, revised 2026-09-02 with the user's decisions (no Move
Prover differential; verification tests are baseline tests in the
end-to-end package; the v0 trees are ported one file at a time).
T0 and T3 are implemented; T1 is in progress (4 of the 23 verification
files are ported, and every generated-route gap those ports exposed has
been closed, recursion included); T2 is not started.

## The two needs

1. **A benchmark plugged into the ground.** v0 (the deprecated `move`
   package) verified its acceptance corpus with mostly bare `verify`
   commands, and its language and negative tests pinned elaboration,
   compilation, and diagnostics.  The leaner stack must reach parity on
   that corpus and then be measured against it:
   - [`move/Move/Tests/Verification/`](../move/Move/Tests/Verification/):
     23 files, 174 `verify` commands, 99 bare (automatic) and 75 with
     proof scripts; `Quicksort.lean` (3 verifies, ~220 proof lines) and
     `OrderedMap.lean` (9 verifies, ~480 proof lines) carry the
     AI-generated proofs.
   - [`move/Move/Tests/Language/`](../move/Move/Tests/Language/): 21
     files covering the language surface (integers of every width, signed
     arithmetic, enums and their payloads and references, generics,
     loops, tuples, vectors, addresses, attributes, abilities).
   - [`move/Move/Tests/Negative/`](../move/Move/Tests/Negative/): 9 files
     pinning diagnostics (borrow errors, returned `&mut` shapes, lowering
     and surface rejections, spec-function and specification errors,
     verification failures).
2. **Every end-to-end test verifies.** We map one language with
   specifications into LeanerLang, and the produced LeanerLang must
   verify.  Today no end-to-end test elaborates its output.

## What exists today

| Suite | What it checks | Verification |
|---|---|---|
| `leaner-ir` `LeanerIR/Tests` (16 files) | IR validation, semantics, interpreter, codecs | none (unit tests) |
| `leaner-ir` `LeanerLang/Tests` | elaborator (`Frontend`), three hand row proofs, 10 `Verification*.lean` fixtures (46 verifies), the `Performance` gate (15 targets) | yes, on hand-authored LeanerLang |
| `leaner-move` `Tests` (2 files) | profile, intrinsics | none |
| `leaner-rust` `LeanerRust/Tests` | driver, registry, source backend, 9 LeanerLang programs | none |
| `leaner-e2e-tests` `MoveToLeanerLang` | 13 direct `.move` files plus MoveStdlib (14 modules, 106 spec blocks), AptosStdlib (6), AptosFramework (2): printed LeanerLang compared textually and re-parsed for a canonical fixed point | none |
| `leaner-e2e-tests` `RustToLeanerLang` | 53 `.rs` fixtures, same textual baseline | none |
| `leaner-e2e-tests` `MonoVM`, `MonoDifferential` | linked MonoVM smoke test; three-engine differential execution | none (execution) |

The gap: the corpus that defines "done" is not run by anything, the
end-to-end output is text that nobody elaborates, and the verification
fixtures that do exist are a small hand-picked set with no relation to
either.

## The convention

A **check** is a LeanerLang source file under
`leaner-e2e-tests/LeanerE2ETests/Check/`: a `.lean` file that imports
`LeanerLang` and contains `leaner module` declarations with
specifications, `verify` commands (bare, or with an inline proof script
where real mathematics lives, as v0 did), and whatever other commands the
test needs.  **Everything under `Check/` is LeanerLang** — no Move
sources, no Rust sources, and not the v0 surface: a v0 test is ported by
rewriting it in LeanerLang.  The frontend paths keep their own
directories (`MoveToLeanerLang`, `RustToLeanerLang`).  The driver
elaborates the file and records its diagnostics.

- **The output is the baseline, verbatim**, side by side as `<name>.exp`,
  the way compiler-v2's baseline tests record theirs
  (`move-prover/test-utils/src/baseline_test.rs`,
  `verify_or_update_baseline`): the `.exp` is exactly what `lean` prints
  for the file — `LeanerE2ETests/Check/Verification/WrongIncrement.lean:12:2:
  error: verification failed for …` and the message body as the user
  reads it — with no reformatting.  The only cleaning is compiler-v2's:
  trailing whitespace trimmed, a single trailing newline.  The driver
  invokes `lean` on the package-relative path, so the path in the
  message is stable across machines without rewriting.
- **A clean check has no `.exp`.**  As in compiler-v2, `UB=1` writes the
  file when there is output and **removes** it when there is none, and
  a missing file is an empty expectation.  Positive and negative tests
  are therefore the same kind of file; a negative test is one whose
  `.exp` exists — a verification failure at its clause range, a rejected
  borrow, an unsupported construct.  Every severity is recorded, so an
  unexpected warning is as visible as an error; tests set options locally
  when they mean to silence something.
- **Each file runs in its own `lean` process** with the package's search
  path, so a runaway proof or a crash in one check cannot take the driver
  down, and under a per-command heartbeat cap that the driver passes
  (start at 50M, five times the largest generated route), so a proof that
  silently falls back to search surfaces as a diff instead of a slow
  suite.  Cost stays gated precisely by `LeanerLang/Tests/Performance`;
  the cap only catches collapses.
- **Discovery, not registration.** Every `.lean` under `Check/` is a
  check; subdirectories mirror the v0 categories.

```text
leaner-e2e-tests/LeanerE2ETests/
  Check/
    Verification/
      Account.lean            # clean: no .exp
      Quicksort.lean          # proofs inline, clean: no .exp
      WrongIncrement.lean     # negative
      WrongIncrement.exp      # LeanerE2ETests/Check/Verification/WrongIncrement.lean:12:2: error: verification failed …
    Negative/
      Borrows.lean
      Borrows.exp
    Language/
      Integers.lean
  MoveToLeanerLang/           # unchanged (see T2 for the verifying stage)
  RustToLeanerLang/           # unchanged
  MonoVM/, MonoDifferential/  # unchanged
```

`LEANER_E2E_SUITE=check` selects the suite; `lake test` runs it with the
others.  `LeanerIR.TestInfra.Baseline` gains compiler-v2's semantics
(`check` today always writes the file, even an empty one): write when
the output is non-empty, remove when it is empty, trim trailing
whitespace.

## The port ledger

The v0 trees are ported one file at a time — each file rewritten in
LeanerLang, since the v0 surface is a different language — each check
landing only when it passes: a clean run for a positive test, the
intended diagnostics for a negative one.  The deprecated file stays as
the reference until its port lands.  The ledger below is kept current in this document; a row
moves to **ported** with the commit that lands it.

| v0 file | verifies (proofs) | status |
|---|---|---|
| `Verification/AbortDirections.lean` | 5 | **ported** (2026-09-02, `Check/Verification/Aborts.lean`; all five verify, `withdraw` through the path-generic bracket and the absent-resource abort direction) |
| `Verification/Account.lean` | 2 | **ported** (2026-09-02, `Check/Verification/Account.lean`; its `#test` execution assertions are left to the differential suite) |
| `Verification/BorrowCertificates.lean` | 0 (certificates only) | not ported |
| `Verification/Callees.lean` | 13 (3 proofs) | **ported** (2026-09-02, `Check/Verification/Callees.lean`; `bump`, `bump_twice`, `take_and_bump`, the added `add_two`/`bump_then_add_two` (a callee whose exit value is not the entry plus one), the unspecified-helper callers `calls_pure_helper`, `embedded_helper`, `helper_condition`, `set_pair`, `forward_set_pair`, and `bump_counter` verify; `drain` and `call_drain` verify since 2026-09-03 through the recursive fixed point, where v0 needed hand proofs over the recursive contract) |
| `Verification/Calls.lean` | 10 (3 proofs) | **ported** (2026-09-02, `Check/Verification/Calls.lean`; `twice`, `increment`, `increment_unspecified`, `add_to`, `effect_caller`, `read_counter`, `choose`, `call_choose` verify, and `recursive_choose` since 2026-09-03 through the recursive fixed point, where v0 needed a hand proof over the recursive contract) |
| `Verification/CorePrimitives.lean` | 5 | not ported |
| `Verification/CrossInv.lean` | 1 | not ported |
| `Verification/EnumRefs.lean` | 10 | not ported |
| `Verification/GenericStorage.lean` | 7 | not ported |
| `Verification/GlobalBorrows.lean` | 6 | not ported |
| `Verification/GlobalInv.lean` | 5 | not ported |
| `Verification/Invariants.lean` | 5 (3 proofs) | not ported |
| `Verification/Loans.lean` | 3 | not ported |
| `Verification/LoopInvariants.lean` | 3 | not ported |
| `Verification/LooseFrame.lean` | 2 | not ported |
| `Verification/OrderedMap.lean` | 9 (9 proofs) | not ported |
| `Verification/Quicksort.lean` | 3 (3 proofs) | not ported |
| `Verification/Read.lean` | 2 (2 proofs) | not ported |
| `Verification/ResourceComposition.lean` | 1 | not ported |
| `Verification/ReturnedMutRefs.lean` | 66 (51 proofs) | not ported |
| `Verification/SpecFunctions.lean` | 12 | not ported |
| `Verification/SpecLogicalArithmetic.lean` | 1 (1 proofs) | not ported |
| `Verification/Summaries.lean` | 3 | not ported |
| `Negative/` (9 files, 39 pinned diagnostics) | | not ported |
| `Language/` (21 files) | | not ported |

Totals to reach: 174 verifies (99 automatic, 75 with proofs) across the
23 verification files.  v0 files also carry `#test` execution assertions
over the lowered module; a port keeps them where the leaner surface has
an equivalent and otherwise leaves execution to the MonoVM differential
suite, which owns it.

## Where existing tests go

- `LeanerLang/Tests/Verification*.lean` (46 verifies): each becomes a
  check under `Check/Verification/` and the fixture is deleted once its
  check lands; the negative ones (`VerificationAborts`, `reborrow_bad`)
  land with their `.exp`.
- `LeanerLang/Tests/Performance.lean` and `.exp`: unchanged; it is the
  cost gate and stays a curated set.
- The hand row proofs (`NativeRow`, `NativeCallRow`, `NativeStoreRow`):
  unchanged; they test proof rules, not programs.
- Assertion-style tests in `leaner-move`, `leaner-rust`, and
  `LeanerIR/Tests`: unchanged.
- The deprecated packages stay until the ledger is complete.

## Milestones

| Status | Step | Gate |
|---|---|---|
| **DONE (2026-09-02)** | **T0 the check driver.** `LEANER_E2E_SUITE=check` (`LeanerE2ETests/Check.lean`): discovers `Check/**/*.lean`, runs each in its own `lean` process under the heartbeat cap, records its output verbatim, compares against the optional `.exp` through `Baseline.checkOutput` (`UB=1` writes or removes, as compiler-v2 does). The cap reaches the generated theorems through the registered option `leaner.verifyHeartbeats`, which the generator reads in place of its former literal budget (default unchanged); the driver passes it as `-Dweak.leaner.verifyHeartbeats`. Seeded with `Check/Verification/Increment.lean` (clean, no `.exp`) and `Check/Negative/WrongIncrement.lean`, whose `.exp` is lean's two diagnostics: the clause not established at its range, and the module's verification failure. | Met: the suite runs in `lake test`; the positive seed has no `.exp`, the negative seed's `.exp` is the verbatim output; a deliberately wrong clause on the positive seed produced a baseline diff, and a budget of 10 produced the deterministic timeout at the `verify`. |
| **IN PROGRESS (since 2026-09-02)** | **T1 port the v0 trees.** One file at a time, ledger kept current. Recommended order: `Verification/` first (it exercises `verify`, the thing being measured; automatic ones before the proof-carrying ones, `Quicksort` and `OrderedMap` last), then `Negative/` (diagnostics depend on validation messages that must be stable first), then `Language/` (many of its assertions need interpreter and compilation hooks the leaner surface must expose). A file that cannot pass yet lands as a negative check whose `.exp` names the construct, so the gap is visible instead of silent. Progress 2026-09-03: `AbortDirections`, `Account`, `Callees`, and `Calls` are ported; the constructs their `.exp` files named (a `let` before a call, `Bool` parameters, unspecified helpers, both-parameter reborrows, global field borrows, the path-generic bracket, the absent-resource abort direction) all landed on generated routes, and recursion followed on 2026-09-03, so none of the four carries a `.exp`. | Every ledger row is ported. Parity target: every v0 automatic verify is automatic here; every v0 proof-carrying verify carries a proof here. |
| NOT STARTED | **T2 the produced LeanerLang verifies.** The Move stage appends `verify f` for every spec'd function with a body (v0's `--verify`), so the `.exp.lean` baselines carry the commands, and every `.exp.lean` under `MoveToLeanerLang` runs as a check with `<module>.check.exp` beside it. Most stdlib rows will read as failures at first: that is the benchmark plugged into the ground. Same later for `RustToLeanerLang` once Rust contracts exist (rust-mir-design M2). | Every produced module with a spec has a check result; a `proved` that turns into anything else fails the suite. |
| **DONE (2026-09-02)** | **T3 retire the hand fixtures.** The ten `LeanerLang/Tests/Verification*.lean` fixtures are checks under `Check/Verification/` (`Prophecies`, `Aborts`, `Storage`, `Generics`, `Corpus`, `References`, `Loops`, `Typed`, `Rust`, plus the seeds `Increment` and `Account`); their `.exp` files record the constructs without a generated route, since the frame route was retired the same day. | Met: `leaner-ir`'s verification tests are the row proofs and the cost gate only. |

T0 and T2 are driver work with no proof engineering and are independent
of the route work in [`certifying-execution.md`](certifying-execution.md);
they are what makes that work measurable.  T1 is where the frontends'
and routes' gaps become ledger rows.

## Deliberately not in scope

- A Move Prover differential.  The parity claim is against v0's recorded
  verdicts and proofs, not against Boogie's.
- Proving compiler correctness for the emitted bytecode; the claim is
  "the same verdict on the same specification", per the CLAUDE.md
  separation of claims.
- Deleting the deprecated packages before the ledger is complete.
