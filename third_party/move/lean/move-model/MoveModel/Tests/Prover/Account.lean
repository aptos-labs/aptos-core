-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.Wp
import MoveModel.IR.Semantics
import MoveModel.Prover.Translate.Compile

/-!
# The Account Example (TACAS'22, Fig. 1/2)

This module formalizes the running example from the TACAS'22 paper: an
`Account` resource holding a `u64` balance and the following `withdraw`
function and contract.

```move
fun withdraw(account: address, amount: u64) acquires Account {
    let balance = &mut borrow_global_mut<Account>(account).balance;
    *balance = *balance - amount;   // aborts on underflow
}
spec withdraw {
    aborts_if !old(exists<Account>(account));
    aborts_if old(global<Account>(account).balance) < amount;
    ensures global<Account>(account).balance
              == old(global<Account>(account).balance) - amount;
    modifies global<Account>(account);
}
```

The Lean definition below models the function after reference elimination:
read the balance, compute its new value, and write it back.  Its body is a
single basic block of three-address instructions.  The contract uses the
deep specification-expression language; `old(..)` is represented by a
`global` access labeled with `preLabel`, the memory snapshot from function
entry.  The theorem `withdraw_verified` discharges the compiled verification
condition with the Lean weakest-precondition calculus, demonstrating that
the definitions work together end to end.

The contract does not state that the boundary state is well typed.  Those
facts come from the injected `WellFormed` assumptions (`typedEntry`).  They
ensure that arguments have their declared types and that each `Account`
resource contains one `u64` field, matching the real prover's multisorted
encoding.
-/

namespace Tests.Prover.Account

open MoveModel.Prover.Ivl
open MoveModel.IR
open MoveModel.Prover.Translate

/-- The `Account` resource type. -/
def ACCOUNT : ResourceId := 0

/-- `global<Account>(account).balance` in the current state. -/
def curBal : SpecExp := .select 0 (.global ACCOUNT none (.loc 0))

/-- `old(global<Account>(account).balance)`: the `preLabel` snapshot. -/
def oldBal : SpecExp := .select 0 (.global ACCOUNT (some preLabel) (.loc 0))

/-- `old(exists<Account>(account))`: existence in the entry snapshot. -/
def oldExists : SpecExp := .exists_ ACCOUNT (some preLabel) (.loc 0)

/-- The specification of `withdraw` from the paper (see module docs). -/
def withdrawContract : Contract where
  requires := .value (.bool true)
  aborts := some
    (.binop .or
      (.not oldExists)
      (.binop .lt oldBal (.loc 1)))
  ensures := .binop .eq curBal (.binop .sub oldBal (.loc 1))
  modifies := [(ACCOUNT, .loc 0)]

/-- Body of `withdraw` (locals: 0 = account address, 1 = amount; 2–5
scratch), a single basic block:

```
t2 := get_global<Account>(t0)    // aborts if absent
t3 := t2.balance
t4 := t3 - t1                    // aborts on underflow
t5 := Account { balance: t4 }
write_global<Account>(t0, t5)
ret
```
-/
def withdrawBody : Cfg where
  blocks := fun b =>
    if b = 0 then
      some ⟨[.call [2] (.getGlobal ACCOUNT) [0],
             .call [3] (.getField 0) [2],
             .call [4] (.sub .u64) [3, 1],
             .call [5] .pack [4],
             .call [] (.writeGlobal ACCOUNT) [0, 5]],
            .ret []⟩
    else none
  entry := 0
  size := 1

def withdrawDecl : FunDecl where
  numParams := 2
  numLocals := 6
  locals := fun t =>
    [Ty.address, .u64, .struct ACCOUNT, .u64, .u64, .struct ACCOUNT][t]?
  returns := []
  body := withdrawBody
  loopSpecs := fun _ => none
  contract := withdrawContract

def prog : Program where
  funs := fun f => if f = 0 then some withdrawDecl else none
  structs := fun r => if r = ACCOUNT then some { fields := [.u64] } else none

