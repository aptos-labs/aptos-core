-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.RefElim.Transform
import MoveModel.IR.Interp.Exec
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Translate.Compile

/-!
# A Verified Cross-Call Borrow

The first *verified* reference across a call boundary:

```move
fun inc(x: &mut u64) { *x = *x + 1 }
spec inc {
    aborts_if !(*x + 1 < 2^64);
    ensures *x' == *x + 1;      // x' = the final value of the parameter
}
fun bump(v: u64): u64 { inc(&mut v); v }
spec bump {
    requires v + 1 < 2^64;
    aborts_if false;
    ensures result == v + 1;
}
```

`refElimProg` uses `inc`'s borrow summary to eliminate both functions.  It
turns `inc` into a value-in/final-out function and appends the final mutable
parameter value to its results.  In `bump`, the borrow becomes a mutation
checkout that is written back after the call.

The contracts use this eliminated view.  `result 0` is the final value of
`inc`'s mutable parameter.  In specifications, `mutVal` (Boogie's
`$Dereference`) reads the mutation payload.  The `IsValid` entry assumption
ensures that mutable-reference parameters have the required mutation shape.

`bump_verified` verifies the caller through the callee contract.  `bump`'s
precondition rules out the abort branch of `callRel`, using `inc`'s
`aborts_if`.  The normal branch obtains the final value from `inc`'s
`ensures`.
-/

namespace Tests.Prover.CrossCall

open MoveModel.Prover.Ivl
open MoveModel.IR
open MoveModel.Prover.Translate

/-- See the module docs. -/
def incContract : Contract where
  requires := .value (.bool true)
  aborts := some (.not (.binop .lt
    (.binop .add (.mutVal (.loc 0)) (.value (.u64 1)))
    (.value (.u64 U64_SIZE))))
  ensures := .binop .eq (.mutVal (.result 0))
    (.binop .add (.mutVal (.loc 0)) (.value (.u64 1)))
  modifies := []

/-- `inc(x: &mut u64) { *x = *x + 1 }` -/
def incDecl : FunDecl where
  numParams := 1
  numLocals := 4
  locals := fun t => [Ty.mutRef .u64, .u64, .u64, .u64][t]?
  returns := []
  body :=
    { blocks := (fun b =>
        if b = 0 then
          some ⟨[.call [1] .readRef [0], .load 2 (.u64 1),
                 .call [3] (.add .u64) [1, 2], .call [] .writeRef [0, 3]], .ret []⟩
        else none),
      entry := 0,
      size := 1 }
  loopSpecs := fun _ => none
  contract := incContract

/-- See the module docs. -/
def bumpContract : Contract where
  requires := .binop .lt (.binop .add (.loc 0) (.value (.u64 1)))
    (.value (.u64 U64_SIZE))
  aborts := some (.value (.bool false))
  ensures := .binop .eq (.result 0) (.binop .add (.loc 0) (.value (.u64 1)))
  modifies := []

/-- `bump(v: u64): u64 { inc(&mut v); v }` -/
def bumpDecl : FunDecl where
  numParams := 1
  numLocals := 2
  locals := fun t => [Ty.u64, .mutRef .u64][t]?
  returns := [.u64]
  body :=
    { blocks := (fun b =>
        if b = 0 then
          some ⟨[.call [1] .borrowLoc [0], .call [] (.function 1) [1]],
                .ret [0]⟩
        else none),
      entry := 0,
      size := 1 }
  loopSpecs := fun _ => none
  contract := bumpContract

def prog : Program where
  funs := fun f =>
    if f = 0 then some bumpDecl else if f = 1 then some incDecl else none
  structs := fun _ => none

-- The borrow-based original executes (normal path and overflow).
#guard interpFun prog 100 0 [] [.u64 5] matches .ok (.ret ⟨_, []⟩ [.u64 6])
#guard interpFun prog 100 0 [] [.u64 18446744073709551615]
  matches .ok (.abort _ 0)

