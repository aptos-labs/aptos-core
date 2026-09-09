-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.IR.Interp.Exec
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Translate.Compile

/-!
# Examples Authored in Move Source

These examples embed Move source rather than masm.  The `move%` elaborator
compiles each self-contained module with compiler v2 and lifts it with the
real stackless generator.  Contracts come from genuine `spec` blocks, loop
invariants come from inline `spec { invariant … }` blocks, and declared types
determine the injected `WellFormed` assumptions.

`count_down` exercises the loop-invariant rule.  `take` exercises global
memory, `move_from`, `old(..)`, and the `modifies` frame.  Both examples run
in the interpreter and verify with the Lean weakest-precondition calculus.
-/

namespace Tests.Prover.MoveSource

open MoveModel.Prover.Ivl
open MoveModel.IR
open MoveModel.Frontend.XIR
open MoveModel.Prover.Translate

def countDown : Program := move% "
module 0x42::count_down {
    fun count_down(x: u64): u64 {
        while (0 < x) {
            x = x - 1;
        } spec {
            invariant x <= 18446744073709551615;
        };
        x
    }
    spec count_down {
        ensures result == 0;
    }
}
"

-- Evaluation: `count_down 5` returns `0`.
#guard interpFun countDown 100 0 [] [.u64 5] matches .ok (.ret ⟨_, []⟩ [.u64 0])

def take : Program := move% "
module 0x42::account {
    struct Account has key { balance: u64 }

    fun take(addr: address): u64 acquires Account {
        let Account { balance } = move_from<Account>(addr);
        balance
    }
    spec take {
        requires exists<Account>(addr);
        aborts_if false;
        ensures result == old(global<Account>(addr).balance);
        ensures !exists<Account>(addr);
        modifies global<Account>(addr);
    }
}
"

-- Evaluation: taking from a balance of 10 returns 10 and removes the
-- resource.
#guard interpFun take 100 0 [(0, 3, .struct [.u64 10])] [.address 3]
  matches .ok (.ret ⟨_, []⟩ [.u64 10])

-- Evaluation: aborts if the account is absent.
#guard interpFun take 100 0 [] [.address 3] matches .ok (.abort _ 0)

