-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.RefElim.Transform
import MoveModel.IR.Interp.Exec
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Translate.Compile
import MoveModel.Tests.Prover.Account

/-!
# The Account Example with References

This version of `withdraw` uses the mutable-borrow form produced by the Move
compiler.  It exercises both the reference semantics and reference
elimination:

```
t2 := borrow_global_mut<Account>(t0)   // aborts if absent
t3 := borrow_field<0>(t2)              // &mut .balance
t4 := read_ref(t3)
t5 := t4 - t1                          // aborts on underflow
write_ref(t3, t5)
ret
```

The borrow-based program executes in the interpreter, as checked by the
`#guard`s below.  It does not verify directly because reference operations
compile to failing assertions.

`refElimFun` rewrites the program into the value-level declaration
`elimDecl`.  A `#guard` fixes the expected output and interpreter agreement.
Finally, `borrow_withdraw_verified` proves the eliminated program against the
same contract as the hand-eliminated `Tests.Prover.Account` version.
-/

namespace Tests.Prover.BorrowAccount

open MoveModel.Prover.Ivl
open MoveModel.IR
open MoveModel.Prover.Translate
open Tests.Prover.Account (ACCOUNT withdrawContract)

/-- `withdraw` through a mutable borrow (see module docs). -/
def borrowDecl : FunDecl where
  numParams := 2
  numLocals := 6
  locals := fun t =>
    [Ty.address, .u64, .mutRef (.struct ACCOUNT), .mutRef .u64, .u64,
      .u64][t]?
  returns := []
  body :=
    { blocks := fun b =>
        if b = 0 then
          some ⟨[.call [2] (.borrowGlobal ACCOUNT) [0],
                 .call [3] (.borrowField 0) [2],
                 .call [4] .readRef [3],
                 .call [5] (.sub .u64) [4, 1],
                 .call [] .writeRef [3, 5]],
                .ret []⟩
        else none
      entry := 0
      size := 1 }
  loopSpecs := fun _ => none
  contract := withdrawContract

def prog : Program where
  funs := fun f => if f = 0 then some borrowDecl else none
  structs := fun r => if r = ACCOUNT then some { fields := [.u64] } else none

-- Evaluation of the borrow-based original (reference semantics): normal
-- path, missing account, and underflow.
#guard interpFun prog 100 0 [(0, 3, .struct [.u64 10])] [.address 3, .u64 4]
  matches .ok (.ret ⟨_, [(0, 3, .struct [.u64 6])]⟩ [])
#guard interpFun prog 100 0 [] [.address 3, .u64 4]
  matches .ok (.abort _ 0)
#guard interpFun prog 100 0 [(0, 3, .struct [.u64 2])] [.address 3, .u64 4]
  matches .ok (.abort _ 0)

/-- The eliminated code — the mutation algebra of the full elimination:
the global borrow checks out the resource into a mutation (aborting if
absent, like the borrow), the field borrow derives a sub-mutation, the
read and the write-through act on the carried value, and the two borrow
deaths write back — the field mutation into its parent's payload
(`update_field`), the root mutation into global memory (`write_global` at
its recorded address). -/
def elimBlock : Block :=
  ⟨[.call [2] (.mkMutGlobal ACCOUNT) [0],
    .call [3] (.childMutField 0) [2],
    .call [4] .getMut [3],
    .call [5] (.sub .u64) [4, 1],
    .call [3] .setMut [3, 5],
    .call [6] .getMut [2],
    .call [7] (.getField 0) [6],
    .call [8] .getMut [3],
    .call [9] (.updateField 0) [6, 8],
    .call [2] .setMut [2, 9],
    .call [10] .mutAddr [2],
    .call [11] .getMut [2],
    .call [] (.writeGlobal ACCOUNT) [10, 11]],
   .ret []⟩

-- `refElimFun` produces exactly this code and the matching declarations.
#guard match refElimFun (fun _ => none) prog.structs borrowDecl with
  | .ok d =>
      decide (d.numLocals = 12) &&
      ((List.range d.body.size).map (fun b => d.body.blocks b)
        == [some elimBlock])
  | .error _ => false

/-- The eliminated function (the code pinned by the `#guard` above). -/
def elimDecl : FunDecl where
  numParams := 2
  numLocals := 12
  locals := fun t =>
    [Ty.address, .u64, .mutRef (.struct ACCOUNT), .mutRef .u64, .u64, .u64,
      .struct ACCOUNT, .u64, .u64, .struct ACCOUNT, .address,
      .struct ACCOUNT][t]?
  returns := []
  body :=
    { blocks := fun b => if b = 0 then some elimBlock else none
      entry := 0
      size := 1 }
  loopSpecs := fun _ => none
  contract := withdrawContract

def elimProg : Program where
  funs := fun f => if f = 0 then some elimDecl else none
  structs := prog.structs

-- The eliminated program agrees with the original on the same runs.
#guard interpFun elimProg 100 0 [(0, 3, .struct [.u64 10])]
    [.address 3, .u64 4]
  matches .ok (.ret ⟨_, [(0, 3, .struct [.u64 6])]⟩ [])
#guard interpFun elimProg 100 0 [] [.address 3, .u64 4]
  matches .ok (.abort _ 0)
#guard interpFun elimProg 100 0 [(0, 3, .struct [.u64 2])]
    [.address 3, .u64 4]
  matches .ok (.abort _ 0)