/-- The eliminated `bump`: checkout, `call r := inc(r)` (the extended
destination receives the final), write-back at the borrow death. -/
def bumpElim : FunDecl := { bumpDecl with
  body :=
    { blocks := (fun b =>
        if b = 0 then
          some ⟨[.call [1] (.mkMutLoc 0) [0],
                 .call [1] (.function 1) [1],
                 .call [0] .getMut [1]],
                .ret [0]⟩
        else none),
      entry := 0,
      size := 1 } }

/-- The eliminated `inc`: `getMut`/`setMut` on the parameter mutation,
which the extended `ret` returns as the final. -/
def incElim : FunDecl := { incDecl with
  returns := [.mutRef .u64]
  body :=
    { blocks := (fun b =>
        if b = 0 then
          some ⟨[.call [1] .getMut [0], .load 2 (.u64 1),
                 .call [3] (.add .u64) [1, 2], .call [0] .setMut [0, 3]],
                .ret [0]⟩
        else none),
      entry := 0,
      size := 1 } }

-- `refElimProg` produces exactly these declarations.
#guard match refElimProg prog.structs [bumpDecl, incDecl] with
  | .ok [b, i] =>
      decide (b.numLocals = 2) && decide (i.numLocals = 4) &&
      (b.body.blocks 0 == bumpElim.body.blocks 0) &&
      (i.body.blocks 0 == incElim.body.blocks 0) &&
      (i.returns == [Ty.mutRef .u64])
  | _ => false

def elimProg : Program where
  funs := fun f =>
    if f = 0 then some bumpElim else if f = 1 then some incElim else none
  structs := prog.structs

-- The eliminated program agrees on the same runs.
#guard interpFun elimProg 100 0 [] [.u64 5] matches .ok (.ret ⟨_, []⟩ [.u64 6])
#guard interpFun elimProg 100 0 [] [.u64 18446744073709551615]
  matches .ok (.abort _ 0)

-- The stepping kit uses one uniform simp list per step.
set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- **The eliminated `inc` verifies**: the mutation parameter's payload is
read, bumped (aborting on overflow, as `aborts_if` claims), and replaced;
the extended `ret` returns the final, pinned by `ensures` via `mutVal`. -/
theorem inc_verified : Verified elimProg 1 := by
  refine ⟨incElim, by simp [elimProg], 3, ?_⟩
  intro m args current frames
  match args with
  | [] =>
    simp only [wpB, compileFun, compAnns, elimProg, incElim, incDecl,
      incContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
      wpCmds, onOk, Option.elim, Option.map, List.map, List.flatten,
      List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_nil] at hlen
    omega
  | v0 :: v1 :: rest =>
    simp only [wpB, compileFun, compAnns, elimProg, incElim, incDecl,
      incContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
      wpCmds, onOk, Option.elim, Option.map, List.map, List.flatten,
      List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_cons] at hlen
    omega
  | [v] =>
  simp only [wpB, compileFun, compAnns, elimProg, incElim, incDecl,
    incContract, compileBlock, termCmds, termGoto, compileInstr,
    retExitBlock, abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    List.cons_append, List.nil_append, List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd, Nat.reduceEqDiff]
  intro htyped _hreq gt hgt _
  simp only [List.mem_singleton] at hgt
  subst hgt
  simp only [compileInstr, List.map, List.flatten, List.append,
    List.cons_append, List.nil_append, List.mem_cons, List.not_mem_nil,
    or_false, reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
  simp only [typedEntry, TypedArgs] at htyped
  obtain ⟨⟨-, hvalid⟩, -⟩ := htyped
  have hv := hvalid 0 (.mutRef .u64) v rfl rfl
  simp only [isValid_mutRef_iff, isValid_uint_iff] at hv
  obtain ⟨rt, w, rfl, k, rfl, h0k, hk⟩ := hv
  by_cases hlt : k + 1 < (IntWidth.w64.size : Int)
  · -- normal path: read, bump, replace, return the final
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
    have hadd : 0 ≤ k + 1 ∧ k + 1 < (U64_SIZE : Int) := by
      rw [u64_size_eq] at hlt; omega
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, hadd.1, hadd.2]
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
    refine ⟨fun hset => ?_, fun _ => ?_⟩
    · simp [flagSet] at hset
    · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
        abortEnv, postEnv, initLocals, Oper.sem, SpecEnv.memAt, Contract.abortsFalse,
        agreesOutside, Contract.footprint, incContract, hlt]
      rw [← u64_size_eq]; exact hlt
  · -- overflow: the bump aborts, as claimed by `aborts_if`
    have hover : ¬(0 ≤ k + 1 ∧ k + 1 < (U64_SIZE : Int)) := by
      rw [u64_size_eq] at hlt; omega
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, MoveState.writeLocals, hover, VState.doAbort]
    iterate 2 refine (wpCmds_onOk_skip rfl).mpr ?_
    refine ⟨fun _ => ?_, fun hclear => ?_⟩
    · simp [wpCmds, Holds, VState.preEnvOf, preEnv, initLocals,
        abortEnv, postEnv, SpecEnv.memAt, Contract.abortsHolds, incContract,
        hlt, VState.doAbort]
      rw [u64_size_eq] at hlt
      omega
    · simp [flagClear, VState.doAbort] at hclear

