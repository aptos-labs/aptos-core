-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.IR.Interp.Exec
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Translate.Compile

/-!
# Borrow-Based Move Source after Reference Elimination

`moveElim%` compiles a self-contained Move module and then runs executable
interprocedural reference elimination.  The result is the transformed IR
program verified below; no hand-written eliminated declaration is needed.

The current `refElim_correct` theorem covers the summary-free transformation,
not the interprocedural pipeline used by this elaborator.  Accordingly, this
example proves `Verified` for the generated IR rather than a source-level
preservation theorem for the original Move function.

The example is the borrow-based `bump`.  Compiler v2 lowers its mutation to
a chain of reference moves and copies.  Elimination replaces that chain with
mutation values and explicit alias write-backs.

```move
fun bump(x: u64): u64 {
    let r = &mut x;
    *r = *r + 1;
    x
}
```

`bump_verified` checks the eliminated program against the genuine `spec`
block using a command-by-command stepping proof.
-/

namespace Tests.Prover.ElimSource

open MoveModel.Prover.Ivl
open MoveModel.IR
open MoveModel.Prover.Translate
open MoveModel.Frontend.XIR

/-- The borrow-based original: the reference semantics executes it (and
its reference operations would *not* verify — they compile to failing
assertions). -/
def bumpSrc : Program := move% "
module 0x42::bump {
    fun bump(x: u64): u64 {
        let r = &mut x;
        *r = *r + 1;
        x
    }
    spec bump {
        requires x < 18446744073709551615;
        aborts_if false;
        ensures result == x + 1;
    }
}
"

/-- The same module through `moveElim%`: elaborated *and* eliminated. -/
def bump : Program := moveElim% "
module 0x42::bump {
    fun bump(x: u64): u64 {
        let r = &mut x;
        *r = *r + 1;
        x
    }
    spec bump {
        requires x < 18446744073709551615;
        aborts_if false;
        ensures result == x + 1;
    }
}
"

-- The original and the eliminated program agree in the interpreter.
#guard interpFun bumpSrc 100 0 [] [.u64 5] matches .ok (.ret ⟨_, []⟩ [.u64 6])
#guard interpFun bump 100 0 [] [.u64 5] matches .ok (.ret ⟨_, []⟩ [.u64 6])
#guard interpFun bumpSrc 100 0 [] [.u64 18446744073709551615]
  matches .ok (.abort _ 0)
#guard interpFun bump 100 0 [] [.u64 18446744073709551615]
  matches .ok (.abort _ 0)

-- The stepping kit uses one uniform simp list per step.
set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- **The borrow-based `bump` verifies from Move source**, through
`moveElim%`, against its genuine `spec` block. -/
theorem bump_verified : Verified bump 0 := by
  refine ⟨_, rfl, 3, ?_⟩
  intro m args current frames
  match args with
  | [] =>
    simp only [wpB, compileFun, compAnns, bump, MProgram.toProgram,
      MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll,
      orAll, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
      abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
      Option.elim, Option.map, List.map, List.flatten, List.append,
      reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_nil] at hlen
    omega
  | v0 :: v1 :: rest =>
    simp only [wpB, compileFun, compAnns, bump, MProgram.toProgram,
      MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll,
      orAll, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
      abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
      Option.elim, Option.map, List.map, List.flatten, List.append,
      reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_cons] at hlen
    omega
  | [v] =>
  simp only [wpB, compileFun, compAnns, bump, MProgram.toProgram,
    MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll,
    orAll, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    List.cons_append, List.nil_append, List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd, Nat.reduceEqDiff]
  intro htyped hreq gt hgt _
  simp only [List.mem_singleton] at hgt
  subst hgt
  simp only [compileInstr, List.map, List.flatten, List.append,
    List.cons_append, List.nil_append, List.mem_cons, List.not_mem_nil,
    or_false, reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
  simp only [typedEntry, TypedArgs] at htyped
  obtain ⟨⟨-, hvalid⟩, -⟩ := htyped
  have hv := hvalid 0 .u64 v rfl rfl
  -- The argument is an unbounded `Int` confined to the `u64` range, and the
  -- proof keeps that view: every value the block computes is then a plain
  -- `Value.int`, so no `Value.u64` abbreviation has to be unfolded inside the
  -- symbolic-execution terms the kernel re-checks.
  simp only [isValid_uint_iff, u64_size_eq] at hv
  obtain ⟨i, rfl, hi0, -⟩ := hv
  -- `requires x < u64::MAX` gives the add its guard
  simp only [Holds, VState.preEnvOf, preEnv, evalSpec_lt_iff,
    evalSpec_loc_iff, evalSpec_value_iff, initLocals, Value.toSVal,
    SVal.int.injEq, exists_eq_left, SVal.bool.injEq,
    List.getElem?_cons_zero, Option.some.injEq, exists_eq_left',
    decide_eq_true_eq] at hreq
  obtain ⟨i', j, hi', rfl, hd⟩ := hreq
  subst i'
  -- checked arithmetic guards on the unbounded value, so the range facts are
  -- supplied in that form; the bound is spelled `U64_SIZE`, as in the goal
  have hltI : i + 1 < (U64_SIZE : Int) := by
    have h := of_decide_eq_true hd.symm
    unfold U64_SIZE
    omega
  have hloI : (0 : Int) ≤ i + 1 := by omega
  -- step the block: checkout, alias moves, read, bump, write, and the
  -- alias write-back cascade to the root
  iterate 8 (refine (wpCmds_onOk_step rfl).mpr ?_
             simp [initLocals, Oper.sem, MoveState.writeLocals,
               hltI, hloI])
  iterate 9 (refine (wpCmds_onOk_step rfl).mpr ?_
             simp [initLocals, Oper.sem, MoveState.writeLocals,
               MoveState.writeLocal, hltI, hloI])
  refine ⟨fun hset => ?_, fun _ => ?_⟩
  · simp [flagSet] at hset
  · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
      postEnv, initLocals, SpecEnv.memAt, Contract.abortsFalse,
      agreesOutside, Contract.footprint, MoveState.writeLocals,
      MoveState.writeLocal]

end Tests.Prover.ElimSource
