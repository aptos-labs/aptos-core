# Leaner structured loops

Status: implemented; open items are listed at the end

This document specifies in-function `loop` / `while` / `break` / `continue` /
`return` for Leaner Move. The surface is designed to host compiler-v2 model
AST directly, so a later AST import can pretty-print rather than extract
helpers at the language boundary.

The two consumers of a `fun` body treat loops differently:

- **Compilation** (Leaner → LIR → IR) keeps the loop idiom the author wrote.
  One source function stays one IR function. `while` and `loop` become
  natural loops in that CFG (header, back edge, exit), not calls to
  generated helpers.
- **Verification** translates each loop into a nested `Spec.fix` fixed point
  directly inside the generated `f.sourceSpec`; no helper declarations are
  created. `satisfies_fix` / `satisfies_fix_of_wp` supply the induction
  rules. Nothing loop-related is `@[move_fun]` or appears in XIR.

`continue f args...` is a separate, explicit tail-call-to-loop form. It means
"this function *is* the loop" and jumps to that function's `entry`.
Structured `loop` / `while` may occur in the same module, but `continue f`
inside a structured loop body is an error.

Related documents: [`leaner-move.md`](leaner-move.md),
[`verification-design.md`](verification-design.md),
[`design-plan.md`](design-plan.md).

## Requirements

- Accept Move-shaped loops in authored Leaner, including `let mut` updates.
- Preserve those idioms in the compiled CFG: the author's `while` is a
  header that tests a condition; their `loop` / `break` is a header at the
  body with an exit edge; nested and sequential loops are nested / adjacent
  natural loops of the same `FunDecl`.
- Translate loops to fixed points only for `spec` / `verify`.
- Reject illegal control flow with a positioned diagnostic.
- Leave a 1-1 pretty-print target for a future model-AST import
  (`Loop` / `LoopCont` / `Return`).

Deliberately out of scope: `for` syntax (compiler-v2 desugars `for` to
`Loop`), reinterpreting Lean's `for ... in` `break`/`continue`, encoding loop
bodies as Move-level closures, and emitting loop helpers as Move functions to
"share" one representation between compile and verify.

## Two paths

```text
authored `fun` body  (while / loop / break / continue / return)
       |
       +-- move_source retains this surface syntax
       |         |
       |         v
       |   verification translator  -->  nested Spec.fix in f.sourceSpec
       |         |
       |         v
       |   `verify f`  (no per-loop declarations exist)
       |
       +-- Lean `do` elaborates to first-order loop markers
                 |
                 v
           LCNF --> Normalize  -->  one FunDecl, internal loop headers
                 |
                 v
           LIR / IR / XIR     (no extra functions)
```

The `fun` command does **not** rewrite the body before retention:
`move_source` stores the authored `while` / `loop`, and the verification
translator consumes that copy. The compiler path never sees the fixed-point
form; the verifier never sees the markers.

## Syntax

All loop forms are `do` elements of `fun` bodies (pure functions that need
mutation also use `do`; the macro wraps them in `Id.run`):

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
`loop` + `if` + `break`: compilation keeps the test on the header (see
below), while verification desugars it inside the generated relation.

### `continue` disambiguation

| Form | Meaning |
|---|---|
| `continue f args...` | Tail self-call (term); lowers the *function* to a loop. |
| `continue` | Continue the innermost structured loop. |
| `continue@name` | Continue the named enclosing loop. |
| `break` / `break@name` | Exit the innermost / named loop. |
| `return e` | Exit the enclosing `fun`. |

A label uses `@ident`, keeping it distinct from `continue f args...` and
avoiding Lean's quote syntax. Active loop labels must be unique.
`continue f args...` inside a structured loop is an error.

### `return`

`return e` is a `do` element whose type is the function result. Compilation
lowers it to `Term.ret` from the current block, including from inside a loop
— a function exit in the CFG, not a loop outcome. Verification of `return`
inside a loop needs an extra outcome in the fixed point and is currently
rejected at `spec` generation; `abort code` already escapes and is unchanged.

## Compilation: keep the idiom

### First-order markers

The `do` elaborator expands `while` / `loop` / `break` / `continue` into
opaque markers in the *same* `do` sequence. Bodies are inlined; nothing is a
Move-level closure. The markers live in `Move/Syntax.lean` and are private,
so authored Leaner cannot forge CFG control:

```lean
loopEnter        (label nonce arity token : Nat) (state : σ) : Nat
loopContinue     (label nonce arity token : Nat) (state : σ) : Nat
loopContinueTail (label nonce arity token : Nat) (state : σ) : Nat
loopBreak        (label nonce arity token : Nat) (state : σ) : Nat
loopTokenLive    (token : Nat) : Bool
loopTokenJoin    (token outerToken : Nat) : Nat
```

