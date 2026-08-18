# Leaner structured loops

Status: design for implementation

This document specifies in-function `loop` / `while` / `break` / `continue` /
`return` for Leaner Move. The goal is to host compiler-v2 model AST directly,
so a later AST import can pretty-print rather than extract helpers at the
language boundary.

The two consumers of a `fun` body are deliberately different:

- **Compilation** (Leaner → LIR → IR) keeps the loop idiom the author wrote.
  One source function stays one IR function. `while` and `loop` become
  natural loops in that CFG (header, back edge, exit), not calls to
  generated helpers.
- **Verification** elaborates loops away into Lean-only helper functions
  and reuses `Spec.fix` / `satisfies_fix`. Those helpers are never
  `@[move_fun]` and never appear in XIR.

`continue f args...` remains a separate, explicit tail-call-to-loop form.
It still means “this function *is* the loop” and still jumps to that
function’s `entry`. Structured `loop` / `while` may occur in the same
module, but not mixed with `continue f` inside the same loop body.

Related documents: [`README.md`](README.md),
[`verification-design.md`](verification-design.md),
[`design-plan.md`](design-plan.md).

## Goals

- Accept Move-shaped loops in authored Leaner, including `let mut` updates.
- Preserve those idioms in the compiled CFG: the author’s `while` is a
  header that tests a condition; their `loop` / `break` is a header at the
  body with an exit edge; nested and sequential loops are nested / adjacent
  natural loops of the same `FunDecl`.
- Elaborate loops to helper functions **only** for `spec` / `verify`.
- Reject illegal control flow with a positioned diagnostic.
- Leave a 1-1 pretty-print target for a future model-AST import
  (`Loop` / `LoopCont` / `Return`).

## Non-goals (this milestone)

- User-facing loop invariants. Verification helpers may later carry
  invariants; compilation does not invent them.
- `for` syntax. Compiler-v2 has already desugared `for` to `Loop`.
- Changing `continue f args...` to allow a non-self or non-tail call.
- Interpreting `loop` / `break` as Lean terms that `simp [f]` can reduce
  without the verification elaborator.
- Emitting generated helpers as Move functions to “share” one
  representation between compile and verify.

## Two paths

```text
authored `fun` body  (while / loop / break / continue / return)
       |
       +-- move_source retains this surface syntax
       |         |
       |         v
       |   verification elaborator  -->  Lean-only helpers + Spec.fix
       |         |
       |         v
       |   `verify f`  (author never names f.loop0)
       |
       +-- Lean `def` elaborates to first-order loop markers
                 |
                 v
           LCNF --> Normalize  -->  one FunDecl, internal loop headers
                 |
                 v
           LIR / IR / XIR     (no extra functions)
```

The `fun` command does **not** rewrite the body to helpers before
retention. `move_source` stores the authored `while` / `loop`.

## Constraints from the current prototype

1. **`let mut` is a `do` element.** Action functions already use `do`. Pure
   functions that need mutation are written with `do` as well.
2. **Source verification walks retained syntax** (`move_source`). That
   syntax still contains `while` / `loop`. The verification elaborator
   consumes a copy and produces helpers; the compiler never sees them.
3. **Normalize rejects ordinary local functions and closures.** Loop
   lowering must not encode the body as a Lean lambda (`whileMarker cond
   (fun _ => body)`). The `do` elaborator inlines the body and emits
   first-order markers in the same statement sequence.
4. **`continue f` jumps to that function’s `entry`.** That rule stays for
   the tail-call form. Structured loops add *internal* headers. Normalize
   must emit `jump` to those headers, not only to `"entry"`.
5. **Lean `do` already has `break` / `continue` for `for ... in`.** Leaner
   loop `break` / `continue` apply only inside a Leaner `loop` / `while`.
   `continue f args...` stays a term.

## Syntax

All new forms are scoped with the existing Move syntax. They are legal in
`do` blocks of `fun` declarations. Term-level `while` / `loop` (outside
`do`) is sugar for a `do` block whose result is the value after the loop.

```lean
loop
  ...
  if c then break
  if d then continue

while cond do
  ...

loop@outer
  loop@inner
    if c then break@outer
    if d then continue@inner

return e
```

