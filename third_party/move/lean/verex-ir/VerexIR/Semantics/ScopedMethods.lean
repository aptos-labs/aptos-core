-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.Focus

namespace VerexIR

open Validation

/-- The first unbound parameter of a closure is its explicit receiver. -/
def receiverScopedTarget (unit : ValidatedUnit) (handle : FunctionHandle) (mask : Nat) : Bool :=
  match unit.namespaces[handle.namespaceId.index]? with
  | none => false
  | some ns => match ns.functions[handle.functionId.index]? with
    | none => false
    | some declaration =>
        match (ClosureMask.extract mask false
            (List.range declaration.signature.parameters.size)).head? with
        | none => false
        | some receiver => ns.borrowCertificates.any fun certificate =>
            certificate.functionId == handle.functionId &&
              certificate.receiverNoEscape.contains receiver

/-- Requirements on a target are obligations on its actual captured values.
An open dictionary parameter is deliberately not accepted by this bounded
refinement: its requirement cannot be discharged by the target alone. -/
def scopedDictionaryCaptures? (unit : ValidatedUnit) (handle : FunctionHandle)
    (mask : Nat) (captures : Array α) : Option (List α) := do
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.functions[handle.functionId.index]?
  let positions := ClosureMask.extract mask true (List.range declaration.signature.parameters.size)
  let requirements := declaration.attributes.toList.filterMap fun
    | .assign "scoped_method_evidence" (.constant (.integer index)) _ => some index
    | _ => none
  requirements.mapM fun index => do
    if index < 0 then none else do
      let offset ← positions.idxOf? index.toNat
      captures[offset]?


private theorem mapM_option_map (f : α → β) (g : γ → Option α) (xs : List γ) :
    xs.mapM (fun x => (g x).map f) = (xs.mapM g).map (List.map f) := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
      simp only [List.mapM_cons, ih]
      cases g x <;> cases xs.mapM g <;> rfl

@[simp] theorem scopedDictionaryCaptures?_map (unit : ValidatedUnit) (handle : FunctionHandle)
    (mask : Nat) (captures : Array α) (f : α → β) :
    scopedDictionaryCaptures? unit handle mask (captures.map f) =
      (scopedDictionaryCaptures? unit handle mask captures).map (List.map f) := by
  unfold scopedDictionaryCaptures?
  cases unit.namespaces[handle.namespaceId.index]? with
  | none => rfl
  | some ns =>
      simp only [bind, Option.bind]
      cases ns.functions[handle.functionId.index]? with
      | none => rfl
      | some declaration =>
          dsimp only
          rw [← mapM_option_map]
          congr 1
          funext index
          split
          · rfl
          · split <;> simp only [*, Array.getElem?_map, Option.map_none]

/-- Reconstruct the binder arguments represented by a raw closure frame. -/
def receiverScopedFrameArguments (ns : ValidatedNamespace)
    (instantiation : Array (TypeId × TypeId)) (generics : Array GenericBinder) : Array GenericArgument :=
  generics.mapIdx fun index binder =>
    let node : Option TypeId := do
      let node ← ns.tables.types.findIdx? fun type => match type with
        | .typeParameter other => other == index
        | _ => false
      some ⟨node⟩
    match binder.kind, node with
    | .typeArg, some node => .typeArg {
        typeId := SemanticOperations.instantiatedTypeId instantiation node, loc := ⟨0⟩ }
    | _, _ => .lifetime ⟨0⟩

/-- A raw closure carries only the canonical frame of its declared generic
binders; it cannot reinterpret concrete scalar nodes as reference types. -/
def receiverScopedFrame (ns : ValidatedNamespace) (declaration : FunctionDecl FunctionBody)
    (instantiation : Array (TypeId × TypeId)) : Bool :=
  if declaration.signature.generics.all (·.kind != .typeArg) then instantiation.isEmpty else
    instantiation == SemanticOperations.invocationTypeInstantiation ns #[]
      (receiverScopedFrameArguments ns instantiation declaration.signature.generics)

/-- Shape checks are essential for raw externally supplied closures, which
have not passed the typed carrier's closure admission check. -/
def receiverScopedShape (unit : ValidatedUnit) (handle : FunctionHandle) (mask : Nat)
    (instantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) : Bool :=
  match unit.namespaces[handle.namespaceId.index]? with
  | none => false
  | some ns => match ns.functions[handle.functionId.index]? with
    | none => false
    | some declaration =>
        mask < 2 ^ declaration.signature.parameters.size &&
          captures.size == (ClosureMask.extract mask true
            (List.range declaration.signature.parameters.size)).length &&
          captures.all SemanticOperations.plain? &&
          receiverScopedFrame ns declaration instantiation

/-- Executable receipt check, recursively checking actual captured dictionaries. -/
def receiverScopedValue (unit : ValidatedUnit) (value : RuntimeValue) : Bool :=
  match value with
  | .closure handle mask instantiation captures =>
      receiverScopedTarget unit handle mask &&
        receiverScopedShape unit handle mask instantiation captures &&
        match scopedDictionaryCaptures? unit handle mask
            (captures.map fun capture => receiverScopedValue unit capture) with
        | none => false
        | some checked => checked.all id
  | _ => false
termination_by sizeOf value

/-- The declared scoped dictionary slots, shared by compiled and runtime entry. -/
def scopedMethodIndices (attributes : Array Attribute) : List Int :=
  attributes.toList.filterMap fun
    | .assign "scoped_method_evidence" (.constant (.integer index)) _ => some index
    | _ => none

/-- Every explicitly scoped argument must carry an actual checked receipt. -/
def scopedValues (unit : ValidatedUnit) (indices : List Int)
    (arguments : Array RuntimeValue) : Bool :=
  indices.all fun index =>
    if index < 0 then false else
      (arguments[index.toNat]?).any (receiverScopedValue unit)

def scopedArguments (unit : ValidatedUnit) (declaration : FunctionDecl FunctionBody)
    (arguments : Array RuntimeValue) : Bool :=
  scopedValues unit (scopedMethodIndices declaration.attributes) arguments

end VerexIR
