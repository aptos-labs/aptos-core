-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeOperands
import LeanerLang.Typed
import LeanerIR.Proofs.NativeValueAgreement
import LeanerIR.Proofs.NativeVectorAgreement

namespace LeanerLang.NativeVector

open Lean Elab Command
open LeanerIR.Proofs.Denotation
set_option quotPrecheck false

def isLength (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    (expression.getArg! 0).isAppOfArity ``PrimitiveLocationOperation.length 1

def emitLength (rep : Typed.ValueRep) (argument : NativeOperands.Value)
    (resultName : Option Ident := none) : CommandElabM NativeOperands.Value := do
  let .vector element true := rep | throwError "native Move length requires a bounded vector"
  let type ← rep.typeSyntax (mkIdent `Carrier)
  let codec ← rep.codecSyntax (mkIdent `codecs)
  let elementCodec ← element.codecSyntax (mkIdent `codecs)
  let operands ← NativeOperands.emit #[⟨type, ← ``(fun value : $type => ($codec).encode value), argument⟩]
  let name := mkIdent `vectorValue
  return {
    computation := ← ``(LeanerIR.Proofs.Spec.bind $(operands.computation)
      (fun $name : $(operands.type) => LeanerIR.Proofs.Spec.pure (LeanerIR.Proofs.NativeVector.length $name.1)))
    verifyWith := fun next => do
      let result ← if let some name := resultName then `(tactic|
          (rw [LeanerIR.Proofs.wp_pure_value]
           intro $name:ident $(mkIdent `valueEquation):ident
           have $(mkIdent `integerValueEquation):ident :=
             congrArg LeanerIR.SpecInt.val $(mkIdent `valueEquation):ident
           simp only [LeanerIR.Proofs.NativeVector.length_val]
             at $(mkIdent `integerValueEquation):ident
           $next:tactic))
        else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
      let proof ← operands.verifyWith result
      `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
    preserves := ← `(tactic|
      (apply LeanerIR.Proofs.StatePreserving.bind
       · $(operands.preserves):tactic
       · intro value; exact LeanerIR.Proofs.StatePreserving.pure _))
    agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_operation_value
         (encodeArgs := $(operands.encode))
         (encodeResult := fun value : LeanerIR.SpecInt (.bits 64) false =>
           LeanerIR.RuntimeValue.integer value.val)
         (value := fun value : $(operands.type) => LeanerIR.Proofs.NativeVector.length value.1)
       · $(operands.agreement):tactic
       · intro value state
         exact LeanerIR.Proofs.NativeVector.length_evaluate $elementCodec value.1 _ _)) }

end LeanerLang.NativeVector
