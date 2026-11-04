-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Invocation
import VerexIR.Proofs.ClosureTyping
import VerexIR.Semantics.ScopedMethods

/-!
# Checked receiver scope of static method dictionaries

This refinement exposes the borrow checker's receipt, rather than assuming a
lifetime property from a dictionary annotation. A closure must have a checked
receiver receipt and satisfy the requirements of every captured dictionary.
-/

namespace VerexIR.Proofs

open Validation Denote

abbrev receiverScopedTarget := VerexIR.receiverScopedTarget
abbrev receiverScopedShape := VerexIR.receiverScopedShape
abbrev scopedDictionaryCaptures? := @VerexIR.scopedDictionaryCaptures?

/-- The public refinement is exactly the runtime's executable receipt check. -/
def ReceiverScopedClosure (unit : ValidatedUnit) (value : RuntimeValue) : Prop :=
  receiverScopedValue unit value = true

theorem ReceiverScopedClosure.checked {unit : ValidatedUnit}
    (handle : FunctionHandle) (mask : Nat) (instantiation : Array (TypeId × TypeId))
    (captures : Array RuntimeValue) (dictionaries : List RuntimeValue)
    (receipt : receiverScopedTarget unit handle mask = true)
    (shape : receiverScopedShape unit handle mask instantiation captures = true)
    (requirements : scopedDictionaryCaptures? unit handle mask captures = some dictionaries)
    (captured : ∀ dictionary ∈ dictionaries, ReceiverScopedClosure unit dictionary) :
    ReceiverScopedClosure unit (.closure handle mask instantiation captures) := by
  unfold ReceiverScopedClosure
  rw [receiverScopedValue, VerexIR.scopedDictionaryCaptures?_map]
  simp only [requirements, Option.map_some, receipt, shape, Bool.true_and, List.all_map]
  exact List.all_eq_true.mpr captured

private theorem admittedLength (unit : ValidatedUnit) (row : NRow)
    (values : List RuntimeValue) (admitted : NRow.admits unit row values = true) :
    values.length = row.length := by
  induction values generalizing row with
  | nil => cases row <;> simp_all [NRow.admits, NRow.length]
  | cons value values ih =>
      cases row with
      | nil => simp [NRow.admits] at admitted
      | cons type rest =>
          simp only [NRow.admits, Bool.and_eq_true] at admitted
          simp only [List.length_cons, NRow.length, ih rest admitted.2]

private theorem substitutedLength (types : Nat → NTy) (items : List NTy) :
    ((NRow.ofList items).substWith types).length = items.length := by
  induction items <;> simp_all [NRow.ofList, NRow.substWith, NRow.length]

/-- Typed closure carriers already enforce the shape/frame check needed by raw
runtime values. This bridges existing closure admission to the new refinement. -/
theorem receiverScopedShape_of_admits {unit : ValidatedUnit} {parameters results : NRow}
    {shared : Sharing} (closure : ClosureValue)
    (admitted : NTy.admits unit (.function parameters shared results) closure.encode = true) :
    receiverScopedShape unit closure.function closure.mask closure.typeInstantiation closure.captures = true := by
  rcases closure with ⟨handle, mask, instantiation, captures, plain⟩
  simp only [ClosureValue.encode, NTy.admits, Bool.and_eq_true] at admitted
  obtain ⟨faithful, rowsAdmitted⟩ := admitted
  split at rowsAdmitted
  · next capturedAt suppliedAt returnedAt rowsAt =>
      simp only [Bool.and_eq_true, beq_iff_eq] at rowsAdmitted
      obtain ⟨⟨⟨_, _⟩, _⟩, capturesAdmitted⟩ := rowsAdmitted
      simp only [closureRowsIn?, Option.map_eq_some_iff, Prod.mk.injEq] at rowsAt
      obtain ⟨⟨captured, supplied, returned⟩, rows, rfl, rfl, rfl⟩ := rowsAt
      obtain ⟨parameterNatives, resultNatives, signature, bounded, _, rfl, rfl, rfl⟩ :=
        closureRows?_eq_some.mp rows
      obtain ⟨ns, declaration, namespace_eq, declaration_eq, readParameters, _, _⟩ :=
        closureSignature?_eq_some signature
      have parameterCount : parameterNatives.length = declaration.signature.parameters.size := by
        simpa only [Array.length_toList] using (List.mapM_length readParameters)
      have captureCount := admittedLength unit _ _ capturesAdmitted
      rw [substitutedLength, Array.length_toList] at captureCount
      have selectedCount := extract_length mask true parameterNatives
        (List.range declaration.signature.parameters.size) (by simpa using parameterCount)
      have canonical : receiverScopedFrame ns declaration instantiation = true := by
        simp only [closureFaithful, namespace_eq, declaration_eq] at faithful
        unfold receiverScopedFrame
        split
        · next noTypes => simpa only [noTypes, ↓reduceIte] using faithful
        · next hasTypes =>
            simp only [hasTypes, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true] at faithful
            have arguments : frameArguments unit handle instantiation declaration.signature.generics =
                receiverScopedFrameArguments ns instantiation declaration.signature.generics := by
              unfold frameArguments receiverScopedFrameArguments
              congr 1
              funext index binder
              simp only [paramNodeIn?, namespace_eq, bind, Option.bind]
              cases ns.tables.types.findIdx? (fun type => match type with
                | .typeParameter other => other == index
                | _ => false) <;> cases binder.kind <;> rfl
            simpa only [arguments] using faithful.1.1
      change VerexIR.receiverScopedShape unit handle mask instantiation captures = true
      simp only [VerexIR.receiverScopedShape, namespace_eq, declaration_eq, Bool.and_eq_true]
      refine ⟨⟨⟨?_, ?_⟩, ?_⟩, canonical⟩
      · simpa only [parameterCount, decide_eq_true_eq] using bounded
      · exact beq_iff_eq.mpr (captureCount.trans selectedCount)
      · exact Array.all_eq_true_iff_forall_mem.mpr fun value member => (SemanticOperations.plain?_iff value).mpr (plain value member)
  · contradiction