`state` is the nested product of the loop's reassigned entry bindings;
`arity` lets Normalize flatten it without treating the product as a Move
runtime value. The token makes every marker result data-dependent on the
loop's syntactic exit, preventing LCNF dead-let elimination from discarding
markers before Normalize sees them; `loopTokenJoin` threads outer-loop tokens
through an inner tail continue so LCNF cannot sink an outer jump marker past
an inner back edge. Each marker occurrence carries a `nonce` registered per
function; Normalize rejects unregistered markers. `loopContinueTail` marks
the implicit end-of-body continue, where Normalize also recovers the exit
continuation. Numeric labels derive from stable source positions, so
elaborator retries cannot mismatch nested markers.

`while cond do body` elaborates to:

```lean
do
  let mut token := loopEnter L nonce arity 0 state
  if cond then
    body
    token := loopContinueTail L nonce arity token state
  else
    token := loopBreak L nonce arity token state
  if loopTokenLive token then skip else return Inhabited.default
```

`loop` elaborates to the same shape without the header test; `break` and
`continue` in the body become `loopBreak` / `loopContinue` assignments.
Loop-local `let` bindings are alpha-renamed first, so a loop-local name never
captures the continuation after a `break`.

### Normalize

`Normalize.walk` recognizes the markers:

| Marker | LIR |
|---|---|
| `loopEnter L ... state` | Start the header and record its canonical state locals. |
| `loopContinue[Tail] L ... state` | Copy current state to header locals and `jump L`. |
| `loopBreak L ... state` | Copy current state and jump to the unique exit block. |
| `loopTokenLive token` | Erase the liveness guard and follow its true branch. |
| `return e` | `Term.ret` (already exists for LCNF `.return`). |
| `continue f args` | Unchanged: assign parameters, `jump "entry"` of `f`. |

Nested loops get distinct labels; a `loopContinue` / `loopBreak` targets only
its label. Sequential loops are two header/exit pairs in one `FunDecl`.

`while` vs `loop` in the CFG:

- **`while`:** the header evaluates `cond` and `branch`es to body or exit;
  the back edge returns to that header.
- **`loop`:** the header is the first body instruction; `break` is the only
  exit; the back edge returns to the start of the body.

The two idioms remain distinguishable in block layout (condition at header vs
`break` in the body); `while` is not compiled by first forgetting that its
test belongs on the header. XIR `MLoop` / IR `LoopSpec` members list the
natural loop of each source `while` / `loop`. No new `FunDecl` is created.

### Why not helper extraction for IR

Extracting `f.loop0` would change the module: extra private functions, calls
instead of back edges, and a different natural-loop structure than the author
(or an imported Move AST) wrote. That fights the AST-import goal and makes
compiler correctness talk about functions the source does not contain. LIR
already has arbitrary `jump` / `branch`; the loops only needed internal
headers, not a new IR construct.

## Verification: inline fixed points

The source-specification translator (in `Move/Verify/Syntax.lean`) runs when
`spec` builds `sourceSpec`; it does not run during `lowerToIR`. It does
not extract helper declarations: each source `loop` / `while` becomes an
inline fixed point in the generated relation,

```lean
Move.Semantics.Spec.fix
  (fun _moveSpecLoop _moveSpecLoopState => translatedBody) packedState
```

where the loop state is the tuple of reassigned loop-entry bindings
(`loopAssignedIdents`), freshly alpha-renamed so loop-local `let`s cannot
capture the continuation.

| Source | Translation inside the fixed point |
|---|---|
| End of `loop` body | recursive call with the current state |
| `continue` | recursive call with the current state |
| `break` | the translated post-loop continuation |
| `while c do b` | `if c then b; recurse else continuation` |
| `return e` | rejected at `spec` generation |

The translator keeps a stack of loop frames. `continue@name` invokes the
selected frame's recursive `Spec.fix` argument with the current state, while
`break@name` selects that frame's translated continuation. Nested labels thus
remain visible as ordinary nested fixed points and direct outer-frame calls,
without introducing Move functions.

`verify f` names only `f`; there are no per-loop declarations to name. The
`contract_intro` tactic opens a loop-shaped `sourceSpec` with
`satisfies_fix_of_wp`, exposing `recursive` and `recursiveVerified` for the
induction step.

### Proof ergonomics

