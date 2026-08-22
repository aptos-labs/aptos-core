# Handover — Leaner Move

Branch `wrwg/leaner`, worktree `dev1`.  **Last commit `97c2d43ade`; everything
below is uncommitted.**

## Layout (done, verified)

Two Lake packages, flat at `third_party/move/lean/`:

```
move/        package "move"       -> library Move       (requires move-model)
move-model/  package "move-model" -> library MoveModel
scripts/  design/  README.md
```

Naming follows the Lean convention: packages and executables lowercase,
libraries and module components PascalCase.  A package is the
distribution/dependency unit; a library is a build target inside it, and its
name need not match the namespace it owns.

Tests are split per library — `Move.Tests.*` and `MoveModel.Tests.*` — with
aggregate roots `move/Move/Tests.lean` and `move-model/MoveModel/Tests.lean`.
Lake needs a root module per test library; `globs` does not work (tried three
spellings).  The shared helper is `MoveModel.Tests.Common`, imported across the
package boundary by the Move tests.

Verify (all with `ulimit -n 65535`):

```bash
cd move        && lake test    # 80 jobs
cd move-model  && lake test    # 38 jobs
cargo test -p move-compiler-v2-transactional-tests -- leaner   # 104
cargo test -p move-compiler-v2 --lib leaner                    # 5
```

`Move.Examples` was deleted: genuinely superseded by `Move.Tests.Account`,
which compiles the same module *and* executes and verifies it.

## The open task

`MoveModel.Examples` (8 files, ~2100 lines) holds **all** the IR-level
verification — 26 theorems, against zero in `MoveModel.Tests`.  It must be
**repaired, then moved into `MoveModel.Tests.*`**, then the `Examples`
`lean_lib` dropped from `move-model/lakefile.toml`.  Porting it in also stops
the rot: it broke unnoticed precisely because `Examples` is in neither
`defaultTargets` nor `lake test`.

Three files were broken by commit `5636dfc1e0` (the integer unification), which
replaced the per-width `Value` constructors with a single `.int`:

| file | state |
|---|---|
| `Adequacy`, `Account`, `CountDown`, `BorrowAccount`, `CrossCall` | build clean — port as-is |
| `MoveSource` | **fixed**, builds clean |
| `MasmSource` | 2 of 4 sites fixed; errors remain at ~347 and ~409 |
| `ElimSource` | 2 of 3 sites fixed; one blocked (below) |

## The repair pattern (from `MoveSource`, applies to all)

1. `case u64 k =>` becomes `case right.int i =>` — `Value.u64` is no longer a
   constructor, it is an abbrev for `.int ↑n`.
2. Values arrive as `Int`; recover the `Nat` view by deriving the **lower
   bound** from `TypedLocals` via `isValid_uint_iff`.
3. Checked arithmetic guards on the unbounded `Int`, so range facts must be
   supplied in `Int` form or the `if` never reduces.  **Match the goal's
   spelling** — `(U64_SIZE : Int)` versus the literal `18446744073709551616`
   matters, and differs between files (forcing `U64_SIZE = 2 ^ 64` open can
   blow the kernel).
4. `Value.u64` is an abbrev simp will not unfold, so `.u64 (k-1)` (truncated
   `Nat`) does not match `.int (↑k - 1)` (`Int`).  Put `Value.u64` in the simp
   set together with
   `have hcast : ((k - 1 : Nat) : Int) = (k : Int) - 1 := by omega`.

Also: a reported "simp failed" line number may point at an `exact_mod_cast`
(norm_cast runs simp), not the tactic you expect.  `omega` replaces
`exact_mod_cast` robustly across the cast.

## The one real blocker

`ElimSource.lean`, second `iterate 9` (the alias write-back cascade): its
**first** step exceeds elaborator recursion.  `maxRecDepth` at 4 000, 20 000,
100 000 and 1 000 000 makes no difference — the terms are materially larger
under the unified model and the step-by-step `simp` style no longer scales.
This needs the proof restructured into per-phase lemmas so each proof term
stays small; it is not a patch.

## Suggested order

1. Finish `MasmSource` (the pattern applies).
2. Restructure `ElimSource`.
3. Move all eight files into `MoveModel/Tests/`, update
   `move-model/lakefile.toml` and `MoveModel/Tests.lean`, drop the `Examples`
   library.
4. Re-verify the four suites above.

`MasmSource` and `ElimSource` are currently mid-repair in the working tree —
partially fixed, not building.  `git checkout --` on those two returns them to
their committed (also broken, but untouched) state.