`while cond do body` is the `while` idiom, not merely pretty-printed
`loop` + `if` + `break`. Compilation must keep the test on the header
(see below). Verification may desugar it to `loop` + `if` + `break`
before extracting helpers.

### `continue` disambiguation

| Form | Meaning |
|---|---|
| `continue f args...` | Existing tail self-call. Unchanged parser `continueCallTerm`. |
| `continue` | Innermost Leaner loop. New `do` element. |
| `continue@name` | Continue the named Leaner loop. |
| `break` / `break@name` | Exit the innermost / named loop. |
| `return e` | Exit the enclosing `fun`. |

A label uses `@ident`; this keeps it distinct from `continue f args...` and
avoids reserving Lean's quote syntax.

`continue f args...` inside a Leaner `loop` / `while` is an error.

### `return`

`return e` is a `do` element. Its type is the function result (`T` in
`Action T`, or the pure result). Compilation lowers it to `Term.ret` from
the current block, including from inside a loop — that is a function exit
in the CFG, not a loop-helper outcome.

Verification of `return` inside a loop needs an outcome in the helper
(`done` vs `ret`). **v1 compiles `return` everywhere, but `verify`
rejects `return` inside a loop** until v1.1 adds that outcome.

`abort code` is unchanged and already escapes.

## Compilation: keep the idiom

### First-order markers

The `do` elaborator expands `while` / `loop` / `break` / `continue` into
opaque markers in the *same* `do` sequence. Bodies are inlined. Nothing
is a Move-level closure.

```lean
loopEnter        (label arity token : Nat) (state : σ) : Nat
loopContinue     (label arity token : Nat) (state : σ) : Nat
loopContinueTail (label arity token : Nat) (state : σ) : Nat
loopBreak        (label arity token : Nat) (state : σ) : Nat
loopTokenLive    (token : Nat) : Bool
```

`state` is the nested product of mutable locals assigned by the loop.
`arity` lets Normalize flatten it without treating the product as a Move
runtime value. The token makes every marker result data-dependent on the
loop's syntactic exit, preventing pure LCNF dead-let elimination from
discarding markers before Normalize sees them. `loopContinueTail` marks the
implicit end-of-body continue, where Normalize also recovers the exit
continuation.

`while cond do body` becomes:

```lean
do
  let mut token := loopEnter 0 arity 0 state
  if cond then
    body
    token := loopContinueTail 0 arity token state
  else
    token := loopBreak 0 arity token state
  if loopTokenLive token then skip else unreachable
```

`loop` / `break` / `continue` become:

```lean
do
  let mut token := loopEnter 0 arity 0 state
  -- body statements, inlined
  -- `break`    => token := loopBreak 0 arity token state
  -- `continue` => token := loopContinue 0 arity token state
  token := loopContinueTail 0 arity token state
  if loopTokenLive token then skip else unreachable
```

`loopContinue` / `loopBreak` are non-returning in the Leaner subset, like
`abort`: they close the current LCNF block. Normalize treats them as
terminators, not calls.

Labels map to the numeric marker of the named active loop. Numeric labels are
derived from stable source positions, so elaborator retries cannot mismatch
nested markers. Inner loop entry/tail markers carry data dependencies on outer
tokens; this prevents pure LCNF sinking an outer jump marker past an inner
back edge.

### Normalize

Extend `walk` to recognize the markers:

| Marker | LIR |
|---|---|
| `loopEnter L ... state` | Start the header and record its canonical state locals. |
| `loopContinue[ Tail] L ... state` | Copy current state to header locals and `jump L`. |
| `loopBreak L ... state` | Copy current state and jump to the unique exit block. |
| `loopTokenLive token` | Erase the liveness guard and follow its true branch. |
| `return e` | `Term.ret` (already exists for LCNF `.return`). |
| `continue f args` | Unchanged: assign params, `jump "entry"` of `f`. |

`loopContinueTail` exposes the continuation after the loop as its exit block.
Nested loops get distinct labels; a `loopContinue` / `loopBreak` targets only
its label. Sequential loops are two header/exit pairs in one `FunDecl`.

`while` vs `loop` in the CFG:

- **`while`:** header evaluates `cond` and `branch`es to body or exit.
  The back edge returns to that header. This is the CountDown shape
  already used in `MoveModel.Examples.CountDown`.
- **`loop`:** header is the first body instruction. `break` is the only
  exit. The back edge returns to the start of the body.

Do not compile `while` by first turning it into `loop { if cond then body
else break }` and then forgetting that the test belongs on the header.
The two idioms must remain distinguishable in block layout (condition at
header vs `break` in the body).

XIR `MLoop` / IR `LoopSpec` members should list the natural loop of each
source `while` / `loop`. No new `FunDecl` is created.

### Why not helper-extract for IR

Extracting `f.loop0` would change the module: extra private functions,
calls instead of back edges, and a different natural-loop structure than
the author (or an imported Move AST) wrote. That fights the later
AST-import goal and makes compiler-correctness talk about functions the
source does not contain.

LIR already has arbitrary `jump` / `branch`. The missing piece is
internal headers, not a new IR construct.

## Verification: elaborate to helpers

A syntax-to-syntax pass (`Move/Verify/Loops.lean`, or a verification-only
entry in `Move/Compiler/Loops.lean`) runs when `spec` / `verify` builds
`sourceSpec`. It does not run during `move_module%`.

Each source `loop` / `while` becomes a Lean-only helper:

```lean
partial def f.loopN (xs...) : R :=
  ...
def f.sourceSpec :=
  -- wrapper uses f.loopN.sourceSpec / Spec.fix
```

These declarations have no `move_fun`. `move_module%` must not discover
them.

Parameters `xs` are the variables assigned in the loop and live at the
header or after the exit. `while` may be desugared to `loop` + `if` +
`break` *in this pass only*.

| Source | Verification helper |
|---|---|
| End of `loop` body | `continue f.loopN xs` |
| `continue` | `continue f.loopN xs` |
| `break` | return `xs` |
| `while c do b` | `if c then b; continue f.loopN xs else xs` |
| `return e` (v1) | rejected |
| `return e` (v1.1) | `LoopOut.ret e`; wrapper matches |

Verification retains a stack of loop frames. `continue@name` invokes the
selected frame's recursive `Spec.fix` argument with the current state, while
`break@name` selects that frame's translated continuation. Thus nested labels
remain visible as ordinary nested fixed points and direct outer-frame calls,
without introducing Move functions.

`verify f` names only `f`. The elaborator installs `f.loopN` as an
internal `sourceSpec` dependency. Authors never write `verify f.loop0`.

### Proof ergonomics

Loop proofs should use `Move.Verify.satisfies_fix_of_wp` when the induction
step is naturally a single weakest-precondition goal, and
`Move.Verify.wp_of_satisfies` to discharge recursive calls. This avoids
duplicating normal-return and abort forwarding. For countdown idioms,
`Move.Semantics.Checked.subSpec_one_eq_pure_of_pos` and
`Move.U64.eq_zero_of_not_pos` expose the two arithmetic facts needed by the
step. These are deliberately explicit helpers rather than global simp rules:
unrestricted `Spec.bind` or fixed-point unfolding makes larger Move proofs
unstable and can erase checked-arithmetic obligations.

Do not add a second loop constructor to `Semantics.Spec`. The helper *is*
the verification semantics.

## Worked example

Authored (retained in `move_source`, compiled as one function):

```lean
fun countDown (n : U64) : U64 := do
  let mut n := n
  while 0 < n do
    n := n - 1
  n
```

**IR** (same function, while-idiom CFG):

```
B0 (header):  t := (0 < n); branch t B1 B2
B1 (body):    n := n - 1;   jump B0
B2 (exit):    ret n
```

No `countDown.loop0` in the module. `hasEntryBackEdge countDown` may be
false if the header is not block 0 (prologue `let mut n := n` can sit in
an entry block that jumps to the header). The test is: `countDown` has a
back edge to its while-header, and `compiled.funs` has no extra function
for this loop.

**Verification elaborator** (Lean-only, not compiled):

```lean
partial def countDown.loop0 (n : U64) : U64 :=
  if 0 < n then continue countDown.loop0 (n - 1) else n
```

`countDown.sourceSpec` calls that helper’s `Spec.fix`.

