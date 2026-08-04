-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.Syntax
import Move.Prover.Ivl.Semantics
import Move.Prover.Ivl.Wp
import Move.IR.Value
import Move.IR.State
import Move.IR.Spec
import Move.IR.Contract
import Move.IR.Syntax
import Move.IR.Semantics

/-!
# Translation: IR CFG → Boogie-Style IVL

The formalization of the Move Prover's *specification injection* (TACAS'22
§3.2 and Appendix A, `SpecInstrumentationProcessor`): compile a bytecode
function together with its contract into an IVL program whose assertion
safety implies the contract.  Deep spec expressions are *denoted* into
shallow IVL predicates here, by constructing the appropriate `SpecEnv` from
the verification state.

**Verification state.**  The IVL is instantiated at `VState`: the current
bytecode state plus verification-only components — the memory snapshots
`snaps` (the `SaveMem` store; `preLabel` is bound to the entry memory,
giving meaning to `old(..)`), the entry argument values `args`, the return
values `rets` (set by the compiled `ret`), and the abort flag `aborted`.
The abort flag is exactly the `$abort_flag`/`$abort_code` pair of the real
Boogie encoding: Move's abort control flow becomes data flow.

**Blocks map 1-1.**  Source block `b` compiles to IVL block `b + 1`; label
`0` is the entry stub (`assume requires`), labels `size + 1`/`size + 2` are
the return/abort exit blocks.  Since source block ids follow code layout
order, the identity function is the rank of the compiled program: back
edges are exactly the rank-non-increasing edges.  Within a block, every
non-call instruction compiles to one `onOk`-guarded assignment (a total
deterministic update, e.g. `$AddU64` sets the flag on overflow); the flag is
consulted once per block by the compiled terminator, which routes to the
abort exit when set — the block-level equivalent of Boogie's
`if ($abort_flag) goto L_abort`.

