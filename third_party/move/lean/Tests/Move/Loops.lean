-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Structured `while` / `loop` / `break` / `continue` / `return`, compiled
as in-function CFG loops and verified from retained source. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Loops where

  /-! ## Functions -/

  fun countDown (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    n

  spec countDown (n : U64) where
    ensures result = 0;
    aborts_if False

  fun countDownLoop (n : U64) : U64 := do
    let mut n := n
    loop
      if n < 1 then break
      n := n - 1
    n

  spec countDownLoop (n : U64) where
    ensures result = 0;
    aborts_if False

  fun skipEvens (n acc : U64) : U64 := do
    let mut n := n
    let mut acc := acc
    while 0 < n do
      n := n - 1
      if n % 2 == 0 then continue
      acc := acc + 1
    acc

  fun twoPhases (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    while n < 3 do
      n := n + 1
    n

  spec twoPhases (n : U64) where
    ensures result = 3;
    aborts_if False

  fun nested (x : U64) : U64 := do
    let mut x := x
    while 0 < x do
      while 10 < x do
        x := x - 10
      x := x - 1
    x

  fun labeledExit (n : U64) : U64 := do
    let mut n := n
    loop@outer
      loop
        if n < 1 then break@outer
        n := n - 1
        break
    n

  spec labeledExit (n : U64) where
    ensures result = 0;
    aborts_if False

  fun labeledContinue (n : U64) : U64 := do
    let mut n := n
    loop@outer
      loop
        if n < 1 then break@outer
        n := n - 1
        continue@outer
    n

  spec labeledContinue (n : U64) where
    ensures result = 0;
    aborts_if False

  fun labeledProof : U64 := do
    loop@outer
      loop
        break@outer
    7

  spec labeledProof where
    ensures result = 7;
    aborts_if False

  fun shadowedLoopState (n : U64) : U64 := do
    loop
      let mut n : U64 := 2
      n := 1
      break
    n

  spec shadowedLoopState (n : U64) where
    ensures result = n;
    aborts_if False

  fun shadowedLoopArrow (n : U64) : Action U64 := do
    loop
      let mut n ← (pure 2 : Action U64)
      n := 1
      break
    pure n

  spec shadowedLoopArrow (n : U64) where
    ensures result = n;
    aborts_if False

  /-- Reassigning a loop-entry binding through `←` must carry that binding
  to the loop exit. -/
  fun arrowReassignLoop (n : U64) : Action U64 := do
    let mut n := n
    loop
      n ← pure 0
      break
    pure n

  @[move_struct]
  structure Counter where
    value : U64
    deriving Key

  fun drain (addr : Address) : Action U64 := do
    let value ← &mut Counter[addr].value
    let mut n ← *value
    while 0 < n do
      n := n - 1
    value := n
    pure n

  spec drain (addr : Address) where
    requires exists<Counter>(addr);
    ensures Counter[addr].value = 0;
    aborts_if False

  fun early (flag : Bool) : Action U64 := do
    if flag then return 7
    pure 8

  spec early (flag : Bool) where
    ensures result = if flag then 7 else 8;
    aborts_if False

  fun returnInLoop (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      if n == 3 then return 1
      n := n - 1
    n

  partial fun countdownTail (value accumulator : U64) : U64 :=
    if value < 1 then accumulator
    else continue countdownTail (value - 1) (accumulator + 1)

  /-! ## Proofs -/

  verify countDown by
    contract_intro
    move_cases hloop : Move.Verify.Source.logicalLT 0 args
    ·
      rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    ·
      subst args
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]

  verify countDownLoop by
    contract_intro
    move_cases hloop : Move.Verify.Source.logicalLT args 1
    · have argsZero : args = 0 := Move.U64.eq_zero_of_not_pos (by omega)
      subst argsZero
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]
    · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos (by omega),
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial

  /-- The second `twoPhases` loop counts up to exactly three. -/
  private theorem upToThreeLoop (State : Type) :
      Move.Verify.Satisfies
        (Move.Semantics.Spec.fix fun recursive n =>
          if Move.Verify.Source.logicalLT n 3 then
            Move.Semantics.Spec.bind
              (Move.Semantics.Checked.addSpec n 1) recursive
          else
            Move.Semantics.Spec.pure n)
        (@Move.Verify.Contract.mk State U64 U64
          (fun n _ => n.toNat ≤ 3)
          (fun _ initial result final => result = 3 ∧ final = initial)
          (fun _ _ _ => False)) := by
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial permitted
    replace permitted : n.toNat ≤ 3 := permitted
    move_cases hloop : Move.Verify.Source.logicalLT n 3
    · spec_norm
      apply Move.Verify.wp_of_satisfies recursiveVerified
      show (Move.U64.ofNat (n.toNat + 1)).toNat ≤ 3
      u64_omega
    · have nEq : n = 3 := by u64_omega
      subst nEq
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]

  verify twoPhases by
    contract_intro
    by_cases hloop : Move.Verify.Source.logicalLT 0 args
    · rw [if_pos hloop]
      simp only [Move.Verify.Source.logicalLT_u64,
        Move.U64.toNat_ofNat_numeral] at hloop
      rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    · rw [if_neg hloop]
      simp only [Move.Verify.Source.logicalLT_u64,
        Move.U64.toNat_ofNat_numeral] at hloop
      refine Move.Verify.wp_mono
        (Move.Verify.wp_of_satisfies (upToThreeLoop _) ?_) ?_ ?_
      · show args.toNat ≤ 3
        omega
      · exact fun result final h => h.1
      · exact fun code h => h.elim

  /-- Both labeled loop examples run one countdown to zero. -/
  private theorem countToZeroLoop (State : Type) :
      Move.Verify.Satisfies
        (Move.Semantics.Spec.fix fun recursive n =>
          if Move.Verify.Source.logicalLT n 1 then
            Move.Semantics.Spec.pure n
          else
            Move.Semantics.Spec.bind
              (Move.Semantics.Checked.subSpec n 1) recursive)
        (@Move.Verify.Contract.mk State U64 U64
          (fun _ _ => True)
          (fun _ _ result _ => result = 0)
          (fun _ _ _ => False)) := by
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial _
    move_cases hloop : Move.Verify.Source.logicalLT n 1
    · have nZero : n = 0 := Move.U64.eq_zero_of_not_pos (by omega)
      subst nZero
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]
    · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos (by omega),
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial

  verify labeledExit by
    contract_intro
    exact Move.Verify.wp_of_satisfies (countToZeroLoop _) trivial

  verify labeledContinue by
    contract_intro
    exact Move.Verify.wp_of_satisfies (countToZeroLoop _) trivial

  verify labeledProof

  verify shadowedLoopState

  verify shadowedLoopArrow

  /-- The `drain` loop counts the borrowed value down to zero and writes the
  final value back through the loan. -/
  private theorem drainLoop (State : Type)
      (reference : Move.Semantics.Mutation U64) :
      Move.Verify.Satisfies
        (Move.Semantics.Spec.fix fun recursive n =>
          if Move.Verify.Source.logicalLT 0 n then
            Move.Semantics.Spec.bind
              (Move.Semantics.Checked.subSpec n 1) recursive
          else
            Move.Semantics.Spec.pure (n, reference.write n))
        (@Move.Verify.Contract.mk State U64
          (U64 × Move.Semantics.Mutation U64)
          (fun _ _ => True)
          (fun _ initial output final =>
            output = (0, reference.write 0) ∧ final = initial)
          (fun _ _ _ => False)) := by
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial _
    move_cases hloop : Move.Verify.Source.logicalLT 0 n
    · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    · simp [Move.Verify.wp, Move.Semantics.Spec.pure]

  verify drain by
    contract_intro
    rcases Option.isSome_iff_exists.mp permitted with ⟨counter, lookup⟩
    rw [Move.Verify.wp_withBorrowMutFocusSpec]
    refine ⟨fun owner ownerLookup => ?_, fun missing => by simp_all⟩
    rw [Move.Verify.wp_withMutation]
    intro future
    refine Move.Verify.wp_mono
      (Move.Verify.wp_of_satisfies
        (drainLoop _ { current := owner.value, prophecy := future })
        trivial) ?_ ?_
    · rintro output final ⟨rfl, rfl⟩ reconciled
      simp only [Move.Semantics.Mutation.write] at reconciled ⊢
      simp [← reconciled, Move.Semantics.ResourceStore.get]
    · exact fun code h => h.elim

  verify early

  /-! ## Tests -/

  def compiled : MModule := move_module% "LoopsTest"

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  private def fun? (name : String) : Option MFun :=
    compiled.funs.find? (·.name == name)

  private def hasFunction (name : String) : Bool :=
    (fun? name).isSome

  private def hasDottedHelper (namePrefix : String) : Bool :=
    compiled.funs.any fun decl =>
      decl.name.startsWith (namePrefix ++ ".")

  private def invokes (callee : MoveModel.IR.FunId) : MoveModel.IR.Instr → Bool
    | .call _ (.function target) _ => target == callee
    | _ => false

  private def hasSelfCall (name : String) : Bool :=
    let id := compiled.funId name
    match compiled.funs[id]? with
    | some decl => decl.blocks.any fun block => block.instrs.any (invokes id)
    | none => false

  private def termSuccs : MoveModel.IR.Term → List Nat
    | .jump b => [b]
    | .branch _ t e => [t, e]
    | .ret _ => []
    | .abort _ => []

  private def backEdgeTargets (decl : MFun) : List Nat :=
    decl.blocks.zipIdx.foldl (init := []) fun acc (block, index) =>
      (termSuccs block.term).foldl (init := acc) fun acc target =>
        if target ≤ index && !acc.contains target then acc ++ [target] else acc

  private def hasBackEdge (name : String) : Bool :=
    match fun? name with
    | some decl => !(backEdgeTargets decl).isEmpty
    | none => false

  private def backEdgeCount (name : String) : Nat :=
    match fun? name with
    | some decl => (backEdgeTargets decl).length
    | none => 0

  private def hasEntryBackEdge (name : String) : Bool :=
    match fun? name with
    | some decl =>
        decl.blocks.any fun block =>
          (termSuccs block.term).contains decl.entry
    | none => false

  private def hasLt : MoveModel.IR.Instr → Bool
    | .call _ .lt _ => true
    | _ => false

  private def headerTestsLt (name : String) : Bool :=
    match fun? name with
    | some decl =>
        (backEdgeTargets decl).any fun header =>
          match decl.blocks[header]? with
          | some block =>
              block.instrs.any hasLt ||
                (match block.term with
                 | .branch _ _ _ => true
                 | _ => false)
          | none => false
    | none => false

  private def retCount (name : String) : Nat :=
    match fun? name with
    | some decl =>
        decl.blocks.foldl (init := 0) fun n block =>
          match block.term with
          | .ret _ => n + 1
          | _ => n
    | none => 0

  #test run "countDown" [] [.u64 5] = Tests.okU64 0
  #test run "countDown" [] [.u64 0] = Tests.okU64 0
  #test run "countDownLoop" [] [.u64 5] = Tests.okU64 0
  #test run "skipEvens" [] [.u64 5, .u64 0] = Tests.okU64 2
  #test run "twoPhases" [] [.u64 2] = Tests.okU64 3
  #test run "nested" [] [.u64 25] = Tests.okU64 0
  #test run "labeledExit" [] [.u64 5] = Tests.okU64 0
  #test run "labeledContinue" [] [.u64 5] = Tests.okU64 0
  #test run "labeledProof" [] [] = Tests.okU64 7
  #test run "shadowedLoopState" [] [.u64 7] = Tests.okU64 7
  #test run "shadowedLoopArrow" [] [.u64 7] = Tests.okU64 7
  #test run "arrowReassignLoop" [] [.u64 3] = Tests.okU64 0
  #test run "drain" (memory 3 4) [.address 3]
    = Tests.okRet (memory 3 0) [.u64 0]
  #test run "early" [] [.bool true] = Tests.okU64 7
  #test run "early" [] [.bool false] = Tests.okU64 8
  #test run "returnInLoop" [] [.u64 5] = Tests.okU64 1
  #test run "returnInLoop" [] [.u64 2] = Tests.okU64 0
  #test run "countdownTail" [] [.u64 100, .u64 40] = Tests.okU64 140

  #test hasFunction "countDown" = true
  #test hasDottedHelper "countDown" = false
  #test hasSelfCall "countDown" = false
  #test hasBackEdge "countDown" = true
  #test headerTestsLt "countDown" = true
  #test hasSelfCall "countDownLoop" = false
  #test hasBackEdge "countDownLoop" = true
  #test hasSelfCall "twoPhases" = false
  #test backEdgeCount "twoPhases" = 2
  #test hasSelfCall "nested" = false
  #test backEdgeCount "nested" = 2
  #test hasSelfCall "labeledExit" = false
  #test hasBackEdge "labeledExit" = true
  #test hasSelfCall "labeledContinue" = false
  #test hasBackEdge "labeledContinue" = true
  #test hasDottedHelper "labeledProof" = false
  #test hasEntryBackEdge "countdownTail" = true
  #test hasSelfCall "countdownTail" = false
  #test (1 < retCount "returnInLoop") = true

  #emit_leaner_xir compiled

end Tests.MovePrograms
