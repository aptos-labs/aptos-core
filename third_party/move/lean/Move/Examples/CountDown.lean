-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.Wp
import Move.IR.Semantics
import Move.Prover.Translate.Compile

/-!
# A Loop Example: `count_down`

A minimal looping function, exercising the parts of the pipeline the
straight-line account example does not: a multi-block CFG with a `branch`
terminator, a back edge, a loop-header annotation, and the invariant rule
of `wpB`.

```move
fun count_down(n: u64): u64 {
    while (0 < n) { n = n - 1 };
    n
}
spec count_down {
    aborts_if false;
    ensures result == 0;
}
```

The CFG follows code layout order (the back edge targets the
lower-numbered header):

```
B0 (header):  t1 := 0; t2 := (t1 < t0); branch t2 B1 B2
B1 (body):    t3 := 1; t0 := t0 - t3;   jump B0     // back edge
B2 (exit):    ret t0
```

The declared loop targets havoc all locals (`valTargets` is the full
set) and no memory.  No loop invariant is needed: the multisorted havoc
leaves `t0` defined and well-formed for its declared type `u64`, and the
loop guard supplies `0 < t0` on the body path and `t0 = 0` on the exit
path — typing alone carries the proof.
-/

namespace Move.Examples.CountDown

open Move.Prover.Ivl
open Move.IR
open Move.Prover.Translate

def countDownBody : Cfg where
  blocks := fun b =>
    if b = 0 then
      some ⟨[.load 1 (.u64 0), .call [2] .lt [1, 0]], .branch 2 1 2⟩
    else if b = 1 then
      some ⟨[.load 3 (.u64 1), .call [0] .sub [0, 3]], .jump 0⟩
    else if b = 2 then
      some ⟨[], .ret [0]⟩
    else none
  entry := 0
  size := 3

def countDownLoopSpec : LoopSpec where
  inv := .value (.bool true)
  valTargets := fun _ => True
  memTargets := fun _ => False
  members := fun b => b = 0 ∨ b = 1

def countDownContract : Contract where
  requires := .value (.bool true)
  aborts := some (.value (.bool false))
  ensures := .binop .eq (.result 0) (.value (.u64 0))
  modifies := []

def countDownDecl : FunDecl where
  numParams := 1
  numLocals := 4
  locals := fun t => [Ty.u64, .u64, .bool, .u64][t]?
  returns := [.u64]
  body := countDownBody
  loopSpecs := fun b => if b = 0 then some countDownLoopSpec else none
  contract := countDownContract

def prog : Program where
  funs := fun f => if f = 0 then some countDownDecl else none
  structs := fun _ => none

set_option maxHeartbeats 1000000 in
/-- **`count_down` verifies**: the compiled verification condition — with
the invariant rule at the loop header — holds via the Lean WP calculus. -/
theorem count_down_verified : Verified prog 0 := by
  refine ⟨countDownDecl, by simp [prog], 5, ?_⟩
  intro m args
  simp only [wpB, compileFun, compAnns, countDownDecl, countDownBody,
    countDownContract, countDownLoopSpec, compileBlock, termCmds, termGoto, compileInstr,
    retExitBlock, abortExitBlock, initVState, wpBlock, wpTerm, wpEdge,
    wpCmds, onOk, denoteLoopSpec, Option.elim, Option.map, List.map,
    List.flatten, List.append, List.cons_append, List.nil_append,
    List.mem_cons, List.not_mem_nil, or_false, reduceIte, Nat.reduceAdd,
    Nat.reduceSub, Nat.reduceEqDiff]
  intro htyped _hreq gt hgt _
  rcases hgt with rfl
  simp only [wpCmds, onOk, compileInstr, List.map, List.flatten,
    List.append, List.cons_append, List.nil_append, List.mem_cons,
    List.not_mem_nil, or_false, List.length_cons, List.length_nil,
    reduceIte, Nat.reduceSub, Nat.reduceLT, Nat.reduceEqDiff]
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
  constructor
  · -- Base case: the (trivial) invariant holds at loop entry.
    exact ⟨trivial, by simp [Holds]⟩
  · -- Inductive case: an arbitrary target-related state passes the header;
    -- the multisorted havoc keeps `t0` a well-formed `u64`.
    intro s' hT hInv
    obtain ⟨hsnaps, hargs, -, htyv, hmem, -⟩ := hT
    obtain ⟨hab, -⟩ := hInv
    obtain ⟨val, hl0, hval⟩ :=
      htyv 0 .u64 trivial rfl ⟨.u64 n, rfl, .u64 hn⟩
    simp only [isValid_u64_iff] at hval
    obtain ⟨k, rfl, hk⟩ := hval
    intro gt2 hgt2 hg2
    rcases hgt2 with rfl | rfl | rfl
    · -- abort-exit edge: the flag is clear
      simp [hab, hl0, Oper.sem, MoveState.writeLocal,
        MoveState.writeLocals, flagSet] at hg2
    · -- loop-body edge (0 < k): decrement re-establishes the invariant at
      -- the back edge
      simp [hab, hl0, Oper.sem, MoveState.writeLocal,
        MoveState.writeLocals] at hg2
      have hk1 : 1 ≤ k := hg2
      simp [wpCmds, compileInstr, onOk, hab, hl0, hg2, hk1,
        Oper.sem, MoveState.writeLocal, MoveState.writeLocals, Holds,
        VState.curEnv, VState.doAbort]
      intro g' b' hedge hg'
      rcases hedge with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · simp [flagSet] at hg'
      · -- back edge: the (trivial) invariant is re-established
        simp
    · -- exit edge (k = 0): return; the exit assertions hold
      simp [hab, hl0, Oper.sem, MoveState.writeLocal,
        MoveState.writeLocals] at hg2
      simp [wpCmds, hab, hl0, hg2,
        Oper.sem, MoveState.writeLocal, MoveState.writeLocals, Holds,
        VState.preEnvOf, VState.postEnvOf, VState.curEnv, VState.doAbort,
        preEnv, postEnv, agreesOutside, Contract.footprint,
        Contract.abortsFalse, Contract.abortsHolds, hsnaps, hargs]
      intro g' b' hedge hg'
      rcases hedge with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · simp [flagSet] at hg'
      · -- return exit: `result = 0` and the (empty-footprint) frame
        simp [wpCmds]
        intro r a'
        exact hmem r a' (fun h => h)

end Move.Examples.CountDown
