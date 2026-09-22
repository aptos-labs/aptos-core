-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerLang.NativeCopy
import LeanerIR.Proofs.ArithmeticAgreement

/-! Pure aggregate construction over native twins. The generated value never
uses a runtime codec; erasure occurs only in the exact execution certificate. -/

namespace LeanerLang.NativeAggregate

open Lean Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

partial def fieldRep : SpecTypes.FieldRep → Typed.ValueRep
  | .int width signed => .int width signed
  | .bool => .bool
  | .string => .string
  | .address => .address
  | .signer => .signer
  | .bytes => .bytes
  | .unit => .unit
  | .vector element bounded => .vector (fieldRep element) bounded
  | .parameter index => .parameter index
  | .nominal name arguments => .twin name (arguments.map fieldRep)

def isVector (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    (expression.getArg! 0).isConstOf ``PrimitiveLocationOperation.vector

def supported (expression : Lean.Expr) : Bool :=
  isVector expression || (expression.isAppOfArity ``nativeOperation 2 &&
    (expression.getArg! 0).isAppOfArity ``NominalConstructor.evaluate? 1)

def isVariantTest (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativeOperation 2 &&
    (expression.getArg! 0).isAppOfArity ``NominalVariantTest.evaluate? 1

def isProjection (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativeOperation 2 &&
    [``NominalVariantFieldLocation.evaluateSelect?, ``NominalFieldLocation.evaluateSelect?].contains
      (expression.getArg! 0).getAppFn.constName!

/-- Compile-time knowledge introduced by a native match arm, never a runtime
value or an unproved assumption in the generated theorem. -/
structure Known where
  expression : Lean.Expr
  value : Term
  variant : String
  fields : Array Term

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

def unaryOperand (expression : Lean.Expr) : CommandElabM Lean.Expr := do
  let operands := expression.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 && (operands.getArg! 1).isConstOf ``valuesNil do
    throwError "native nominal operation requires one operand"
  return operands.getArg! 0

/-- Read a local or a path whose enum variants have been selected by enclosing
native matches. Missing path knowledge is rejected, not filled with a default. -/
partial def read (twins : Array SpecTypes.TwinInfo) (locals : Array Typed.LocalInfo)
    (slots : Array (Option Term)) (known : Array Known) (expression : Lean.Expr) :
    CommandElabM (Typed.ValueRep × Term × TSyntax `tactic) := do
  if NativeCopy.supported expression then
    let (rep, value, proof) ← read twins locals slots known (← NativeCopy.operand expression)
    return (rep, value, ← NativeCopy.returns proof)
  let (rep, value, proof) ← if expression.isAppOfArity ``localVar 1 then do
    let some index := index? (expression.getArg! 0) | throwError "invalid native nominal local"
    let some (some value) := slots[index]? | throwError "native nominal local is unavailable"
    pure (locals[index]!.rep, value,
      ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.read
         first | rfl | (simp_all only <;> rfl))))
  else do
    unless isProjection expression do throwError "native path requires a local or nominal projection"
    let owner := ← unaryOperand expression
    let (ownerRep, ownerValue, ownerProof) ← read twins locals slots known owner
    let .twin name #[] := ownerRep | throwError "native projection requires a monomorphic owner"
    let some info := twins.find? (·.twin == name) | throwError "missing native projection owner"
    let descriptor := (expression.getArg! 0).getArg! 0
    let choice := known.find? (·.expression == owner)
    let (fields, values) ← if info.variants.isEmpty then do
        let values ← info.fields.mapM fun (field, _) =>
          ``($(rooted (name ++ Name.mkSimple field)) $ownerValue)
        pure (info.fields, values)
      else do
        let some choice := choice | throwError "native enum payload needs a selected variant"
        let some variant := info.variants.find? (·.name == choice.variant)
          | throwError "unknown native enum variant"
        pure (variant.fields, choice.fields)
    let index ← if descriptor.isAppOfArity ``NominalVariantFieldLocation.mk 2 then do
        let some choices ← liftTermElabM <| Meta.getArrayLit? (descriptor.getArg! 1)
          | throwError "native payload choices are not resolved"
        let some choice := choice | throwError "native variant payload needs a selected variant"
        let some pair := choices.find? fun pair =>
          (pair.getArg! 2) == mkStrLit choice.variant
          | throwError "native payload is unavailable on the selected variant"
        let some index := index? (pair.getArg! 3) | throwError "unresolved native payload position"
        pure index
      else do
        unless descriptor.isAppOfArity ``NominalFieldLocation.mk 3 do
          throwError "native field location is not resolved"
        let some index := index? (descriptor.getArg! 2) | throwError "unresolved native field position"
        pure index
    let some (_, field) := fields[index]? | throwError "native field position is out of bounds"
    let value := values[index]!
    let rep := fieldRep field
    let codec ← rep.codecSyntax (mkIdent `codecs)
    let proof ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.nativeOperation_value
         (value := ($codec).encode $value)
       · apply LeanerIR.Proofs.ComputationAgreement.cons
         · $ownerProof:tactic
         · exact LeanerIR.Proofs.ComputationAgreement.nil _
       · intro state
         first | rfl | (simp_all only <;> rfl)))
    pure (rep, value, proof)
  let value := (known.find? (·.expression == expression)).map (·.value) |>.getD value
  return (rep, value, proof)

def variants (expression : Lean.Expr) : CommandElabM (Array String) := do
  let descriptor := (expression.getArg! 0).getArg! 0
  unless descriptor.isAppOfArity ``NominalVariantTest.mk 2 do
    throwError "native variant test descriptor is not resolved"
  let some variants ← liftTermElabM <| Meta.getArrayLit? (descriptor.getArg! 1)
    | throwError "native variant test needs a closed variant set"
  variants.mapM fun variant => do
    let .lit (.strVal name) := variant | throwError "native variant name is not resolved"
    pure name

/-- Irrefutable owned patterns bind typed projections. The caller separately
checks the lowered binder against the resulting frame; this is not an assumed
pattern match or a fallback for refutable enum patterns. -/
partial def bindPattern (twins : Array SpecTypes.TwinInfo)
    (locals : Array Typed.LocalInfo) (slots : Array (Option Term))
    (pattern : Lean.Expr) (rep : Typed.ValueRep) (value : Term) :
    CommandElabM (Array (Option Term)) := do
  if pattern.isConstOf ``LeanerIR.SemanticOperations.NativePattern.wildcard then
    return slots
  if pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1 then
    let some index := index? (pattern.getArg! 0) | throwError "invalid native pattern local"
    let some slot := locals[index]? | throwError "native pattern local is out of bounds"
    unless slot.kind == .plain && slot.rep == rep do
      throwError "native pattern local type mismatch"
    return slots.set! index (some value)
  unless pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.constructor 3 &&
      (pattern.getArg! 1).isAppOf ``Option.none do
    throwError "native binding requires an irrefutable owned struct pattern"
  let .twin name #[] := rep | throwError "native struct pattern requires a monomorphic twin"
  let some info := twins.find? (·.twin == name) | throwError "missing native pattern twin"
  unless info.variants.isEmpty do throwError "native struct binding cannot assume an enum variant"
  let mut fields := pattern.getArg! 2
  let mut bound := slots
  for (field, fieldType) in info.fields do
    unless fields.isAppOfArity ``List.cons 3 do throwError "missing native pattern field"
    let projection ← ``($(rooted (name ++ Name.mkSimple field)) $value)
    bound ← bindPattern twins locals bound (fields.getArg! 1) (fieldRep fieldType) projection
    fields := fields.getArg! 2
  unless fields.isAppOf ``List.nil do throwError "extra native pattern field"
  return bound

/-- A variant predicate is an ordinary exhaustive match over its typed twin.
The descriptor is checked again by the separate evaluator equation. -/
def emitTest (twins : Array SpecTypes.TwinInfo) (rep : Typed.ValueRep) (value : Term)
    (returns : TSyntax `tactic) (expression : Lean.Expr) :
    CommandElabM (Term × TSyntax `tactic) := do
  let .twin name #[] := rep | throwError "native variant test requires a monomorphic enum"
  let some info := twins.find? (·.twin == name) | throwError "missing native enum twin"
  if info.variants.isEmpty then throwError "native variant test requires an enum"
  let descriptor := (expression.getArg! 0).getArg! 0
  unless descriptor.isAppOfArity ``NominalVariantTest.mk 2 do
    throwError "native variant test descriptor is not resolved"
  let some variants ← liftTermElabM <| Meta.getArrayLit? (descriptor.getArg! 1)
    | throwError "native variant test needs a closed variant set"
  let variants ← variants.mapM fun variant => do
    let .lit (.strVal name) := variant | throwError "native variant name is not resolved"
    pure name
  let mut arms : Array (TSyntax ``Lean.Parser.Term.matchAlt) := #[]
  for variant in info.variants do
    let fields ← variant.fields.mapM fun _ => `(term| _)
    let pattern ← ``($(rooted (name ++ Name.mkSimple variant.name)) $fields*)
    let selected := quote (variants.contains variant.name)
    arms := arms.push (← `(Lean.Parser.Term.matchAltExpr| | $pattern:term => $selected))
  let result ← `(term| ((match $value:term with $arms:matchAlt*) : Bool))
  let proof ← `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.nativeOperation_value
       (value := LeanerIR.RuntimeValue.bool $result)
     · apply LeanerIR.Proofs.ComputationAgreement.cons
       · $returns:tactic
       · exact LeanerIR.Proofs.ComputationAgreement.nil _
     · intro state
       cases h : $value:term <;>
         simp_all [$(rooted (name ++ `codec)):term, $(rooted (name ++ `erase)):term,
           LeanerIR.Proofs.Denotation.NominalVariantTest.evaluate?,
           LeanerIR.Proofs.Denotation.liftConstructorEvaluator,
           LeanerIR.SemanticOperations.testNominalVariants?]))
  return (result, proof)

