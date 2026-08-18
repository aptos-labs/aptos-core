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
    unfold countDownLoop.contract countDownLoop.sourceSpec
    intro State
    simp only [Move.Semantics.Spec.pure_bind]
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified
    intro n initial _
    by_cases hloop : Move.Verify.Source.logicalLT n 1
    · simp only [hloop, if_true]
      simp only [Move.Verify.Source.logicalLT_u64] at hloop
      change n.toNat < 1 at hloop
      have nzero := Move.U64.eq_zero_of_not_pos (by omega : ¬0 < n.toNat)
      subst n
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]
    · simp only [hloop, if_false]
      simp only [Move.Verify.Source.logicalLT_u64] at hloop
      change ¬n.toNat < 1 at hloop
      have positive : 0 < n.toNat := by omega
      rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos positive,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial

  verify twoPhases by
    unfold twoPhases.contract twoPhases.sourceSpec
    intro State
    simp only [Move.Semantics.Spec.pure_bind]
    let upContract : Move.Verify.Contract State U64 U64 :=
      @Move.Verify.Contract.mk State U64 U64
        (fun n _ => n.toNat ≤ 3)
        (fun _ initial result final => result = 3 ∧ final = initial)
        (fun _ _ _ => False)
    have upVerified :
        Move.Verify.Satisfies
          (Move.Semantics.Spec.fix fun recursive n =>
            if Move.Verify.Source.logicalLT n 3 then
              Move.Semantics.Spec.bind
                (Move.Semantics.Checked.addSpec n 1) recursive
            else
              Move.Semantics.Spec.pure n)
          upContract := by
      apply Move.Verify.satisfies_fix_of_wp
      intro recursive recursiveVerified n initial permitted
      by_cases hloop : Move.Verify.Source.logicalLT n 3
      · simp only [hloop, if_true]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change n.toNat < 3 at hloop
        have safe : n.toNat + 1 < U64.size := by
          simp [U64.size]
          omega
        have addStep :
            (Move.Semantics.Checked.addSpec n 1 :
              Move.Semantics.Spec State U64) =
              Move.Semantics.Spec.pure
                (Move.U64.ofNat (n.toNat + 1)) :=
          Move.Semantics.Checked.addSpec_eq_pure safe
        rw [addStep, Move.Semantics.Spec.pure_bind]
        apply Move.Verify.wp_of_satisfies recursiveVerified
        change (Move.U64.ofNat (n.toNat + 1)).toNat ≤ 3
        simp [Move.U64.toNat_ofNat]
        omega
      · simp only [hloop, if_false]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change ¬n.toNat < 3 at hloop
        have nequals : n = 3 := by
          apply Move.U64.ext
          change n.toNat = 3
          omega
        subst n
        simp [Move.Verify.wp, Move.Semantics.Spec.pure, upContract]
    apply Move.Verify.satisfies_fix_of_wp
    intro recursive recursiveVerified n initial _
    by_cases hloop : Move.Verify.Source.logicalLT 0 n
    · simp only [hloop, if_true]
      simp only [Move.Verify.Source.logicalLT_u64] at hloop
      change 0 < n.toNat at hloop
      rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
        Move.Semantics.Spec.pure_bind]
      exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    · simp only [hloop, if_false]
      simp only [Move.Verify.Source.logicalLT_u64] at hloop
      change ¬0 < n.toNat at hloop
      have nzero := Move.U64.eq_zero_of_not_pos hloop
      subst n
      have verified := Move.Verify.wp_of_satisfies
        (args := (0 : U64)) (initial := initial) upVerified (by
          change (0 : U64).toNat ≤ 3
          decide)
      constructor
      · intro result final execution
        exact (verified.1 result final execution).1
      · exact verified.2

  verify labeledExit by
    unfold labeledExit.contract labeledExit.sourceSpec
    intro State
    simp only [Move.Semantics.Spec.pure_bind]
    let loopContract : Move.Verify.Contract State U64 U64 :=
      @Move.Verify.Contract.mk State U64 U64
        (fun _ _ => True)
        (fun _ _ result _ => result = 0)
        (fun _ _ _ => False)
    have loopVerified :
        Move.Verify.Satisfies
          (Move.Semantics.Spec.fix fun recursive n =>
            if Move.Verify.Source.logicalLT n 1 then
              Move.Semantics.Spec.pure n
            else
              Move.Semantics.Spec.bind
                (Move.Semantics.Checked.subSpec n 1) recursive)
          loopContract := by
      apply Move.Verify.satisfies_fix_of_wp
      intro recursive recursiveVerified n initial _
      by_cases hloop : Move.Verify.Source.logicalLT n 1
      · simp only [hloop, if_true]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change n.toNat < 1 at hloop
        have nzero := Move.U64.eq_zero_of_not_pos (by omega : ¬0 < n.toNat)
        subst n
        simp [Move.Verify.wp, Move.Semantics.Spec.pure, loopContract]
      · simp only [hloop, if_false]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change ¬n.toNat < 1 at hloop
        have positive : 0 < n.toNat := by omega
        rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos positive,
          Move.Semantics.Spec.pure_bind]
        exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    apply Move.Verify.satisfies_fix_of_wp
    intro _ _ n initial _
    exact Move.Verify.wp_of_satisfies loopVerified trivial

  verify labeledContinue by
    unfold labeledContinue.contract labeledContinue.sourceSpec
    intro State
    simp only [Move.Semantics.Spec.pure_bind]
    let loopContract : Move.Verify.Contract State U64 U64 :=
      @Move.Verify.Contract.mk State U64 U64
        (fun _ _ => True)
        (fun _ _ result _ => result = 0)
        (fun _ _ _ => False)
    have loopVerified :
        Move.Verify.Satisfies
          (Move.Semantics.Spec.fix fun recursive n =>
            if Move.Verify.Source.logicalLT n 1 then
              Move.Semantics.Spec.pure n
            else
              Move.Semantics.Spec.bind
                (Move.Semantics.Checked.subSpec n 1) recursive)
          loopContract := by
      apply Move.Verify.satisfies_fix_of_wp
      intro recursive recursiveVerified n initial _
      by_cases hloop : Move.Verify.Source.logicalLT n 1
      · simp only [hloop, if_true]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change n.toNat < 1 at hloop
        have nzero := Move.U64.eq_zero_of_not_pos (by omega : ¬0 < n.toNat)
        subst n
        simp [Move.Verify.wp, Move.Semantics.Spec.pure, loopContract]
      · simp only [hloop, if_false]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change ¬n.toNat < 1 at hloop
        have positive : 0 < n.toNat := by omega
        rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos positive,
          Move.Semantics.Spec.pure_bind]
        exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    apply Move.Verify.satisfies_fix_of_wp
    intro _ _ n initial _
    exact Move.Verify.wp_of_satisfies loopVerified trivial

  verify labeledProof

  verify shadowedLoopState

  verify shadowedLoopArrow

  verify drain by
    unfold drain.contract drain.sourceSpec
    intro State store
    have loopVerified (reference : Move.Semantics.Mutation U64) :
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
              output.1 = 0 ∧ output.2.current = 0 ∧ final = initial)
            (fun _ _ _ => False)) := by
      apply Move.Verify.satisfies_fix_of_wp
      intro recursive recursiveVerified n initial _
      by_cases hloop : Move.Verify.Source.logicalLT 0 n
      · simp only [hloop, if_true]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change 0 < n.toNat at hloop
        rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
          Move.Semantics.Spec.pure_bind]
        exact Move.Verify.wp_of_satisfies recursiveVerified trivial
      · simp only [hloop, if_false]
        simp only [Move.Verify.Source.logicalLT_u64] at hloop
        change ¬0 < n.toNat at hloop
        have nzero := Move.U64.eq_zero_of_not_pos hloop
        subst n
        simp [Move.Verify.wp, Move.Semantics.Spec.pure,
          Move.Semantics.Mutation.write]
    intro addr initial permitted
    constructor
    · intro result final execution
      simp only [Move.Semantics.Spec.pure_bind] at execution
      unfold Move.Semantics.Resource.withBorrowMutFocusSpec
        Move.Semantics.Resource.withBorrowMutSpec at execution
      rcases execution with ⟨owner, bodyWorld, finalOwner, lookup,
        bodyExecution, finalEq⟩
      rcases bodyExecution with ⟨output, mutationWorld, mutationExecution,
        pureExecution⟩
      rcases pureExecution with ⟨outputEq, worldEq⟩
      unfold Move.Semantics.withMutation at mutationExecution
      rcases mutationExecution with ⟨future, reference, loopExecution,
        referenceFinished, outputFuture⟩
      have verified := (loopVerified
        ({ current := owner.value, prophecy := future } :
          Move.Semantics.Mutation U64)
        owner.value initial trivial).1 (output.1, reference) mutationWorld
        loopExecution
      have resultZero : output.1 = 0 := verified.1
      have referenceZero : reference.current = 0 := verified.2.1
      have mutationWorldEq : mutationWorld = initial := verified.2.2
      have outputZero : output.2 = 0 := by
        rw [outputFuture, ← referenceFinished, referenceZero]
      rcases outputEq with ⟨rfl, rfl⟩
      subst mutationWorld
      subst final
      simp [outputZero]
    · intro code execution
      simp only [Move.Semantics.Spec.pure_bind] at execution
      unfold Move.Semantics.Resource.withBorrowMutFocusSpec
        Move.Semantics.Resource.withBorrowMutSpec at execution
      rcases execution with missing | ⟨owner, lookup, bodyAbort⟩
      · change False
        change Move.Semantics.ResourceStore.contains initial addr at permitted
        have missingLookup :
            Move.Semantics.ResourceStore.lookup initial addr = none :=
          missing.1
        simp [Move.Semantics.ResourceStore.contains, missingLookup] at permitted
      · rcases bodyAbort with mutationAbort |
          ⟨output, mutationWorld, _, pureAbort⟩
        · unfold Move.Semantics.withMutation at mutationAbort
          rcases mutationAbort with ⟨future, loopAbort⟩
          exact (loopVerified
            ({ current := owner.value, prophecy := future } :
              Move.Semantics.Mutation U64)
            owner.value initial trivial).2 code loopAbort
        · exact pureAbort.elim

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
