-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.IR.Interp.Exec
import MoveModel.Prover.Ivl.Wp
import MoveModel.Prover.Translate.Compile

/-!
# Examples Authored in masm

These examples embed the textual Move assembler syntax from `tools/move-asm`,
extended with specification clauses.  The `masm%` elaborator runs the real
assembler and stackless generator through `aptos move exchange`, then splices
the resulting program into Lean.

The file contains a loop (`count_down`), the TACAS'22 account example
(`withdraw`), and a typed quantifier (`quantId`).  The loop exercises a
branch, back edge, and the `wpB` invariant rule.

This module uses `masm%`, not `masmElim%`, so `withdraw` takes the value-level
`move_from`/`unpack`/`pack`/`move_to` route.  Because `move_to` requires a
`&signer`, the function receives both a signer and its address.  The
precondition `s == addr` connects them; signers are represented by addresses
at this level.  The examples are pinned by interpreter `#guard`s and verified
with the Lean weakest-precondition calculus.

The specifications contain no typing clauses.  Injected `WellFormed`
assumptions provide boundary-state typing (`typedEntry`) and typed loop
havoc.  These facts are enough to verify `count_down` without an explicit
loop invariant.
-/

namespace Tests.Prover.MasmSource

open MoveModel.Prover.Ivl
open MoveModel.IR
open MoveModel.Frontend.XIR
open MoveModel.Prover.Translate

def countDown : Program := masm% "
module 0x42::count_down

fun count_down(x: u64): u64
    ensures result == 0
l1: copy_loc x
    ld_u64 0
    gt
    br_false l2
    copy_loc x
    ld_u64 1
    sub
    st_loc x
    branch l1
l2: move_loc x
    ret
"

-- Evaluation: `count_down 5` returns `0` and does not touch memory.
#guard interpFun countDown 100 0 [] [.u64 5] matches .ok (.ret ⟨_, []⟩ [.u64 0])

-- Evaluation: insufficient fuel is reported.
#guard interpFun countDown 3 0 [] [.u64 5] matches .error .outOfFuel

