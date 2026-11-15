-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

/-!
# Vector size observations

Shared canonical size computations for normalization and guarded-write deciders.
The vector's whole-value representation stays intact.
-/

namespace VerexIR.Proofs.Denote

open Lean Elab Tactic Meta

section VectorSize

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

/-- Reframing vector elements preserves the number of elements. -/
private theorem Skolems.reframe_vector_size
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {element : NTy} (bounded) (value : @NTy.carrier Θ.toCarriers (.vector element)) :
    (Θ.reframe own bounded value).values.size = value.values.size := by
  have encoded := Θ.encode_reframe own bounded value
  rw [@NTy.encode_vector (Θ.instance params own).toCarriers,
    @NTy.encode_vector Θ.toCarriers] at encoded
  have same := congrArg Array.size (RuntimeValue.vector.inj encoded)
  simpa only [Array.size_map] using same

end VectorSize

/-- Reframing vector elements to a base function-carrier family preserves the number of elements. -/
theorem Skolems.reframeAtBase_vector_size {unit : Validation.ValidatedUnit}
    {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions) {params : NRow} (own : params.isParamsFrom 0 = true)
    (inhabitable : (params.map Θ.resolve).inhabitable = true)
    {element : NTy} (bounded) (value : @NTy.carrier Θ.toCarriers (.vector element)) :
    (Skolems.reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable
        (τ := .vector element) bounded value).values.size = value.values.size := by
  let Φ := (Skolems.atArguments unit base (params.map Θ.resolve) inhabitable).toCarriers
  have encoded := Skolems.encode_reframeAtBase sameCarrier sameFunctions sameNominals Θ own inhabitable bounded value
  rw [@NTy.encode_vector Φ, @NTy.encode_vector Θ.toCarriers] at encoded
  have same := congrArg Array.size (RuntimeValue.vector.inj encoded)
  simpa only [Array.size_map] using same

/-- Read vector lengths through frame transport without unfolding the element's
carrier family. Construct the matching theorem at the actual carrier spelling
before checking the equation at default transparency. -/
simproc ↓ [vir_denote_norm] vectorCertificateSize (Array.size (VerexIR.SpecVector.values _)) := fun e => do
  let_expr Array.size _ values := e | return .continue
  let_expr VerexIR.SpecVector.values _ vector := values | return .continue
  let proof? : Option Lean.Expr :=
    if vector.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.reframe 8 then
      let args := vector.getAppArgs
      if args[5]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 then
        some (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.reframe_vector_size)
          (args.set! 5 args[5]!.appArg!))
      else none
    else if vector.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.reframeAtBase 13 then
      let args := vector.getAppArgs
      if args[10]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 then
        some (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.reframeAtBase_vector_size)
          (args.set! 10 args[10]!.appArg!))
      else none
    else none
  let some proof := proof? | return .continue
  let some rhs ← (try withNewMCtxDepth <| withDefault do
      let some (_, lhs, rhs) := (← inferType proof).eq? | return none
      unless ← isDefEq lhs e do return none
      return some rhs
    catch _ => pure none) | return .continue
  return .visit { expr := rhs, proof? := some proof }

simproc [vir_denote_norm] vectorCertificateSizePost (Array.size (VerexIR.SpecVector.values _)) := fun e => do
  let_expr Array.size _ values := e | return .continue
  let_expr VerexIR.SpecVector.values _ vector := values | return .continue
  let vector := vector.consumeMData
  let proof? : Option Lean.Expr :=
    if vector.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.reframe 8 then
      let args := vector.getAppArgs
      if args[5]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 then
        some (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.reframe_vector_size)
          (args.set! 5 args[5]!.appArg!))
      else none
    else if vector.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.reframeAtBase 13 then
      let args := vector.getAppArgs
      if args[10]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 then
        some (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.reframeAtBase_vector_size)
          (args.set! 10 args[10]!.appArg!))
      else none
    else none
  let some rawProof := proof? | return .continue
  let some (rhs, proof) ← (try withNewMCtxDepth <| withDefault do
      let some (_, _, rhs) := (← inferType rawProof).eq? | return none
      let proof ← mkExpectedTypeHint rawProof (← mkEq e rhs)
      return some (rhs, proof)
    catch _ => pure none) | return .continue
  return .done { expr := rhs, proof? := some proof }

simproc [vir_denote, vir_denote_norm] arrayMapSize (Array.size (Array.map _ _)) := fun e => do
  let e := e.consumeMData
  unless e.isAppOfArity ``Array.size 2 do return .continue
  let mapped := (e.getArg! 1).consumeMData
  unless mapped.isAppOfArity ``Array.map 4 do return .continue
  let (α, β, f, xs) := (mapped.getArg! 0, mapped.getArg! 1, mapped.getArg! 2, mapped.getArg! 3)
  let rhs := mkApp2 (mkConst ``Array.size [0]) α xs
  let some equation ← (try withNewMCtxDepth <| withDefault do
      let rawProof ← mkAppOptM ``Array.size_map #[some α, some β, some f, some xs]
      some <$> mkExpectedTypeHint rawProof (← mkEq e rhs)
    catch _ => pure none) | return .continue
  return .done { expr := rhs, proof? := some equation }

end VerexIR.Proofs.Denote