set_option maxHeartbeats 1000000 in
/-- **`count_down` verifies**, from Move source. -/
theorem count_down_verified : Verified countDown 0 := by
  refine ⟨_, rfl, 5, ?_⟩
  intro m args current frames
  simp only [wpB, compileFun, compAnns, countDown, MProgram.toProgram,
    MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll, orAll,
    compileBlock, termCmds, termGoto, retExitBlock, abortExitBlock, initVStateAt, MoveState.locals, wpBlock,
    wpTerm, wpEdge, wpCmds, onOk, denoteLoopSpec, Option.elim, Option.map,
    List.mem_cons, List.not_mem_nil, or_false, List.find?,
    List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd,
    Nat.reduceEqDiff]
  intro htyped _hreq gt hgt _
  rcases hgt with rfl
  simp only [reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
  -- Typing of the argument from the injected `WellFormed` assumption.
  simp only [typedEntry, TypedArgs] at htyped
  obtain ⟨⟨hlen, hvalid⟩, hTM⟩ := htyped
  obtain ⟨v0, rfl⟩ : ∃ v, args = [v] := by
    cases args with
    | nil => simp at hlen
    | cons a as =>
      cases as with
      | nil => exact ⟨a, rfl⟩
      | cons b bs => simp at hlen
  have hv0 := hvalid 0 .u64 v0 rfl rfl
  simp only [isValid_u64_iff] at hv0
  obtain ⟨n, rfl, hn⟩ := hv0
  simp only [U64_SIZE] at hn
  constructor
  · -- Base case: the invariant holds at loop entry (`n ≤ u64::MAX` from
    -- typing).
    refine ⟨rfl, ?_, hTM, ?_⟩
    · intro i t v ht hv
      apply hvalid i t v ht
      simpa [initLocals] using hv
    · simp [Holds, VState.curEnv, andAll, initLocals]
      omega
  · -- Inductive case.
    intro s' hT hInv
    obtain ⟨hsnaps, hargs, -, -, hmem, -⟩ := hT
    obtain ⟨hab, hTL, hTM', hu⟩ := hInv
    simp [Holds, VState.curEnv, andAll] at hu
    obtain ⟨i, ⟨v0, hl0, hveq⟩, hk⟩ := hu
    cases v0 <;> simp at hveq
    case right.int i =>
    subst hveq
    -- `Value.u64` is an abbreviation for `.int`, so the loop variable arrives
    -- as an `Int`.  Local 0 is declared `u64`, and typing bounds it below;
    -- that recovers the natural-number view the rest of this proof works in.
    have h0 : (0 : Int) ≤ i := by
      have hv := hTL 0 (Ty.uint .w64) _ (by rfl) hl0
      rw [isValid_uint_iff] at hv
      obtain ⟨j, hj, hlo, -⟩ := hv
      injection hj with hj; omega
    obtain ⟨k, rfl⟩ : ∃ k : Nat, i = (k : Int) := ⟨i.toNat, by omega⟩
    -- and put it back in the `u64` spelling the rewrites below are keyed on
    replace hl0 : s'.cur.frames s'.cur.current 0 = some (Value.u64 k) := hl0
    intro gt2 hgt2 hg2
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hgt2
    rcases hgt2 with rfl | rfl | rfl
    · -- abort-exit edge: the flag is clear
      simp [hab, hl0, Oper.sem,         MoveState.writeLocals, flagSet] at hg2
    · -- loop-body edge (0 < k)
      simp [hab, hl0, Oper.sem,         MoveState.writeLocals] at hg2
      have hk1 : 1 ≤ k := hg2
      have hkValid : k < U64_SIZE := by
        simp [U64_SIZE] at hk ⊢
        omega
      have hTL1 := hTL.writeU64 (x := 1) (n := 0) (by simp)
        (by simp [U64_SIZE])
      have hTL2 := hTL1.writeU64 (x := 2) (n := k) (by simp) hkValid
      have hTL3 := hTL2.writeBool (x := 3) (b := true) (by simp)
      have hTL4 := hTL3.writeU64 (x := 4) (n := k) (by simp) hkValid
      have hTL5 := hTL4.writeU64 (x := 5) (n := 1) (by simp)
        (by simp [U64_SIZE])
      have hTL6 := hTL5.writeU64 (x := 6) (n := k - 1) (by simp) (by omega)
      have hTL7 := hTL6.writeU64 (x := 0) (n := k - 1) (by simp) (by omega)
      -- checked arithmetic now guards on the unbounded `Int` value, so the
      -- range facts have to be supplied in that form
      have hk1' : (1 : Int) ≤ (k : Int) := by exact_mod_cast hk1
      have hkSub : (k : Int) - 1 < (U64_SIZE : Int) := by
        have hkI : (k : Int) < (U64_SIZE : Int) := by exact_mod_cast hkValid
        omega
      simp [wpCmds, compileInstr, onOk, hab, hl0, hg2, hk1', hkSub,
        Oper.sem, MoveState.writeLocals, Holds,
        VState.curEnv, VState.doAbort]
      intro g' b' hedge hg'
      rcases hedge with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · simp [flagSet] at hg'
      · -- back edge: the invariant is re-established for `k - 1`
        refine ⟨rfl, ?_, by simpa using hTM', ?_⟩
        · -- `k - 1` is truncated `Nat` subtraction in the typing chain and
          -- unbounded `Int` subtraction in the goal; they agree since `1 ≤ k`
          have hcast : ((k - 1 : Nat) : Int) = (k : Int) - 1 := by omega
          simpa [MoveState.locals, Value.u64, hcast] using hTL7
        · simp [andAll]
          omega
    · -- exit edge (k = 0): the exit assertions hold
      simp [hab, hl0, Oper.sem,         MoveState.writeLocals] at hg2
      simp [wpCmds, compileInstr, onOk, hab, hl0, hg2, Oper.sem,
        MoveState.writeLocals, Holds,
        VState.preEnvOf, VState.postEnvOf, VState.curEnv, VState.doAbort,
        preEnv, postEnv, agreesOutside, Contract.footprint,
        Contract.abortsFalse, hsnaps, hargs]
      intro g' b' hedge hg'
      rcases hedge with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · simp [flagSet] at hg'
      · simp [wpCmds]
        intro r a'
        exact hmem r a' (fun h => nomatch h)

set_option maxHeartbeats 8000000 in
/-- **`take` verifies**, from Move source: when the account exists, it cannot
abort, returns the old balance, removes the resource, and touches nothing
else. -/
theorem take_verified : Verified take 0 := by
  refine ⟨_, rfl, 3, ?_⟩
  intro m args current frames
  simp only [wpB, compileFun, compAnns, take, MProgram.toProgram,
    MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll, orAll,
    compileBlock, termCmds, termGoto, retExitBlock, abortExitBlock, initVStateAt, MoveState.locals, wpBlock,
    wpTerm, wpEdge, wpCmds, onOk, Option.elim, Option.map,
    List.mem_cons, List.not_mem_nil, or_false, List.find?,
    List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd,
    Nat.reduceEqDiff]
  intro htyped hreq gt hgt _
  rcases hgt with rfl
  simp only [reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
  -- Typing of the boundary state from the injected `WellFormed`
  -- assumption: the argument shape and the canonical form of `Account`
  -- resources in memory.
  simp only [typedEntry, TypedArgs, TypedMemory] at htyped
  obtain ⟨⟨hlen, hvalid⟩, hmem⟩ := htyped
  obtain ⟨v0, rfl⟩ : ∃ v, args = [v] := by
    cases args with
    | nil => simp at hlen
    | cons a as =>
      cases as with
      | nil => exact ⟨a, rfl⟩
      | cons b bs => simp at hlen
  have hv0 := hvalid 0 .address v0 rfl rfl
  simp only [isValid_address_iff] at hv0
  obtain ⟨a, rfl⟩ := hv0
  -- Canonical form of the account state at the target address.
  have hacct : m 0 a = none ∨
      ∃ b, b < U64_SIZE ∧ m 0 a = some (.struct [.u64 b]) := by
    match hm : m 0 a with
    | none => exact .inl rfl
    | some v =>
      have hval := hmem 0 { fields := [.u64] } a v rfl hm
      simp only [isValid_struct_iff] at hval
      obtain ⟨d, fs, hd, rfl, hfs⟩ := hval
      obtain rfl : d = { fields := [Ty.u64] } := by
        simp [MStruct.toStructDecl] at hd
        exact hd.symm
      simp only [isValidList_cons_iff, isValidList_nil_iff,
        isValid_u64_iff] at hfs
      obtain ⟨v', vs', rfl, ⟨b, rfl, hb⟩, rfl⟩ := hfs
      exact .inr ⟨b, hb, rfl⟩
  clear hmem hvalid hlen
  rcases hacct with habs | ⟨b, hb, hpres⟩
  · -- The precondition excludes an absent account.
    change m ({ resource := 0 } : ResourceKey) a = none at habs
    simp [Holds, VState.preEnvOf, preEnv, initLocals, SpecEnv.memAt] at hreq
    rw [habs] at hreq
    contradiction
  · -- Account present: normal path through the return exit.
    change m ({ resource := 0 } : ResourceKey) a =
      some (.struct [.u64 b]) at hpres
    intro gt hgt hg
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hgt
    rcases hgt with rfl | rfl
    · simp [initLocals, hpres, Oper.sem, flagSet,
        MoveState.writeLocals] at hg
    · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
        postEnv, initLocals, hpres, Oper.sem, SpecEnv.memAt,
        Contract.abortsFalse, memRemove,         MoveState.writeLocals, agreesOutside, Contract.footprint]
      intro r a' hout hr ha'
      exact absurd ha' (hout hr.symm)

end Tests.Prover.MoveSource
