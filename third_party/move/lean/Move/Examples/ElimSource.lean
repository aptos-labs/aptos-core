-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Frontend.Elab
import Move.IR.Interp
import Move.Prover.Ivl.Wp
import Move.Prover.Translate.Compile

/-!
# Borrow-Based Move Source, Verified Directly

`moveElim%` closes the loop of roadmap item 4: it elaborates a
self-contained Move module through the real compiler *and* applies the
reference elimination at elaboration time, so borrow-based source
verifies directly — no hand-pinned eliminated code.

The example is the borrow-based `bump` (compiler v2 lowers the mutation
through a chain of reference moves and copies, all of which the
elimination turns into mutation-value plumbing with alias write-backs):

```move
fun bump(x: u64): u64 {
    let r = &mut x;
    *r = *r + 1;
    x
}
```

`bump_verified` proves the eliminated program against the genuine `spec`
block, with the one-command-at-a-time stepping proof.
-/

namespace Move.Examples.ElimSource

open Move.Prover.Ivl
open Move.IR
open Move.Prover.Translate
open Move.Frontend

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
#guard interpFun bumpSrc 100 0 [] [.u64 5] matches .ok (.ret [] [.u64 6])
#guard interpFun bump 100 0 [] [.u64 5] matches .ok (.ret [] [.u64 6])
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
  intro m args
  match args with
  | [] =>
    simp only [wpB, compileFun, compAnns, bump, MProgram.toProgram,
      MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll,
      orAll, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
      abortExitBlock, initVState, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
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
      abortExitBlock, initVState, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
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
    abortExitBlock, initVState, wpBlock, wpTerm, wpEdge,
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
  simp only [isValid_u64_iff] at hv
  obtain ⟨n, rfl, hn⟩ := hv
  -- `requires x < u64::MAX` gives the add its guard
  simp only [Holds, VState.preEnvOf, preEnv, evalSpec_lt_iff,
    evalSpec_loc_iff, evalSpec_value_iff, initLocals, Value.toSVal,
    SVal.int.injEq, exists_eq_left, SVal.bool.injEq,
    List.getElem?_cons_zero, Option.some.injEq, exists_eq_left',
    decide_eq_true_eq] at hreq
  obtain ⟨i, j, rfl, rfl, hd⟩ := hreq
  have hlt : n + 1 < U64_SIZE := by
    have h := of_decide_eq_true hd.symm
    have hn' : n < 18446744073709551615 := by exact_mod_cast h
    unfold U64_SIZE
    omega
  -- step the block: checkout, alias moves, read, bump, write, and the
  -- alias write-back cascade to the root
  iterate 8 (refine (wpCmds_onOk_step rfl).mpr ?_
             simp [initLocals, Oper.sem, MoveState.writeLocals,
               MoveState.writeLocal, hlt])
  iterate 9 (refine (wpCmds_onOk_step rfl).mpr ?_
             simp [initLocals, Oper.sem, MoveState.writeLocals,
               MoveState.writeLocal])
  refine ⟨fun hset => ?_, fun _ => ?_⟩
  · simp [flagSet] at hset
  · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
      postEnv, initLocals, SpecEnv.memAt, Contract.abortsFalse,
      agreesOutside, Contract.footprint, MoveState.writeLocals,
      MoveState.writeLocal]

end Move.Examples.ElimSource
