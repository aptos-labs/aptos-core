-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Validation.Validated

/-!
# Executable place-index forms

Preparation, ownership analysis, and execution share this classifier so an
index admitted by the semantic boundary cannot be interpreted differently by
the initialization, borrowing, or runtime layers.
-/

namespace LeanerIR.Validation

inductive PlaceIndexForm where
  | literal (value : Int)
  | local (localId : LocalId)
  | copyLocal (localId : LocalId)
  | fromEnd (source : PlaceId) (offset : Nat)
  deriving Repr, BEq

/-- Classify the side-effect-free place-index expressions currently admitted
for execution. The `fromEnd` form is the exact expression emitted by the Rust
frontend for a slice index measured from its end. -/
def placeIndexForm? (ns : ValidatedNamespace) (expressionId : ExprId) :
    Option PlaceIndexForm := do
  let expression ← ns.expressions[expressionId.index]?
  match expression.kind with
  | .value (.integer value) _ => some (.literal value)
  | .localVar localId => some (.local localId)
  | .operation (.copy place) _ arguments _ => do
      if !arguments.isEmpty then none
      let .localVar localId ← ns.places[place.index]? | none
      some (.copyLocal localId)
  | .operation (.primitive .subtract) _ arguments _ => do
      let [lengthId, offsetId] := arguments.toList | none
      let lengthExpression ← ns.expressions[lengthId.index]?
      let .operation (.primitive .length) _ lengthArguments _ :=
        lengthExpression.kind | none
      let [sourceId] := lengthArguments.toList | none
      let sourceExpression ← ns.expressions[sourceId.index]?
      let .operation (.read source) _ sourceArguments _ := sourceExpression.kind | none
      if !sourceArguments.isEmpty then none
      let offsetExpression ← ns.expressions[offsetId.index]?
      let .value (.integer offset) _ := offsetExpression.kind | none
      if offset < 0 then none else some (.fromEnd source offset.toNat)
  | _ => none

end LeanerIR.Validation
