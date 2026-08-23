-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Structured `while` / `loop` / `break` / `continue` / `return`, compiled
as in-function CFG loops and verified from retained source. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Loops where

  /-! ## Functions -/

  fun count_down (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    n

  spec count_down (n : U64) where
    ensures result = 0;
    aborts_if False

  fun count_down_loop (n : U64) : U64 := do
    let mut n := n
    loop
      if n < 1 then break
      n := n - 1
    n

  spec count_down_loop (n : U64) where
    ensures result = 0;
    aborts_if False

  fun skip_evens (n acc : U64) : U64 := do
    let mut n := n
    let mut acc := acc
    while 0 < n do
      n := n - 1
      if n % 2 == 0 then continue
      acc := acc + 1
    acc

  fun two_phases (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      n := n - 1
    while n < 3 do
      n := n + 1
    n

  spec two_phases (n : U64) where
    ensures result = 3;
    aborts_if False

  fun nested (x : U64) : U64 := do
    let mut x := x
    while 0 < x do
      while 10 < x do
        x := x - 10
      x := x - 1
    x

  fun labeled_exit (n : U64) : U64 := do
    let mut n := n
    loop@outer
      loop
        if n < 1 then break@outer
        n := n - 1
        break
    n

  spec labeled_exit (n : U64) where
    ensures result = 0;
    aborts_if False

  fun labeled_continue (n : U64) : U64 := do
    let mut n := n
    loop@outer
      loop
        if n < 1 then break@outer
        n := n - 1
        continue@outer
    n

  spec labeled_continue (n : U64) where
    ensures result = 0;
    aborts_if False

  fun labeled_proof : U64 := do
    loop@outer
      loop
        break@outer
    7

  spec labeled_proof where
    ensures result = 7;
    aborts_if False

  fun shadowed_loop_state (n : U64) : U64 := do
    loop
      let mut n : U64 := 2
      n := 1
      break
    n

  spec shadowed_loop_state (n : U64) where
    ensures result = n;
    aborts_if False

  fun shadowed_loop_arrow (n : U64) : Action U64 := do
    loop
      let mut n ← (pure 2 : Action U64)
      n := 1
      break
    pure n

  spec shadowed_loop_arrow (n : U64) where
    ensures result = n;
    aborts_if False

  /-- Reassigning a loop-entry binding through `←` must carry that binding
  to the loop exit. -/
  fun arrow_reassign_loop (n : U64) : Action U64 := do
    let mut n := n
    loop
      n ← pure 0
      break
    pure n

  struct Counter has Key where
    value : U64

  fun drain (addr : Address) : Action U64 := do
    let value ← &mut Counter[addr].value
    let mut n ← *value
    while 0 < n do
      n := n - 1
    value := n
    pure n

  spec drain (addr : Address) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = 0;
    aborts_if False

  fun early (flag : Bool) : Action U64 := do
    if flag then return 7
    pure 8

  spec early (flag : Bool) where
    ensures result = if flag then 7 else 8;
    aborts_if False

  fun return_in_loop (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      if n == 3 then return 1
      n := n - 1
    n

  partial fun countdown_tail (value accumulator : U64) : U64 :=
    if value < 1 then accumulator
    else continue countdown_tail (value - 1) (accumulator + 1)

  /-! ## Proofs -/

  verify count_down by
    contract_intro
    move_cases hloop : Move.Verify.Source.logicalLT 0 args
    ·
      rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    ·
      subst args
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]

  verify count_down_loop by
    contract_intro
    move_cases hloop : Move.Verify.Source.logicalLT args 1
    · have argsZero : args = 0 := Move.UInt.eq_zero_of_not_pos (by omega)
      subst argsZero
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]
    · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos (by omega),
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial

  /-- The second `two_phases` loop counts up to exactly three. -/
  private theorem upToThreeLoop (State : Type) :
      Move.Verify.Satisfies
        (Move.Semantics.Spec.fix fun recursive n =>
          if Move.Verify.Source.logicalLT n 3 then
            Move.Semantics.Spec.bind
              (Move.Semantics.Checked.addSpec n 1) recursive
          else
            Move.Semantics.Spec.pure n)
        ({ «requires» := fun n _ => n.toNat ≤ 3
           «ensures» := fun _ _ result _ => result = 3
           «aborts» := fun _ _ _ => False
           mayAbort := fun _ _ => False } :
          Move.Verify.Contract State U64 U64) := by
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial permitted
    abort_norm
    replace permitted : n.toNat ≤ 3 := permitted
    move_cases hloop : Move.Verify.Source.logicalLT n 3
    · spec_norm
      apply Move.Verify.wp_of_satisfies recursiveVerified
      show (Move.UInt.ofNat (n.toNat + 1)).toNat ≤ 3
      u64_omega
    · have nEq : n = 3 := by u64_omega
      subst nEq
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]

  verify two_phases by
    contract_intro
    by_cases hloop : Move.Verify.Source.logicalLT 0 args
    · rw [if_pos hloop]
      simp only [Move.Verify.Source.logicalLT_uint,
        Move.UInt.toNat_ofNat_numeral, move_norm, Nat.reducePow, Nat.reduceMod] at hloop
      rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    · rw [if_neg hloop]
      simp only [Move.Verify.Source.logicalLT_uint,
        Move.UInt.toNat_ofNat_numeral, move_norm, Nat.reducePow, Nat.reduceMod] at hloop
      refine Move.Verify.wp_mono
        (Move.Verify.wp_of_satisfies (upToThreeLoop _) ?_) ?_ ?_
      · show args.toNat ≤ 3
        omega
      · exact fun result final h => h
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
        ({ «requires» := fun _ _ => True
           «ensures» := fun _ _ result _ => result = 0
           «aborts» := fun _ _ _ => False
           mayAbort := fun _ _ => False } :
          Move.Verify.Contract State U64 U64) := by
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial _
    abort_norm
    move_cases hloop : Move.Verify.Source.logicalLT n 1
    · have nZero : n = 0 := Move.UInt.eq_zero_of_not_pos (by omega)
      subst nZero
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]
    · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos (by omega),
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial

  verify labeled_exit by
    contract_intro
    exact Move.Verify.wp_of_satisfies (countToZeroLoop _) trivial

  verify labeled_continue by
    contract_intro
    exact Move.Verify.wp_of_satisfies (countToZeroLoop _) trivial

  verify labeled_proof

  verify shadowed_loop_state

  verify shadowed_loop_arrow

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
        ({ «requires» := fun _ _ => True
           «ensures» := fun _ _ output _ => output = (0, reference.write 0)
           «aborts» := fun _ _ _ => False
           mayAbort := fun _ _ => False } :
          Move.Verify.Contract State U64
            (U64 × Move.Semantics.Mutation U64)) := by
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial _
    abort_norm
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
      refine ⟨by simp [← reconciled, Move.Semantics.ResourceStore.get], ?_⟩
      exact fun _ distinct => Move.Semantics.ResourceStore.lookup_insert_other
        _ _ _ _ distinct
    · exact fun code h => h.elim

  verify early

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Loops

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  private def fun? (name : String) : Option MoveModel.IR.FunDecl :=
    compiled.funDecl? name

  private def hasFunction (name : String) : Bool :=
    (fun? name).isSome

  private def hasDottedHelper (namePrefix : String) : Bool :=
    (List.range compiled.numFuns).any fun id =>
      (compiled.funMeta id).any fun info => info.name.startsWith (namePrefix ++ ".")

  private def invokes (callee : MoveModel.IR.FunId) : MoveModel.IR.Instr → Bool
    | .call _ (.function target) _ => target == callee
    | _ => false

  private def hasSelfCall (name : String) : Bool :=
    let id := compiled.funId name
    match compiled.program.funs id with
    | some decl => decl.blocksList.any fun block => block.instrs.any (invokes id)
    | none => false

  private def termSuccs : MoveModel.IR.Term → List Nat
    | .jump b => [b]
    | .branch _ t e => [t, e]
    | .ret _ => []
    | .abort _ => []

  private def backEdgeTargets (decl : MoveModel.IR.FunDecl) : List Nat :=
    decl.blocksList.zipIdx.foldl (init := []) fun acc (block, index) =>
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
        decl.blocksList.any fun block =>
          (termSuccs block.term).contains decl.body.entry
    | none => false

  private def hasLt : MoveModel.IR.Instr → Bool
    | .call _ .lt _ => true
    | _ => false

  private def headerTestsLt (name : String) : Bool :=
    match fun? name with
    | some decl =>
        (backEdgeTargets decl).any fun header =>
          match decl.body.blocks header with
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
        decl.blocksList.foldl (init := 0) fun n block =>
          match block.term with
          | .ret _ => n + 1
          | _ => n
    | none => 0

  #test run "count_down" [] [.u64 5] = Tests.okU64 0
  #test run "count_down" [] [.u64 0] = Tests.okU64 0
  #test run "count_down_loop" [] [.u64 5] = Tests.okU64 0
  #test run "skip_evens" [] [.u64 5, .u64 0] = Tests.okU64 2
  #test run "two_phases" [] [.u64 2] = Tests.okU64 3
  #test run "nested" [] [.u64 25] = Tests.okU64 0
  #test run "labeled_exit" [] [.u64 5] = Tests.okU64 0
  #test run "labeled_continue" [] [.u64 5] = Tests.okU64 0
  #test run "labeled_proof" [] [] = Tests.okU64 7
  #test run "shadowed_loop_state" [] [.u64 7] = Tests.okU64 7
  #test run "shadowed_loop_arrow" [] [.u64 7] = Tests.okU64 7
  #test run "arrow_reassign_loop" [] [.u64 3] = Tests.okU64 0
  #test run "drain" (memory 3 4) [.address 3]
    = Tests.okRet (memory 3 0) [.u64 0]
  #test run "early" [] [.bool true] = Tests.okU64 7
  #test run "early" [] [.bool false] = Tests.okU64 8
  #test run "return_in_loop" [] [.u64 5] = Tests.okU64 1
  #test run "return_in_loop" [] [.u64 2] = Tests.okU64 0
  #test run "countdown_tail" [] [.u64 100, .u64 40] = Tests.okU64 140

  #test hasFunction "count_down" = true
  #test hasDottedHelper "count_down" = false
  #test hasSelfCall "count_down" = false
  #test hasBackEdge "count_down" = true
  #test headerTestsLt "count_down" = true
  #test hasSelfCall "count_down_loop" = false
  #test hasBackEdge "count_down_loop" = true
  #test hasSelfCall "two_phases" = false
  #test backEdgeCount "two_phases" = 2
  #test hasSelfCall "nested" = false
  #test backEdgeCount "nested" = 2
  #test hasSelfCall "labeled_exit" = false
  #test hasBackEdge "labeled_exit" = true
  #test hasSelfCall "labeled_continue" = false
  #test hasBackEdge "labeled_continue" = true
  #test hasDottedHelper "labeled_proof" = false
  #test hasEntryBackEdge "countdown_tail" = true
  #test hasSelfCall "countdown_tail" = false
  #test (1 < retCount "return_in_loop") = true

end Tests.MovePrograms