Loop proofs use `Move.Verify.satisfies_fix_of_wp` when the induction step is
naturally a single weakest-precondition goal, and
`Move.Verify.wp_of_satisfies` to discharge recursive calls; this avoids
duplicating normal-return and abort forwarding. For countdown idioms,
`Move.Semantics.Checked.subSpec_one_eq_pure_of_pos` and
`Move.U64.eq_zero_of_not_pos` expose the two arithmetic facts needed by the
step; `move_cases` performs the guard split. These are deliberately explicit
helpers rather than global simp rules: unrestricted `Spec.bind` or
fixed-point unfolding makes larger Move proofs unstable and can erase
checked-arithmetic obligations.

There is deliberately no second loop constructor in `Semantics.Spec`: the
inline `Spec.fix` *is* the verification semantics.

## Worked example

Authored (retained in `move_source`, compiled as one function):

```lean
fun count_down (n : U64) : U64 := do
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

No `countDown.loop0` exists in the module. The compile contract is: a back
edge to a header that tests `0 < n`, and no extra `FunDecl` for the loop
(the prologue `let mut n := n` may sit in an entry block that jumps to the
header).

**Verification translation** (inline in `countDown.sourceSpec`):

```lean
Move.Semantics.Spec.fix
  (fun recursive n =>
    if Move.Verify.Source.logicalLT 0 n then
      Move.Semantics.Spec.bind (Move.Semantics.Checked.subSpec n 1) recursive
    else
      Move.Semantics.Spec.pure n) n
```

Nested `while` compiles to two natural loops in one CFG (inner header inside
the outer member set) and verifies as two nested fixed points, the outer body
containing the inner.

## Diagnostics

| Situation | Error |
|---|---|
| `break` / bare `continue` outside a loop | Lean's own `` `break`/`continue` must be nested inside a loop `` |
| `continue f args` inside a structured loop | `` `continue f` cannot appear inside `loop` / `while`; use `continue` `` |
| `continue f` not a tail self-call | `` `continue` must mark a direct self-call in tail position `` |
| `break@n` / `continue@n` with no active `loop@n` | `` unknown loop label `n` `` |
| nested `loop@n` while `n` is active | `` duplicate active loop label `n` `` |
| hand-written loop marker | `` unregistered compiler loop marker `` |
| `break` / `continue` targeting a Lean `for` | left to Lean |

`return` inside a loop compiles and verifies: the loop's fixed point exits
with the returned value (`ControlForms.return_in_loop`).

## Implementation map

| Piece | Where |
|---|---|
| `while`, `loop@n`, `break@n`, `continue@n` parsers and `do` elaborators | `Move/Syntax.lean` |
| Marker definitions and per-function nonce registration | `Move/Syntax.lean` |
| Internal headers, back edges, exit blocks | `Move/Compiler/Normalize.lean` |
| Retained authored surface (`move_source`, no rewrite) | `Move/Compiler/Export.lean` |
| Verification loop translation (inline `Spec.fix`) | `Move/Verify/Syntax.lean` (`translateDo`) |
| Positive tests, CFG-shape checks, and proofs | [`Tests/Language/Loops.lean`](Tests/Language/Loops.lean) |
| Rejection fixtures | [`Tests/Negative/Surface.lean`](Tests/Negative/Surface.lean) |

`Tests/Language/Loops.lean` pins the compile contract with CFG-shape checks: no
generated per-loop `FunDecl`, back edges to headers rather than calls, the
`while` test on the header, two natural loops for sequential and nested
loops, and a `ret` from inside the loop for `return`.

## Settled decisions

- Compilation preserves source loop idioms in a single function CFG.
- Verification translates loops to inline nested `Spec.fix` fixed points in
  the generated `sourceSpec`; no helper declarations are created.
- `move_source` retains authored `while` / `loop`.
- Loop bodies are inlined via first-order markers, not lambdas.
- `while` keeps its test on the CFG header; `loop` does not pretend to be
  `while`.
- `continue f args...` is unchanged and is not used to compile `while`.
- Labels compile to direct named-header / named-exit jumps; verification
  keeps nested `Spec.fix` frames and resolves labeled control to the selected
  frame.
- In-loop `return` compiles; its verification outcome is deferred.
- Lean `for` `break` / `continue` are not reinterpreted.

## Open items

- Verification of `return` inside a loop via an explicit loop outcome.
- Loop-invariant annotations for both the verification fixed points and the
  compiled CFG loop metadata.
- `for` sugar.
- Model-AST import pretty-printing into this surface.
- Whether XIR should record a loop kind (`while` vs `loop`) in `MLoop`, or
  only header / members / back edges.
- How much prologue (`let mut n := n`) sits before the while-header versus
  being merged into it.