Nested `while` compiles to two natural loops in `nested`’s CFG (inner
header inside the outer member set). Verification extracts two Lean-only
helpers, the outer calling the inner.

## Diagnostics

| Situation | Error |
|---|---|
| `break` / bare `continue` outside a Leaner loop | `` `break` / `continue` requires an enclosing `loop` or `while` `` |
| `continue f args` inside a Leaner loop | `` `continue f` cannot appear inside `loop` / `while`; use `continue` `` |
| `continue f` not a tail self-call (existing) | unchanged |
| `return` inside a loop at `verify` (v1) | `` `return` inside `loop` / `while` is not yet supported for `verify` `` |
| `break@n` / `continue@n` with no active `loop@n` | `` unknown loop label `n` `` |
| nested `loop@n` while `n` is active | `` duplicate active loop label `n` `` |
| `break` / `continue` targeting a Lean `for` | leave to Lean |
| `loopContinue` / `loopBreak` not in tail position of the current `do` branch | `` `break` / `continue` must be in tail position of the current branch `` |

Compilation of `return` inside a loop is allowed in v1 even when `verify`
is not.

## Implementation sketch

| Piece | Where |
|---|---|
| Parsers for `while`, `loop`, `loop@n`, `break`, `break@n`, `continue`, `continue@n`, `return` | `Move/Syntax.lean` (`doElem` + term sugar) |
| `do` elaborator: inline body, emit `loopEnter` / `loopContinue` / `loopBreak` | `Move/Syntax.lean` or `Move/Compiler/LoopMarkers.lean` |
| Marker definitions | `Move/Basic.lean` (next to `continueMarker`) |
| Normalize: internal headers, back edges, exit blocks | `Move/Compiler/Normalize.lean` |
| Retain authored surface in `move_source` | `Move/Compiler/Export.lean` (no rewrite) |
| Verification helper extraction | `Move/Verify/Loops.lean`, invoked from `Move/Verify/Syntax.lean` |
| Tests | `Tests/Move/Loops.lean`, `Tests/Move/LowLevel/Rejections.lean` |

The verification pass is syntax-to-syntax and does not depend on LCNF. A
later AST import can pretty-print into the same surface; compilation and
`verify` then apply unchanged.

## Milestones

**v1 — implement now**

- Unlabeled `loop`, `while`, `break`, `continue` in `do`.
- Term-level `while` / `loop` desugaring into `do` for pure `fun`.
- Marker elaboration + Normalize internal headers.
- One IR function per source function; interpreter tests on CFG shape.
- `return` compiled everywhere; `verify` only if it is outside a loop.
- Verification helper extraction for `while` / `loop` / `break` /
  `continue` without `return`-in-loop.

**v1.1 — labels implemented**

- Labels (`LoopCont` nest > 0): `jump` to the named header / exit.

**v1.2**

- `verify` of `return` inside a loop via a Lean-only outcome.

**Later**

- `for` sugar.
- Loop invariants on verification helpers.
- Model-AST import pretty-printing into this surface.

## Test cases

Add `Tests/Move/Loops.lean` as an ordinary `move_module` example, and
rejection fixtures next to the existing `continue` diagnostics.

### Accepted: compile and interpret

```lean
move_module Loops where

  fun countDown (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    n

  fun countDownLoop (n : U64) : U64 := do
    let mut n := n
    loop
      if n < 1 then break
      n := n - 1
    n

  fun skipEvens (n acc : U64) : U64 := do
    let mut n := n
    let mut acc := acc
    while 0 < n do
      n := n - 1
      if n % 2 == 0 then continue
      acc := acc + 1
    acc

  fun twoPhases (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    while n < 3 do
      n := n + 1
    n

  fun nested (x : U64) : U64 := do
    let mut x := x
    while 0 < x do
      while 10 < x do
        x := x - 10
      x := x - 1
    x

  @[move_struct]
  structure Counter where
    value : U64
    deriving Key

  fun drain (addr : Address) : Action U64 := do
    let value ← &mut Counter[addr].value
    let mut n ← *value
    while 0 < n do
      n := n - 1
    value := n
    pure n

  fun early (flag : Bool) : U64 := do
    if flag then return 7
    8

  -- compiled in v1; verify deferred
  fun returnInLoop (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      if n == 3 then return 1
      n := n - 1
    n

  partial fun countdownTail (value accumulator : U64) : U64 :=
    if value < 1 then accumulator
    else continue countdownTail (value - 1) (accumulator + 1)
```