set_option maxHeartbeats 1000000 in
/-- **`count_down` verifies**: the verification condition of the program
compiled from the embedded masm — with the invariant rule at the loop
header — holds via the Lean WP calculus. -/
theorem masm_count_down_verified : Verified countDown 0 := by
  refine ⟨_, rfl, 5, ?_⟩
  intro m args current frames
  simp only [wpB, compileFun, compAnns, countDown, MProgram.toProgram,
    MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll, orAll,
    denoteLoopSpec, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVStateAt, MoveState.locals, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    List.cons_append, List.nil_append, List.mem_cons, List.not_mem_nil,
    or_false, List.find?, List.getElem?_cons_zero,
    List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd, Nat.reduceSub,
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
  constructor
  · -- Base case: the (trivial) invariant holds at loop entry.
    exact ⟨rfl, (by
      intro i t v ht hv
      apply hvalid i t v ht
      simpa [initLocals] using hv), hTM, by simp [Holds, andAll]⟩
  · -- Inductive case: the multisorted havoc keeps `x` a well-formed
    -- `u64`.
    intro s' hT hInv
    obtain ⟨hsnaps, hargs, -, htyv, hmem, -⟩ := hT
    obtain ⟨hab, hTL, hTM', -⟩ := hInv
    obtain ⟨val, hl0, hval⟩ :=
      htyv 0 .u64 (by simp) rfl
        ⟨.u64 n, by simp [initLocals], .u64 hn⟩
    simp only [isValid_u64_iff] at hval
    obtain ⟨k, rfl, hk⟩ := hval
    intro gt2 hgt2 hg2
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hgt2
    rcases hgt2 with rfl | rfl | rfl
    · -- abort-exit edge: the flag is clear
      simp [hab, hl0, Oper.sem,         MoveState.writeLocals, flagSet] at hg2
    · -- loop-body edge (0 < k): decrement re-establishes the invariant
      simp [hab, hl0, Oper.sem,         MoveState.writeLocals] at hg2
      have hk1 : 1 ≤ k := hg2
      have hTL1 := hTL.writeU64 (x := 1) (n := k) (by simp) hk
      have hTL2 := hTL1.writeU64 (x := 2) (n := 0) (by simp)
        (by simp [U64_SIZE])
      have hTL3 := hTL2.writeBool (x := 3) (b := true) (by simp)
      have hTL4 := hTL3.writeU64 (x := 4) (n := k) (by simp) hk
      have hTL5 := hTL4.writeU64 (x := 5) (n := 1) (by simp)
        (by simp [U64_SIZE])
      have hTL6 := hTL5.writeU64 (x := 6) (n := k - 1) (by simp) (by omega)
      have hTL7 := hTL6.writeU64 (x := 0) (n := k - 1) (by simp) (by omega)
      -- checked arithmetic guards on the unbounded `Int` value now, so the
      -- range facts have to be supplied in that form
      have hk1' : (1 : Int) ≤ (k : Int) := by exact_mod_cast hk1
      have hkSub : (k : Int) - 1 < (U64_SIZE : Int) := by
        have h := hk; omega
      simp [wpCmds, compileInstr, onOk, hab, hl0, hg2, hk1', hkSub,
        Oper.sem, MoveState.writeLocals, Holds,
        VState.curEnv, VState.doAbort]
      intro g' b' hedge hg'
      rcases hedge with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · simp [flagSet] at hg'
      · -- back edge: the (trivial) invariant is re-established
        refine ⟨rfl, ?_, by simpa using hTM', by simp [andAll]⟩
        -- `k - 1` is truncated `Nat` subtraction in the typing chain and
        -- unbounded `Int` subtraction in the goal; they agree since `1 ≤ k`
        have hcast : ((k - 1 : Nat) : Int) = (k : Int) - 1 := by omega
        simpa [MoveState.locals, Value.u64, hcast] using hTL7
    · -- exit edge (k = 0): the exit assertions hold
      simp [hab, hl0, Oper.sem,         MoveState.writeLocals] at hg2
      simp [wpCmds, compileInstr, onOk, hab, hl0, hg2,
        Oper.sem, MoveState.writeLocals, Holds,
        VState.preEnvOf, VState.postEnvOf, VState.curEnv, VState.doAbort,
        preEnv, postEnv, agreesOutside, Contract.footprint,
        Contract.abortsFalse, Contract.abortsHolds, hsnaps, hargs]
      intro g' b' hedge hg'
      rcases hedge with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · simp [flagSet] at hg'
      · simp [wpCmds]
        intro r a'
        exact hmem r a' (fun h => nomatch h)

def account : Program := masm% "
module 0x42::account

struct Account has key
  balance: u64

fun withdraw(s: &signer, addr: address, amount: u64) acquires Account
    local acc: Account
    requires s == addr
    aborts_if !old(exists<Account>(addr)) || old(global<Account>(addr).balance) < amount
    ensures global<Account>(addr).balance == old(global<Account>(addr).balance) - amount
    modifies global<Account>(addr)
    copy_loc addr
    move_from Account
    unpack Account
    copy_loc amount
    sub
    pack Account
    st_loc acc
    copy_loc s
    move_loc acc
    move_to Account
    ret
"

-- Evaluation, normal path: withdrawing 4 from a balance of 10 leaves 6.
#guard interpFun account 100 0 [(0, 3, .struct [.u64 10])]
    [.address 3, .address 3, .u64 4]
  matches .ok (.ret ⟨_, [(0, 3, .struct [.u64 6])]⟩ [])

-- Evaluation, abort: no account at the address.
#guard interpFun account 100 0 [] [.address 3, .address 3, .u64 4]
  matches .ok (.abort _ 0)

-- Evaluation, abort: insufficient balance.
#guard interpFun account 100 0 [(0, 3, .struct [.u64 2])]
    [.address 3, .address 3, .u64 4]
  matches .ok (.abort _ 0)

-- The stepping kit below uses one uniform simp list per step; the linter
-- would have each list minimized per instruction, which hurts uniformity.
set_option linter.unusedSimpArgs false in
set_option maxHeartbeats 8000000 in
/-- **`withdraw` verifies**: the verification condition of the program
compiled from the embedded masm holds via the Lean WP calculus, for every
boundary state. -/
theorem masm_withdraw_verified : Verified account 0 := by
  refine ⟨_, rfl, 3, ?_⟩
  intro m args current frames
  simp only [wpB, compileFun, compAnns, account, MProgram.toProgram,
    MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll, orAll,
    compileBlock, termCmds, termGoto, retExitBlock,
    abortExitBlock, initVStateAt, MoveState.locals, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
    Option.elim, Option.map,
    List.mem_cons, List.not_mem_nil,
    or_false, List.find?, List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd, Nat.reduceEqDiff]
  intro htyped hreq gt hgt _
  rcases hgt with rfl
  simp only [reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
  -- Typing of the boundary state from the injected `WellFormed`
  -- assumption: argument shapes and the canonical form of `Account`
  -- resources in memory.
  simp only [typedEntry, TypedArgs, TypedMemory] at htyped
  obtain ⟨⟨hlen, hvalid⟩, hmem⟩ := htyped
  obtain ⟨v0, v1, v2, rfl⟩ : ∃ u v w, args = [u, v, w] := by
    cases args with
    | nil => simp at hlen
    | cons a as =>
      cases as with
      | nil => simp at hlen
      | cons b bs =>
        cases bs with
        | nil => simp at hlen
        | cons c cs =>
          cases cs with
          | nil => exact ⟨a, b, c, rfl⟩
          | cons d ds => simp at hlen
  have hv0 := hvalid 0 (.ref .signer) v0 rfl rfl
  have hv1 := hvalid 1 .address v1 rfl rfl
  have hv2 := hvalid 2 .u64 v2 rfl rfl
  simp only [isValid_ref_iff, isValid_signer_iff] at hv0
  simp only [isValid_address_iff] at hv1
  simp only [isValid_u64_iff] at hv2
  obtain ⟨a0, rfl⟩ := hv0
  obtain ⟨a, rfl⟩ := hv1
  obtain ⟨amt, rfl, -⟩ := hv2
  -- `requires s == addr` ties the signer to the address.
  simp [Holds, VState.preEnvOf, preEnv, initLocals] at hreq
  obtain rfl : a = a0 := hreq.symm
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
  clear hmem hvalid hlen hreq
  rcases hacct with habs | ⟨b, hb, hpres⟩
  · -- Account absent: `move_from` aborts; the abort exit's assert needs
    -- the first `aborts_if` disjunct.
    change m { resource := 0, typeArgs := [] } a = none at habs
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem]
    refine (wpCmds_onOk_step rfl).mpr ?_
    simp [initLocals, Oper.sem, habs, VState.doAbort]
    iterate 9 refine (wpCmds_onOk_skip rfl).mpr ?_
    refine ⟨fun _ => ?_, fun hclear => ?_⟩
    · simp [wpCmds, Holds, VState.preEnvOf, VState.doAbort, preEnv,
        abortEnv, postEnv, initLocals, habs, Oper.sem, SpecEnv.memAt,
        Contract.abortsHolds, MoveState.writeLocal]
    · simp [flagClear, VState.doAbort] at hclear
  · change m { resource := 0, typeArgs := [] } a = some (.struct [.u64 b]) at hpres
    rcases Nat.lt_or_ge b amt with hlt | hge
    · -- Insufficient balance: `sub` aborts; second `aborts_if` disjunct.
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, hpres, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        Nat.not_le.mpr hlt, VState.doAbort]
      iterate 6 refine (wpCmds_onOk_skip rfl).mpr ?_
      refine ⟨fun _ => ?_, fun hclear => ?_⟩
      · simp [wpCmds, Holds, VState.preEnvOf, VState.doAbort, preEnv,
          abortEnv, postEnv, initLocals, hpres, Oper.sem, SpecEnv.memAt,
          Contract.abortsHolds, MoveState.writeLocals,
          Nat.not_le.mpr hlt, hlt]
      · simp [flagClear, VState.doAbort] at hclear
    · -- Sufficient balance: normal path through the return exit.
      -- checked subtraction guards on the unbounded `Int` value now
      have hgeI : (amt : Int) ≤ (b : Int) := by exact_mod_cast hge
      have hsubLo : (0 : Int) ≤ (b : Int) - (amt : Int) := by omega
      have hsubHi : (b : Int) - (amt : Int) < (U64_SIZE : Int) := by
        have h := hb; omega
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, hpres, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals, hge, hgeI, hsubLo, hsubHi]
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
      simp [initLocals, Oper.sem, MoveState.writeLocals, hpres, memRemove,
        memWrite]
      refine (wpCmds_onOk_step rfl).mpr ?_
      simp [initLocals, Oper.sem, MoveState.writeLocals,
        MoveState.writeLocal]
      refine ⟨fun hset => ?_, fun _ => ?_⟩
      · simp [flagSet] at hset
      · simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
          abortEnv, postEnv, initLocals, hpres, Oper.sem,
          SpecEnv.memAt, Contract.abortsFalse,
          hge, memWrite, memRemove, MoveState.writeLocals, agreesOutside, Contract.footprint]
        intro r a' hout
        have hcond : ¬(r = { resource := 0 } ∧ a' = a) := fun ⟨hr, ha'⟩ =>
          hout hr.symm ha'
        rw [if_neg hcond, if_neg hcond]

def quantId : Program := masm% "
module 0x42::quant

fun id(x: u64): u64
    ensures result == x
    ensures forall y: u64 . 0 <= y
    move_loc x
    ret
"

-- Evaluation: the identity.
#guard interpFun quantId 100 0 [] [.u64 7] matches .ok (.ret ⟨_, []⟩ [.u64 7])

set_option maxHeartbeats 1000000 in
/-- **`id` verifies, with a typed quantifier**: `forall y: u64 . 0 <= y`
holds precisely because the binder's declared domain bounds the range to
the well-formed `u64` values (`IsValid`) — over the unbounded spec
integers the body would be false.  This is the semantic content of the
typed quantifier encoding (Boogie's typed `forall` plus the
`$IsValid'u64'` range guard). -/
theorem masm_quant_id_verified : Verified quantId 0 := by
  refine ⟨_, rfl, 3, ?_⟩
  intro m args current frames
  simp only [wpB, compileFun, compAnns, quantId, MProgram.toProgram,
    MFun.toFunDecl, MLoop.toLoopSpec, MContract.toContract, andAll, orAll,
    compileBlock, termCmds, termGoto, retExitBlock, abortExitBlock, initVStateAt, MoveState.locals,
    wpBlock, wpTerm, wpEdge, wpCmds, onOk, Option.elim, Option.map,
    List.mem_cons, List.not_mem_nil, or_false, List.find?,
    reduceIte, Nat.reduceAdd, Nat.reduceEqDiff]
  intro htyped _hreq gt hgt _
  rcases hgt with rfl
  simp only [reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
  -- Typing of the argument from the injected `WellFormed` assumption.
  simp only [typedEntry, TypedArgs] at htyped
  obtain ⟨⟨hlen, hvalid⟩, -⟩ := htyped
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
  intro gt2 hgt2 hg2
  simp only [List.mem_cons, List.not_mem_nil, or_false, List.length_cons,
    List.length_nil, Nat.reduceAdd] at hgt2
  rcases hgt2 with rfl | rfl
  · -- abort exit: unreachable, the flag stays clear
    simp [initLocals, flagSet] at hg2
  · -- return exit: both `ensures` clauses hold
    simp [wpCmds, Holds, VState.preEnvOf, VState.postEnvOf, preEnv,
      postEnv, initLocals, agreesOutside, Contract.footprint,
      Contract.abortsFalse, MoveState.writeLocal]
    rintro v x rfl hlo -
    exact ⟨x, by simp, by simpa using hlo⟩

end Tests.Prover.MasmSource
