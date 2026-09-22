-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerIR.Proofs.NativeStore

/-!
# Storage over the row, end to end

`deposit` — a mutable global borrow, a field reborrow, a checked add
written back through it — is the smallest loan-bracketed segment.  This
proof drives it through the bracket law with the inner block on the
ordinary row rules, and closes the contract through the family
representation, never through the map.
-/

namespace LeanerLang.Tests.NativeStoreRow

set_option Elab.async false

/-- The path `Coin.amount.value`: the resource, then its one field, no
siblings at either step. -/
private def coinPath : List LeanerIR.SemanticOperations.FocusStep :=
  [⟨⟨⟨0⟩, 1⟩, #[], #[], none⟩, ⟨⟨⟨0⟩, 0⟩, #[], #[], none⟩]

leaner module 0x42::storeprobe where
  struct Amount has Copy, Drop, Store where
    value : u64

  struct Coin has Key where
    amount : Amount

  public entry fun deposit(addr : Address, amount : u64) -> Unit := do
    let value := &mut Coin[addr].amount.value
    *value := *value + amount

  spec deposit where
    requires exists<Coin>(addr)
    modifies global<Coin>(addr)
    ensures global<Coin>(addr).amount.value
        == old(global<Coin>(addr).amount.value) + amount
    aborts_if old(global<Coin>(addr).amount.value) + amount
        > 18446744073709551615

  verify deposit

set_option maxRecDepth 100000 in
set_option maxHeartbeats 25000 in
theorem depositRowRoute {registry : LeanerIR.Validation.SemanticsRegistry}
    {executable : LeanerIR.Validation.ExecutableUnit}
    (prepared : LeanerIR.Validation.prepareExecution registry
      «0x42».storeprobe.unit = .ok executable) :
    LeanerIR.Proofs.Satisfies
      («0x42».storeprobe.deposit.typedDenotation executable)
      «0x42».storeprobe.deposit.typedContract := by
  have hu := (LeanerIR.Validation.prepareExecution_unit prepared).trans
    «0x42».storeprobe.semantics_eq
  have hw :=
    (LeanerIR.Validation.prepareExecution_targetPointerWidth prepared).trans
      «0x42».storeprobe.targetPointerWidth_eq
  apply LeanerIR.Proofs.satisfies_of_wp
  intro arguments initial permitted
  simp only [«0x42».storeprobe.deposit.typedContract,
    LeanerIR.Proofs.Contract.typed] at permitted ⊢
  simp [«0x42».storeprobe.deposit.rawContract,
    «0x42».storeprobe.deposit.argumentsCodec, lir_data_norm] at permitted
  leaner_cases permitted
  leaner_native_cases arguments
  simp only [«0x42».storeprobe.deposit.typedDenotation]
  rw [LeanerIR.Proofs.wp_typedFunction]
  simp only [«0x42».storeprobe._denotation_dependencies.namespace0.function0.deposit.denotation,
    «0x42».storeprobe._denotation_dependencies.namespace0.function0.deposit.denotationBody]
  rw [LeanerIR.Proofs.Denotation.wp_nativeFunction]
  leaner_rename Coin_contents => contents
  leaner_rename Coin_represented => represented
  leaner_rename amount => val
  leaner_rename amount_fits => fitsVar
  leaner_rename requires_0 => present
  leaner_rename amount_nonNeg => valNonNeg
  leaner_rename amount_max => valMax
  /- The resource at the key, read through the representation and
  destructured to its field. -/
  have lookupEq := represented (.address addr)
  rw [lookupEq] at present
  obtain ⟨resource, resourceEq⟩ :=
    Option.isSome_iff_exists.mp (by simpa using present)
  obtain ⟨⟨⟨fieldVal, fieldFits⟩⟩⟩ := resource
  have presentValue : initial.globals.lookup
      (LeanerIR.SemanticOperations.globalKey ⟨0⟩ ⟨6⟩ (.address addr)) =
      some (LeanerIR.SemanticOperations.focusValue coinPath (.integer fieldVal)) := by
    rw [show LeanerIR.SemanticOperations.globalKey ⟨0⟩ ⟨6⟩ (.address addr) =
      ⟨⟨0⟩, ⟨6⟩, .address addr⟩ from rfl, lookupEq, resourceEq]
    rfl
  apply LeanerIR.Proofs.Denotation.nativeEntryThrow_rowFrame
  case arity => rfl
  case declared => simp [«0x42».storeprobe.deposit.argumentsCodec,
    «0x42».storeprobe._denotation_dependencies.namespace0.function0.deposit.denotationShape]
  simp only [«0x42».storeprobe.deposit.argumentsCodec,
    «0x42».storeprobe._denotation_dependencies.namespace0.function0.deposit.denotationShape,
    LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.address]
  rw [show LeanerIR.SemanticOperations.initialLocals 4
      #[.address addr, .integer val] =
      #[some (.address addr), some (.integer val), none, none] by
        simp [LeanerIR.SemanticOperations.initialLocals]; rfl,
    show LeanerIR.SemanticOperations.parameterLoanLocations
      #[.address addr, .integer val] = #[] by
        simp [LeanerIR.SemanticOperations.parameterLoanLocations]]
  apply LeanerIR.Proofs.Denotation.wpRowThrow_focusedFieldBracket (steps := coinPath)
  case borrowMutable => rfl
  case fieldMutable => rfl
  case present => exact presentValue
  case innerStable => leaner_row_stable
  have bounds : (LeanerIR.Ty.integer (LeanerIR.IntWidth.bits 64) false).integerBounds?
      = some (0, 2 ^ 64 - 1) := rfl
  leaner_row_drive
  rename_i readOne readEq readTwo readEqTwo
  simp only [List.getElem?_toArray, List.getElem?_cons_zero,
    List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
    at readEq readEqTwo
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
    rename_i readThree readEqThree
    simp only [List.getElem?_toArray, List.getElem?_cons_zero,
      List.getElem?_cons_succ, Option.join_some, Option.some.injEq]
      at readEqThree
    subst readEqThree
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
          rw [LeanerIR.Proofs.Denotation.mutate_evaluate_focusedField] at mutated
          injection mutated with inner
          injection inner with frameEq stateEq valueEq
          injection frameEq with rowEq activeEq locationsEq
          subst rowEq stateEq valueEq
          leaner_row_drive
          rename_i exitFrame exitState retired exited
          have plainSteps : LeanerIR.SemanticOperations.PlainSteps coinPath := by
            unfold coinPath; leaner_plain
          rw [LeanerIR.Proofs.Denotation.endLoan_evaluate_focusedField
            (steps := coinPath) (plainSteps := plainSteps)] at exited
          injection exited with exitInner
          injection exitInner with exitFrameEq exitStateEq retiredEq
          subst exitFrame exitState retired
          refine ⟨_, _, rfl, ?_⟩
          intro outcome finished
          simp only [LeanerIR.SemanticOperations.finishControl?,
            LeanerIR.SemanticOperations.unpackFallthrough,
            Option.map_eq_map, Option.map_some, Option.some.injEq] at finished
          subst finished
          leaner_finalize
          rw [LeanerIR.Proofs.Denotation.exportFrameLoans_rowFrame_borrowFree _ _ _
            (by
              intro slot mem value eq
              simp only [List.mem_toArray, List.mem_cons, List.mem_nil_iff,
                or_false] at mem
              rcases mem with rfl | rfl | rfl | rfl <;> cases eq <;>
                simp [LeanerIR.SemanticOperations.outermostBorrows,
                  LeanerIR.SemanticOperations.borrowEntry?,
                  LeanerIR.SemanticOperations.collectPruned])]
          have fitsSum : LeanerIR.IntegerValueFits (.bits 64) false (fieldVal + val) := by
            simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
              LeanerIR.Ty.integerBounds?]
            omega
          rw [show LeanerIR.SemanticOperations.globalKey ⟨0⟩ ⟨6⟩ (.address addr) =
            ⟨⟨0⟩, ⟨6⟩, .address addr⟩ from rfl]
          /- The updated representation, stated at the map's own spelling
          of the written resource so the closer's rewrite matches it. -/
          have represented' : LeanerIR.FamilyRepresentation
              «0x42».storeprobe.Coin.erase ⟨0⟩ ⟨6⟩
              (LeanerIR.updateContents contents (.address addr)
                (some ⟨⟨⟨fieldVal + val, fitsSum⟩⟩⟩))
              ((initial.globals.insert ⟨⟨0⟩, ⟨6⟩, .address addr⟩
                (.loanHole initial.nextLoan)).insert ⟨⟨0⟩, ⟨6⟩, .address addr⟩
                (LeanerIR.SemanticOperations.focusValue coinPath
                  (.integer (fieldVal + val)))) :=
            LeanerIR.FamilyRepresentation.insert_over_hole
              (erase := «0x42».storeprobe.Coin.erase) represented (.address addr)
              (.loanHole initial.nextLoan) ⟨⟨⟨fieldVal + val, fitsSum⟩⟩⟩
          simp only [«0x42».storeprobe.deposit.rawContract,
            «0x42».storeprobe.deposit.resultsCodec,
            «0x42».storeprobe.Coin.get, «0x42».storeprobe.Coin.read,
            «0x42».storeprobe.Coin.key]
          leaner_certified_close!
        · intro finalRow finalState throwKind thrown mutated
          rw [LeanerIR.Proofs.Denotation.mutate_evaluate_focusedField] at mutated
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
        injection evaluated with inner
        injection inner with frameEq stateEq kindEq thrownEq
        injection frameEq with rowEq activeEq locationsEq
        subst rowEq stateEq kindEq thrownEq
        leaner_row_drive
        rename_i outcome finished
        simp only [LeanerIR.SemanticOperations.finishControl?,
          Option.some.injEq] at finished
        subst finished
        simp only [«0x42».storeprobe.deposit.rawContract,
          «0x42».storeprobe.Coin.get, «0x42».storeprobe.Coin.read,
          «0x42».storeprobe.Coin.key]
        leaner_certified_close!
  · intro finalRow finalState throwKind thrown evaluated
    simp only [LeanerIR.Proofs.Denotation.ReferenceLocationOperation.evaluate?,
      LeanerIR.Proofs.Denotation.liftPlaceEvaluator,
      LeanerIR.SemanticOperations.dereferenceBorrow?] at evaluated
    injection evaluated with inner
    injection inner

end LeanerLang.Tests.NativeStoreRow