set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- **`withdraw` verifies**: the compiled verification condition holds via
the Lean WP calculus, for every boundary state. -/
theorem withdraw_verified : Verified prog 0 := by
  refine ⟨withdrawDecl, by simp [prog], 3, ?_⟩
  intro m args current frames
  -- Keep the VC folded while splitting the argument shape.  Case analysis
  -- after expanding a straight-line block duplicates a very large term.
  match args with
  | [] =>
    simp only [wpB, compileFun, compAnns, withdrawDecl, withdrawBody,
      withdrawContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVStateAt, MoveState.locals,
      wpBlock, wpTerm, wpEdge, wpCmds, onOk, Option.elim, Option.map,
      List.map, List.flatten, List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_nil] at hlen
    omega
  | [v] =>
    simp only [wpB, compileFun, compAnns, withdrawDecl, withdrawBody,
      withdrawContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVStateAt, MoveState.locals,
      wpBlock, wpTerm, wpEdge, wpCmds, onOk, Option.elim, Option.map,
      List.map, List.flatten, List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_cons, List.length_nil] at hlen
    omega
  | v0 :: v1 :: v2 :: rest =>
    simp only [wpB, compileFun, compAnns, withdrawDecl, withdrawBody,
      withdrawContract, compileBlock, termCmds, termGoto, compileInstr,
      retExitBlock, abortExitBlock, initVStateAt, MoveState.locals,
      wpBlock, wpTerm, wpEdge, wpCmds, onOk, Option.elim, Option.map,
      List.map, List.flatten, List.append, reduceIte, Nat.reduceAdd]
    intro htyped _hreq gt hgt _
    simp only [typedEntry, TypedArgs] at htyped
    obtain ⟨⟨hlen, -⟩, -⟩ := htyped
    simp only [List.length_cons] at hlen
    omega
  | [v0, v1] =>
  -- Expose the block structure, but leave its `wpCmds` folded.  The body is
  -- reduced one command at a time below.
  simp only [wpB, compileFun, compAnns, withdrawDecl, withdrawBody,
    withdrawContract, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVStateAt, MoveState.locals, wpBlock, wpTerm, wpEdge,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    List.cons_append, List.nil_append, List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd, Nat.reduceEqDiff]
  intro htyped _hreq gt hgt _
  simp only [List.mem_singleton] at hgt
  subst hgt
  simp only [compileInstr, List.map, List.flatten,
    List.append, List.cons_append, List.nil_append, List.mem_cons,
    List.not_mem_nil, or_false, reduceIte, Nat.reduceSub, Nat.reduceLT,
    Nat.reduceEqDiff]
  -- Typing of the boundary state from the injected `WellFormed`
  -- assumption: argument shapes and the canonical form of `Account`
  -- resources in memory.
  simp only [typedEntry, TypedArgs, TypedMemory] at htyped
  obtain ⟨⟨-, hvalid⟩, hmem⟩ := htyped
  have hv0 := hvalid 0 .address v0 rfl rfl
  have hv1 := hvalid 1 .u64 v1 rfl rfl
  simp only [isValid_address_iff] at hv0
  simp only [isValid_u64_iff] at hv1
  obtain ⟨a, rfl⟩ := hv0
  obtain ⟨amt, rfl, -⟩ := hv1
  -- Canonical form of the account state at the target address.
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
  · -- Account absent: `get_global` aborts; only the abort exit is enabled,
    -- and its assert needs the first `aborts_if` disjunct.
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, habs, VState.doAbort]
    iterate 5 refine (wpCmds_onOk_skip rfl).mpr ?_
    refine ⟨fun _ => ?_, fun hclear => ?_⟩
    · simp [wpCmds, Holds, VState.preEnvOf, VState.doAbort, preEnv,
        abortEnv, postEnv, initLocals, habs, Oper.sem, SpecEnv.memAt,
        Contract.abortsHolds, curBal, oldBal, oldExists]
    · simp [flagClear, VState.doAbort] at hclear
  · rcases Nat.lt_or_ge b amt with hlt | hge
    · -- Insufficient balance: `sub` aborts; the abort exit's assert needs
      -- the second `aborts_if` disjunct.
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, hpres]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        Nat.not_le.mpr hlt, VState.doAbort]
      iterate 3 refine (wpCmds_onOk_skip rfl).mpr ?_
      refine ⟨fun _ => ?_, fun hclear => ?_⟩
      · simp [wpCmds, Holds, VState.preEnvOf, VState.doAbort, preEnv,
          abortEnv, postEnv, initLocals, hpres, Oper.sem, SpecEnv.memAt,
          Contract.abortsHolds, curBal, oldBal, oldExists, hlt]
      · simp [flagClear, VState.doAbort] at hclear
    · -- Sufficient balance: normal path through the return exit.
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, hpres]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      have hsub : (b : Int) - (amt : Int) < (U64_SIZE : Int) := by
        have : (b : Int) < (U64_SIZE : Int) := by exact_mod_cast hb
        have : (0 : Int) ≤ (amt : Int) := Int.natCast_nonneg _
        omega
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals, hge, hsub]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals, memWrite]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine ⟨fun hset => ?_, fun _ => ?_⟩
      · simp [flagSet] at hset
      · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
          abortEnv, postEnv, initLocals, hpres, Oper.sem, SpecEnv.memAt,
          Contract.abortsFalse, curBal, oldBal, oldExists, hge, memWrite,
          MoveState.writeLocals, agreesOutside, Contract.footprint]
        intro r a' hout hr ha'
        exact absurd ha' (hout hr.symm)

end Tests.Prover.Account