/-- Every typed function value, including values at a generic skolem frame,
meets the raw closure's shape requirements. Receipt checking remains separate. -/
theorem receiverScopedShape_of_function {unit : ValidatedUnit} [FunctionCarriers unit]
    [Θ : Skolems unit] (parameters : NRow) (shared : Sharing) (results : NRow)
    (function : (NTy.function parameters shared results).carrier) :
    let closure := (Carriers.functions parameters shared results).closure function
    receiverScopedShape unit closure.function closure.mask closure.typeInstantiation closure.captures = true := by
  have admitted := NTy.admits_encode unit (Θ.resolve (.function parameters shared results))
    (Θ.toRuntime (.function parameters shared results) function)
  change NTy.admits unit (Θ.resolve (.function parameters shared results))
    (@NTy.encode (Carriers.runtime unit) _
      (Θ.toRuntime (.function parameters shared results) function)) = true at admitted
  rw [Θ.encode_toRuntime, Θ.resolve_eq] at admitted
  simp only [NTy.substWith, NTy.encode_function] at admitted
  exact receiverScopedShape_of_admits _ admitted

/-- The public contract refinement of a borrowed method dictionary. -/
def ReceiverScopedMethod {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
    (parameters : NRow) (shared : Sharing) (shape : ResultShape)
    (function : (NTy.function parameters shared shape.row).carrier) : Prop :=
  ReceiverScopedClosure unit ((Carriers.functions parameters shared shape.row).closure function).encode

theorem ReceiverScopedMethod.of_closure {unit : ValidatedUnit} [FunctionCarriers unit]
    [Skolems unit] {parameters : NRow} {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function parameters shared shape.row).carrier} {closure : ClosureValue}
    (closed : (Carriers.functions parameters shared shape.row).closure function = closure)
    (checked : ReceiverScopedClosure unit closure.encode) :
    ReceiverScopedMethod parameters shared shape function := by
  simpa only [ReceiverScopedMethod, closed] using checked

/-- Retain typed closure shape while opening the raw receipt obligation. -/
theorem ReceiverScopedMethod.of_closure_scoped {unit : ValidatedUnit} [FunctionCarriers unit]
    [Skolems unit] {parameters : NRow} {shared : Sharing} {shape : ResultShape}
    {function : (NTy.function parameters shared shape.row).carrier} {closure : ClosureValue}
    (closed : (Carriers.functions parameters shared shape.row).closure function = closure)
    (checked : receiverScopedShape unit closure.function closure.mask closure.typeInstantiation
        closure.captures = true → ReceiverScopedClosure unit closure.encode) :
    ReceiverScopedMethod parameters shared shape function := by
  apply ReceiverScopedMethod.of_closure closed
  apply checked
  rw [← closed]
  exact receiverScopedShape_of_function parameters shared shape.row function

theorem ReceiverScopedMethod.of_instantiate {unit : ValidatedUnit} [FunctionCarriers unit]
    [Θ : Skolems unit] (θ : TypeArgs) {parameters : NRow} {shared : Sharing} {shape : ResultShape}
    {function : @NTy.carrier (Skolems.instantiate θ Θ).toCarriers
      (.function parameters shared shape.row)}
    (checked : ReceiverScopedMethod (NRow.subst θ.1 parameters) shared (shape.subst θ.1)
      (shape.functionOfSkolem θ parameters shared function)) :
    @ReceiverScopedMethod unit _ (Skolems.instantiate θ Θ) parameters shared shape function := by
  simpa only [ReceiverScopedMethod, ResultShape.closure_functionOfSkolem] using checked

end VerexIR.Proofs
