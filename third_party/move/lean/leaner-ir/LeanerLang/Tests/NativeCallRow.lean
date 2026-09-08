-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerIR.Proofs.NativeCall

/-!
# Modular calls over the row, end to end

`bump_twice` calls `bump` through a fresh reborrow, twice.  The frame
route re-executes the callee at both sites; this proof consumes `bump`'s
proved contract instead — `wpFunction_of_satisfies` at each site, the
composed call-statement law carrying the spine, and the loan discipline
of the strengthened frame clause re-establishing the second site's
freshness facts.  Every step here is what the generated script will emit.
-/

namespace LeanerLang.Tests.NativeCallRow

set_option Elab.async false

leaner module 0x42::callprobe where
  public fun bump(slot : &mut u64) -> Unit := do
    *slot := *slot + 1

  spec bump where
    ensures slot == old(slot) + 1
    aborts_if slot + 1 > 18446744073709551615

  public fun bump_twice(slot : &mut u64) -> Unit := do
    core.call bump::<>(&mut *slot)
    core.call bump::<>(&mut *slot)

  spec bump_twice where
    ensures slot == old(slot) + 2
    aborts_if slot + 2 > 18446744073709551615

  verify bump
  verify bump_twice

set_option maxRecDepth 100000 in
set_option maxHeartbeats 15000 in
theorem bumpTwiceModular {registry : LeanerIR.Validation.SemanticsRegistry}
    {executable : LeanerIR.Validation.ExecutableUnit}
    (prepared : LeanerIR.Validation.prepareExecution registry
      «0x42».callprobe.unit = .ok executable) :
    LeanerIR.Proofs.Satisfies
      («0x42».callprobe.bump_twice.typedDenotation executable)
      «0x42».callprobe.bump_twice.typedContract := by
  have hu := (LeanerIR.Validation.prepareExecution_unit prepared).trans
    «0x42».callprobe.semantics_eq
  have hw :=
    (LeanerIR.Validation.prepareExecution_targetPointerWidth prepared).trans
      «0x42».callprobe.targetPointerWidth_eq
  have calleeSat := (LeanerIR.Proofs.satisfies_congr
    («0x42».callprobe._denotation_dependencies.namespace0.function0.bump.denotation_agrees
      hu)
    «0x42».callprobe.bump.contract).mpr
    («0x42».callprobe.bump.verified prepared)
  apply LeanerIR.Proofs.satisfies_of_wp
  intro arguments initial permitted
  simp only [«0x42».callprobe.bump_twice.typedContract,
    LeanerIR.Proofs.Contract.typed] at permitted ⊢
  simp [«0x42».callprobe.bump_twice.rawContract,
    «0x42».callprobe.bump_twice.argumentsCodec, lir_data_norm] at permitted
  leaner_cases permitted
  leaner_native_cases arguments
  leaner_rename slot_loan => loanVar
  leaner_rename slot => valVar
  leaner_rename slot_fits => fitsVar
  leaner_rename slot_loan_keyFree => keyFree
  leaner_rename slot_loan_bound => loanBound
  leaner_rename slot_nonNeg => valNonNeg
  leaner_rename slot_max => valMax
  simp only [«0x42».callprobe.bump_twice.typedDenotation]
  rw [LeanerIR.Proofs.wp_typedFunction]
  simp only [«0x42».callprobe._denotation_dependencies.namespace0.function1.bump_twice.denotation,
    «0x42».callprobe._denotation_dependencies.namespace0.function1.bump_twice.denotationBody]
  rw [LeanerIR.Proofs.Denotation.wp_nativeFunction]
  apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
  case arity => rfl
  case declared => simp [«0x42».callprobe.bump_twice.argumentsCodec,
    «0x42».callprobe._denotation_dependencies.namespace0.function1.bump_twice.denotationShape]
  simp only [«0x42».callprobe.bump_twice.argumentsCodec,
    «0x42».callprobe._denotation_dependencies.namespace0.function1.bump_twice.denotationShape,
    LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.mutable,
    LeanerIR.Proofs.Denotation.initialLocals_one,
    LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
  apply LeanerIR.Proofs.Denotation.wpRowThrow_blockUnit
  apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_consCallReborrow
  case mutableKind => rfl
  case priorLoan => exact loanBound
  case calleeWp =>
    apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies calleeSat
    case permitted =>
      simp only [«0x42».callprobe.bump.contract,
        LeanerIR.Proofs.Contract.runtime,
        «0x42».callprobe.bump.typedContract, LeanerIR.Proofs.Contract.typed,
        «0x42».callprobe.bump.rawContract,
        «0x42».callprobe.bump.argumentsCodec,
        LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
      refine ⟨⟨⟨initial.nextLoan, ⟨valVar, fitsVar⟩⟩⟩, rfl,
        initial.nextLoan, valVar,
        ⟨⟨⟨rfl, valNonNeg, valMax⟩, ?_⟩, by omega⟩, ?_⟩
      · intro loan bound
        have advanced : initial.nextLoan + 1 ≤ loan := bound
        exact freshLoans loan (by omega)
      · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
        exact freshLoans initial.nextLoan (by omega)
    case onReturn =>
      intro results final ensuresFn frameFn notMust
      have ensures := ensuresFn (by exact notMust)
      simp only [«0x42».callprobe.bump.contract,
        LeanerIR.Proofs.Contract.runtime,
        «0x42».callprobe.bump.typedContract, LeanerIR.Proofs.Contract.typed,
        «0x42».callprobe.bump.rawContract] at ensures
      simp only [«0x42».callprobe.bump.contract,
        LeanerIR.Proofs.Contract.runtime,
        «0x42».callprobe.bump.typedContract, LeanerIR.Proofs.Contract.typed,
        «0x42».callprobe.bump.rawContract] at frameFn
      obtain ⟨args, result, argsEq, decodeEq, slot_loan, slot_entry, slot,
        slot_pending, ⟨⟨⟨argsEq2, resultsEq⟩, pendingEq, pendResolve⟩,
          slotNonNeg, slotMax⟩, obligation⟩ := ensures
      rw [argsEq] at argsEq2
      simp only [Array.mk.injEq, List.cons.injEq,
        LeanerIR.RuntimeValue.borrow.injEq,
        LeanerIR.RuntimeValue.integer.injEq, and_true] at argsEq2
      obtain ⟨loanEq, entryEq⟩ := argsEq2
      subst loanEq entryEq
      rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty]
        at pendResolve
      subst pendResolve
      simp only [LeanerIR.Proofs.Obligation_iff] at obligation
      subst obligation
      obtain ⟨args', argsEq', globalsEq, discipline⟩ := frameFn
      have monotone : initial.nextLoan + 1 ≤ final.nextLoan :=
        discipline.2.2
      have freshFinal :
          LeanerIR.SemanticOperations.FreshGlobalLoanIds final :=
        discipline.1 (fun loan bound => freshLoans loan (by
          exact Nat.le_trans (Nat.le_succ _) bound))
      have stableLookups := discipline.2.1
      refine ⟨valVar + 1, pendingEq, ?_⟩
      apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_consCallReborrow
      case mutableKind => rfl
      case priorLoan =>
        show loanVar < final.nextLoan
        omega
      case calleeWp =>
        apply LeanerIR.Proofs.Denotation.wpFunction_of_satisfies calleeSat
        case permitted =>
          simp only [«0x42».callprobe.bump.contract,
            LeanerIR.Proofs.Contract.runtime,
            «0x42».callprobe.bump.typedContract,
            LeanerIR.Proofs.Contract.typed,
            «0x42».callprobe.bump.rawContract,
            «0x42».callprobe.bump.argumentsCodec,
            LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt]
          refine ⟨⟨⟨final.nextLoan, ⟨valVar + 1, ?_⟩⟩⟩, rfl,
            final.nextLoan, valVar + 1,
            ⟨⟨⟨rfl, by omega, by omega⟩, ?_⟩, by omega⟩, ?_⟩
          · simp [LeanerIR.IntegerValueFits,
              LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
            omega
          · intro loan bound
            have advanced : final.nextLoan + 1 ≤ loan := bound
            exact freshFinal loan (by omega)
          · simp only [LeanerIR.SemanticOperations.globalLoanKey?]
            exact freshFinal final.nextLoan (by omega)
        case onReturn =>
          intro results2 final2 ensuresFn2 frameFn2 notMust2
          have ensures2 := ensuresFn2 (by exact notMust2)
          simp only [«0x42».callprobe.bump.contract,
            LeanerIR.Proofs.Contract.runtime,
            «0x42».callprobe.bump.typedContract,
            LeanerIR.Proofs.Contract.typed,
            «0x42».callprobe.bump.rawContract] at ensures2
          simp only [«0x42».callprobe.bump.contract,
            LeanerIR.Proofs.Contract.runtime,
            «0x42».callprobe.bump.typedContract,
            LeanerIR.Proofs.Contract.typed,
            «0x42».callprobe.bump.rawContract] at frameFn2
          obtain ⟨args2, result2, argsEq2', decodeEq2, slot_loan2,
            slot_entry2, slot2, slot_pending2,
            ⟨⟨⟨argsEq22, resultsEq2⟩, pendingEq2, pendResolve2⟩,
              slotNonNeg2, slotMax2⟩, obligation2⟩ := ensures2
          rw [argsEq2'] at argsEq22
          simp only [Array.mk.injEq, List.cons.injEq,
            LeanerIR.RuntimeValue.borrow.injEq,
            LeanerIR.RuntimeValue.integer.injEq, and_true] at argsEq22
          obtain ⟨loanEq2, entryEq2⟩ := argsEq22
          subst loanEq2 entryEq2
          rw [LeanerIR.SemanticOperations.resolveReturnedBorrows_empty]
            at pendResolve2
          subst pendResolve2
          simp only [LeanerIR.Proofs.Obligation_iff] at obligation2
          subst obligation2
          obtain ⟨args2', argsEq2'', globalsEq2, discipline2⟩ := frameFn2
          have monotone2 : final.nextLoan + 1 ≤ final2.nextLoan :=
            discipline2.2.2
          have freshFinal2 :
              LeanerIR.SemanticOperations.FreshGlobalLoanIds final2 :=
            discipline2.1 (fun loan bound => freshFinal loan (by
              exact Nat.le_trans (Nat.le_succ _) bound))
          have stableLookups2 := discipline2.2.1
          refine ⟨valVar + 1 + 1, pendingEq2, ?_⟩
          apply LeanerIR.Proofs.Denotation.wpStatementsRowThrow_nil
          intro outcome finishEq
          simp only [LeanerIR.SemanticOperations.finishControl?,
            LeanerIR.SemanticOperations.unpackFallthrough,
            Option.map_eq_map, Option.map_some, Option.some.injEq]
            at finishEq
          subst finishEq
          simp only [LeanerIR.SemanticOperations.finalizeFunctionState]
          rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_singleInteger
            _ _ _ _ _ ?keyStillFree]
          case keyStillFree =>
            have step2 := stableLookups2 loanVar
              (show loanVar < final.nextLoan + 1 by omega)
            have step1 := stableLookups loanVar
              (show loanVar < initial.nextLoan + 1 by omega)
            exact step2.trans (step1.trans keyFree)
          simp only [«0x42».callprobe.bump_twice.rawContract,
            «0x42».callprobe.bump_twice.resultsCodec]
          have notAbort : ¬∃ slot_loan slot,
              (#[LeanerIR.RuntimeValue.borrow loanVar
                (LeanerIR.RuntimeValue.integer valVar)] :
                  Array LeanerIR.RuntimeValue) =
                #[LeanerIR.RuntimeValue.borrow slot_loan
                  (LeanerIR.RuntimeValue.integer slot)] ∧
              18446744073709551615 < slot + 2 := by
            rintro ⟨slot_loan, slot, rowEq, overflow⟩
            simp only [Array.mk.injEq, List.cons.injEq,
              LeanerIR.RuntimeValue.borrow.injEq,
              LeanerIR.RuntimeValue.integer.injEq, and_true] at rowEq
            obtain ⟨-, valEq⟩ := rowEq
            omega
          have stateDiscipline : LeanerIR.SemanticOperations.LoanDiscipline
              initial
              { globals := final2.globals, globalLoans := final2.globalLoans,
                nextLoan := final2.nextLoan,
                pending := initial.pending.push
                  (loanVar, LeanerIR.RuntimeValue.integer (valVar + 1 + 1)) } := by
            have hopEntry : LeanerIR.SemanticOperations.LoanDiscipline initial
                { globals := initial.globals,
                  globalLoans := initial.globalLoans,
                  nextLoan := initial.nextLoan + 1,
                  pending := initial.pending } :=
              LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl
                (Nat.le_succ _)
            have hopBetween : LeanerIR.SemanticOperations.LoanDiscipline final
                { globals := final.globals, globalLoans := final.globalLoans,
                  nextLoan := final.nextLoan + 1,
                  pending := initial.pending } :=
              LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl
                (Nat.le_succ _)
            have hopExit : LeanerIR.SemanticOperations.LoanDiscipline final2
                { globals := final2.globals,
                  globalLoans := final2.globalLoans,
                  nextLoan := final2.nextLoan,
                  pending := initial.pending.push
                    (loanVar,
                      LeanerIR.RuntimeValue.integer (valVar + 1 + 1)) } :=
              LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl
                (Nat.le_refl _)
            exact ((hopEntry.trans discipline).trans
              (hopBetween.trans discipline2)).trans hopExit
          refine ⟨PUnit.unit, trivial,
            fun _ => ⟨loanVar, valVar, valVar + 1 + 1,
              .integer (valVar + 1 + 1),
              ⟨⟨⟨rfl, trivial⟩, rfl,
                (LeanerIR.SemanticOperations.resolveReturnedBorrows_empty
                  _).symm⟩,
                by omega, by omega⟩, ?_⟩,
            ⟨?_, stateDiscipline⟩, notAbort⟩
          · simp only [LeanerIR.Proofs.Obligation_iff]
            omega
          · exact globalsEq2.trans globalsEq
        case onThrow =>
          intro kind thrown final2 abortsFact2
          intro finalFrame finalState outcome finishEq
          simp only [LeanerIR.SemanticOperations.finishControl?,
            Option.some.injEq] at finishEq
          subst finishEq
          simp only [«0x42».callprobe.bump.contract,
            LeanerIR.Proofs.Contract.runtime,
            «0x42».callprobe.bump.typedContract,
            LeanerIR.Proofs.Contract.typed,
            «0x42».callprobe.bump.rawContract] at abortsFact2
          obtain ⟨args2, argsEq2', slot_loan2, slot2, argsEq22, obligation2⟩ :=
            abortsFact2
          rw [argsEq2'] at argsEq22
          simp only [Array.mk.injEq, List.cons.injEq,
            LeanerIR.RuntimeValue.borrow.injEq,
            LeanerIR.RuntimeValue.integer.injEq, and_true] at argsEq22
          obtain ⟨-, slotEq2⟩ := argsEq22
          subst slotEq2
          simp only [«0x42».callprobe.bump_twice.rawContract]
          refine ⟨loanVar, valVar, rfl, ?_⟩
          simp only [LeanerIR.Proofs.Obligation_iff] at obligation2 ⊢
          omega
    case onThrow =>
      intro kind thrown final abortsFact
      intro finalFrame finalState outcome finishEq
      simp only [LeanerIR.SemanticOperations.finishControl?,
        Option.some.injEq] at finishEq
      subst finishEq
      simp only [«0x42».callprobe.bump.contract,
        LeanerIR.Proofs.Contract.runtime,
        «0x42».callprobe.bump.typedContract, LeanerIR.Proofs.Contract.typed,
        «0x42».callprobe.bump.rawContract] at abortsFact
      obtain ⟨args, argsEq, slot_loan, slot, argsEq2, obligation⟩ :=
        abortsFact
      rw [argsEq] at argsEq2
      simp only [Array.mk.injEq, List.cons.injEq,
        LeanerIR.RuntimeValue.borrow.injEq,
        LeanerIR.RuntimeValue.integer.injEq, and_true] at argsEq2
      obtain ⟨-, slotEq⟩ := argsEq2
      subst slotEq
      simp only [«0x42».callprobe.bump_twice.rawContract]
      refine ⟨loanVar, valVar, rfl, ?_⟩
      simp only [LeanerIR.Proofs.Obligation_iff] at obligation ⊢
      omega

end LeanerLang.Tests.NativeCallRow
