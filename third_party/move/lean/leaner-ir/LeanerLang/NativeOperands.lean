-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.OperandAgreement
import LeanerIR.Proofs.NativeValue

/-! Shared left-to-right native operand products. Computational values and
their separately checked encodings travel together only in generator metadata.
Generated computations contain typed products and `Spec.bind`, never encodings. -/

namespace LeanerLang.NativeOperands

open Lean Elab Command
set_option quotPrecheck false

structure Value where
  computation : Term
  verifyWith : TSyntax `tactic → CommandElabM (TSyntax `tactic)
  preserves : TSyntax `tactic
  agreement : TSyntax `tactic

structure Operand where
  type : Term
  encode : Term
  value : Value

structure Emitted where
  type : Term
  computation : Term
  encode : Term
  verifyWith : TSyntax `tactic → CommandElabM (TSyntax `tactic)
  verifyNamedWith : Ident → TSyntax `tactic → CommandElabM (TSyntax `tactic)
  preserves : TSyntax `tactic
  agreement : TSyntax `tactic

private def pureProof (name : Option Ident) (next : TSyntax `tactic) :
    CommandElabM (TSyntax `tactic) := do
  if let some name := name then `(tactic|
      (rw [LeanerIR.Proofs.wp_pure_value]
       intro $name:ident $(mkIdent `operandsEquation):ident
       $next:tactic))
  else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))

def emit (operands : Array Operand) : CommandElabM Emitted := do
  let mut row : Emitted := {
    type := ← ``(Unit)
    computation := ← ``(LeanerIR.Proofs.Spec.pure ())
    encode := ← ``(fun _ : Unit => ([] : List LeanerIR.RuntimeValue))
    verifyWith := pureProof none
    verifyNamedWith := fun name => pureProof (some name)
    preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure ())
    agreement := ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.operands_nil _) }
  for operand in operands.reverse do
    let rest := row
    let type ← ``($(operand.type) × $(rest.type))
    let computation ← ``(LeanerIR.Proofs.Spec.bind $(operand.value.computation) (fun head =>
      LeanerIR.Proofs.Spec.bind $(rest.computation) (fun tail =>
        LeanerIR.Proofs.Spec.pure (head, tail))))
    let verify (name : Option Ident) (next : TSyntax `tactic) : CommandElabM (TSyntax `tactic) := do
      let afterRest ← rest.verifyWith (← pureProof name next)
      let afterHead ← operand.value.verifyWith (← `(tactic|
        (rw [LeanerIR.Proofs.wp_bind]; $afterRest:tactic)))
      `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $afterHead:tactic))
    row := {
      type, computation
      encode := ← ``(fun pair : $type => $(operand.encode) pair.1 :: $(rest.encode) pair.2)
      verifyWith := verify none
      verifyNamedWith := fun name => verify (some name)
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(operand.value.preserves):tactic
         · intro head
           apply LeanerIR.Proofs.StatePreserving.bind
           · $(rest.preserves):tactic
           · intro tail; exact LeanerIR.Proofs.StatePreserving.pure _))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.operands_cons
           (headEncode := $(operand.encode)) (tailEncode := $(rest.encode))
         · $(operand.value.agreement):tactic
         · $(rest.agreement):tactic)) }
  return row

end LeanerLang.NativeOperands
