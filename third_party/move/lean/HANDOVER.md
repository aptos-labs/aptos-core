# Handover — Leaner Move

Branch `wrwg/leaner`, worktree `dev1`.  Last commit `5c26924660` (IR
verification examples moved into the `move-model` test suite); everything
below it is **uncommitted**.

## Layout (done, verified)

Two Lake packages, flat at `third_party/move/lean/`:

```
move/        package "move"       -> library Move       (requires move-model)
move-model/  package "move-model" -> library MoveModel
scripts/  design/  README.md
```

Naming follows the Lean convention: packages and executables lowercase,
libraries and module components PascalCase.  Tests are split per library —
`Move.Tests.*` and `MoveModel.Tests.*` — with aggregate roots
`move/Move/Tests.lean` and `move-model/MoveModel/Tests.lean` (Lake needs a
root module per test library; `globs` does not work).  The shared helper is
`MoveModel.Tests.Common`.

Verify (all with `ulimit -n 1048576`):

```bash
cd move        && lake test    # 85 jobs
cd move-model  && lake test    # 53 jobs
cargo test -p move-compiler-v2-transactional-tests -- leaner
cargo test -p move-compiler-v2 --lib leaner
```

## Source-verification roadmap (this round, uncommitted)

`Move/project-plan.md` now lists as implemented — and `Move/leaner-move.md`
documents — what was the roadmap: direct global borrows and borrows chained
through references; callees with a `&mut` parameter (incl. recursive); pure
callees without a `spec` (semantics generated on demand, persisted across
modules); effects hoisted out of value positions; the explicit core primitives
desugared to their surface forms; dependent/pattern `if`, `if`/`else` with a
continuation, `match` statements, `return` in loops; and **generic global
storage** (`existsAt (Vault T) a`, `&mut (Vault U64)[a].f`, spec forms
`existsAt<Vault T>(a)`, `(Vault T)[a].f`, `modifies (Vault T)[a]`).

Design points worth knowing (all in `move/Move/Verify/Syntax.lean`):

* Resource families are `Family {term, head, key, concrete}`.  Store binders
  are **per head**, universally quantified for a generic head
  (`[∀ {T : Type}, ResourceStore S (Vault T)]`), so a callee's `Vault T` at
  the callee's own `T` needs no syntactic instantiation at the caller;
  independence is assumed only between distinct heads; frames range over the
  concrete instantiations the body and the spec clauses name
  (`addMentionedFamilies`).
* A contract applies `f.sourceSpec` to the spec's type parameters **by name**
  (`applyTypeParameters`, one application node — nested named-argument
  applications fail) so a type parameter no argument determines
  (`has_generic {T}`) is instantiated; call sites pass source named type
  arguments (`has_generic (T := U64) a`) through.
* The `module` elaborator (`move/Move/Compiler/Export.lean`) elaborates items
  one by one and flushes messages/info trees/snapshot tasks per item, so
  `#guard_msgs` inside a `module` works.
* `baseIdent` (`move/Move/Syntax.lean`) gives the base of a dotted place
  `counter.value` an identifier with its original source span: the
  unused-variable linter counts only original syntax as a use.

New tests: `Move/Tests/{GlobalBorrows,Callees,ControlForms,CorePrimitives,
GenericStorage}.lean` (registered in `Move/Tests.lean`); a new rejection
fixture in `Move/Tests/LowLevel/Rejections.lean` (a clause naming a family
the function does not touch).

`Move/project-plan.md` also gained a **Move language coverage** section —
the Move-book features Leaner source cannot express at all (function values
and lambdas, inline/native functions, tuples, `for`, `assert!`, positional
structs and `..`/struct patterns, `match` guards and literal/range patterns,
address/byte-string literals, most `vector` operations, MSL) — with its own
priority list (function values first).  While surveying it, `<=`, Bool-valued
comparisons (`a < b` as a value), `>`, `>=` turned out not to compile
(`Decidable.decide` / `MoveInt.instDecidableLe` were unrecognized in
`Move/Compiler/Normalize.lean`) although the reference claimed them; both are
now lowered (`decide` is the identity on the Boolean the comparison produced),
and the translator maps `>`/`>=`/`!=` to the sealed markers like `<`/`==`
(`Move/Tests/Arithmetic.lean` covers the spellings).

The remaining verification-roadmap items were completed afterward: mutually
recursive SCCs and their contract families, two simultaneous mutable-reference
parameters, concrete generic-family global invariants, and automatic reuse of
a verified recursive callee's contract. See `Move/project-plan.md` for the
current boundaries.

## IR-level verification examples (done, committed)

`MoveModel.Examples` no longer exists; its eight files live in
`move-model/MoveModel/Tests/Prover/` and are part of `lake test`.