Interpreter expectations (`Tests.run`, same style as
`Tests/Move/Calls.lean`):

| Call | Result |
|---|---|
| `countDown [5]` | `0` |
| `countDown [0]` | `0` |
| `countDownLoop [5]` | `0` |
| `skipEvens [5, 0]` | `2` |
| `twoPhases [2]` | `3` |
| `nested [25]` | `0` |
| `drain` with `Counter = 4` | memory `0`, result `0` |
| `early [true]` / `[false]` | `7` / `8` |
| `returnInLoop [5]` | `1` |
| `returnInLoop [2]` | `0` |
| `countdownTail [100, 40]` | `140` |

CFG shape (this is the v1 compile contract):

- The module has **no** `countDown.loop0` (or similar) `FunDecl`.
- `countDown` contains a back edge to a header that tests `0 < n`, not a
  `function` call.
- `countDownLoop` contains a back edge to a body header; the `n < 1`
  test is a `break` edge to the exit, not the while-header shape.
- `twoPhases` has two natural loops in **one** function.
- `nested` has an inner loop whose members are a subset of the outer.
- `returnInLoop` has a `ret` from a block inside the loop.
- `countdownTail` still has an entry back edge on the authored function.

### Accepted: `spec` / `verify`

```lean
spec countDown (n : U64) where
  ensures result = 0

verify countDown

spec early (flag : Bool) where
  ensures result = if flag then 7 else 8

verify early

spec drain (addr : Address) where
  requires exists<Counter>(addr);
  ensures Counter[addr].value = 0;
  aborts_if False

verify drain
```

`countDown` / `drain` go through `satisfies_fix` on the **verification**
helper. `early` does not use a helper.

`verify returnInLoop` is a v1 rejection (or omitted from the file) until
v1.1.

A first `verify` may need `by` with `satisfies_fix`, as `recursiveChoose`
does today, until automatic glue covers generated helpers. Authors still
write `verify countDown`, never `verify countDown.loop0`.

### Rejected: `#guard_msgs`

```lean
fun bareBreak (n : U64) : U64 := do
  break
  n
-- `break` requires an enclosing `loop` or `while`

fun bareContinue (n : U64) : U64 := do
  continue
  n
-- `continue` requires an enclosing `loop` or `while`

fun continueCallInside (n : U64) : U64 := do
  let mut n := n
  while 0 < n do
    continue continueCallInside (n - 1)
  n
-- `continue f` cannot appear inside `loop` / `while`; use `continue`

fun unknownLabel (n : U64) : U64 := do
  loop@outer
    break@missing
  n
-- unknown loop label `missing`
```

`return` inside a loop is **not** a compile rejection in v1.

Keep the existing non-tail / non-self `continue f` tests.

### Optional golden LIR

Snapshot `countDown` as one function: prologue, while-header `lt` +
`branch`, body `sub` + `jump` header, exit `ret`. Snapshot `countDownLoop`
with the test on a `break` edge. Do not snapshot helper function names;
helpers are not in the IR.

## Settled decisions

- Compilation preserves source loop idioms in a single function CFG.
- Verification elaborates loops to Lean-only helpers and `Spec.fix`.
- `move_source` retains authored `while` / `loop`.
- Loop bodies are inlined via first-order markers, not lambdas.
- `while` keeps its test on the CFG header; `loop` does not pretend to be
  `while`.
- `continue f args...` is unchanged and is not used to compile `while`.
- Labels compile to direct named-header / named-exit jumps; verification keeps
  nested `Spec.fix` frames and resolves labeled control to the selected frame.
- In-loop `return` still compiles and remains deferred for verification.
- Lean `for` `break` / `continue` are not reinterpreted.

## Open questions

- How much prologue (`let mut n := n`) sits before the while-header
  versus being merged into it.
- Whether XIR should record a loop kind (`while` vs `loop`) in `MLoop`,
  or only header / members / back edges.
- How automatic `verify` should bind helper `sourceSpec` so the author
  never mentions the helper.
