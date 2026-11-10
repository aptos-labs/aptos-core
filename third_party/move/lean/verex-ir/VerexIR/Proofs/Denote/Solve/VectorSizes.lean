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

/-- Certifying vector elements preserves the number of elements. -/
private theorem NTy.ofDeepHolds_vector_size [Carriers] (element : NTy)
    (value : (NTy.vector element).plain) (holds) :
    (NTy.ofDeepHolds (.vector element) value holds).values.size = value.values.size := by
  have encoded := NTy.plainEncode_ofDeepHolds (.vector element) value holds
  rw [NTy.encode_vector, NTy.plainEncode_vector] at encoded
  have same := congrArg Array.size (RuntimeValue.vector.inj encoded)
  simpa only [Array.size_map] using same

/-- Reading a certified vector field preserves its plain row's length. -/
private theorem Var.selectCertified_vector_size [Carriers] (source : VerexIR.StructHandle)
    (arguments fields : NRow) (element : NTy) (position : Var fields (.vector element))
    (value : (NTy.struct source arguments fields).carrier) :
    (position.selectCertified value).values.size = (position.selectPlain value.val).values.size := by
  have plain := congrArg (fun vector : (NTy.vector element).plain => vector.values.size)
    (Var.forget_selectCertified position value)
  simpa only [NTy.forget_vector_values, Array.size_map] using plain

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  [CarriersNonempty (Carriers.runtime unit)]
/-- Reframing vector elements preserves the number of elements. -/
private theorem Skolems.plainReframe_vector_size
    (Θ : Skolems unit) {params : NRow} (own : params.isParamsFrom 0 = true)
    {element : NTy} (bounded) (value : @NTy.plain Θ.toCarriers (.vector element)) :
    (Θ.plainReframe own bounded value).values.size = value.values.size := by
  have encoded := Θ.plainEncode_plainReframe own bounded value
  rw [@NTy.plainEncode_vector (Θ.instance params own).toCarriers,
    @NTy.plainEncode_vector Θ.toCarriers] at encoded
  have same := congrArg Array.size (RuntimeValue.vector.inj encoded)
  simpa only [Array.size_map] using same

end VectorSize

/-- Read vector lengths through certification and frame transport without
unfolding the element's carrier family. Construct the matching theorem at the
actual carrier spelling before checking the equation at default transparency. -/
simproc ↓ [vir_denote_norm] vectorCertificateSize (Array.size (VerexIR.SpecVector.values _)) := fun e => do
  let_expr Array.size _ values := e | return .continue
  let_expr VerexIR.SpecVector.values _ vector := values | return .continue
  let proof ← if vector.isAppOfArity ``VerexIR.Proofs.Denote.NTy.ofDeepHolds 4 then
      let args := vector.getAppArgs
      unless args[1]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 do return .continue
      pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.NTy.ofDeepHolds_vector_size)
        (args.set! 1 args[1]!.appArg!))
    else if vector.isAppOfArity ``VerexIR.Proofs.Denote.Var.selectCertified 7 then
      let args := vector.getAppArgs
      unless args[4]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 do return .continue
      pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.Var.selectCertified_vector_size)
        (args.set! 4 args[4]!.appArg!))
    else if vector.isAppOfArity ``VerexIR.Proofs.Denote.Skolems.plainReframe 9 then
      let args := vector.getAppArgs
      unless args[6]!.isAppOfArity ``VerexIR.Proofs.Denote.NTy.vector 1 do return .continue
      pure (mkAppN (mkConst ``VerexIR.Proofs.Denote.Skolems.plainReframe_vector_size)
        (args.set! 6 args[6]!.appArg!))
    else return .continue
  let some rhs ← (try withNewMCtxDepth <| withDefault do
      let some (_, lhs, rhs) := (← inferType proof).eq? | return none
      unless ← isDefEq lhs e do return none
      return some rhs
    catch _ => pure none) | return .continue
  return .visit { expr := rhs, proof? := some proof }

end VerexIR.Proofs.Denote