-- The stepping kit below uses one uniform simp list per step; the linter
-- would have each list minimized per instruction, which hurts uniformity.
set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- **The eliminated `withdraw` verifies** against the same contract as the
hand-eliminated `Tests.Prover.Account` version — borrow-based code becomes
verifiable through `refElimFun`.  The proof *steps* the compiled block one
command at a time (`wpCmds_onOk_step`/`wpCmds_onOk_skip`), which keeps the
verification-condition terms small. -/
theorem borrow_withdraw_verified : Verified elimProg 0 := by
  refine ⟨elimDecl, by simp [elimProg], 3, ?_⟩
  intro m args current frames
  -- Destructure the argument list while the verification condition is
  -- still folded: case analysis over the unfolded VC does not scale.
  -- Wrong arities are refuted by the arity part of the typing assumption.
  match args with
  | [] =>
    simp only [wpB, compileFun, compAnns, elimProg, elimDecl, elimBlock,
    withdrawContract, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_nil] at hlen
    omega
  | [v] =>
    simp only [wpB, compileFun, compAnns, elimProg, elimDecl, elimBlock,
    withdrawContract, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_nil, List.length_cons] at hlen
    omega
  | v0 :: v1 :: v2 :: rest =>
    simp only [wpB, compileFun, compAnns, elimProg, elimDecl, elimBlock,
    withdrawContract, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVState, initVStateAt, MoveState.locals, setFrame, setFrame_same, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_cons] at hlen
    omega
  | [v0, v1] =>
  -- Unfold the block structure but keep `wpCmds` of the body folded — the
  -- body is stepped command by command below.
  simp only [wpB, compileFun, compAnns, elimProg, elimDecl, elimBlock,
    withdrawContract, compileBlock, termCmds, termGoto, compileInstr,
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
  simp only [typedEntry, TypedArgs, TypedMemory] at htyped
  obtain ⟨⟨-, hvalid⟩, hmem⟩ := htyped
  have hv0 := hvalid 0 .address v0 rfl rfl
  have hv1 := hvalid 1 .u64 v1 rfl rfl
  simp only [isValid_address_iff] at hv0
  simp only [isValid_u64_iff] at hv1
  obtain ⟨a, rfl⟩ := hv0
  obtain ⟨amt, rfl, -⟩ := hv1
  have hacct : m ACCOUNT a = none ∨
      ∃ b, b < U64_SIZE ∧ m ACCOUNT a = some (.struct [.u64 b]) := by
    match hm : m ACCOUNT a with
    | none => exact .inl rfl
    | some v =>
      have hval := hmem ACCOUNT { fields := [.u64] } a v rfl hm
      simp only [isValid_struct_iff] at hval
      obtain ⟨d, fs, hd, rfl, hfs⟩ := hval
      obtain rfl : d = { fields := [Ty.u64] } := by
        simp [prog, ACCOUNT] at hd
        exact hd.symm
      simp only [isValidList_cons_iff, isValidList_nil_iff,
        isValid_u64_iff] at hfs
      obtain ⟨v', vs', rfl, ⟨b, rfl, hb⟩, rfl⟩ := hfs
      exact .inr ⟨b, hb, rfl⟩
  clear hmem hvalid _hreq
  rcases hacct with habs | ⟨b, hb, hpres⟩
  · -- Account absent: the mutation checkout (the borrow point) aborts.
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, habs, VState.doAbort]
    iterate 12 refine (wpCmds_onOk_skip rfl).mpr ?_
    refine ⟨fun _ => ?_, fun hclear => ?_⟩
    · simp [wpCmds, Holds, VState.preEnvOf, preEnv, abortEnv, postEnv,
        initLocals, SpecEnv.memAt, Contract.abortsHolds, Account.curBal,
        Account.oldBal, Account.oldExists, habs, VState.doAbort]
    · simp [flagClear] at hclear
  · rcases Nat.lt_or_ge b amt with hlt | hge
    · -- Insufficient balance: `sub` aborts.
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, hpres]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        Nat.not_le.mpr hlt, VState.doAbort]
      iterate 9 refine (wpCmds_onOk_skip rfl).mpr ?_
      refine ⟨fun _ => ?_, fun hclear => ?_⟩
      · simp [wpCmds, Holds, VState.preEnvOf, preEnv, abortEnv, postEnv,
          initLocals, SpecEnv.memAt, Contract.abortsHolds, Account.curBal,
          Account.oldBal, Account.oldExists, hpres, hlt, VState.doAbort]
      · simp [flagClear, VState.doAbort] at hclear
    · -- Sufficient balance: normal path through the return exit.
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, hpres]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      have hsub : (b : Int) - (amt : Int) < (U64_SIZE : Int) := by
        have : (b : Int) < (U64_SIZE : Int) := by exact_mod_cast hb
        have : (0 : Int) ≤ (amt : Int) := Int.natCast_nonneg _
        omega
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        hge, hsub]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        memWrite]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine ⟨fun hset => ?_, fun _ => ?_⟩
      · simp [flagSet] at hset
      · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
          abortEnv, postEnv, initLocals, hpres, Oper.sem, SpecEnv.memAt,
          Contract.abortsFalse, Account.curBal, Account.oldBal,
          Account.oldExists, hge, memWrite, MoveState.writeLocals,
          agreesOutside, Contract.footprint]
        intro r a' hout hr ha'
        exact absurd ha' (hout hr.symm)

end Tests.Prover.BorrowAccount