-- The stepping kit uses one uniform simp list per step.
set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- **The eliminated `bump` verifies through `inc`'s contract**: the
`callRel` havoc's abort branch is refuted by `requires` against `inc`'s
`aborts_if`, its normal branch pins the returned final via `inc`'s
`ensures`, and the write-back delivers it to the local. -/
theorem bump_verified : Verified elimProg 0 := by
  refine ⟨bumpElim, by simp [elimProg], 3, ?_⟩
  intro m args current frames
  match args with
  | [] =>
    simp only [wpB, compileFun, compAnns, elimProg, bumpElim, bumpDecl,
      bumpContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
      wpCmds, onOk, Option.elim, Option.map, List.map, List.flatten,
      List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_nil] at hlen
    omega
  | v0 :: v1 :: rest =>
    simp only [wpB, compileFun, compAnns, elimProg, bumpElim, bumpDecl,
      bumpContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
      wpCmds, onOk, Option.elim, Option.map, List.map, List.flatten,
      List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_cons] at hlen
    omega
  | [v] =>
  simp only [wpB, compileFun, compAnns, elimProg, bumpElim, bumpDecl,
    bumpContract, compileBlock, termCmds, termGoto, compileInstr,
    retExitBlock, abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge,
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
  -- the caller's `requires`: n + 1 < 2^64
  simp only [Holds, VState.preEnvOf, preEnv, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, evalSpec_lt_iff,
    evalSpec_add_iff, evalSpec_loc_iff, evalSpec_value_iff, initLocals,
    Value.toSVal, SVal.int.injEq, exists_eq_left, SVal.bool.injEq,
    List.getElem?_cons_zero, Option.some.injEq, exists_eq_left',
    decide_eq_true_eq] at hreq
  obtain ⟨i, j, ⟨i₁, j₁, rfl, rfl, rfl⟩, rfl, hd⟩ := hreq
  have hn1 : n + 1 < U64_SIZE := by
    have := of_decide_eq_true hd.symm
    exact_mod_cast this
  -- checkout
  refine (wpCmds_onOk_step rfl).mpr ?_
  simp [initLocals, Oper.sem, MoveState.writeLocals, MoveState.writeLocal]
  -- the call: assert the callee's `requires` (trivial), then the havoc
  refine (wpCmds_cons_assert).mpr ⟨?_, ?_⟩
  · intro _
    refine ⟨[.mut ⟨.loc current 0, []⟩ (.u64 n)], by
      simp [initLocals], ?_⟩
    simp [Holds, preEnv, incElim, incDecl, incContract]
  · refine (wpCmds_cons_havoc).mpr ?_
    intro v' hrel
    rw [callRel] at hrel
    simp only [initLocals] at hrel
    rw [if_neg (by simp)] at hrel
    obtain ⟨cargs, hcargs, hbranch⟩ := hrel
    simp [MoveState.writeLocal] at hcargs
    subst hcargs
    rcases hbranch with ⟨code, _mAbort, habort, rfl⟩ | hnormal
    · -- the callee's abort branch is refuted by the caller's `requires`
      exfalso
      simp [Contract.abortsHolds, incElim, incDecl, incContract, Holds,
        preEnv, initLocals] at habort
      omega
    · obtain ⟨mNew, rets, hlen, -, hens, hframe, htyv, hvis⟩ := hnormal
      have hvis' := hvis.symm
      have hvAborted : v'.aborted = none := by simpa using hvis'.aborted
      -- the final: a mutation carrying n + 1, from `ensures` + typing
      have hTM : TypedMemory prog.structs m := by
        intro r sd a w hsd
        simp [prog] at hsd
      obtain ⟨-, hretsV⟩ := htyv hTM
      simp only [List.length_cons, List.length_nil] at hlen
      match rets, hlen with
      | [r₀], _ =>
      simp only [incElim, incDecl, isValidList_cons_iff,
        isValidList_nil_iff, isValid_mutRef_iff, isValid_u64_iff,
        List.cons.injEq] at hretsV
      obtain ⟨w0, tail, ⟨rfl, -⟩, ⟨rt', w', rfl, k', rfl, hk'⟩, -⟩ :=
        hretsV
      -- `ensures` pins the payload: k' = n + 1
      simp only [incElim, incDecl, incContract, Holds, postEnv,
        evalSpec_eq_iff,
        evalSpec_mutVal_iff, evalSpec_add_iff, evalSpec_result_iff,
        evalSpec_loc_iff, evalSpec_value_iff, initLocals,
        List.getElem?_cons_zero, Option.some.injEq, Value.toSVal] at hens
      have hkn : k' = n + 1 := by
        rcases hens with ⟨v, hres, harg, -⟩ | ⟨-, -, -, -, -, hcontra⟩
        · obtain ⟨t, v₁, hv₁, hmv⟩ := hres
          obtain ⟨i, j, ⟨t', v'', hv'', hmv'⟩, hj, hvij⟩ := harg
          subst hv₁
          subst hv''
          simp only [Value.toSVal, SVal.mut.injEq, SVal.int.injEq]
            at hmv hmv' hj
          obtain ⟨-, rfl⟩ := hmv
          obtain ⟨-, hi⟩ := hmv'
          subst hj
          simp only [SVal.int.injEq] at hvij
          omega
        · simp at hcontra
      subst hkn
      have hvMut : v'.cur.locals 1 =
          some (.mut rt' (.u64 (n + 1))) := by
        rw [hvis'.locals]
        simp [MoveState.writeLocals, initLocals]
      have hvMut' : v'.cur.frames v'.cur.current 1 =
          some (.mut rt' (.u64 (n + 1))) := hvMut
      -- the frame: `inc` modifies nothing
      have hmemEq : ∀ r a, mNew r a = m r a := by
        intro r a
        refine hframe r a ?_
        simp [Contract.footprint, incElim, incDecl, incContract]
      -- write-back: the final's payload lands in the local
      refine (wpCmds_onOk_step hvAborted).mpr ?_
      simp [Oper.sem, MoveState.writeLocals, initLocals,
        hvMut', hvis'.memory, hvAborted]
      -- return
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [MoveState.writeLocals, initLocals, hvis'.locals, hvAborted]
      refine ⟨fun hset => ?_, fun _ => ?_⟩
      · simp [flagSet] at hset
      · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
          postEnv, initLocals, SpecEnv.memAt, Contract.abortsFalse,
          bumpContract, agreesOutside, Contract.footprint,
          MoveState.writeLocals, MoveState.writeLocal,
          hvis'.memory, hvis'.snaps, hvis'.args, hvis'.rets, hvAborted]
        intro r a'
        exact hmemEq r a'

end Tests.Prover.CrossCall
