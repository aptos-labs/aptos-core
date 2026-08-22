# Handover — Leaner Move

Branch `wrwg/leaner`, worktree `dev1`.  Base commit `84ac57b862`; the test
migration below is uncommitted.

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
cd move-model  && lake test    # 53 jobs, no warnings
cargo test -p move-compiler-v2-transactional-tests -- leaner   # 104
cargo test -p move-compiler-v2 --lib leaner                    # 5
```

## IR-level verification examples (done)

`MoveModel.Examples` no longer exists.  Its eight files — all 26 IR-level
verification theorems — live in `move-model/MoveModel/Tests/Prover/`
(`Account`, `Adequacy`, `BorrowAccount`, `CountDown`, `CrossCall`,
`ElimSource`, `MasmSource`, `MoveSource`), namespaced `Tests.Prover.*`, and
are part of `lake test`; the `Examples` `lean_lib` is gone from
`move-model/lakefile.toml`, and the library root `MoveModel.lean` no longer
imports examples.  Being in the suite is what stops the rot: they broke
unnoticed under commit `5636dfc1e0` (integer unification) precisely because
`Examples` was in neither `defaultTargets` nor `lake test`.

The repairs needed for the unified integer model:

* `Value.u64 n` is an abbreviation for `.int ↑n`, so `case u64 k` becomes
  `case right.int i`; checked arithmetic guards on the unbounded `Int`, so
  range facts must be stated in `Int` **in the goal's spelling**
  (`(U64_SIZE : Int)`, not the literal) or the guard `if` never reduces.
* `ElimSource.bump_verified` hit `(kernel) deep recursion detected`.  The
  cause was not term size or `maxRecDepth` (neither that, `--tstack`, nor
  `ulimit -s` change anything): obtaining the argument as `Value.u64 n` via
  `isValid_u64_iff` leaves the abbreviation inside every symbolic-execution
  term, and each rfl-style unfold simp performs (`Oper.sem`, `NumType.checked`,
  `u64_size`) makes the kernel re-derive the definitional step through that
  abbreviation.  The proof now destructures with `isValid_uint_iff`
  (`Value.int i`, bounds in `Int`), which is the native view of the unified
  model; the stepping structure is unchanged and elaborates in ~2 s.
* The per-step stepping proofs keep one uniform simp list per step under
  `linter.unusedSimpArgs false` (as `Account`/`BorrowAccount`/`CrossCall`
  already did); the remaining unused-argument warnings in the moved files and
  two in the library (`ValueTyping`, `Sim`) were removed, so `lake test` is
  warning-free.

Nothing is open.