/-- Shared constructor metadata for pure and effectful operand composition. -/
def constructorInfo (twins : Array SpecTypes.TwinInfo)
    (rep : Typed.ValueRep) (expression : Lean.Expr) :
    CommandElabM (Term × Array Typed.ValueRep) := do
  if isVector expression then
    let .vector element bounded := rep | throwError "native vector constructor requires a vector type"
    let mut remaining := expression.getArg! 1
    let mut count := 0
    while remaining.isAppOfArity ``valuesCons 2 do
      count := count + 1
      remaining := remaining.getArg! 1
    unless remaining.isConstOf ``valuesNil do throwError "unresolved native vector operands"
    let type ← element.typeSyntax (mkIdent `Carrier)
    let names := (List.range count).toArray.map fun index => mkIdent (Name.mkSimple s!"element_{index}")
    let values := names.map fun name => (⟨name.raw⟩ : Term)
    let typeOfVector ← rep.typeSyntax (mkIdent `Carrier)
    let mut constructor ← if bounded then
        ``((⟨#[$values,*], by change $(Syntax.mkNatLit count) < 2 ^ 64; decide⟩ : $typeOfVector))
      else ``((#[$values,*] : $typeOfVector))
    for name in names.reverse do
      constructor ← ``(fun $name : $type => $constructor)
    return (constructor, Array.replicate count element)
  unless supported expression do throwError "native aggregate requires a nominal constructor"
  let .twin name #[] := rep
    | throwError "native aggregate construction requires a monomorphic twin"
  let some info := twins.find? (·.twin == name)
    | throwError "native aggregate has no registered twin"
  let descriptor := (expression.getArg! 0).getArg! 0
  unless descriptor.isAppOfArity ``NominalConstructor.mk 3 do
    throwError "native aggregate constructor descriptor is not resolved"
  let variant := descriptor.getArg! 1
  let (constructor, fields) ← if variant.isAppOf ``Option.none then
      pure (name ++ `mk, info.fields)
    else do
      let .lit (.strVal variantName) := variant.getArg! 1
        | throwError "native aggregate variant is not resolved"
      let some variantInfo := info.variants.find? (·.name == variantName)
        | throwError "native aggregate variant has no twin constructor"
      pure (name ++ Name.mkSimple variantName, variantInfo.fields)
  return (⟨(rooted constructor).raw⟩, fields.map (fieldRep ·.2))

/-- Mapping the element codec over a native array is an extensional array
equation, not a definitional equality of the two array builders. -/
def constructorAgreement (expression : Lean.Expr) : CommandElabM (TSyntax `tactic) :=
  if isVector expression then `(tactic|
    simp [LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
      LeanerIR.Proofs.Codec.boundedVector_encode, LeanerIR.Proofs.Codec.vector_encode,
      LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
      LeanerIR.Proofs.Codec.address, LeanerIR.Proofs.Codec.string,
      LeanerIR.Proofs.Codec.signer, LeanerIR.Proofs.Codec.bytes])
  else `(tactic| rfl)

def emit (twins : Array SpecTypes.TwinInfo)
    (operand : Typed.ValueRep → Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (rep : Typed.ValueRep) (expression : Lean.Expr) :
    CommandElabM (Term × TSyntax `tactic) := do
  let (constructor, fields) ← constructorInfo twins rep expression
  let mut remaining := expression.getArg! 1
  let mut values : Array Term := #[]
  let mut proofs : Array (TSyntax `tactic) := #[]
  for field in fields do
    unless remaining.isAppOfArity ``valuesCons 2 do
      throwError "native aggregate is missing a field operand"
    let (value, proof) ← operand field (remaining.getArg! 0)
    values := values.push value
    proofs := proofs.push proof
    remaining := remaining.getArg! 1
  unless remaining.isConstOf ``valuesNil do
    throwError "native aggregate has extra field operands"
  let value ← ``($constructor $values*)
  let codec ← rep.codecSyntax (mkIdent `codecs)
  let mut operandsProof ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.nil _)
  for proof in proofs.reverse do
    operandsProof ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.cons
       · $proof:tactic
       · $operandsProof:tactic))
  let rule := mkIdent (if isVector expression then ``LeanerIR.Proofs.ComputationAgreement.operation_value
    else ``LeanerIR.Proofs.ComputationAgreement.nativeOperation_value)
  let agreement ← `(tactic|
    (apply $rule:term
       (value := ($codec).encode $value)
     · $operandsProof:tactic
     · intro state; $(← constructorAgreement expression):tactic))
  return (value, agreement)

end LeanerLang.NativeAggregate
