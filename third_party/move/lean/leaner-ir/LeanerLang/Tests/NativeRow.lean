-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# The frame-free route, end to end

`bump` — one mutable scalar parameter, a checked increment through it — is
the smallest function whose verification exercises every piece of the
frame-free layer: the entry law, the row drive, evaluator resolution for a
read, a checked add with its range split, a write through the reference,
the prophecy-pair exit, and the constructive contract closing.

This proof is hand-scripted where the automated resolver is still being
built, and it is the measurement anchor for the direction: the same
theorem through the frame route costs 35.2M heartbeats; this route costs
about 6M.  Every step here is what the generated script will emit.
-/

namespace LeanerLang.Tests.NativeRow

set_option Elab.async false

leaner module 0x42::rowprobe where
  public fun bump(slot : &mut u64) -> Unit := do
    *slot := *slot + 1

  spec bump where
    ensures slot == old(slot) + 1
    aborts_if slot + 1 > 18446744073709551615

  verify bump

set_option maxRecDepth 100000 in
set_option maxHeartbeats 30000 in
theorem bumpRowRoute {registry : LeanerIR.Validation.SemanticsRegistry}
    {executable : LeanerIR.Validation.ExecutableUnit}
    (prepared : LeanerIR.Validation.prepareExecution registry «0x42».rowprobe.unit
      = .ok executable) :
    LeanerIR.Proofs.Satisfies («0x42».rowprobe.bump.typedDenotation executable)
      «0x42».rowprobe.bump.typedContract := by
  have hu := (LeanerIR.Validation.prepareExecution_unit prepared).trans
    «0x42».rowprobe.semantics_eq
  have hw :=
    (LeanerIR.Validation.prepareExecution_targetPointerWidth prepared).trans
      «0x42».rowprobe.targetPointerWidth_eq
  have hn : executable.unit.namespaces[0]? =
      some «0x42».rowprobe._denotation_dependencies.namespace0.function0.denotationNamespace := by
    rw [hu]
    rfl
  apply LeanerIR.Proofs.satisfies_of_wp
  intro arguments initial permitted
  simp only [«0x42».rowprobe.bump.typedContract,
    LeanerIR.Proofs.Contract.typed] at permitted ⊢
  simp [«0x42».rowprobe.bump.rawContract,
    «0x42».rowprobe.bump.argumentsCodec, lir_data_norm] at permitted
  leaner_cases permitted
  leaner_native_cases arguments
  leaner_rename slot_loan => loanVar
  leaner_rename slot => valVar
  leaner_rename slot_fits => fitsVar
  leaner_rename slot_loan_keyFree => keyFree
  leaner_rename slot_loan_bound => loanBound
  leaner_rename slot_nonNeg => valNonNeg
  leaner_rename slot_max => valMax
  simp only [«0x42».rowprobe.bump.typedDenotation]
  rw [LeanerIR.Proofs.wp_typedFunction]
  simp only [«0x42».rowprobe._denotation_dependencies.namespace0.function0.bump.denotation,
    «0x42».rowprobe._denotation_dependencies.namespace0.function0.bump.denotationBody]
  rw [LeanerIR.Proofs.Denotation.wp_nativeFunction]
  apply LeanerIR.Proofs.Denotation.nativeEntry_rowFrame
  case arity => rfl
  case declared => simp [«0x42».rowprobe.bump.argumentsCodec,
    «0x42».rowprobe._denotation_dependencies.namespace0.function0.bump.denotationShape]
  case stable => leaner_row_stable
  simp only [«0x42».rowprobe.bump.argumentsCodec,
    «0x42».rowprobe._denotation_dependencies.namespace0.function0.bump.denotationShape,
    LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
    LeanerIR.Proofs.Denotation.initialLocals_one,
    LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
  have bounds : (LeanerIR.Ty.integer (LeanerIR.IntWidth.bits 64) false).integerBounds?
      = some (0, 2 ^ 64 - 1) := rfl
  leaner_row_drive
  rename_i readOne readEq readTwo readEqTwo
  simp only [List.getElem?_toArray, List.getElem?_cons_zero, Option.join_some,
    Option.some.injEq] at readEq readEqTwo
  subst readEq readEqTwo
  constructor
  · intro finalRow finalState readValue evaluated
    simp only [LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
      LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
      LeanerIR.SemanticOperations.dereferenceBorrow?] at evaluated
    injection evaluated with inner
    injection inner with frameEq stateEq valueEq
    injection frameEq with rowEq activeEq locationsEq
    subst rowEq stateEq valueEq
    leaner_row_drive
    constructor
    · intro finalRow finalState sum evaluated
      simp only [LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
        LeanerIR.SemanticOperations.checkedBinaryInteger,
        LeanerIR.SemanticOperations.checkedInteger, bounds] at evaluated
      split at evaluated
      next inRange =>
        simp only [Bool.and_eq_true, decide_eq_true_eq] at inRange
        obtain ⟨sumNonNeg, sumMax⟩ := inRange
        injection evaluated with inner
        injection inner with frameEq stateEq valueEq
        injection frameEq with rowEq activeEq locationsEq
        subst rowEq stateEq valueEq
        leaner_row_drive
        constructor
        · intro finalRow finalState written mutated
          rw [LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow] at mutated
          injection mutated with inner
          injection inner with frameEq stateEq valueEq
          injection frameEq with rowEq activeEq locationsEq
          subst rowEq stateEq valueEq
          leaner_row_drive
          rename_i outcome finished
          simp only [LeanerIR.SemanticOperations.finishControl?,
            LeanerIR.SemanticOperations.unpackFallthrough,
            Option.map_eq_map, Option.map_some, Option.some.injEq] at finished
          subst finished
          simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
          rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_singleInteger
            _ _ _ _ _ (by assumption)]
          leaner_certified_close!
        · intro finalRow finalState throwKind thrown mutated
          rw [LeanerIR.Proofs.Denotation.mutate_evaluate_rowFrame_singleBorrow] at mutated
          injection mutated with inner
          injection inner
      next outOfRange =>
        injection evaluated with inner
        injection inner
    · intro finalRow finalState throwKind thrown evaluated
      simp only [LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
        LeanerIR.SemanticOperations.checkedBinaryInteger,
        LeanerIR.SemanticOperations.checkedInteger, bounds] at evaluated
      split at evaluated
      next inRange =>
        injection evaluated with inner
        injection inner
      next outOfRange =>
        simp only [Bool.and_eq_true, decide_eq_true_eq, not_and] at outOfRange
        have overflow : 18446744073709551615 < valVar + 1 := by
          have := outOfRange (by omega)
          omega
        injection evaluated with inner
        injection inner with frameEq stateEq kindEq thrownEq
        injection frameEq with rowEq activeEq locationsEq
        subst rowEq stateEq kindEq thrownEq
        leaner_row_drive
        rename_i outcome finished
        simp only [LeanerIR.SemanticOperations.finishControl?,
          Option.some.injEq] at finished
        subst finished
        leaner_certified_close!
  · intro finalRow finalState throwKind thrown evaluated
    simp only [LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
      LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
      LeanerIR.SemanticOperations.dereferenceBorrow?] at evaluated
    injection evaluated with inner
    injection inner

end LeanerLang.Tests.NativeRow
