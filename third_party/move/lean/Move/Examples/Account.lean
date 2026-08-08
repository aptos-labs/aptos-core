-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.Wp
import Move.IR.Semantics
import Move.Prover.Translate.Compile

/-!
# The Account Example (TACAS'22, Fig. 1/2)

The running example of the TACAS'22 paper, in bytecode: an
`Account` resource holding a `u64` balance, and a `withdraw` function

```move
fun withdraw(account: address, amount: u64) acquires Account {
    let balance = &mut borrow_global_mut<Account>(account).balance;
    *balance = *balance - amount;   // aborts on underflow
}
spec withdraw {
    aborts_if !exists<Account>(account);
    aborts_if global<Account>(account).balance < amount;
    ensures global<Account>(account).balance
              == old(global<Account>(account).balance) - amount;
    modifies global<Account>(account);
}
```

after reference elimination (read / compute / write-back), as a one-block
CFG of three-address instructions.  The contract is written in the deep
spec-expression language; `old(..)` appears as a `global` access against
the `preLabel` snapshot, exactly the post-`SaveMem` form.  The theorem
`withdraw_verified` discharges the compiled verification condition through
the Lean WP calculus — the end-to-end sanity check that the definitions are
usable, not merely well-typed.

Well-typedness of the boundary state — the arguments have the declared
types, `Account` resources in memory are one-field structs holding a
`u64` — is *not* part of the contract: it comes from the injected
`WellFormed` assumptions (`typedEntry`), derived from the declared types,
exactly as in the real prover's multisorted encoding.
-/

namespace Move.Examples.Account

open Move.Prover.Ivl
open Move.IR
open Move.Prover.Translate

/-- The `Account` resource type. -/
def ACCOUNT : ResourceId := 0

/-- `global<Account>(account).balance` in the current state. -/
def curBal : SpecExp := .select 0 (.global ACCOUNT none (.loc 0))

/-- `old(global<Account>(account).balance)`: the `preLabel` snapshot. -/
def oldBal : SpecExp := .select 0 (.global ACCOUNT (some preLabel) (.loc 0))

/-- The specification of `withdraw` from the paper (see module docs). -/
def withdrawContract : Contract where
  requires := .value (.bool true)
  aborts := some
    (.binop .or
      (.not (.exists_ ACCOUNT none (.loc 0)))
      (.binop .lt curBal (.loc 1)))
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
             .call [4] .sub [3, 1],
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
  structs := fun r => if r = ACCOUNT then some ⟨[.u64]⟩ else none

set_option maxHeartbeats 8000000 in
/-- **`withdraw` verifies**: the compiled verification condition holds via
the Lean WP calculus, for every boundary state. -/
theorem withdraw_verified : Verified prog 0 := by
  refine ⟨withdrawDecl, by simp [prog], 3, ?_⟩
  intro m args
  simp only [wpB, compileFun, compAnns, withdrawDecl, withdrawBody,
    withdrawContract, compileBlock, termCmds, termGoto, compileInstr, retExitBlock,
    abortExitBlock, initVState, wpBlock, wpTerm, wpEdge, wpCmds, onOk,
    Option.elim, Option.map, List.map, List.flatten, List.append,
    List.length_cons, List.length_nil,
    reduceIte, Nat.reduceAdd, Nat.reduceEqDiff]
  intro htyped _hreq gt hgt _
  simp only [List.mem_singleton] at hgt
  subst hgt
  simp only [wpCmds, onOk, compileInstr, List.map, List.flatten,
    List.append, List.cons_append, List.nil_append, List.mem_cons,
    List.not_mem_nil, or_false, reduceIte, Nat.reduceSub, Nat.reduceLT,
    Nat.reduceEqDiff]
  -- Typing of the boundary state from the injected `WellFormed`
  -- assumption: argument shapes and the canonical form of `Account`
  -- resources in memory.
  simp only [typedEntry, TypedArgs, TypedMemory] at htyped
  obtain ⟨⟨hlen, hvalid⟩, hmem⟩ := htyped
  obtain ⟨v0, v1, rfl⟩ : ∃ v w, args = [v, w] := by
    cases args with
    | nil => simp at hlen
    | cons a as =>
      cases as with
      | nil => simp at hlen
      | cons b bs =>
        cases bs with
        | nil => exact ⟨a, b, rfl⟩
        | cons c cs => simp at hlen
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
      have hval := hmem ACCOUNT ⟨[.u64]⟩ a v rfl hm
      simp only [isValid_struct_iff] at hval
      obtain ⟨d, fs, hd, rfl, hfs⟩ := hval
      obtain rfl : d = ⟨[Ty.u64]⟩ := by
        simp [prog, ACCOUNT] at hd
        exact hd.symm
      simp only [isValidList_cons_iff, isValidList_nil_iff,
        isValid_u64_iff] at hfs
      obtain ⟨v', vs', rfl, ⟨b, rfl, hb⟩, rfl⟩ := hfs
      exact .inr ⟨b, hb, rfl⟩
  clear hmem hvalid hlen _hreq
  rcases hacct with habs | ⟨b, hb, hpres⟩
  · -- Account absent: `get_global` aborts; only the abort exit is enabled,
    -- and its assert needs the first `aborts_if` disjunct.
    intro gt hgt hg
    rcases hgt with rfl | rfl
    · simp [wpCmds, Holds, VState.preEnvOf, VState.doAbort,
        preEnv, initLocals, habs, Oper.sem, SpecEnv.memAt,
        Contract.abortsHolds, curBal]
    · simp [initLocals, habs, Oper.sem, VState.doAbort,
        flagClear] at hg
  · rcases Nat.lt_or_ge b amt with hlt | hge
    · -- Insufficient balance: `sub` aborts; the abort exit's assert needs
      -- the second `aborts_if` disjunct.
      intro gt hgt hg
      rcases hgt with rfl | rfl
      · simp [wpCmds, Holds, VState.preEnvOf,
          VState.doAbort, preEnv, initLocals, hpres, Oper.sem,
          SpecEnv.memAt, Contract.abortsHolds, curBal,
          MoveState.writeLocals, MoveState.writeLocal,
          Nat.not_le.mpr hlt, hlt]
      · simp [initLocals, hpres, Oper.sem, VState.doAbort,
          flagClear, MoveState.writeLocals, MoveState.writeLocal,
          Nat.not_le.mpr hlt] at hg
    · -- Sufficient balance: normal path through the return exit.
      intro gt hgt hg
      rcases hgt with rfl | rfl
      · simp [initLocals, hpres, Oper.sem, flagSet, hge,
          MoveState.writeLocals, MoveState.writeLocal] at hg
      · simp [wpCmds, Holds, VState.preEnvOf,
          VState.postEnvOf, preEnv, postEnv, initLocals, hpres,
          Oper.sem, SpecEnv.memAt, Contract.abortsFalse,
          curBal, oldBal, hge, memWrite,
          MoveState.writeLocal, MoveState.writeLocals, agreesOutside,
          Contract.footprint]
        refine ⟨by omega, ?_⟩
        intro r a' hout hr ha'
        exact absurd ha' (hout hr.symm)

end Move.Examples.Account