**Calls** (TACAS'22 Fig. 9) use the opaque schema against the callee's
contract, as two commands inside the block:

```
assert requires;
havoc { abort branch: flag := some code, aborts holds
      | normal branch: memory havoc'd within modifies, ensures ∧ frame }
```

**Exit checks** (TACAS'22 Fig. 8): the abort exit asserts the abort
condition (completeness of `aborts_if`); the return exit asserts that the
abort condition evaluates to false (tightness — the biconditional), and
`ensures` plus the `modifies` frame.  (The real prover checks `modifies`
permissions per instruction; asserting the frame at exit is its callee-side
semantic content.)

**Stuckness.**  Ill-typed situations are stuck in the source semantics; the
compiled code treats them as no-ops (or unprovable verification conditions).
Since stuck configurations have no source outcome, this is sound for the
simulation, and irrelevant for well-typed programs.
-/

namespace Move.Prover.Translate

open Move.Prover.Ivl
open Move.IR

/-- The verification state (see module docs). -/
structure VState where
  cur : MoveState
  snaps : MemLabel → Memory
  args : List Value
  rets : List Value
  aborted : Option Nat

namespace VState

/-- Raise the abort flag with `code`. -/
def doAbort (v : VState) (code : Nat) : VState :=
  { v with aborted := some code }

/-- The spec environment of pre-state contract clauses (`requires`,
`aborts`, `modifies`), reconstructed from the verification state: arguments
as locals, the entry snapshot as memory. -/
def preEnvOf (v : VState) (Δ : StructDecls) : SpecEnv :=
  preEnv Δ (v.snaps preLabel) v.args

/-- The spec environment of the `ensures` clause at exit. -/
def postEnvOf (v : VState) (Δ : StructDecls) : SpecEnv :=
  postEnv Δ (v.snaps preLabel) v.cur.memory v.args v.rets

/-- The spec environment of a loop invariant: the *current* locals and
memory, with the snapshot store available (invariants may reference
`old(..)` through it). -/
def curEnv (v : VState) (Δ : StructDecls) : SpecEnv where
  structs := Δ
  locals := v.cur.locals
  result := []
  mem := v.cur.memory
  snaps := v.snaps
  bound := []

end VState

/-- The verification state at function entry: arguments bound, every
snapshot label bound to the entry memory (the `SaveMem` of `preLabel`),
abort flag clear. -/
def initVState (m : Memory) (args : List Value) : VState where
  cur := ⟨initLocals args, m⟩
  snaps := fun _ => m
  args := args
  rets := []
  aborted := none

/-- The abort flag is set. -/
def flagSet : VState → Prop := fun v => v.aborted.isSome

/-- The abort flag is clear. -/
def flagClear : VState → Prop := fun v => v.aborted = none

/-- Guard a state update on the abort flag: once the flag is set, compiled
code has no effect (Move's abort short-circuiting as data flow). -/
def onOk (f : VState → VState) : BCmd VState :=
  .assign fun v => if v.aborted.isSome then v else f v

/-- Step one `onOk` command from a flag-clear state.  Verification-condition
proofs over long straight-line blocks step commands one at a time with this
and `wpCmds_onOk_skip` — unfolding the whole `wpCmds` nest at once produces
terms whose kernel-checking cost grows superlinearly with the block
length. -/
theorem wpCmds_onOk_step {f : VState → VState} {cs : List (BCmd VState)}
    {Q : VState → Prop} {v : VState} (h : v.aborted = none) :
    wpCmds (onOk f :: cs) Q v ↔ wpCmds cs Q (f v) := by
  simp [wpCmds, onOk, h]

/-- Step one `onOk` command from a flag-set state: a no-op. -/
theorem wpCmds_onOk_skip {f : VState → VState} {cs : List (BCmd VState)}
    {Q : VState → Prop} {v : VState} (h : v.aborted.isSome) :
    wpCmds (onOk f :: cs) Q v ↔ wpCmds cs Q v := by
  simp [wpCmds, onOk, h]

/-- The compiled `assert` of a callee's precondition at a call site: the
arguments must evaluate and satisfy `requires` in the current state. -/
def callRequires (Δ : StructDecls) (c : Contract) (srcs : List LocalIndex) :
    VState → Prop :=
  fun v => v.aborted = none →
    ∃ args, srcs.mapM v.cur.locals = some args ∧
      Holds (preEnv Δ v.cur.memory args) c.requires

/-- The havoc relation of an opaque call (TACAS'22 Fig. 9): either the
callee aborts — the flag is raised with an arbitrary code, under the
callee's abort condition — or it returns: the memory changes to an
arbitrary `mNew` within the `modifies` footprint of the callee, results are
arbitrary values satisfying `ensures`, and the abort condition evaluates to
false (biconditional `aborts_if`).  On the normal branch the results and
the new memory are additionally well-formed for the callee's declared
types, provided the pre-call memory is — the `WellFormed` assumptions the
real prover emits for call results. -/
def callRel (Δ : StructDecls) (d : FunDecl) (dsts srcs : List LocalIndex)
    (v v' : VState) : Prop :=
  if v.aborted.isSome then v' = v
  else
    ∃ args, srcs.mapM v.cur.locals = some args ∧
      ((∃ code, d.contract.abortsHolds (preEnv Δ v.cur.memory args) ∧
          v' = v.doAbort code) ∨
       (∃ mNew rets,
          rets.length = dsts.length ∧
          d.contract.abortsFalse (preEnv Δ v.cur.memory args) ∧
          Holds (postEnv Δ v.cur.memory mNew args rets) d.contract.ensures ∧
          agreesOutside (d.contract.footprint (preEnv Δ v.cur.memory args))
            v.cur.memory mNew ∧
          (TypedMemory Δ v.cur.memory →
            TypedMemory Δ mNew ∧ IsValidList Δ d.returns rets) ∧
          v' = { v with
            cur := MoveState.writeLocals ⟨v.cur.locals, mNew⟩ dsts rets }))

/-- The compilation of the (semantically unsupported) reference operations:
a failing assertion, so that their presence is a loud verification failure
rather than a silent gap. -/
def refFail : List (BCmd VState) := [.assert fun _ => False]

/-- Compile one instruction to IVL commands.  Non-call instructions become
one `onOk`-guarded total update; calls become the two-command opaque schema.
Undeclared callees compile to nothing (the source is stuck); reference
operations compile to `refFail`. -/
def compileInstr (P : Program) : Instr → List (BCmd VState)
  | .load dst val => [onOk fun v => { v with cur := v.cur.writeLocal dst val }]
  | .assign dst src =>
      [onOk fun v =>
        match v.cur.locals src with
        | some val => { v with cur := v.cur.writeLocal dst val }
        | none => v]
  | .nop => []
  | .call dsts (.function f) srcs =>
      match P.funs f with
      | some d => [.assert (callRequires P.structs d.contract srcs),
                   .havoc (callRel P.structs d dsts srcs)]
      | none => []
  | .call _ .borrowLoc _ => refFail
  | .call _ (.borrowField _) _ => refFail
  | .call _ (.borrowGlobal _) _ => refFail
  | .call _ .borrowVecElem _ => refFail
  | .call _ .readRef _ => refFail
  | .call _ .writeRef _ => refFail
  | .call _ .freezeRef _ => refFail
  | .call dsts op srcs =>
      [onOk fun v =>
        match srcs.mapM v.cur.locals with
        | some vs =>
          match op.sem v.cur.readTarget vs v.cur.memory with
          | some (.ok rets m') =>
              if dsts.length = rets.length then
                { v with
                  cur := MoveState.writeLocals ⟨v.cur.locals, m'⟩ dsts rets }
              else v
          | some .abort => v.doAbort runtimeAbortCode
          | none => v
        | none => v]

/-- The commands a terminator contributes (before its goto): `ret` stores
the returned values, `abort` raises the flag with the code. -/
def termCmds : Term → List (BCmd VState)
  | .ret srcs =>
      [onOk fun v =>
        match srcs.mapM v.cur.locals with
        | some vals => { v with rets := vals }
        | none => v]
  | .abort code =>
      [onOk fun v =>
        match v.cur.locals code with
        | some (.u64 n) => v.doAbort n
        | _ => v]
  | .jump _ => []
  | .branch _ _ _ => []

/-- The compiled goto targets of a terminator: route to the abort exit
when the flag is set, and along the (shifted) source edge otherwise. -/
def termGoto (size : Nat) : Term → List ((VState → Prop) × Label)
  | .jump b => [(flagSet, size + 2), (flagClear, b + 1)]
  | .branch c b₁ b₂ =>
      [(flagSet, size + 2),
       (fun v => v.aborted = none ∧ v.cur.locals c = some (.bool true),
         b₁ + 1),
       (fun v => v.aborted = none ∧ v.cur.locals c = some (.bool false),
         b₂ + 1)]
  | .ret _ => [(flagSet, size + 2), (flagClear, size + 1)]
  | .abort _ => [(flagSet, size + 2)]

/-- Compile a source block at id `b` (IVL label `b + 1`).  `ret`/`abort`
terminators contribute a final command (store the return values / raise the
flag); the compiled terminator routes to the abort exit when the flag is
set, and follows the source edge (shifted by one) otherwise. -/
def compileBlock (P : Program) (size : Nat) (blk : Block) : BBlock VState where
  cmds := (blk.instrs.map (compileInstr P)).flatten ++ termCmds blk.term
  term := .goto (termGoto size blk.term)

/-- The return exit block (label `size + 1`, reachable only with the flag
clear): asserts tightness of `aborts_if`, the postcondition, and the
`modifies` frame — TACAS'22 Fig. 8's normal-path exit checks. -/
def retExitBlock (Δ : StructDecls) (c : Contract) : BBlock VState where
  cmds := [.assert fun v =>
    c.abortsFalse (v.preEnvOf Δ) ∧
    Holds (v.postEnvOf Δ) c.ensures ∧
    agreesOutside (c.footprint (v.preEnvOf Δ)) (v.snaps preLabel)
      v.cur.memory]
  term := .ret

/-- The abort exit block (label `size + 2`, reachable only with the flag
set): asserts completeness of `aborts_if` — Fig. 8's abort-path check. -/
def abortExitBlock (Δ : StructDecls) (c : Contract) : BBlock VState where
  cmds := [.assert fun v => c.abortsHolds (v.preEnvOf Δ)]
  term := .ret

/-- The typing assumptions injected at function entry — the `WellFormed`
assumptions the real prover derives from the declarations: the arguments
are well-formed for the parameter types, and global memory is well-formed
for the struct declarations. -/
def typedEntry (P : Program) (d : FunDecl) : VState → Prop := fun v =>
  TypedArgs P.structs d v.args ∧ TypedMemory P.structs (v.snaps preLabel)

/-- Compile a function: entry stub (`assume WellFormed; assume requires`),
the body blocks shifted by one, and the two exit blocks. -/
def compileFun (P : Program) (d : FunDecl) : BProgram VState where
  blocks := fun l =>
    if l = 0 then
      some ⟨[.assume (typedEntry P d),
             .assume fun v => Holds (v.preEnvOf P.structs) d.contract.requires],
            .goto [(fun _ => True, d.body.entry + 1)]⟩
    else if l = d.body.size + 1 then some (retExitBlock P.structs d.contract)
    else if l = d.body.size + 2 then some (abortExitBlock P.structs d.contract)
    else (d.body.blocks (l - 1)).map (compileBlock P d.body.size)
  entry := 0

/-- Denote a source loop annotation over the verification state: the
invariant additionally pins the abort flag clear (loop headers are entered
only on the normal path); the target relation is the frame of the declared
loop targets — everything but the `valTargets` locals and the
`memTargets` locations is unchanged (snapshots and arguments are never
written by compiled code) — plus the **multisorted havoc** discipline,
stated as *preservation*: a havocked local with a declared type stays
defined and well-formed if it was (Boogie's havoc ranges over the sort),
and havocked memory contents of declared resources stay well-formed.  The
preservation form keeps the relation reflexive (`WfProgram.targetsRefl`);
at loop entry the pre-facts come from the typed boundary
(`typedEntry`). -/
def denoteLoopSpec (Δ : StructDecls) (locals : LocalIndex → Option Ty)
    (ls : LoopSpec) : LoopAnn VState where
  inv := fun v => v.aborted = none ∧ Holds (v.curEnv Δ) ls.inv
  targets := fun v v' =>
    v'.snaps = v.snaps ∧ v'.args = v.args ∧
    (∀ y, ¬ ls.valTargets y → v'.cur.locals y = v.cur.locals y) ∧
    (∀ y t, ls.valTargets y → locals y = some t →
      (∃ val, v.cur.locals y = some val ∧ IsValid Δ t val) →
      ∃ val, v'.cur.locals y = some val ∧ IsValid Δ t val) ∧
    agreesOutside ls.memTargets v.cur.memory v'.cur.memory ∧
    (∀ r sd a, Δ r = some sd → ls.memTargets ⟨r, a⟩ →
      (∀ val, v.cur.memory r a = some val → IsValid Δ (.struct r) val) →
      ∀ val, v'.cur.memory r a = some val → IsValid Δ (.struct r) val)
  members := fun l => ∃ b, l = b + 1 ∧ ls.members b

/-- The loop annotations of a compiled function, on the shifted labels. -/
def compAnns (P : Program) (d : FunDecl) : Anns VState := fun l =>
  match l with
  | 0 => none
  | b + 1 => (d.loopSpecs b).map (denoteLoopSpec P.structs d.locals)

/-- **A function verifies**: the weakest precondition of its compiled
program holds at the entry stub for every boundary state (some fuel
suffices uniformly).  The rank of the compiled program is the identity —
source block ids follow code layout order. -/
def Verified (P : Program) (f : FunId) : Prop :=
  ∃ d, P.funs f = some d ∧ ∃ fuel, ∀ (m : Memory) (args : List Value),
    wpB (compileFun P d) (compAnns P d) (fun l => l) (fun _ => True) fuel 0
      (initVState m args)

/-- Writing locals does not touch memory. -/
theorem writeLocals_memory (s : MoveState) :
    ∀ (xs : List LocalIndex) (vs : List Value),
      (MoveState.writeLocals s xs vs).memory = s.memory := by
  intro xs
  induction xs generalizing s with
  | nil => intro vs; rfl
  | cons x xs ih =>
    intro vs
    cases vs with
    | nil => rfl
    | cons v vs => exact ih (s.writeLocal x v) vs

end Move.Prover.Translate
